use std::collections::HashMap;
use std::collections::HashSet;

use tedge_core::TedgeApplicationBuilder;

/// Helper type for registering PluginBuilder instances and doc-printing functions
pub struct Registry {
    pub app_builder: TedgeApplicationBuilder,
    pub plugin_kinds: HashSet<String>,
    pub doc_printers: HashMap<String, Box<dyn FnOnce() -> Result<(), miette::Error>>>,
}

impl Registry {
    pub fn new() -> Self {
        Registry {
            app_builder: tedge_core::TedgeApplication::builder(),
            plugin_kinds: HashSet::new(),
            doc_printers: HashMap::new(),
        }
    }
}

#[macro_export]
macro_rules! register_plugin {
    (if feature $cfg:tt is enabled then
     register on $registry:ident
     builder of type $pluginbuilder:ty,
     with instance $pbinstance:expr
    ) => {{
        cfg_if::cfg_if! {
            if #[cfg(feature = $cfg)] {
                let kind_name: &'static str = <$pluginbuilder as tedge_api::PluginBuilder<tedge_core::PluginDirectory>>::kind_name();
                info!(%kind_name, "Registering plugin builder");
                let mut registry = $registry;
                if !registry.plugin_kinds.insert(kind_name.to_string()) {
                    miette::bail!("Plugin kind '{}' was already registered, cannot register!", kind_name)
                }

                let kind_name_str = kind_name.to_string();
                registry.doc_printers.insert(kind_name.to_string(), Box::new(move || {
                    use std::io::Write;
                    use miette::IntoDiagnostic;
                    use pretty::Arena;

                    let mut stdout = std::io::stdout();
                    if let Some(config_desc) = <$pluginbuilder as tedge_api::PluginBuilder<tedge_core::PluginDirectory>>::kind_configuration() {
                        let terminal_width = term_size::dimensions().map(|(w, _)| w).unwrap_or(80);
                        let arena = Arena::new();

                        let rendered_doc = tedge_cli::config::as_terminal_doc(&config_desc, &arena);

                        let mut output = String::new();
                        rendered_doc.render_fmt(terminal_width, &mut output).into_diagnostic()?;

                        writeln!(stdout, " ----- Documentation for plugin '{}'", kind_name_str)
                                .into_diagnostic()?;

                        writeln!(stdout, "{}", output).into_diagnostic()?;
                    } else {
                        let msg = format!(" Documentation for plugin '{}' is unavailable", kind_name);
                        writeln!(stdout, "{}", nu_ansi_term::Color::Red.bold().paint(msg))
                            .into_diagnostic()?;
                    }
                    Ok(())
                }));

                Registry {
                    app_builder: registry.app_builder.with_plugin_builder($pbinstance),
                    plugin_kinds: registry.plugin_kinds,
                    doc_printers: registry.doc_printers,
                }
            } else {
                tracing::trace!("Not supporting plugins of type {}", std::stringify!($pluginbuilder));
                $registry
            }
        }
    }}
}
