use crate::config::{Config, Id};
use privsep::imsg::Message;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

/// Send configuration
pub const CONFIG: u32 = Message::RESERVED + 1;

/// Start process operation
pub const START: u32 = Message::RESERVED + 2;

/// Host is up
pub const HOST_UP: u32 = Message::RESERVED + 3;

/// Host is down
pub const HOST_DOWN: u32 = Message::RESERVED + 4;

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
