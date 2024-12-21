//! A template processor
//!
//! Utilizing nom.

use std::collections::BTreeMap;

use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{alphanumeric1, digit1},
    combinator::{map, map_res, value},
    multi::many0,
    sequence::delimited,
    IResult,
};

/// A simple tempate error
#[derive(Debug)]
pub enum TemplateError {
    /// Error related to nom
    NomError(String),
}

impl std::error::Error for TemplateError {}

impl std::fmt::Display for TemplateError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            TemplateError::NomError(e) => write!(f, "Nom error: {}", e),
        }
    }
}

fn throw_nom_error(error: nom::Err<nom::error::Error<&str>>) -> TemplateError {
    // clone the error to avoid lifetime issues
    match &error {
        nom::Err::Error(e) | nom::Err::Failure(e) => {
            // get like 64 bytes of data to show in the error message
            let str_data = if let nom::Err::Error(_) = &error {
                "Error"
            } else {
                "Failure"
            };
            let data = e.input;
            let data = if data.len() > 64 { &data[..64] } else { data };

            TemplateError::NomError(format!("{}: {:?}, data: {:?}", str_data, e.code, data))
        }
        nom::Err::Incomplete(e) => {
            let need_amount = if let nom::Needed::Size(amount) = e {
                format!("{} bytes", amount)
            } else {
                "unknown amount".to_string()
            };

            TemplateError::NomError(format!("Incomplete data, need: {}", need_amount))
        }
    }
}

/// A template token
#[derive(PartialEq, Clone, Debug)]
pub enum TemplateToken<'a> {
    /// Your standard text, that it's not a template format.
    ///
    /// This include a leading/trailing space.
    Standard(&'a str),
    /// A template format.
    Template(Template<'a>),
    /// An escaped `{` or `}`.
    Escaped(char),
}

/// A template format
#[derive(PartialEq, Clone, Debug)]
pub enum Template<'a> {
    /// Empty template format, usually something like `{}`
    ///
    /// This depends on the current index.
    Empty,
    /// Indexed template format, something like `{0}`, `{1}`, `{2}`, etc.
    ///
    /// This does not depend on the current index.
    ///
    /// When a duplicate index is found, the indexed will take precedence.
    Indexed(u32),
    /// Named template format, something like `{key}`.
    ///
    /// This does not depend on the current index.
    ///
    /// When there is a multiple named template with the same name,
    /// all of them will be replaced.
    Named(&'a str),
}

fn take_until0_multiple<'a>(
    delimiters: &'a [&'a str],
) -> impl Fn(&'a str) -> IResult<&'a str, &'a str> {
    move |input: &'a str| {
        if input.is_empty() {
            return Err(nom::Err::Error(nom::error::Error::new(
                input,
                nom::error::ErrorKind::Eof,
            )));
        }

        let mut end_index = input.len();
        for delimiter in delimiters {
            if let Some(index) = input.find(delimiter) {
                end_index = end_index.min(index);
            }
        }

        Ok((&input[end_index..], &input[..end_index]))
    }
}

/// Parse a template string into a vector of template tokens.
///
/// We use [nom](https://github.com/rust-bakery/nom) for parsing
///
/// The following "template" format is supported:
/// - `{key}` for a named template
/// - `{0}`, `{1}` for an indexed template
/// - `{}` for an empty template, this depends on the internal index counter, separate from the Indexed.
///
/// To escape use `{{` and `}}`
pub fn parse_template(text_template: &str) -> Result<Vec<TemplateToken<'_>>, TemplateError> {
    fn parse_template_token_other(input: &str) -> IResult<&str, Template> {
        alt((
            map_res(digit1, |s: &str| s.parse().map(Template::Indexed)),
            map(alphanumeric1, Template::Named),
        ))(input)
    }

    fn parse_template_token(input: &str) -> IResult<&str, Template> {
        let (rest, token) = alt((tag("{}"), delimited(tag("{"), alphanumeric1, tag("}"))))(input)?;

        if token == "{}" {
            Ok((rest, Template::Empty))
        } else {
            let (_, key) = parse_template_token_other(token)?;

            Ok((rest, key))
        }
    }

    fn map_template(input: &str) -> IResult<&str, TemplateToken> {
        map(parse_template_token, TemplateToken::Template)(input)
    }

    fn map_escaped_open(input: &str) -> IResult<&str, TemplateToken> {
        value(TemplateToken::Escaped('{'), tag("{{"))(input)
    }

    fn map_escaped_close(input: &str) -> IResult<&str, TemplateToken> {
        value(TemplateToken::Escaped('}'), tag("}}"))(input)
    }

    fn map_standard(input: &str) -> IResult<&str, TemplateToken> {
        map(
            take_until0_multiple(&["{", "}", "{{", "}}"]),
            TemplateToken::Standard,
        )(input)
    }

    fn map_template_token(input: &str) -> IResult<&str, TemplateToken> {
        if input.is_empty() {
            // Do this so many0 will stop
            return Err(nom::Err::Error(nom::error::Error::new(
                input,
                nom::error::ErrorKind::Eof,
            )));
        }

        let (rest, token) = alt((
            map_escaped_open,
            map_escaped_close,
            map_template,
            map_standard,
        ))(input)?;

        Ok((rest, token))
    }

    fn internal_parser(input: &str) -> IResult<&str, Vec<TemplateToken>> {
        many0(map_template_token)(input)
    }

    let (rest, mut tokens) = internal_parser(text_template).map_err(throw_nom_error)?;

    if !rest.is_empty() {
        tokens.push(TemplateToken::Standard(rest));
    }

    Ok(tokens)
}

