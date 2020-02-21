use anyhow::Context;
use serde::de::DeserializeOwned;
use std::io;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const DEVSECRETS_UUID_FILE: &str = ".devsecrets_uuid.txt";

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn devsecrets_config_root_dir() -> anyhow::Result<PathBuf> {
    let xdg_basedirs = xdg::BaseDirectories::new()?;
    Ok(xdg_basedirs.create_config_directory("rust-devsecrets")?)
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

pub fn get_devsecrets_dir() -> anyhow::Result<PathBuf> {
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").context("CARGO_MANIFEST_DIR must be defined")?;
    get_devsecrets_dir_from_manifest_dir(&manifest_dir)
}

pub fn read_devsecret<T: DeserializeOwned>(secret_name: &str) -> anyhow::Result<T> {
    let devsecrets_dir = get_devsecrets_dir()?;
    let secret_filename = std::path::PathBuf::from(format!("{}.json", secret_name));
    let secret_path = devsecrets_dir.join(&secret_filename);
    // It's possible the secret is something that would escape the
    // devsecrets_dir. Check that this isn't the case.
    if secret_path.parent() != Some(&*devsecrets_dir) {
        anyhow::bail!(
            "secret filename \"{:?}\" escaped devsecrets_dir \"{:?}\".",
            &secret_filename,
            devsecrets_dir
        );
    }
    Ok(serde_json::from_str(&std::fs::read_to_string(
        secret_path,
    )?)?)
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
