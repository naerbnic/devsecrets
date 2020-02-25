use std::path::Path;

mod cli;
mod workspace;

fn main() {
    env_logger::init();
    let matches = cli::build_cli().get_matches();

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
    } else if let Some(matches) = matches.subcommand_matches("completions") {
        let shell = matches.value_of("SHELL").unwrap();
        cli::build_cli().gen_completions_to(
            "cargo",
            shell.parse().unwrap(),
            &mut std::io::stdout(),
        );
    }
}
