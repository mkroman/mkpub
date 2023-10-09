//! Application errors

use std::io;

use miette::Diagnostic;
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
pub enum Error {
    #[error("Could not determine project directories")]
    #[diagnostic(
        code(mkpub::project_dirs),
        help("this could mean your platform is not supported")
    )]
    ProjectDirs,
    #[error("The given path is not a valid file")]
    InvalidFile,
    #[error("I/O error")]
    IoError(#[from] io::Error),
}
