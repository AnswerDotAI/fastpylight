//! pyo3 bindings for the `fastpylight` Python package.

use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyDict;

use std::panic::{AssertUnwindSafe, catch_unwind};

use crate::{
    HighlightError, guess, highlight_component, highlight_spans, languages, theme_colors,
    theme_css, themes, tokenize,
};

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
