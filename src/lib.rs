use lumis::highlight::highlight_iter;
use lumis::languages::Language;
use lumis::themes as lumis_themes;
use thiserror::Error;

#[cfg(feature = "themes")]
use lumis::themes::Style;
#[cfg(feature = "pyo3")]
use pyo3::exceptions::PyValueError;
#[cfg(feature = "pyo3")]
use pyo3::prelude::*;

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
    lang.parse::<Language>()
        .map_err(|_| HighlightError::UnknownLanguage(lang.to_string()))
}

pub fn tokenize(code: &str, lang: &str) -> Result<Vec<Token>, HighlightError> {
    let language = parse_lang(lang)?;
    let mut toks = Vec::new();
    highlight_iter(
        code,
        language,
        None,
        |_, _, range, scope, _| -> Result<(), std::fmt::Error> {
            if !scope.is_empty() {
                toks.push(Token {
                    start: range.start,
                    end: range.end,
                    kind: scope.to_string(),
                });
            }
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

#[cfg(feature = "pyo3")]
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

#[cfg(feature = "pyo3")]
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

pub fn themes() -> Vec<&'static str> {
    let mut names: Vec<_> = lumis_themes::available_themes()
        .map(|theme| theme.name.as_str())
        .collect();
    names.sort_unstable();
    names
}

#[cfg(feature = "pyo3")]
fn py_err(err: HighlightError) -> PyErr {
    PyValueError::new_err(err.to_string())
}

#[cfg(feature = "pyo3")]
#[pyfunction(name = "tokenize")]
fn py_tokenize(code: &str, lang: &str) -> PyResult<Vec<(usize, usize, String)>> {
    tokenize(code, lang)
        .map(|toks| {
            toks.into_iter()
                .map(|tok| (tok.start, tok.end, tok.kind))
                .collect()
        })
        .map_err(py_err)
}

#[cfg(feature = "pyo3")]
#[pyfunction(name = "highlight")]
fn py_highlight(code: &str, lang: &str) -> PyResult<String> {
    highlight_component(code, lang).map_err(py_err)
}

#[cfg(feature = "pyo3")]
#[pyfunction(name = "highlight_spans")]
#[pyo3(signature = (code, lang, class_prefix=None))]
fn py_highlight_spans(code: &str, lang: &str, class_prefix: Option<&str>) -> PyResult<String> {
    highlight_spans(code, lang, class_prefix.unwrap_or("hl-")).map_err(py_err)
}

#[cfg(feature = "pyo3")]
#[pyfunction(name = "languages")]
fn py_languages() -> Vec<&'static str> {
    languages()
}

#[cfg(feature = "pyo3")]
#[pyfunction(name = "theme_css")]
#[pyo3(signature = (theme, selector=None, class_prefix=None))]
fn py_theme_css(
    theme: &str,
    selector: Option<&str>,
    class_prefix: Option<&str>,
) -> PyResult<String> {
    theme_css(theme, selector, class_prefix.unwrap_or("")).map_err(py_err)
}

#[cfg(feature = "pyo3")]
#[pyfunction(name = "themes")]
fn py_themes() -> Vec<&'static str> {
    themes()
}

#[cfg(feature = "pyo3")]
#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(py_tokenize, m)?)?;
    m.add_function(wrap_pyfunction!(py_highlight, m)?)?;
    m.add_function(wrap_pyfunction!(py_highlight_spans, m)?)?;
    m.add_function(wrap_pyfunction!(py_languages, m)?)?;
    m.add_function(wrap_pyfunction!(py_theme_css, m)?)?;
    m.add_function(wrap_pyfunction!(py_themes, m)?)?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenize_python_returns_sorted_tokens() {
        let toks = tokenize("def f(): return 1", "python").unwrap();
        assert!(!toks.is_empty());
        let starts: Vec<_> = toks.iter().map(|tok| tok.start).collect();
        let mut sorted = starts.clone();
        sorted.sort();
        assert_eq!(starts, sorted);
    }

    #[test]
    fn highlighted_inner_has_only_inner_markup() {
        let html = highlighted_inner("if x < 1:\n    return \"&\"", "python", "hl-").unwrap();
        assert!(html.contains("<span class=\"hl-keyword"));
        assert!(html.contains("&lt;"));
        assert!(html.contains("&quot;&amp;&quot;"));
        assert!(!html.contains("<pre>"));
        assert!(!html.contains("<code>"));
    }

    #[test]
    fn highlight_spans_wraps_pre_code() {
        let html = highlight_spans("let x = 1;", "javascript", "hl-").unwrap();
        assert!(html.starts_with("<pre><code>"));
        assert!(html.ends_with("</code></pre>"));
    }

    #[test]
    fn unknown_language_errors() {
        assert!(matches!(
            tokenize("x", "not-a-language"),
            Err(HighlightError::UnknownLanguage(_))
        ));
    }

    #[test]
    fn non_ascii_input_is_safe() {
        let html = highlighted_inner("s = \"é\"\n", "python", "hl-").unwrap();
        assert!(html.contains("é"));
    }

    #[test]
    fn theme_css_emits_class_selectors() {
        let css = theme_css("github_light", Some("pre code"), "hl-").unwrap();
        assert!(css.contains("pre code .hl-"));
    }

    #[test]
    fn theme_css_emits_css_highlight_selectors() {
        let css = theme_css("github_light", None, "hl-").unwrap();
        assert!(css.contains("::highlight("));
    }
}
