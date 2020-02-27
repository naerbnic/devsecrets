use std::path::{Path, PathBuf};

mod cli;
mod workspace;

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
        match init_devsecrets_dir_from_manifest_dir(manifest_dir) {
            Ok(dir) => println!("Dir: {}", dir.to_str().unwrap()),
            Err(e) => println!("Unable to init directory: {}", e),
        }
    } else if let Some(_) = matches.subcommand_matches("path") {
        match get_devsecrets_dir_from_manifest_dir(manifest_dir) {
            Ok(Some(dir)) => println!("{}", dir.to_str().unwrap()),
            Ok(None) => println!("Devsecrets dir has not be initialized. Run init."),
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
