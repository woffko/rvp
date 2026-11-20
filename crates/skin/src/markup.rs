//! A tolerant reader for PotPlayer's XML-like skin markup.
//!
//! PotPlayer accepts files that no XML parser would. The rules below
//! reproduce what it visibly does with the BlackSpace skin:
//!
//! * A tag starts at `<` followed directly by a letter, a digit
//!   (`<1stCaptureSkinArea>`) or `/`. Text outside tags is ignored, which is
//!   how skin authors disable a line: they delete its leading `<`.
//! * A tag ends at the first `>`, except the `>` of the `< >` separator used
//!   in two-part tooltips (`Tooltip='Pause< >DVD Control'`). Quotes do not
//!   protect `>`, so a tag with an unbalanced quote still ends on its line.
//!   A tag that runs into the next tag start ends there.
//! * Attributes are read left to right as `Name=value`, where the value is
//!   quoted with `'` or `"`, or unquoted up to whitespace. Anything that does
//!   not fit is skipped. With a broken quote such as
//!   `ThumbHeight='15 ImageLeft='132'`, `ThumbHeight` gets `15 ImageLeft=` and
//!   `ImageLeft` is lost; PotPlayer's volume slider in BlackSpace shows the
//!   same loss (it draws the sprite at x = 0).
//! * Attribute names are case-insensitive; the first occurrence wins.
//! * Closing tags close the nearest open element with the same name; stray
//!   closing tags are ignored and open elements are closed at the end.
//!
//! VLC's themes (M9) are XML that VLC reads with libxml2: [`parse_xml`]
//! keeps the rules above except that quotes protect `>` and `<`
//! (`tooltiptext="Show Info >> ..."`).
//!
//! RVP's own rule, which PotPlayer does not have: the tree is bounded
//! ([`crate::limits`]). Elements nest at most `MARKUP_DEPTH` deep, a
//! document has at most `MARKUP_ELEMENTS` of them and an element at most
//! `ATTRIBUTES` attributes, so that every walk over the tree is short and
//! shallow. A skin's files do not load beyond that ([`parse_within`]);
//! the tolerant readers ([`parse_document`], [`parse_xml`], [`parse_attrs`])
//! leave out the elements that lie deeper, and the elements and
//! attributes that are too many.

use crate::error::SkinError;
use crate::limits;

/// The attributes of one element, in document order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Attrs {
    entries: Vec<(String, String)>,
}

impl Attrs {
    /// Returns the raw value of the first attribute named `name`.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    /// Returns true when the attribute is present, even with an empty value.
    #[must_use]
    pub fn has(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    /// Returns the attribute as an integer with C `atoi` rules: leading
    /// blanks, an optional sign, then digits up to the first non-digit.
    /// `0x` prefixes are read as hexadecimal. Missing attributes give `None`.
    #[must_use]
    pub fn int(&self, name: &str) -> Option<i32> {
        self.get(name).map(atoi)
    }

    /// Like [`Attrs::int`], with a default for missing attributes.
    #[must_use]
    pub fn int_or(&self, name: &str, default: i32) -> i32 {
        self.int(name).unwrap_or(default)
    }

    /// True when the attribute is present and its integer value is not zero.
    #[must_use]
    pub fn flag(&self, name: &str) -> bool {
        self.int(name).is_some_and(|value| value != 0)
    }

    /// Iterates over all attributes in document order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.entries
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
    }

    /// Number of attributes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True when there are no attributes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub(crate) fn push(&mut self, name: &str, value: &str) {
        if !self.has(name) {
            self.entries.push((name.to_owned(), value.to_owned()));
        }
    }
}

