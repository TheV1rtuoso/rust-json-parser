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
    JsonElement::Object(entries.into_iter().map(|(key, value)| (key.to_owned(), value)).collect())
}

fn check_fixture(name: &str, expected: JsonElement) {
    let input = read_fixture(name);
    let actual = parse_json(&input);
    let expected = Ok(expected);
    assert!(actual == expected, "fixture {name}: expected {expected:?}, got {actual:?}");
}

#[test]
fn catalog_file_preserves_all_records() {
    use JsonElement::{Boolean, Null};
    let records = (0..120).map(|i| {
        object(vec![
            ("id", text(format!("item-{i:03}"))),
            ("active", Boolean(i % 2 == 0)),
            ("description", text(format!("Item {i}: \"quoted\"\nC:\\data"))),
            ("metadata", object(vec![
                ("aliases", array(vec![text(format!("alias-{i}")), text(""), text("[],:{}")])),
                ("archived", Boolean(false)),
                ("owner", Null),
            ])),
            ("groups", array(vec![
                array(vec![Boolean(true), Boolean(false)]),
                array(vec![]),
                array(vec![Null, text(format!("group-{}", i % 7))]),
            ])),
        ])
    }).collect();
    check_fixture("catalog.json", object(vec![
        ("name", text("Example catalog")),
        ("records", array(records)),
        ("empty", object(vec![])),
    ]));
}

#[test]
fn events_file_preserves_order_and_payloads() {
    use JsonElement::{Boolean, Null};
    let events = (0..256).map(|i| {
        object(vec![
            ("sequence", text(i.to_string())),
            ("accepted", Boolean(i % 3 == 0)),
            ("payload", object(vec![
                ("tags", array(vec![text("event"), text(format!("batch-{}", i % 8))])),
                ("value", Null),
                ("message", text(format!("Event {i}\n\"received\""))),
                ("matrix", array(vec![
                    array(vec![Boolean(false)]),
                    array(vec![Boolean(true), Null]),
                    array(vec![]),
                ])),
            ])),
        ])
    }).collect();
    check_fixture("events.json", array(events));
}

#[test]
fn nested_file_preserves_every_level() {
    use JsonElement::{Boolean, Null};
    let mut expected = object(vec![("leaf", array(vec![
        text("done"), Boolean(true), Boolean(false), Null, object(vec![]), array(vec![]),
    ]))]);
    for i in (0..24).rev() {
        expected = object(vec![
            ("level", text(i.to_string())),
            ("children", array(vec![expected, object(vec![("empty", array(vec![]))])])),
            ("enabled", Boolean(true)),
        ]);
    }
    check_fixture("nested.json", expected);
}

#[test]
fn missing_colon_file_reports_offending_byte() {
    let input = read_fixture("invalid_missing_colon.json");
    let position = input.find("\"active\" true").expect("fixture must contain malformed member")
        + "\"active\" ".len();
    let actual = parse_json(&input);
    let expected = Err(ParseError::UnexpectedToken(position));
    assert!(actual == expected, "expected {expected:?}, got {actual:?}");
}

#[test]
fn truncated_file_reports_end_of_input() {
    let input = read_fixture("invalid_truncated.json");
    let actual = parse_json(&input);
    let expected = Err(ParseError::UnexpectedToken(input.len()));
    assert!(actual == expected, "expected {expected:?}, got {actual:?}");
}

#[test]
fn trailing_value_file_reports_second_value() {
    let input = read_fixture("invalid_trailing_value.json");
    let position = input.rfind("false").expect("fixture must contain trailing value");
    let actual = parse_json(&input);
    let expected = Err(ParseError::UnexpectedToken(position));
    assert!(actual == expected, "expected {expected:?}, got {actual:?}");
}
