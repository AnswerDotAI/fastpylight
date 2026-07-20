use lumis::highlight::highlight_iter;
use lumis::languages::Language;
use lumis::themes as lumis_themes;
use thiserror::Error;

use std::panic::{catch_unwind, AssertUnwindSafe};

#[cfg(feature = "themes")]
use lumis::themes::Style;
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyDict;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Token {
    pub start: usize,
    pub end: usize,
    pub kind: String,
}

#[derive(Debug, Error)]
pub enum HighlightError {
    #[error("unknown language: {0}")]
    UnknownLanguage(String),
    #[error("highlight error: {0}")]
    Highlight(String),
    #[error("theme parse error: {0}")]
    Theme(String),
}

fn parse_lang(lang: &str) -> Result<Language, HighlightError> {
    // `PlainText` has no `FromStr` alias in lumis, so its `id_name` can't be
    // parsed; special-case it so the requested-language path can select it.
    if lang.eq_ignore_ascii_case(Language::PlainText.id_name()) {
        return Ok(Language::PlainText);
    }
    lang.parse::<Language>()
        .map_err(|_| HighlightError::UnknownLanguage(lang.to_string()))
}

pub fn guess(lang: Option<&str>, code: &str) -> &'static str {
    Language::guess(lang, code).id_name()
}

pub fn tokenize(code: &str, lang: &str) -> Result<Vec<Token>, HighlightError> {
    let language = parse_lang(lang)?;
    let host_md = matches!(language, Language::Markdown | Language::MarkdownInline);
    let mut toks: Vec<Token> = Vec::new();
    highlight_iter(
        code,
        language,
        None,
        |_, tok_lang, range, scope, _| -> Result<(), std::fmt::Error> {
            // Markdown is quotation: injected-language tokens (fence bodies, raw HTML)
            // flatten to the base markup.raw.block scope instead of impersonating the
            // embedded language; markdown's own structure keeps its scopes.
            let foreign =
                host_md && !matches!(tok_lang, Language::Markdown | Language::MarkdownInline);
            let scope = if foreign { "markup.raw.block" } else { scope };
            if scope.is_empty() {
                return Ok(());
            }
            if host_md {
                if let Some(last) = toks.last_mut() {
                    if last.kind == scope && last.end >= range.start {
                        last.end = last.end.max(range.end);
                        return Ok(());
                    }
                }
            }
            toks.push(Token {
                start: range.start,
                end: range.end,
                kind: scope.to_string(),
            });
            Ok(())
        },
    )
    .map_err(|e| HighlightError::Highlight(e.to_string()))?;
    toks.sort_by_key(|tok| (tok.start, tok.end));
    Ok(toks)
}

