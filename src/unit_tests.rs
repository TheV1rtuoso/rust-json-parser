use super::*;

mod number_tests {
    use super::*;
    use JsonElement::{Array, FloatNumber};

    fn check(input: &str, expected: JsonElement) {
        let actual = parse_json(input);
        let expected = Ok(expected);
        assert!(
            actual == expected,
            "input {input:?}: expected {expected:?}, got {actual:?}"
        );
    }

    fn check_invalid(input: &str) {
        let actual = parse_json(input);
        assert!(
            actual.is_err(),
            "input {input:?}: expected a parsing error, got {actual:?}"
        );
    }

    #[test]
    fn positive_integers() {
        for (input, value) in [("0", 0), ("7", 7), ("42", 42), ("123456789", 123456789)] {
            check(input, IntegerNumber(value));
        }
    }

    #[test]
    fn negative_integers() {
        for (input, value) in [("-0", 0), ("-1", -1), ("-987654", -987654)] {
            check(input, IntegerNumber(value));
        }
    }

    #[test]
    fn integer_boundaries() {
        check("9223372036854775807", IntegerNumber(i64::MAX));
        check("-9223372036854775808", IntegerNumber(i64::MIN));
    }

    #[test]
    fn decimal_numbers() {
        for (input, value) in [
            ("0.0", 0.0),
            ("1.5", 1.5),
            ("-0.25", -0.25),
            ("12.00", 12.0),
        ] {
            check(input, FloatNumber(value));
        }
    }

    #[test]
    fn exponent_numbers() {
        for (input, value) in [
            ("1e3", 1000.0),
            ("2E+2", 200.0),
            ("5e-1", 0.5),
            ("-2.5e2", -250.0),
            ("0e0", 0.0),
        ] {
            check(input, FloatNumber(value));
        }
    }

    #[test]
    fn negative_float_zero_preserves_sign() {
        let actual = parse_json("-0.0");
        match actual {
            Ok(FloatNumber(value)) => {
                assert_eq!(value, 0.0);
                assert!(
                    value.is_sign_negative(),
                    "expected negative zero, got {value:?}"
                );
            }
            other => panic!("expected Ok(FloatNumber(-0.0)), got {other:?}"),
        }
    }

    #[test]
    fn surrounding_whitespace() {
        check(" \t42\r\n", IntegerNumber(42));
        check("\n -1.5 \t", FloatNumber(-1.5));
    }

    #[test]
    fn numbers_in_arrays() {
        check(
            "[0,-2,1.5,3e2]",
            Array(vec![
                IntegerNumber(0),
                IntegerNumber(-2),
                FloatNumber(1.5),
                FloatNumber(300.0),
            ]),
        );
    }

    #[test]
    fn numbers_in_objects() {
        check(
            r#"{"count":42,"ratio":-0.5}"#,
            JsonElement::Object(HashMap::from([
                ("count".to_owned(), IntegerNumber(42)),
                ("ratio".to_owned(), FloatNumber(-0.5)),
            ])),
        );
    }

    #[test]
    fn stops_before_delimiter_at_current_position() {
        let mut parser = Parser {
            input: b"xx12.5,tail",
            position: 2,
        };
        let actual = parser.parse_json_number();
        let expected = Ok(FloatNumber(12.5));
        assert!(actual == expected, "expected {expected:?}, got {actual:?}");
        assert_eq!(
            parser.position, 6,
            "comma must remain for the containing parser"
        );
    }

    #[test]
    fn rejects_leading_zeros_and_plus() {
        for input in ["01", "00", "-01", "00.5", "+1"] {
            check_invalid(input);
        }
    }

    #[test]
    fn rejects_incomplete_and_malformed_numbers() {
        for input in [
            "-", ".5", "-.5", "1.", "1..2", "--1", "1e", "1e+", "1e-", "1e+-2", "1.2.3",
        ] {
            check_invalid(input);
        }
    }

    #[test]
    fn rejects_trailing_content() {
        for input in ["12abc", "1 2", "1true", "1,", "1]"] {
            check_invalid(input);
        }
    }

    #[test]
    fn rejects_non_json_numeric_literals() {
        for input in ["NaN", "Infinity", "-Infinity", "0x10"] {
            check_invalid(input);
        }
    }

    #[test]
    fn invalid_number_inside_array() {
        for input in ["[1.]", "[01]", "[1e+]", "[--1]"] {
            check_invalid(input);
        }
    }
}

#[cfg(test)]
mod object_tests {
    use super::*;
    use JsonElement::{Array, Boolean, Null};

