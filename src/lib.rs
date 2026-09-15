/*!
A simple library for parsing human-readable duration strings into `std::time::Duration`.

## Usage

This library provides a [`parse`] function for quick and easy parsing, and a [`Parser`]
struct for more control over parsing behavior.

### The `parse` function

The [`parse`] function is a convenience wrapper around a default [`Parser`].

```
use durstr::parse;
use std::time::Duration;

let dur = parse("12 minutes, 21 seconds");
assert_eq!(dur, Ok(Duration::from_secs(741)));

let dur = parse("1hr 2min 3sec");
assert_eq!(dur, Ok(Duration::from_secs(3723)));
```

### The `Parser` struct

For more control, you can use the [`Parser`] struct directly. For example, to parse with case-insensitivity:

```
use durstr::{Parser, ParserOptions};
use std::time::Duration;

let options = ParserOptions::default().ignore_case(true);
let parser = Parser::new(options);

let dur = parser.parse("1 MINUTE, 2 SECONDS");
assert_eq!(dur, Ok(Duration::from_secs(62)));
```

## Units

By default, the following units are provided:

| Unit        | Aliases                            |
|-------------|------------------------------------|
| Millisecond | `ms`, `msec(s)`, `millisecond(s)`  |
| Second      | `s`, `sec(s)`, `second(s)`         |
| Minute      | `m`, `min(s)`, `minute(s)`         |
| Hour        | `h`, `hr(s)`, `hour(s)`            |

You can define your own units, and their values, using the `ParserUnits` struct:

```
use durstr::{Parser, ParserOptions, ParserUnits};
use std::time::Duration;

let mut units = ParserUnits::default();
units.add_unit("days", Duration::from_secs(3600) * 24);

let parser = Parser::new(ParserOptions::default().with_units(units));

let d = parser.parse("4 days");
assert_eq!(d, Ok(Duration::from_secs(3600) * 24 * 4));
```
*/

use std::{borrow::Cow, collections::HashMap, iter::Peekable, str::CharIndices, time::Duration};

/// An error that can occur when parsing a duration string.
#[derive(thiserror::Error, Debug, PartialEq)]
pub enum Error {
    /// An unexpected character was found.
    #[error("unexpected character: {0}")]
    UnexpectedChar(char),
    /// An unexpected unit was found.
    #[error("unexpected unit: {0}")]
    UnexpectedUnit(String),
    /// A unit was expected, but not found.
    #[error("expected a unit")]
    ExpectedUnit,
    /// A number was expected, but not found.
    #[error("expected a number")]
    ExpectedNumber,
    /// A number was too large.
    #[error("number was too large: {0}")]
    Overflow(String),
    /// A duration was too large.
    #[error("duration was too large")]
    DurationOverflow,
}

#[derive(Debug, PartialEq, Eq)]
enum Token<'a> {
    Number(u32),
    Unit(&'a str),
}

struct Scanner<'a> {
    source: &'a str,
    chars: Peekable<CharIndices<'a>>,
}

impl<'a> Scanner<'a> {
    fn new(source: &'a str) -> Self {
        Scanner {
            source,
            chars: source.char_indices().peekable(),
        }
    }

    fn scan_tokens(mut self) -> Result<Vec<Token<'a>>, Error> {
        let mut tokens = vec![];

        while let Some(&(i, c)) = self.chars.peek() {
            match c {
                c if self.should_skip(c) => {
                    self.chars.next();
                }
                c if c.is_ascii_digit() => {
                    tokens.push(Token::Number(self.scan_number(i)?));
                }
                c if c.is_ascii_alphabetic() => {
                    tokens.push(Token::Unit(self.scan_unit(i)));
                }
                unexpected => return Err(Error::UnexpectedChar(unexpected)),
            };
        }

