use anyhow::Context;
use serde::de::DeserializeOwned;
use std::io;
use std::path::{Component, Path, PathBuf};
use uuid::Uuid;

pub use devsecrets_macros::devsecrets_config;

/// An internal module to provide features to macros safely
pub mod internal {
    pub use lazy_static::lazy_static;
}

const DEVSECRETS_UUID_FILE: &str = ".devsecrets_uuid.txt";

pub struct DevSecrets {
    subdir: String,
}

impl DevSecrets {
    pub fn from_uuid_string(uuid_string: String) -> Self {
        DevSecrets {
            subdir: uuid_string,
        }
    }

    pub fn from_uuid_str(uuid_str: &str) -> Self {
        DevSecrets {
            subdir: uuid_str.to_string(),
        }
    }

    fn root_dir(&self) -> anyhow::Result<PathBuf> {
        Ok(devsecrets_config_root_dir()?.join(&self.subdir))
    }

    fn get_relative_path(&self, relpath: impl AsRef<Path>) -> anyhow::Result<PathBuf> {
        let relpath = relpath.as_ref();
        if relpath.is_absolute() {
            anyhow::bail!("Path {:?} must not be absolute.", relpath);
        }

        // Check that we only have normal parts of the path
        for component in relpath.components() {
            match component {
                Component::Normal(_) => (),
                _ => anyhow::bail!("Path {:?} has a non-normal component.", relpath),
            }
        }

        Ok(self.root_dir()?.join(relpath))
    }

    pub fn read_json_secret<T: DeserializeOwned, P: AsRef<Path>>(
        &self,
        path: P,
    ) -> anyhow::Result<T> {
        let path = path.as_ref();
        if path.extension() != Some(std::ffi::OsStr::new("json")) {
            anyhow::bail!("Path {:?} must have a .json extension.", path)
        }
        let fullpath = self.get_relative_path(path)?;
        log::info!("Reading json secret from {:?}", fullpath);
        let contents = std::fs::read_to_string(fullpath)?;
        Ok(serde_json::from_str::<T>(&contents)?)
    }
}

fn devsecrets_config_root_dir() -> anyhow::Result<PathBuf> {
    dirs::config_dir()
        .map(|p| p.join("rust-devsecrets"))
        .ok_or(anyhow::anyhow!("Could not find root config directory."))
}

fn read_file_or_create(
    path: impl AsRef<Path>,
    f: impl FnOnce() -> String,
) -> anyhow::Result<String> {
    match std::fs::read_to_string(path.as_ref()) {
        Ok(contents) => Ok(contents),
        Err(e) => match e.kind() {
            io::ErrorKind::NotFound => {
                let new_contents = f();
                std::fs::write(path.as_ref(), &new_contents)?;
                Ok(new_contents)
            }
            _ => Err(e.into()),
        },
    }
}

pub fn init_devsecrets_dir_from_manifest_dir(
    manifest_dir: impl AsRef<Path>,
) -> anyhow::Result<PathBuf> {
    let manifest_dir = manifest_dir.as_ref();
    let uuid_file = manifest_dir.join(DEVSECRETS_UUID_FILE);
    let uuid_text = read_file_or_create(uuid_file, || {
        Uuid::new_v4()
            .to_hyphenated()
            .encode_lower(&mut Uuid::encode_buffer())
            .to_string()
    })?;

    let root_dir = devsecrets_config_root_dir()?;
    let config_dir_path = root_dir.join(&uuid_text);
    std::fs::create_dir_all(&config_dir_path)?;
    Ok(config_dir_path)
}

pub fn get_devsecrets_dir_from_manifest_dir(
    manifest_dir: impl AsRef<Path>,
) -> anyhow::Result<PathBuf> {
    let manifest_dir = manifest_dir.as_ref();
    let uuid_file = manifest_dir.join(DEVSECRETS_UUID_FILE);
    let uuid_text = std::fs::read_to_string(&uuid_file).context(format!(
        "Could not read file {:?}",
        uuid_file.to_string_lossy(),
    ))?;
    // We parse but don't keep the value to validate it's a UUID
    Uuid::parse_str(&uuid_text)?;
    let config_dir_path = devsecrets_config_root_dir()?.join(&uuid_text);
    let config_dir_metadata = std::fs::metadata(&config_dir_path)?;
    if !config_dir_metadata.is_dir() {
        anyhow::bail!("Config root dir must be a directory.")
    }
    Ok(config_dir_path)
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
