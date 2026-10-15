//! A strict reader for the XML that RVP's own sources are written in.
//!
//! The skin crate's markup reader is tolerant on purpose (it reproduces
//! PotPlayer) and takes `stroke-width` for `width`. RVP's SVG files are
//! well-formed XML, so this reader rejects anything else: unclosed or
//! mismatched tags, unquoted or repeated attributes, unknown entities.
//! Comments, the XML declaration and `DOCTYPE` are skipped; text between
//! tags is ignored.

/// One element with its attributes in document order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub name: String,
    pub attrs: Vec<(String, String)>,
    pub children: Vec<Node>,
    /// One-based line of the start tag.
    pub line: usize,
}

impl Node {
    /// The value of an attribute (names are case-sensitive, as in XML).
    #[must_use]
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }

    /// Appends the element as XML text (attributes in order, escaped).
    pub fn write(&self, out: &mut String) {
        out.push('<');
        out.push_str(&self.name);
        for (key, value) in &self.attrs {
            out.push(' ');
            out.push_str(key);
            out.push_str("=\"");
            for c in value.chars() {
                match c {
                    '&' => out.push_str("&amp;"),
                    '<' => out.push_str("&lt;"),
                    '>' => out.push_str("&gt;"),
                    '"' => out.push_str("&quot;"),
                    other => out.push(other),
                }
            }
            out.push('"');
        }
        if self.children.is_empty() {
            out.push_str("/>");
            return;
        }
        out.push('>');
        for child in &self.children {
            child.write(out);
        }
        out.push_str("</");
        out.push_str(&self.name);
        out.push('>');
    }

    /// Calls `visit` for this element and every descendant in document
    /// order.
    pub fn walk<'a>(&'a self, visit: &mut impl FnMut(&'a Node)) {
        visit(self);
        for child in &self.children {
            child.walk(visit);
        }
    }
}

/// Parses a document and returns its root element.
///
/// # Errors
/// Reports the first problem with its line.
pub fn parse(text: &str) -> Result<Node, String> {
    let mut reader = Reader {
        text,
        bytes: text.as_bytes(),
        at: 0,
    };
    let mut stack: Vec<Node> = Vec::new();
    let mut root: Option<Node> = None;
    while let Some(offset) = text[reader.at..].find('<') {
        let start = reader.at + offset;
        reader.at = start;
        let rest = &text[start..];
        if rest.starts_with("<!--") {
            let end = rest
                .find("-->")
                .ok_or_else(|| reader.error("unclosed comment"))?;
            reader.at = start + end + 3;
        } else if rest.starts_with("<?") || rest.starts_with("<!") {
            let end = rest
                .find('>')
                .ok_or_else(|| reader.error("unclosed declaration"))?;
            reader.at = start + end + 1;
        } else if let Some(after) = rest.strip_prefix("</") {
            let name_len = name_length(after.as_bytes());
            let name = &after[..name_len];
            reader.at = start + 2 + name_len;
            reader.skip_blanks();
            reader.expect(b'>')?;
            let node = stack
                .pop()
                .ok_or_else(|| reader.error(&format!("</{name}> closes nothing")))?;
            if node.name != name {
                return Err(reader.error(&format!(
                    "</{name}> closes <{}> from line {}",
                    node.name, node.line
                )));
            }
            attach(&mut stack, &mut root, node, &reader)?;
        } else {
            let (node, self_closing) = reader.start_tag()?;
            if self_closing {
                attach(&mut stack, &mut root, node, &reader)?;
            } else {
                stack.push(node);
            }
        }
    }
    if let Some(open) = stack.last() {
        return Err(format!(
            "line {}: <{}> is never closed",
            open.line, open.name
        ));
    }
    root.ok_or_else(|| "the document has no element".to_owned())
}

fn attach(
    stack: &mut [Node],
    root: &mut Option<Node>,
    node: Node,
    reader: &Reader,
) -> Result<(), String> {
    if let Some(parent) = stack.last_mut() {
        parent.children.push(node);
    } else if root.is_some() {
        return Err(reader.error("a second root element"));
    } else {
        *root = Some(node);
    }
    Ok(())
}

fn name_length(bytes: &[u8]) -> usize {
    bytes
        .iter()
        .position(|&b| !(b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b':' | b'.')))
        .unwrap_or(bytes.len())
}