/// C `atoi` with an extension for `0x` hexadecimal numbers.
#[must_use]
pub fn atoi(text: &str) -> i32 {
    let text = text.trim_start();
    let (negative, digits) = match text.as_bytes().first() {
        Some(b'-') => (true, &text[1..]),
        Some(b'+') => (false, &text[1..]),
        _ => (false, text),
    };
    let (radix, digits) = match digits.get(..2) {
        Some(prefix) if prefix.eq_ignore_ascii_case("0x") => (16, &digits[2..]),
        _ => (10, digits),
    };
    let mut value: i64 = 0;
    for ch in digits.chars() {
        let Some(digit) = ch.to_digit(radix) else {
            break;
        };
        value = (value * i64::from(radix) + i64::from(digit)).min(i64::from(u32::MAX));
    }
    if negative {
        value = -value;
    }
    // Hexadecimal colours such as 0xFFFFFFFF wrap like a C cast to int.
    i32::try_from(value).unwrap_or_else(|_| {
        let wrapped = value.rem_euclid(1_i64 << 32);
        i32::try_from(wrapped - (1_i64 << 32)).unwrap_or(i32::MAX)
    })
}

/// Parses the attribute part of a tag (everything after the element name).
#[must_use]
pub fn parse_attrs(body: &str) -> Attrs {
    read_attrs(body).0
}

/// [`parse_attrs`], and whether attributes were left out as too many.
fn read_attrs(body: &str) -> (Attrs, bool) {
    let bytes = body.as_bytes();
    let mut attrs = Attrs::default();
    let mut i = 0;
    while i < bytes.len() {
        let starts_name = (bytes[i].is_ascii_alphabetic() || bytes[i] == b'_')
            && (i == 0 || !is_name_byte(bytes[i - 1]));
        if !starts_name {
            i += 1;
            continue;
        }
        let name_start = i;
        while i < bytes.len() && is_name_byte(bytes[i]) {
            i += 1;
        }
        let name = &body[name_start..i];
        let mut j = skip_blanks(bytes, i);
        if j >= bytes.len() || bytes[j] != b'=' {
            continue;
        }
        j = skip_blanks(bytes, j + 1);
        if j < bytes.len() && (bytes[j] == b'\'' || bytes[j] == b'"') {
            let quote = bytes[j];
            let value_start = j + 1;
            let value_end = bytes[value_start..]
                .iter()
                .position(|&b| b == quote)
                .map_or(bytes.len(), |offset| value_start + offset);
            if attrs.len() >= limits::ATTRIBUTES {
                return (attrs, true);
            }
            attrs.push(name, &body[value_start..value_end]);
            i = (value_end + 1).min(bytes.len());
        } else {
            let value_start = j;
            while j < bytes.len() && !bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            if attrs.len() >= limits::ATTRIBUTES {
                return (attrs, true);
            }
            attrs.push(name, &body[value_start..j]);
            i = j;
        }
    }
    (attrs, false)
}

fn is_name_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn skip_blanks(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t') {
        i += 1;
    }
    i
}

/// One element of the skin markup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Element {
    /// Pre-order index in the document; stable for one loaded skin.
    pub id: usize,
    /// Element name as written (`Static`, `Button`, ...).
    pub name: String,
    pub attrs: Attrs,
    pub children: Vec<Element>,
    /// Zero-based line in the preprocessed text where the tag starts.
    pub line: usize,
}

impl Element {
    pub(crate) fn new(name: &str, attrs: Attrs, line: usize) -> Self {
        Self {
            id: 0,
            name: name.to_owned(),
            attrs,
            children: Vec::new(),
            line,
        }
    }

    /// True when the element has the given name, ignoring case.
    #[must_use]
    pub fn is(&self, name: &str) -> bool {
        self.name.eq_ignore_ascii_case(name)
    }

    /// The first direct child with the given name.
    #[must_use]
    pub fn child(&self, name: &str) -> Option<&Element> {
        self.children.iter().find(|child| child.is(name))
    }

    /// The first element with the given name in pre-order, including `self`.
    #[must_use]
    pub fn find(&self, name: &str) -> Option<&Element> {
        if self.is(name) {
            return Some(self);
        }
        self.children.iter().find_map(|child| child.find(name))
    }

    /// Calls `visit` for this element and every descendant in pre-order.
    pub fn walk<'a>(&'a self, visit: &mut impl FnMut(&'a Element)) {
        visit(self);
        for child in &self.children {
            child.walk(visit);
        }
    }

