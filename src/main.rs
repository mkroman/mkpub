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
mod script;

async fn load_aws_sdk_config(app_aws_config: &config::AwsConfig) -> aws_config::SdkConfig {
    let mut sdk_config = aws_config::from_env();

    if let Some(ref profile_name) = app_aws_config.profile_name {
        sdk_config = sdk_config.profile_name(profile_name);
    }

    if let Some(ref endpoint_url) = app_aws_config.endpoint_url {
        sdk_config = sdk_config.endpoint_url(endpoint_url.as_str());
    }

    sdk_config.load().await
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
    let app_aws_config = &config.aws_config;
    let s3_config = &app_aws_config.s3_config;
    let sdk_config = load_aws_sdk_config(app_aws_config).await;
    let client = aws_s3::Client::new(&sdk_config);

    // Initialize the scripting engine.
    let engine = script::build_engine();

    // Precompile the program AST so we can evaluate it faster.
    let script_path = opts
        .script_path
        .unwrap_or(proj_dirs.config_dir().join("program.rhai"));

    trace!(
        script_path = %script_path.display(),
        "compiling rhai script"
    );

    let start = Instant::now();
    let ast = engine.compile_file(script_path).unwrap();

    trace!(elapsed = ?start.elapsed(), "script compiled");

    // Upload the provided files.
    for path in &opts.paths {
        // Upload the provided file.
        let body = ByteStream::from_path(path).await;

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

                let mut put_object = client
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
                    .or(app_aws_config.endpoint_url.as_ref())
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
