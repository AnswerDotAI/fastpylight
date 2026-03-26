from fastpylight import tokenize, highlight, languages

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
