//! M12's pixel shaders from PotPlayer: MPC-HC's single-pass HLSL (`ps_2_0`
//! and `ps_3_0`, `sampler s0`, the `p0` and `p1` constants, `tex2D`)
//! translated into an mpv GLSL hook when it loads.
//!
//! The translation is textual: RVP's prelude maps HLSL's types and
//! functions onto GLSL's with macros, and a few rewrites do what macros
//! cannot (semantics, attributes, brace initializers, a vector set from a
//! scalar, the entry point's name). Matrices keep one rule: HLSL's
//! `floatNxN(...)` lists rows and GLSL's `matN(...)` columns, so every
//! matrix is stored transposed, and `mul(a, b)` becomes `b * a`, which
//! gives HLSL's products in that same transposed storage (indexing a row
//! of HLSL's matrix is indexing a column of GLSL's).
//!
//! Pre-resize shaders hook `MAIN` (RGB at the video's size), post-resize
//! ones `OUTPUT` (after scaling). `p0` is (width, height, frame counter,
//! clock), `p1` (1/width, 1/height) and `p2` the video's rectangle in the
//! texture (left, top, right, bottom), which in mpv's hooks is the whole
//! texture; mpv has no clock, so frames at 30 a second stand in for it.
//! Shaders written with a GLSL branch (`#ifdef GLSL`) get it. A shader that
//! needs what mpv's hooks lack (a second texture, more constants) is
//! refused with the reason.

use std::fmt::Write as _;

/// Where a shader runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Stage {
    /// Before scaling (PotPlayer's pre-resize list).
    Pre,
    /// After scaling (the post-resize list).
    Post,
}

/// The prelude: HLSL's names in GLSL.
const PRELUDE: &str = "#define GLSL 1
#define float2 vec2
#define float3 vec3
#define float4 vec4
#define float2x2 mat2
#define float3x3 mat3
#define float4x4 mat4
#define half float
#define half2 vec2
#define half3 vec3
#define half4 vec4
#define int2 ivec2
#define int3 ivec3
#define int4 ivec4
#define bool2 bvec2
#define bool3 bvec3
#define bool4 bvec4
#define lerp mix
#define frac fract
#define saturate(x) clamp((x), 0.0, 1.0)
#define rcp(x) (1.0 / (x))
#define rsqrt inversesqrt
#define fmod mod
#define atan2(y, x) atan((y), (x))
#define ddx dFdx
#define ddy dFdy
#define mad(a, b, c) ((a) * (b) + (c))
#define log10(x) (log(x) * 0.4342944819)
#define mul(a, b) ((b) * (a))
#define tex2D(s, uv) HOOKED_tex(uv)
#define tex2Dlod(s, uv) HOOKED_tex((uv).xy)
#define tex2Dbias(s, uv) HOOKED_tex((uv).xy)
vec4 p0;
vec4 p1;
vec4 p2;
";

/// The shader's name from its first line (`// name=ps_2_0`), if it has one.
#[must_use]
pub fn declared_name(source: &str) -> Option<String> {
    let first = source.lines().next()?.trim().strip_prefix("//")?;
    let (name, profile) = first.rsplit_once('=')?;
    profile
        .trim()
        .starts_with("ps_")
        .then(|| name.trim().to_owned())
        .filter(|name| !name.is_empty() && !name.starts_with('$'))
}

