use std::time::Duration;

use durstr::{Error, Parser, ParserOptions, ParserUnits, parse};

#[test]
fn test_parsing() {
    let d = parse("2 minutes, 12 seconds");
    assert_eq!(d, Ok(Duration::from_secs(120 + 12)));

    let d = parse("45 msecs");
    assert_eq!(d, Ok(Duration::from_millis(45)));

    let d = parse("21 minutes 12 seconds");
    assert_eq!(d, Ok(Duration::from_secs(1272)));

    let d = parse("1 hr 2 mins 3 secs");
    assert_eq!(d, Ok(Duration::from_secs(3723)));

    let d = parse("1h 2min 3s 62ms");
    assert_eq!(d, Ok(Duration::from_millis(3723062)));

    let d = parse("1h2min3s62ms");
    assert_eq!(d, Ok(Duration::from_millis(3723062)));

    let d = parse("2min 3s62ms");
    assert_eq!(d, Ok(Duration::from_millis(123062)));

    let d = parse("2min 1*2 sec");
    assert_eq!(d, Err(Error::UnexpectedChar('*')));

    let d = parse("2 min 1 r");
    assert_eq!(d, Err(Error::UnexpectedUnit("r".to_owned())));

    let d = parse("2.1 min");
    assert_eq!(d, Err(Error::UnexpectedChar('.')));

    let d = parse("1 2");
    assert_eq!(d, Err(Error::ExpectedUnit));

    let d = parse("1 s m");
    assert_eq!(d, Err(Error::ExpectedNumber));
}

#[test]
fn test_parsing_case_sensitivity() {
    let parser = Parser::new(ParserOptions::default());

    let d = parser.parse("1 min 2 sec");
    assert_eq!(d, Ok(Duration::from_secs(62)));

    let d = parser.parse("1 Min 2 sec");
    assert_eq!(d, Err(Error::UnexpectedUnit("Min".to_owned())));

    let d = parser.parse("1 min 2 seC");
    assert_eq!(d, Err(Error::UnexpectedUnit("seC".to_owned())));

    let parser = Parser::new(ParserOptions::default().ignore_case(true));

    let d = parser.parse("1 min 2 sec");
    assert_eq!(d, Ok(Duration::from_secs(62)));

    let d = parser.parse("1 Min 2 sec");
    assert_eq!(d, Ok(Duration::from_secs(62)));

    let d = parser.parse("1 min 2 seC");
    assert_eq!(d, Ok(Duration::from_secs(62)));

    let d = parser.parse("1 MIN 2 SEC");
    assert_eq!(d, Ok(Duration::from_secs(62)));
}

#[test]
fn test_parsing_custom_units() {
    let mut units = ParserUnits::default();
    units.add_unit("day", Duration::from_secs(3600) * 24);
    let parser = Parser::new(ParserOptions::default().with_units(units));

    let d = parser.parse("1 day");
    assert_eq!(d, Ok(Duration::from_secs(3600) * 24));

    let d = parser.parse("4 day");
    assert_eq!(d, Ok(Duration::from_secs(3600) * 24 * 4));
}
