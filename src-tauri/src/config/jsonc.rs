//! Span-preserving JSON / JSONC reader and minimal editor.
//!
//! Edits replace, insert or remove only the bytes of one member; everything else (comments,
//! ordering, whitespace, unknown keys) is preserved byte for byte. Strict mode rejects comments
//! and trailing commas so a strict-JSON file (Claude Code settings) never becomes invalid JSON.

use crate::core::{NexusError, NexusResult};
use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dialect {
    Json,
    Jsonc,
}

#[derive(Clone, Debug)]
pub struct Member {
    pub key: String,
    /// Byte offset of the opening quote of the key.
    pub start: usize,
    pub value: Node,
}

#[derive(Clone, Debug)]
pub enum Node {
    Object { start: usize, end: usize, members: Vec<Member> },
    Array { start: usize, end: usize, items: Vec<Node> },
    Scalar { start: usize, end: usize, value: Value },
}

impl Node {
    pub fn span(&self) -> (usize, usize) {
        match self {
            Node::Object { start, end, .. } | Node::Array { start, end, .. } | Node::Scalar { start, end, .. } => (*start, *end),
        }
    }
    pub fn to_value(&self) -> Value {
        match self {
            Node::Scalar { value, .. } => value.clone(),
            Node::Array { items, .. } => Value::Array(items.iter().map(Node::to_value).collect()),
            Node::Object { members, .. } => {
                // Duplicate keys: the last occurrence wins, matching common JSON parsers.
                let mut map = serde_json::Map::new();
                for m in members {
                    map.insert(m.key.clone(), m.value.to_value());
                }
                Value::Object(map)
            }
        }
    }
    pub fn member(&self, key: &str) -> Option<&Member> {
        match self {
            Node::Object { members, .. } => members.iter().rev().find(|m| m.key == key),
            _ => None,
        }
    }
    pub fn get(&self, path: &[&str]) -> Option<&Node> {
        let mut node = self;
        for key in path {
            node = &node.member(key)?.value;
        }
        Some(node)
    }
}

#[derive(Clone, Debug)]
pub struct Document {
    pub text: String,
    pub root: Node,
}

struct Parser<'a> {
    b: &'a [u8],
    i: usize,
    dialect: Dialect,
    depth: usize,
}

fn err(code: &'static str, at: usize) -> NexusError {
    NexusError::new(code).with("offset", at.to_string())
}

impl<'a> Parser<'a> {
    fn ws(&mut self) -> NexusResult<()> {
        loop {
            while self.i < self.b.len() && matches!(self.b[self.i], b' ' | b'\t' | b'\n' | b'\r') {
                self.i += 1;
            }
            if self.i + 1 < self.b.len() && self.b[self.i] == b'/' {
                if self.dialect == Dialect::Json {
                    return Err(err("config.parse_failed", self.i));
                }
                match self.b[self.i + 1] {
                    b'/' => {
                        while self.i < self.b.len() && self.b[self.i] != b'\n' {
                            self.i += 1;
                        }
                    }
                    b'*' => {
                        let start = self.i;
                        self.i += 2;
                        loop {
                            if self.i + 1 >= self.b.len() {
                                return Err(err("config.parse_failed", start));
                            }
                            if self.b[self.i] == b'*' && self.b[self.i + 1] == b'/' {
                                self.i += 2;
                                break;
                            }
                            self.i += 1;
                        }
                    }
                    _ => return Err(err("config.parse_failed", self.i)),
                }
                continue;
            }
            return Ok(());
        }
    }

    fn value(&mut self) -> NexusResult<Node> {
        self.depth += 1;
        if self.depth > 64 {
            return Err(err("config.parse_failed", self.i));
        }
        self.ws()?;
        let start = self.i;
        let node = match self.b.get(self.i) {
            Some(b'{') => self.object()?,
            Some(b'[') => self.array()?,
            Some(b'"') => {
                let s = self.string()?;
                Node::Scalar { start, end: self.i, value: Value::String(s) }
            }
            Some(_) => self.literal()?,
            None => return Err(err("config.parse_failed", self.i)),
        };
        self.depth -= 1;
        Ok(node)
    }

    fn object(&mut self) -> NexusResult<Node> {
        let start = self.i;
        self.i += 1;
        let mut members = Vec::new();
        loop {
            self.ws()?;
            match self.b.get(self.i) {
                Some(b'}') => {
                    self.i += 1;
                    return Ok(Node::Object { start, end: self.i, members });
                }
                Some(b'"') => {
                    let key_start = self.i;
                    let key = self.string()?;
                    self.ws()?;
                    if self.b.get(self.i) != Some(&b':') {
                        return Err(err("config.parse_failed", self.i));
                    }
                    self.i += 1;
                    let value = self.value()?;
                    members.push(Member { key, start: key_start, value });
                    self.ws()?;
                    match self.b.get(self.i) {
                        Some(b',') => {
                            self.i += 1;
                            self.ws()?;
                            if self.b.get(self.i) == Some(&b'}') && self.dialect == Dialect::Json {
                                return Err(err("config.parse_failed", self.i));
                            }
                        }
                        Some(b'}') => {}
                        _ => return Err(err("config.parse_failed", self.i)),
                    }
                }
                _ => return Err(err("config.parse_failed", self.i)),
            }
        }
    }

