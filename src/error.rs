//! Application errors

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
}
