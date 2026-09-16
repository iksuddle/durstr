use std::time::Duration;

use durstr::{Error, Parser, ParserOptions, ParserUnits, parse};

#[test]
fn test_builtin_aliases() {
    let check = |aliases: &[&str], duration| {
        for alias in aliases {
            let input = format!("1{alias}");
            assert_eq!(parse(&input), Ok(duration), "{input:?}");
        }
    };

    check(
        &["h", "hr", "hrs", "hour", "hours"],
        Duration::from_secs(3600),
    );
    check(
        &["m", "min", "mins", "minute", "minutes"],
        Duration::from_secs(60),
    );
    check(
        &["s", "sec", "secs", "second", "seconds"],
        Duration::from_secs(1),
    );
    check(
        &["ms", "msec", "msecs", "millisecond", "milliseconds"],
        Duration::from_millis(1),
    );
}

#[test]
fn test_parsing() {
    assert_eq!(parse(""), Ok(Duration::ZERO));
    assert_eq!(parse(",,,"), Ok(Duration::ZERO));
    assert_eq!(parse(" ,\t\r\n\u{a0}\u{2003}, "), Ok(Duration::ZERO));
    assert_eq!(parse("0h0min0s0ms"), Ok(Duration::ZERO));

    let mixed = Duration::from_millis(3_723_062);
    assert_eq!(parse("1h2min3s62ms"), Ok(mixed));
    assert_eq!(
        parse(" 1 hour,, 2 minutes, 3 seconds 62 milliseconds, "),
        Ok(mixed)
    );
    assert_eq!(parse("1,s,2,ms"), Ok(Duration::from_millis(1002)));
    assert_eq!(parse("0003s2min1h1s"), Ok(Duration::from_secs(3724)));
    assert_eq!(parse("1s1000ms"), Ok(Duration::from_secs(2)));
}

#[test]
fn test_whitespace() {
    // Exercise ASCII and multibyte whitespace at each token boundary.
    for whitespace in [' ', '\t', '\n', '\u{b}', '\u{a0}', '\u{2003}'] {
        let input = format!("{whitespace}1{whitespace}s{whitespace}2{whitespace}ms{whitespace}");
        assert_eq!(parse(&input), Ok(Duration::from_millis(1002)), "{input:?}");
    }
}

#[test]
fn test_invalid_input_and_parser_reuse() {
    let parser = Parser::default();
    let check = |input: &str, error| {
        assert_eq!(parser.parse(input), Err(error), "{input:?}");
        assert_eq!(
            parser.parse("2s"),
            Ok(Duration::from_secs(2)),
            "after {input:?}"
        );
    };

    check("1s 2", Error::ExpectedUnit);
    check("1 2s", Error::ExpectedUnit);
    check("s", Error::ExpectedNumber);
    check("1s min", Error::ExpectedNumber);
    check("1s1unknown", Error::UnexpectedUnit("unknown".into()));
    check("1MIN", Error::UnexpectedUnit("MIN".into()));

    check("-1s", Error::UnexpectedChar('-'));
    check("+1s", Error::UnexpectedChar('+'));
    check("1.5s", Error::UnexpectedChar('.'));
    check("1_s", Error::UnexpectedChar('_'));
    check("１s", Error::UnexpectedChar('１'));
    check("1µs", Error::UnexpectedChar('µ'));
    check("1s\u{200b}", Error::UnexpectedChar('\u{200b}'));
    check("\u{a0}1s💥", Error::UnexpectedChar('💥'));
}

#[test]
fn test_number_boundaries() {
    let max = u64::from(u32::MAX);
    assert_eq!(parse("4294967295s"), Ok(Duration::from_secs(max)));
    assert_eq!(parse("4294967295ms"), Ok(Duration::from_millis(max)));
    assert_eq!(parse("4294967295s1s"), Ok(Duration::from_secs(max + 1)));

    for number in ["4294967296", "0004294967296", "18446744073709551616"] {
        assert_eq!(
            parse(&format!("{number}s")),
            Err(Error::Overflow(number.into())),
            "{number:?}"
        );
    }

    let input = format!("{}1s", "0".repeat(100));
    assert_eq!(parse(&input), Ok(Duration::from_secs(1)));
}