fn html_escape(s: &str, out: &mut String) {
    for c in s.chars() {
        match c {
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
}

fn class_name(kind: &str, class_prefix: &str) -> String {
    format!("{class_prefix}{}", kind.replace('.', "-"))
}

fn push_escaped_slice(
    code: &str,
    start: usize,
    end: usize,
    out: &mut String,
) -> Result<(), HighlightError> {
    match code.get(start..end) {
        Some(s) => {
            html_escape(s, out);
            Ok(())
        }
        None => Err(HighlightError::Highlight(format!(
            "invalid UTF-8 token range: {start}..{end}"
        ))),
    }
}

pub fn write_highlighted_inner(
    code: &str,
    lang: &str,
    class_prefix: &str,
    out: &mut String,
) -> Result<(), HighlightError> {
    let toks = tokenize(code, lang)?;
    let mut pos = 0usize;
    for tok in &toks {
        if tok.end == tok.start {
            continue;
        }
        if tok.start < pos || tok.end < tok.start || tok.end > code.len() {
            return Err(HighlightError::Highlight(format!(
                "invalid token range: {}..{}",
                tok.start, tok.end
            )));
        }
        if pos < tok.start {
            push_escaped_slice(code, pos, tok.start, out)?;
        }
        out.push_str("<span class=\"");
        out.push_str(&class_name(&tok.kind, class_prefix));
        out.push_str("\">");
        push_escaped_slice(code, tok.start, tok.end, out)?;
        out.push_str("</span>");
        pos = tok.end;
    }
    if pos < code.len() {
        push_escaped_slice(code, pos, code.len(), out)?;
    }
    Ok(())
}

pub fn highlighted_inner(
    code: &str,
    lang: &str,
    class_prefix: &str,
) -> Result<String, HighlightError> {
    let mut out = String::with_capacity(code.len() * 2);
    write_highlighted_inner(code, lang, class_prefix, &mut out)?;
    Ok(out)
}

pub fn highlight_spans(
    code: &str,
    lang: &str,
    class_prefix: &str,
) -> Result<String, HighlightError> {
    let inner = highlighted_inner(code, lang, class_prefix)?;
    let mut out = String::with_capacity(inner.len() + 24);
    out.push_str("<pre><code>");
    out.push_str(&inner);
    out.push_str("</code></pre>");
    Ok(out)
}

fn byte_to_utf16_table(s: &str) -> Vec<usize> {
    let mut table = vec![0usize; s.len() + 1];
    let mut utf16_idx = 0usize;
    for (byte_idx, ch) in s.char_indices() {
        table[byte_idx] = utf16_idx;
        utf16_idx += ch.len_utf16();
    }
    table[s.len()] = utf16_idx;
    table
}

fn highlight_component(code: &str, lang: &str) -> Result<String, HighlightError> {
    let toks = tokenize(code, lang)?;
    let b2c = byte_to_utf16_table(code);
    let mut toks_json = String::from("[");
    for (i, tok) in toks.iter().enumerate() {
        if i > 0 {
            toks_json.push(',');
        }
        let cs = b2c[tok.start];
        let ce = b2c[tok.end];
        toks_json.push_str(&format!("[{cs},{ce},\"{}\"]", tok.kind.replace('.', "-")));
    }
    toks_json.push(']');
    let mut out = String::with_capacity(code.len() + toks_json.len() + 64);
    out.push_str("<hl-code toks='");
    out.push_str(&toks_json);
    out.push_str("'><pre><code>");
    html_escape(code, &mut out);
    out.push_str("</code></pre></hl-code>");
    Ok(out)
}

pub fn languages() -> Vec<&'static str> {
    Language::iter()
        .map(|language| language.id_name())
        .collect()
}

#[cfg(feature = "themes")]
fn style_to_css(style: &Style) -> String {
    style.css(true, " ")
}

#[cfg(feature = "themes")]
pub fn theme_css(
    theme: &str,
    selector: Option<&str>,
    class_prefix: &str,
) -> Result<String, HighlightError> {
    let theme = lumis_themes::get(theme).map_err(|e| HighlightError::Theme(e.to_string()))?;
    let mut out = String::with_capacity(4096);
    for (scope, style) in &theme.highlights {
        if scope == "normal" {
            continue;
        }
        let css = style_to_css(style);
        if css.is_empty() {
            continue;
        }
        let scope = scope.replace('.', "-");
        match selector {
            Some(sel) => out.push_str(&format!("{sel} .{class_prefix}{scope} {{ {css} }}\n")),
            None => out.push_str(&format!("::highlight({scope}) {{ {css} }}\n")),
        }
    }
    Ok(out)
}

#[cfg(not(feature = "themes"))]
pub fn theme_css(
    _theme: &str,
    _selector: Option<&str>,
    _class_prefix: &str,
) -> Result<String, HighlightError> {
    Err(HighlightError::Theme("themes feature is disabled".into()))
}

/// Per-scope theme styles as data: (scope, fg, bg, bold, italic, underline, strikethrough).
/// Unlike `theme_css`, scope names stay dotted and the `normal` scope is included.
#[cfg(feature = "themes")]
pub fn theme_colors(theme: &str) -> Result<ThemeColors, HighlightError> {
    let theme = lumis_themes::get(theme).map_err(|e| HighlightError::Theme(e.to_string()))?;
    Ok(theme
        .highlights
        .iter()
        .map(|(scope, style)| {
            let underline = match style.text_decoration.underline {
                lumis_themes::UnderlineStyle::None => None,
                lumis_themes::UnderlineStyle::Solid => Some("solid"),
                lumis_themes::UnderlineStyle::Wavy => Some("wavy"),
                lumis_themes::UnderlineStyle::Double => Some("double"),
                lumis_themes::UnderlineStyle::Dotted => Some("dotted"),
                lumis_themes::UnderlineStyle::Dashed => Some("dashed"),
            };
            (
                scope.clone(),
                style.fg.clone(),
                style.bg.clone(),
                style.bold,
                style.italic,
                underline,
                style.text_decoration.strikethrough,
            )
        })
        .collect())
}

pub type ThemeColors = Vec<(
    String,
    Option<String>,
    Option<String>,
    bool,
    bool,
    Option<&'static str>,
    bool,
)>;

#[cfg(not(feature = "themes"))]
pub fn theme_colors(_theme: &str) -> Result<ThemeColors, HighlightError> {
    Err(HighlightError::Theme("themes feature is disabled".into()))
}

pub fn themes() -> Vec<&'static str> {
    let mut names: Vec<_> = lumis_themes::available_themes()
        .map(|theme| theme.name.as_str())
        .collect();
    names.sort_unstable();
    names
}

