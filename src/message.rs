use crate::config::{Config, Id};
use derive_more::Display;
use privsep::imsg::Message;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

#[derive(Clone, Copy, Debug, Display)]
#[repr(u32)]
pub enum Type {
    /// Send configuration
    Config = Message::RESERVED + 1,
    /// Start process operation
    Start,
    /// Host is up
    HostUp,
    /// Host is down
    HostDown,
    /// Unknown message
    Unknown,
}

impl Type {
    pub const CONFIG: u32 = Self::Config as u32;
    pub const START: u32 = Self::Start as u32;
    pub const HOST_UP: u32 = Self::HostUp as u32;
    pub const HOST_DOWN: u32 = Self::HostDown as u32;
}

impl From<u32> for Type {
    fn from(id: u32) -> Self {
        match id {
            Type::CONFIG => Self::Config,
            Type::START => Self::Start,
            Type::HOST_UP => Self::HostUp,
            Type::HOST_DOWN => Self::HostDown,
            _ => Self::Unknown,
        }
    }
}

impl From<Type> for Message {
    fn from(typ: Type) -> Self {
        Self::from(typ as u32)
    }
}

/// Internal message data
#[derive(Debug, Deserialize, Serialize)]
pub enum Data<'a> {
    Config(Cow<'a, Config>),
    Host(Id),
    None,
}

impl<'a> From<&'a Config> for Data<'a> {
    fn from(config: &'a Config) -> Self {
        Self::Config(Cow::Borrowed(config))
    }
}

impl From<()> for Data<'_> {
    fn from(_none: ()) -> Self {
        Self::None
    }
}
