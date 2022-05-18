/// One or many of something
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
