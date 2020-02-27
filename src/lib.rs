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

#[doc(hidden)]
pub use devsecrets_core as internal_core;

pub struct Id(#[doc(hidden)] pub internal_core::DevSecretsId);

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

fn check_extension(p: &Path, ext: &str) -> Result<(), AccessError> {
    if p.extension() != Some(std::ffi::OsStr::new(ext)) {
        return Err(AccessError::InvalidExtension(format!(
            "Path {:?} must have a .json extension.",
            p
        )));
    }

    Ok(())
}

/// Used to access the files inside of the devsecrets directory for your project.
///
/// This can be obtained by calling `DevSecrets::from_id(&ID)` with a devsecrets
/// ID imported via `import_id!()`.
/// 
/// Note: This API does not allow writing to the secrets directory. Use the
/// `cargo devsecrets` tool to help with that.
pub struct DevSecrets {
    dir: devsecrets_core::DevSecretsDir,
}

impl DevSecrets {
    /// Create a `DevSecrets` instance from an `Id`.
    ///
    /// Returns an `Err(std::io::Error)` if there was a low-level issue reading
    /// the devsecrets directory. Returns `Ok(None)` if no devsecrets directory
    /// was found. Returns `Ok(Some(_))` when the devsecrets directory was found
    /// and ready to use.
    ///
    /// The `Id` value passed to this function can be obtained via `import_id!()`.
    pub fn from_id(id: &Id) -> std::io::Result<Option<Self>> {
        let root = match devsecrets_core::DevSecretsRootDir::new()? {
            Some(root) => root,
            None => return Ok(None),
        };
        let child = match root.get_child(&id.0)? {
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

    /// Creates a reader to the given relative path in the devsecrets directory.
    pub fn make_reader(&self, path: impl AsRef<Path>) -> Result<impl std::io::Read, AccessError> {
        let path = path.as_ref();
        let fullpath = self.get_relative_path(path)?;
        Ok(std::fs::File::open(fullpath).map_err(AccessError::FileError)?)
    }

    /// Returns the contents of the given secrets file as a vector buffer.
    pub fn read(&self, path: impl AsRef<Path>) -> Result<Vec<u8>, AccessError> {
        let path = path.as_ref();
        let fullpath = self.get_relative_path(path)?;
        let contents = std::fs::read(fullpath).map_err(AccessError::FileError)?;
        Ok(contents)
    }

    /// Returns the contents of the given secrets file as a string. A ParseError
    /// is returned if the file is not a valid utf8 encoded text file.
    pub fn read_str(&self, path: impl AsRef<Path>) -> Result<String, AccessError> {
        let contents = self.read(path)?;
        let string =
            String::from_utf8(contents).map_err(|e| AccessError::ParseError(Box::new(e)))?;
        Ok(string)
    }

    pub fn read_json_secret<T: DeserializeOwned>(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<T, AccessError> {
        let path = path.as_ref();
        check_extension(path, "json")?;
        Ok(serde_json::from_reader(self.make_reader(path)?)
            .map_err(|e| AccessError::ParseError(Box::new(e)))?)
    }
}
