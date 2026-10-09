//! The skin text preprocessor.
//!
//! Directives sit on their own lines:
//!
//! * `#include<File.txt>` and `#include<File.txt #NAME=value ...>` insert a
//!   file; `#NAME` in the included text is replaced by `value`.
//! * `#if<Name==Value>`, `#if<Name!=Value>`, `#else`, `#endif` keep or drop
//!   lines. Names are looked up in the variables given by the caller
//!   (`SkinName`, for example) and compared case-insensitively.
//!
//! Everything else passes through unchanged.

use std::collections::BTreeMap;

use crate::archive::SkinArchive;
use crate::text;

/// Nesting limit for `#include`; protects against include cycles.
const MAX_INCLUDE_DEPTH: usize = 16;

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
#[must_use]
pub fn expand(
    archive: &SkinArchive,
    file_name: &str,
    variables: &BTreeMap<String, String>,
) -> Expanded {
    let mut out = Expanded::default();
    let variables: BTreeMap<String, String> = variables
        .iter()
        .map(|(name, value)| (name.to_ascii_lowercase(), value.clone()))
        .collect();
    match archive.get(file_name) {
        Some(bytes) => {
            let content = text::decode(bytes);
            expand_text(archive, file_name, &content, &variables, &[], 0, &mut out);
        }
        None => out.warnings.push(format!("{file_name} is missing")),
    }
    out
}

fn expand_text(
    archive: &SkinArchive,
    file_name: &str,
    content: &str,
    variables: &BTreeMap<String, String>,
    macros: &[(String, String)],
    depth: usize,
    out: &mut Expanded,
) {
    let file_index = out.files.len();
    out.files.push(file_name.to_owned());
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
            include(archive, inside, variables, macros, depth, out);
            if !after.trim().is_empty() {
                push_line(out, after, macros, file_index, line_number);
            }
            continue;
        }
        push_line(out, line, macros, file_index, line_number);
    }
}

fn include(
    archive: &SkinArchive,
    directive: &str,
    variables: &BTreeMap<String, String>,
    macros: &[(String, String)],
    depth: usize,
    out: &mut Expanded,
) {
    let mut parts = directive.split_whitespace();
    let Some(file_name) = parts.next() else {
        out.warnings.push("empty #include directive".to_owned());
        return;
    };
    if depth >= MAX_INCLUDE_DEPTH {
        out.warnings
            .push(format!("#include<{file_name}> is nested too deeply"));
        return;
    }
    let mut child_macros: Vec<(String, String)> = macros.to_vec();
    for part in parts {
        if let Some((name, value)) = part.strip_prefix('#').and_then(|p| p.split_once('=')) {
            child_macros.retain(|(existing, _)| existing != name);
            child_macros.push((name.to_owned(), value.to_owned()));
        }
    }
    // Longer names first, so that #CONTROL10 is not replaced as #CONTROL1.
    child_macros.sort_by(|a, b| b.0.len().cmp(&a.0.len()).then_with(|| a.0.cmp(&b.0)));
    match archive.get(file_name) {
        Some(bytes) => {
            let content = text::decode(bytes);
            expand_text(
                archive,
                file_name,
                &content,
                variables,
                &child_macros,
                depth + 1,
                out,
            );
        }
        None => out
            .warnings
            .push(format!("#include<{file_name}>: file is missing")),
    }
}

fn push_line(
    out: &mut Expanded,
    line: &str,
    macros: &[(String, String)],
    file: usize,
    line_number: u32,
) {
    if !out.text.is_empty() {
        out.text.push('\n');
    }
    if macros.is_empty() || !line.contains('#') {
        out.text.push_str(line);
    } else {
        let mut replaced = line.to_owned();
        for (name, value) in macros {
            replaced = replaced.replace(&format!("#{name}"), value);
        }
        out.text.push_str(&replaced);
    }
    out.lines.push(SourceLine {
        file,
        line: line_number,
    });
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

    use super::expand;
    use crate::archive::SkinArchive;

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
        let expanded = expand(&skin, "Main.xml", &BTreeMap::new());
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
        assert_eq!(expand(&skin, "Main.xml", &variables).text, "a\nc\nd");
        variables.insert("SkinName".to_owned(), "wdmskin".to_owned());
        assert_eq!(expand(&skin, "Main.xml", &variables).text, "a\nb\nd");
    }

    #[test]
    fn reports_missing_includes_and_cycles() {
        let skin = archive(&[
            ("a.txt", "#include<a.txt>"),
            ("b.txt", "#include<none.txt>"),
        ]);
        assert!(!expand(&skin, "a.txt", &BTreeMap::new()).warnings.is_empty());
        assert!(!expand(&skin, "b.txt", &BTreeMap::new()).warnings.is_empty());
    }
}