    fn array(&mut self) -> NexusResult<Node> {
        let start = self.i;
        self.i += 1;
        let mut items = Vec::new();
        loop {
            self.ws()?;
            if self.b.get(self.i) == Some(&b']') {
                self.i += 1;
                return Ok(Node::Array { start, end: self.i, items });
            }
            items.push(self.value()?);
            self.ws()?;
            match self.b.get(self.i) {
                Some(b',') => {
                    self.i += 1;
                    self.ws()?;
                    if self.b.get(self.i) == Some(&b']') && self.dialect == Dialect::Json {
                        return Err(err("config.parse_failed", self.i));
                    }
                }
                Some(b']') => {}
                _ => return Err(err("config.parse_failed", self.i)),
            }
        }
    }

    fn string(&mut self) -> NexusResult<String> {
        let start = self.i;
        self.i += 1;
        while self.i < self.b.len() {
            match self.b[self.i] {
                b'\\' => self.i += 2,
                b'"' => {
                    self.i += 1;
                    let raw = std::str::from_utf8(&self.b[start..self.i]).map_err(|_| err("config.parse_failed", start))?;
                    return serde_json::from_str::<String>(raw).map_err(|_| err("config.parse_failed", start));
                }
                c if c < 0x20 => return Err(err("config.parse_failed", self.i)),
                _ => self.i += 1,
            }
        }
        Err(err("config.parse_failed", start))
    }

    fn literal(&mut self) -> NexusResult<Node> {
        let start = self.i;
        while self.i < self.b.len() && !matches!(self.b[self.i], b',' | b'}' | b']' | b' ' | b'\t' | b'\n' | b'\r' | b'/') {
            self.i += 1;
        }
        let raw = std::str::from_utf8(&self.b[start..self.i]).map_err(|_| err("config.parse_failed", start))?;
        let value: Value = serde_json::from_str(raw).map_err(|_| err("config.parse_failed", start))?;
        if value.is_object() || value.is_array() || value.is_string() {
            return Err(err("config.parse_failed", start));
        }
        Ok(Node::Scalar { start, end: self.i, value })
    }
}

impl Document {
    pub fn parse(text: &str, dialect: Dialect) -> NexusResult<Document> {
        let body = text.strip_prefix('\u{feff}').unwrap_or(text);
        let offset = text.len() - body.len();
        let mut parser = Parser { b: text.as_bytes(), i: offset, dialect, depth: 0 };
        let root = parser.value()?;
        parser.ws()?;
        if parser.i != text.len() {
            return Err(err("config.parse_failed", parser.i));
        }
        if !matches!(root, Node::Object { .. }) {
            return Err(NexusError::new("config.unexpected_shape"));
        }
        Ok(Document { text: text.to_string(), root })
    }

    pub fn value(&self) -> Value {
        self.root.to_value()
    }

    fn indent_unit(&self) -> String {
        for line in self.text.lines() {
            let ws: String = line.chars().take_while(|c| *c == ' ' || *c == '\t').collect();
            if !ws.is_empty() && line.trim_start().starts_with('"') {
                return ws;
            }
        }
        "  ".to_string()
    }

    fn line_indent(&self, at: usize) -> String {
        let line_start = self.text[..at].rfind('\n').map(|i| i + 1).unwrap_or(0);
        self.text[line_start..at].chars().take_while(|c| *c == ' ' || *c == '\t').collect()
    }

