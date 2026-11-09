// A very small stand-in for a web page's DOM, enough for lib/keyfind.js: elements with children, text nodes, the
// value of inputs, parentElement, textContent and querySelectorAll("*").
"use strict";

class Text {
  constructor(text) {
    this.nodeType = 3;
    this.nodeValue = text;
    this.parentElement = null;
  }
  get textContent() {
    return this.nodeValue;
  }
}

class Element {
  constructor(tag, attrs, kids) {
    this.nodeType = 1;
    this.tagName = tag.toUpperCase();
    this.parentElement = null;
    this.childNodes = [];
    this.value = attrs && "value" in attrs ? attrs.value : undefined;
    for (const k of kids || []) this.append(k);
  }
  append(k) {
    const n = typeof k === "string" ? new Text(k) : k;
    n.parentElement = this;
    this.childNodes.push(n);
  }
  get children() {
    return this.childNodes.filter((n) => n.nodeType === 1);
  }
  get textContent() {
    return this.childNodes.map((n) => n.textContent).join("");
  }
  querySelectorAll(sel) {
    if (sel !== "*") throw new Error("the fake page only knows *");
    const out = [];
    const walk = (e) => {
      for (const c of e.children) {
        out.push(c);
        walk(c);
      }
    };
    walk(this);
    return out;
  }
}

// h("div", {value}, ...children) or h("div", ...children)
function h(tag, ...rest) {
  let attrs = {};
  if (rest.length && rest[0] && typeof rest[0] === "object" && !(rest[0] instanceof Element)) attrs = rest.shift();
  return new Element(tag, attrs, rest);
}

// A page: its body is the root that querySelectorAll walks.
function page(...body) {
  const html = h("html", h("body", ...body));
  return { documentElement: html, querySelectorAll: (s) => [html, ...html.querySelectorAll(s)] };
}

module.exports = { h, page };
