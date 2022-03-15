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
    #[clap(name = "run")]
    Run { config: PathBuf },

    #[clap(name = "validate-config")]
    ValidateConfig { config: PathBuf },
}
