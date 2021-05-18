mod expand;
mod parser;

use crate::error::Error;
use expand::config_expand;
use nom::{error::convert_error, Finish};
use parser::config_parser;
use serde_derive::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path};
use tokio::fs;

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Config {
    redirects: Vec<Redirect>,
    relays: Vec<Relay>,
    protocols: Vec<Protocol>,
    tables: Vec<Table>,
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

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Table {
    name: String,
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
