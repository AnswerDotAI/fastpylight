use inkjet::{Highlighter, Language};
use inkjet::constants::HIGHLIGHT_NAMES;
use inkjet::theme::{Theme, vendored, Modifier};
use tree_sitter_highlight::{HighlightEvent, HighlightConfiguration, Highlighter as TSHighlighter};
use std::sync::LazyLock;
use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;

extern "C" { fn tree_sitter_python() -> tree_sitter::Language; }

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

static PY_CONFIG: LazyLock<HighlightConfiguration> = LazyLock::new(|| {
    let mut c = HighlightConfiguration::new(
        unsafe { tree_sitter_python() }, "python",
        include_str!("../queries/python/highlights.scm"),
        include_str!("../queries/python/injections.scm"),
        include_str!("../queries/python/locals.scm"),
    ).expect("Failed to load Python highlight config");
    c.configure(HIGHLIGHT_NAMES);
    c
});

static MD_BLOCK_CONFIG: LazyLock<HighlightConfiguration> = LazyLock::new(|| {
    let hl = remap_queries(tree_sitter_md::HIGHLIGHT_QUERY_BLOCK);
    let inj = tree_sitter_md::INJECTION_QUERY_BLOCK;
    let mut c = HighlightConfiguration::new(
        tree_sitter_md::LANGUAGE.into(), "markdown",
        &hl, inj, "",
    ).expect("Failed to load Markdown block config");
    c.configure(HIGHLIGHT_NAMES);
    c
});

static MD_INLINE_CONFIG: LazyLock<HighlightConfiguration> = LazyLock::new(|| {
    let hl = remap_queries(tree_sitter_md::HIGHLIGHT_QUERY_INLINE);
    let inj = tree_sitter_md::INJECTION_QUERY_INLINE;
    let mut c = HighlightConfiguration::new(
        tree_sitter_md::INLINE_LANGUAGE.into(), "markdown_inline",
        &hl, inj, "",
    ).expect("Failed to load Markdown inline config");
    c.configure(HIGHLIGHT_NAMES);
    c
});

fn parse_lang(lang: &str) -> PyResult<Language> {
    Language::from_token(lang)
        .ok_or_else(|| PyValueError::new_err(format!("Unknown language: {lang}")))
}

