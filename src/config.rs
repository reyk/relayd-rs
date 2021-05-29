mod expand;
mod parser;

use crate::error::Error;
use expand::config_expand;
use nom::{error::convert_error, Finish};
use parser::config_parser;
use serde_derive::{Deserialize, Serialize};
use serde_with::{serde_as, DurationMilliSeconds, DurationSeconds};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::fs;

#[serde_as]
#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    /// The interval in seconds at which the hosts will be checked.
    #[serde_as(as = "DurationSeconds<u64>")]
    interval: Duration,
    /// Create a control socket at path.
    socket: PathBuf,
    /// The global timeout in milliseconds for checks.
    #[serde_as(as = "DurationMilliSeconds<u64>")]
    timeout: Duration,

    redirects: Vec<Redirect>,
    relays: Vec<Relay>,
    protocols: Vec<Protocol>,
    tables: Vec<Table>,
    // Currently not supported:
    //agentx: not supported
    //log: TODO
    //prefork: not supported
}

impl Default for Config {
    fn default() -> Self {
        Self {
            interval: crate::CHECK_INTERVAL,
            socket: PathBuf::from(crate::RELAYD_SOCKET),
            timeout: crate::CHECK_TIMEOUT,
            redirects: Default::default(),
            relays: Default::default(),
            protocols: Default::default(),
            tables: Default::default(),
        }
    }
}

impl Config {
    pub async fn load<P: AsRef<Path> + ?Sized>(
        path: &P,
        variables: Variables,
    ) -> Result<Self, Error> {
        let input = fs::read_to_string(path).await?;
        Self::parse(input, variables)
    }

    pub fn parse<S: AsRef<str>>(input: S, variables: Variables) -> Result<Self, Error> {
        let input = input.as_ref();
        let (_, input) = config_expand(input, variables)
            .finish()
            .map_err(|err| Error::ParserError(convert_error(input, err)))?;
        let input = input.as_ref();
        config_parser(input)
            .finish()
            .map_err(|err| Error::ParserError(convert_error(input, err)))
            .map(|(_, o)| o)
    }
}

#[derive(Debug, Default)]
pub struct Variable {
    key: String,
    value: String,
}

impl From<(String, String)> for Variable {
    fn from((key, value): (String, String)) -> Self {
        Self { key, value }
    }
}

impl ToString for Variable {
    fn to_string(&self) -> String {
        format!("{}=\"{}\"", self.key, self.value)
    }
}

pub type Variables = HashMap<String, String>;

/// General relayd object Id.
pub type Id = u32;

/// Table of hosts.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Table {
    /// Symbolic name of the table.
    name: String,
    /// Target host pool.
    hosts: Vec<Host>,
    /// Whether to disable the table.
    disabled: bool,
}

/// Target host pool and definitions.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Host {
    /// FQDN or IP address of the host.
    name: String,
    /// Time-to-live value in the IP headers for host checks.
    ip_ttl: Option<u8>,
    /// Optional parent Id to inherit the state from.
    parent: Option<Id>,
    /// Optional route priority.
    priority: Option<u8>,
    /// Retry tolerance for host checks.
    retry: usize,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Redirect {
    name: String,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Relay {
    name: String,
}

#[derive(Debug, Deserialize, Serialize)]
enum ProtocolType {
    Tcp,
    Http,
    Dns,
}

impl Default for ProtocolType {
    fn default() -> Self {
        Self::Tcp
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Protocol {
    name: String,
    typ: ProtocolType,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_example() {
        let _guard = privsep_log::sync_logger(
            "config",
            privsep_log::Config {
                foreground: true,
                filter: Some("trace".to_string()),
            },
        )
        .unwrap();
        let config = include_bytes!("../examples/relayd.conf");

        Config::parse(
            &String::from_utf8(config.to_vec()).unwrap(),
            Default::default(),
        )
        .unwrap();
    }
}
