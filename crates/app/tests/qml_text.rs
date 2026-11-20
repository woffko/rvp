//! Text from outside RVP shows as written in its QML (`shell/qml`, the
//! self-test files left out). Qt Quick's `Text` and `Label`, and the
//! `ToolTip` of every Controls style, take a text that looks like HTML for
//! styled text unless told otherwise: markup in a file's title, a tag, a
//! skin's tooltip or a path would be styled, and its pictures loaded. So:
//!
//! * a `Text` or `Label` whose text is not a fixed string of RVP's own (a
//!   literal, a bare `qsTr("...")`, or a choice `c ? a : b` between such)
//!   sets `textFormat`; a text built from anything else can carry markup,
//!   `qsTr("...").arg(...)` too;
//! * a `ToolTip` with such a text brings its own content item, and an
//!   attached `ToolTip.text` is a fixed string;
//! * a control whose text the style draws with Qt's own label
//!   (`ItemDelegate`, `RadioButton`, `MenuItem`, `Button`...) brings its
//!   own content item for such a text, as Qt's label also takes "&" for a
//!   mnemonic; one that keeps the style's look says in a comment beside
//!   its text that the text is RVP's own (a page's title, a key's name);
//! * a format other than plain text says why in a comment on its line or
//!   the line before.

use std::path::{Path, PathBuf};

/// Items that lay their text out as Qt Quick's `Text` does.
const TEXTS: &[&str] = &["Text", "Label"];

/// Controls whose text the style draws with Qt's own label.
const CONTROLS: &[&str] = &[
    "AbstractButton",
    "Button",
    "CheckBox",
    "CheckDelegate",
    "DelayButton",
    "ItemDelegate",
    "MenuBarItem",
    "MenuItem",
    "RadioButton",
    "RadioDelegate",
    "RoundButton",
    "SwipeDelegate",
    "Switch",
    "SwitchDelegate",
    "TabButton",
    "ToolButton",
];

/// A QML file with its comments blanked (`plain`), and also the insides of
/// its strings and regular expressions (`code`), byte for byte: offsets
/// and lines stay the file's.
struct Source {
    original: String,
    plain: Vec<u8>,
    code: Vec<u8>,
}

impl Source {
    fn new(original: &str) -> Self {
        let bytes = original.as_bytes();
        let mut source = Self {
            original: original.to_owned(),
            plain: bytes.to_vec(),
            code: bytes.to_vec(),
        };
        let mut at = 0;
        while at < bytes.len() {
            let byte = bytes[at];
            let next = bytes.get(at + 1).copied().unwrap_or(0);
            if byte == b'/' && next == b'/' {
                let end = original[at..]
                    .find('\n')
                    .map_or(bytes.len(), |end| at + end);
                source.blank(at, end, true);
                at = end;
            } else if byte == b'/' && next == b'*' {
                let end = original[at + 2..]
                    .find("*/")
                    .map_or(bytes.len(), |end| at + 2 + end + 2);
                source.blank(at, end, true);
                at = end;
            } else if matches!(byte, b'"' | b'\'' | b'`')
                || (byte == b'/' && pattern_starts(&source.code, at))
            {
                let mut end = at + 1;
                let mut class = false;
                while end < bytes.len() {
                    let inside = bytes[end];
                    if inside == b'\\' {
                        end += 2;
                        continue;
                    }
                    if inside == b'\n' && byte != b'`' {
                        break;
                    }
                    if byte == b'/' {
                        match inside {
                            b'[' => class = true,
                            b']' => class = false,
                            b'/' if !class => break,
                            _ => {}
                        }
                    } else if inside == byte {
                        break;
                    }
                    end += 1;
                }
                source.blank(at + 1, end, false);
                at = end + 1;
            } else {
                at += 1;
            }
        }
        source
    }

    /// Blanks `from..to` in `code`, and in `plain` too for a comment.
    fn blank(&mut self, from: usize, to: usize, comment: bool) {
        for at in from..to.min(self.code.len()) {
            if self.code[at] != b'\n' {
                self.code[at] = b' ';
                if comment {
                    self.plain[at] = b' ';
                }
            }
        }
    }

    fn line(&self, at: usize) -> usize {
        // The pieces between line breaks before `at`: its line's number.
        self.original.as_bytes()[..at]
            .split(|&byte| byte == b'\n')
            .count()
    }

