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

/// An opaque devsecrets ID for a project.
///
/// This value must be defined using the `import_id!()` macro. It's contents are
/// opaque, but can be used to create a DevSecrets instance using `DevSecrets::from_id(&id)`.
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

    fn make_reader_inner(&self, path: impl AsRef<Path>) -> Result<std::fs::File, AccessError> {
        let path = path.as_ref();
        let fullpath = self.get_relative_path(path)?;
        Ok(std::fs::File::open(fullpath).map_err(AccessError::FileError)?)
    }

    fn read(&self, path: impl AsRef<Path>) -> Result<Vec<u8>, AccessError> {
        let path = path.as_ref();
        let fullpath = self.get_relative_path(path)?;
        let contents = std::fs::read(fullpath).map_err(AccessError::FileError)?;
        Ok(contents)
    }

    fn read_str(&self, path: impl AsRef<Path>) -> Result<String, AccessError> {
        let contents = self.read(path)?;
        let string =
            String::from_utf8(contents).map_err(|e| AccessError::ParseError(Box::new(e)))?;
        Ok(string)
    }

    pub fn read_from<'a, P: AsRef<Path> + ?Sized>(&'a self, path: &'a P) -> Source<'a> {
        Source {
            secrets: self,
            path: path.as_ref(),
        }
    }
}

pub struct Source<'a> {
    secrets: &'a DevSecrets,
    path: &'a Path,
}

impl<'a> Source<'a> {
    pub fn with_format<F: Format>(&self, fmt: F) -> SourceWithFormat<'a, F> {
        SourceWithFormat {
            secrets: self.secrets,
            path: self.path,
            format: fmt,
        }
    }

    /// Creates a reader to the given relative path in the devsecrets directory.
    pub fn as_reader(&self) -> Result<impl std::io::Read, AccessError> {
        self.secrets.make_reader_inner(self.path)
    }

    /// Returns the contents of the given secrets file as a vector buffer.
    pub fn to_bytes(&self) -> Result<Vec<u8>, AccessError> {
        self.secrets.read(self.path)
    }

    /// Returns the contents of the given secrets file as a string. A ParseError
    /// is returned if the file is not a valid utf8 encoded text file.
    pub fn to_string(&self) -> Result<String, AccessError> {
        self.secrets.read_str(self.path)
    }
}

pub struct SourceWithFormat<'a, F>
where
    F: Format,
{
    secrets: &'a DevSecrets,
    path: &'a Path,
    format: F,
}

impl<'a, F> SourceWithFormat<'a, F>
where
    F: Format,
{
    pub fn into_value<T: serde::de::DeserializeOwned>(&self) -> Result<T, AccessError> {
        check_extension(self.path, self.format.extension().as_ref())?;
        Ok(self
            .format
            .deserialize::<T, std::fs::File>(self.secrets.make_reader_inner(self.path)?)
            .map_err(|e: F::Error| AccessError::ParseError(Box::new(e)))?)
    }
}

pub trait Format {
    type Error: std::error::Error + Sync + Send + Sized + 'static;

    fn extension(&self) -> &str;
    fn deserialize<T: serde::de::DeserializeOwned, R: std::io::Read>(
        &self,
        reader: R,
    ) -> Result<T, Self::Error>;
}

#[derive(Debug)]
pub struct JsonFormat;

impl Format for JsonFormat {
    type Error = serde_json::Error;

    fn extension(&self) -> &str {
        "json"
    }

    fn deserialize<T: serde::de::DeserializeOwned, R: std::io::Read>(
        &self,
        reader: R,
    ) -> Result<T, serde_json::Error> {
        serde_json::from_reader(reader)
    }
}
