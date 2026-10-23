use super::*;

fn read_fixture(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("test_files")
        .join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("could not read {}: {error}", path.display()))
}

fn text(value: impl Into<String>) -> JsonElement {
    JsonElement::String(value.into())
}

fn array(values: Vec<JsonElement>) -> JsonElement {
    JsonElement::Array(values)
}

fn object(entries: Vec<(&str, JsonElement)>) -> JsonElement {
    JsonElement::Object(
        entries
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
    )
}

fn check_fixture(name: &str, expected: JsonElement) {
    let input = read_fixture(name);
    let actual = parse_json(&input, None);
    let expected = Ok(expected);
    assert!(
        actual == expected,
        "fixture {name}: expected {expected:?}, got {actual:?}"
    );
}

#[test]
fn catalog_file_preserves_all_records() {
    use JsonElement::{Boolean, Null};
    let records = (0..120)
        .map(|i| {
            object(vec![
                ("id", text(format!("item-{i:03}"))),
                ("active", Boolean(i % 2 == 0)),
                (
                    "description",
                    text(format!("Item {i}: \"quoted\"\nC:\\data")),
                ),
                (
                    "metadata",
                    object(vec![
                        (
                            "aliases",
                            array(vec![text(format!("alias-{i}")), text(""), text("[],:{}")]),
                        ),
                        ("archived", Boolean(false)),
                        ("owner", Null),
                    ]),
                ),
                (
                    "groups",
                    array(vec![
                        array(vec![Boolean(true), Boolean(false)]),
                        array(vec![]),
                        array(vec![Null, text(format!("group-{}", i % 7))]),
                    ]),
                ),
            ])
        })
        .collect();
    check_fixture(
        "catalog.json",
        object(vec![
            ("name", text("Example catalog")),
            ("records", array(records)),
            ("empty", object(vec![])),
        ]),
    );
}

#[test]
fn events_file_preserves_order_and_payloads() {
    use JsonElement::{Boolean, Null};
    let events = (0..256)
        .map(|i| {
            object(vec![
                ("sequence", text(i.to_string())),
                ("accepted", Boolean(i % 3 == 0)),
                (
                    "payload",
                    object(vec![
                        (
                            "tags",
                            array(vec![text("event"), text(format!("batch-{}", i % 8))]),
                        ),
                        ("value", Null),
                        ("message", text(format!("Event {i}\n\"received\""))),
                        (
                            "matrix",
                            array(vec![
                                array(vec![Boolean(false)]),
                                array(vec![Boolean(true), Null]),
                                array(vec![]),
                            ]),
                        ),
                    ]),
                ),
            ])
        })
        .collect();
    check_fixture("events.json", array(events));
}

#[test]
fn nested_file_preserves_every_level() {
    use JsonElement::{Boolean, Null};
    let mut expected = object(vec![(
        "leaf",
        array(vec![
            text("done"),
            Boolean(true),
            Boolean(false),
            Null,
            object(vec![]),
            array(vec![]),
        ]),
    )]);
    for i in (0..24).rev() {
        expected = object(vec![
            ("level", text(i.to_string())),
            (
                "children",
                array(vec![expected, object(vec![("empty", array(vec![]))])]),
            ),
            ("enabled", Boolean(true)),
        ]);
    }
    check_fixture("nested.json", expected);
}

#[test]
fn missing_colon_file_reports_offending_byte() {
    let input = read_fixture("invalid_missing_colon.json");
    let position = input
        .find("\"active\" true")
        .expect("fixture must contain malformed member")
        + "\"active\" ".len();
    let actual = parse_json(&input, None);
    let expected = Err(ParseError::UnexpectedToken(position));
    assert!(actual == expected, "expected {expected:?}, got {actual:?}");
}

#[test]
fn truncated_file_reports_end_of_input() {
    let input = read_fixture("invalid_truncated.json");
    let actual = parse_json(&input, None);
    let expected = Err(ParseError::UnexpectedEOF(input.len()));
    assert!(actual == expected, "expected {expected:?}, got {actual:?}");
}

#[test]
fn trailing_value_file_reports_second_value() {
    let input = read_fixture("invalid_trailing_value.json");
    let position = input
        .rfind("false")
        .expect("fixture must contain trailing value");
    let actual = parse_json(&input, None);
    let expected = Err(ParseError::UnexpectedToken(position));
    assert!(actual == expected, "expected {expected:?}, got {actual:?}");
}

#[test]
fn literal_unicode_in_object_keys_and_array_values() {
    let input = r#"{"café":["é","中","🦀"]}"#;
    let expected = object(vec![(
        "café",
        array(vec![text("é"), text("中"), text("🦀")]),
    )]);

    assert_eq!(parse_json(input, Some(2)), Ok(expected));
}

#[test]
fn literal_and_escaped_unicode_have_equal_values() {
    let input = r#"["é中🦀","\u00E9\u4e2d\uD83E\uDD80"]"#;
    let expected = array(vec![text("é中🦀"), text("é中🦀")]);

    assert_eq!(parse_json(input, Some(1)), Ok(expected));
}

