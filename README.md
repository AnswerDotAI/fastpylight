## Installation

```sh
pip install fastpylight
```

`fastpylight` is a small Python/Rust wrapper around [Lumis](https://github.com/leandrocp/lumis), a Tree-sitter syntax highlighter with Neovim themes. Language names, token scopes, and theme names follow Lumis.

## Python

```py
from fastpylight import highlight, highlight_spans, theme_css

code = "def f(x):\n    return x + 1\n"

# Output for the CSS Highlight API.
html = highlight(code, "python")

# Static span output.
spans = highlight_spans(code, "python", "hl-")

# CSS rules for the span output.
css = theme_css("github_light", "pre code", "hl-")
```

`highlight` returns HTML with token ranges in UTF-16 code units, for use with the CSS Highlight API:

```html
<hl-code toks='[[0,3,"keyword"],[4,5,"function"],[6,7,"variable"],[14,20,"keyword"],[21,22,"variable"],[23,24,"operator"],[25,26,"number"]]'><pre><code>def f(x):
    return x + 1
</code></pre></hl-code>
```

`highlight_spans` returns normal HTML spans:

```html
<pre><code><span class="hl-keyword">def</span> <span class="hl-function">f</span>(<span class="hl-variable">x</span>):
    <span class="hl-keyword">return</span> <span class="hl-variable">x</span> <span class="hl-operator">+</span> <span class="hl-number">1</span>
</code></pre>
```

The exact classes depend on Lumis scopes. A complete GitHub Light CSS example for both output modes is in [docs/github_light.css](docs/github_light.css).

## Rust

`fastpylight` can also be used as a Rust library without PyO3:

```toml
[dependencies]
fastpylight = { path = "../fastpylight", default-features = false, features = ["standard-languages"] }
```

Use `write_highlighted_inner` when the caller already owns the surrounding `<pre><code>` block:

```rust
let mut out = String::new();
fastpylight::write_highlighted_inner("def f(): return 1", "python", "hl-", &mut out)?;
```

The default language set enables Lumis web, web-extra, system, and backend bundles, plus R, Julia, PowerShell, Lua, Swift, MATLAB, Perl, Pascal, Fortran, and Objective-C. Use the `all-languages` feature if you want Lumis' full language set.

## CSS Highlight API

The `highlight` function is for browser code that applies token ranges with the CSS Highlight API. A minimal component looks like this:

```js
class HlCode extends HTMLElement {
  connectedCallback(){
    if(!CSS.highlights) return;
    setTimeout(() => {
      const d=this.getAttribute('toks'); if(!d) return;
      const tn=this.querySelector('code').firstChild, toks=JSON.parse(d);
      this._ranges=[];
      toks.forEach(([s,e,k])=>{
        const r=new Range(); r.setStart(tn,s); r.setEnd(tn,e);
        const h=CSS.highlights.get(k)||new Highlight();
        h.add(r); CSS.highlights.set(k,h);
        this._ranges.push([r,k]);
      });
      this.removeAttribute('toks');
    }, 0);
  }
  disconnectedCallback(){
    if(!this._ranges) return;
    this._ranges.forEach(([r,k])=>{
      const h=CSS.highlights.get(k); if(h) h.delete(r);
    });
    this._ranges=null;
  }
}
if(!customElements.get('hl-code')) customElements.define('hl-code',HlCode);
```

`highlight` needs `::highlight(...)` rules. `highlight_spans` needs class rules. [docs/github_light.css](docs/github_light.css) includes both for the default `hl-` prefix.

You can generate CSS at runtime:

```py
highlight_css = theme_css("github_light")
span_css = theme_css("github_light", "pre code", "hl-")
```
