use inkjet::constants::HIGHLIGHT_NAMES;
use inkjet::{Highlighter, Language};
use std::sync::LazyLock;
use thiserror::Error;
use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter as TSHighlighter};

#[cfg(feature = "themes")]
use inkjet::theme::{vendored, Modifier, Theme};

#[cfg(feature = "python")]
use pyo3::exceptions::PyValueError;
#[cfg(feature = "python")]
use pyo3::prelude::*;

#[cfg(any(feature = "all-languages", feature = "language-python"))]
extern "C" {
    fn tree_sitter_python() -> tree_sitter::Language;
}

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

fn remap_queries(src: &str) -> String {
    src.replace("@text.title", "@markup.heading")
        .replace("@text.literal", "@markup.raw")
        .replace("@text.emphasis", "@markup.italic")
        .replace("@text.strong", "@markup.bold")
        .replace("@text.uri", "@markup.link.url")
        .replace("@text.reference", "@markup.link.label")
        .replace("@string.escape", "@constant.character.escape")
        .replace("@none", "@comment")
}

#[cfg(any(feature = "all-languages", feature = "language-python"))]
static PY_CONFIG: LazyLock<HighlightConfiguration> = LazyLock::new(|| {
    let mut c = HighlightConfiguration::new(
        unsafe { tree_sitter_python() },
        "python",
        include_str!("../queries/python/highlights.scm"),
        include_str!("../queries/python/injections.scm"),
        include_str!("../queries/python/locals.scm"),
    )
    .expect("Failed to load Python highlight config");
    c.configure(HIGHLIGHT_NAMES);
    c
});

static MD_BLOCK_CONFIG: LazyLock<HighlightConfiguration> = LazyLock::new(|| {
    let hl = remap_queries(tree_sitter_md::HIGHLIGHT_QUERY_BLOCK);
    let inj = tree_sitter_md::INJECTION_QUERY_BLOCK;
    let mut c =
        HighlightConfiguration::new(tree_sitter_md::LANGUAGE.into(), "markdown", &hl, inj, "")
            .expect("Failed to load Markdown block config");
    c.configure(HIGHLIGHT_NAMES);
    c
});

static MD_INLINE_CONFIG: LazyLock<HighlightConfiguration> = LazyLock::new(|| {
    let hl = remap_queries(tree_sitter_md::HIGHLIGHT_QUERY_INLINE);
    let inj = tree_sitter_md::INJECTION_QUERY_INLINE;
    let mut c = HighlightConfiguration::new(
        tree_sitter_md::INLINE_LANGUAGE.into(),
        "markdown_inline",
        &hl,
        inj,
        "",
    )
    .expect("Failed to load Markdown inline config");
    c.configure(HIGHLIGHT_NAMES);
    c
});

fn resolve_alias(lang: &str) -> &str {
    match lang {
        "yml" => "yaml",
        "zsh" => "bash",
        _ => lang,
    }
}

fn parse_lang(lang: &str) -> Result<Language, HighlightError> {
    let resolved = resolve_alias(lang);
    Language::from_token(resolved).ok_or_else(|| HighlightError::UnknownLanguage(lang.to_string()))
}

fn run_highlights<E>(
    events: impl Iterator<Item = Result<HighlightEvent, E>>,
) -> Result<Vec<Token>, HighlightError>
where
    E: std::fmt::Display,
{
    let mut toks = Vec::new();
    let mut stack: Vec<&str> = Vec::new();
    for event in events {
        let event = event.map_err(|e| HighlightError::Highlight(e.to_string()))?;
        match event {
            HighlightEvent::Source { start, end } => {
                if let Some(&kind) = stack.last() {
                    toks.push(Token {
                        start,
                        end,
                        kind: kind.to_string(),
                    });
                }
            }
            HighlightEvent::HighlightStart(idx) => {
                let name = HIGHLIGHT_NAMES.get(idx.0).copied().unwrap_or("unknown");
                stack.push(name);
            }
            HighlightEvent::HighlightEnd => {
                stack.pop();
            }
        }
    }
    Ok(toks)
}

