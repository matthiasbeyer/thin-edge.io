# tedge-cli

This is the CLI implementation of thin-edge.io

## What this does

This uses the [tedge_core] crate and builds a CLI interface upon it.
It does so by implementing a CLI interface with [clap](https://docs.rs/clap/),
listing the available [PluginBuilder](tedge_api::PluginBuilder) implementations
and registering them in the [TedgeApplication](tedge_core::TedgeApplication) and
then booting that application.

It also provides convenience interfaces to the user.
For example it provides a command to only validate the configuration of the
plugins, but not running anything.
It can also be used to list the available plugin kinds or fetch some explanation
on how these plugin kinds can be configured.

For more details, have a look at the output of `tedge-cli --help`.


## Note

Using `Ctrl-C` will request a shutdown from the application, giving all plugins
the opportunity to clean up their resources.

Using `Ctrl-C` a second time will kill the plugins and force application
shutdown, even though plugins did not yet shutdown cleanly.

