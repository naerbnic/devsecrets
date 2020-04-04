use std::borrow::Cow;
use std::io;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub const DEVSECRETS_CONFIG_DIR: &str = "rust-devsecrets";
pub const DEVSECRETS_ID_FILE: &str = ".devsecrets_id.txt";

fn read_uuid(manifest_dir: impl AsRef<Path>) -> io::Result<Option<Uuid>> {
    let uuid_file = manifest_dir.as_ref().join(DEVSECRETS_ID_FILE);
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

pub fn read_devsecrets_id(manifest_dir: impl AsRef<Path>) -> io::Result<Option<DevSecretsId>> {
    Ok(read_uuid(manifest_dir)?.map(DevSecretsId::from_uuid))
}

pub fn ensure_devsecrets_id(manifest_dir: impl AsRef<Path>) -> io::Result<DevSecretsId> {
    let manifest_dir = manifest_dir.as_ref();
    match read_devsecrets_id(manifest_dir)? {
        Some(id) => Ok(id),
        None => {
            let uuid_file = manifest_dir.join(DEVSECRETS_ID_FILE);
            let new_id = DevSecretsId::new_unique();
            std::fs::write(uuid_file, new_id.id_str())?;
            Ok(new_id)
        }
    }
}

pub struct DevSecretsRootDir {
    config_dir: PathBuf,
}

impl DevSecretsRootDir {
    pub fn with_config_root(root: impl AsRef<Path>) -> io::Result<Option<Self>> {
        let root = root.as_ref();
        let config_dir = root.join(DEVSECRETS_CONFIG_DIR);
        if !config_dir.exists() {
            return Ok(None);
        }
        if !config_dir.is_dir() {
            return Err(io::ErrorKind::AlreadyExists.into());
        }
        Ok(Some(DevSecretsRootDir { config_dir }))
    }

    pub fn new() -> io::Result<Option<Self>> {
        match dirs::config_dir() {
            Some(p) => DevSecretsRootDir::with_config_root(p),
            None => Ok(None),
        }
    }

    pub fn ensure_with_config_root(root: impl AsRef<Path>) -> io::Result<Self> {
        let root = root.as_ref();
        let root_metadata = root.metadata()?;
        if !root_metadata.is_dir() {
            return Err(io::ErrorKind::AlreadyExists.into());
        }

        let config_dir = root.join(DEVSECRETS_CONFIG_DIR);
        std::fs::create_dir_all(&config_dir)?;
        Ok(DevSecretsRootDir { config_dir })
    }

    pub fn ensure_new() -> io::Result<Self> {
        let config_root = match dirs::config_dir() {
            Some(p) => p,
            None => return Err(io::ErrorKind::NotFound.into()),
        };
        DevSecretsRootDir::ensure_with_config_root(config_root)
    }

    pub fn get_child(&self, id: &DevSecretsId) -> io::Result<Option<DevSecretsDir>> {
        let child_dir = self.config_dir.join(id.id_str());

        if !child_dir.exists() {
            return Ok(None);
        }

        let metadata = child_dir.metadata()?;
        if !metadata.is_dir() {
            return Err(io::ErrorKind::AlreadyExists.into());
        }

        Ok(Some(DevSecretsDir { dir: child_dir }))
    }

    pub fn ensure_child(&self, id: &DevSecretsId) -> io::Result<DevSecretsDir> {
        let child_dir = self.config_dir.join(id.id_str());

        std::fs::create_dir_all(&child_dir)?;
        Ok(DevSecretsDir { dir: child_dir })
    }
}

pub struct DevSecretsId(pub Cow<'static, str>);

impl DevSecretsId {
    pub fn new_unique() -> Self {
        let uuid = Uuid::new_v4();
        DevSecretsId(
            uuid.to_hyphenated()
                .encode_lower(&mut Uuid::encode_buffer())
                .to_string()
                .into(),
        )
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        let mut buffer = Uuid::encode_buffer();
        let uuid_str = uuid.to_hyphenated().encode_lower(&mut buffer);
        DevSecretsId(uuid_str.to_string().into())
    }

    pub fn id_str(&self) -> &str {
        self.0.as_ref()
    }
}

pub struct DevSecretsDir {
    dir: PathBuf,
}

impl DevSecretsDir {
    pub fn path<'a>(&'a self) -> &'a Path {
        &self.dir
    }
}