        Ok(tokens)
    }

    fn should_skip(&self, c: char) -> bool {
        c.is_ascii_whitespace() || c == ','
    }

    fn scan_number(&mut self, start: usize) -> Result<u32, Error> {
        let mut end = start;
        while let Some((_, c)) = self.chars.peek() {
            if !c.is_ascii_digit() {
                break;
            }
            // peek guarantees this won't panic
            end = self.chars.next().unwrap().0;
        }

        self.source[start..=end]
            .parse()
            .map_err(|_e| Error::Overflow(self.source[start..=end].to_string()))
    }

    fn scan_unit(&mut self, start: usize) -> &'a str {
        let mut end = start;
        while let Some((_, c)) = self.chars.peek() {
            if !c.is_ascii_alphabetic() {
                break;
            }
            end = self.chars.next().unwrap().0;
        }

        &self.source[start..=end]
    }
}

/// Used to customize the parser's units and their values.
///
/// ## Example
/// ```
/// use durstr::{Parser, ParserOptions, ParserUnits};
/// use std::time::Duration;
///
/// let mut units = ParserUnits::default();
/// units.add_unit("days", Duration::from_secs(3600) * 24);
///
/// let parser = Parser::new(ParserOptions::default().with_units(units));
///
/// let d = parser.parse("4 days");
/// assert_eq!(d, Ok(Duration::from_secs(3600) * 24 * 4));
/// ```
pub struct ParserUnits {
    values: HashMap<String, Duration>,
}

impl ParserUnits {
    /// Returns a [`ParserUnits`] with no units.
    ///
    /// Unlike [`ParserUnits::default`], this does not include the built-in units
    /// (hours, minutes, seconds, milliseconds). Use this when you want full control
    /// over which units are available.
    pub fn new() -> Self {
        ParserUnits {
            values: HashMap::new(),
        }
    }

    /// Insert/update a unit and its value.
    ///
    /// For example, to add a unit 'day' with a duration of 24 hours:
    /// ```
    /// use durstr::ParserUnits;
    /// use std::time::Duration;
    ///
    /// let mut units = ParserUnits::default();
    /// units.add_unit("day", Duration::from_secs(3600) * 24);
    /// ```
    pub fn add_unit(&mut self, k: impl Into<String>, v: Duration) {
        self.values.insert(k.into(), v);
    }

    fn get_duration(&self, k: &str) -> Option<&Duration> {
        self.values.get(k)
    }
}

impl Default for ParserUnits {
    /// Provides the default set of units for parsing durations.
    ///
    /// Default Units
    /// - `ms`, `msec(s)`, `millisecond(s)`
    /// - `s`, `sec(s)`, `second(s)`
    /// - `m`, `min(s)`, `minute(s)`
    /// - `h`, `hr(s)`, `hour(s)`
    fn default() -> Self {
        let mut parser_units = ParserUnits::new();

        for u in ["h", "hr", "hrs", "hour", "hours"] {
            parser_units.add_unit(u, Duration::from_secs(3600));
        }
        for u in ["m", "min", "mins", "minute", "minutes"] {
            parser_units.add_unit(u, Duration::from_secs(60));
        }
        for u in ["s", "sec", "secs", "second", "seconds"] {
            parser_units.add_unit(u, Duration::from_secs(1));
        }
        for u in ["ms", "msec", "msecs", "millisecond", "milliseconds"] {
            parser_units.add_unit(u, Duration::from_millis(1));
        }

        parser_units
    }
}

/// Options to customize the behavior of a [`Parser`].
///
/// This struct allows for more control over how duration strings are
/// interpreted. (e.g. enabling case-insensitivity)
#[derive(Default)]
pub struct ParserOptions {
    ignore_case: bool,
    units: ParserUnits,
}

impl ParserOptions {
    /// Enable the ignore_case flag for these options.
    pub fn ignore_case(mut self, ignore: bool) -> Self {
        self.ignore_case = ignore;
        self
    }

    /// Provide custom units for these options.
    pub fn with_units(mut self, units: ParserUnits) -> Self {
        self.units = units;
        self
    }
}

/// A configurable parser for duration strings.
///
/// Use this when you need to configure the parsing logic. Otherwise, the
/// top-level [`parse`] function is likely sufficient.
#[derive(Default)]
pub struct Parser {
    options: ParserOptions,
}

