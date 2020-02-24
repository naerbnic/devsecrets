use cargo_metadata::{Metadata, Package};
use clap::{App, AppSettings, Arg, SubCommand};
use serde::Deserialize;
use std::path::{Path, PathBuf};

struct CargoWorkspace {
    manifest_path: PathBuf,
    metadata: Metadata,
}

pub fn find_crate_root(
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

pub fn retrieve_metadata(manifest_path: impl AsRef<Path>) -> anyhow::Result<Metadata> {
    let mut cmd = cargo_metadata::MetadataCommand::new();
    cmd.manifest_path(manifest_path);
    cmd.no_deps();
    Ok(cmd.exec()?)
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

fn main() {
    env_logger::init();
    let matches = App::new("cargo")
        .bin_name("cargo")
        .author(clap::crate_authors!())
        .version(clap::crate_version!())
        .setting(AppSettings::SubcommandRequired)
        .subcommand(
            SubCommand::with_name("devsecrets")
                .setting(AppSettings::SubcommandRequired)
                .arg(
                    Arg::with_name("manifest-path")
                        .long("metadata-path")
                        .takes_value(true)
                        .value_name("CRATEDIR")
                        .help(
                            "The path to the crate to work with. If not set, selects the closest \
                            ancestor that has a Cargo.toml file.",
                        ),
                )
                .arg(
                    Arg::with_name("package")
                        .long("package")
                        .short("p")
                        .takes_value(true)
                        .value_name("PACKAGENAME")
                        .help(
                            "The package name within the workspace to work with. \
                        Defaults to the current package.",
                        ),
                )
                .subcommand(
                    SubCommand::with_name("init")
                        .about("Initializes a devsecret directory for the current crate"),
                )
                .subcommand(
                    SubCommand::with_name("path")
                        .about("Prints the devsecret config path to stdout"),
                ),
        )
        .get_matches();

    let matches = matches
        .subcommand_matches("devsecrets")
        .expect("Must have devsecrets subcommand.");

    let workspace = CargoWorkspace::with_opt_manifest_path(
        matches.value_of_os("manifest-path").map(|p| Path::new(p)),
    )
    .expect("");

    let curr_package = match matches.value_of("package") {
        Some(pkg_name) => workspace.find_package(pkg_name).unwrap(),
        None => workspace.find_default_package(),
    };

    let manifest_dir = &curr_package.manifest_path.parent().unwrap();

    if let Some(_) = matches.subcommand_matches("init") {
        match devsecrets::init_devsecrets_dir_from_manifest_dir(manifest_dir) {
            Ok(dir) => println!("Dir: {}", dir.to_str().unwrap()),
            Err(e) => println!("Unable to init directory: {}", e),
        }
    } else if let Some(_) = matches.subcommand_matches("path") {
        match devsecrets::get_devsecrets_dir_from_manifest_dir(manifest_dir) {
            Ok(dir) => println!("{}", dir.to_str().unwrap()),
            Err(e) => println!("Unable to find devsecrets directory: {:#}", e),
        }
    }
}
