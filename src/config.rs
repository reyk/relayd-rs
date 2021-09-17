mod expand;
mod parser;

use crate::error::Error;
use expand::config_expand;
use nom::{error::convert_error, Finish};
use parser::config_parser;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DurationMilliSeconds, DurationSeconds};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU32, Ordering},
    time::Duration,
};
use tokio::fs;

#[serde_as]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    /// Privsep and log configuration.
    #[serde(skip)]
    pub privsep: privsep::Config,

    /// The interval in seconds at which the hosts will be checked.
    #[serde_as(as = "DurationSeconds<u64>")]
    pub interval: Duration,
    /// Create a control socket at path.
    pub socket: PathBuf,
    /// The global timeout in milliseconds for checks.
    #[serde_as(as = "DurationMilliSeconds<u64>")]
    pub timeout: Duration,

    pub redirects: Vec<Redirect>,
    pub relays: Vec<Relay>,
    pub protocols: Vec<Protocol>,
    pub tables: Vec<Table>,
    // Currently not supported:
    //agentx: not supported
    //log: TODO
    //prefork: not supported
}

impl Default for Config {
    fn default() -> Self {
        Self {
            privsep: Default::default(),
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

/// Counter of tables.
pub static TABLE_ID: AtomicU32 = AtomicU32::new(1);

/// Counter of hosts.
pub static HOST_ID: AtomicU32 = AtomicU32::new(1);

/// Counter of redirects.
pub static REDIRECT_ID: AtomicU32 = AtomicU32::new(1);

/// Counter of relays.
pub static RELAY_ID: AtomicU32 = AtomicU32::new(1);

/// Counter of protocols.
pub static PROTOCOL_ID: AtomicU32 = AtomicU32::new(1);

/// Table of hosts.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Table {
    /// Id.
    id: Id,
    /// Symbolic name of the table.
    name: String,
    /// Target host pool.
    hosts: Vec<Host>,
    /// Whether to disable the table.
    disabled: bool,
}

impl Table {
    fn new() -> Self {
        Self {
            id: TABLE_ID.fetch_add(1, Ordering::SeqCst),
            ..Default::default()
        }
    }
}

/// Target host pool and definitions.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Host {
    /// Id.
    id: Id,
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

impl Host {
    fn new() -> Self {
        Self {
            id: HOST_ID.fetch_add(1, Ordering::SeqCst),
            ..Default::default()
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Redirect {
    /// Id.
    id: Id,
    /// Symbolic name of the redirect.
    name: String,
}

impl Redirect {
    fn new() -> Self {
        Self {
            id: REDIRECT_ID.fetch_add(1, Ordering::SeqCst),
            ..Default::default()
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Relay {
    /// Id.
    id: Id,
    /// Symbolic name of the relay.
    name: String,
}

impl Relay {
    fn new() -> Self {
        Self {
            id: RELAY_ID.fetch_add(1, Ordering::SeqCst),
            ..Default::default()
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
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

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Protocol {
    /// Id.
    id: Id,
    /// Symbolic name of the protocol.
    name: String,
    /// Protocol or application type.
    typ: ProtocolType,
}

impl Protocol {
    fn new() -> Self {
        Self {
            id: PROTOCOL_ID.fetch_add(1, Ordering::SeqCst),
            ..Default::default()
        }
    }
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
