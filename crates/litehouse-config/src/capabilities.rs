//! Capabilities for Litehouse plugins.
//!
//! This module defines the capabilities that can be granted to plugins, allowing them to interact with the system and external resources in a controlled manner.

use std::fmt;
use std::str::FromStr;
use serde::{Serialize, Deserialize};
use thiserror::Error;

/// Represents the different capabilities that can be granted to plugins.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Capability {
    /// Allows the plugin to start an HTTP server on the specified port.
    #[serde(rename = "http-server")]
    HttpServer(u16),
    /// Allows the plugin to make HTTP requests to the specified URL.
    #[serde(rename = "http-client")]
    HttpClient(String),
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
            ["http-server", port] => port.parse().map(Capability::HttpServer).map_err(|_| CapabilityParseError::InvalidPort),
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
