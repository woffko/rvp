//! The skin text preprocessor.
//!
//! Directives sit on their own lines:
//!
//! * `#include<File.txt>` and `#include<File.txt #NAME=value ...>` insert a
//!   file; `#NAME` in the included text is replaced by `value`.
//! * `#if<Name==Value>`, `#if<Name!=Value>`, `#else`, `#endif` keep or drop
//!   lines. Names are looked up in the variables given by the caller
//!   (`SkinName`, for example, and `SkinType=Touch` for a skin's touch
//!   layout, M14) and compared case-insensitively; a name not given
//!   matches nothing.
//!
//! Everything else passes through unchanged.
//!
//! A macro's value may name a macro replaced after it (longer names
//! first, then by name), which an include that passes its parent's macro
//! on relies on (`#A=#B`). Values that name each other, and files that
//! include themselves, would grow the text without end, so the expansion
//! is bounded ([`crate::limits`]): an include nested deeper than
//! `INCLUDE_DEPTH` is left out with a warning, and a skin whose expanded
//! text is over `TEXT_BYTES`, that reads more than that of its files, has
//! more than `INCLUDES` includes or gives one more than `MACROS` macros
//! does not load. A line's length after each replacement is known before
//! the replacement is made.

use std::collections::BTreeMap;

use crate::archive::SkinArchive;
use crate::error::SkinError;
use crate::limits;
use crate::text;

/// Where an expanded line came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceLine {
    /// Index into [`Expanded::files`].
    pub file: usize,
    /// One-based line number in that file.
    pub line: u32,
}

/// The result of preprocessing one skin file.
#[derive(Debug, Clone, Default)]
pub struct Expanded {
    /// The expanded text, lines separated by `\n`.
    pub text: String,
    /// Source of each line of `text`, in order.
    pub lines: Vec<SourceLine>,
    /// File names referenced by [`SourceLine::file`].
    pub files: Vec<String>,
    /// Problems that did not stop preprocessing, such as a missing include.
    pub warnings: Vec<String>,
    /// The deepest `#include` nesting reached (0: no include).
    pub depth: usize,
    /// The most macros one included file was given.
    pub macros: usize,
    /// The bytes of text read from the skin's files, each include of a
    /// file counted.
    pub read: usize,
}

impl Expanded {
    /// Describes an expanded line (zero-based) as `file:line` for diagnostics.
    #[must_use]
    pub fn describe_line(&self, expanded_line: usize) -> String {
        match self.lines.get(expanded_line) {
            Some(source) => format!("{}:{}", self.files[source.file], source.line),
            None => format!("line {}", expanded_line + 1),
        }
    }
}

struct Frame {
    parent_active: bool,
    condition: bool,
    active: bool,
}

/// Expands `file_name` from `archive`. `variables` are matched
/// case-insensitively by name.
///
/// # Errors
/// Fails when the expansion is over one of [`crate::limits`].
pub fn expand(
    archive: &SkinArchive,
    file_name: &str,
    variables: &BTreeMap<String, String>,
) -> Result<Expanded, SkinError> {
    let mut out = Expanded::default();
    let variables: BTreeMap<String, String> = variables
        .iter()
        .map(|(name, value)| (name.to_ascii_lowercase(), value.clone()))
        .collect();
    match archive.get(file_name) {
        Some(bytes) => {
            let content = read(bytes, &mut out)?;
            expand_text(archive, file_name, &content, &variables, &[], 0, &mut out)?;
        }
        None => out.warnings.push(format!("{file_name} is missing")),
    }
    Ok(out)
}

/// A file's text, its bytes counted against what the expansion may read
/// before they are decoded: every include decodes its file, and holds the
/// text while the files inside it expand.
fn read(bytes: &[u8], out: &mut Expanded) -> Result<String, SkinError> {
    out.read = out.read.saturating_add(bytes.len());
    if out.read > limits::TEXT_BYTES {
        return Err(too_much_text());
    }
    Ok(text::decode(bytes))
}

fn too_much_text() -> SkinError {
    limits::exceeded("more bytes of markup", limits::TEXT_BYTES)
}