pub fn tokenize(code: &str, lang: &str) -> Result<Vec<Token>, HighlightError> {
    let lang = resolve_alias(lang);
    #[cfg(any(feature = "all-languages", feature = "language-python"))]
    if lang == "python" || lang == "py" {
        let mut h = TSHighlighter::new();
        let events = h
            .highlight(&PY_CONFIG, code.as_bytes(), None, |token| {
                Language::from_token(token).map(|l| l.config())
            })
            .map_err(|e| HighlightError::Highlight(e.to_string()))?;
        return run_highlights(events);
    }
    if lang == "markdown" || lang == "md" {
        let mut h = TSHighlighter::new();
        let block_events = h
            .highlight(&MD_BLOCK_CONFIG, code.as_bytes(), None, |_| None)
            .map_err(|e| HighlightError::Highlight(e.to_string()))?;
        let mut toks = run_highlights(block_events)?;
        let mut parser = tree_sitter::Parser::new();
        let _ = parser.set_language(&tree_sitter_md::LANGUAGE.into());
        if let Some(tree) = parser.parse(code, None) {
            let mut cursor = tree.walk();
            loop {
                let node = cursor.node();
                if node.kind() == "inline" {
                    let start = node.start_byte();
                    let end = node.end_byte();
                    if start < end {
                        let slice = &code.as_bytes()[start..end];
                        let mut h2 = TSHighlighter::new();
                        let events = h2.highlight(&MD_INLINE_CONFIG, slice, None, |t| {
                            Language::from_token(t).map(|l| l.config())
                        });
                        if let Ok(evts) = events {
                            let collected: Vec<_> = evts.collect();
                            if let Ok(inline_toks) = run_highlights(collected.into_iter()) {
                                for tok in inline_toks {
                                    toks.push(Token {
                                        start: start + tok.start,
                                        end: start + tok.end,
                                        kind: tok.kind,
                                    });
                                }
                            }
                        }
                    }
                }
                if cursor.goto_first_child() {
                    continue;
                }
                while !cursor.goto_next_sibling() {
                    if !cursor.goto_parent() {
                        break;
                    }
                }
                if cursor.node() == tree.root_node() {
                    break;
                }
            }
        }
        toks.sort_by_key(|tok| tok.start);
        return Ok(toks);
    }
    let language = parse_lang(lang)?;
    let mut h = Highlighter::new();
    let source = code.to_string();
    let events = h
        .highlight_raw(language, &source)
        .map_err(|e| HighlightError::Highlight(e.to_string()))?;
    run_highlights(events)
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

#[cfg(feature = "python")]
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

#[cfg(feature = "python")]
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

const LANGUAGES: &[&str] = &[
    "ada",
    "asm",
    "astro",
    "awk",
    "bash",
    "bibtex",
    "bicep",
    "blueprint",
    "c",
    "capnp",
    "clojure",
    "c_sharp",
    "commonlisp",
    "cpp",
    "css",
    "cue",
    "d",
    "dart",
    "diff",
    "dockerfile",
    "eex",
    "elisp",
    "elixir",
    "elm",
    "erlang",
    "forth",
    "fortran",
    "gdscript",
    "gleam",
    "glsl",
    "go",
    "haskell",
    "hcl",
    "heex",
    "html",
    "iex",
    "ini",
    "java",
    "javascript",
    "json",
    "jsx",
    "kotlin",
    "latex",
    "llvm",
    "lua",
    "make",
    "markdown",
    "md",
    "matlab",
    "meson",
    "nim",
    "nix",
    "objc",
    "ocaml",
    "openscad",
    "pascal",
    "php",
    "plaintext",
    "proto",
    "python",
    "py",
    "r",
    "racket",
    "regex",
    "ruby",
    "rust",
    "scala",
    "scheme",
    "scss",
    "sql",
    "svelte",
    "swift",
    "toml",
    "typescript",
    "tsx",
    "vim",
    "wast",
    "wat",
    "x86asm",
    "wgsl",
    "yaml",
    "yml",
    "zsh",
    "zig",
];

pub fn languages() -> Vec<&'static str> {
    LANGUAGES
        .iter()
        .copied()
        .filter(|t| {
            matches!(*t, "markdown" | "md")
                || matches!(*t, "python" | "py")
                    && cfg!(any(feature = "all-languages", feature = "language-python"))
                || Language::from_token(resolve_alias(t)).is_some()
        })
        .collect()
}