#[test]
fn trailing_input_position_after_literal_unicode_is_a_byte_offset() {
    let input = "\"é中🦀\"true";
    let position = input.find("true").unwrap();

    assert_eq!(
        parse_json(input, None),
        Err(ParseError::UnexpectedToken(position))
    );
}

mod recursion_depth_tests {
    use super::*;
    use JsonElement::{Boolean, FloatNumber, Null};

    #[test]
    fn zero_limit_allows_all_scalar_types() {
        for (input, expected) in [
            ("null", Null),
            ("true", Boolean(true)),
            ("false", Boolean(false)),
            ("42", IntegerNumber(42)),
            ("-1.5", FloatNumber(-1.5)),
            (r#""[{}]\"""#, text("[{}]\"")),
        ] {
            assert_eq!(parse_json(input, Some(0)), Ok(expected), "{input:?}");
        }
    }

    #[test]
    fn zero_limit_rejects_even_empty_containers() {
        for (input, position) in [("[]", 0), ("{}", 0), (" \n []", 3), ("\t{}", 1)] {
            assert_eq!(
                parse_json(input, Some(0)),
                Err(ParseError::MaxRecursionDepthReached(position)),
                "{input:?}"
            );
        }
    }

    #[test]
    fn one_limit_allows_containers_with_scalar_contents() {
        for (input, expected) in [
            ("[]", array(vec![])),
            ("{}", object(vec![])),
            (
                "[null,true,false,42,1.5]",
                array(vec![
                    Null,
                    Boolean(true),
                    Boolean(false),
                    IntegerNumber(42),
                    FloatNumber(1.5),
                ]),
            ),
            (
                r#"{"[{}]":"[{}]","n":null}"#,
                object(vec![("[{}]", text("[{}]")), ("n", Null)]),
            ),
        ] {
            assert_eq!(parse_json(input, Some(1)), Ok(expected), "{input:?}");
        }
    }

    #[test]
    fn arrays_and_objects_share_the_same_nesting_limit() {
        for (input, position, expected) in [
            ("[[]]", 1, array(vec![array(vec![])])),
            ("[{}]", 1, array(vec![object(vec![])])),
            (r#"{"x":[]}"#, 5, object(vec![("x", array(vec![]))])),
            (r#"{"x":{}}"#, 5, object(vec![("x", object(vec![]))])),
        ] {
            assert_eq!(parse_json(input, Some(2)), Ok(expected), "{input:?}");
            assert_eq!(
                parse_json(input, Some(1)),
                Err(ParseError::MaxRecursionDepthReached(position)),
                "{input:?}"
            );
        }
    }

    #[test]
    fn siblings_reuse_depth_after_deeper_containers_close() {
        let input = r#"{"a":[[],{}],"b":{},"c":[{},[]]}"#;
        let expected = object(vec![
            ("a", array(vec![array(vec![]), object(vec![])])),
            ("b", object(vec![])),
            ("c", array(vec![object(vec![]), array(vec![])])),
        ]);

        assert_eq!(parse_json(input, Some(3)), Ok(expected));
    }

    #[test]
    fn unlimited_and_exact_boundary_preserve_every_nested_value() {
        let input = format!("{}null{}", "[".repeat(32), "]".repeat(32));
        let expected = (0..32).fold(Null, |value, _| array(vec![value]));

        for limit in [None, Some(32), Some(33), Some(usize::MAX)] {
            let actual = parse_json(&input, limit).expect("nesting must be within the limit");
            assert_eq!(actual, expected, "{limit:?}");
        }
        assert_eq!(
            parse_json(&input, Some(31)),
            Err(ParseError::MaxRecursionDepthReached(31))
        );
    }

    #[test]
    fn depth_error_reports_opening_byte_after_whitespace() {
        let input = " \n [ \t { \"x\": [] } ] ";
        let position = input.find("[]").unwrap();

        assert_eq!(
            parse_json(input, Some(2)),
            Err(ParseError::MaxRecursionDepthReached(position))
        );
    }

    #[test]
    fn syntax_eof_and_trailing_input_errors_survive_enabled_limit() {
        for (input, expected) in [
            ("[truX]", ParseError::UnexpectedToken(4)),
            ("[", ParseError::UnexpectedEOF(1)),
            ("[]true", ParseError::UnexpectedToken(2)),
        ] {
            assert_eq!(parse_json(input, Some(1)), Err(expected), "{input:?}");
        }
    }

    #[test]
    fn nested_fixture_accepts_exact_limit_and_rejects_one_less() {
        let input = read_fixture("nested.json");
        let expected = parse_json(&input, None).expect("fixture must be valid JSON");
        // 24 object/children-array pairs, then the leaf object, leaf array,
        // and its empty object or array: 24 * 2 + 3 = 51 containers.
        assert_eq!(parse_json(&input, Some(51)), Ok(expected));
        let position = input.find("{}").expect("fixture must contain empty object");
        assert_eq!(
            parse_json(&input, Some(50)),
            Err(ParseError::MaxRecursionDepthReached(position))
        );
    }
}
