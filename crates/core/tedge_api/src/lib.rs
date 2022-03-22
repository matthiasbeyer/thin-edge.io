#![cfg_attr(test, deny(
    missing_docs,
    missing_debug_implementations,
    unreachable_pub,
    unsafe_code,
))]
#![doc = include_str!("../README.md")]

/// All the parts required to write a plugin
pub mod plugin;
pub use plugin::{PluginDirectory, Plugin, PluginBuilder, PluginConfiguration};

/// Addresses allow plugins to exchange messages
pub mod address;
pub use address::Address;

/// Known error types
pub mod error;
pub use error::PluginError;

/// Predefined messages
pub mod message;
pub use message::CoreMessages;

#[doc(hidden)]
mod _internal {
    pub use futures::future::BoxFuture;
}
