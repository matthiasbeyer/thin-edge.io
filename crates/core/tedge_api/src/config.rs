use std::collections::HashMap;

use serde::Serialize;

/// Generic config that represents what kind of config a plugin wishes to accept
#[derive(Debug, Serialize, PartialEq)]
pub struct ConfigDescription {
    name: String,
    kind: ConfigKind,
    doc: Option<&'static str>,
}

impl ConfigDescription {
    /// Construct a new generic config explanation
    #[must_use]
    pub fn new(name: String, kind: ConfigKind, doc: Option<&'static str>) -> Self {
        Self { name, kind, doc }
    }

    /// Get a reference to the config's documentation.
    #[must_use]
    pub fn doc(&self) -> Option<&'static str> {
        self.doc
    }

    /// Get a reference to the config's kind.
    #[must_use]
    pub fn kind(&self) -> &ConfigKind {
        &self.kind
    }

    /// Set or replace the documentation of this [`Config`]
    #[must_use]
    pub fn with_doc(mut self, doc: Option<&'static str>) -> Self {
        self.doc = doc;
        self
    }

    /// Get the config's name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// How an enum is represented
#[derive(Debug, Serialize, PartialEq)]
pub enum EnumVariantRepresentation {
    /// The enum is represented by a string
    ///
    /// This is the case with unit variants for example
    String(&'static str),
    /// The enum is represented by the value presented here
    Wrapped(Box<ConfigDescription>),
}

/// The kind of enum tagging used by the [`ConfigKind`]
#[derive(Debug, Serialize, PartialEq)]
pub enum ConfigEnumKind {
    /// An internal tag with the given tag name
    Tagged(&'static str),
    /// An untagged enum variant
    Untagged,
}

/// The specific kind a [`Config`] represents
#[derive(Debug, Serialize, PartialEq)]
pub enum ConfigKind {
    /// Config represents a boolean `true`/`false`
    Bool,

    /// Config represents an integer `1, 10, 200, 10_000, ...`
    ///
    /// # Note
    ///
    /// The maximum value that can be represented is between [`i64::MIN`] and [`i64::MAX`]
    Integer,

    /// Config represents a floating point value `1.0, 20.235, 3.1419`
    ///
    /// # Note
    /// Integers are also accepted and converted to their floating point variant
    ///
    /// The maximum value that can be represented is between [`f64::MIN`] and [`f64::MAX`]
    Float,

    /// Config represents a string
    String,

    /// Wrap another config
    ///
    /// This is particularly useful if you want to restrict another kind. The common example is a
    /// `Port` config object which is represented as a `u16` but with an explanation of what it is
    /// meant to represent.
    Wrapped(Box<ConfigDescription>),

    /// Config represents an array of values of the given [`ConfigKind`]
    Array(Box<ConfigDescription>),

    /// Config represents a hashmap of named configurations of the same type
    ///
    /// # Note
    ///
    /// The key is always a [`String`] so this only holds the value config
    HashMap(Box<ConfigDescription>),

    /// Config represents a map of different configurations
    ///
    /// The tuple represent `(field_name, documentation, config_description)`
    Struct(Vec<(&'static str, Option<&'static str>, ConfigDescription)>),

    /// Config represents multiple choice of configurations
    Enum(
        ConfigEnumKind,
        Vec<(
            &'static str,
            Option<&'static str>,
            EnumVariantRepresentation,
        )>,
    ),
}

/// Turn a plugin configuration into a [`Config`] object
///
/// Plugin authors are expected to implement this for their configurations to give users
pub trait AsConfig {
    /// Get a [`Config`] object from the type
    fn as_config() -> ConfigDescription;
}

impl<T: AsConfig> AsConfig for Vec<T> {
    fn as_config() -> ConfigDescription {
        ConfigDescription::new(
            format!("Array of '{}'s", T::as_config().name()),
            ConfigKind::Array(Box::new(T::as_config())),
            None,
        )
    }
}

impl<V: AsConfig> AsConfig for HashMap<String, V> {
    fn as_config() -> ConfigDescription {
        ConfigDescription::new(
            format!("Table of '{}'s", V::as_config().name()),
            ConfigKind::HashMap(Box::new(V::as_config())),
            None,
        )
    }
}

impl<V: AsConfig> AsConfig for HashMap<std::path::PathBuf, V> {
    fn as_config() -> ConfigDescription {
        ConfigDescription::new(
            format!("Table of '{}'s", V::as_config().name()),
            ConfigKind::HashMap(Box::new(V::as_config())),
            None,
        )
    }
}

macro_rules! impl_config_kind {
    ($kind:expr; $name:expr; $doc:expr => $($typ:ty),+) => {
        $(
            impl AsConfig for $typ {
                fn as_config() -> ConfigDescription {
                    ConfigDescription::new({$name}.into(), $kind, Some($doc))
                }
            }
        )+
    };
}

impl_config_kind!(ConfigKind::Integer; "Integer"; "A signed integer with 64 bits" => i64);
impl_config_kind!(ConfigKind::Integer; "Integer"; "An unsigned integer with 64 bits" => u64);

impl_config_kind!(ConfigKind::Integer; "Integer"; "A signed integer with 32 bits" => i32);
impl_config_kind!(ConfigKind::Integer; "Integer"; "An unsigned integer with 32 bits" => u32);

impl_config_kind!(ConfigKind::Integer; "Integer"; "A signed integer with 16 bits" => i16);
impl_config_kind!(ConfigKind::Integer; "Integer"; "An unsigned integer with 16 bits" => u16);

impl_config_kind!(ConfigKind::Integer; "Integer"; "A signed integer with 8 bits" => i8);
impl_config_kind!(ConfigKind::Integer; "Integer"; "An unsigned integer with 8 bits" => u8);

impl_config_kind!(ConfigKind::Float; "Float"; "A floating point value with 64 bits" => f64);
impl_config_kind!(ConfigKind::Float; "Float"; "A floating point value with 32 bits" => f32);

impl_config_kind!(ConfigKind::Bool; "Boolean"; "A boolean" => bool);
impl_config_kind!(ConfigKind::String; "String"; "An UTF-8 string" => String);

impl_config_kind!(ConfigKind::String; "String"; "A socket address" => std::net::SocketAddr);
impl_config_kind!(ConfigKind::String; "String"; "An IPv4 socket address" => std::net::SocketAddrV4);
impl_config_kind!(ConfigKind::String; "String"; "An IPv6 socket address" => std::net::SocketAddrV6);

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::config::{AsConfig, ConfigDescription, ConfigKind};

    #[test]
    fn verify_correct_config_kinds() {
        assert!(matches!(
            Vec::<f64>::as_config(),
            ConfigDescription {
                doc: None,
                kind: ConfigKind::Array(x),
                ..
            } if matches!(x.kind(), ConfigKind::Float)
        ));

        let complex_config = HashMap::<String, Vec<HashMap<String, String>>>::as_config();
        println!("Complex config: {:#?}", complex_config);

        assert!(
            matches!(complex_config.kind(), ConfigKind::HashMap(map) if matches!(map.kind(), ConfigKind::Array(arr) if matches!(arr.kind(), ConfigKind::HashMap(inner_map) if matches!(inner_map.kind(), ConfigKind::String))))
        );
    }
}
