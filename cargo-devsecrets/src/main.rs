use clap::{App, AppSettings, Arg, SubCommand};
use std::path::Path;

mod workspace;

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
                        .long("manifest-path")
                        .takes_value(true)
                        .value_name("MANIFESTFILE")
                        .help("The path to the crate manifest to work with."),
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

    let workspace = workspace::CargoWorkspace::with_opt_manifest_path(
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