/// Translates an MPC-HC pixel shader into an mpv hook for `stage`, named
/// `name` in mpv's log.
///
/// # Errors
/// Fails with the reason when the shader needs what mpv's hooks lack.
pub fn translate(source: &str, stage: Stage, name: &str) -> Result<String, String> {
    // A file saved as "UTF-8 with BOM" starts with one.
    let source = source.strip_prefix('\u{feff}').unwrap_or(source);
    let source = source.replace("\r\n", "\n").replace('\r', "\n");
    refuse(&source)?;
    let mut body = String::with_capacity(source.len() + 256);
    for line in source.lines() {
        if let Some(kept) = rewrite_line(line) {
            body.push_str(&kept);
            body.push('\n');
        }
    }
    let body = brace_initializers(&body)?;
    let body = rename_entry(&body)?;
    let body = vector_returns(&body);
    let hook = match stage {
        Stage::Pre => "MAIN",
        Stage::Post => "OUTPUT",
    };
    let description = name.replace(['\n', '\r'], " ");
    let mut out = String::with_capacity(body.len() + PRELUDE.len() + 512);
    let _ = writeln!(out, "//!HOOK {hook}");
    out.push_str("//!BIND HOOKED\n");
    let _ = writeln!(out, "//!DESC RVP: {description} (PotPlayer pixel shader)");
    out.push('\n');
    out.push_str(PRELUDE);
    out.push_str(&body);
    out.push_str(
        // No swizzles: a shader's own macros (`#define x 0.8`) would
        // replace them.
        "\nvec4 hook() {\n    p0 = vec4(HOOKED_size, float(frame), float(frame) / 30.0);\n    p1 = vec4(HOOKED_pt, 0.0, 0.0);\n    p2 = vec4(0.0, 0.0, 1.0, 1.0);\n    return rvp_main(HOOKED_pos);\n}\n",
    );
    Ok(out)
}

/// What mpv's hooks lack.
fn refuse(source: &str) -> Result<(), String> {
    let code = strip_comments(source);
    let compact: String = code.chars().filter(|c| !c.is_whitespace()).collect();
    if compact.contains("register(s1)")
        || compact.contains("samplers1")
        || compact.contains("tex2D(s1,")
    {
        return Err("it reads a second texture (s1), which mpv's hooks do not give".to_owned());
    }
    for (pattern, reason) in [
        ("VPOS", "it reads the screen position (VPOS)"),
        ("tex2Dproj", "it uses projected sampling (tex2Dproj)"),
        ("tex2Dgrad", "it uses gradient sampling (tex2Dgrad)"),
    ] {
        if contains_word(&code, pattern) {
            return Err(reason.to_owned());
        }
    }
    for register in 3..16 {
        if code.contains(&format!("register(c{register})")) {
            return Err(format!(
                "it reads constant c{register}; PotPlayer gives only p0, p1 and p2"
            ));
        }
    }
    if !code.contains("main") {
        return Err("it has no main function".to_owned());
    }
    Ok(())
}

/// The source without `//` and `/* */` comments (for the checks only).
fn strip_comments(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut rest = source;
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix("//") {
            rest = after.find('\n').map_or("", |end| &after[end..]);
        } else if let Some(after) = rest.strip_prefix("/*") {
            rest = after.find("*/").map_or("", |end| &after[end + 2..]);
        } else {
            let mut chars = rest.chars();
            if let Some(c) = chars.next() {
                out.push(c);
            }
            rest = chars.as_str();
        }
    }
    out
}

/// True when `word` appears in `text` as a whole word.
fn contains_word(text: &str, word: &str) -> bool {
    let is_word = |c: char| c.is_ascii_alphanumeric() || c == '_';
    text.match_indices(word).any(|(at, _)| {
        let before = text[..at].chars().next_back();
        let after = text[at + word.len()..].chars().next();
        !before.is_some_and(is_word) && !after.is_some_and(is_word)
    })
}