pub(crate) fn format_template<'a, T: ToString>(
    tokens: &[TemplateToken<'a>],
    args: &[T],
    kwargs: &BTreeMap<&'a str, T>,
) -> String {
    let mut index = 0;
    let mut result = String::new();

    for token in tokens {
        match token {
            TemplateToken::Standard(s) => result.push_str(s),
            TemplateToken::Escaped(c) => result.push(*c),
            TemplateToken::Template(t) => {
                let value = match t {
                    Template::Indexed(i) => {
                        let res = args.get(*i as usize).map(ToString::to_string);
                        res.unwrap_or(format!("{{{}}}", i))
                    }
                    Template::Empty => {
                        let res = args
                            .get(index)
                            .map(ToString::to_string)
                            .unwrap_or("{}".to_string());
                        index += 1;
                        res
                    }
                    Template::Named(key) => {
                        let res = kwargs.get(key).map(ToString::to_string);
                        res.unwrap_or(format!("{{{}}}", key))
                    }
                };

                result.push_str(&value);
            }
        }
    }

    result
}

/// Format a template string with given positional and keyword arguments
///
/// The following "template" format is supported:
/// - `{key}` for a named template
/// - `{0}`, `{1}` for an indexed template
/// - `{}` for an empty template, this depends on the internal index counter, separate from the Indexed.
///
/// To escape use `{{` and `}}`, this will convert `{{` to `{` and `}}` to `}`
///
/// When the template string doesn't match the arguments, the original template string will be returned.
pub fn format_text<T: ToString>(
    template: &str,
    args: &[T],
    kwargs: &BTreeMap<&str, T>,
) -> Result<String, TemplateError> {
    Ok(format_template(&parse_template(template)?, args, kwargs))
}

/// A vector that stores strings and provides a more convenient
/// interface for pushing strings into it.
#[derive(Debug, Default)]
pub struct VecString {
    /// The internal vector
    internal: Vec<String>,
}

impl VecString {
    /// Create a new empty vector
    pub fn new() -> Self {
        Self {
            internal: Vec::new(),
        }
    }

    /// Push a string into the vector
    pub fn push(&mut self, s: impl Into<String>) {
        self.internal.push(s.into());
    }

    /// Get the internal vector
    pub fn into_inner(self) -> Vec<String> {
        self.internal
    }

    /// Convert the vector into a string separated by commas
    pub fn join(&self) -> String {
        self.internal.join(", ")
    }

    /// Consume input vector and convert it into [`VecString`]
    pub fn from_vec(input: Vec<String>) -> Self {
        Self { internal: input }
    }
}

impl std::fmt::Display for VecString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.internal.is_empty() {
            write!(f, "")
        } else {
            let joined_letters = if self.internal.len() == 1 {
                &self.internal[0]
            } else if self.internal.len() == 2 {
                &format!("{} and {}", self.internal[0], self.internal[1])
            } else {
                let last_one = self.internal.last().expect("internal vector is not empty");

                let before_last_one = self
                    .internal
                    .iter()
                    .take(self.internal.len() - 1)
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");

                &format!("{}, and {}", before_last_one, last_one)
            };

            write!(f, "{}", joined_letters)
        }
    }
}

impl From<Vec<String>> for VecString {
    fn from(value: Vec<String>) -> Self {
        Self::from_vec(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_template() {
        let parsed = parse_template(
            "This is a nice and {{escaped}} other {} empty then {0} indexed {syntax} final key",
        )
        .unwrap();

        assert_eq!(
            parsed,
            vec![
                TemplateToken::Standard("This is a nice and "),
                TemplateToken::Escaped('{'),
                TemplateToken::Standard("escaped"),
                TemplateToken::Escaped('}'),
                TemplateToken::Standard(" other "),
                TemplateToken::Template(Template::Empty),
                TemplateToken::Standard(" empty then "),
                TemplateToken::Template(Template::Indexed(0)),
                TemplateToken::Standard(" indexed "),
                TemplateToken::Template(Template::Named("syntax")),
                TemplateToken::Standard(" final key"),
            ]
        );
    }

    #[test]
    fn test_format_template() {
        let parsed = vec![
            TemplateToken::Standard("This is a nice and "),
            TemplateToken::Escaped('{'),
            TemplateToken::Standard("escaped"),
            TemplateToken::Escaped('}'),
            TemplateToken::Standard(" other "),
            TemplateToken::Template(Template::Empty),
            TemplateToken::Standard(" empty then "),
            TemplateToken::Template(Template::Indexed(0)),
            TemplateToken::Standard(" indexed "),
            TemplateToken::Template(Template::Named("syntax")),
            TemplateToken::Standard(" final key"),
        ];

        let mut tree_map = BTreeMap::new();
        tree_map.insert("syntax", "xdd");
        let result = format_template(&parsed, &["nice guy"], &tree_map);

        assert_eq!(
            result,
            "This is a nice and {escaped} other nice guy empty then nice guy indexed xdd final key"
        );
    }
}
