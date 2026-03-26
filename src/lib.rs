use inkjet::{Highlighter, Language};
use inkjet::tree_sitter_highlight::HighlightEvent;
use inkjet::constants::HIGHLIGHT_NAMES;
use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;

fn parse_lang(lang: &str) -> PyResult<Language> {
    Language::from_token(lang)
        .ok_or_else(|| PyValueError::new_err(format!("Unknown language: {lang}")))
}

#[pyfunction]
fn tokenize(code: &str, lang: &str) -> PyResult<Vec<(usize, usize, String)>> {
    let language = parse_lang(lang)?;
    let mut h = Highlighter::new();
    let mut toks: Vec<(usize, usize, String)> = Vec::new();
    let mut stack: Vec<&str> = Vec::new();
    let source = code.to_string();
    let events = h.highlight_raw(language, &source)
        .map_err(|e| PyValueError::new_err(format!("Highlight error: {e}")))?;
    for event in events {
        let event = event.map_err(|e| PyValueError::new_err(format!("Highlight error: {e}")))?;
        match event {
            HighlightEvent::Source { start, end } => {
                if let Some(&kind) = stack.last() {
                    toks.push((start, end, kind.to_string()));
                }
            }
            HighlightEvent::HighlightStart(idx) => {
                let name = HIGHLIGHT_NAMES.get(idx.0).copied().unwrap_or("unknown");
                stack.push(name);
            }
            HighlightEvent::HighlightEnd => { stack.pop(); }
        }
    }
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

#[pyfunction]
fn highlight(code: &str, lang: &str) -> PyResult<String> {
    let toks = tokenize(code, lang)?;
    let mut toks_json = String::from("[");
    for (i, (start, end, ref kind)) in toks.iter().enumerate() {
        if i > 0 { toks_json.push(','); }
        toks_json.push_str(&format!("[{},{},\"{}\"]", start, end, kind.replace('.', "-")));
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

#[pyfunction]
#[pyo3(signature = (code, lang, class_prefix=None))]
fn highlight_spans(code: &str, lang: &str, class_prefix: Option<&str>) -> PyResult<String> {
    let pfx = class_prefix.unwrap_or("hl-");
    let toks = tokenize(code, lang)?;
    let mut out = String::with_capacity(code.len() * 2);
    out.push_str("<pre><code>");
    let mut pos = 0usize;
    for (start, end, ref kind) in &toks {
        if pos < *start { html_escape(&code[pos..*start], &mut out); }
        out.push_str(&format!("<span class=\"{pfx}{}\">", kind.replace('.', "-")));
        html_escape(&code[*start..*end], &mut out);
        out.push_str("</span>");
        pos = *end;
    }
    if pos < code.len() { html_escape(&code[pos..], &mut out); }
    out.push_str("</code></pre>");
    Ok(out)
}

#[pyfunction]
fn languages() -> Vec<&'static str> {
    vec![
        "ada", "asm", "astro", "awk", "bash", "bibtex", "bicep", "blueprint", "c", "capnp",
        "clojure", "c_sharp", "commonlisp", "cpp", "css", "cue", "d", "dart", "diff",
        "dockerfile", "eex", "elisp", "elixir", "elm", "erlang", "forth", "fortran", "gdscript",
        "gleam", "glsl", "go", "haskell", "hcl", "heex", "html", "iex", "ini", "java",
        "javascript", "json", "jsx", "kotlin", "latex", "llvm", "lua", "make", "markdown", "md",
        "matlab", "meson",
        "nim", "nix", "objc", "ocaml", "openscad", "pascal", "php", "plaintext", "proto",
        "python", "r", "racket", "regex", "ruby", "rust", "scala", "scheme", "scss", "sql",
        "svelte", "swift", "toml", "typescript", "tsx", "vim", "wast", "wat", "x86asm", "wgsl",
        "yaml", "zig",
    ]
}

#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(tokenize, m)?)?;
    m.add_function(wrap_pyfunction!(highlight, m)?)?;
    m.add_function(wrap_pyfunction!(highlight_spans, m)?)?;
    m.add_function(wrap_pyfunction!(languages, m)?)?;
    Ok(())
}
