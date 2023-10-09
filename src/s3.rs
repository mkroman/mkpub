//! AWS S3 interactions

use std::path::{Path, PathBuf};

use tracing::{instrument, trace};

use crate::{Context, Error};

/// Uploads a batch of files to S3.
#[instrument]
pub async fn upload_batch(
    ctx: Context,
    client: &aws_sdk_s3::Client,
    bucket: &str,
    batch: Batch,
) -> Result<(), Error> {
    trace!("uploading batch");

    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchUploader {
    /// The name of the destination bucket.
    bucket_name: String,
}

/// Batch of files that can be uploaded.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Batch {
    /// List of files included in this batch.
    files: Vec<PathBuf>,
    /// Sum of the size of files.
    size: usize,
}

impl Batch {
    /// Constructs a new and empty [`Batch`].
    pub fn new() -> Self {
        Batch {
            files: vec![],
            size: 0,
        }
    }

    /// Returns a list of paths for the files in this batch.
    pub fn files(&self) -> Vec<PathBuf> {
        self.files.clone()
    }

    /// Adds the file at `path` to this upload batch.
    ///
    /// # Errors
    ///
    /// The function will return an IO error if the given `path` isn't a valid file.
    pub fn add(&mut self, path: impl AsRef<Path>) -> Result<(), Error> {
        let path = path.as_ref();
        let metadata = path.metadata()?;

        if metadata.is_file() {
            self.files.push(path.into());
            self.size += metadata.len() as usize;
        } else {
            return Err(Error::InvalidFile);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn it_should_error_on_nonexistent_path() {
        let mut batch = Batch::new();

        batch.add("/tmp/shoulda/woulda/coulda-123").unwrap();
    }

    #[test]
    fn it_should_be_empty_batch() {
        let batch = Batch::new();

        assert_eq!(batch.size, 0);
        assert_eq!(batch.files.len(), 0);
    }
}
