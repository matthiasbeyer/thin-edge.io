/// One or many of some configuration value
///
/// This is a helper type for specifying that a value might be there one time or many times.
///
/// # Example
///
/// If a plugin is able to send out messages to one or many other plugins, its configuration could
/// look like this:
///
/// ```rust
/// # extern crate tedge_lib;
/// use tedge_lib::config::Address;
/// use tedge_lib::config::OneOrMany;
///
/// #[derive(Debug, serde::Deserialize, tedge_api::Config)]
/// struct MyConfig {
///     // Whom to send data to
///     targets: OneOrMany<Address>,
/// }
/// ```
///
/// Now users of this plugin can specify either one or many addresses for the `targets` key:
///
/// ```toml
/// targets = "other_plugin" # or:
/// targets = [ "one_other", "another", "a_third" ]
/// ```
///
/// Using this configuration helper type has also the benefit that it already has a
/// [AsConfig](tedge_api::AsConfig) implementation and the developer does not have to concern
/// themselves with it.
///
#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
pub enum OneOrMany<T> {
    /// A setting with only one
    One(T),

    /// A setting with a list
    Many(Vec<T>),
}

impl<T> OneOrMany<T> {
    pub fn into_vec(self) -> Vec<T> {
        match self {
            OneOrMany::One(t) => vec![t],
            OneOrMany::Many(v) => v,
        }
    }
}

impl<T> tedge_api::AsConfig for OneOrMany<T>
where
    T: tedge_api::AsConfig,
{
    fn as_config() -> tedge_api::ConfigDescription {
        tedge_api::ConfigDescription::new(
            format!("Either one or many '{}'", T::as_config().name()),
            tedge_api::ConfigKind::Enum(
                tedge_api::config::ConfigEnumKind::Untagged,
                vec![
                    (
                        "One",
                        None,
                        tedge_api::config::EnumVariantRepresentation::Wrapped(Box::new(
                            T::as_config(),
                        )),
                    ),
                    (
                        "Many",
                        None,
                        tedge_api::config::EnumVariantRepresentation::Wrapped(Box::new(
                            Vec::<T>::as_config(),
                        )),
                    ),
                ],
            ),
            Some("One or many elements of something"),
        )
    }
}
