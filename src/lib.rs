use serde::de::DeserializeOwned;
use std::path::{Component, Path, PathBuf};

pub use devsecrets_macros::devsecrets_id;

pub use devsecrets_core::DevSecretsId as Id;

pub struct DevSecrets {
    dir: devsecrets_core::DevSecretsDir,
}

impl DevSecrets {
    pub fn from_id(id: &Id) -> anyhow::Result<Option<Self>> {
        let root = match devsecrets_core::DevSecretsRootDir::new()? {
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

        Ok(self.root_dir().join(relpath))
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

pub fn init_devsecrets_dir_from_manifest_dir(
    manifest_dir: impl AsRef<Path>,
) -> anyhow::Result<PathBuf> {
    let id = devsecrets_core::ensure_devsecrets_id(manifest_dir)?;
    let root = devsecrets_core::DevSecretsRootDir::ensure_new()?;
    let child = root.ensure_child(&id)?;
    Ok(child.path().to_path_buf())
}

pub fn get_devsecrets_dir_from_manifest_dir(
    manifest_dir: impl AsRef<Path>,
) -> anyhow::Result<Option<PathBuf>> {
    let id = devsecrets_core::read_devsecrets_id(manifest_dir)?
        .ok_or(anyhow::anyhow!("Could not read devsecrets id from project"))?;
    let root = match devsecrets_core::DevSecretsRootDir::new()? {
        Some(root) => root,
        None => return Ok(None),
    };

    let child = match root.get_child(&id)? {
        Some(child) => child,
        None => return Ok(None),
    };
    Ok(Some(child.path().to_path_buf()))
}
