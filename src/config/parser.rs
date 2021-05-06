use crate::config::{Config, Protocol, ProtocolType, Redirect, Table, Variable};
use nom::{
    branch::alt,
    bytes::complete::{escaped_transform, is_not, tag, take_until, take_while},
    character::complete::{char, multispace0},
    combinator::{all_consuming, map, map_parser, opt, peek, value},
    error::VerboseError,
    multi::{many0, many_till},
    sequence::{delimited, pair, preceded, separated_pair, tuple},
    IResult,
};

//use nom::{character::complete::*, error::*, *};
use privsep_log::debug;

type Result<'a, T> = IResult<&'a str, T, VerboseError<&'a str>>;

enum Section {
    Table(Table),
    Redirect(Redirect),
    Protocol(Protocol),
    Ignore,
}

fn section(s: &str) -> Result<Section> {
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
        map(variable, |v| {
            debug!("{:?}", v);
            Section::Ignore
        }),
        map(nl, |_| Section::Ignore),
    ))(s)
}

fn table(s: &str) -> Result<Table> {
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

fn redirect(s: &str) -> Result<Redirect> {
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

fn protocol_type(s: &str) -> Result<ProtocolType> {
    alt((
        map(tag("tcp"), |_| ProtocolType::Tcp),
        map(tag("http"), |_| ProtocolType::Http),
        map(tag("dns"), |_| ProtocolType::Dns),
    ))(s)
}

fn protocol_option(s: &str) -> Result<()> {
    alt((
        map(preceded(tag("match"), line), |_| {
            debug!("match");
            ()
        }),
        map(preceded(tag("tcp"), line), |_| {
            debug!("tcp");
            ()
        }),
        map(preceded(tag("tls"), line), |_| ()),
        map(comment, |_| ()),
        map(nl, |_| ()),
    ))(s)
}

fn protocol_options(s: &str) -> Result<()> {
    delimited(
        char('{'),
        map(
            many_till(protocol_option, peek(char('}'))),
            |_options: (Vec<()>, _)| {
                // TODO
                ()
            },
        ),
        char('}'),
    )(s)
}

fn protocol(s: &str) -> Result<Protocol> {
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

fn string(s: &str) -> Result<&str> {
    take_while(allowed_in_string)(s)
}

fn line(s: &str) -> Result<&str> {
    take_until("\n")(s).and_then(|(s, value)| nl(s).map(|(s, _)| (s, value)))
}

fn quoted(s: &str) -> Result<&str> {
    alt((delimited(char('\"'), take_until("\""), char('\"')), string))(s)
}

fn nl(s: &str) -> Result<Option<&str>> {
    alt((map(multispace0, |_| None), map(comment, |c| Some(c))))(s)
}

fn comment(s: &str) -> Result<&str> {
    preceded(pair(nl, char('#')), line)(s)
}

fn variable(s: &str) -> Result<Variable> {
    map(separated_pair(string, char('='), quoted), |(key, value)| {
        Variable {
            key: key.to_string(),
            value: value.to_string(),
        }
    })(s)
}

pub fn config_preprocess(s: &str) -> IResult<&str, String, VerboseError<&str>> {
    escaped_transform(is_not("\\"), '\\', value(" ", tag("\n")))(s)
}

pub fn config_parser(s: &str) -> Result<Config> {
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
