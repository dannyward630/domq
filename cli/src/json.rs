//! A few hand-written JSON helpers (the CLI has no `serde`): writers for the
//! output formats, and a small parser for the messages `domq mcp` reads.

/// `s` as a JSON string literal, quotes included.
pub fn string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    push_string(&mut out, s);
    out
}

/// Append `s` to `out` as a JSON string literal.
pub fn push_string(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// A JSON string, or `null`.
pub fn opt_string(s: Option<&str>) -> String {
    s.map_or_else(|| "null".to_string(), string)
}

/// A JSON array of already-encoded values.
pub fn array<I: IntoIterator<Item = String>>(items: I) -> String {
    let items: Vec<String> = items.into_iter().collect();
    format!("[{}]", items.join(","))
}

/// A JSON array of strings.
pub fn string_array<'s, I: IntoIterator<Item = &'s str>>(items: I) -> String {
    array(items.into_iter().map(string))
}

/// Builds a JSON object, keys in insertion order.
#[derive(Default)]
pub struct Object {
    out: String,
}

impl Object {
    /// Add `key` with an already-encoded JSON `value`.
    pub fn field(&mut self, key: &str, value: &str) -> &mut Self {
        self.out.push(if self.out.is_empty() { '{' } else { ',' });
        push_string(&mut self.out, key);
        self.out.push(':');
        self.out.push_str(value);
        self
    }

    /// The finished object.
    pub fn finish(&self) -> String {
        if self.out.is_empty() {
            "{}".to_string()
        } else {
            format!("{}}}", self.out)
        }
    }
}

/// A parsed JSON value (see [`parse`]).
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    /// The number exactly as written, so a request id echoes back unchanged.
    Number(String),
    String(String),
    Array(Vec<Value>),
    /// Members in document order; [`Value::get`] returns the last of a
    /// repeated key.
    Object(Vec<(String, Value)>),
}

impl Value {
    /// The member `key` of an object.
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Object(members) => members.iter().rev().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    /// The value as JSON text.
    pub fn to_json(&self) -> String {
        match self {
            Value::Null => "null".to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Number(n) => n.clone(),
            Value::String(s) => string(s),
            Value::Array(items) => array(items.iter().map(Value::to_json)),
            Value::Object(members) => {
                let mut obj = Object::default();
                for (k, v) in members {
                    obj.field(k, &v.to_json());
                }
                obj.finish()
            }
        }
    }
}

/// How deeply arrays and objects may nest: deeper input is rejected instead
/// of overflowing the stack.
const MAX_DEPTH: usize = 128;

/// Parse one JSON value (RFC 8259), with surrounding whitespace.
///
/// Errors name the byte offset. A `\u` escape of a lone surrogate becomes
/// U+FFFD rather than an error.
pub fn parse(text: &str) -> Result<Value, String> {
    let mut p = Parser { text, pos: 0 };
    let value = p.value(0)?;
    p.skip_ws();
    if p.pos < text.len() {
        return Err(p.error("unexpected data after the value"));
    }
    Ok(value)
}

struct Parser<'t> {
    text: &'t str,
    pos: usize,
}

