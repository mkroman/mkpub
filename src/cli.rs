use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Opts {
    /// The file to publish
    pub path: PathBuf,

    /// Path to rhai script
    #[arg(short, long = "script")]
    pub script_path: Option<PathBuf>,

    /// Path to configuration file
    #[arg(short, long = "config")]
    pub config_path: Option<PathBuf>,
}
