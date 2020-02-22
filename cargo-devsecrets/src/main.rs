use cargo_metadata::{Metadata, Package};
use clap::{App, AppSettings, Arg, SubCommand};
use serde::Deserialize;
use std::path::{Path, PathBuf};

fn find_crate_root(curr_dir: impl AsRef<Path>) -> anyhow::Result<Option<PathBuf>> {
    let output = std::process::Command::new(std::env::var("CARGO").unwrap())
        .arg("locate-project")
        .current_dir(curr_dir.as_ref())
        .output()?;
    #[derive(Deserialize)]
    struct LocateProjectOutput {
        root: String,
    }

    let LocateProjectOutput { root } =
        serde_json::from_slice::<LocateProjectOutput>(&output.stdout)?;
    Ok(Some(PathBuf::from(root)))
}

fn find_crate_root_from_wd() -> anyhow::Result<PathBuf> {
    match find_crate_root(std::env::current_dir()?)? {
        Some(path) => Ok(path),
        None => anyhow::bail!("Could not find crate root."),
    }
}

fn find_package_with_name<'a>(metadata: &'a Metadata, name: &str) -> Option<&'a Package> {
    metadata.packages.iter().find(|p| p.name == name)
}

fn find_package_from_manifest_dir<'a>(
    metadata: &'a Metadata,
    manifest_dir: impl AsRef<Path>,
) -> Option<&'a Package> {
    let manifest_dir = manifest_dir.as_ref();
    metadata
        .packages
        .iter()
        .find(|p| p.manifest_path == manifest_dir)
}

fn find_curr_package<'a>(metadata: &'a Metadata, name_opt: Option<&str>) -> Option<&'a Package> {
    match name_opt {
        Some(name) => find_package_with_name(metadata, name),
        None => find_package_from_manifest_dir(metadata, find_crate_root_from_wd().unwrap()),
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
                        Defaults to the current package."
                    )
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

    let mut cmd = cargo_metadata::MetadataCommand::new();
    if let Some(path) = matches.value_of("manifest-path") {
        cmd.manifest_path(path);
    }
    cmd.no_deps();
    let metadata = cmd.exec().unwrap();

    let curr_package = find_curr_package(&metadata, matches.value_of("package")).unwrap();

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