    /// Whether the line of `at`, or the one before, has a comment: a part
    /// that `plain` blanked.
    fn explained(&self, at: usize) -> bool {
        let original = self.original.as_bytes();
        let start = |line_end: usize| {
            original[..line_end]
                .iter()
                .rposition(|&byte| byte == b'\n')
                .map_or(0, |newline| newline + 1)
        };
        let line = start(at);
        let from = if line > 0 { start(line - 1) } else { 0 };
        let to = original[at..]
            .iter()
            .position(|&byte| byte == b'\n')
            .map_or(original.len(), |end| at + end);
        original[from..to] != self.plain[from..to]
    }
}

/// An object declaration: `Kind {` ... `}`.
struct Object {
    kind: String,
    open: usize,
    close: usize,
}

impl Object {
    /// The kind without its module qualifier (`T.Label` -> `Label`).
    fn short_kind(&self) -> &str {
        self.kind.rsplit('.').next().unwrap_or(&self.kind)
    }
}

/// Whether the `/` at `at` starts a regular expression rather than a
/// division: an operator, an opening bracket or a keyword comes before it.
fn pattern_starts(code: &[u8], at: usize) -> bool {
    let before = &code[..at];
    let Some(end) = before.iter().rposition(|byte| !byte.is_ascii_whitespace()) else {
        return true;
    };
    if b"(,=:[!&|?{};+-*%<>~^".contains(&before[end]) {
        return true;
    }
    let start = before[..=end]
        .iter()
        .rposition(|byte| !(byte.is_ascii_alphanumeric() || *byte == b'_'))
        .map_or(0, |start| start + 1);
    matches!(
        &before[start..=end],
        b"return" | b"typeof" | b"case" | b"in" | b"of" | b"delete" | b"void"
    )
}

/// The type name right before the brace at `at`, if it names one.
fn kind_before(code: &[u8], at: usize) -> Option<String> {
    let mut end = at;
    while end > 0 && code[end - 1].is_ascii_whitespace() {
        end -= 1;
    }
    let mut start = end;
    while start > 0 && (code[start - 1].is_ascii_alphanumeric() || b"_.".contains(&code[start - 1]))
    {
        start -= 1;
    }
    let word = std::str::from_utf8(&code[start..end]).ok()?;
    let last = word.rsplit('.').next()?;
    last.starts_with(|first: char| first.is_ascii_uppercase())
        .then(|| word.to_owned())
}

fn objects(source: &Source) -> Vec<Object> {
    let mut open = Vec::new();
    let mut found = Vec::new();
    for (at, &byte) in source.code.iter().enumerate() {
        match byte {
            b'{' => open.push((at, kind_before(&source.code, at))),
            b'}' => {
                if let Some((start, Some(kind))) = open.pop() {
                    found.push(Object {
                        kind,
                        open: start,
                        close: at,
                    });
                }
            }
            _ => {}
        }
    }
    found.sort_by_key(|object| object.open);
    found
}

/// A binding directly in an object's body: its name, and its value from
/// the file with comments blanked.
struct Binding {
    name: String,
    value: String,
    at: usize,
}

/// Whether a statement with its brackets closed goes on past the line
/// break at `at`: the line ends with an operator or the next one starts
/// with one.
fn continues(code: &[u8], start: usize, at: usize) -> bool {
    let before = code[start..at]
        .iter()
        .rev()
        .find(|byte| !byte.is_ascii_whitespace());
    let after = code[at..].iter().find(|byte| !byte.is_ascii_whitespace());
    before.is_some_and(|byte| b"+-*/%&|?:,=!<>.(".contains(byte))
        || after.is_some_and(|byte| b"+-*/%&|?:,=<>.".contains(byte))
}

