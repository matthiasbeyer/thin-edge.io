//! Helper types for making configuration parsing more comfortable for the developer
//!
//! This module contains types and functionality for defining configuration structs with less
//! overhead.
//!
//! For example, one might write a configuration struct like this:
//!
//!
//! ```rust
//! #[derive(Debug, serde::Deserialize, tedge_api::Config)]
//! struct MyConfig {
//!     // The address to send data to
//!     target_address: String,
//!
//!     // The interval to send data in (in milliseconds)
//!     interval: std::num::NonZeroU64,
//! }
//! ```
//!
//! To define that the configuration of a plugin has a `target_address` (which is a `String`) and a
//! `interval` which is a non-zero unsigned 64-bit integer.
//!
//! With types from this module, this configuration gets easier to write and automatically gets
//! more documentation (via the `tedge_api::Config` mechanisms).
//!
//! The above would be written like this:
//!
//! ```rust
//! # extern crate tedge_lib;
//! #[derive(Debug, serde::Deserialize, tedge_api::Config)]
//! struct MyConfig {
//!     // The address to send data to
//!     target_address: tedge_lib::config::Address,
//!
//!     // The interval to send data in
//!     interval: tedge_lib::config::Humantime,
//! }
//! ```
//!
//! By using the [Address](crate::config::Address) type, we get user documentation on how an
//! address might look like. Using the [Humantime](crate::config::Humantime) type, we get
//! human-readable time configuration (e.g. "5 mins") plus nice user documentation for our
//! configuration type.
//!

mod address;
pub use crate::config::address::Address;

mod humantime;
pub use crate::config::humantime::Humantime;

mod one_or_many;
pub use one_or_many::OneOrMany;
