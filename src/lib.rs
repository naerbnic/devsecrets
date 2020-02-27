use serde::de::DeserializeOwned;
use std::path::{Component, Path, PathBuf};

// Re-export the devsecrets_id macro to make it available to users.
pub use devsecrets_macros::devsecrets_id;
// Re-export the DevSecretsId struct to be used as an Id
pub use devsecrets_core::DevSecretsId as Id;

#[derive(thiserror::Error, Debug)]
pub enum SecretCreateError {
    #[error("Could not find child dir: {0}")]
    ChildDirNotFound(#[source] std::io::Error),

    #[error("Could not find root dir: {0}")]
    RootDirNotFound(#[source] std::io::Error),
}

/// A general error for DevSecrets operations.
#[derive(thiserror::Error, Debug)]
pub enum SecretReadError {
    #[error("Got invalid relative path: {0}")]
    InvalidRelativePath(String),

    #[error("Invalid extension: {0}")]
    InvalidExtension(String),

    #[error("File error: {0}")]
    FileError(#[from] std::io::Error),

    #[error("Could not parse file data: {0}")]
    ParseError(#[source] Box<dyn std::error::Error + Send + Sync + 'static>),
}

pub struct DevSecrets {
    dir: devsecrets_core::DevSecretsDir,
}

impl DevSecrets {
    pub fn from_id(id: &Id) -> std::result::Result<Option<Self>, SecretCreateError> {
        let root = match devsecrets_core::DevSecretsRootDir::new()
            .map_err(SecretCreateError::ChildDirNotFound)?
        {
            Some(root) => root,
            None => return Ok(None),
        };
        let child = match root
            .get_child(id)
            .map_err(SecretCreateError::RootDirNotFound)?
        {
            Some(child) => child,
            None => return Ok(None),
        };

        Ok(Some(DevSecrets { dir: child }))
    }

    fn root_dir(&self) -> &Path {
        self.dir.path()
    }

    fn get_relative_path(&self, relpath: impl AsRef<Path>) -> Result<PathBuf, SecretReadError> {
        let relpath = relpath.as_ref();
        if relpath.is_absolute() {
            return Err(SecretReadError::InvalidRelativePath(format!(
                "Path {:?} must not be absolute.",
                relpath
            )));
        }

        // Check that we only have normal parts of the path
        for component in relpath.components() {
            match component {
                Component::Normal(_) => (),
                _ => {
                    return Err(SecretReadError::InvalidRelativePath(format!(
                        "Path {:?} has a non-normal component.",
                        relpath
                    )))
                }
            }
        }

        Ok(self.root_dir().join(relpath))
    }

    pub fn read_json_secret<T: DeserializeOwned, P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Result<T, SecretReadError> {
        let path = path.as_ref();
        if path.extension() != Some(std::ffi::OsStr::new("json")) {
            return Err(SecretReadError::InvalidExtension(format!(
                "Path {:?} must have a .json extension.",
                path
            )));
        }
        let fullpath = self.get_relative_path(path)?;
        log::info!("Reading json secret from {:?}", fullpath);
        let contents = std::fs::read_to_string(fullpath).map_err(SecretReadError::FileError)?;
        Ok(serde_json::from_str::<T>(&contents)
            .map_err(|e| SecretReadError::ParseError(Box::new(e)))?)
    }
}