impl Parser<'_> {
    fn error(&self, what: &str) -> String {
        format!("{what} at byte {}", self.pos)
    }

    fn peek(&self) -> Option<u8> {
        self.text.as_bytes().get(self.pos).copied()
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.pos += 1;
        }
    }

    fn eat(&mut self, byte: u8) -> bool {
        let found = self.peek() == Some(byte);
        if found {
            self.pos += 1;
        }
        found
    }

    fn literal(&mut self, word: &str, value: Value) -> Result<Value, String> {
        if self
            .text
            .get(self.pos..)
            .is_some_and(|rest| rest.starts_with(word))
        {
            self.pos += word.len();
            Ok(value)
        } else {
            Err(self.error("invalid literal"))
        }
    }

    fn value(&mut self, depth: usize) -> Result<Value, String> {
        self.skip_ws();
        match self.peek() {
            Some(b'{') => self.object(depth + 1),
            Some(b'[') => self.array(depth + 1),
            Some(b'"') => self.string().map(Value::String),
            Some(b't') => self.literal("true", Value::Bool(true)),
            Some(b'f') => self.literal("false", Value::Bool(false)),
            Some(b'n') => self.literal("null", Value::Null),
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(_) => Err(self.error("unexpected character")),
            None => Err(self.error("unexpected end of input")),
        }
    }

    fn object(&mut self, depth: usize) -> Result<Value, String> {
        if depth > MAX_DEPTH {
            return Err(self.error("nested too deeply"));
        }
        self.pos += 1; // `{`
        let mut members = Vec::new();
        self.skip_ws();
        if self.eat(b'}') {
            return Ok(Value::Object(members));
        }
        loop {
            self.skip_ws();
            if self.peek() != Some(b'"') {
                return Err(self.error("expected a string key"));
            }
            let key = self.string()?;
            self.skip_ws();
            if !self.eat(b':') {
                return Err(self.error("expected ':'"));
            }
            members.push((key, self.value(depth)?));
            self.skip_ws();
            if self.eat(b'}') {
                return Ok(Value::Object(members));
            }
            if !self.eat(b',') {
                return Err(self.error("expected ',' or '}'"));
            }
        }
    }

    fn array(&mut self, depth: usize) -> Result<Value, String> {
        if depth > MAX_DEPTH {
            return Err(self.error("nested too deeply"));
        }
        self.pos += 1; // `[`
        let mut items = Vec::new();
        self.skip_ws();
        if self.eat(b']') {
            return Ok(Value::Array(items));
        }
        loop {
            items.push(self.value(depth)?);
            self.skip_ws();
            if self.eat(b']') {
                return Ok(Value::Array(items));
            }
            if !self.eat(b',') {
                return Err(self.error("expected ',' or ']'"));
            }
        }
    }

    fn number(&mut self) -> Result<Value, String> {
        let start = self.pos;
        let digits = |p: &mut Self| {
            let from = p.pos;
            while matches!(p.peek(), Some(b'0'..=b'9')) {
                p.pos += 1;
            }
            p.pos > from
        };
        self.eat(b'-');
        if !self.eat(b'0') && !digits(self) {
            return Err(self.error("invalid number"));
        }
        if self.eat(b'.') && !digits(self) {
            return Err(self.error("invalid number"));
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.pos += 1;
            if !self.eat(b'+') {
                self.eat(b'-');
            }
            if !digits(self) {
                return Err(self.error("invalid number"));
            }
        }
        Ok(Value::Number(
            self.text.get(start..self.pos).unwrap_or("0").to_string(),
        ))
    }

    /// A string literal, starting at its opening quote.
    fn string(&mut self) -> Result<String, String> {
        self.pos += 1; // `"`
        let mut out = String::new();
        loop {
            // Copy the run up to the next quote, backslash or control
            // character in one go. Those are ASCII, so `pos` stays on a char
            // boundary.
            let start = self.pos;
            while matches!(self.peek(), Some(b) if b != b'"' && b != b'\\' && b >= 0x20) {
                self.pos += 1;
            }
            out.push_str(self.text.get(start..self.pos).unwrap_or(""));
            match self.peek() {
                Some(b'"') => {
                    self.pos += 1;
                    return Ok(out);
                }
                Some(b'\\') => {
                    self.pos += 1;
                    self.escape(&mut out)?;
                }
                Some(_) => return Err(self.error("control character in string")),
                None => return Err(self.error("unterminated string")),
            }
        }
    }

    /// One escape sequence, after its backslash.
    fn escape(&mut self, out: &mut String) -> Result<(), String> {
        let Some(b) = self.peek() else {
            return Err(self.error("unterminated string"));
        };
        self.pos += 1;
        out.push(match b {
            b'"' => '"',
            b'\\' => '\\',
            b'/' => '/',
            b'b' => '\u{8}',
            b'f' => '\u{c}',
            b'n' => '\n',
            b'r' => '\r',
            b't' => '\t',
            b'u' => {
                let unit = self.hex4()?;
                let code = if (0xD800..0xDC00).contains(&unit)
                    && self
                        .text
                        .get(self.pos..)
                        .is_some_and(|r| r.starts_with("\\u"))
                {
                    // A high surrogate: combine it with a following low one.
                    let save = self.pos;
                    self.pos += 2;
                    let low = self.hex4()?;
                    if (0xDC00..0xE000).contains(&low) {
                        0x10000 + ((unit - 0xD800) << 10) + (low - 0xDC00)
                    } else {
                        self.pos = save;
                        unit
                    }
                } else {
                    unit
                };
                char::from_u32(code).unwrap_or(char::REPLACEMENT_CHARACTER)
            }
            _ => return Err(self.error("invalid escape")),
        });
        Ok(())
    }

    fn hex4(&mut self) -> Result<u32, String> {
        let digits = self
            .text
            .get(self.pos..self.pos + 4)
            .filter(|d| d.bytes().all(|b| b.is_ascii_hexdigit()))
            .ok_or_else(|| self.error("invalid \\u escape"))?;
        self.pos += 4;
        u32::from_str_radix(digits, 16).map_err(|_| self.error("invalid \\u escape"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes() {
        assert_eq!(string("a\"b\\c\n\u{1}é"), r#""a\"b\\c\n\u0001é""#);
    }

    #[test]
    fn objects_and_arrays() {
        assert_eq!(Object::default().finish(), "{}");
        let mut o = Object::default();
        o.field("a", "1").field("b\"", &opt_string(None));
        assert_eq!(o.finish(), r#"{"a":1,"b\"":null}"#);
        assert_eq!(string_array(["x", "y"]), r#"["x","y"]"#);
        assert_eq!(array(Vec::new()), "[]");
    }

    #[test]
    fn parses_values() {
        let v = parse(
            r#" {"a": [1, -2.5e3, true, false, null], "b": "x\"\u00e9\ud83d\ude00\n", "a": 0} "#,
        )
        .unwrap();
        assert_eq!(v.get("a"), Some(&Value::Number("0".into())));
        assert_eq!(v.get("b").and_then(Value::as_str), Some("x\"é😀\n"));
        assert_eq!(parse("[]").unwrap(), Value::Array(Vec::new()));
        assert_eq!(
            parse(r#""\ud800x""#).unwrap(),
            Value::String("\u{fffd}x".into())
        );
    }

    #[test]
    fn round_trips() {
        let text = r#"{"id":7,"s":"a\"b","n":[null,true,1.5e-3],"o":{}}"#;
        assert_eq!(parse(text).unwrap().to_json(), text);
    }

    #[test]
    fn rejects_bad_json() {
        for bad in [
            "",
            "{",
            "[1,]",
            "{\"a\" 1}",
            "01",
            "1.",
            "-",
            "\"a",
            "\"\u{1}\"",
            "nul",
            "[1] x",
            "{1:2}",
            "\"\\x\"",
            "\"\\u12\"",
        ] {
            assert!(parse(bad).is_err(), "{bad:?}");
        }
        let deep = "[".repeat(MAX_DEPTH + 1) + &"]".repeat(MAX_DEPTH + 1);
        assert!(parse(&deep).unwrap_err().contains("nested too deeply"));
        let ok = "[".repeat(MAX_DEPTH) + &"]".repeat(MAX_DEPTH);
        assert!(parse(&ok).is_ok());
    }
}
