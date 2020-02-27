use clap::{App, AppSettings, Arg, SubCommand};

pub fn build_cli() -> App<'static, 'static> {
    App::new("cargo")
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
                )
                .subcommand(
                    SubCommand::with_name("completions")
                        .about("Generates completions for your shell")
                        .arg(
                            Arg::with_name("SHELL")
                                .required(true)
                                .possible_values(&["zsh", "bash", "fish"])
                                .help("The shell to generate completions for"),
                        ),
                ),
        )
}
