// Finds the Personal API Key on Nexus's API Keys page, only when the player asked Turbine for it (sign-in page,
// "Give Turbine my API key too"). Reads the page, never changes or clicks anything on it.
// How: the element that says "Personal API Key", then the nearest box around it that holds exactly one thing shaped
// like a key (a text field's value, or a line of text on its own). Other apps' keys on the same page sit in their own
// boxes further away: the search never widens into a box with another heading (another app's name, the page's
// title) or the whole page, and when the nearest box holds two keys it is unclear which is the player's, so nothing
// is taken.
// Plain DOM calls only (querySelectorAll, childNodes, parentElement), so the tests can run it on a small fake page.
(function (root) {
  "use strict";

  const P = typeof module !== "undefined" && module.exports ? require("./protocol.js") : root.TurbineProtocol;
  const LABEL = /personal\s+api\s+key/i;
  const MAX_UP = 6;

  // The element's own words (its text nodes, not its children's).
  function ownText(el) {
    let t = "";
    for (const n of el.childNodes || []) if (n.nodeType === 3) t += n.nodeValue || "";
    return t;
  }

  function keyOf(el) {
    const tag = String(el.tagName || "").toUpperCase();
    if (tag === "INPUT" || tag === "TEXTAREA") return typeof el.value === "string" ? el.value.trim() : "";
    if (tag === "SCRIPT" || tag === "STYLE" || tag === "TEMPLATE") return "";
    // A line of text on its own (no elements inside).
    if (el.children && el.children.length > 0) return "";
    return String(el.textContent || "").trim();
  }

  const HEADING = /^H[1-6]$/;
  const WHOLE = ["BODY", "HTML", "MAIN"];

  // May the search widen from `box` to its parent? Not into the whole page, nor a box with a heading of its own
  // that isn't the Personal API Key's (that box holds other apps' keys too).
  function mayWiden(parent) {
    if (!parent || WHOLE.includes(String(parent.tagName || "").toUpperCase())) return false;
    const els = Array.from(parent.querySelectorAll ? parent.querySelectorAll("*") : []);
    return !els.some((el) => HEADING.test(String(el.tagName || "").toUpperCase()) && !LABEL.test(String(el.textContent || "")));
  }

  // Every different key-shaped value inside `box` (the box itself included).
  function keysIn(box) {
    const found = [];
    const els = [box, ...Array.from(box.querySelectorAll ? box.querySelectorAll("*") : [])];
    for (const el of els) {
      const k = keyOf(el);
      if (k && P.plausibleKey(k) && !found.includes(k)) found.push(k);
    }
    return found;
  }

  // Answers {key} or {why} ("nolabel": not the API Keys page, or signed out; "nokey": no key shown, it may need to be
  // requested first; "unclear": more than one key next to the label).
  function findPersonalKey(doc) {
    const all = Array.from(doc.querySelectorAll("*"));
    const labels = all.filter((el) => LABEL.test(ownText(el)));
    if (labels.length === 0) return { why: "nolabel" };
    let unclear = false;
    for (const label of labels) {
      let box = label;
      for (let up = 0; up <= MAX_UP && box; up++) {
        const keys = keysIn(box);
        if (keys.length === 1) return { key: keys[0] };
        if (keys.length > 1) {
          unclear = true;
          break;
        }
        if (!mayWiden(box.parentElement)) break;
        box = box.parentElement;
      }
    }
    return { why: unclear ? "unclear" : "nokey" };
  }

  const api = { findPersonalKey, ownText };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else root.TurbineKeyFind = api;
})(typeof globalThis !== "undefined" ? globalThis : this);
