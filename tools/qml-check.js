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
    // A property or handler set twice in one object stops the file loading ("Property value set multiple times").
    const assigned = new Set();
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
        if (assigned.has(word)) fail(at, word + " is set twice in one " + type);
        assigned.add(word);
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

// Every Settings action row (kind: "action", key: "x") must have its handler in Main.qml's rowActions, so a
// row can never do nothing when pressed.
function settingsRows(file) {
  const src = require("fs").readFileSync(file, "utf8");
  const start = src.indexOf("readonly property var rowActions: ({");
  if (start < 0) return [file + ": no rowActions table"];
  const end = src.indexOf("\n    })", start);
  const table = src.slice(start, end);
  const handled = new Set([...table.matchAll(/(?:^|[\s,{])([A-Za-z]\w*):\s*(?:\(|\w+\s*=>)/gm)].map(m => m[1]));
  const keys = [...src.matchAll(/kind:\s*"action",\s*key:\s*"(\w+)"/g)].map(m => m[1]);
  const missing = [...new Set(keys.filter(k => !handled.has(k)))];
  console.log("Settings rows: " + keys.length + " action rows, " + missing.length + " without an action");
  return missing.map(k => file + ": the Settings row \"" + k + "\" has no handler in rowActions");
}

// A property declared with the name of one of Item's own FINAL properties stops the whole file loading ("Cannot
// override FINAL property"), which this reader can't know from types: check the names.
// (Only the anchor lines: other Item names such as x or states are fine on a QtObject or a Settings.)
const FINAL = new Set(["left", "right", "top", "bottom", "horizontalCenter", "verticalCenter", "baseline"]);
function finalNames(file) {
  const src = require("fs").readFileSync(file, "utf8");
  const out = [];
  const re = /^\s*(?:readonly\s+|required\s+|default\s+)*property\s+[\w.<>]+\s+(\w+)/gm;
  let m;
  while ((m = re.exec(src))) {
    if (FINAL.has(m[1])) out.push(file + ":" + (src.slice(0, m.index).split("\n").length + 1) + ": property " + m[1] + " has the name of an Item property (Cannot override FINAL property)");
  }
  // A property with the name of a function in the same file hides the function ("is not a function" at run time).
  // (The root object's own, four spaces in.)
  const fns = new Set([...src.matchAll(/^    function\s+(\w+)\s*\(/gm)].map(x => x[1]));
  const re2 = /^    (?:readonly\s+|required\s+|default\s+)*property\s+[\w.<>]+\s+(\w+)/gm;
  while ((m = re2.exec(src))) {
    if (fns.has(m[1])) out.push(file + ":" + (src.slice(0, m.index).split("\n").length + 1) + ": property " + m[1] + " has the name of a function in this file");
  }
  return out;
}

// Every overlay uses Overlay.qml. In Qt 6 a backdrop MouseArea stops the items under it but never their
// pointer handlers, so a full-window backdrop of a file's own lets taps through to the page beneath: the only
// backdrop allowed is Overlay's (which also never closes while another layer is on top; Main.qml turns the layers
// beneath off). In Main.qml every overlay sits in the sheet layer (the quick wheel and the tutorial above it), and
// every tap wired from the page layer, the browser and the sheets asks app.gate() first.
// Main.qml is in parts: a file whose root takes Main.qml's window as app (required property var app) is one. The
// same rules hold in the parts: an overlay only in the parts Main.qml puts in the sheet layer, and a gate() on every
// tap in the parts it puts between mainRow and the bar (the page layer and the sheet layer, with the parts inside
// those), the same lines the rules read when they were all in Main.qml.
function overlays(files) {
  const fs = require("fs");
  const out = [];
  const rootOf = {};
  const fileOf = {};
  const isPart = {};
  for (const f of files) {
    const src = fs.readFileSync(f, "utf8");
    const m = /^([A-Z][\w.]*)\s*\{/m.exec(src.replace(/^\s*(import|pragma)[^\n]*$/gm, "").replace(/^\s*\/\/[^\n]*$/gm, ""));
    const name = f.split("/").pop().replace(/\.qml$/, "");
    rootOf[name] = m ? m[1] : "";
    fileOf[name] = f;
    isPart[name] = name !== "Main" && /^ {4}required property var app\b/m.test(src);
  }
  const overlayTypes = new Set(Object.keys(rootOf).filter(k => rootOf[k] === "Overlay").concat(["Overlay"]));
  const made = (l) => { const m = /^\s*([A-Z]\w*) \{/.exec(l); return m ? m[1] : "" };
  // The parts placed in these lines, and the parts placed inside those.
  const partsIn = (lines) => {
    const found = new Set();
    const todo = lines.map(made).filter(t => isPart[t]);
    while (todo.length) {
      const t = todo.pop();
      if (found.has(t)) continue;
      found.add(t);
      for (const l of fs.readFileSync(fileOf[t], "utf8").split("\n")) if (isPart[made(l)]) todo.push(made(l));
    }
    return found;
  };
  // A tap with no app.gate() first, in lines from..to of a file (the status card's own windows, over other apps,
  // aren't under any layer of this window).
  const gateRule = (f, lines, from, to) => {
    for (let k = from; k < to; k++) {
      const l = lines[k];
      if (!/^\s*on(Tapped|[A-Z]\w*Clicked|Activated|ItemMenu|RowMenu|RowHeld|Swiped|Closed|OutsideTapped|Choose|Next|Back|Stop)\s*:/.test(l)) continue;
      if (/gate\(/.test(l) || /gate\(/.test(lines[k + 1] || "")) continue;
      let j = k; while (j > 0 && !/^ {4}[A-Z]/.test(lines[j])) j--;
      if (/^ {4}StatusOverlay\b/.test(lines[j])) continue;
      out.push(f + ":" + (k + 1) + ": a tap with no app.gate() first: " + l.trim().slice(0, 80));
    }
  };
  let pageParts = new Set(), sheetParts = new Set();
  for (const f of files) {
    const name = f.split("/").pop().replace(/\.qml$/, "");
    if (name === "Overlay" || name === "CardGuard") continue;
    const lines = fs.readFileSync(f, "utf8").split("\n");
    // A full-window MouseArea or TapHandler straight under the root object (four spaces in), or one filling a
    // root-level backdrop Rectangle.
    for (let k = 0; k < lines.length; k++) {
      const l = lines[k];
      if (!/^ {4}(MouseArea|TapHandler)\b/.test(l) && !/^ {8}(MouseArea|TapHandler) \{ anchors\.fill: parent/.test(l)) continue;
      if (/^ {8}/.test(l)) {
        // Only when its parent is a root-level Rectangle that fills the window.
        let j = k - 1; while (j >= 0 && !/^ {4}\S/.test(lines[j])) j--;
        const block = lines.slice(j, k).join("\n");
        if (!/^ {4}Rectangle \{/.test(lines[j] || "") || !/anchors\.fill: parent/.test(block)) continue;
      } else {
        let block = l; for (let j = k + 1; j < lines.length && j < k + 10 && !/^ {4}\}/.test(lines[j - 1]); j++) block += "\n" + lines[j];
        if (!/anchors\.fill: parent/.test(block)) continue;
      }
      if (/overlay-check: /.test(l)) continue;
      if (rootOf[name] !== "Overlay") out.push(f + ":" + (k + 1) + ": a full-window backdrop of its own: make the root Overlay (Overlay.qml) and use onOutsideTapped");
    }
    if (name !== "Main") continue;
    const at = (re) => lines.findIndex(x => re.test(x));
    const sheetsFrom = at(/^ {8}id: sheetLayer$/), barFrom = at(/^ {8}id: legend$/);
    const pageFrom = at(/^ {8}id: mainRow$/), pageTo = at(/^ {8}id: sheetLayer$/);
    if (sheetsFrom < 0 || barFrom < 0 || pageFrom < 0) { out.push(f + ": no mainRow, sheetLayer or legend"); continue; }
    // From the object whose id is sheetLayer (or mainRow) to the bar's: an object's opening line is above its id.
    const opening = (k) => { while (k > 0 && !made(lines[k])) k--; return k };
    sheetParts = partsIn(lines.slice(opening(sheetsFrom), opening(barFrom)));
    pageParts = partsIn(lines.slice(opening(pageFrom), opening(barFrom)));
    if (!sheetParts.size) out.push(f + ": no part (a file with required property var app) in the sheet layer");
    for (let k = 0; k < lines.length; k++) {
      const m = /^\s*([A-Z]\w*) \{/.exec(lines[k]);
      if (m && overlayTypes.has(m[1]) && !(k > sheetsFrom && k < barFrom) && ["QuickWheel", "Tutorial"].indexOf(m[1]) < 0)
        out.push(f + ":" + (k + 1) + ": " + m[1] + " is an overlay outside the sheet layer");
    }
    // Taps from the page (main row and the Nexus page) and the sheets ask app.gate() first.
    gateRule(f, lines, pageFrom, barFrom);
  }
  // The parts: overlays only in the sheet layer's, a gate() on every tap in the page layer's and the sheet layer's.
  // (Only with Main.qml in the files, which places them.)
  for (const name of Object.keys(isPart).filter(k => isPart[k] && fileOf.Main).sort()) {
    const f = fileOf[name];
    const lines = fs.readFileSync(f, "utf8").split("\n");
    if (!sheetParts.has(name)) {
      for (let k = 0; k < lines.length; k++) {
        const t = made(lines[k]);
        if (overlayTypes.has(t) && ["QuickWheel", "Tutorial"].indexOf(t) < 0)
          out.push(f + ":" + (k + 1) + ": " + t + " is an overlay outside the sheet layer (Main.qml doesn't put " + name + " in it)");
      }
    }
    if (pageParts.has(name)) gateRule(f, lines, 0, lines.length);
  }
  // The Nexus page's own taps, wherever its Connections are (Main.qml or a part).
  for (const name of ["Main"].concat(Object.keys(isPart).filter(k => isPart[k]))) {
    if (!fileOf[name]) continue;
    const f = fileOf[name];
    const lines = fs.readFileSync(f, "utf8").split("\n");
    for (const fn of ["onSigninClicked", "onGooglePassword", "onGoogleChrome", "onCloseClicked", "onTabClicked", "onDownloadClicked"]) {
      const k = lines.findIndex(x => x.indexOf("function " + fn + "(") >= 0);
      if (k >= 0 && !/gate\(/.test(lines[k])) out.push(f + ":" + (k + 1) + ": the Nexus page's " + fn + " has no app.gate() first");
    }
  }
  console.log("Overlay check: " + [...overlayTypes].filter(t => t !== "Overlay").length + " overlays on Overlay.qml, taps checked in Main.qml and " +
    pageParts.size + " parts (" + [...pageParts].sort().join(", ") + "), " + out.length + " problems");
  return out;
}

// One motion system. Every duration and curve comes from Motion.qml (ui.motion, or mo in Main.qml): a
// duration written as a number, or an easing curve named, anywhere else is a problem. (Timers' intervals and the
// time a hold takes are not animations and are not looked at.)
function motionRule(files) {
  const out = [];
  let n = 0;
  for (const f of files) {
    if (/(^|\/)Motion\.qml$/.test(f)) continue;
    const lines = fs.readFileSync(f, "utf8").split("\n");
    for (let k = 0; k < lines.length; k++) {
      const l = lines[k].replace(/\/\/.*$/, "");
      if (/\b(duration|highlightMoveDuration|highlightResizeDuration|velocity)\s*:\s*-?\d/.test(l)) out.push(f + ":" + (k + 1) + ": a duration written as a number (take it from Motion.qml): " + l.trim().slice(0, 80));
      if (/\bEasing\.[A-Z]\w+/.test(l)) out.push(f + ":" + (k + 1) + ": an easing curve named here (take it from Motion.qml): " + l.trim().slice(0, 80));
      if (/\b(duration|easing\.type)\s*:/.test(l)) n++;
    }
  }
  console.log("Motion check: " + n + " durations and curves, " + out.length + " outside Motion.qml");
  return out;
}

const files = process.argv.slice(2);
let all = [];
for (const f of files) all = all.concat(checkFile(f)).concat(finalNames(f));
all = all.concat(motionRule(files));
for (const f of files) if (f.endsWith("Main.qml")) all = all.concat(settingsRows(f));
all = all.concat(overlays(files));
for (const e of all) console.log(e);
console.log("QML check: " + files.length + " files, " + all.length + " problems");
process.exit(all.length ? 1 : 0);
