use std::{cmp::max, collections::HashMap, eprintln};
#[cfg(test)]
mod unit_tests;
mod system_tests;

pub struct Parser<'a> {
    input: &'a [u8],
    position: usize,
}

#[derive(Debug, PartialEq)]
pub enum ParseError {
    UnexpectedError,
    NotSupported(String),
    UnexpectedToken(usize),
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
    fn parse_json_bool(&mut self) -> Result<JsonElement, ParseError> {
        const TRUE_LITERAL: &str = "true";
        const FALSE_LITERAL: &str = "false";
        let try_true = self.input[self.position] == b't';
        let literal = if try_true {
            TRUE_LITERAL
        } else {
            FALSE_LITERAL
        };
        if self.input.len() < self.position + 4 {
            return Err(ParseError::UnexpectedToken(self.input.len()));
        }
        for (i, c) in literal.bytes().enumerate() {
            if self.input[self.position + i] != c {
                return Err(ParseError::UnexpectedToken(self.position + i));
            }
        }
        self.position += literal.len();
        return Ok(JsonElement::Boolean(try_true));
    }

    fn parse_json_string(&mut self) -> Result<JsonElement, ParseError> {
        if self.check_eof()? && self.input[self.position] != b'"' {
            return Err(ParseError::UnexpectedToken(self.position));
        }
        self.position += 1;
        let mut output = String::with_capacity(32);
        let mut is_escaped = false;
        while self.position < self.input.len() && !(!is_escaped && self.input[self.position] == b'"') {
            if is_escaped {
                match self.input[self.position] {
                    b'\\' => {
                        output.push('\\');
                    }
                    b'n' => { 
                        output.push('\n');
                    }
                    b'"' => { 
                        output.push('"');
                    }
                    b'u' => { 
                        return Err(ParseError::NotSupported("Unicode not supported yet".to_string()));
                    }
                    _ => {
                        return Err(ParseError::UnexpectedToken(self.position));
                    }
                }
                is_escaped = false;
            } else {
                match self.input[self.position] {
                    b'\\' => is_escaped = true,
                    b'\0' => {
                        return Err(ParseError::UnexpectedToken(self.position));
                    }
                    x => {
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
            if self.input.len() <= self.position {
                Err(ParseError::UnexpectedToken(self.position))
            } else {
                if self.input[self.position] == expected {
                    self.position += 1;
                    Ok(())
                } else {
                Err(ParseError::UnexpectedToken(self.position))
                }
            }
    }
    fn ignore_whitespace(&mut self) -> () {
            while self.input.len() > self.position && self.input[self.position].is_ascii_whitespace() {
                self.position += 1;
            }
    }


    fn parse_json_array(&mut self) -> Result<JsonElement, ParseError> {
        self.accept(b'[')?;
        let mut output = Vec::<JsonElement>::with_capacity(16);
        self.ignore_whitespace();
        while self.position < self.input.len() && !(self.input[self.position] == b']') {
            if output.len() > 0 {
                self.accept(b',')?;
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
            return Err(ParseError::UnexpectedToken(max(self.input.len(), 0)));
        }
        for (i, c) in NULL_LITERAL.bytes().enumerate() {
            if self.input[self.position + i] != c {
                return Err(ParseError::UnexpectedToken(self.position + i));
            }
        }
        self.position += 4;
        Ok(JsonElement::Null)
    }
    fn check_eof(&self) -> Result<bool, ParseError>{
        if self.position < self.input.len() {
            Ok(true)
        } else {
            Err(ParseError::UnexpectedToken(self.position))
        }
    }

    fn parse_json_object(&mut self) -> Result<JsonElement, ParseError> {
        self.accept(b'{')?;
        let mut output = HashMap::<String, JsonElement>::new();
        self.ignore_whitespace();
        while self.check_eof()? && !(self.input[self.position] == b'}') {
            if output.len() > 0 {
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

    fn parse_json_element(&mut self) -> Result<JsonElement, ParseError> {
        self.check_eof()?;
        match self.input[self.position] {
                b'n' => self.parse_null(),
                b't' => self.parse_json_bool(),
                b'f' => self.parse_json_bool(),
                b'"' => self.parse_json_string(),
                b'[' => self.parse_json_array(),
                b'{' => self.parse_json_object(),
                _ => Err(ParseError::UnexpectedToken(self.position)),
        }
    }
}

pub fn print_json_error(parser: &Parser, err:  &ParseError) {
    if let ParseError::UnexpectedToken(idx) = err {
        if idx.to_owned() >= parser.input.len() {
            eprintln!("Json Parsing Error: Unexpected End of File");
        } else {
            let error_char =  parser.input[idx.to_owned()];
            eprintln!("Json Parsing Error at idx {idx} ({}): UnexpectedToken", char::from(error_char));
        }
    } else {
        eprintln!("Unexpected Json Parsing Error");
    }
}
pub fn parse_json(json_string: &str) -> Result<JsonElement, ParseError> {
    let mut parser = Parser {
        input: json_string.as_bytes(),
        position: 0,
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