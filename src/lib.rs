//! The `devsecrets` crate allows easy access to development secrets without accidentally commiting
//! them to a repository.
//!
//! # Devsecret

use serde::de::DeserializeOwned;
use std::path::{Component, Path, PathBuf};

// Re-export the devsecrets_id macro to make it available to users.

/// Imports the devsecrets ID from your project.
///
/// This defines a static value with the name you pass to the macro:
///
/// ```text
/// use devsecrets::import_id;
///
/// import_id!(SECRET_ID);
/// ```
///
/// If you so desire, you can also make this variable public:
///
/// ```text
/// use devsecrets::import_id;
///
/// import_id!(pub SECRET_ID);
/// ```
///
/// This macro reads the value of your devsecrets ID file at compile time. It
/// will fail if that file does not exist in your project, but will otherwise
/// succeed, even if the devsecrets directory has not been created in your
/// current environment.
pub use devsecrets_macros::devsecrets_id as import_id;

// Re-export the DevSecretsId struct to be used as an Id
pub use devsecrets_core::DevSecretsId as Id;

/// Errors that occur when attempting to access secret files within a `DevSecrets` instance.
#[non_exhaustive]
#[derive(thiserror::Error, Debug)]
pub enum AccessError {
    /// Indicates the relative path used to access the secret is invalid.
    /// 
    /// Relative paths must be relative, and not include any up-references to the parent path.
    /// For example, on unix platforms:
    /// 
    /// - `"mysecret.txt"` is **valid**.
    /// - `"a/b/c.txt"` is **valid**.
    /// - `"/etc/passwd"` is **invalid**.
    /// - `"../../anotherfile.txt"` is **invalid**.
    /// - `"a/../d/e.txt"` is **invalid**.
    #[error("Got invalid relative path: {0}")]
    InvalidRelativePath(String),

    /// Indicates the relative path used does not end with the expected extension.
    /// 
    /// This is used when using a method of `DevSecrets` that expects a specific
    /// datatype.
    #[error("Invalid file extension: {0}")]
    InvalidExtension(String),

    /// Indicates a low-level error occured during the access.
    #[error("File error: {0}")]
    FileError(#[source] std::io::Error),

    /// Indicates a parse error, when the accessor intends to parse the data.
    #[error("Could not parse file data: {0}")]
    ParseError(#[source] Box<dyn std::error::Error + Send + Sync + 'static>),
}
pub struct DevSecrets {
    dir: devsecrets_core::DevSecretsDir,
}

impl DevSecrets {
    pub fn from_id(id: &Id) -> std::io::Result<Option<Self>> {
        let root = match devsecrets_core::DevSecretsRootDir::new()?
        {
            Some(root) => root,
            None => return Ok(None),
        };
        let child = match root.get_child(id)? {
            Some(child) => child,
            None => return Ok(None),
        };

        Ok(Some(DevSecrets { dir: child }))
    }

    fn root_dir(&self) -> &Path {
        self.dir.path()
    }

    fn get_relative_path(&self, relpath: impl AsRef<Path>) -> Result<PathBuf, AccessError> {
        let relpath = relpath.as_ref();
        if relpath.is_absolute() {
            return Err(AccessError::InvalidRelativePath(format!(
                "Path {:?} must not be absolute.",
                relpath
            )));
        }

        // Check that we only have normal parts of the path
        for component in relpath.components() {
            match component {
                Component::Normal(_) => (),
                _ => {
                    return Err(AccessError::InvalidRelativePath(format!(
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
    ) -> Result<T, AccessError> {
        let path = path.as_ref();
        if path.extension() != Some(std::ffi::OsStr::new("json")) {
            return Err(AccessError::InvalidExtension(format!(
                "Path {:?} must have a .json extension.",
                path
            )));
        }
        let fullpath = self.get_relative_path(path)?;
        log::info!("Reading json secret from {:?}", fullpath);
        let contents = std::fs::read_to_string(fullpath).map_err(AccessError::FileError)?;
        Ok(serde_json::from_str::<T>(&contents)
            .map_err(|e| AccessError::ParseError(Box::new(e)))?)
    }
}
