//! Capabilities for Litehouse plugins.
//!
//! This module defines the capabilities that can be granted to plugins, allowing them to interact with the system and external resources in a controlled manner.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use thiserror::Error;

/// Represents the different capabilities that can be granted to plugins.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Capability {
    /// Allows the plugin to start an HTTP server on the specified port.
    HttpServer(u16),
    /// Allows the plugin to make HTTP requests to the specified URL.
    HttpClient(String),
}

impl Serialize for Capability {
    fn serialize<S>(&self, serializer: S) -> std::prelude::v1::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let string = self.to_string();
        serializer.serialize_str(&string)
    }
}

impl<'de> Deserialize<'de> for Capability {
    fn deserialize<D>(deserializer: D) -> std::prelude::v1::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

impl fmt::Display for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Capability::HttpServer(port) => write!(f, "http-server:{}", port),
            Capability::HttpClient(url) => write!(f, "http-client:{}", url),
        }
    }
}

impl FromStr for Capability {
    type Err = CapabilityParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.splitn(2, ':').collect();
        match parts.as_slice() {
            ["http-server", port] => port
                .parse()
                .map(Capability::HttpServer)
                .map_err(|_| CapabilityParseError::InvalidPort),
            ["http-client", url] => Ok(Capability::HttpClient(url.to_string())),
            _ => Err(CapabilityParseError::InvalidFormat),
        }
    }
}

/// Errors that can occur when parsing a string into a `Capability`.
#[derive(Debug, Error)]
pub enum CapabilityParseError {
    #[error("invalid capability format")]
    InvalidFormat,
    #[error("invalid port number")]
    InvalidPort,
}
