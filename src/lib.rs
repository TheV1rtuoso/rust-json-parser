use std::{
    collections::HashMap,
    eprintln,
    num::{ParseFloatError, ParseIntError},
};

use crate::{
    JsonElement::IntegerNumber,
    ParseError::{LeadingZero, UnexpectedToken},
};
#[cfg(test)]
mod system_tests;
#[cfg(test)]
mod unit_tests;

pub struct Parser<'a> {
    input: &'a [u8],
    position: usize,
    max_nesting: Option<usize>,
    recursion_depth: usize,
}

#[derive(Debug, PartialEq)]
pub enum ParseError {
    UnexpectedError,
    MaxRecursionDepthReached(usize),
    FloatParsingError(ParseFloatError),
    IntParsingError(ParseIntError),
    NotSupported(String),
    UnexpectedToken(usize),
    UnexpectedEOF(usize),
    LeadingZero(usize),
    LeadingPlus(usize),
}

#[derive(Debug, PartialEq)]
pub enum JsonElement {
    Null,
    Boolean(bool),
    String(String),
    IntegerNumber(i64),
    FloatNumber(f64),
    Array(Vec<JsonElement>),
    Object(HashMap<String, JsonElement>),
}

impl<'a> Parser<'a> {
    fn curr(&mut self) -> Result<u8, ParseError> {
        if self.position < self.input.len() {
            Ok(self.input[self.position])
        } else {
            Err(ParseError::UnexpectedEOF(self.position))
        }
    }
    fn next(&mut self) -> Result<u8, ParseError> {
        self.position += 1;
        self.curr()
    }
    #[allow(unused)]
    fn peek(&mut self) -> Result<u8, ParseError> {
        if self.position + 1 < self.input.len() {
            Ok(self.input[self.position + 1])
        } else {
            Err(ParseError::UnexpectedToken(self.position))
        }
    }
    fn parse_json_number(&mut self) -> Result<JsonElement, ParseError> {
        assert!(self.check_eof().is_ok());
        let start_idx = self.position;
        let _is_negative = self.accept(b'-').is_ok();
        if self.curr() == Ok(b'0') {
            self.position += 1;
            if self.curr().is_err() {
                return Ok(IntegerNumber(0));
            }
            if self.curr()?.is_ascii_digit() {
                return Err(LeadingZero(self.position));
            }
        } else if self.curr()?.is_ascii_digit() {
            while self.next().is_ok_and(|b| b.is_ascii_digit()) {}
        } else {
            return Err(UnexpectedToken(self.position));
        }
        let is_float = self.accept(b'.').is_ok();
        if is_float {
            if self.curr()?.is_ascii_digit() {
                while self.next().is_ok_and(|b| b.is_ascii_digit()) {}
            } else {
                return Err(UnexpectedToken(self.position));
            }
        }
        let is_exponential = self.accept(b'E').is_ok() || self.accept(b'e').is_ok();
        if is_exponential {
            let _exponential_sign = self.accept(b'-').is_ok() || self.accept(b'+').is_ok();
            if self.curr()?.is_ascii_digit() {
                while self.next().is_ok_and(|b| b.is_ascii_digit()) {}
            } else {
                return Err(UnexpectedToken(self.position));
            }
        }
        if is_float || is_exponential {
            let text = std::str::from_utf8(&self.input[start_idx..self.position])
                .expect("Failed to transform to string");
            let number = text.parse::<f64>().map_err(ParseError::FloatParsingError)?;
            Ok(JsonElement::FloatNumber(number))
        } else {
            let text = std::str::from_utf8(&self.input[start_idx..self.position])
                .expect("Failed to transform to string");
            let number = text.parse::<i64>().map_err(ParseError::IntParsingError)?;
            Ok(JsonElement::IntegerNumber(number))
        }
    }

    fn parse_json_bool(&mut self) -> Result<JsonElement, ParseError> {
        const TRUE_LITERAL: &str = "true";
        const FALSE_LITERAL: &str = "false";
        let try_true = self.input[self.position] == b't';
        let literal = if try_true {
            TRUE_LITERAL
        } else {
            FALSE_LITERAL
        };
        if self.input.len() < self.position + literal.len() {
            return Err(ParseError::UnexpectedEOF(self.input.len()));
        }
        for (i, c) in literal.bytes().enumerate() {
            if self.input[self.position + i] != c {
                return Err(ParseError::UnexpectedToken(self.position + i));
            }
        }
        self.position += literal.len();
        Ok(JsonElement::Boolean(try_true))
    }

    fn parse_json_string(&mut self) -> Result<JsonElement, ParseError> {
        self.accept(b'"')?;
        let mut output = String::with_capacity(16);
        let mut is_escaped = false;
        while self.curr()? != b'"' || is_escaped {
            if is_escaped {
                match self.input[self.position] {
                    b'\\' => {
                        output.push('\\');
                    }
                    b'n' => {
                        output.push('\n');
                    }
                    b'r' => {
                        output.push('\r');
                    }
                    b'/' => {
                        output.push('/');
                    }
                    b't' => {
                        output.push('\t');
                    }
                    b'"' => {
                        output.push('"');
                    }
                    b'u' => {
                        return Err(ParseError::NotSupported(
                            "Unicode not supported yet".to_string(),
                        ));
                    }
                    _ => {
                        return Err(ParseError::UnexpectedToken(self.position));
                    }
                }
                is_escaped = false;
            } else {
                match self.input[self.position] {
                    b'\\' => is_escaped = true,
                    x => {
                        if x < 0x20 {
                            return Err(ParseError::UnexpectedToken(self.position));
                        }
                        output.push(char::from(x));
                    }
                }
            }
            self.position += 1;
        }
        self.accept(b'"')?;
        Ok(JsonElement::String(output))
    }
    fn accept(&mut self, expected: u8) -> Result<(), ParseError> {
        if self.curr()? == expected {
            self.position += 1;
            Ok(())
        } else {
            Err(ParseError::UnexpectedToken(self.position))
        }
    }

