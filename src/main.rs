use std::env;

use aws_sdk_s3 as s3;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::types::ObjectCannedAcl;
use clap::Parser;
use directories::ProjectDirs;
use miette::{IntoDiagnostic, Result, WrapErr};
use tracing::debug;

use config::Config;
use error::Error;

mod cli;
mod config;
mod error;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    if env::var("RUST_LOG").is_err() {
        env::set_var(
            "RUST_LOG",
            "mkpub=info,aws_config=error,aws_credential_types=error",
        );
    }
    tracing_subscriber::fmt::init();

    // Configure project directory paths
    let proj_dirs = ProjectDirs::from("dk.maero", "", "mkpub").ok_or(Error::ProjectDirs)?;

    let opts = cli::Opts::parse();

    let config_path = opts
        .config_path
        .unwrap_or(proj_dirs.config_dir().join("config.toml"));

    let config = Config::load_path(&config_path)
        .with_context(|| format!("Loading configuration file `{}'", config_path.display()))?;
    let aws_config = &config.aws_config;
    let s3_config = &aws_config.s3_config;

    let key_prefix = s3_config.key_prefix.clone().unwrap_or(String::new());

    let mut shared_config = aws_config::from_env();

    if let Some(ref profile_name) = aws_config.profile_name {
        shared_config = shared_config.profile_name(profile_name);
    }

    if let Some(ref endpoint_url) = aws_config.endpoint_url {
        shared_config = shared_config.endpoint_url(endpoint_url.as_str());
    }

    let shared_config = shared_config.load().await;

    let client = s3::Client::new(&shared_config);
    let body = ByteStream::from_path(&opts.path).await;
    let key = format!(
        "{}{}",
        key_prefix,
        &opts
            .path
            .file_name()
            .expect("no filename")
            .to_string_lossy()
    );

    match body {
        Ok(b) => {
            let content_type = mime_guess::from_path(&opts.path).first_or_octet_stream();
            let content_disposition: &str = match content_type.essence_str() {
                "image/jpeg" | "image/png" | "image/gif" | "image/svg" => "inline",
                "text/plain" => "inline",
                _ => "attachment",
            };
            debug!(?content_type);
            let resp = client
                .put_object()
                .bucket(&s3_config.bucket_name)
                .acl(ObjectCannedAcl::PublicRead)
                .key(&key)
                .content_type(content_type.as_ref())
                .content_disposition(content_disposition)
                .body(b)
                .send()
                .await
                .into_diagnostic()?;

            debug!(?resp);

            let url = s3_config
                .public_url
                .as_ref()
                .or(aws_config.endpoint_url.as_ref())
                .expect("could not derive public url");
            let url_with_path = url.join(&key).into_diagnostic()?;

            println!("{url_with_path}");
        }
        Err(e) => {
            miette::bail!(format!("{e}"));
        }
    }

    Ok(())
}
