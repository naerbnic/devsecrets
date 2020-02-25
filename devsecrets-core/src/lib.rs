use std::borrow::Cow;
use std::io;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub const DEVSECRETS_CONFIG_DIR: &str = "rust-devsecrets";
pub const DEVSECRETS_UUID_FILE: &str = ".devsecrets_uuid.txt";

pub fn read_uuid(manifest_dir: impl AsRef<Path>) -> io::Result<Option<Uuid>> {
    let uuid_file = manifest_dir.as_ref().join(DEVSECRETS_UUID_FILE);
    if !uuid_file.exists() {
        return Ok(None);
    }
    let contents = std::fs::read_to_string(&uuid_file)?;
    let uuid = match Uuid::parse_str(&contents) {
        Ok(uuid) => uuid,
        Err(_) => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Did not read valid UUID from {:?}", uuid_file),
            ))
        }
    };
    Ok(Some(uuid))
}

pub fn read_devsecrets_id(manifest_dir: impl AsRef<Path>) -> io::Result<Option<DevsecretsId>> {
    Ok(read_uuid(manifest_dir)?.map(DevsecretsId::from_uuid))
}

pub fn ensure_devsecrets_id(manifest_dir: impl AsRef<Path>) -> io::Result<DevsecretsId> {
    let manifest_dir = manifest_dir.as_ref();
    match read_devsecrets_id(manifest_dir)? {
        Some(id) => Ok(id),
        None => {
            let uuid_file = manifest_dir.join(DEVSECRETS_UUID_FILE);
            let new_id = DevsecretsId::new_unique();
            std::fs::write(uuid_file, new_id.id_str())?;
            Ok(new_id)
        }
    }
}

pub struct DevsecretsRootDir {
    config_dir: PathBuf,
}

impl DevsecretsRootDir {
    pub fn with_config_root(root: impl AsRef<Path>) -> io::Result<Option<Self>> {
        let root = root.as_ref();
        let config_dir = root.join(DEVSECRETS_CONFIG_DIR);
        if !config_dir.exists() {
            return Ok(None);
        }
        if !config_dir.is_dir() {
            return Err(io::ErrorKind::AlreadyExists.into());
        }
        Ok(Some(DevsecretsRootDir { config_dir }))
    }

    pub fn new() -> io::Result<Option<Self>> {
        let config_root = match dirs::config_dir() {
            Some(p) => p,
            None => return Err(io::ErrorKind::NotFound.into()),
        };
        DevsecretsRootDir::with_config_root(config_root)
    }

    pub fn ensure_with_config_root(root: impl AsRef<Path>) -> io::Result<Self> {
        let root = root.as_ref();
        let root_metadata = root.metadata()?;
        if !root_metadata.is_dir() {
            return Err(io::ErrorKind::AlreadyExists.into());
        }

        let config_dir = root.join(DEVSECRETS_CONFIG_DIR);
        std::fs::create_dir_all(&config_dir)?;
        Ok(DevsecretsRootDir { config_dir })
    }

    pub fn ensure_new() -> io::Result<Self> {
        let config_root = match dirs::config_dir() {
            Some(p) => p,
            None => return Err(io::ErrorKind::NotFound.into()),
        };
        DevsecretsRootDir::ensure_with_config_root(config_root)
    }

    pub fn get_child(&self, id: &DevsecretsId) -> io::Result<Option<DevsecretsDir>> {
        let child_dir = self.config_dir.join(id.id_str());

        if !child_dir.exists() {
            return Ok(None);
        }

        let metadata = child_dir.metadata()?;
        if !metadata.is_dir() {
            return Err(io::ErrorKind::AlreadyExists.into());
        }

        Ok(Some(DevsecretsDir { dir: child_dir }))
    }

    pub fn ensure_child(&self, id: &DevsecretsId) -> io::Result<DevsecretsDir> {
        let child_dir = self.config_dir.join(id.id_str());

        std::fs::create_dir_all(&child_dir)?;
        Ok(DevsecretsDir { dir: child_dir })
    }
}

pub struct DevsecretsId(pub Cow<'static, str>);

impl DevsecretsId {
    pub fn new_unique() -> Self {
        let uuid = Uuid::new_v4();
        DevsecretsId(
            uuid.to_hyphenated()
                .encode_lower(&mut Uuid::encode_buffer())
                .to_string()
                .into(),
        )
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        let mut buffer = Uuid::encode_buffer();
        let uuid_str = uuid.to_hyphenated().encode_lower(&mut buffer);
        DevsecretsId(uuid_str.to_string().into())
    }

    pub fn id_str(&self) -> &str {
        self.0.as_ref()
    }
}

pub struct DevsecretsDir {
    dir: PathBuf,
}

impl DevsecretsDir {
    pub fn path<'a>(&'a self) -> &'a Path {
        &self.dir
    }
}
