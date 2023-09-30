//! User configuration

use std::path::Path;
use std::{fs, io};

use miette::Diagnostic;
use serde::Deserialize;
use thiserror::Error;
use toml::de::Error as TomlError;
use url::Url;

#[derive(Error, Debug, Diagnostic)]
pub enum ConfigError {
    #[diagnostic(code(mkpub::config::load))]
    #[error(transparent)]
    Io(#[from] io::Error),

    #[error("could not parse config file as toml")]
    #[diagnostic(code(mkpub::config::parse))]
    ParseToml(#[source] TomlError),
}

#[derive(Debug, Eq, PartialEq, Deserialize)]
pub struct Config {
    #[serde(rename = "aws")]
    pub aws_config: AwsConfig,
}

#[derive(Debug, Eq, PartialEq, Deserialize)]
pub struct AwsConfig {
    pub profile_name: Option<String>,
    pub endpoint_url: Option<Url>,
    #[serde(rename = "s3")]
    pub s3_config: AwsS3Config,
}

#[derive(Debug, Eq, PartialEq, Deserialize)]
pub struct AwsS3Config {
    pub bucket_name: String,
    pub public_url: Option<Url>,
}

impl Config {
    pub fn load_path<P: AsRef<Path>>(path: P) -> Result<Config, ConfigError> {
        let buf = fs::read_to_string(path)?;
        let toml: Config = ::toml::from_str(&buf).map_err(ConfigError::ParseToml)?;

        Ok(toml)
    }
}