fn bindings(source: &Source, object: &Object) -> Vec<Binding> {
    let code = &source.code;
    let mut found = Vec::new();
    let mut statements = Vec::new();
    let mut start = None;
    let mut depth = 0usize;
    let mut at = object.open + 1;
    while at < object.close {
        let byte = code[at];
        match byte {
            b'{' | b'(' | b'[' => depth += 1,
            b'}' | b')' | b']' => depth = depth.saturating_sub(1),
            _ => {}
        }
        match start {
            None if !byte.is_ascii_whitespace() && byte != b';' => start = Some(at),
            Some(from)
                if depth == 0
                    && (byte == b';' || (byte == b'\n' && !continues(code, from, at))) =>
            {
                statements.push((from, at));
                start = None;
            }
            _ => {}
        }
        at += 1;
    }
    if let Some(from) = start {
        statements.push((from, object.close));
    }
    for (from, to) in statements {
        let text = &code[from..to];
        let name_end = text
            .iter()
            .position(|byte| !(byte.is_ascii_alphanumeric() || b"_.".contains(byte)))
            .unwrap_or(text.len());
        if name_end == 0 || !text[0].is_ascii_alphabetic() {
            continue;
        }
        let rest = &text[name_end..];
        let colon = rest
            .iter()
            .position(|byte| !byte.is_ascii_whitespace())
            .filter(|&colon| rest[colon] == b':');
        let Some(colon) = colon else {
            continue;
        };
        let value_start = from + name_end + colon + 1;
        found.push(Binding {
            name: String::from_utf8_lossy(&text[..name_end]).into_owned(),
            value: String::from_utf8_lossy(&source.plain[value_start..to])
                .trim()
                .to_owned(),
            at: from,
        });
    }
    found
}

/// A string literal and nothing else.
fn literal(value: &str) -> bool {
    let bytes = value.as_bytes();
    let Some(&quote) = bytes.first() else {
        return false;
    };
    if quote != b'"' && quote != b'\'' {
        return false;
    }
    let mut at = 1;
    while at < bytes.len() {
        match bytes[at] {
            b'\\' => at += 2,
            byte if byte == quote => return at == bytes.len() - 1,
            _ => at += 1,
        }
    }
    false
}

/// The bytes of `value` outside its strings, each with its bracket depth.
fn outside_strings(value: &str) -> Vec<(usize, u8, i32)> {
    let bytes = value.as_bytes();
    let mut found = Vec::new();
    let mut quote = None;
    let mut depth = 0;
    let mut at = 0;
    while at < bytes.len() {
        let byte = bytes[at];
        if let Some(open) = quote {
            if byte == b'\\' {
                at += 1;
            } else if byte == open {
                quote = None;
            }
        } else if matches!(byte, b'"' | b'\'' | b'`') {
            quote = Some(byte);
        } else {
            if matches!(byte, b')' | b']' | b'}') {
                depth -= 1;
            }
            found.push((at, byte, depth));
            if matches!(byte, b'(' | b'[' | b'{') {
                depth += 1;
            }
        }
        at += 1;
    }
    found
}

/// `value` without brackets around the whole of it.
fn unwrapped(value: &str) -> &str {
    let value = value.trim();
    let marks = outside_strings(value);
    let wrapped = value.starts_with('(')
        && value.ends_with(')')
        && marks
            .iter()
            .filter(|&&(at, _, depth)| at > 0 && at + 1 < value.len() && depth == 0)
            .count()
            == 0;
    if wrapped {
        unwrapped(&value[1..value.len() - 1])
    } else {
        value
    }
}

/// The two choices of `condition ? a : b` (not `?.` or `??`).
fn choices(value: &str) -> Option<(&str, &str)> {
    let bytes = value.as_bytes();
    let mut question = None;
    let mut nested = 0;
    for (at, byte, depth) in outside_strings(value) {
        if depth != 0 {
            continue;
        }
        let next = bytes.get(at + 1).copied().unwrap_or(0);
        let before = if at > 0 { bytes[at - 1] } else { 0 };
        match byte {
            b'?' if next != b'.' && next != b'?' && before != b'?' => {
                if question.is_none() {
                    question = Some(at);
                } else {
                    nested += 1;
                }
            }
            b':' => {
                if let Some(question) = question {
                    if nested == 0 {
                        return Some((&value[question + 1..at], &value[at + 1..]));
                    }
                    nested -= 1;
                }
            }
            _ => {}
        }
    }
    None
}

/// A fixed string of RVP's own: a literal, a bare `qsTr` of one, or a
/// choice between such strings.
fn fixed(value: &str) -> bool {
    let value = unwrapped(value);
    if let Some((first, second)) = choices(value) {
        return fixed(first) && fixed(second);
    }
    match value
        .strip_prefix("qsTr(")
        .and_then(|rest| rest.strip_suffix(')'))
    {
        Some(inner) => literal(inner.trim()),
        None => literal(value),
    }
}