/// One line: declarations PotPlayer's constants make are dropped, and
/// semantics, attributes, `static` and float suffixes go.
fn rewrite_line(line: &str) -> Option<String> {
    let code = line.trim();
    // A global `float2 c0;` is register c0, PotPlayer's p0.
    let compact: String = code.chars().filter(|c| !c.is_whitespace()).collect();
    match compact.as_str() {
        "float2c0;" | "half2c0;" => return Some("#define c0 (p0.xy)".to_owned()),
        "float4c0;" | "half4c0;" => return Some("#define c0 p0".to_owned()),
        _ => {}
    }
    // RVP declares the texture and the constants itself.
    if (code.starts_with("sampler") || code.starts_with("Texture2D") || code.starts_with("texture"))
        && code.ends_with(';')
    {
        return None;
    }
    let compact: String = code.chars().filter(|c| !c.is_whitespace()).collect();
    if ["register(c0)", "register(c1)", "register(c2)"]
        .iter()
        .any(|register| compact.contains(register))
    {
        return None;
    }
    let mut out = line.to_owned();
    for attribute in ["[unroll]", "[loop]", "[branch]", "[flatten]", "[fastopt]"] {
        out = out.replace(attribute, "");
    }
    out = remove_unroll_counts(&out);
    for semantic in [
        ": TEXCOORD0",
        ":TEXCOORD0",
        ": TEXCOORD",
        ":TEXCOORD",
        ": COLOR0",
        ":COLOR0",
        ": COLOR",
        ":COLOR",
        ": SV_Target",
        ":SV_Target",
    ] {
        out = out.replace(semantic, "");
    }
    out = out
        .replace("static const ", "const ")
        .replace("static ", "");
    out = float_suffixes(&out);
    out = scalar_initializer(&out);
    Some(out)
}

/// `[unroll(8)]` and the like.
fn remove_unroll_counts(line: &str) -> String {
    let mut out = line.to_owned();
    while let Some(start) = out.find("[unroll(").or_else(|| out.find("[loop(")) {
        let Some(end) = out[start..].find(")]") else {
            break;
        };
        out.replace_range(start..start + end + 2, "");
    }
    out
}

/// `1.5f` and `2f` lose their suffix. The line is copied in slices
/// between the suffixes, so text outside ASCII (in comments) stays whole.
fn float_suffixes(line: &str) -> String {
    let bytes = line.as_bytes();
    let mut out = String::with_capacity(line.len());
    let mut copied = 0;
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        let starts_number = (c.is_ascii_digit() || c == b'.')
            && (i == 0 || !(bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_'));
        if starts_number {
            while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                i += 1;
            }
            // An exponent: `1e-3`.
            if i < bytes.len()
                && (bytes[i] == b'e' || bytes[i] == b'E')
                && i + 1 < bytes.len()
                && (bytes[i + 1].is_ascii_digit() || bytes[i + 1] == b'-' || bytes[i + 1] == b'+')
            {
                i += 2;
                while i < bytes.len() && bytes[i].is_ascii_digit() {
                    i += 1;
                }
            }
            let suffix = i < bytes.len() && (bytes[i] == b'f' || bytes[i] == b'F');
            let ends = i + 1 >= bytes.len()
                || !(bytes[i + 1].is_ascii_alphanumeric() || bytes[i + 1] == b'_');
            if suffix && ends {
                out.push_str(&line[copied..i]);
                i += 1;
                copied = i;
            }
            continue;
        }
        i += 1;
    }
    out.push_str(&line[copied..]);
    out
}

/// `float4 c = 0;` sets every component in HLSL; GLSL needs the
/// constructor: `float4 c = float4(0);` (a vector of the size is kept).
fn scalar_initializer(line: &str) -> String {
    let code = line.trim_start();
    let indent = &line[..line.len() - code.len()];
    let (prefix, rest) = match code.strip_prefix("const ") {
        Some(rest) => ("const ", rest),
        None => ("", code),
    };
    for ty in ["float2", "float3", "float4", "half2", "half3", "half4"] {
        let Some(after) = rest
            .strip_prefix(ty)
            .and_then(|after| after.strip_prefix(' '))
        else {
            continue;
        };
        let Some((name, value)) = after.split_once('=') else {
            return line.to_owned();
        };
        let name = name.trim();
        let value = value.trim();
        let simple_name =
            !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
        let Some(value) = value.strip_suffix(';') else {
            return line.to_owned();
        };
        if !simple_name || value.starts_with('{') || value.contains(',') && !value.contains('(') {
            return line.to_owned();
        }
        return format!("{indent}{prefix}{ty} {name} = {ty}({});", value.trim());
    }
    line.to_owned()
}

