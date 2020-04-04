use serde::de::DeserializeOwned;
use std::error::Error;
use std::io::Read;

/// A type of file format that can be deserialized using `serde`.
pub trait Format {
    /// The error type that deserialization can create. Is returned as the
    /// cause of `Error::ParseError`.
    type Error: Error + Sync + Send + Sized + 'static;

    /// The file extension expected for the source file.
    fn extension(&self) -> &str;

    /// Deserializes the data in the given reader into a value of type T, or
    /// returns a `Self::Error`.
    fn deserialize<T, R>(&self, reader: R) -> Result<T, Self::Error>
    where
        T: DeserializeOwned,
        R: Read;
}

impl<F: Format> Format for &'_ F {
    type Error = F::Error;

    fn extension(&self) -> &str {
        (*self).extension()
    }

    fn deserialize<T, R>(&self, reader: R) -> Result<T, Self::Error>
    where
        T: DeserializeOwned,
        R: Read,
    {
        (*self).deserialize::<T, R>(reader)
    }
}

/// The JSON file format.
///
/// Used as input for `Source::with_format()` when the file format should be a
/// JSON file.
#[derive(Debug, Default)]
pub struct JsonFormat;

impl Format for JsonFormat {
    type Error = serde_json::Error;

    fn extension(&self) -> &str {
        "json"
    }

    fn deserialize<T, R>(&self, reader: R) -> Result<T, Self::Error>
    where
        T: DeserializeOwned,
        R: Read,
    {
        serde_json::from_reader(reader)
    }
}
