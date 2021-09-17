mod config;
mod error;
mod health;
mod message;
mod options;
mod parent;
mod redirect;
mod relay;

use privsep_derive::Privsep;
use std::time::Duration;
pub use {
    error::Error,
    options::Options,
    privsep::process::{Child, Parent},
};

/// Privsep processes.
#[derive(Debug, Privsep)]
#[username = "nobody"]
pub enum Privsep {
    /// Parent process.
    Parent,
    /// Health Check Engine
    #[connect(Relay, Redirect)]
    Health,
    /// Packet Filter Engine
    Redirect,
    /// L7 Relays
    Relay,
}

/// Default configuration path.
const RELAYD_CONFIG: &str = "/etc/relayd.conf";
/// Default control socket path.
const RELAYD_SOCKET: &str = "/var/run/relayd.sock";
/// Default relayd server name.
#[allow(unused)]
const RELAYD_SERVERNAME: &str = "relayd-rs";

/// Default health check timeout.
const CHECK_TIMEOUT: Duration = Duration::from_millis(200);
/// Default health check interval.
const CHECK_INTERVAL: Duration = Duration::from_secs(10);

/// Default PF socket.
#[allow(unused)]
const PF_SOCKET: &str = "/dev/pf";
/// Default relayd PF anchor.
#[allow(unused)]
const PF_RELAYD_ANCHOR: &str = "relayd";
