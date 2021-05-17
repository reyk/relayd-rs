use crate::config::{Config, Protocol, ProtocolType, Redirect, Table, Variable, Variables};
use nom::{
    branch::{alt, Alt},
    bytes::complete::{escaped_transform, is_not, tag, take_until, take_while},
    character::complete::{char, multispace0},
    combinator::{all_consuming, map, opt, peek, value},
    error::{ParseError, VerboseError},
    multi::{many0, many_till},
    sequence::{delimited, pair, preceded, separated_pair, tuple},
    Err, IResult,
};
use privsep_log::debug;

type CResult<'a, T> = IResult<&'a str, T, VerboseError<&'a str>>;

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
        map(variable, |v| {
            debug!("{:?}", v);
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

fn string(s: &str) -> CResult<'_, &str> {
    take_while(allowed_in_string)(s)
}

fn line(s: &str) -> CResult<'_, &str> {
    take_until("\n")(s).and_then(|(s, value)| nl(s).map(|(s, _)| (s, value)))
}

fn quoted(s: &str) -> CResult<'_, &str> {
    alt((delimited(char('\"'), take_until("\""), char('\"')), string))(s)
}

fn nl(s: &str) -> CResult<'_, Option<&str>> {
    alt((map(multispace0, |_| None), map(comment, Some)))(s)
}

fn comment(s: &str) -> CResult<'_, &str> {
    preceded(pair(nl, char('#')), line)(s)
}

fn variable(s: &str) -> CResult<'_, Variable> {
    map(separated_pair(string, char('='), quoted), |(key, value)| {
        Variable {
            key: key.to_string(),
            value: value.to_string(),
        }
    })(s)
}

fn variable_section(s: &str) -> CResult<'_, Option<Variable>> {
    alt((map(variable, Some), map(line, |_| None)))(s)
}

impl<'a, E: ParseError<&'a str>> Alt<&'a str, &'a str, E> for &'a Variables {
    fn choice(&mut self, input: &'a str) -> IResult<&'a str, &'a str, E> {
        for variable in self.0.iter() {
            if input.starts_with(&variable.key) {
                return Ok((&input[variable.key.len()..], &variable.value));
            }
        }

        is_not("$")(input)
    }
}

// TODO: parse everything in one step
#[allow(clippy::let_and_return)]
pub fn config_expand(s: &str) -> IResult<&str, String, VerboseError<&str>> {
    let (_, variables) = map(
        many0(variable_section),
        |variables: Vec<Option<Variable>>| {
            let variables: Vec<Variable> = variables.into_iter().flatten().collect();
            Variables::from(variables)
        },
    )(s)?;
    let (_, output) = escaped_transform(is_not("\\"), '\\', value(" ", tag("\n")))(s)?;
    let result = escaped_transform(is_not("$"), '$', alt(&variables))(output.as_ref())
        .map_err(|_err: Err<VerboseError<&str>>| {
            Err::<VerboseError<&str>>::Error(VerboseError::<&str> {
                errors: vec![(
                    "",
                    nom::error::VerboseErrorKind::Context("invalid variable"),
                )],
            })
        })
        .map(|(_, o)| (s, o));
    result
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