#[cfg(feature = "themes")]
fn lookup_vendored(name: &str) -> Result<&'static str, HighlightError> {
    let data = match name {
        "acme" => vendored::ACME,
        "adwaita_dark" => vendored::ADWAITA_DARK,
        "amberwood" => vendored::AMBERWOOD,
        "ao" => vendored::AO,
        "ayu_dark" => vendored::AYU_DARK,
        "ayu_light" => vendored::AYU_LIGHT,
        "ayu_mirage" => vendored::AYU_MIRAGE,
        "base16_default_dark" => vendored::BASE16_DEFAULT_DARK,
        "base16_default_light" => vendored::BASE16_DEFAULT_LIGHT,
        "base16_terminal" => vendored::BASE16_TERMINAL,
        "base16_transparent" => vendored::BASE16_TRANSPARENT,
        "bogster" => vendored::BOGSTER,
        "bogster_light" => vendored::BOGSTER_LIGHT,
        "boo_berry" => vendored::BOO_BERRY,
        "catppuccin_mocha" => vendored::CATPPUCCIN_MOCHA,
        "curzon" => vendored::CURZON,
        "cyan_light" => vendored::CYAN_LIGHT,
        "darcula" => vendored::DARCULA,
        "dark_high_contrast" => vendored::DARK_HIGH_CONTRAST,
        "dark_plus" => vendored::DARK_PLUS,
        "doom_acario_dark" => vendored::DOOM_ACARIO_DARK,
        "dracula" => vendored::DRACULA,
        "dracula_at_night" => vendored::DRACULA_AT_NIGHT,
        "emacs" => vendored::EMACS,
        "everblush" => vendored::EVERBLUSH,
        "everforest_dark" => vendored::EVERFOREST_DARK,
        "everforest_light" => vendored::EVERFOREST_LIGHT,
        "ferra" => vendored::FERRA,
        "flatwhite" => vendored::FLATWHITE,
        "fleet_dark" => vendored::FLEET_DARK,
        "flexoki_light" => vendored::FLEXOKI_LIGHT,
        "github_dark" => vendored::GITHUB_DARK,
        "github_light" => vendored::GITHUB_LIGHT,
        "gruber_darker" => vendored::GRUBER_DARKER,
        "gruvbox" => vendored::GRUVBOX,
        "heisenberg" => vendored::HEISENBERG,
        "hex_steel" => vendored::HEX_STEEL,
        "horizon_dark" => vendored::HORIZON_DARK,
        "iceberg_dark" => vendored::ICEBERG_DARK,
        "ingrid" => vendored::INGRID,
        "iroaseta" => vendored::IROASETA,
        "jellybeans" => vendored::JELLYBEANS,
        "jetbrains_dark" => vendored::JETBRAINS_DARK,
        "kanagawa" => vendored::KANAGAWA,
        "kaolin_dark" => vendored::KAOLIN_DARK,
        "material_deep_ocean" => vendored::MATERIAL_DEEP_OCEAN,
        "meliora" => vendored::MELIORA,
        "mellow" => vendored::MELLOW,
        "merionette" => vendored::MERIONETTE,
        "modus_operandi" => vendored::MODUS_OPERANDI,
        "monokai" => vendored::MONOKAI,
        "monokai_pro" => vendored::MONOKAI_PRO,
        "monokai_pro_machine" => vendored::MONOKAI_PRO_MACHINE,
        "monokai_pro_octagon" => vendored::MONOKAI_PRO_OCTAGON,
        "monokai_pro_ristretto" => vendored::MONOKAI_PRO_RISTRETTO,
        "monokai_pro_spectrum" => vendored::MONOKAI_PRO_SPECTRUM,
        "monokai_soda" => vendored::MONOKAI_SODA,
        "naysayer" => vendored::NAYSAYER,
        "new_moon" => vendored::NEW_MOON,
        "nightfox" => vendored::NIGHTFOX,
        "night_owl" => vendored::NIGHT_OWL,
        "noctis" => vendored::NOCTIS,
        "noctis_bordo" => vendored::NOCTIS_BORDO,
        "nord" => vendored::NORD,
        "nord_light" => vendored::NORD_LIGHT,
        "onedark" => vendored::ONEDARK,
        "onedarker" => vendored::ONEDARKER,
        "onelight" => vendored::ONELIGHT,
        "papercolor_light" => vendored::PAPERCOLOR_LIGHT,
        "penumbra_plus" => vendored::PENUMBRA_PLUS,
        "poimandres" => vendored::POIMANDRES,
        "pop_dark" => vendored::POP_DARK,
        "rasmus" => vendored::RASMUS,
        "rose_pine" => vendored::ROSE_PINE,
        "serika_dark" => vendored::SERIKA_DARK,
        "serika_light" => vendored::SERIKA_LIGHT,
        "snazzy" => vendored::SNAZZY,
        "solarized_dark" => vendored::SOLARIZED_DARK,
        "solarized_light" => vendored::SOLARIZED_LIGHT,
        "sonokai" => vendored::SONOKAI,
        "spacebones_light" => vendored::SPACEBONES_LIGHT,
        "starlight" => vendored::STARLIGHT,
        "term16_dark" => vendored::TERM16_DARK,
        "tokyonight" => vendored::TOKYONIGHT,
        "ttox" => vendored::TTOX,
        "varua" => vendored::VARUA,
        "vim_dark_high_contrast" => vendored::VIM_DARK_HIGH_CONTRAST,
        "voxed" => vendored::VOXED,
        "yellowed" => vendored::YELLOWED,
        "zed_onedark" => vendored::ZED_ONEDARK,
        "zenburn" => vendored::ZENBURN,
        _ => return Err(HighlightError::Theme(format!("unknown theme: {name}"))),
    };
    Ok(data)
}

