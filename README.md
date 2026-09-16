# `durstr`

[<img alt="Crates.io Version" src="https://img.shields.io/crates/v/durstr?style=flat-square">](https://crates.io/crates/durstr)
[<img alt="docs.rs" src="https://img.shields.io/docsrs/durstr?style=flat-square">](https://docs.rs/durstr)

A simple library for parsing human-readable duration strings into `std::time::Duration`.

## Usage

Add `durstr` to `Cargo.toml`:

```toml
[dependencies]
durstr = "0.5.0"
```

This library provides a `parse` function for quick and easy parsing, and a `Parser` struct for more control over parsing behavior.

The `parse` function is a convenience wrapper around a default `Parser`.

```rust
use durstr::parse;
use std::time::Duration;

let dur = parse("12 minutes, 21 seconds");
assert_eq!(dur, Ok(Duration::from_secs(741)));

let dur = parse("1hr 2min 3sec");
assert_eq!(dur, Ok(Duration::from_secs(3723)));
```

Empty input, or input containing only whitespace and commas, returns `Duration::ZERO`.

For more control, you can use the `Parser` struct directly. For example, you can lowercase parsed unit tokens before they are looked up:

```rust
use durstr::{Parser, ParserOptions};
use std::time::Duration;

let parser = Parser::new(ParserOptions::default().ignore_case(true));
let dur = parser.parse("1 MINUTE, 2 SECONDS");
assert_eq!(dur, Ok(Duration::from_secs(62)));
```

`ignore_case(true)` lowercases each unit token from the input before looking it up. It does not change unit names stored in `ParserUnits`, so custom units must be added with lowercase names to match differently-cased input.

## Units

By default, the following units are provided:

| Unit        | Aliases                            |
|-------------|------------------------------------|
| Millisecond | `ms`, `msec(s)`, `millisecond(s)`  |
| Second      | `s`, `sec(s)`, `second(s)`         |
| Minute      | `m`, `min(s)`, `minute(s)`         |
| Hour        | `h`, `hr(s)`, `hour(s)`            |

You can define your own units, and their values, using the `ParserUnits` struct:

Unit names must be nonempty ASCII letters; `add_unit` will err otherwise.

```rust
use durstr::{Parser, ParserOptions, ParserUnits};
use std::time::Duration;

let mut units = ParserUnits::default();
units.add_unit("days", Duration::from_secs(3600) * 24).expect("only ASCII");

let parser = Parser::new(ParserOptions::default().with_units(units));

let d = parser.parse("4 days");
assert_eq!(d, Ok(Duration::from_secs(3600) * 24 * 4));
```