impl Parser {
    /// Create a new [`Parser`] with provided [`ParserOptions`]
    pub fn new(options: ParserOptions) -> Self {
        Parser { options }
    }

    /// Parses a string into a `Duration`, ignoring whitespaces and commas.
    ///
    /// Default Units
    /// - `ms`, `msec(s)`, `millisecond(s)`
    /// - `s`, `sec(s)`, `second(s)`
    /// - `m`, `min(s)`, `minute(s)`
    /// - `h`, `hr(s)`, `hour(s)`
    ///
    /// ## Examples
    /// ```
    /// use durstr::{Parser, ParserOptions};
    /// use std::time::Duration;
    ///
    /// let parser = Parser::default();
    /// let dur = parser.parse("1 minute, 2 seconds");
    /// assert_eq!(dur, Ok(Duration::from_secs(62)));
    /// ```
    pub fn parse(&self, input: &str) -> Result<Duration, Error> {
        let tokens = Scanner::new(input).scan_tokens()?;
        self.parse_tokens(tokens)
    }

    fn parse_tokens(&self, tokens: Vec<Token>) -> Result<Duration, Error> {
        let mut tokens = tokens.into_iter();
        let mut dur = Duration::ZERO;

        while let Some(token) = tokens.next() {
            let num = match token {
                Token::Number(n) => n,
                Token::Unit(_) => return Err(Error::ExpectedNumber),
            };

            let unit = match tokens.next() {
                Some(Token::Unit(u)) => u,
                _ => return Err(Error::ExpectedUnit),
            };

            let unit_duration = self.get_unit_duration(unit)?;
            let component = unit_duration
                .checked_mul(num)
                .ok_or(Error::DurationOverflow)?;
            dur = dur.checked_add(component).ok_or(Error::DurationOverflow)?;
        }

        Ok(dur)
    }

    fn get_unit_duration(&self, unit: &str) -> Result<Duration, Error> {
        let unit = if self.options.ignore_case {
            Cow::Owned(unit.to_lowercase())
        } else {
            Cow::Borrowed(unit)
        };

        match self.options.units.get_duration(&unit) {
            Some(d) => Ok(*d),
            None => Err(Error::UnexpectedUnit(unit.into_owned())),
        }
    }
}

/// Parses a duration string into a `std::time::Duration`.
///
/// This function provides a quick and easy way to parse common duration
/// formats. It is a convenience wrapper around a default [`Parser`], which is
/// case-sensitive and ignores whitespace and commas.
///
/// For more control over parsing behavior, such as enabling case-insensitivity,
/// construct a [`Parser`] with custom [`ParserOptions`].
///
/// ## Examples
/// ```
/// use durstr::parse;
/// use std::time::Duration;
///
/// let dur = parse("12 minutes, 21 seconds");
/// assert_eq!(dur, Ok(Duration::from_secs(741)));
///
/// let dur = parse("1hr 2min 3sec");
/// assert_eq!(dur, Ok(Duration::from_secs(3723)));
///
/// // By default, parsing is case-sensitive.
/// let dur = parse("1 MINUTE");
/// assert!(dur.is_err());
/// ```
pub fn parse(input: &str) -> Result<Duration, Error> {
    Parser::default().parse(input)
}

#[cfg(test)]
mod tests {
    use crate::{Scanner, Token};

    #[test]
    fn test_scanner() {
        let scanner = Scanner::new("10 seconds");
        let tokens = scanner.scan_tokens();
        assert_eq!(tokens, Ok(vec![Token::Number(10), Token::Unit("seconds")]));

        let scanner = Scanner::new("9hr1min");
        let tokens = scanner.scan_tokens();
        assert_eq!(
            tokens,
            Ok(vec![
                Token::Number(9),
                Token::Unit("hr"),
                Token::Number(1),
                Token::Unit("min"),
            ])
        );

        let scanner = Scanner::new("712635 days");
        let tokens = scanner.scan_tokens();
        assert_eq!(tokens, Ok(vec![Token::Number(712635), Token::Unit("days")]));
    }
}