#[test]
fn test_duration_boundaries() {
    let mut units = ParserUnits::new();
    units.add_unit("max", Duration::MAX).unwrap();
    units.add_unit("ns", Duration::from_nanos(1)).unwrap();
    units
        .add_unit("almost", Duration::new(u64::MAX, 999_999_998))
        .unwrap();
    units
        .add_unit("fraction", Duration::from_nanos(999_999_999))
        .unwrap();
    units.add_unit("zero", Duration::ZERO).unwrap();
    let parser = Parser::new(ParserOptions::default().with_units(units));

    assert_eq!(parser.parse("1max"), Ok(Duration::MAX));
    assert_eq!(parser.parse("0max"), Ok(Duration::ZERO));
    assert_eq!(parser.parse("4294967295zero"), Ok(Duration::ZERO));
    assert_eq!(parser.parse("1almost1ns"), Ok(Duration::MAX));
    assert_eq!(parser.parse("2fraction"), Ok(Duration::new(1, 999_999_998)));
    assert_eq!(parser.parse("1fraction1ns"), Ok(Duration::from_secs(1)));

    assert_eq!(parser.parse("2max"), Err(Error::DurationOverflow));
    assert_eq!(parser.parse("1max1ns"), Err(Error::DurationOverflow));
}

#[test]
fn test_custom_only_units() {
    let parser = Parser::new(ParserOptions::default().with_units(ParserUnits::new()));
    assert_eq!(parser.parse(""), Ok(Duration::ZERO));
    assert_eq!(parser.parse("1s"), Err(Error::UnexpectedUnit("s".into())));

    let mut units = ParserUnits::new();
    units.add_unit("tick", Duration::from_nanos(100)).unwrap();
    let parser = Parser::new(ParserOptions::default().with_units(units));

    assert_eq!(parser.parse("2tick"), Ok(Duration::from_nanos(200)));
    assert_eq!(parser.parse("1s"), Err(Error::UnexpectedUnit("s".into())));
}

#[test]
fn test_adding_and_replacing_units() {
    let mut units = ParserUnits::default();
    units.add_unit("s", Duration::from_secs(10)).unwrap();
    units.add_unit("tick", Duration::from_secs(2)).unwrap();
    units.add_unit("tick", Duration::from_secs(3)).unwrap();
    let parser = Parser::new(ParserOptions::default().with_units(units));

    assert_eq!(parser.parse("1s"), Ok(Duration::from_secs(10)));
    assert_eq!(parser.parse("1sec"), Ok(Duration::from_secs(1)));
    assert_eq!(parser.parse("2tick"), Ok(Duration::from_secs(6)));
}

#[test]
fn test_invalid_unit_names() {
    let mut units = ParserUnits::default();
    units.add_unit("tick", Duration::from_secs(7)).unwrap();

    for name in ["", "1tick", "h2m", " tick", "tick ", "tick!", "µs"] {
        assert_eq!(
            units.add_unit(name, Duration::from_secs(100)),
            Err(Error::InvalidUnit(name.into())),
            "{name:?}"
        );
    }

    // Rejected registrations must preserve existing units and compact syntax.
    let parser = Parser::new(ParserOptions::default().with_units(units));

    assert_eq!(parser.parse("1tick"), Ok(Duration::from_secs(7)));
    assert_eq!(parser.parse("1s"), Ok(Duration::from_secs(1)));
    assert_eq!(parser.parse("1h2m"), Ok(Duration::from_secs(3720)));
}

fn parser_with_case_sensitive_units(ignore_case: bool) -> Parser {
    let mut units = ParserUnits::default();
    units.add_unit("tick", Duration::from_secs(3)).unwrap();
    units.add_unit("Tick", Duration::from_secs(7)).unwrap();
    units.add_unit("DAY", Duration::from_secs(9)).unwrap();
    Parser::new(
        ParserOptions::default()
            .with_units(units)
            .ignore_case(ignore_case),
    )
}

#[test]
fn test_case_sensitivity() {
    let parser = parser_with_case_sensitive_units(false);

    assert_eq!(parser.parse("1Tick"), Ok(Duration::from_secs(7)));
    assert_eq!(
        parser.parse("1MiN"),
        Err(Error::UnexpectedUnit("MiN".into()))
    );
    assert_eq!(parser.parse("1DAY"), Ok(Duration::from_secs(9)));
}

#[test]
fn test_ignore_case() {
    let parser = parser_with_case_sensitive_units(true);

    assert_eq!(parser.parse("1Tick"), Ok(Duration::from_secs(3)));
    assert_eq!(parser.parse("1MiN"), Ok(Duration::from_secs(60)));
    assert_eq!(
        parser.parse("1DAY"),
        Err(Error::UnexpectedUnit("day".into()))
    );
}