    fn assign_ids(&mut self, next: &mut usize) {
        self.id = *next;
        *next += 1;
        for child in &mut self.children {
            child.assign_ids(next);
        }
    }
}

/// Parses preprocessed skin text into an element tree. The returned root is
/// a synthetic element named `#document`.
#[must_use]
pub fn parse_document(text: &str) -> Element {
    parse(text, false).0
}

/// Parses an XML file (VLC's `theme.xml`) as [`parse_document`] does, with
/// quoted values that may hold `>` and `<`.
#[must_use]
pub fn parse_xml(text: &str) -> Element {
    parse(text, true).0
}

/// Parses a skin's own markup (`xml`: as [`parse_xml`], else as
/// [`parse_document`]).
///
/// # Errors
/// Fails when the markup nests deeper, or has more elements or more
/// attributes on one of them, than [`crate::limits`] allow.
pub fn parse_within(text: &str, xml: bool) -> Result<Element, SkinError> {
    match parse(text, xml) {
        (root, None) => Ok(root),
        (_, Some(error)) => Err(error),
    }
}

/// The tree a text is read into, within the limits.
struct Tree {
    /// The document and, below it, the elements that are still open.
    stack: Vec<Element>,
    /// The first limit the text went over.
    over: Option<SkinError>,
    /// The opening tags read so far.
    elements: usize,
    /// Open elements that lie deeper than the limit and are left out:
    /// their closing tags close them, not the elements above.
    left_out: usize,
}

impl Tree {
    /// Adds the element of an opening tag: `name`, its attributes' text
    /// and whether the tag closes it. False when the document has all the
    /// elements it may have: the rest of the text is left out.
    fn open(&mut self, name: &str, body: &str, self_closing: bool, line: usize) -> bool {
        self.elements += 1;
        if self.elements > limits::MARKUP_ELEMENTS {
            self.over.get_or_insert_with(|| {
                limits::exceeded("more elements in the markup", limits::MARKUP_ELEMENTS)
            });
            return false;
        }
        let (attrs, more) = read_attrs(body);
        if more {
            self.over.get_or_insert_with(|| {
                limits::exceeded(
                    &format!("more attributes on an element (line {})", line + 1),
                    limits::ATTRIBUTES,
                )
            });
        }
        let element = Element::new(name, attrs, line);
        // The stack holds the document, which is no element of the skin,
        // and the open elements: as many as the new one is deep.
        if self.stack.len() > limits::MARKUP_DEPTH {
            self.over.get_or_insert_with(|| {
                limits::exceeded(
                    &format!("markup nested deeper (line {})", line + 1),
                    limits::MARKUP_DEPTH,
                )
            });
            self.left_out += usize::from(!self_closing);
        } else if self_closing {
            append(&mut self.stack, element);
        } else {
            self.stack.push(element);
        }
        true
    }

    /// Closes the nearest open element named `name`.
    fn close(&mut self, name: &str) {
        if self.left_out > 0 {
            self.left_out -= 1;
        } else {
            close(&mut self.stack, name);
        }
    }
}

