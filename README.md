> [!WARNING]
> This library is still a WIP - client-side setup, theming etc are very manual at this stage

## Installation

```sh
pip install fastpylight
```

## Usage

> [!NOTE]
> Applying the highlights requires a web component implementation & styling, which need to be manually added to the server HTML

### Python:

```py
from fastpylight import highlight

code_block = '''
import time

def my_func():
    print(f'Hello World! {1 * 1}')
    time.sleep(4)

if '__name__' == '__main__':
    my_func()
'''

# `highlight` returns the HTML with token->range mappings set as a `toks` attribute on the web component

hl_html = highlight(code=code_block, lang='python')

```

### Example Javascript:

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

### Example CSS:

```css
::highlight(attribute) { color: light-dark(oklch(0.45 0.16 260), oklch(0.7 0.13 235)); }
::highlight(type) { color: light-dark(oklch(0.45 0.13 55), oklch(0.73 0.12 65)); }
::highlight(type-builtin) { color: light-dark(oklch(0.45 0.13 55), oklch(0.73 0.12 65)); }
::highlight(type-enum) { color: light-dark(oklch(0.45 0.13 55), oklch(0.73 0.12 65)); }
::highlight(type-enum-variant) { color: light-dark(oklch(0.45 0.13 55), oklch(0.73 0.12 65)); }
::highlight(constructor) { color: light-dark(oklch(0.45 0.13 55), oklch(0.73 0.12 65)); }
::highlight(constant) { color: light-dark(oklch(0.45 0.16 260), oklch(0.7 0.13 235)); }
::highlight(constant-builtin) { color: light-dark(oklch(0.45 0.16 260), oklch(0.7 0.13 235)); text-shadow: 0.3px 0 0 currentColor; }
::highlight(constant-builtin-boolean) { color: light-dark(oklch(0.45 0.16 260), oklch(0.7 0.13 235)); text-shadow: 0.3px 0 0 currentColor; }
::highlight(constant-character) { color: light-dark(oklch(0.35 0.12 260), oklch(0.72 0.1 220)); }
::highlight(constant-character-escape) { color: light-dark(oklch(0.55 0.2 25), oklch(0.72 0.16 20)); }
::highlight(constant-numeric) { color: light-dark(oklch(0.45 0.16 260), oklch(0.7 0.13 235)); }
::highlight(constant-numeric-integer) { color: light-dark(oklch(0.45 0.16 260), oklch(0.7 0.13 235)); }
::highlight(constant-numeric-float) { color: light-dark(oklch(0.45 0.16 260), oklch(0.7 0.13 235)); }
::highlight(string) { color: light-dark(oklch(0.35 0.12 260), oklch(0.72 0.1 220)); }
::highlight(string-regexp) { color: light-dark(oklch(0.35 0.12 260), oklch(0.72 0.1 220)); }
::highlight(string-special) { color: light-dark(oklch(0.35 0.12 260), oklch(0.72 0.1 220)); }
::highlight(string-special-path) { color: light-dark(oklch(0.35 0.12 260), oklch(0.72 0.1 220)); }
::highlight(string-special-url) { color: light-dark(oklch(0.45 0.16 260), oklch(0.7 0.13 235)); text-decoration: underline; }
::highlight(string-special-symbol) { color: light-dark(oklch(0.35 0.12 260), oklch(0.72 0.1 220)); }
::highlight(escape) { color: light-dark(oklch(0.55 0.2 25), oklch(0.72 0.16 20)); }
::highlight(comment) { color: light-dark(oklch(0.55 0.02 250), oklch(0.58 0.02 250)); }
::highlight(comment-line) { color: light-dark(oklch(0.55 0.02 250), oklch(0.58 0.02 250)); }
::highlight(comment-block) { color: light-dark(oklch(0.55 0.02 250), oklch(0.58 0.02 250)); }
::highlight(comment-block-documentation) { color: light-dark(oklch(0.55 0.02 250), oklch(0.58 0.02 250)); }
::highlight(variable) { color: light-dark(oklch(0.25 0.02 250), oklch(0.85 0.02 250)); }
::highlight(variable-builtin) { color: light-dark(oklch(0.6 0.18 55), oklch(0.73 0.14 60)); }
::highlight(variable-parameter) { color: light-dark(oklch(0.45 0.13 55), oklch(0.73 0.12 65)); }
::highlight(variable-other) { color: light-dark(oklch(0.25 0.02 250), oklch(0.85 0.02 250)); }
::highlight(variable-other-member) { color: light-dark(oklch(0.45 0.16 260), oklch(0.7 0.13 235)); }
::highlight(label) { color: light-dark(oklch(0.45 0.16 260), oklch(0.7 0.13 235)); }
::highlight(punctuation) { color: light-dark(oklch(0.48 0.02 250), oklch(0.65 0.02 250)); }
::highlight(punctuation-delimiter) { color: light-dark(oklch(0.48 0.02 250), oklch(0.65 0.02 250)); }
::highlight(punctuation-bracket) { color: light-dark(oklch(0.48 0.02 250), oklch(0.65 0.02 250)); }
::highlight(punctuation-special) { color: light-dark(oklch(0.55 0.2 25), oklch(0.72 0.16 20)); }
::highlight(keyword) { color: light-dark(oklch(0.55 0.2 25), oklch(0.72 0.16 20)); text-shadow: 0.3px 0 0 currentColor; }
::highlight(keyword-control) { color: light-dark(oklch(0.55 0.2 25), oklch(0.72 0.16 20)); text-shadow: 0.3px 0 0 currentColor; }
::highlight(keyword-control-conditional) { color: light-dark(oklch(0.55 0.2 25), oklch(0.72 0.16 20)); text-shadow: 0.3px 0 0 currentColor; }
::highlight(keyword-control-repeat) { color: light-dark(oklch(0.55 0.2 25), oklch(0.72 0.16 20)); text-shadow: 0.3px 0 0 currentColor; }
::highlight(keyword-control-import) { color: light-dark(oklch(0.55 0.2 25), oklch(0.72 0.16 20)); text-shadow: 0.3px 0 0 currentColor; }
::highlight(keyword-control-return) { color: light-dark(oklch(0.55 0.2 25), oklch(0.72 0.16 20)); text-shadow: 0.3px 0 0 currentColor; }
::highlight(keyword-control-exception) { color: light-dark(oklch(0.55 0.2 25), oklch(0.72 0.16 20)); text-shadow: 0.3px 0 0 currentColor; }
::highlight(keyword-operator) { color: light-dark(oklch(0.55 0.2 25), oklch(0.72 0.16 20)); text-shadow: 0.3px 0 0 currentColor; }
::highlight(keyword-directive) { color: light-dark(oklch(0.55 0.2 25), oklch(0.72 0.16 20)); text-shadow: 0.3px 0 0 currentColor; }
::highlight(keyword-function) { color: light-dark(oklch(0.55 0.2 25), oklch(0.72 0.16 20)); text-shadow: 0.3px 0 0 currentColor; }
::highlight(keyword-storage) { color: light-dark(oklch(0.55 0.2 25), oklch(0.72 0.16 20)); text-shadow: 0.3px 0 0 currentColor; }
::highlight(keyword-storage-type) { color: light-dark(oklch(0.55 0.2 25), oklch(0.72 0.16 20)); text-shadow: 0.3px 0 0 currentColor; }
::highlight(keyword-storage-modifier) { color: light-dark(oklch(0.55 0.2 25), oklch(0.72 0.16 20)); text-shadow: 0.3px 0 0 currentColor; }
::highlight(operator) { color: light-dark(oklch(0.45 0.16 260), oklch(0.7 0.13 235)); }
::highlight(function) { color: light-dark(oklch(0.5 0.2 290), oklch(0.72 0.15 290)); }
::highlight(function-builtin) { color: light-dark(oklch(0.5 0.2 290), oklch(0.72 0.15 290)); text-shadow: 0.3px 0 0 currentColor; }
::highlight(function-method) { color: light-dark(oklch(0.5 0.2 290), oklch(0.72 0.15 290)); }
::highlight(function-macro) { color: light-dark(oklch(0.5 0.2 290), oklch(0.72 0.15 290)); }
::highlight(function-special) { color: light-dark(oklch(0.5 0.2 290), oklch(0.72 0.15 290)); }
::highlight(tag) { color: light-dark(oklch(0.43 0.13 150), oklch(0.7 0.12 150)); }
::highlight(tag-builtin) { color: light-dark(oklch(0.43 0.13 150), oklch(0.7 0.12 150)); }
::highlight(namespace) { color: light-dark(oklch(0.45 0.13 55), oklch(0.73 0.12 65)); }
::highlight(special) { color: light-dark(oklch(0.55 0.2 25), oklch(0.72 0.16 20)); }
::highlight(markup-heading) { color: light-dark(oklch(0.45 0.16 260), oklch(0.7 0.13 235)); text-shadow: 0.3px 0 0 currentColor; }
::highlight(markup-heading-marker) { color: light-dark(oklch(0.45 0.16 260), oklch(0.7 0.13 235)); }
::highlight(markup-heading-1) { color: light-dark(oklch(0.45 0.16 260), oklch(0.7 0.13 235)); text-shadow: 0.3px 0 0 currentColor; }
::highlight(markup-heading-2) { color: light-dark(oklch(0.45 0.16 260), oklch(0.7 0.13 235)); text-shadow: 0.3px 0 0 currentColor; }
::highlight(markup-heading-3) { color: light-dark(oklch(0.45 0.16 260), oklch(0.7 0.13 235)); text-shadow: 0.3px 0 0 currentColor; }
::highlight(markup-heading-4) { color: light-dark(oklch(0.45 0.16 260), oklch(0.7 0.13 235)); text-shadow: 0.3px 0 0 currentColor; }
::highlight(markup-heading-5) { color: light-dark(oklch(0.45 0.16 260), oklch(0.7 0.13 235)); text-shadow: 0.3px 0 0 currentColor; }
::highlight(markup-heading-6) { color: light-dark(oklch(0.45 0.16 260), oklch(0.7 0.13 235)); text-shadow: 0.3px 0 0 currentColor; }
::highlight(markup-list) { color: light-dark(oklch(0.55 0.2 25), oklch(0.72 0.16 20)); }
::highlight(markup-list-unnumbered) { color: light-dark(oklch(0.55 0.2 25), oklch(0.72 0.16 20)); }
::highlight(markup-list-numbered) { color: light-dark(oklch(0.55 0.2 25), oklch(0.72 0.16 20)); }
::highlight(markup-list-checked) { color: light-dark(oklch(0.52 0.15 150), oklch(0.7 0.12 150)); }
::highlight(markup-list-unchecked) { color: light-dark(oklch(0.48 0.02 250), oklch(0.65 0.02 250)); }
::highlight(markup-bold) { text-shadow: 0.3px 0 0 currentColor; }
::highlight(markup-italic) { color: light-dark(oklch(0.45 0.13 55), oklch(0.73 0.12 65)); }
::highlight(markup-strikethrough) { text-decoration: line-through; }
::highlight(markup-link) { color: light-dark(oklch(0.45 0.16 260), oklch(0.7 0.13 235)); text-decoration: underline; }
::highlight(markup-link-url) { color: light-dark(oklch(0.45 0.16 260), oklch(0.7 0.13 235)); text-decoration: underline; }
::highlight(markup-link-label) { color: light-dark(oklch(0.45 0.16 260), oklch(0.7 0.13 235)); }
::highlight(markup-link-text) { color: light-dark(oklch(0.45 0.16 260), oklch(0.7 0.13 235)); text-decoration: underline; }
::highlight(markup-quote) { color: light-dark(oklch(0.48 0.02 250), oklch(0.65 0.02 250)); }
::highlight(markup-raw) { color: light-dark(oklch(0.35 0.12 260), oklch(0.72 0.1 220)); background-color: light-dark(oklch(0.97 0 0 / 0.5), oklch(0.2 0 0 / 0.5)); }
::highlight(markup-raw-inline) { color: light-dark(oklch(0.35 0.12 260), oklch(0.72 0.1 220)); background-color: light-dark(oklch(0.97 0 0 / 0.5), oklch(0.2 0 0 / 0.5)); }
::highlight(markup-raw-block) { color: light-dark(oklch(0.35 0.12 260), oklch(0.72 0.1 220)); background-color: light-dark(oklch(0.97 0 0 / 0.5), oklch(0.2 0 0 / 0.5)); }
::highlight(diff-plus) { color: light-dark(oklch(0.52 0.15 150), oklch(0.7 0.12 150)); background-color: light-dark(oklch(0.95 0.05 145), oklch(0.3 0.05 145)); }
::highlight(diff-minus) { color: light-dark(oklch(0.55 0.2 25), oklch(0.72 0.16 20)); background-color: light-dark(oklch(0.95 0.05 25), oklch(0.3 0.05 25)); }
::highlight(diff-delta) { color: light-dark(oklch(0.45 0.13 55), oklch(0.73 0.12 65)); background-color: light-dark(oklch(0.95 0.05 85), oklch(0.3 0.05 85)); }
::highlight(diff-delta-moved) { color: light-dark(oklch(0.5 0.2 290), oklch(0.72 0.15 290)); background-color: light-dark(oklch(0.95 0.03 300), oklch(0.3 0.03 300)); }
hl-code > pre { color-scheme: light dark; background: oklch(0.7 0 0 / 0.1); padding: 0.75em; border-radius: 6px; white-space: pre-wrap; font-family: ui-monospace, monospace; }
```
