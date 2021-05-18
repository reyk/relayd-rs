use crate::config::{Config, Protocol, ProtocolType, Redirect, Table};
use nom::{
    branch::alt,
    bytes::complete::{tag, take_until, take_while1},
    character::complete::{char, multispace0},
    combinator::{all_consuming, map, opt, peek},
    error::VerboseError,
    multi::{many0, many_till},
    sequence::{delimited, pair, preceded, tuple},
    IResult,
};
use privsep_log::debug;

pub(super) type CResult<'a, T> = IResult<&'a str, T, VerboseError<&'a str>>;

enum Section {
    Table(Table),
    Redirect(Redirect),
    Protocol(Protocol),
    Ignore,
}

fn section(s: &str) -> CResult<'_, Section> {
    alt((
        map(table, |t| {
            debug!("{:?}", t);
            Section::Table(t)
        }),
        map(redirect, |r| {
            debug!("{:?}", r);
            Section::Redirect(r)
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
    ))(s)
}

fn table(s: &str) -> CResult<'_, Table> {
    map(
        tuple((
            tag("table"),
            nl,
            delimited(char('<'), string, char('>')),
            nl,
            char('{'),
            take_until("}"),
            line,
        )),
        |(_, _, name, _n, _, _, _)| Table {
            name: name.to_string(),
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

fn protocol_type(s: &str) -> CResult<'_, ProtocolType> {
    alt((
        map(tag("tcp"), |_| ProtocolType::Tcp),
        map(tag("http"), |_| ProtocolType::Http),
        map(tag("dns"), |_| ProtocolType::Dns),
    ))(s)
}

fn protocol_option(s: &str) -> CResult<'_, ()> {
    alt((
        map(preceded(tag("match"), line), |_| {
            debug!("match");
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

pub(super) fn comment(s: &str) -> CResult<'_, &str> {
    preceded(pair(nl, char('#')), line)(s)
}

pub fn config_parser(s: &str) -> CResult<'_, Config> {
    all_consuming(map(many0(section), |sections: Vec<Section>| {
        let mut config = Config::default();
        for section in sections {
            match section {
                Section::Table(t) => config.tables.push(t),
                Section::Redirect(r) => config.redirects.push(r),
                Section::Protocol(p) => config.protocols.push(p),
                Section::Ignore => (),
            }
        }
        config
    }))(s)
}