#[cfg(feature = "themes")]
fn style_to_css(style: &inkjet::theme::Style) -> String {
    let mut props = Vec::new();
    if let Some(fg) = &style.fg {
        props.push(format!("color: {}", fg.into_hex()));
    }
    if let Some(bg) = &style.bg {
        props.push(format!("background-color: {}", bg.into_hex()));
    }
    if style.modifiers.contains(&Modifier::Bold) {
        props.push("text-shadow: 0.3px 0 0 currentColor".into());
    }
    if style.modifiers.contains(&Modifier::Underlined) || style.underline.is_some() {
        props.push("text-decoration: underline".into());
    }
    if style.modifiers.contains(&Modifier::Strikethrough) {
        props.push("text-decoration: line-through".into());
    }
    props.join("; ")
}

#[cfg(feature = "themes")]
pub fn theme_css(
    theme: &str,
    selector: Option<&str>,
    class_prefix: &str,
) -> Result<String, HighlightError> {
    let data = lookup_vendored(theme)?;
    let t = Theme::from_helix(data).map_err(|e| HighlightError::Theme(e.to_string()))?;
    let mut out = String::with_capacity(4096);
    for name in HIGHLIGHT_NAMES {
        if let Some(style) = t.get_style(name) {
            let css = style_to_css(style);
            if !css.is_empty() {
                let hl_name = name.replace('.', "-");
                match selector {
                    Some(sel) => {
                        out.push_str(&format!("{sel} .{class_prefix}{hl_name} {{ {css} }}\n"))
                    }
                    None => out.push_str(&format!("::highlight({hl_name}) {{ {css} }}\n")),
                }
            }
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

const THEMES: &[&str] = &[
    "acme",
    "adwaita_dark",
    "amberwood",
    "ao",
    "ayu_dark",
    "ayu_light",
    "ayu_mirage",
    "base16_default_dark",
    "base16_default_light",
    "base16_terminal",
    "base16_transparent",
    "bogster",
    "bogster_light",
    "boo_berry",
    "catppuccin_mocha",
    "curzon",
    "cyan_light",
    "darcula",
    "dark_high_contrast",
    "dark_plus",
    "doom_acario_dark",
    "dracula",
    "dracula_at_night",
    "emacs",
    "everblush",
    "everforest_dark",
    "everforest_light",
    "ferra",
    "flatwhite",
    "fleet_dark",
    "flexoki_light",
    "github_dark",
    "github_light",
    "gruber_darker",
    "gruvbox",
    "heisenberg",
    "hex_steel",
    "horizon_dark",
    "iceberg_dark",
    "ingrid",
    "iroaseta",
    "jellybeans",
    "jetbrains_dark",
    "kanagawa",
    "kaolin_dark",
    "material_deep_ocean",
    "meliora",
    "mellow",
    "merionette",
    "modus_operandi",
    "monokai",
    "monokai_pro",
    "monokai_pro_machine",
    "monokai_pro_octagon",
    "monokai_pro_ristretto",
    "monokai_pro_spectrum",
    "monokai_soda",
    "naysayer",
    "new_moon",
    "nightfox",
    "night_owl",
    "noctis",
    "noctis_bordo",
    "nord",
    "nord_light",
    "onedark",
    "onedarker",
    "onelight",
    "papercolor_light",
    "penumbra_plus",
    "poimandres",
    "pop_dark",
    "rasmus",
    "rose_pine",
    "serika_dark",
    "serika_light",
    "snazzy",
    "solarized_dark",
    "solarized_light",
    "sonokai",
    "spacebones_light",
    "starlight",
    "term16_dark",
    "tokyonight",
    "ttox",
    "varua",
    "vim_dark_high_contrast",
    "voxed",
    "yellowed",
    "zed_onedark",
    "zenburn",
];

pub fn themes() -> &'static [&'static str] {
    THEMES
}

#[cfg(feature = "python")]
fn py_err(err: HighlightError) -> PyErr {
    PyValueError::new_err(err.to_string())
}

#[cfg(feature = "python")]
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

#[cfg(feature = "python")]
#[pyfunction(name = "highlight")]
fn py_highlight(code: &str, lang: &str) -> PyResult<String> {
    highlight_component(code, lang).map_err(py_err)
}

#[cfg(feature = "python")]
#[pyfunction(name = "highlight_spans")]
#[pyo3(signature = (code, lang, class_prefix=None))]
fn py_highlight_spans(code: &str, lang: &str, class_prefix: Option<&str>) -> PyResult<String> {
    highlight_spans(code, lang, class_prefix.unwrap_or("hl-")).map_err(py_err)
}

#[cfg(feature = "python")]
#[pyfunction(name = "languages")]
fn py_languages() -> Vec<&'static str> {
    languages()
}

#[cfg(feature = "python")]
#[pyfunction(name = "theme_css")]
#[pyo3(signature = (theme, selector=None, class_prefix=None))]
fn py_theme_css(
    theme: &str,
    selector: Option<&str>,
    class_prefix: Option<&str>,
) -> PyResult<String> {
    theme_css(theme, selector, class_prefix.unwrap_or("")).map_err(py_err)
}

#[cfg(feature = "python")]
#[pyfunction(name = "themes")]
fn py_themes() -> Vec<&'static str> {
    themes().to_vec()
}

#[cfg(feature = "python")]
#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(py_tokenize, m)?)?;
    m.add_function(wrap_pyfunction!(py_highlight, m)?)?;
    m.add_function(wrap_pyfunction!(py_highlight_spans, m)?)?;
    m.add_function(wrap_pyfunction!(py_languages, m)?)?;
    m.add_function(wrap_pyfunction!(py_theme_css, m)?)?;
    m.add_function(wrap_pyfunction!(py_themes, m)?)?;
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
        assert!(html.contains("<span class=\"hl-keyword-control-conditional\">if</span>"));
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