fn expand_text(
    archive: &SkinArchive,
    file_name: &str,
    content: &str,
    variables: &BTreeMap<String, String>,
    macros: &[(String, String)],
    depth: usize,
    out: &mut Expanded,
) -> Result<(), SkinError> {
    let file_index = out.files.len();
    // The first file is the skin's own, not an include.
    if file_index > limits::INCLUDES {
        return Err(limits::exceeded("more includes", limits::INCLUDES));
    }
    out.files.push(file_name.to_owned());
    out.depth = out.depth.max(depth);
    out.macros = out.macros.max(macros.len());
    let mut frames: Vec<Frame> = Vec::new();
    for (index, line) in content.split('\n').enumerate() {
        let line_number = u32::try_from(index + 1).unwrap_or(u32::MAX);
        let active = frames.last().is_none_or(|frame| frame.active);
        let trimmed = line.trim_start();
        if let Some(rest) = strip_directive(trimmed, "#if<") {
            let condition = active && evaluate(rest.split('>').next().unwrap_or(""), variables);
            frames.push(Frame {
                parent_active: active,
                condition,
                active: condition,
            });
            continue;
        }
        if strip_directive(trimmed, "#else").is_some() {
            if let Some(frame) = frames.last_mut() {
                frame.active = frame.parent_active && !frame.condition;
            }
            continue;
        }
        if strip_directive(trimmed, "#endif").is_some() {
            frames.pop();
            continue;
        }
        if !active {
            continue;
        }
        if let Some(rest) = strip_directive(trimmed, "#include<") {
            let (inside, after) = rest.split_once('>').unwrap_or((rest, ""));
            include(archive, inside, variables, macros, depth, out)?;
            if !after.trim().is_empty() {
                push_line(out, after, macros, file_index, line_number)?;
            }
            continue;
        }
        push_line(out, line, macros, file_index, line_number)?;
    }
    Ok(())
}

fn include(
    archive: &SkinArchive,
    directive: &str,
    variables: &BTreeMap<String, String>,
    macros: &[(String, String)],
    depth: usize,
    out: &mut Expanded,
) -> Result<(), SkinError> {
    let mut parts = directive.split_whitespace();
    let Some(file_name) = parts.next() else {
        out.warnings.push("empty #include directive".to_owned());
        return Ok(());
    };
    if depth >= limits::INCLUDE_DEPTH {
        out.warnings
            .push(format!("#include<{file_name}> is nested too deeply"));
        return Ok(());
    }
    let mut child_macros: Vec<(String, String)> = macros.to_vec();
    for part in parts {
        if let Some((name, value)) = part.strip_prefix('#').and_then(|p| p.split_once('=')) {
            child_macros.retain(|(existing, _)| existing != name);
            child_macros.push((name.to_owned(), value.to_owned()));
            if child_macros.len() > limits::MACROS {
                return Err(limits::exceeded(
                    &format!("more macros for #include<{file_name}>"),
                    limits::MACROS,
                ));
            }
        }
    }
    // Longer names first, so that #CONTROL10 is not replaced as #CONTROL1.
    child_macros.sort_by(|a, b| b.0.len().cmp(&a.0.len()).then_with(|| a.0.cmp(&b.0)));
    match archive.get(file_name) {
        Some(bytes) => {
            let content = read(bytes, out)?;
            expand_text(
                archive,
                file_name,
                &content,
                variables,
                &child_macros,
                depth + 1,
                out,
            )?;
        }
        None => out
            .warnings
            .push(format!("#include<{file_name}>: file is missing")),
    }
    Ok(())
}

fn push_line(
    out: &mut Expanded,
    line: &str,
    macros: &[(String, String)],
    file: usize,
    line_number: u32,
) -> Result<(), SkinError> {
    // What the text may still take, the line's end included.
    let room = limits::TEXT_BYTES
        .checked_sub(out.text.len() + 1)
        .ok_or_else(too_much_text)?;
    if !out.text.is_empty() {
        out.text.push('\n');
    }
    if macros.is_empty() || !line.contains('#') {
        if line.len() > room {
            return Err(too_much_text());
        }
        out.text.push_str(line);
    } else {
        let mut replaced = line.to_owned();
        for (name, value) in macros {
            let pattern = format!("#{name}");
            let count = replaced.matches(&pattern).count();
            if count == 0 {
                continue;
            }
            // The length the line will have, known before it is made.
            let grown = (replaced.len() - count * pattern.len())
                .saturating_add(count.saturating_mul(value.len()));
            if grown > room {
                return Err(too_much_text());
            }
            replaced = replaced.replace(&pattern, value);
        }
        if replaced.len() > room {
            return Err(too_much_text());
        }
        out.text.push_str(&replaced);
    }
    out.lines.push(SourceLine {
        file,
        line: line_number,
    });
    Ok(())
}