/// What breaks the rules in one file: `line: what`.
fn check(source: &Source) -> Vec<(usize, String)> {
    let mut found = Vec::new();
    for object in &objects(source) {
        let own = bindings(source, object);
        let get = |name: &str| own.iter().find(|binding| binding.name == name);
        let text = get("text");
        let dynamic = text.is_none_or(|text| !fixed(&text.value));
        let shown = |text: Option<&Binding>| {
            text.map_or_else(|| "(set elsewhere)".to_owned(), |text| text.value.clone())
        };
        let line = source.line(object.open);
        let kind = object.short_kind();
        if TEXTS.contains(&kind) {
            match get("textFormat") {
                Some(format)
                    if format.value != "Text.PlainText" && !source.explained(format.at) =>
                {
                    found.push((
                        source.line(format.at),
                        format!("{} with {} says no reason", object.kind, format.value),
                    ));
                }
                None if dynamic => found.push((
                    line,
                    format!("{} without textFormat: {}", object.kind, shown(text)),
                )),
                _ => {}
            }
        }
        let plain = get("contentItem").is_some();
        let tip = kind == "ToolTip" && dynamic;
        // A control's text of RVP's own that is not fixed (a key's name)
        // says so beside it.
        let control = CONTROLS.contains(&kind)
            && text.is_some_and(|text| !fixed(&text.value) && !source.explained(text.at));
        if (tip || control) && !plain {
            found.push((
                line,
                format!("{} without contentItem: {}", object.kind, shown(text)),
            ));
        }
        if let Some(tip) = get("ToolTip.text")
            && !fixed(&tip.value)
        {
            found.push((source.line(tip.at), format!("ToolTip.text: {}", tip.value)));
        }
    }
    found
}

fn qml_files() -> Vec<PathBuf> {
    let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../shell/qml");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&folder)
        .expect("shell/qml")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "qml"))
        .filter(|path| {
            path.file_name()
                .is_some_and(|name| !name.to_string_lossy().starts_with("SelfTest"))
        })
        .collect();
    files.sort();
    files
}

#[test]
fn text_from_outside_shows_as_written() {
    let files = qml_files();
    assert!(files.len() > 20, "the QML files: {files:?}");
    let mut found = Vec::new();
    for path in files {
        let text = std::fs::read_to_string(&path).expect("read a QML file");
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        for (line, what) in check(&Source::new(&text)) {
            found.push(format!("{name}:{line}: {what}"));
        }
    }
    assert!(
        found.is_empty(),
        "{} texts could take markup:\n{}",
        found.len(),
        found.join("\n")
    );
}

/// The check itself, on a file made for it.
#[test]
fn the_check_tells_fixed_texts_from_others() {
    let source = Source::new(
        r#"Item {
    // Text { text: player.title }
    property string sample: "Label { text: player.title }"
    function test(path) { return /\/\/[{]/.test(path) }
    Text { text: "fixed" }
    Label { text: qsTr("fixed") }
    Text {
        text: player.title
        textFormat: Text.PlainText
    }
    Text {
        text: qsTr("a")
              + player.title
        textFormat: Text.PlainText
    }
    Text {
        // RVP's own markup.
        textFormat: Text.StyledText
        text: "<b>RVP</b>"
    }
    ToolTip {
        id: tip
        contentItem: Text { text: tip.text; textFormat: Text.PlainText }
    }
    ItemDelegate {
        text: modelData.title
        contentItem: Label { text: parent.text; textFormat: Text.PlainText }
    }
    Button { text: qsTr("OK"); ToolTip.text: qsTr("Fixed") }
    Label { text: (count > 0 ? qsTr("Some") : count < 0 ? qsTr("Less") : "None") }
    RadioButton {
        id: device
        text: modelData.name
        contentItem: Label { text: device.text; textFormat: Text.PlainText }
    }
    Button {
        // RVP's own: a key's name.
        text: modelData + "  ×"
    }
    Text { text: player.title }
    Label { text: qsTr("%1 files").arg(count) }
    T.Label {
        id: setElsewhere
    }
    Text { text: other; textFormat: Text.RichText }
    ToolTip { id: bare }
    ItemDelegate { text: modelData.title }
    Button { text: qsTr("OK"); ToolTip.text: name }
    Label { text: count > 0 ? qsTr("Some") + count : qsTr("None") }
    CheckBox { text: player.title ?? qsTr("None") }
}
"#,
    );
    let found: Vec<usize> = check(&source).into_iter().map(|(line, _)| line).collect();
    assert_eq!(found, [40, 41, 42, 45, 46, 47, 48, 49, 50]);
}
