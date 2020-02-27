use cargo_metadata::{Metadata, Package};
use serde::Deserialize;
use std::path::{Path, PathBuf};

fn find_crate_root(
    cargo_bin_path: impl AsRef<Path>,
    working_dir: impl AsRef<Path>,
    manifest_path_opt: Option<impl AsRef<Path>>,
) -> anyhow::Result<PathBuf> {
    let mut cmd = std::process::Command::new(cargo_bin_path.as_ref());
    cmd.arg("locate-project").current_dir(working_dir);

    if let Some(manifest_path) = manifest_path_opt {
        cmd.arg("--manifest-path").arg(manifest_path.as_ref());
    }

    let output = cmd.output()?;

    if output.status.code() == Some(101) {
        anyhow::bail!(
            "Could not find manifest file: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[derive(Deserialize)]
    struct LocateProjectOutput {
        root: String,
    }

    let LocateProjectOutput { root } =
        serde_json::from_slice::<LocateProjectOutput>(&output.stdout)?;
    Ok(PathBuf::from(root))
}

fn retrieve_metadata(manifest_path: impl AsRef<Path>) -> anyhow::Result<Metadata> {
    let mut cmd = cargo_metadata::MetadataCommand::new();
    cmd.manifest_path(manifest_path);
    cmd.no_deps();
    Ok(cmd.exec()?)
}

pub struct CargoWorkspace {
    manifest_path: PathBuf,
    metadata: Metadata,
}

impl CargoWorkspace {
    pub fn with_opt_manifest_path(manifest_path_opt: Option<&Path>) -> anyhow::Result<Self> {
        let cargo_bin_path = match std::env::var_os("CARGO") {
            Some(p) => p,
            None => anyhow::bail!(""),
        };

        let working_dir = std::env::current_dir()?;

        let manifest_path = find_crate_root(cargo_bin_path, working_dir, manifest_path_opt)?;

        let metadata = retrieve_metadata(&manifest_path)?;

        Ok(CargoWorkspace {
            manifest_path,
            metadata,
        })
    }

    pub fn find_default_package<'a>(&'a self) -> &'a Package {
        for package in &self.metadata.packages {
            if package.manifest_path == self.manifest_path {
                return package;
            }
        }

        panic!("Metadata must include the default package");
    }

    pub fn find_package<'a>(&'a self, name: &str) -> Option<&'a Package> {
        for package in &self.metadata.packages {
            if package.name == name {
                return Some(package);
            }
        }

        None
    }
}