fn py_err(err: HighlightError) -> PyErr {
    PyValueError::new_err(err.to_string())
}

/// Run a panic-prone pure-Rust step, converting any panic into a clean
/// `RuntimeError` instead of surfacing pyo3's `BaseException`-derived
/// `PanicException`.
fn guard<T>(what: &str, f: impl FnOnce() -> T) -> PyResult<T> {
    catch_unwind(AssertUnwindSafe(f)).map_err(|_| {
        PyRuntimeError::new_err(format!(
            "internal error in fastpylight while {what} (this is a bug, please report it)"
        ))
    })
}

#[pyfunction(name = "tokenize")]
fn py_tokenize(code: &str, lang: &str) -> PyResult<Vec<(usize, usize, String)>> {
    guard("tokenizing", || tokenize(code, lang))?
        .map(|toks| {
            toks.into_iter()
                .map(|tok| (tok.start, tok.end, tok.kind))
                .collect()
        })
        .map_err(py_err)
}

#[pyfunction(name = "highlight")]
fn py_highlight(code: &str, lang: &str) -> PyResult<String> {
    guard("highlighting", || highlight_component(code, lang))?.map_err(py_err)
}

#[pyfunction(name = "highlight_spans")]
#[pyo3(signature = (code, lang, class_prefix=None))]
fn py_highlight_spans(code: &str, lang: &str, class_prefix: Option<&str>) -> PyResult<String> {
    let cp = class_prefix.unwrap_or("hl-");
    guard("highlighting", || highlight_spans(code, lang, cp))?.map_err(py_err)
}

#[pyfunction(name = "languages")]
fn py_languages() -> Vec<&'static str> {
    languages()
}

#[pyfunction(name = "guess")]
#[pyo3(signature = (code, lang=None))]
fn py_guess(code: &str, lang: Option<&str>) -> PyResult<&'static str> {
    guard("guessing language", || guess(lang, code))
}

#[pyfunction(name = "theme_css")]
#[pyo3(signature = (theme, selector=None, class_prefix=None))]
fn py_theme_css(
    theme: &str,
    selector: Option<&str>,
    class_prefix: Option<&str>,
) -> PyResult<String> {
    let cp = class_prefix.unwrap_or("");
    guard("building theme css", || theme_css(theme, selector, cp))?.map_err(py_err)
}

#[pyfunction(name = "theme_colors")]
fn py_theme_colors(py: Python<'_>, theme: &str) -> PyResult<Py<PyDict>> {
    let rows = guard("reading theme colors", || theme_colors(theme))?.map_err(py_err)?;
    let out = PyDict::new(py);
    for (scope, fg, bg, bold, italic, underline, strikethrough) in rows {
        let d = PyDict::new(py);
        d.set_item("fg", fg)?;
        d.set_item("bg", bg)?;
        d.set_item("bold", bold)?;
        d.set_item("italic", italic)?;
        d.set_item("underline", underline)?;
        d.set_item("strikethrough", strikethrough)?;
        out.set_item(scope, d)?;
    }
    Ok(out.into())
}

#[pyfunction(name = "themes")]
fn py_themes() -> Vec<&'static str> {
    themes()
}

#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(py_tokenize, m)?)?;
    m.add_function(wrap_pyfunction!(py_highlight, m)?)?;
    m.add_function(wrap_pyfunction!(py_highlight_spans, m)?)?;
    m.add_function(wrap_pyfunction!(py_languages, m)?)?;
    m.add_function(wrap_pyfunction!(py_guess, m)?)?;
    m.add_function(wrap_pyfunction!(py_theme_css, m)?)?;
    m.add_function(wrap_pyfunction!(py_theme_colors, m)?)?;
    m.add_function(wrap_pyfunction!(py_themes, m)?)?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
