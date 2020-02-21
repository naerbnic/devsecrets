use clap::{App, AppSettings, Arg, SubCommand};
use std::io;
use std::path::{Path, PathBuf};

fn file_exists(file_path: impl AsRef<Path>) -> io::Result<bool> {
    match std::fs::metadata(file_path) {
        Ok(metadata) => Ok(metadata.is_file()),
        Err(e) => match e.kind() {
            io::ErrorKind::NotFound => Ok(false),
            _ => Err(e),
        },
    }
}

fn find_crate_root(curr_dir: impl AsRef<Path>) -> anyhow::Result<Option<PathBuf>> {
    for ancestor in curr_dir.as_ref().ancestors() {
        let manifest_file_path = ancestor.join("Cargo.toml");
        if file_exists(&manifest_file_path)? {
            return Ok(Some(ancestor.to_path_buf()));
        }
    }

    return Ok(None);
}

fn find_crate_root_from_wd() -> anyhow::Result<PathBuf> {
    match find_crate_root(std::env::current_dir()?)? {
        Some(path) => Ok(path),
        None => anyhow::bail!("Could not find crate root."),
    }
}

fn main() {
    let matches = App::new("cargo devsecrets")
        .bin_name("cargo devsecrets")
        .author(clap::crate_authors!())
        .version(clap::crate_version!())
        .setting(AppSettings::SubcommandRequired)
        .arg(
            Arg::with_name("crate_path")
                .short("p")
                .long("crate_path")
                .takes_value(true)
                .value_name("CRATEDIR")
                .help(
                    "The path to the crate to work with. If not set, \
                     selects the closest ancestor that has a Cargo.toml file.",
                ),
        )
        .subcommand(
            SubCommand::with_name("init")
                .about("Initializes a devsecret directory for the current crate"),
        )
        .subcommand(SubCommand::with_name("path").about("Prints the devsecret path to stdout"))
        .get_matches();

    let crate_path = match matches.value_of_os("crate_path") {
        Some(path_str) => {
            let path: &Path = path_str.as_ref();
            path.to_path_buf()
        }
        None => find_crate_root_from_wd().expect("Problem finding crate root."),
    };

    println!("Crate path: {:?}", crate_path);

    if let Some(matches) = matches.subcommand_matches("init") {
        match devsecrets::init_devsecrets_dir_from_manifest_dir(&crate_path) {
            Ok(dir) => println!("Dir: {}", dir.to_str().unwrap()),
            Err(e) => println!("Unable to init directory: {}", e),
        }
    } else if let Some(matches) = matches.subcommand_matches("path") {
        match devsecrets::get_devsecrets_dir_from_manifest_dir(&crate_path) {
            Ok(dir) => println!("{}", dir.to_str().unwrap()),
            Err(e) => println!("Unable to find devsecrets directory: {:#}", e),
        }
        
    }
}