/// The tree, and the first limit the text went over.
fn parse(text: &str, quotes: bool) -> (Element, Option<SkinError>) {
    let bytes = text.as_bytes();
    let mut tree = Tree {
        stack: vec![Element::new("#document", Attrs::default(), 0)],
        over: None,
        elements: 0,
        left_out: 0,
    };
    let mut line = 0;
    let mut line_counted_to = 0;
    let mut i = 0;
    while let Some(offset) = text[i..].find('<') {
        let start = i + offset;
        // A skin is a few hundred kilobytes; a plain count is fast enough.
        #[allow(clippy::naive_bytecount)]
        let newlines = bytes[line_counted_to..start]
            .iter()
            .filter(|&&b| b == b'\n')
            .count();
        line += newlines;
        line_counted_to = start;
        let next = bytes.get(start + 1).copied().unwrap_or(b' ');
        if text[start..].starts_with("<!--") {
            i = text[start..]
                .find("-->")
                .map_or(bytes.len(), |end| start + end + 3);
        } else if next == b'?' || next == b'!' {
            i = text[start..]
                .find('>')
                .map_or(bytes.len(), |end| start + end + 1);
        } else if next == b'/' {
            let name_start = start + 2;
            let mut name_end = name_start;
            while name_end < bytes.len() && is_name_byte(bytes[name_end]) {
                name_end += 1;
            }
            tree.close(&text[name_start..name_end]);
            i = text[start..]
                .find('>')
                .map_or(bytes.len(), |end| start + end + 1);
        } else if next.is_ascii_alphanumeric() {
            let name_start = start + 1;
            let mut name_end = name_start;
            while name_end < bytes.len() && is_name_byte(bytes[name_end]) {
                name_end += 1;
            }
            let (body_end, resume) = if quotes {
                quoted_tag_end(bytes, name_end)
            } else {
                tag_end(bytes, name_end)
            };
            let mut body = text[name_end..body_end].trim_end();
            let self_closing = body.ends_with('/');
            if self_closing {
                body = &body[..body.len() - 1];
            }
            if !tree.open(&text[name_start..name_end], body, self_closing, line) {
                break;
            }
            i = resume;
        } else {
            i = start + 1;
        }
    }
    let Tree {
        mut stack, over, ..
    } = tree;
    while stack.len() > 1 {
        let element = stack
            .pop()
            .unwrap_or_else(|| Element::new("", Attrs::default(), 0));
        append(&mut stack, element);
    }
    let mut root = stack
        .pop()
        .unwrap_or_else(|| Element::new("#document", Attrs::default(), 0));
    let mut next_id = 0;
    root.assign_ids(&mut next_id);
    (root, over)
}

/// Finds where a tag body ends. Returns the end of the body and the offset
/// where scanning resumes.
fn tag_end(bytes: &[u8], from: usize) -> (usize, usize) {
    let mut j = from;
    while j < bytes.len() {
        match bytes[j] {
            b'>' => return (j, j + 1),
            b'<' => {
                let after = skip_blanks(bytes, j + 1);
                if after < bytes.len() && bytes[after] == b'>' {
                    // The `< >` tooltip separator.
                    j = after + 1;
                    continue;
                }
                let next = bytes.get(j + 1).copied().unwrap_or(b' ');
                if next.is_ascii_alphanumeric() || next == b'/' {
                    // An unterminated tag: the next tag starts here.
                    return (j, j);
                }
                j += 1;
            }
            _ => j += 1,
        }
    }
    (bytes.len(), bytes.len())
}

/// [`tag_end`] for XML: the first `>` outside quotes.
fn quoted_tag_end(bytes: &[u8], from: usize) -> (usize, usize) {
    let mut quote = None;
    for (j, &byte) in bytes.iter().enumerate().skip(from) {
        match (quote, byte) {
            (None, b'\'' | b'"') => quote = Some(byte),
            (Some(open), _) if open == byte => quote = None,
            (None, b'>') => return (j, j + 1),
            _ => {}
        }
    }
    (bytes.len(), bytes.len())
}

fn append(stack: &mut [Element], element: Element) {
    if let Some(parent) = stack.last_mut() {
        parent.children.push(element);
    }
}

