mod error;
mod health;
mod parent;
mod redirect;
mod relay;

use privsep_derive::Privsep;
pub use {
    error::Error,
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