/// `T name[N] = { ... };` and `T name = { ... };` become constructors:
/// `T[N](T(...), ...)` and `T(...)`.
fn brace_initializers(body: &str) -> Result<String, String> {
    let mut out = String::with_capacity(body.len());
    let mut rest = body;
    while let Some(at) = rest.find('{') {
        let before = &rest[..at];
        // Only initializers: `= {`.
        if !before.trim_end().ends_with('=') {
            out.push_str(&rest[..=at]);
            rest = &rest[at + 1..];
            continue;
        }
        let statement_start = before.rfind([';', '}', '{']).map_or(0, |i| i + 1);
        // The declaration's own lines (not the preprocessor's or comments).
        let declaration: Vec<&str> = before[statement_start..]
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#') && !line.starts_with("//"))
            .collect();
        let declaration = declaration.join(" ");
        let declaration = declaration.trim_end().trim_end_matches('=').trim();
        let (ty, array) = declared_type(declaration)
            .ok_or_else(|| format!("an initializer list it cannot read: {declaration}"))?;
        let end = matching_brace(rest, at).ok_or("an unbalanced initializer list")?;
        let inner = &rest[at + 1..end];
        let element = element_type(&ty);
        let converted = convert_list(inner, &element);
        out.push_str(before);
        if array {
            let _ = write!(out, "{ty}[]({converted})");
        } else {
            let _ = write!(out, "{ty}({converted})");
        }
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

/// The type of `const float3 name[4]` and whether it is an array.
fn declared_type(declaration: &str) -> Option<(String, bool)> {
    let words: Vec<&str> = declaration
        .split_whitespace()
        .filter(|word| !matches!(*word, "const" | "uniform" | "static"))
        .collect();
    let (ty, name) = (words.first()?, words.get(1)?);
    let array = name.contains('[');
    Some(((*ty).to_owned(), array))
}

/// What one element of a list of `ty` is: a vector's or a matrix's
/// scalars, an array's vectors.
fn element_type(ty: &str) -> String {
    match ty {
        "float2" | "float3" | "float4" | "float2x2" | "float3x3" | "float4x4" => "float".to_owned(),
        "half2" | "half3" | "half4" => "half".to_owned(),
        other => other.to_owned(),
    }
}

/// The items of a brace list; nested lists become `element(...)`.
fn convert_list(inner: &str, element: &str) -> String {
    let mut out = String::with_capacity(inner.len());
    let mut rest = inner;
    while let Some(at) = rest.find('{') {
        out.push_str(&rest[..at]);
        if let Some(end) = matching_brace(rest, at) {
            let _ = write!(out, "{element}({})", &rest[at + 1..end]);
            rest = &rest[end + 1..];
        } else {
            out.push_str(&rest[at..]);
            rest = "";
        }
    }
    out.push_str(rest);
    // A trailing comma is HLSL's.
    out.trim().trim_end_matches(',').to_owned()
}

/// The `}` that closes the `{` at `open`.
fn matching_brace(text: &str, open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (i, c) in text[open..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + i);
                }
            }
            _ => {}
        }
    }
    None
}