fn close(stack: &mut Vec<Element>, name: &str) {
    let Some(position) = stack.iter().rposition(|element| element.is(name)) else {
        return;
    };
    if position == 0 {
        return;
    }
    while stack.len() > position {
        let Some(element) = stack.pop() else {
            break;
        };
        append(stack, element);
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use super::{Element, atoi, parse_attrs, parse_document, parse_within, parse_xml};
    use crate::error::SkinError;
    use crate::limits;

    #[test]
    fn atoi_matches_c() {
        assert_eq!(atoi("15 ImageLeft="), 15);
        assert_eq!(atoi("  -87"), -87);
        assert_eq!(atoi("HAND"), 0);
        assert_eq!(atoi("0x000000"), 0);
        assert_eq!(atoi("0x7300ffff"), 0x7300_ffff);
        assert_eq!(atoi("0xFFFFFFFF"), -1);
    }

    #[test]
    fn reads_quoted_unquoted_and_glued_attributes() {
        let attrs = parse_attrs(" A='1' B=\"two words\" C=plain D='4'E='5' f='lower' A='ignored'");
        assert_eq!(attrs.get("A"), Some("1"));
        assert_eq!(attrs.get("b"), Some("two words"));
        assert_eq!(attrs.get("C"), Some("plain"));
        assert_eq!(attrs.int("E"), Some(5));
        assert_eq!(attrs.get("F"), Some("lower"));
        assert_eq!(attrs.len(), 6);
    }

    #[test]
    fn broken_quote_loses_the_next_attribute_like_potplayer() {
        let attrs = parse_attrs(
            " ThumbWidth='1' ThumbHeight='15 ImageLeft='132' ImageTop='279' BackImageLeft='132'",
        );
        assert_eq!(attrs.int("ThumbHeight"), Some(15));
        assert_eq!(attrs.get("ImageLeft"), None);
        assert_eq!(attrs.int("ImageTop"), Some(279));
        assert_eq!(attrs.int("BackImageLeft"), Some(132));
    }

    #[test]
    fn disabled_lines_and_separators() {
        let doc = parse_document(
            "<Static>\n Button CmdID='OFF' />\n <Button Tooltip='Play< >Open' CmdID='ON' />\n</Static>",
        );
        let static_element = &doc.children[0];
        assert_eq!(static_element.children.len(), 1);
        let button = &static_element.children[0];
        assert_eq!(button.attrs.get("Tooltip"), Some("Play< >Open"));
        assert_eq!(button.attrs.get("CmdID"), Some("ON"));
        assert_eq!(button.line, 2);
    }

    #[test]
    fn unbalanced_quote_still_ends_the_tag_on_its_line() {
        let doc =
            parse_document("<Bar>\n<Slider A='1' B='2 C='3' D='4' />\n<Button E='5' />\n</Bar>");
        let bar = &doc.children[0];
        assert_eq!(bar.children.len(), 2);
        assert!(bar.children[0].is("Slider"));
        assert_eq!(bar.children[0].attrs.int("B"), Some(2));
        assert_eq!(bar.children[0].attrs.int("C"), None);
        assert_eq!(bar.children[0].attrs.int("D"), Some(4));
        assert_eq!(bar.children[1].attrs.int("E"), Some(5));
    }

    #[test]
    fn tolerates_stray_and_missing_closing_tags() {
        let doc = parse_document(
            "<?XML version='1.0' ?><!-- c --><Skin><A><B></A></C><1stArea X=1></Skin>[<D/>]",
        );
        let skin = &doc.children[0];
        assert!(skin.is("Skin"));
        assert!(skin.children[0].is("A"));
        assert!(skin.children[0].children[0].is("B"));
        assert!(skin.children[1].is("1stArea"));
        assert!(doc.children[1].is("D"));
        let mut ids = Vec::new();
        doc.walk(&mut |element| ids.push(element.id));
        assert_eq!(ids, (0..ids.len()).collect::<Vec<_>>());
    }

    #[test]
    fn unterminated_tag_ends_at_next_tag() {
        let doc = parse_document("<A X='1'\n<B Y='2' />");
        assert_eq!(doc.children.len(), 1);
        assert_eq!(doc.children[0].attrs.int("X"), Some(1));
        assert!(doc.children[0].children[0].is("B"));
    }

    #[test]
    fn xml_quotes_protect_brackets() {
        let text = "<W><B T=\"Info >> site\" X='1'/><B T='a<b' X='2'/></W>";
        let doc = parse_xml(text);
        let buttons = &doc.children[0].children;
        assert_eq!(buttons.len(), 2);
        assert_eq!(buttons[0].attrs.get("T"), Some("Info >> site"));
        assert_eq!(buttons[1].attrs.int("X"), Some(2));
        // PotPlayer's markup ends the tag at the quoted `>`.
        assert_eq!(
            parse_document(text).children[0].children[0].attrs.get("T"),
            Some("Info")
        );
    }

    /// The depth of the tree below `element` and its number of elements.
    fn shape(element: &Element) -> (usize, usize) {
        let mut deepest = 0;
        let mut count = 1;
        for child in &element.children {
            let (depth, elements) = shape(child);
            deepest = deepest.max(depth);
            count += elements;
        }
        (deepest + 1, count)
    }

    fn limit(result: Result<Element, SkinError>) -> String {
        match result {
            Err(SkinError::Limit(what)) => what,
            other => panic!("not a limit: {:?}", other.map(|root| shape(&root))),
        }
    }

    #[test]
    fn nesting_is_bounded() {
        // Tags that are never closed nest: a tree 100,000 deep ends the
        // process when it is walked, copied or dropped.
        let text = "<a>".repeat(100_000);
        let (error, depth, elements) = limits::on_small_stack(move || {
            let error = limit(parse_within(&text, false));
            let tree = parse_document(&text);
            let copy = tree.clone();
            let mut walked = 0;
            copy.walk(&mut |_| walked += 1);
            let (depth, elements) = shape(&tree);
            assert_eq!(walked, elements);
            assert!(tree.find("none").is_none());
            (error, depth, elements)
        });
        assert!(error.contains("deeper"), "{error}");
        // The document, and no more levels than the limit below it.
        assert_eq!(depth, limits::MARKUP_DEPTH + 1);
        assert_eq!(elements, limits::MARKUP_DEPTH + 1);
        // The deepest level a skin may have, and one more.
        let nested = |levels: usize| "<a>".repeat(levels) + &"</a>".repeat(levels);
        let deepest = parse_within(&nested(limits::MARKUP_DEPTH), false).unwrap();
        assert_eq!(shape(&deepest).0, limits::MARKUP_DEPTH + 1);
        assert!(parse_within(&nested(limits::MARKUP_DEPTH + 1), true).is_err());
        // Elements that lie deeper are left out, with their closing
        // tags: what follows them keeps its place.
        let text = format!("<w>{}</w><last/>", nested(limits::MARKUP_DEPTH));
        let tree = parse_xml(&text);
        assert_eq!(
            shape(&tree),
            (limits::MARKUP_DEPTH + 1, limits::MARKUP_DEPTH + 2)
        );
        assert!(tree.children[0].is("w") && tree.children[1].is("last"));
    }

    #[test]
    fn elements_are_bounded() {
        let text = "<a/>".repeat(limits::MARKUP_ELEMENTS + 1);
        assert!(limit(parse_within(&text, false)).contains("elements"));
        assert_eq!(shape(&parse_document(&text)).1, limits::MARKUP_ELEMENTS + 1);
        let text = "<a/>".repeat(limits::MARKUP_ELEMENTS);
        assert_eq!(
            shape(&parse_within(&text, false).unwrap()).1,
            limits::MARKUP_ELEMENTS + 1
        );
    }

    #[test]
    fn attributes_are_bounded() {
        let tag = |count: usize| {
            let mut tag = String::from("<e ");
            for n in 0..count {
                let _ = write!(tag, "a{n}='{n}' ");
            }
            tag + "/>"
        };
        let full = parse_within(&tag(limits::ATTRIBUTES), false).unwrap();
        assert_eq!(full.children[0].attrs.len(), limits::ATTRIBUTES);
        assert!(limit(parse_within(&tag(limits::ATTRIBUTES + 1), false)).contains("attributes"));
        // Each attribute is compared with those before it: 200,000 of
        // them on one element would take hours.
        let many = tag(200_000);
        assert!(limit(parse_within(&many, true)).contains("attributes"));
        let kept = parse_document(&many);
        assert_eq!(kept.children[0].attrs.len(), limits::ATTRIBUTES);
        assert_eq!(kept.children[0].attrs.int("a7"), Some(7));
        // A name that comes again is not one more attribute.
        let again = format!("<e {}/>", "a='1' ".repeat(limits::ATTRIBUTES * 4));
        assert_eq!(
            parse_within(&again, false).unwrap().children[0].attrs.len(),
            1
        );
    }
}
