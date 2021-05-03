use crate::config::{Config, Protocol, ProtocolType, Redirect, Table, Variable};
use nom::{
    alt, character::complete::multispace0, delimited, do_parse, error::VerboseError, many0,
    many_till, map, named, opt, peek, tag, take_until, take_while,
};
//use nom::{character::complete::*, error::*, *};
use privsep_log::debug;

enum Section {
    Table(Table),
    Redirect(Redirect),
    Protocol(Protocol),
    Ignore,
}

named!(
    section<&str, Section, VerboseError<&str>>,
    alt!(
        table => { |t| {
            debug!("{:?}", t);
            Section::Table(t)
        } } |
        redirect => { |r| {
            debug!("{:?}", r);
            Section::Redirect(r)
        } } |
        protocol => { |p| {
            debug!("{:?}", p);
            Section::Protocol(p)
        } } |
        comment => { |c| {
            debug!("#{}", c);
            Section::Ignore
        } } |
        variable => { |v| {
            debug!("{:?}", v);
            Section::Ignore
        } } |
        nl => { |_| Section::Ignore }
    )
);

named!(
    table<&str, Table, VerboseError<&str>>,
    do_parse!(
        tag!("table") >>
        nl >>
        name: delimited!(
            tag!("<"),
            string,
            tag!(">")
        ) >>
        nl >>
        tag!("{") >>
        take_until!("}") >>
        line >>
        ({ Table { name } })
    )
);

named!(
    redirect<&str, Redirect, VerboseError<&str>>,
    do_parse!(
        tag!("redirect") >>
        nl >>
        name: quoted >>
        nl >>
        tag!("{") >>
        take_until!("}") >>
        line >>
        ({ Redirect { name } })
    )
);

named!(
    protocol_type<&str, ProtocolType, VerboseError<&str>>,
    alt!(
        do_parse!(tag!("tcp") >> (ProtocolType::Tcp)) |
        do_parse!(tag!("http") >> (ProtocolType::Http)) |
        do_parse!(tag!("dns") >> (ProtocolType::Dns))
    )
);

named!(
    protocol_option<&str, (), VerboseError<&str>>,
    alt!(
        do_parse!(tag!("match") >> line >> (())) => { |m| {
            debug!("match");
            m
        } } |
        do_parse!(tag!("tcp") >> line >> (())) => { |tcp| {
            debug!("tcp");
            tcp
        } } |
        do_parse!(tag!("tls") >> line >> (())) => { |tls| {
           debug!("tls");
           tls
        } } |
        comment => { |c| {
           debug!("#{}", c);
           ()
        } } |
        nl => { |_nl| {
           debug!("nl");
           ()
        } }
    )
);

named!(
    protocol_options<&str, (), VerboseError<&str>>,
    delimited!(
        tag!("{"),
        map!(many_till!(protocol_option, peek!(tag!("}"))), |_options: (Vec<()>, _)| {
            // TODO
            ()
        }),
        tag!("}")
    )
);

named!(
    protocol<&str, Protocol, VerboseError<&str>>,
    do_parse!(
        typ: opt!(protocol_type) >>
        nl >>
        tag!("protocol") >>
        nl >>
        name: quoted >>
        nl >>
        protocol_options >>
        line >>
        ({ Protocol { name, typ: typ.unwrap_or_default() } })
    )
);

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

named!(
    string<&str, String, VerboseError<&str>>,
    map!(
        take_while!(allowed_in_string),
        |b: &str| String::from(b)
    )
);

named!(
    line<&str, String, VerboseError<&str>>,
    map!(
        do_parse!(line: take_until!("\n") >> nl >> (line)),
        |s: &str| s.to_string()
    )
);

named!(
    quoted<&str, String, VerboseError<&str>>,
    alt!(
        do_parse!(
            value: delimited!(
                tag!("\""),
                take_until!("\""),
                tag!("\"")
            )
            >>
            (String::from(value))
        ) |
        do_parse!(
            value: string >>
            (String::from(value))
        )
    )
);

named!(nl<&str, Option<String>, VerboseError<&str>>,
   alt!(
       map!(multispace0, |_| None) |
       map!(comment, |c| Some(c))
    )
);

named!(comment<&str, String, VerboseError<&str>>,
   do_parse!(
       nl >>
       tag!("#") >>
       comment: line >>
       nl >>
       (comment)
   )
);

named!(variable<&str, Variable, VerboseError<&str>>,
   do_parse!(
       key: string >>
       tag!("=") >>
       value: quoted >>
       (Variable { key, value })
   )
);

named!(pub config_parser<&str, Config, VerboseError<&str>>,
    map!(many0!(section), |sections: Vec<Section>| {
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
    })
);
