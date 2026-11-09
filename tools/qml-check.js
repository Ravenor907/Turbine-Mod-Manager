#!/usr/bin/env node
// A quick syntax check of Turbine's QML files with nothing to install (Node only), so a typo fails in seconds rather
// than 25 minutes into the Flatpak build. It reads each file's object tree (objects, ids, properties, signals,
// functions, inline components) and compiles every JavaScript binding, handler and function with Node's own parser.
// It doesn't know types or imports: the Flatpak build's QML checks still do that.
//
//   node tools/qml-check.js app/qml/*.qml
"use strict";

const fs = require("fs");
const vm = require("vm");

function checkFile(file) {
  const src = fs.readFileSync(file, "utf8");
  const errors = [];
  let i = 0;
  const n = src.length;
  const lineAt = (pos) => src.slice(0, pos).split("\n").length;
  const fail = (pos, msg) => errors.push(file + ":" + lineAt(pos) + ": " + msg);
  // Ids must be unique in a file (an inline component has its own).
  const idScopes = [new Set()];

  function skipSpace(stopAtNewline) {
    for (;;) {
      while (i < n && (src[i] === " " || src[i] === "\t" || src[i] === "\r" || (!stopAtNewline && src[i] === "\n") || src[i] === ";")) i++;
      if (src.startsWith("//", i)) { while (i < n && src[i] !== "\n") i++; continue; }
      if (src.startsWith("/*", i)) { const e = src.indexOf("*/", i + 2); i = e < 0 ? n : e + 2; continue; }
      return;
    }
  }
  function ident() {
    const m = /^[A-Za-z_$][\w$]*(\.[A-Za-z_$][\w$]*)*/.exec(src.slice(i, i + 200));
    if (!m) return null;
    i += m[0].length;
    return m[0];
  }
  // Reads one JavaScript value from i to its end: brackets, strings, templates, regexes and comments are followed; a
  // newline outside brackets ends it unless the line plainly goes on.
  function readExpr() {
    const start = i;
    const stack = [];
    let prev = "";
    while (i < n) {
      const c = src[i];
      if (c === '"' || c === "'") {
        i++;
        while (i < n && src[i] !== c) { if (src[i] === "\\") i++; if (src[i] === "\n") { fail(i, "string not closed"); break; } i++; }
        i++; prev = "a"; continue;
      }
      if (c === "`") {
        i++;
        while (i < n && src[i] !== "`") { if (src[i] === "\\") i++; i++; }
        i++; prev = "a"; continue;
      }
      if (src.startsWith("//", i)) { while (i < n && src[i] !== "\n") i++; continue; }
      if (src.startsWith("/*", i)) { const e = src.indexOf("*/", i + 2); i = e < 0 ? n : e + 2; continue; }
      if (c === "/" && (prev === "" || /[(,=:[!&|?{};+\-*%<>~^]/.test(prev) || /\b(return|typeof|in|of|case)$/.test(src.slice(Math.max(0, i - 7), i).trimEnd()))) {
        // A regular expression.
        i++;
        let cls = false;
        while (i < n && (src[i] !== "/" || cls)) {
          if (src[i] === "\\") i++;
          else if (src[i] === "[") cls = true;
          else if (src[i] === "]") cls = false;
          else if (src[i] === "\n") { fail(i, "regular expression not closed"); break; }
          i++;
        }
        i++;
        while (i < n && /[a-z]/.test(src[i])) i++;
        prev = "a"; continue;
      }
      if (c === "(" || c === "[" || c === "{") { stack.push(c); i++; prev = c; continue; }
      if (c === ")" || c === "]" || c === "}") {
        if (stack.length === 0) break;
        const o = stack.pop();
        if ((o === "(" && c !== ")") || (o === "[" && c !== "]") || (o === "{" && c !== "}")) { fail(i, "'" + o + "' closed by '" + c + "'"); }
        i++; prev = "a"; continue;
      }
      if (c === ";" && stack.length === 0) {
        // "if (a) b(); else c()" on one line goes on.
        if (/^;\s*else\b/.test(src.slice(i, i + 20))) { i++; continue; }
        break;
      }
      if (c === "\n" && stack.length === 0) {
        // Goes on when the line ends in an operator or the next line starts with one.
        let j = i + 1;
        // The next line that isn't blank or a comment.
        for (;;) {
          while (j < n && /\s/.test(src[j])) j++;
          if (src.startsWith("//", j)) { while (j < n && src[j] !== "\n") j++; continue; }
          break;
        }
        const next = src.slice(j, j + 5);
        if (/[+\-*/%=&|?:,.([{!<>]/.test(prev) && !/\+\+$|--$/.test(src.slice(start, i).trimEnd())) { i++; continue; }
        if (/^(\.[A-Za-z_]|\?|:|\+|-[^-]|\*|\/[^/*]|&&|\|\||===|!==|==|!=|<|>|else\b)/.test(next)) { i++; continue; }
        break;
      }
      if (!/\s/.test(c)) prev = c;
      i++;
    }
    if (stack.length) fail(start, "'" + stack[stack.length - 1] + "' never closed");
    return src.slice(start, i);
  }
  function compile(code, pos, how) {
    try {
      new vm.Script(how === "expr" ? "(" + code + "\n)" : how === "block" ? "(function(){" + code + "\n})" : code, { filename: file });
    } catch (e) {
      fail(pos, e.message + " in: " + code.trim().split("\n")[0].slice(0, 100));
    }
  }
  // A value after "name:": an object, a list of objects, or JavaScript.
  function value(handler) {
    skipSpace(true);
    const at = i;
    const save = i;
    const t = ident();
    if (t && /^[A-Z]/.test(t.split(".").pop())) {
      skipSpace(true);
      if (src[i] === "{") { object(t); return; }
    }
    i = save;
    if (src[i] === "[") {
      // A list of objects?
      const k = i;
      i++;
      skipSpace(false);
      const t2 = ident();
      skipSpace(false);
      if (t2 && /^[A-Z]/.test(t2.split(".").pop()) && src[i] === "{") {
        i = k + 1;
        for (;;) {
          skipSpace(false);
          if (src[i] === "]") { i++; return; }
          if (src[i] === ",") { i++; continue; }
          const ty = ident();
          skipSpace(false);
          if (!ty || src[i] !== "{") { fail(i, "expected an object in the list"); return; }
          object(ty);
        }
      }
      i = k;
    }
    const e = readExpr();
    if (e.trim() === "") { fail(at, "a property with no value"); return; }
    // A signal handler is statements; anything else is one expression (or a block that returns one).
    if (e.trim()[0] === "{") compile(e.trim().slice(1, -1), at, "block");
    else if (handler && !/^\s*(\([^)]*\)|[A-Za-z_$][\w$]*)\s*=>/.test(e) && !/^\s*function\b/.test(e)) compile(e, at, "block");
    else compile(e, at, "expr");
  }
  function object(type) {
    const open = i;
    if (src[i] !== "{") { fail(i, "expected { after " + type); return; }
    i++;
    for (;;) {
      skipSpace(false);
      if (i >= n) { fail(open, type + " { never closed"); return; }
      if (src[i] === "}") { i++; return; }
      const at = i;
      let word = ident();
      if (!word) { fail(i, "unexpected '" + src[i] + "'"); i++; continue; }
      if (word === "readonly" || word === "required" || word === "default") { skipSpace(true); word = ident(); }
      if (word === "property") {
        skipSpace(true);
        if (src.startsWith("list<", i)) { const e = src.indexOf(">", i); i = e + 1; } else ident();
        skipSpace(true);
        const name = ident();
        if (!name) { fail(i, "a property with no name"); continue; }
        skipSpace(true);
        if (src[i] === ":") { i++; value(); }
        continue;
      }
      if (word === "signal") {
        skipSpace(true); ident(); skipSpace(true);
        if (src[i] === "(") { const e = src.indexOf(")", i); i = e + 1; }
        continue;
      }
      if (word === "function") {
        skipSpace(true);
        const name = ident();
        const s = i;
        if (src[i] !== "(") { fail(i, "function " + name + " without ("); continue; }
        readExpr();
        compile("function " + name + src.slice(s, i), at, "code");
        continue;
      }
      if (word === "component") {
        skipSpace(true); ident(); skipSpace(true);
        if (src[i] === ":") i++;
        skipSpace(true);
        const t = ident();
        skipSpace(true);
        // An inline component has ids of its own.
        idScopes.push(new Set());
        object(t);
        idScopes.pop();
        continue;
      }
      if (word === "enum") { const e = src.indexOf("}", i); i = e + 1; continue; }
      if (word === "import" || word === "pragma") { while (i < n && src[i] !== "\n") i++; continue; }
      skipSpace(true);
      // "Behavior on x { }" and friends.
      if (src.startsWith("on ", i)) { i += 3; skipSpace(true); ident(); skipSpace(true); object(word); continue; }
      if (src[i] === "{" && /^[A-Z]/.test(word.split(".").pop())) { object(word); continue; }
      if (src[i] === ":") {
        i++;
        if (word === "id") {
          skipSpace(true);
          const at = i, name = ident();
          if (!name) fail(i, "id with no name");
          else if (idScopes[idScopes.length - 1].has(name)) fail(at, "id " + name + " is not unique");
          else idScopes[idScopes.length - 1].add(name);
          continue;
        }
        value(/^on[A-Z]/.test(word.split(".").pop()));
        continue;
      }
      fail(i, "unexpected text after " + word);
      while (i < n && src[i] !== "\n") i++;
    }
  }

  // The file: imports, then one root object.
  for (;;) {
    skipSpace(false);
    if (src.startsWith("import ", i) || src.startsWith("pragma ", i)) { while (i < n && src[i] !== "\n") i++; continue; }
    break;
  }
  const root = ident();
  skipSpace(false);
  if (!root) fail(i, "no root object");
  else object(root);
  skipSpace(false);
  if (i < n) fail(i, "text after the root object");
  return errors;
}

const files = process.argv.slice(2);
let all = [];
for (const f of files) all = all.concat(checkFile(f));
for (const e of all) console.log(e);
console.log("QML check: " + files.length + " files, " + all.length + " problems");
process.exit(all.length ? 1 : 0);
