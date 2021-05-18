use crate::config::{
    parser::{comment, line, quoted, string, CResult},
    Variable, Variables,
};
use nom::{
    branch::alt,
    bytes::complete::{is_not, tag},
    character::complete::{char, multispace1},
    combinator::{map, opt},
    error::VerboseError,
    multi::many0,
    sequence::{pair, separated_pair},
    Err, IResult,
};

#[derive(Debug)]
enum Token {
    Variable(Variable),
    UseVariable(String),
    Escaped(String),
    Next(String),
    Nested(Vec<Self>),
    None,
}

impl Token {
    pub fn resolve(&self, output: &mut String, variables: &mut Variables) {
        match self {
            Token::None => {}
            Token::Variable(variable) => {
                if !variables.contains_key(&variable.key) {
                    variables.insert(variable.key.clone(), variable.value.clone());
                }
            }
            Token::UseVariable(name) if variables.contains_key(name) => {
                if let Some(value) = variables.get(name) {
                    output.push_str(&value);
                }
            }
            Token::Nested(values) => {
                for token in values {
                    token.resolve(output, variables);
                }
            }
            _ => output.push_str(&self.to_string()),
        }
    }
}

impl Default for Token {
    fn default() -> Self {
        Self::None
    }
}

impl ToString for Token {
    fn to_string(&self) -> String {
        let mut result = String::new();
        match self {
            Token::Variable(variable) => {
                result = variable.to_string();
            }
            Token::UseVariable(name) => {
                result.push('$');
                result.push_str(&name);
            }
            Token::Escaped(value) => {
                result.push_str(&value);
            }
            Token::Nested(values) => {
                result = values
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("\n");
            }
            Token::Next(value) => {
                result.push_str(&value);
            }
            Token::None => {}
        }
        result
    }
}

fn escaped_line(s: &str) -> CResult<'_, Token> {
    map(
        pair(is_not("\\\n"), tag("\\\n")),
        |(a, _b): (&str, &str)| Token::Escaped(a.to_string()),
    )(s)
}

fn variable(s: &str) -> CResult<'_, Variable> {
    map(separated_pair(string, char('='), quoted), |(key, value)| {
        Variable {
            key: key.to_string(),
            value: value.to_string(),
        }
    })(s)
}

fn use_variable(s: &str) -> CResult<'_, Token> {
    map(
        pair(
            opt(is_not("$\n")),
            map(pair(char('$'), string), |(_, value): (_, &str)| {
                Token::UseVariable(value.to_string())
            }),
        ),
        |(line, variable)| {
            Token::Nested(vec![
                line.map(|line| Token::Next(line.to_string()))
                    .unwrap_or_default(),
                variable,
            ])
        },
    )(s)
}

fn include(s: &str) -> CResult<'_, Token> {
    let (input, (_, file)) = separated_pair(tag("include"), multispace1, quoted)(s)?;

    let output = std::fs::read_to_string(file).map_err(|_err| {
        Err::<VerboseError<&str>>::Error(VerboseError::<&str> {
            errors: vec![(
                input,
                nom::error::VerboseErrorKind::Context("include file not found"),
            )],
        })
    })?;
    let (_, tokens) = many0(ast)(&output).map_err(|_err| {
        Err::<VerboseError<&str>>::Error(VerboseError::<&str> {
            errors: vec![(
                input,
                nom::error::VerboseErrorKind::Context("invalid include"),
            )],
        })
    })?;

    Ok((input, Token::Nested(tokens)))
}

fn read_line(s: &str) -> CResult<'_, Token> {
    alt((
        map(comment, |_| Token::None),
        map(line, |s| Token::Next(format!("{}\n", s))),
    ))(s)
}

fn ast(s: &str) -> CResult<'_, Token> {
    alt((
        map(variable, Token::Variable),
        escaped_line,
        use_variable,
        include,
        read_line,
    ))(s)
}

pub fn config_expand2(
    s: &str,
    mut variables: Variables,
) -> IResult<&str, String, VerboseError<&str>> {
    map(many0(ast), |ast: Vec<Token>| {
        let mut result = String::new();
        for token in ast {
            token.resolve(&mut result, &mut variables);
        }
        result
    })(s)
}