fn strip_directive<'a>(line: &'a str, directive: &str) -> Option<&'a str> {
    let head = line.get(..directive.len())?;
    head.eq_ignore_ascii_case(directive)
        .then(|| &line[directive.len()..])
}

fn evaluate(condition: &str, variables: &BTreeMap<String, String>) -> bool {
    let lookup = |name: &str| {
        variables
            .get(&name.trim().to_ascii_lowercase())
            .map_or("", String::as_str)
    };
    if let Some((name, value)) = condition.split_once("!=") {
        return !lookup(name).eq_ignore_ascii_case(value.trim());
    }
    if let Some((name, value)) = condition.split_once("==") {
        return lookup(name).eq_ignore_ascii_case(value.trim());
    }
    !lookup(condition).is_empty()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fmt::Write as _;

    use super::expand;
    use crate::archive::SkinArchive;
    use crate::error::SkinError;
    use crate::limits;

    fn archive(files: &[(&str, &str)]) -> SkinArchive {
        SkinArchive::from_files(
            "test",
            files
                .iter()
                .map(|(name, content)| (*name, content.as_bytes().to_vec())),
        )
    }

    #[test]
    fn includes_files_with_macros() {
        let skin = archive(&[
            ("Main.xml", "<A>\n\t#include<Part.txt #CTRL=Button>\n</A>"),
            ("part.txt", "<#CTRL Left='1' />"),
        ]);
        let expanded = expand(&skin, "Main.xml", &BTreeMap::new()).expect("within the limits");
        assert_eq!(expanded.text, "<A>\n<Button Left='1' />\n</A>");
        assert_eq!(expanded.describe_line(1), "Part.txt:1");
        assert!(expanded.warnings.is_empty());
    }

    #[test]
    fn evaluates_conditions() {
        let skin = archive(&[(
            "Main.xml",
            "a\n#if<SkinName==WdmSkin>\nb\n#else\nc\n#endif\nd",
        )]);
        let mut variables = BTreeMap::new();
        variables.insert("SkinName".to_owned(), "VideoSkin".to_owned());
        assert_eq!(
            expand(&skin, "Main.xml", &variables).unwrap().text,
            "a\nc\nd"
        );
        variables.insert("SkinName".to_owned(), "wdmskin".to_owned());
        assert_eq!(
            expand(&skin, "Main.xml", &variables).unwrap().text,
            "a\nb\nd"
        );
    }

    #[test]
    fn reports_missing_includes_and_cycles() {
        let skin = archive(&[
            ("a.txt", "#include<a.txt>"),
            ("b.txt", "#include<none.txt>"),
        ]);
        let warnings = |file: &str| expand(&skin, file, &BTreeMap::new()).unwrap().warnings;
        assert!(!warnings("a.txt").is_empty());
        assert!(!warnings("b.txt").is_empty());
    }

    fn limit(skin: &SkinArchive, file: &str) -> String {
        match expand(skin, file, &BTreeMap::new()) {
            Err(SkinError::Limit(what)) => what,
            Err(other) => panic!("not a limit: {other}"),
            Ok(expanded) => panic!("expanded to {} bytes", expanded.text.len()),
        }
    }

    #[test]
    fn a_parents_macro_is_passed_on_through_a_value() {
        // The value of `INNER` names `OUTER`, which is replaced after it.
        let skin = archive(&[
            ("Main.xml", "#include<a.txt #OUTER=Button>"),
            ("a.txt", "#include<b.txt #INNER=#OUTER>"),
            ("b.txt", "<#INNER/>"),
        ]);
        let expanded = expand(&skin, "Main.xml", &BTreeMap::new()).unwrap();
        assert_eq!(expanded.text, "<Button/>");
        assert_eq!((expanded.depth, expanded.macros), (2, 2));
        assert_eq!(expanded.read, 29 + 29 + 9);
    }

    #[test]
    fn macros_that_name_each_other_do_not_grow_the_text_without_end() {
        // Each macro's value names the next one four times: twenty of
        // them turn one `#M01` into a million million characters.
        let mut main = String::from("#include<part.txt ");
        for n in 1..20 {
            let next = format!("#M{:02}", n + 1);
            let _ = write!(main, "#M{n:02}={next}{next}{next}{next} ");
        }
        main.push_str("#M20=x>");
        assert!(main.len() < 600);
        let skin = archive(&[("Main.xml", main.as_str()), ("part.txt", "#M01")]);
        assert!(limit(&skin, "Main.xml").contains("bytes of markup"));
        // Three of them stay small, and expand.
        let skin = archive(&[
            ("Main.xml", "#include<part.txt #M1=#M2#M2 #M2=#M3#M3 #M3=x>"),
            ("part.txt", "<#M1>"),
        ]);
        assert_eq!(
            expand(&skin, "Main.xml", &BTreeMap::new()).unwrap().text,
            "<xxxx>"
        );
    }

    #[test]
    fn a_file_that_includes_itself_twice_stops_at_the_number_of_includes() {
        // Two includes at each of sixteen levels are 131,071 files.
        let skin = archive(&[("a.txt", "#include<a.txt>\n#include<a.txt>")]);
        assert!(limit(&skin, "a.txt").contains("includes"));
    }

    #[test]
    fn the_macros_of_an_include_are_bounded() {
        let directive = |count: usize| {
            let mut directive = String::from("#include<part.txt");
            for n in 0..count {
                let _ = write!(directive, " #N{n}={n}");
            }
            directive + ">"
        };
        let most = directive(limits::MACROS);
        let skin = archive(&[("Main.xml", most.as_str()), ("part.txt", "#N7")]);
        let expanded = expand(&skin, "Main.xml", &BTreeMap::new()).unwrap();
        assert_eq!(
            (expanded.text.as_str(), expanded.macros),
            ("7", limits::MACROS)
        );
        // A macro given again takes the place of the first.
        let again = format!("#include<part.txt{}>", " #N7=7".repeat(limits::MACROS * 4));
        let skin = archive(&[("Main.xml", again.as_str()), ("part.txt", "#N7")]);
        assert_eq!(
            expand(&skin, "Main.xml", &BTreeMap::new()).unwrap().macros,
            1
        );
        let more = directive(limits::MACROS + 1);
        let skin = archive(&[("Main.xml", more.as_str()), ("part.txt", "#N7")]);
        assert!(limit(&skin, "Main.xml").contains("macros"));
    }

    #[test]
    fn the_text_and_the_files_read_for_it_are_bounded() {
        // One file larger than the text may be.
        let large = "<a/>\n".repeat(limits::TEXT_BYTES / 5 + 1);
        let skin = archive(&[("Main.xml", large.as_str())]);
        assert!(limit(&skin, "Main.xml").contains("bytes of markup"));
        // A file within the limit, read more often than the limit holds.
        let part = "x".repeat(limits::TEXT_BYTES / 8);
        let main = "#include<part.txt>\n".repeat(9);
        let skin = archive(&[("Main.xml", main.as_str()), ("part.txt", part.as_str())]);
        assert!(limit(&skin, "Main.xml").contains("bytes of markup"));
        // Files that are read and add nothing to the text count too.
        let skipped = format!("#if<Never>\n{part}\n#endif");
        let skin = archive(&[("Main.xml", main.as_str()), ("part.txt", skipped.as_str())]);
        assert!(limit(&skin, "Main.xml").contains("bytes of markup"));
        // Seven of them are within it.
        let main = "#include<part.txt>\n".repeat(7);
        let skin = archive(&[("Main.xml", main.as_str()), ("part.txt", part.as_str())]);
        let expanded = expand(&skin, "Main.xml", &BTreeMap::new()).unwrap();
        // Seven lines and the empty one after the last line's end.
        assert_eq!(expanded.text.len(), 7 * (part.len() + 1));
        // A line that a macro makes too long, known before it is made.
        let value = "y".repeat(limits::TEXT_BYTES / 4);
        let main = format!("#include<part.txt #V={value}>");
        let skin = archive(&[("Main.xml", main.as_str()), ("part.txt", "#V#V#V#V#V")]);
        assert!(limit(&skin, "Main.xml").contains("bytes of markup"));
    }
}