    #[allow(clippy::byte_char_slices)]
    fn ignore_whitespace(&mut self) {
        while self.input.len() > self.position
            && [b'\t', b' ', b'\r', b'\n'].contains(&self.input[self.position])
        {
            self.position += 1;
        }
    }

    fn parse_json_array(&mut self) -> Result<JsonElement, ParseError> {
        self.accept(b'[')?;
        let mut output = Vec::<JsonElement>::with_capacity(16);
        self.ignore_whitespace();
        while self.curr()? != b']' {
            if !output.is_empty() && self.accept(b',').is_err() {
                return Err(ParseError::UnexpectedToken(self.position));
            }
            self.ignore_whitespace();
            output.push(self.parse_json_element()?);
            self.ignore_whitespace();
        }
        self.accept(b']')?;
        Ok(JsonElement::Array(output))
    }

    fn parse_null(&mut self) -> Result<JsonElement, ParseError> {
        const NULL_LITERAL: &str = "null";
        if self.input.len() < self.position + 4 {
            return Err(ParseError::UnexpectedEOF(self.input.len()));
        }
        for (i, c) in NULL_LITERAL.bytes().enumerate() {
            if self.input[self.position + i] != c {
                return Err(ParseError::UnexpectedToken(self.position + i));
            }
        }
        self.position += 4;
        Ok(JsonElement::Null)
    }
    fn check_eof(&self) -> Result<bool, ParseError> {
        if self.position < self.input.len() {
            Ok(true)
        } else {
            Err(ParseError::UnexpectedEOF(self.position))
        }
    }

    fn parse_json_object(&mut self) -> Result<JsonElement, ParseError> {
        self.accept(b'{')?;
        let mut output = HashMap::<String, JsonElement>::new();
        self.ignore_whitespace();
        while self.curr()? != b'}' {
            if !output.is_empty() {
                self.accept(b',')?;
                self.ignore_whitespace();
            }
            let key = self.parse_json_string()?;
            self.ignore_whitespace();
            self.accept(b':')?;
            self.ignore_whitespace();
            let value = self.parse_json_element()?;
            if let JsonElement::String(text) = key {
                output.insert(text, value);
            }
            self.ignore_whitespace();
        }
        self.accept(b'}')?;
        Ok(JsonElement::Object(output))
    }

    fn recursion_guard<F>(&mut self, f: F) -> Result<JsonElement, ParseError>
    where
        F: FnOnce(&mut Self) -> Result<JsonElement, ParseError>,
    {
        if let Some(max_depth) = self.max_nesting {
            if self.recursion_depth + 1 > max_depth {
                return Err(ParseError::MaxRecursionDepthReached(self.position));
            }
            self.recursion_depth += 1;
        }
        let res = f(self);
        if self.max_nesting.is_some() {
            self.recursion_depth -= 1;
        }
        res
    }

    fn parse_json_element(&mut self) -> Result<JsonElement, ParseError> {
        self.check_eof()?;
        match self.input[self.position] {
            b'n' => self.parse_null(),
            b't' => self.parse_json_bool(),
            b'f' => self.parse_json_bool(),
            b'"' => self.parse_json_string(),
            b'[' => self.recursion_guard(Parser::parse_json_array),
            b'{' => self.recursion_guard(Parser::parse_json_object),
            b'-' => self.parse_json_number(),
            b'+' => Err(ParseError::LeadingPlus(self.position)),
            _x @ b'0'..=b'9' => self.parse_json_number(),
            _ => Err(ParseError::UnexpectedToken(self.position)),
        }
    }
}

pub fn print_json_error(parser: &Parser, err: &ParseError) {
    if let ParseError::UnexpectedToken(idx) = err {
        if *idx >= parser.input.len() {
            eprintln!("Json Parsing Error: Unexpected End of File");
        } else {
            let error_char = parser.input[idx.to_owned()];
            eprintln!(
                "Json Parsing Error at idx {idx} ({}): UnexpectedToken",
                char::from(error_char)
            );
        }
    } else {
        eprintln!("Unexpected Json Parsing Error");
    }
}
pub fn parse_json(
    json_string: &str,
    max_nesting: Option<usize>,
) -> Result<JsonElement, ParseError> {
    let mut parser = Parser {
        input: json_string.as_bytes(),
        position: 0,
        recursion_depth: 0,
        max_nesting,
    };
    parser.ignore_whitespace();
    let res = parser.parse_json_element()?;
    parser.ignore_whitespace();
    if parser.position == parser.input.len() {
        Ok(res)
    } else {
        Err(ParseError::UnexpectedToken(parser.position))
    }
}
