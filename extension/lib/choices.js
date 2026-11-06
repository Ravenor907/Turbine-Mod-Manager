// Installer choices: keeps what the player picked on one step and enforces each group's rule.
// Group kinds: one (exactly one), atmostone (none or one), atleastone (one or more), any, all (every option).
// Option states: required (always on, locked), disabled (never on), selected (on at start), available.
(function (root) {
  "use strict";

  function create(msg) {
    const sel = msg.groups.map((g) => {
      const on = new Set();
      for (const o of g.options) {
        if (o.state === "disabled") continue;
        if (g.kind === "all" || o.state === "required" || o.state === "selected") on.add(o.id);
      }
      if ((g.kind === "one" || g.kind === "atmostone") && on.size > 1) {
        const keep = g.options.find((o) => on.has(o.id) && o.state === "required") || g.options.find((o) => on.has(o.id));
        on.clear();
        on.add(keep.id);
      }
      if (g.kind === "one" && on.size === 0) {
        const first = g.options.find((o) => o.state !== "disabled");
        if (first) on.add(first.id);
      }
      return on;
    });
    return { msg, sel };
  }

  function option(state, gi, id) {
    const g = state.msg.groups[gi];
    return g ? g.options.find((o) => o.id === id) || null : null;
  }

  function isSelected(state, gi, id) {
    return !!state.sel[gi] && state.sel[gi].has(id);
  }

  // Radio style groups show round marks, the others show boxes.
  function isRadio(state, gi) {
    const k = state.msg.groups[gi].kind;
    return k === "one" || k === "atmostone";
  }

  function hasRequired(state, gi) {
    return state.msg.groups[gi].options.some((o) => o.state === "required");
  }

  function canToggle(state, gi, id) {
    const g = state.msg.groups[gi];
    const o = option(state, gi, id);
    if (!g || !o) return false;
    if (o.state === "disabled" || o.state === "required" || g.kind === "all") return false;
    if (g.kind === "one") return !isSelected(state, gi, id) && !hasRequired(state, gi);
    if (g.kind === "atmostone") return !hasRequired(state, gi);
    return true;
  }

  // Applies a tap on an option. Returns true when something changed.
  function toggle(state, gi, id) {
    if (!canToggle(state, gi, id)) return false;
    const g = state.msg.groups[gi];
    const on = state.sel[gi];
    if (g.kind === "one") {
      on.clear();
      on.add(id);
    } else if (g.kind === "atmostone") {
      if (on.has(id)) on.delete(id);
      else {
        on.clear();
        on.add(id);
      }
    } else if (on.has(id)) on.delete(id);
    else on.add(id);
    return true;
  }

  // Plain words for every group whose rule is not met yet.
  function problems(state) {
    const out = [];
    state.msg.groups.forEach((g, gi) => {
      const n = state.sel[gi].size;
      const usable = g.options.some((o) => o.state !== "disabled");
      if (g.kind === "one" && n !== 1 && usable) out.push({ group: gi, text: "Pick one option in " + (g.title || "this group") });
      if (g.kind === "atleastone" && n < 1 && usable) out.push({ group: gi, text: "Pick at least one option in " + (g.title || "this group") });
      if (g.kind === "atmostone" && n > 1) out.push({ group: gi, text: "Pick no more than one option in " + (g.title || "this group") });
    });
    return out;
  }

  function isValid(state) {
    return problems(state).length === 0;
  }

  // Every selected option id on this step, in the order Turbine sent them.
  function selectedIds(state) {
    const out = [];
    state.msg.groups.forEach((g, gi) => {
      for (const o of g.options) if (state.sel[gi].has(o.id) && !out.includes(o.id)) out.push(o.id);
    });
    return out;
  }

  function isLastStep(state) {
    return state.msg.stepIndex >= state.msg.stepCount - 1;
  }

  function canGoBack(state) {
    return state.msg.stepIndex > 0;
  }

  const api = { create, isSelected, isRadio, canToggle, toggle, problems, isValid, selectedIds, isLastStep, canGoBack };
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else root.TurbineChoices = api;
})(typeof globalThis !== "undefined" ? globalThis : this);
