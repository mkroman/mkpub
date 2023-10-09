use std::env;
use std::time::Instant;

use aws_sdk_s3 as aws_s3;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::types::ObjectCannedAcl;
use clap::Parser;
use directories::ProjectDirs;
use miette::{IntoDiagnostic, Result, WrapErr};
use rhai::Scope;
use tracing::{debug, trace};

use config::Config;
pub use error::Error;
use script::ScriptablePutObject;

mod cli;
mod config;
mod error;
mod s3;
mod script;

#[derive(Clone, Debug)]
pub struct Context {
    pub s3_client: aws_s3::Client,
    pub aws_config: config::AwsConfig,
}

async fn load_aws_config(app_aws_config: &config::AwsConfig) -> aws_config::SdkConfig {
    let mut config_loader = aws_config::from_env();

    // Override the profile name to load.
    if let Some(ref profile_name) = app_aws_config.profile_name {
        config_loader = config_loader.profile_name(profile_name);
    }

    // Override the endpoint URL for all AWS services if provided.
    if let Some(ref endpoint_url) = app_aws_config.endpoint_url {
        config_loader = config_loader.endpoint_url(endpoint_url.as_str());
    }

    config_loader.load().await
}

fn init_tracing() {
    tracing_subscriber::fmt::init();
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    if env::var("RUST_LOG").is_err() {
        env::set_var(
            "RUST_LOG",
            "mkpub=info,aws_config=error,aws_credential_types=error",
        );
    }

    init_tracing();

    // Parse CLI options.
    let opts = cli::Opts::parse();

    // Configure project directory paths.
    let proj_dirs = ProjectDirs::from("dk.maero", "", "mkpub").ok_or(Error::ProjectDirs)?;

    trace!(config_dir = ?proj_dirs.config_dir(), "using project dirs");

    // Load configuration file.
    let config_path = opts
        .config_path
        .unwrap_or(proj_dirs.config_dir().join("config.toml"));

    trace!(config_path = %config_path.display(), "loading configuration");
    let config = Config::load_path(&config_path)
        .with_context(|| format!("Loading configuration file `{}'", config_path.display()))?;

    // Prepare the AWS S3 client.
    let sdk_config = load_aws_config(&config.aws_config).await;
    let s3_client = aws_s3::Client::new(&sdk_config);
    let ctx = Context {
        s3_client,
        aws_config: config.aws_config,
    };
    let s3_config = ctx.aws_config.s3_config;

    // Initialize the scripting engine.
    let engine = script::build_engine();

    // Precompile the program AST so we can evaluate it faster.
    let start = Instant::now();
    let ast = if let Some(script_path) = opts.script_path {
        trace!(
            script_path = %script_path.display(),
            "compiling rhai script"
        );

        engine.compile_file(script_path).unwrap()
    } else {
        engine.compile(script::DEFAULT_RHAI_PROGRAM).unwrap()
    };
    trace!(elapsed = ?start.elapsed(), "script compiled");

    let mut batch = s3::Batch::new();

    // Add files to the upload batch.
    for path in &opts.paths {
        debug!(path = %path.display(), "adding file to batch");
        batch.add(path)?;
    }

    // Upload the provided files.
    for path in batch.files() {
        trace!(path = %path.display(), "processing batch file");

        // Upload the provided file.
        let body = ByteStream::from_path(&path).await;

        match body {
            Ok(b) => {
                // Create a scope for each run of the script.
                let mut scope = Scope::new();

                let object = ScriptablePutObject::default();

                scope.push("object", object).push("path", path.clone());

                // Run the script with the new scope.
                engine.run_ast_with_scope(&mut scope, &ast).unwrap();

                let updated_object = scope.get_value::<ScriptablePutObject>("object").unwrap();
                debug!(?updated_object);

                let mut put_object = ctx
                    .s3_client
                    .put_object()
                    .bucket(&s3_config.bucket_name)
                    .acl(ObjectCannedAcl::PublicRead);

                let mut key = Option::None;

                if let Some(updated_key) = updated_object.key {
                    key = Some(updated_key.clone());
                    put_object = put_object.key(updated_key);
                }

                if let Some(content_type) = updated_object.content_type {
                    put_object = put_object.content_type(content_type);
                }

                if let Some(content_disposition) = updated_object.content_disposition {
                    put_object = put_object.content_disposition(content_disposition);
                } else {
                    put_object = put_object.content_disposition("attachment");
                }

                let resp = put_object.body(b).send().await.into_diagnostic()?;

                debug!(?resp);

                let url = s3_config
                    .public_url
                    .as_ref()
                    .or(ctx.aws_config.endpoint_url.as_ref())
                    .expect("could not derive public url");
                let url_with_path = url.join(key.unwrap().as_str()).into_diagnostic()?;

                println!("{url_with_path}");
            }
            Err(e) => {
                miette::bail!(format!("{e}"));
            }
        }
    }

    Ok(())
}
