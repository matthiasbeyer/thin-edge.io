use std::path::PathBuf;

#[derive(Debug, clap::Parser)]
#[clap(
    name = clap::crate_name!(),
    version = clap::crate_version!(),
    about = clap::crate_description!()
)]
pub(crate) struct Cli {
    #[clap(short, long)]
    pub(crate) verbose: bool,

    #[clap(short, long)]
    pub(crate) debug: bool,

    #[clap(subcommand)]
    pub(crate) command: CliCommand,
}

#[derive(Debug, clap::Subcommand)]
pub(crate) enum CliCommand {
    /// Run thin-edge with the passed configuration
    #[clap(name = "run")]
    Run { config: PathBuf },

    /// Validate the passed configuration
    #[clap(name = "validate-config")]
    ValidateConfig { config: PathBuf },

    /// Print the supported plugin kinds
    #[clap(name = "get-plugin-kinds")]
    GetPluginKinds,
}
