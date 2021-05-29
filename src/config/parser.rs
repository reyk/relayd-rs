use crate::config::{Config, Host, Protocol, ProtocolType, Redirect, Relay, Table};
use nom::{
    branch::alt,
    bytes::complete::{tag, take_until, take_while1},
    character::complete::{char, digit1, multispace0},
    combinator::{all_consuming, eof, map, map_res, not, opt, peek, recognize},
    error::VerboseError,
    multi::{many0, many_till},
    sequence::{delimited, pair, preceded, separated_pair, tuple},
    IResult,
};
use privsep_log::debug;
use std::{path::PathBuf, time::Duration};

pub(super) type CResult<'a, T> = IResult<&'a str, T, VerboseError<&'a str>>;

enum Section {
    // Global configuration.
    Interval(Duration),
    Socket(PathBuf),
    Timeout(Duration),

    // Other sections.
    Table(Table),
    Redirect(Redirect),
    Relay(Relay),
    Protocol(Protocol),
    Ignore,
}

fn section(s: &str) -> CResult<'_, Section> {
    preceded(
        not(eof),
        alt((
            map(interval, |d| {
                debug!("interval {:?}", d);
                Section::Interval(d)
            }),
            map(socket, |p| {
                debug!("socket {:?}", p);
                Section::Socket(p)
            }),
            map(timeout, |d| {
                debug!("timeout {:?}", d);
                Section::Timeout(d)
            }),
            map(table, |t| {
                debug!("{:?}", t);
                Section::Table(t)
            }),
            map(redirect, |r| {
                debug!("{:?}", r);
                Section::Redirect(r)
            }),
            map(relay, |r| {
                debug!("{:?}", r);
                Section::Relay(r)
            }),
            map(protocol, |p| {
                debug!("{:?}", p);
                Section::Protocol(p)
            }),
            map(comment, |c| {
                debug!("#{}", c);
                Section::Ignore
            }),
            map(nl, |_| Section::Ignore),
        )),
    )(s)
}

fn interval(s: &str) -> CResult<'_, Duration> {
    map(
        separated_pair(tag("interval"), nl, integer),
        |(_, seconds)| Duration::from_secs(seconds),
    )(s)
}

fn socket(s: &str) -> CResult<'_, PathBuf> {
    map(separated_pair(tag("socket<"), nl, string), |(_, path)| {
        PathBuf::from(path)
    })(s)
}

fn timeout(s: &str) -> CResult<'_, Duration> {
    map(
        separated_pair(tag("timeout"), nl, integer),
        |(_, seconds)| Duration::from_millis(seconds),
    )(s)
}

fn host(s: &str) -> CResult<'_, Host> {
    map(tuple((sep, string, sep)), |(_, name, _)| Host {
        name: name.to_string(),
        ..Default::default()
    })(s)
}

fn table_options(s: &str) -> CResult<'_, Vec<Host>> {
    delimited(
        char('{'),
        map(many_till(host, peek(char('}'))), |(hosts, _)| hosts),
        char('}'),
    )(s)
}

fn table(s: &str) -> CResult<'_, Table> {
    map(
        tuple((
            tag("table"),
            nl,
            delimited(char('<'), string, char('>')),
            nl,
            opt(pair(tag("disable"), nl)),
            table_options,
            line,
        )),
        |(_, _, name, _n, disable, hosts, _)| Table {
            name: name.to_string(),
            disabled: disable.is_some(),
            hosts,
        },
    )(s)
}

fn redirect(s: &str) -> CResult<'_, Redirect> {
    map(
        tuple((
            tag("redirect"),
            nl,
            quoted,
            nl,
            char('{'),
            take_until("}"),
            line,
        )),
        |(_, _, name, _, _, _, _)| Redirect {
            name: name.to_string(),
        },
    )(s)
}

fn relay(s: &str) -> CResult<'_, Relay> {
    map(
        tuple((
            tag("relay"),
            nl,
            quoted,
            nl,
            char('{'),
            take_until("}"),
            line,
        )),
        |(_, _, name, _, _, _, _)| Relay {
            name: name.to_string(),
        },
    )(s)
}

fn protocol_type(s: &str) -> CResult<'_, ProtocolType> {
    alt((
        map(tag("tcp"), |_| ProtocolType::Tcp),
        map(tag("http"), |_| ProtocolType::Http),
        map(tag("dns"), |_| ProtocolType::Dns),
    ))(s)
}

fn protocol_option(s: &str) -> CResult<'_, ()> {
    alt((
        map(preceded(tag("return"), line), |_| {
            debug!("return");
        }),
        map(pair(alt((tag("block"), tag("match"))), line), |(t, _)| {
            debug!("{}", t);
        }),
        map(preceded(tag("tcp"), line), |_| {
            debug!("tcp");
        }),
        map(preceded(tag("tls"), line), |_| ()),
        map(comment, |_| ()),
        map(nl, |_| ()),
    ))(s)
}

fn protocol_options(s: &str) -> CResult<'_, ()> {
    delimited(
        char('{'),
        map(
            many_till(protocol_option, peek(char('}'))),
            |_options: (Vec<()>, _)| {
                // TODO
            },
        ),
        char('}'),
    )(s)
}

fn protocol(s: &str) -> CResult<'_, Protocol> {
    map(
        tuple((
            opt(protocol_type),
            nl,
            tag("protocol"),
            nl,
            quoted,
            nl,
            protocol_options,
            line,
        )),
        |(typ, _, _, _, name, _, _, _)| Protocol {
            name: name.to_string(),
            typ: typ.unwrap_or_default(),
        },
    )(s)
}

fn allowed_in_string(ch: char) -> bool {
    ch.is_ascii_alphanumeric()
        || (ch.is_ascii_punctuation()
            && ch != '('
            && ch != ')'
            && ch != '{'
            && ch != '}'
            && ch != '<'
            && ch != '>'
            && ch != '!'
            && ch != '='
            && ch != '#'
            && ch != ','
            && ch != '/')
}

pub(super) fn string(s: &str) -> CResult<'_, &str> {
    take_while1(allowed_in_string)(s)
}

pub(super) fn line(s: &str) -> CResult<'_, &str> {
    take_until("\n")(s).and_then(|(s, value)| nl(s).map(|(s, _)| (s, value)))
}

pub(super) fn quoted(s: &str) -> CResult<'_, &str> {
    alt((delimited(char('\"'), take_until("\""), char('\"')), string))(s)
}

pub(super) fn nl(s: &str) -> CResult<'_, Option<&str>> {
    alt((map(multispace0, |_| None), map(comment, Some)))(s)
}

fn sep(s: &str) -> CResult<'_, ()> {
    map(tuple((nl, opt(char(',')), nl)), |_| ())(s)
}

pub(super) fn comment(s: &str) -> CResult<'_, &str> {
    preceded(pair(nl, char('#')), line)(s)
}

fn integer(s: &str) -> CResult<'_, u64> {
    map_res(recognize(digit1), str::parse)(s)
}

pub fn config_parser(s: &str) -> CResult<'_, Config> {
    all_consuming(map(many0(section), |sections: Vec<Section>| {
        let mut config = Config::default();
        for section in sections {
            match section {
                Section::Interval(d) => config.interval = d,
                Section::Socket(p) => config.socket = p,
                Section::Timeout(d) => config.timeout = d,
                Section::Table(t) => config.tables.push(t),
                Section::Redirect(r) => config.redirects.push(r),
                Section::Relay(r) => config.relays.push(r),
                Section::Protocol(p) => config.protocols.push(p),
                Section::Ignore => (),
            }
        }
        config
    }))(s)
}