fn run_highlights<'a>(events: impl Iterator<Item = Result<HighlightEvent, tree_sitter_highlight::Error>>) -> PyResult<Vec<(usize, usize, String)>> {
    let mut toks: Vec<(usize, usize, String)> = Vec::new();
    let mut stack: Vec<&str> = Vec::new();
    for event in events {
        let event = event.map_err(|e| PyValueError::new_err(format!("Highlight error: {e}")))?;
        match event {
            HighlightEvent::Source { start, end } => {
                if let Some(&kind) = stack.last() { toks.push((start, end, kind.to_string())); }
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

#[pyfunction]
fn tokenize(code: &str, lang: &str) -> PyResult<Vec<(usize, usize, String)>> {
    if lang == "python" || lang == "py" {
        let mut h = TSHighlighter::new();
        let events = h.highlight(&PY_CONFIG, code.as_bytes(), None, |token| {
            match Language::from_token(token) { Some(l) => Some(l.config()), None => None }
        }).map_err(|e| PyValueError::new_err(format!("Highlight error: {e}")))?;
        return run_highlights(events);
    }
    if lang == "markdown" || lang == "md" {
        let mut h = TSHighlighter::new();
        let block_events = h.highlight(&MD_BLOCK_CONFIG, code.as_bytes(), None, |_| None)
            .map_err(|e| PyValueError::new_err(format!("Highlight error: {e}")))?;
        let mut toks = run_highlights(block_events)?;
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_md::LANGUAGE.into()).ok();
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
                                for (s, e, kind) in inline_toks { toks.push((start + s, start + e, kind)); }
                            }
                        }
                    }
                }
                if cursor.goto_first_child() { continue; }
                while !cursor.goto_next_sibling() {
                    if !cursor.goto_parent() { break; }
                }
                if cursor.node() == tree.root_node() { break; }
            }
        }
        toks.sort_by_key(|(s, _, _)| *s);
        return Ok(toks);
    }
    let language = parse_lang(lang)?;
    let mut h = Highlighter::new();
    let source = code.to_string();
    let events = h.highlight_raw(language, &source)
        .map_err(|e| PyValueError::new_err(format!("Highlight error: {e}")))?;
    let mut toks: Vec<(usize, usize, String)> = Vec::new();
    let mut stack: Vec<&str> = Vec::new();
    for event in events {
        let event = event.map_err(|e| PyValueError::new_err(format!("Highlight error: {e}")))?;
        match event {
            HighlightEvent::Source { start, end } => {
                if let Some(&kind) = stack.last() { toks.push((start, end, kind.to_string())); }
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

#[pyfunction]
fn highlight(code: &str, lang: &str) -> PyResult<String> {
    let toks = tokenize(code, lang)?;
    let b2c = byte_to_utf16_table(code);
    let mut toks_json = String::from("[");
    for (i, (start, end, ref kind)) in toks.iter().enumerate() {
        if i > 0 { toks_json.push(','); }
        let cs = b2c[*start];
        let ce = b2c[*end];
        toks_json.push_str(&format!("[{},{},\"{}\"]", cs, ce, kind.replace('.', "-")));
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
    let all = vec![
        "ada", "asm", "astro", "awk", "bash", "bibtex", "bicep", "blueprint", "c", "capnp",
        "clojure", "c_sharp", "commonlisp", "cpp", "css", "cue", "d", "dart", "diff",
        "dockerfile", "eex", "elisp", "elixir", "elm", "erlang", "forth", "fortran", "gdscript",
        "gleam", "glsl", "go", "haskell", "hcl", "heex", "html", "iex", "ini", "java",
        "javascript", "json", "jsx", "kotlin", "latex", "llvm", "lua", "make", "markdown", "md",
        "matlab", "meson", "nim", "nix", "objc", "ocaml", "openscad", "pascal", "php",
        "plaintext", "proto", "python", "r", "racket", "regex", "ruby", "rust", "scala",
        "scheme", "scss", "sql", "svelte", "swift", "toml", "typescript", "tsx", "vim", "wast",
        "wat", "x86asm", "wgsl", "yaml", "zig",
    ];
    all.into_iter().filter(|t| matches!(*t, "python" | "py" | "markdown" | "md") || Language::from_token(t).is_some()).collect()
}

fn lookup_vendored(name: &str) -> PyResult<&'static str> {
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
        _ => return Err(PyValueError::new_err(format!("Unknown theme: {name}"))),
    };
    Ok(data)
}

fn style_to_css(style: &inkjet::theme::Style) -> String {
    let mut props = Vec::new();
    if let Some(fg) = &style.fg { props.push(format!("color: {}", fg.into_hex())); }
    if let Some(bg) = &style.bg { props.push(format!("background-color: {}", bg.into_hex())); }
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

#[pyfunction]
#[pyo3(signature = (theme, selector=None))]
fn theme_css(theme: &str, selector: Option<&str>) -> PyResult<String> {
    let data = lookup_vendored(theme)?;
    let t = Theme::from_helix(data)
        .map_err(|e| PyValueError::new_err(format!("Theme parse error: {e}")))?;

    let mut out = String::with_capacity(4096);
    for name in HIGHLIGHT_NAMES {
        if let Some(style) = t.get_style(name) {
            let css = style_to_css(style);
            if !css.is_empty() {
                let hl_name = name.replace('.', "-");
                match selector {
                    Some(sel) => out.push_str(&format!("{sel} .{hl_name} {{ {css} }}\n")),
                    None => out.push_str(&format!("::highlight({hl_name}) {{ {css} }}\n")),
                }
            }
        }
    }
    Ok(out)
}

#[pyfunction]
fn themes() -> Vec<&'static str> {
    vec![
        "acme", "adwaita_dark", "amberwood", "ao",
        "ayu_dark", "ayu_light", "ayu_mirage",
        "base16_default_dark", "base16_default_light", "base16_terminal", "base16_transparent",
        "bogster", "bogster_light", "boo_berry",
        "catppuccin_mocha", "curzon", "cyan_light",
        "darcula", "dark_high_contrast", "dark_plus", "doom_acario_dark",
        "dracula", "dracula_at_night",
        "emacs", "everblush", "everforest_dark", "everforest_light",
        "ferra", "flatwhite", "fleet_dark", "flexoki_light",
        "github_dark", "github_light", "gruber_darker", "gruvbox",
        "heisenberg", "hex_steel", "horizon_dark",
        "iceberg_dark", "ingrid", "iroaseta",
        "jellybeans", "jetbrains_dark",
        "kanagawa", "kaolin_dark",
        "material_deep_ocean", "meliora", "mellow", "merionette", "modus_operandi",
        "monokai", "monokai_pro", "monokai_pro_machine", "monokai_pro_octagon",
        "monokai_pro_ristretto", "monokai_pro_spectrum", "monokai_soda",
        "naysayer", "new_moon", "nightfox", "night_owl", "noctis", "noctis_bordo",
        "nord", "nord_light",
        "onedark", "onedarker", "onelight",
        "papercolor_light", "penumbra_plus", "poimandres", "pop_dark",
        "rasmus", "rose_pine",
        "serika_dark", "serika_light", "snazzy",
        "solarized_dark", "solarized_light", "sonokai", "spacebones_light", "starlight",
        "term16_dark", "tokyonight", "ttox",
        "varua", "vim_dark_high_contrast", "voxed",
        "yellowed", "zed_onedark", "zenburn",
    ]
}

#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(tokenize, m)?)?;
    m.add_function(wrap_pyfunction!(highlight, m)?)?;
    m.add_function(wrap_pyfunction!(highlight_spans, m)?)?;
    m.add_function(wrap_pyfunction!(languages, m)?)?;
    m.add_function(wrap_pyfunction!(theme_css, m)?)?;
    m.add_function(wrap_pyfunction!(themes, m)?)?;
    Ok(())
}