struct Reader<'a> {
    text: &'a str,
    bytes: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn line(&self) -> usize {
        // Sources are small; counting on each error or tag is cheap.
        #[allow(clippy::naive_bytecount)]
        let newlines = self.bytes[..self.at]
            .iter()
            .filter(|&&b| b == b'\n')
            .count();
        newlines + 1
    }

    fn error(&self, message: &str) -> String {
        format!("line {}: {message}", self.line())
    }

    fn skip_blanks(&mut self) {
        while self.at < self.bytes.len() && self.bytes[self.at].is_ascii_whitespace() {
            self.at += 1;
        }
    }

    fn expect(&mut self, byte: u8) -> Result<(), String> {
        if self.bytes.get(self.at) == Some(&byte) {
            self.at += 1;
            Ok(())
        } else {
            Err(self.error(&format!("expected '{}'", char::from(byte))))
        }
    }

    /// Reads `<name attr="value" ...>` or `.../>` at the cursor.
    fn start_tag(&mut self) -> Result<(Node, bool), String> {
        let line = self.line();
        self.at += 1;
        let name_len = name_length(&self.bytes[self.at..]);
        if name_len == 0 {
            return Err(self.error("a tag without a name"));
        }
        let name = self.text[self.at..self.at + name_len].to_owned();
        self.at += name_len;
        let mut attrs: Vec<(String, String)> = Vec::new();
        loop {
            let before = self.at;
            self.skip_blanks();
            match self.bytes.get(self.at) {
                Some(b'>') => {
                    self.at += 1;
                    return Ok((node(name, attrs, line), false));
                }
                Some(b'/') => {
                    self.at += 1;
                    self.expect(b'>')?;
                    return Ok((node(name, attrs, line), true));
                }
                None => return Err(self.error(&format!("<{name}> is not closed"))),
                Some(_) if before == self.at => {
                    return Err(self.error("expected a blank before the attribute"));
                }
                Some(_) => {}
            }
            let key_len = name_length(&self.bytes[self.at..]);
            if key_len == 0 {
                return Err(self.error(&format!("unexpected text in <{name}>")));
            }
            let key = self.text[self.at..self.at + key_len].to_owned();
            self.at += key_len;
            self.skip_blanks();
            self.expect(b'=')?;
            self.skip_blanks();
            let Some(&quote @ (b'"' | b'\'')) = self.bytes.get(self.at) else {
                return Err(self.error(&format!("the value of {key} is not quoted")));
            };
            self.at += 1;
            let length = self.bytes[self.at..]
                .iter()
                .position(|&b| b == quote)
                .ok_or_else(|| self.error(&format!("the value of {key} is not closed")))?;
            let raw = &self.text[self.at..self.at + length];
            self.at += length + 1;
            if attrs.iter().any(|(existing, _)| *existing == key) {
                return Err(self.error(&format!("{key} is repeated")));
            }
            let value = unescape(raw).map_err(|message| self.error(&message))?;
            attrs.push((key, value));
        }
    }
}

fn node(name: String, attrs: Vec<(String, String)>, line: usize) -> Node {
    Node {
        name,
        attrs,
        children: Vec::new(),
        line,
    }
}

/// Replaces the five predefined entities.
fn unescape(raw: &str) -> Result<String, String> {
    if raw.contains('<') {
        return Err("'<' in an attribute value".to_owned());
    }
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        let after = &rest[amp..];
        let end = after
            .find(';')
            .ok_or_else(|| "an unterminated entity".to_owned())?;
        out.push(match &after[..=end] {
            "&amp;" => '&',
            "&lt;" => '<',
            "&gt;" => '>',
            "&quot;" => '"',
            "&apos;" => '\'',
            other => return Err(format!("unknown entity {other}")),
        });
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn reads_nested_elements_and_attributes() {
        let root = parse(
            "<?xml version=\"1.0\"?>\n<!-- art -->\n<svg width=\"16\">\n  <g stroke-width='2'>\
             <path d=\"M0 0 L1 1\"/></g>\n  <title>RVP &amp; co</title>\n</svg>",
        )
        .expect("parse");
        assert_eq!(root.name, "svg");
        assert_eq!(root.attr("width"), Some("16"));
        assert_eq!(root.children[0].attr("stroke-width"), Some("2"));
        assert_eq!(root.children[0].children[0].attr("d"), Some("M0 0 L1 1"));
        assert_eq!(root.children[0].children[0].line, 4);
        assert_eq!(root.children[1].name, "title");
    }

    #[test]
    fn writes_what_it_reads() {
        let text = "<g a=\"1 &amp; 2\"><path d=\"M0 0\"/></g>";
        let mut out = String::new();
        parse(text).expect("parse").write(&mut out);
        assert_eq!(out, text);
    }

    #[test]
    fn rejects_malformed_documents() {
        for (text, needle) in [
            ("<svg><g></svg>", "closes <g>"),
            ("<svg width=16/>", "not quoted"),
            ("<svg a='1' a='2'/>", "repeated"),
            ("<svg a='&nbsp;'/>", "unknown entity"),
            ("<svg>", "never closed"),
            ("<svg/><svg/>", "second root"),
            ("<svg a='1'b='2'/>", "blank before"),
        ] {
            let error = parse(text).expect_err(text);
            assert!(error.contains(needle), "{text}: {error}");
        }
    }
}