    fn render(&self, value: &Value, indent: &str) -> String {
        let pretty = serde_json::to_string_pretty(value).expect("serializable value");
        let unit = self.indent_unit();
        // serde_json pretty uses two spaces; re-indent with the file's unit and base indent.
        pretty
            .lines()
            .enumerate()
            .map(|(n, line)| {
                let level = (line.len() - line.trim_start_matches(' ').len()) / 2;
                let body = line.trim_start_matches(' ');
                if n == 0 {
                    body.to_string()
                } else {
                    format!("{indent}{}{body}", unit.repeat(level))
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Sets `path` to `value`, creating missing intermediate objects. Returns the new text.
    pub fn set(&self, path: &[&str], value: &Value) -> NexusResult<String> {
        if path.is_empty() {
            return Err(NexusError::new("config.unexpected_shape"));
        }
        let mut node = &self.root;
        for (depth, key) in path.iter().enumerate() {
            match node.member(key) {
                Some(member) if depth + 1 == path.len() => {
                    let (s, e) = member.value.span();
                    let indent = self.line_indent(member.start);
                    let rendered = self.render(value, &indent);
                    return Ok(format!("{}{}{}", &self.text[..s], rendered, &self.text[e..]));
                }
                Some(member) => {
                    if !matches!(member.value, Node::Object { .. }) {
                        return Err(NexusError::new("config.unexpected_shape").with("key", *key));
                    }
                    node = &member.value;
                }
                None => {
                    // Build the remaining nested value and insert one member here.
                    let mut nested = value.clone();
                    for k in path[depth + 1..].iter().rev() {
                        let mut map = serde_json::Map::new();
                        map.insert((*k).to_string(), nested);
                        nested = Value::Object(map);
                    }
                    return self.insert_member(node, key, &nested);
                }
            }
        }
        unreachable!()
    }

    fn insert_member(&self, object: &Node, key: &str, value: &Value) -> NexusResult<String> {
        let Node::Object { start, end, members } = object else {
            return Err(NexusError::new("config.unexpected_shape"));
        };
        let key_json = serde_json::to_string(key).expect("string key");
        match members.last() {
            Some(last) => {
                let indent = self.line_indent(members[0].start);
                let indent = if indent.is_empty() { self.indent_unit() } else { indent };
                let at = last.value.span().1;
                let rendered = self.render(value, &indent);
                let same_line = !self.text[*start..members[0].start].contains('\n');
                let sep = if same_line { " ".to_string() } else { format!("\n{indent}") };
                Ok(format!("{},{sep}{key_json}: {rendered}{}", &self.text[..at], &self.text[at..]))
            }
            None => {
                let base = self.line_indent(*start);
                let indent = format!("{base}{}", self.indent_unit());
                let rendered = self.render(value, &indent);
                let inner_end = end - 1;
                Ok(format!("{}{{\n{indent}{key_json}: {rendered}\n{base}}}{}", &self.text[..*start], &self.text[inner_end + 1..]))
            }
        }
    }

    /// Removes the member at `path`. Missing members are an error so callers notice drift.
    pub fn remove(&self, path: &[&str]) -> NexusResult<String> {
        let (last, parents) = path.split_last().ok_or_else(|| NexusError::new("config.unexpected_shape"))?;
        let parent = self.root.get(parents).ok_or_else(|| NexusError::new("config.entry_missing"))?;
        let Node::Object { members, .. } = parent else {
            return Err(NexusError::new("config.unexpected_shape"));
        };
        let index = members.iter().rposition(|m| m.key == *last).ok_or_else(|| NexusError::new("config.entry_missing"))?;
        let member = &members[index];
        let member_end = member.value.span().1;
        let (from, to) = if index + 1 < members.len() {
            // Remove "key": value, plus the separating comma and whitespace up to the next member.
            let line_start = self.text[..member.start].rfind('\n').map(|i| i + 1).unwrap_or(0);
            let from = if self.text[line_start..member.start].trim().is_empty() { line_start } else { member.start };
            let next = members[index + 1].start;
            let next_line_start = self.text[..next].rfind('\n').map(|i| i + 1).unwrap_or(0);
            let to = if next_line_start > member_end && self.text[next_line_start..next].trim().is_empty() && from == line_start { next_line_start } else { next };
            (from, to)
        } else if index > 0 {
            // Last member: remove from the end of the previous member's value.
            (members[index - 1].value.span().1, member_end)
        } else {
            (member.start, member_end)
        };
        let mut out = format!("{}{}", &self.text[..from], &self.text[to..]);
        // A lone member followed by a JSONC trailing comma: drop the dangling comma.
        if index == 0 && members.len() == 1 {
            let rest = out[from..].trim_start();
            if rest.starts_with(',') {
                let comma = from + (out[from..].len() - rest.len());
                out.replace_range(comma..comma + 1, "");
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const JSONC: &str = r#"{
  // Theme chosen by hand
  "$schema": "https://opencode.ai/config.json",
  "theme": "system", /* inline */
  "mcp": {
    "docs": { "type": "remote", "url": "https://example.com/mcp", "enabled": true },
  },
}
"#;

    fn round(text: &str, dialect: Dialect) -> Value {
        Document::parse(text, dialect).unwrap().value()
    }

    #[test]
    fn parses_jsonc_and_rejects_comments_in_strict_json() {
        let v = round(JSONC, Dialect::Jsonc);
        assert_eq!(v["mcp"]["docs"]["enabled"], json!(true));
        assert_eq!(Document::parse(JSONC, Dialect::Json).unwrap_err().code, "config.parse_failed");
        assert_eq!(Document::parse("{\"a\":1,}", Dialect::Json).unwrap_err().code, "config.parse_failed");
        assert!(Document::parse("{\"a\":1,}", Dialect::Jsonc).is_ok());
        assert_eq!(Document::parse("[1]", Dialect::Json).unwrap_err().code, "config.unexpected_shape");
        assert!(Document::parse("{\"a\": tru}", Dialect::Json).is_err());
        assert!(Document::parse("{\"a\": 1} trailing", Dialect::Json).is_err());
        assert!(Document::parse("{\"a\": \"unterminated}", Dialect::Json).is_err());
    }

    #[test]
    fn replacing_a_value_preserves_every_other_byte() {
        let doc = Document::parse(JSONC, Dialect::Jsonc).unwrap();
        let out = doc.set(&["mcp", "docs", "enabled"], &json!(false)).unwrap();
        assert_eq!(out, JSONC.replace("\"enabled\": true", "\"enabled\": false"));
        assert!(out.contains("// Theme chosen by hand") && out.contains("/* inline */"));
    }

    #[test]
    fn inserting_top_level_and_nested_members() {
        let doc = Document::parse(JSONC, Dialect::Jsonc).unwrap();
        let out = doc.set(&["model"], &json!("anthropic/claude-x")).unwrap();
        let v = round(&out, Dialect::Jsonc);
        assert_eq!(v["model"], json!("anthropic/claude-x"));
        assert_eq!(v["theme"], json!("system"));
        assert!(out.contains("// Theme chosen by hand"));

        let strict = "{\n    \"theme\": \"dark\"\n}\n";
        let doc = Document::parse(strict, Dialect::Json).unwrap();
        let out = doc.set(&["model", "name"], &json!("gemini-x")).unwrap();
        assert_eq!(out, "{\n    \"theme\": \"dark\",\n    \"model\": {\n        \"name\": \"gemini-x\"\n    }\n}\n");
        Document::parse(&out, Dialect::Json).unwrap();
    }

    #[test]
    fn inserting_into_empty_object() {
        let doc = Document::parse("{}", Dialect::Json).unwrap();
        let out = doc.set(&["model"], &json!("opus")).unwrap();
        assert_eq!(out, "{\n  \"model\": \"opus\"\n}");
        let doc = Document::parse("{\"a\": {}}", Dialect::Json).unwrap();
        let out = doc.set(&["a", "b"], &json!(1)).unwrap();
        assert_eq!(round(&out, Dialect::Json), json!({"a": {"b": 1}}));
    }

    #[test]
    fn removing_members_keeps_valid_syntax() {
        let strict = "{\n  \"a\": 1,\n  \"b\": 2,\n  \"c\": 3\n}";
        let doc = Document::parse(strict, Dialect::Json).unwrap();
        assert_eq!(doc.remove(&["b"]).unwrap(), "{\n  \"a\": 1,\n  \"c\": 3\n}");
        assert_eq!(doc.remove(&["c"]).unwrap(), "{\n  \"a\": 1,\n  \"b\": 2\n}");
        assert_eq!(doc.remove(&["a"]).unwrap(), "{\n  \"b\": 2,\n  \"c\": 3\n}");
        let single = Document::parse("{ \"only\": true }", Dialect::Json).unwrap();
        assert_eq!(round(&single.remove(&["only"]).unwrap(), Dialect::Json), json!({}));
        let doc = Document::parse(JSONC, Dialect::Jsonc).unwrap();
        let out = doc.remove(&["mcp", "docs"]).unwrap();
        assert_eq!(round(&out, Dialect::Jsonc)["mcp"], json!({}));
        assert!(out.contains("// Theme chosen by hand"));
        assert_eq!(doc.remove(&["missing"]).unwrap_err().code, "config.entry_missing");
    }

    #[test]
    fn non_object_intermediate_is_refused() {
        let doc = Document::parse("{\"model\": \"x\"}", Dialect::Json).unwrap();
        assert_eq!(doc.set(&["model", "name"], &json!("y")).unwrap_err().code, "config.unexpected_shape");
    }

    #[test]
    fn strings_with_escapes_and_unicode_survive() {
        let text = "{\"path\": \"C:\\\\Users\\\\张\", \"q\": \"a\\\"b\"}";
        let doc = Document::parse(text, Dialect::Json).unwrap();
        assert_eq!(doc.value()["path"], json!("C:\\Users\\张"));
        let out = doc.set(&["q"], &json!("new")).unwrap();
        assert!(out.starts_with("{\"path\": \"C:\\\\Users\\\\张\""));
    }

    #[test]
    fn deep_nesting_is_bounded() {
        let text = format!("{{\"a\":{}1{}}}", "[".repeat(100), "]".repeat(100));
        assert!(Document::parse(&text, Dialect::Json).is_err());
    }
}
