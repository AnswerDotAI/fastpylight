from fastpylight._core import __version__, tokenize, highlight, highlight_spans, languages, guess, theme_css, theme_colors, themes


def component_js():
    "The reference `<hl-code>` custom element: registers `toks` ranges with the CSS Highlight API on connect, and removes them on disconnect."
    return r"""class HlCode extends HTMLElement {
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
if(!customElements.get('hl-code')) customElements.define('hl-code',HlCode);"""