    fn object(entries: Vec<(&str, JsonElement)>) -> JsonElement {
        JsonElement::Object(
            entries
                .into_iter()
                .map(|(key, value)| (key.to_owned(), value))
                .collect(),
        )
    }

    fn check(input: &str, expected: Result<JsonElement, ParseError>) {
        let actual = parse_json(input);
        assert!(
            actual == expected,
            "input {input:?}: expected {expected:?}, got {actual:?}"
        );
    }

    #[test]
    fn empty() {
        check("{}", Ok(object(vec![])));
    }

    #[test]
    fn single_entry() {
        check(r#"{"a":null}"#, Ok(object(vec![("a", Null)])));
    }

    #[test]
    fn multiple_entries() {
        check(
            r#"{"a":null,"b":true,"c":false}"#,
            Ok(object(vec![
                ("a", Null),
                ("b", Boolean(true)),
                ("c", Boolean(false)),
            ])),
        );
    }

    #[test]
    fn string_keys_and_values() {
        check(
            r#"{"":"","a\"b":"hello","{}:,":"[]"}"#,
            Ok(object(vec![
                ("", JsonElement::String("".to_owned())),
                ("a\"b", JsonElement::String("hello".to_owned())),
                ("{}:,", JsonElement::String("[]".to_owned())),
            ])),
        );
    }

    #[test]
    fn nested_objects() {
        check(
            r#"{"outer":{"inner":true},"empty":{}}"#,
            Ok(object(vec![
                ("outer", object(vec![("inner", Boolean(true))])),
                ("empty", object(vec![])),
            ])),
        );
    }

    #[test]
    fn arrays_and_objects() {
        check(
            r#"{"items":[null,{"ok":true},[]]}"#,
            Ok(object(vec![(
                "items",
                Array(vec![
                    Null,
                    object(vec![("ok", Boolean(true))]),
                    Array(vec![]),
                ]),
            )])),
        );
    }

    #[test]
    fn whitespace() {
        check("{ \t\n\r }", Ok(object(vec![])));
        check(
            "{ \"a\" \t: \nnull , \"b\" : false }",
            Ok(object(vec![("a", Null), ("b", Boolean(false))])),
        );
    }

    #[test]
    fn consumes_closing_brace() {
        let mut parser = Parser {
            input: b"xx{\"a\":null}tail",
            position: 2,
        };
        let actual = parser.parse_json_object();
        let expected = Ok(object(vec![("a", Null)]));
        assert!(actual == expected, "expected {expected:?}, got {actual:?}");
        assert_eq!(parser.position, 12, "closing brace must be consumed");
    }

    #[test]
    fn rejects_non_string_keys() {
        for input in ["{true:null}", "{null:true}", "{[]:null}", "{a:null}"] {
            check(input, Err(ParseError::UnexpectedToken(1)));
        }
    }

    #[test]
    fn missing_colon() {
        check(r#"{"a"true}"#, Err(ParseError::UnexpectedToken(4)));
    }

    #[test]
    fn missing_value() {
        check(r#"{"a":}"#, Err(ParseError::UnexpectedToken(5)));
    }

    #[test]
    fn invalid_separators() {
        for (input, position) in [
            (r#"{,"a":null}"#, 1),
            (r#"{"a":null,}"#, 10),
            (r#"{"a":null,,"b":true}"#, 10),
            (r#"{"a":null"b":true}"#, 9),
        ] {
            check(input, Err(ParseError::UnexpectedToken(position)));
        }
    }

    #[test]
    fn incomplete_input() {
        for input in [
            "{",
            "{\"a",
            "{\"a\"",
            "{\"a\":",
            "{\"a\":null",
            "{\"a\":null,",
        ] {
            check(input, Err(ParseError::UnexpectedEOF(input.len())));
        }
    }

    #[test]
    fn propagates_value_error() {
        check(r#"{"a":truX}"#, Err(ParseError::UnexpectedToken(8)));
    }

    #[test]
    fn rejects_trailing_content() {
        check("{}true", Err(ParseError::UnexpectedToken(2)));
        check("{}}", Err(ParseError::UnexpectedToken(2)));
    }
}

#[cfg(test)]
mod array_tests {
    use super::*;
    use JsonElement::{Array, Boolean, Null};

    fn check(input: &str, expected: Result<JsonElement, ParseError>) {
        let actual = parse_json(input);
        assert!(
            actual == expected,
            "input {input:?}: expected {expected:?}, got {actual:?}"
        );
    }

    #[test]
    fn empty() {
        check("[]", Ok(Array(vec![])));
    }

    #[test]
    fn single_literal() {
        check("[null]", Ok(Array(vec![Null])));
        check("[true]", Ok(Array(vec![Boolean(true)])));
        check("[false]", Ok(Array(vec![Boolean(false)])));
    }

    #[test]
    fn multiple_literals() {
        check(
            "[null,true,false,null]",
            Ok(Array(vec![Null, Boolean(true), Boolean(false), Null])),
        );
    }

    #[test]
    fn strings() {
        check(
            r#"["hello","",",[]"]"#,
            Ok(Array(vec![
                JsonElement::String("hello".to_owned()),
                JsonElement::String("".to_owned()),
                JsonElement::String(",[]".to_owned()),
            ])),
        );
    }

    #[test]
    fn escaped_string_and_following_value() {
        check(
            r#"["a\"b",true]"#,
            Ok(Array(vec![
                JsonElement::String("a\"b".to_owned()),
                Boolean(true),
            ])),
        );
    }

    #[test]
    fn nested_empty_arrays() {
        check("[[],[]]", Ok(Array(vec![Array(vec![]), Array(vec![])])));
    }

    #[test]
    fn nested_mixed_values() {
        check(
            "[[null,true],[false],null]",
            Ok(Array(vec![
                Array(vec![Null, Boolean(true)]),
                Array(vec![Boolean(false)]),
                Null,
            ])),
        );
    }

    #[test]
    fn deep_nesting() {
        check(
            "[[[[null]]]]",
            Ok(Array(vec![Array(vec![Array(vec![Array(vec![Null])])])])),
        );
    }

    #[test]
    fn whitespace() {
        check("[ ]", Ok(Array(vec![])));
        check(
            "[ \ttrue ,\n null,\r false ]",
            Ok(Array(vec![Boolean(true), Null, Boolean(false)])),
        );
    }

    #[test]
    fn consumes_closing_bracket() {
        for input in ["[]", "[null]", "[[null]]"] {
            let mut parser = Parser {
                input: input.as_bytes(),
                position: 0,
            };
            let actual = parser.parse_json_array();
            assert!(
                actual.is_ok(),
                "input {input:?}: expected success, got {actual:?}"
            );
            assert_eq!(
                parser.position,
                input.len(),
                "input {input:?}: closing bracket must be consumed"
            );
        }
    }

    #[test]
    fn starts_at_current_position() {
        let mut parser = Parser {
            input: b"xx[null]tail",
            position: 2,
        };
        let actual = parser.parse_json_array();
        let expected = Ok(Array(vec![Null]));
        assert!(actual == expected, "expected {expected:?}, got {actual:?}");
        assert_eq!(parser.position, 8);
    }

    #[test]
    fn missing_closing_bracket() {
        for input in ["[", "[null", "[true,", "[[null]"] {
            check(input, Err(ParseError::UnexpectedEOF(input.len())));
        }
    }

    #[test]
    fn invalid_commas() {
        for (input, position) in [
            ("[,]", 1),
            ("[,null]", 1),
            ("[null,]", 6),
            ("[null,,true]", 6),
        ] {
            check(input, Err(ParseError::UnexpectedToken(position)));
        }
    }

    #[test]
    fn missing_comma() {
        for (input, position) in [("[nulltrue]", 5), ("[true false]", 6), ("[[][]]", 3)] {
            check(input, Err(ParseError::UnexpectedToken(position)));
        }
    }

    #[test]
    fn propagates_element_errors() {
        for (input, position) in [("[truX]", 4), ("[nulX]", 4), ("[?]", 1), (r#"["\q"]"#, 3)] {
            check(input, Err(ParseError::UnexpectedToken(position)));
        }
    }

    #[test]
    fn rejects_trailing_content() {
        for (input, position) in [("[]true", 2), ("[]]", 2), ("[null]x", 6)] {
            check(input, Err(ParseError::UnexpectedToken(position)));
        }
    }
}

#[test]
fn test_json_string() {
    for (input, text) in [
        (r#""hello""#, "hello"),
        (r#""""#, ""),
        (r#""hello world""#, "hello world"),
        //(r#""café 🦀""#, "café 🦀"), TODO implement
    ] {
        let actual = parse_json(input);
        let expected = Ok(JsonElement::String(text.to_owned()));
        assert!(
            actual == expected,
            "input {input:?}: expected {expected:?}, got {actual:?}"
        );
    }
}

#[test]
fn test_json_string_unterminated() {
    for input in ["\"", "\"hello", "\"café"] {
        let actual = parse_json(input);
        let expected = Err(ParseError::UnexpectedEOF(input.len()));
        assert!(
            actual == expected,
            "input {input:?}: expected {expected:?}, got {actual:?}"
        );
    }
}

#[test]
fn test_json_string_escaped_quote() {
    let actual = parse_json(r#""say \"hello\"""#);
    let expected = Ok(JsonElement::String("say \"hello\"".to_owned()));
    assert!(actual == expected, "expected {expected:?}, got {actual:?}");
}

#[test]
fn test_json_string_escaped_newline() {
    let actual = parse_json(r#""hello\nworld""#);
    let expected = Ok(JsonElement::String("hello\nworld".to_owned()));
    assert!(actual == expected, "expected {expected:?}, got {actual:?}");
}

#[test]
fn test_json_string_escaped_backslash() {
    let actual = parse_json(r#""a\\b""#);
    let expected = Ok(JsonElement::String("a\\b".to_owned()));
    assert!(actual == expected, "expected {expected:?}, got {actual:?}");
}

#[test]
fn test_json_string_error_invalid_escape() {
    for input in [r#""\q""#, r#""\x41""#] {
        let actual = parse_json(input);
        // The byte after the backslash must name a valid JSON escape.
        let expected = Err(ParseError::UnexpectedToken(2));
        assert!(
            actual == expected,
            "input {input:?}: expected {expected:?}, got {actual:?}"
        );
    }
}

#[test]
#[ignore = "Unicode escapes not implemented yet"]
fn test_json_string_error_invalid_unicode_escape() {
    for (input, position) in [(r#""\u12G4""#, 5), (r#""\u12""#, 5)] {
        let actual = parse_json(input);
        // A Unicode escape requires four hexadecimal digits.
        let expected = Err(ParseError::UnexpectedToken(position));
        assert!(
            actual == expected,
            "input {input:?}: expected {expected:?}, got {actual:?}"
        );
    }
}

#[test]
fn test_json_string_error_unescaped_control_character() {
    for input in ["\"a\nb\"", "\"a\tb\"", "\"a\rb\"", "\"a\0b\""] {
        let actual = parse_json(input);
        let expected = Err(ParseError::UnexpectedToken(2));
        assert!(
            actual == expected,
            "input {input:?}: expected {expected:?}, got {actual:?}"
        );
    }
}

#[test]
fn test_json_string_error_dangling_escape() {
    let input = "\"hello\\";
    let actual = parse_json(input);
    let expected = Err(ParseError::UnexpectedEOF(input.len()));
    assert!(actual == expected, "expected {expected:?}, got {actual:?}");
}

#[test]
fn test_json_string_error_escaped_quote_without_closing_quote() {
    let input = r#""hello\""#;
    let actual = parse_json(input);
    // The escaped quote is string content, so the closing quote is missing.
    let expected = Err(ParseError::UnexpectedEOF(input.len()));
    assert!(actual == expected, "expected {expected:?}, got {actual:?}");
}

#[test]
fn parse_null() {
    let actual = parse_json("n");
    let expected: Result<JsonElement, ParseError> = Err(ParseError::UnexpectedEOF(1));
    assert!(actual == expected, "expected {expected:?}, got {actual:?}");
    let actual = parse_json("nu");
    let expected: Result<JsonElement, ParseError> = Err(ParseError::UnexpectedEOF(2));
    assert!(actual == expected, "expected {expected:?}, got {actual:?}");
    let actual = parse_json("null");
    let expected: Result<JsonElement, ParseError> = Ok(JsonElement::Null);
    assert!(actual == expected, "expected {expected:?}, got {actual:?}");
}

#[test]
fn parse_bool() {
    let actual = parse_json("t");
    let expected: Result<JsonElement, ParseError> = Err(ParseError::UnexpectedEOF(1));
    assert!(actual == expected, "expected {expected:?}, got {actual:?}");
    let actual = parse_json("fa");
    let expected: Result<JsonElement, ParseError> = Err(ParseError::UnexpectedEOF(2));
    assert!(actual == expected, "expected {expected:?}, got {actual:?}");
    let actual = parse_json("true");
    let expected: Result<JsonElement, ParseError> = Ok(JsonElement::Boolean(true));
    assert!(actual == expected, "expected {expected:?}, got {actual:?}");
    let actual = parse_json("false");
    let expected: Result<JsonElement, ParseError> = Ok(JsonElement::Boolean(false));
    assert!(actual == expected, "expected {expected:?}, got {actual:?}");
}
