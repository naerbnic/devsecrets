//! The `devsecrets` crate allows easy access to development secrets without accidentally commiting
//! them to a repository.
//!
//! # Devsecret

mod format;

use serde::de::DeserializeOwned;
use std::error::Error as StdError;
use std::io;
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

pub use format::{Format, JsonFormat};

/// An opaque devsecrets ID for a project.
///
/// This value must be defined using the `import_id!()` macro. It's contents are
/// opaque, but can be used to create a DevSecrets instance using `DevSecrets::from_id(&id)`.
pub struct Id(#[doc(hidden)] pub internal_core::DevSecretsId);

/// Errors that occur when attempting to access secret files within a `DevSecrets` instance.
#[non_exhaustive]
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Indicates this package's devsecrets directory has not been initialized.
    #[error("Devsecrets directory was not initialized")]
    DirectoryNotInitialized,

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
    ParseError(#[source] Box<dyn StdError + Send + Sync + 'static>),

    #[error(transparent)]
    IoError(#[from] io::Error),
}

fn check_extension(p: &Path, ext: &str) -> Result<()> {
    if p.extension() != Some(std::ffi::OsStr::new(ext)) {
        return Err(Error::InvalidExtension(format!(
            "Path {:?} must have a .json extension.",
            p
        )));
    }

    Ok(())
}

type Result<T> = std::result::Result<T, Error>;

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
    pub fn from_id(id: &Id) -> Result<Self> {
        let root =
            devsecrets_core::DevSecretsRootDir::new()?.ok_or(Error::DirectoryNotInitialized)?;
        let child = root
            .get_child(&id.0)?
            .ok_or(Error::DirectoryNotInitialized)?;

        Ok(DevSecrets { dir: child })
    }

    fn root_dir(&self) -> &Path {
        self.dir.path()
    }

    fn get_relative_path(&self, relpath: impl AsRef<Path>) -> Result<PathBuf> {
        let relpath = relpath.as_ref();
        if relpath.is_absolute() {
            return Err(Error::InvalidRelativePath(format!(
                "Path {:?} must not be absolute.",
                relpath
            )));
        }

        // Check that we only have normal parts of the path
        for component in relpath.components() {
            match component {
                Component::Normal(_) => (),
                _ => {
                    return Err(Error::InvalidRelativePath(format!(
                        "Path {:?} has a non-normal component.",
                        relpath
                    )))
                }
            }
        }

        Ok(self.root_dir().join(relpath))
    }

    fn make_reader_inner(&self, path: impl AsRef<Path>) -> Result<std::fs::File> {
        let path = path.as_ref();
        let fullpath = self.get_relative_path(path)?;
        Ok(std::fs::File::open(fullpath).map_err(Error::FileError)?)
    }

    fn read(&self, path: impl AsRef<Path>) -> Result<Vec<u8>> {
        let path = path.as_ref();
        let fullpath = self.get_relative_path(path)?;
        let contents = std::fs::read(fullpath).map_err(Error::FileError)?;
        Ok(contents)
    }

    fn read_str(&self, path: impl AsRef<Path>) -> Result<String> {
        let contents = self.read(path)?;
        let string = String::from_utf8(contents).map_err(|e| Error::ParseError(Box::new(e)))?;
        Ok(string)
    }

    /// Indicates that data should be read from the given path.
    ///
    /// We use a builder-like pattern to read data to allow types to be explicitly
    /// stated when necessary, such as the type the value should be deserialized into.
    ///
    /// Example:
    ///
    /// ```text
    /// secrets
    ///     .read_from("my_path.json")
    ///     .with_format(devsecrets::JsonFormat)
    ///     .into_value::<MyType>()?;
    /// ```
    pub fn read_from<'a, P: AsRef<Path> + ?Sized>(&'a self, path: &'a P) -> Source<'a> {
        Source {
            secrets: self,
            path: path.as_ref(),
        }
    }
}

/// An intermediate type created from `DevSecrets::read_from()`.
pub struct Source<'a> {
    secrets: &'a DevSecrets,
    path: &'a Path,
}

impl<'a> Source<'a> {
    /// Indicates that the file should be deserialized with the given format.
    ///
    /// Returns a `SourceWithFormat` that can be used to deserialize a specific
    /// type.
    pub fn with_format<F: Format>(&self, fmt: F) -> SourceWithFormat<'a, F> {
        SourceWithFormat {
            secrets: self.secrets,
            path: self.path,
            format: fmt,
        }
    }

    /// Creates a reader to the given relative path in the devsecrets directory.
    pub fn to_reader(&self) -> Result<impl std::io::Read> {
        self.secrets.make_reader_inner(self.path)
    }

    /// Returns the contents of the given secrets file as a vector buffer.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        self.secrets.read(self.path)
    }

    /// Returns the contents of the given secrets file as a string. A ParseError
    /// is returned if the file is not a valid utf8 encoded text file.
    pub fn to_string(&self) -> Result<String> {
        self.secrets.read_str(self.path)
    }
}

/// An intermediate type created from `Source::with_format()`.
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
    /// Deserializes the indicated file using the indicated format of type `T`.
    pub fn into_value<T: DeserializeOwned>(&self) -> Result<T> {
        check_extension(self.path, self.format.extension().as_ref())?;
        Ok(self
            .format
            .deserialize::<T, std::fs::File>(self.secrets.make_reader_inner(self.path)?)
            .map_err(|e: F::Error| Error::ParseError(Box::new(e)))?)
    }
}
