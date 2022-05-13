use std::path::PathBuf;

#[derive(Debug, clap::Parser)]
#[clap(
    name = clap::crate_name!(),
    version = clap::crate_version!(),
    about = clap::crate_description!()
)]
pub(crate) struct Cli {
    /// Enable logging
    ///
    /// If not specified, only WARN and ERROR messages will be logged.
    /// If set to "off" or "false", no logging will be done at all.
    ///
    /// This setting overwrites the logging specification set via the RUST_LOG env variable.
    #[clap(short, long, arg_enum)]
    pub(crate) logging: Option<LoggingSpec>,

    /// Enable chrome compatible tracing output
    ///
    /// If set, chrome-compatible tracing output will be written to the file specified.
    #[clap(long)]
    pub(crate) chrome_logging: Option<PathBuf>,

    /// Enable tracy compatible tracing output
    ///
    /// If set, "tracy" compatible tracing output will be produced
    #[clap(long)]
    pub(crate) tracy_logging: bool,

    #[clap(subcommand)]
    pub(crate) command: CliCommand,
}

#[derive(Copy, Clone, Debug, clap::ArgEnum)]
pub(crate) enum LoggingSpec {
    False,
    Off,
    Trace,
    Debug,
    Info,
    Warn,
    Error,
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

    /// Print the documentation of the configuration of the available plugins
    #[clap(name = "doc")]
    Doc {
        /// Print the doc only for this plugin
        plugin_name: Option<String>,
    },
}