/// HLSL turns a scalar the entry point returns into a vector; GLSL needs
/// the constructor: `return x;` becomes `return float4(x);` (a vector
/// stays as it is).
fn vector_returns(body: &str) -> String {
    let Some(entry) = body.find("rvp_main(") else {
        return body.to_owned();
    };
    let Some(open) = body[entry..].find('{').map(|at| entry + at) else {
        return body.to_owned();
    };
    let Some(close) = matching_brace(body, open) else {
        return body.to_owned();
    };
    let function = &body[open..=close];
    let bytes = function.as_bytes();
    let mut out = String::with_capacity(function.len() + 64);
    let mut i = 0;
    let mut copied = 0;
    while i < bytes.len() {
        // Comments are copied as they are.
        if function[i..].starts_with("//") {
            i = function[i..].find('\n').map_or(bytes.len(), |end| i + end);
            continue;
        }
        if function[i..].starts_with("/*") {
            i = function[i..]
                .find("*/")
                .map_or(bytes.len(), |end| i + end + 2);
            continue;
        }
        let word_before = i > 0 && (bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_');
        let is_return = !word_before
            && function[i..].starts_with("return")
            && !function[i + 6..]
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_');
        if !is_return {
            // By characters: the code may hold text outside ASCII.
            i += function[i..].chars().next().map_or(1, char::len_utf8);
            continue;
        }
        // The value runs to the `;` outside parentheses.
        let value_start = i + 6;
        let mut depth = 0i32;
        let mut end = None;
        for (at, c) in function[value_start..].char_indices() {
            match c {
                '(' => depth += 1,
                ')' => depth -= 1,
                ';' if depth == 0 => {
                    end = Some(value_start + at);
                    break;
                }
                _ => {}
            }
        }
        let Some(end) = end else {
            break;
        };
        out.push_str(&function[copied..i]);
        let _ = write!(out, "return float4({})", function[value_start..end].trim());
        copied = end;
        i = end;
    }
    out.push_str(&function[copied..]);
    format!("{}{}{}", &body[..open], out, &body[close + 1..])
}

/// The entry point becomes `rvp_main` (mpv's shader has its own `main`).
fn rename_entry(body: &str) -> Result<String, String> {
    let mut out = String::with_capacity(body.len());
    let mut renamed = false;
    let mut rest = body;
    while let Some(at) = rest.find("main") {
        let before = rest[..at].chars().next_back();
        let after = rest[at + 4..].trim_start().chars().next();
        let word = !before.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_');
        out.push_str(&rest[..at]);
        if word && after == Some('(') {
            out.push_str("rvp_main");
            renamed = true;
        } else {
            out.push_str("main");
        }
        rest = &rest[at + 4..];
    }
    out.push_str(rest);
    if renamed {
        Ok(out)
    } else {
        Err("it has no main function".to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Shaders written for these tests in MPC-HC's form.
    const INVERT: &str = "// invert=ps_2_0\r\n\r\nsampler s0 : register(s0);\r\n\r\nfloat4 main(float2 tex : TEXCOORD0) : COLOR\r\n{\r\n\treturn float4(1, 1, 1, 1) - tex2D(s0, tex);\r\n}\r\n";

    const LUMA: &str = "// luma test=ps_2_0
sampler s0 : register(s0);
float4 p0 : register(c0);
float4 p1 : register(c1);
#define width (p0[0])
#define one_over_width (p1[0])
static const float3x3 toYuv = {
    0.2126, 0.7152, 0.0722,
    -0.1146, -0.3854, 0.5,
    0.5, -0.4542, -0.0458,
};
static const float2 offsets[2] = { {1.0f, 0}, {-1.0f, 0} };

float4 main(float2 tex : TEXCOORD0) : COLOR
{
\tfloat4 c0 = 0;
\t[unroll] for (int i = 0; i < 2; i++)
\t\tc0 += tex2D(s0, tex + offsets[i] * one_over_width) * 0.5f;
\tfloat3 yuv = mul(toYuv, c0.rgb);
\treturn float4(yuv.xxx, 1);
}
";

    #[test]
    fn mpc_shaders_become_mpv_hooks() {
        let glsl = translate(INVERT, Stage::Post, "Invert").unwrap_or_default();
        assert!(glsl.starts_with("//!HOOK OUTPUT\n//!BIND HOOKED\n//!DESC RVP: Invert"));
        assert!(!glsl.contains("sampler"));
        assert!(!glsl.contains("TEXCOORD") && !glsl.contains(": COLOR"));
        assert!(glsl.contains("float4 rvp_main(float2 tex )"), "{glsl}");
        assert!(glsl.contains("return rvp_main(HOOKED_pos);"));
        assert!(glsl.contains("#define tex2D(s, uv) HOOKED_tex(uv)"));
        let pre = translate(INVERT, Stage::Pre, "Invert").unwrap_or_default();
        assert!(pre.starts_with("//!HOOK MAIN\n"));
    }

    #[test]
    fn hlsl_forms_are_rewritten() {
        let glsl = translate(LUMA, Stage::Pre, "Luma").unwrap_or_default();
        // PotPlayer's constants are RVP's, the attribute and suffixes go.
        assert!(!glsl.contains("register("));
        assert!(!glsl.contains("[unroll]"));
        assert!(glsl.contains("* 0.5;"), "{glsl}");
        assert!(
            glsl.contains("{1.0, 0}") || glsl.contains("float(1.0, 0)"),
            "{glsl}"
        );
        // Brace lists become constructors; a scalar fills a vector.
        assert!(glsl.contains("const float3x3 toYuv = float3x3("), "{glsl}");
        assert!(
            glsl.contains("const float2 offsets[2] = float2[](float(1.0, 0), float(-1.0, 0))")
                || glsl.contains("const float2 offsets[2] = float2[]("),
            "{glsl}"
        );
        assert!(glsl.contains("float4 c0 = float4(0);"), "{glsl}");
        assert!(glsl.contains("#define mul(a, b) ((b) * (a))"));
    }

    #[test]
    fn text_outside_ascii_is_kept_whole() {
        // Saved with a BOM, a no-break space in the entry point's code (as
        // pasted from a web page), and comments in other scripts.
        let source = "\u{feff}// Яркость=ps_2_0\nsampler s0 : register(s0);\n// Коэффициент 1.5f\nfloat4 main(float2 tex : TEXCOORD0) : COLOR\n{\n\tfloat\u{a0}k = 1.5f; // é\n\treturn tex2D(s0, tex) * k; /* è */\n}\n";
        let glsl = translate(source, Stage::Pre, "x").expect("translates");
        assert!(glsl.contains("// Коэффициент 1.5\n"), "{glsl}");
        assert!(glsl.contains("float\u{a0}k = 1.5; // é"), "{glsl}");
        assert!(
            glsl.contains("return float4(tex2D(s0, tex) * k); /* è */"),
            "{glsl}"
        );
        assert!(!glsl.contains('\u{feff}'), "{glsl}");
        assert_eq!(
            declared_name("// Яркость=ps_2_0"),
            Some("Яркость".to_owned())
        );
    }

    #[test]
    fn what_mpv_lacks_is_refused_with_its_reason() {
        let second = "sampler s0 : register(s0);\nsampler s1 : register(s1);\nfloat4 main(float2 t : TEXCOORD0) : COLOR { return tex2D(s1, t); }";
        assert_eq!(
            translate(second, Stage::Pre, "x"),
            Err("it reads a second texture (s1), which mpv's hooks do not give".to_owned())
        );
        let constant =
            "float4 p3 : register(c3);\nfloat4 main(float2 t : TEXCOORD0) : COLOR { return p3; }";
        assert!(translate(constant, Stage::Pre, "x").is_err_and(|e| e.contains("c3")));
        // The video's rectangle (c2) is the whole texture in mpv's hooks.
        let rect =
            "float4 p2 : register(c2);\nfloat4 main(float2 t : TEXCOORD0) : COLOR { return p2; }";
        assert!(
            translate(rect, Stage::Pre, "x")
                .is_ok_and(|glsl| glsl.contains("p2 = vec4(0.0, 0.0, 1.0, 1.0);"))
        );
        // A variable named s1 is no second texture.
        let named = "sampler s0 : register(s0);\nfloat4 main(float2 t : TEXCOORD0) : COLOR { float4 s1 = tex2D(s0, t); return s1; }";
        assert!(translate(named, Stage::Pre, "x").is_ok());
        assert_eq!(declared_name(INVERT), Some("invert".to_owned()));
        assert_eq!(declared_name("// $MinimumShaderProfile=ps_3_0"), None);
    }
}
