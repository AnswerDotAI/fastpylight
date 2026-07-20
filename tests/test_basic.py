import pytest

from fastpylight import guess, highlight, highlight_spans, languages, theme_colors, theme_css, themes, tokenize

def test_tokenize():
    toks = tokenize("def foo(): return 42", "python")
    assert len(toks) > 0
    starts = [t[0] for t in toks]
    assert starts == sorted(starts)

def test_languages():
    langs = languages()
    assert "python" in langs
    assert "javascript" in langs
    assert "rust" in langs

def test_highlight():
    html = highlight("let x = 1;", "javascript")
    assert "<pre>" in html

def test_highlight_spans():
    html = highlight_spans('if x < 1: return "&"', "python")
    assert html.startswith("<pre><code>")
    assert 'class="hl-keyword' in html
    assert "&lt;" in html
    assert "&quot;&amp;&quot;" in html

def test_theme_css_class_prefix():
    css = theme_css("github_light", "pre code", "hl-")
    assert "pre code .hl-" in css

def test_theme_css_highlight_selectors():
    css = theme_css("github_light")
    assert "::highlight(" in css

def test_unknown_language_raises():
    with pytest.raises(ValueError): tokenize("x", "not-a-language")

def test_non_ascii_is_safe():
    assert "é" in highlight_spans('s = "é"\n', "python")

def test_themes():
    ts = themes()
    assert "github_light" in ts
    assert ts == sorted(ts)

def test_theme_colors():
    tc = theme_colors("github_light")
    assert "normal" in tc and tc["normal"]["fg"]          # theme_css skips normal; the data API must not
    kw = tc["keyword"]
    assert kw["fg"].startswith("#")
    assert set(kw) == {"fg", "bg", "bold", "italic", "underline", "strikethrough"}
    assert "attribute.builtin" in tc                      # dotted scopes, not CSS-hyphenated
    assert tc["comment.error"]["bg"].startswith("#")

def test_plaintext_is_unhighlighted():
    # 'plaintext' must select the PlainText lexer, not the diff lexer:
    # leading -/+ lines should NOT be tokenized.
    assert tokenize("- milk\n+ eggs\n", "plaintext") == []
    assert "toks='[]'" in highlight("- milk\n+ eggs\n", "plaintext")

def test_guess():
    assert guess("anything", "python") == "python"   # explicit hint resolves
    assert guess("just some prose here") == "plaintext"  # no match -> fallback

def test_markdown_specimen():
    "Markdown is quotation: embedded fence bodies flatten to markup.raw.block; markdown's own structure keeps its scopes"
    md = '# Head\n\nSome `inline` code:\n\n``` rust\nfn main() {}\n```\n'
    toks = tokenize(md, "markdown")
    kinds = {k for _, _, k in toks}
    assert not any(k.startswith(("keyword", "function", "punctuation")) for k in kinds), kinds
    assert "markup.heading.1" in kinds
    assert "markup.raw" in kinds       # the `inline` span
    body = md.index("fn main")
    assert any(s <= body and e >= body + len("fn main() {}") and k == "markup.raw.block"
               for s, e, k in toks)    # fence body is one flat raw run
