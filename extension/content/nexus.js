// Content script for nexusmods.com: Turbine's own small panel (in a closed shadow root), the page report and nxm links.
// It never clicks, changes or reads anything on the Nexus page except the URL and the nxm links the player clicks.
// The URL also tells the helper when the player is back on Nexus after its sign-in page (sign-in pass-through).
// While Turbine waits for the Nexus sign-in, a banner across the top of every Nexus page says so. On the API Keys
// page, and only after the player pressed "Give Turbine my API key too", it reads the Personal API Key for Turbine
// (lib/keyfind.js); that is the one thing it reads from a page.
/* global chrome, TurbineProtocol, TurbineChoices, TurbineKeyFind */
(function () {
  "use strict";
  if (window.top !== window || window.__turbineHelper) return;
  window.__turbineHelper = true;

  const P = TurbineProtocol;
  const C = TurbineChoices;

  let state = { conn: "connecting", status: null, added: null, choices: null, error: null, lastError: null };
  let port = null;
  let alive = true;
  let open = false;
  let lastHref = "";
  let choiceState = null;
  let choiceKey = "";
  let waiting = false;
  let seenAdded = 0;
  let seenChoices = "";
  let seenWait = "";
  let seenSignin = "";

  // ---------- Link to the service worker ----------
  function connectPort() {
    if (!alive) return;
    try {
      port = chrome.runtime.connect({ name: "turbine-panel" });
    } catch (e) {
      shutDown();
      return;
    }
    port.onMessage.addListener((msg) => {
      if (msg && msg.kind === "state") onState(msg);
    });
    port.onDisconnect.addListener(() => {
      port = null;
      // The worker restarted (reconnect) or the extension was removed or reloaded (stop).
      if (!chrome.runtime || !chrome.runtime.id) shutDown();
      else setTimeout(connectPort, 1000);
    });
    post({ kind: "hello" });
    lastHref = "";
    reportPage();
  }

  function post(msg) {
    if (!port) return false;
    try {
      port.postMessage(msg);
      return true;
    } catch (e) {
      return false;
    }
  }

  function shutDown() {
    alive = false;
    document.removeEventListener("click", onClick, true);
    if (host && host.parentNode) host.parentNode.removeChild(host);
  }

  // ---------- Page report (URL only) ----------
  function reportPage() {
    if (location.href === lastHref) return;
    lastHref = location.href;
    post({ kind: "where", url: location.href });
    if (P.parseNexusUrl(location.href)) post({ kind: "page", url: location.href });
  }
  setInterval(reportPage, 1000);

  // ---------- nxm links ----------
  // Only taken over while Turbine is connected; otherwise the browser hands the link to the system as usual.
  function onClick(ev) {
    if (ev.defaultPrevented || ev.button !== 0) return;
    const path = ev.composedPath ? ev.composedPath() : [];
    let a = null;
    for (const el of path) {
      if (el && el.tagName === "A" && el.getAttribute && el.getAttribute("href")) {
        a = el;
        break;
      }
    }
    if (!a) return;
    const href = a.href || a.getAttribute("href");
    if (!/^nxm:\/\//i.test(href) || !P.isNxmUrl(href)) return;
    if (state.conn !== "connected" || !port) return;
    if (post({ kind: "nxm", url: href })) ev.preventDefault();
  }
  document.addEventListener("click", onClick, true);

  // ---------- Panel ----------
  const host = document.createElement("turbine-helper");
  host.style.cssText =
    "all:initial!important;position:fixed!important;z-index:2147483647!important;right:16px;bottom:16px;display:block!important;";
  const root = host.attachShadow({ mode: "closed" });

  const css = document.createElement("style");
  css.textContent = `
    :host { all: initial; }
    * { box-sizing: border-box; }
    .wrap { font: 18px/1.35 Barlow, "Segoe UI", Roboto, Ubuntu, Arial, sans-serif; color: #EADFC6; -webkit-font-smoothing: antialiased; }
    button { font: inherit; color: inherit; cursor: pointer; min-height: 44px; min-width: 44px; border-radius: 6px; }
    button:focus-visible, a:focus-visible { outline: 3px solid #EDC87A; outline-offset: 2px; }
    .pill { display: flex; align-items: center; gap: 10px; padding: 8px 16px; background: #0B0A08; border: 2px solid #C99E52;
      border-radius: 6px; box-shadow: 0 4px 18px rgba(0,0,0,.55); touch-action: none; user-select: none; max-width: 70vw; }
    .pill .name { font-family: Cinzel, Georgia, serif; font-weight: 700; color: #EDC87A; letter-spacing: .04em; }
    .pill .short { color: #CDBE9F; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
    .dot { width: 14px; height: 14px; border-radius: 50%; flex: none; background: #6B5A3E; }
    .dot.ok { background: #7FA35A; } .dot.busy { background: #C99E52; } .dot.wait { background: #E0A33A; } .dot.warn { background: #E0A33A; } .dot.stop { background: #C8553D; }
    .card { width: min(440px, calc(100vw - 24px)); max-height: min(72vh, 680px); display: flex; flex-direction: column;
      background: #0B0A08; border: 2px solid #C99E52; border-radius: 6px; box-shadow: 0 8px 28px rgba(0,0,0,.6); overflow: hidden; }
    .head { display: flex; align-items: center; gap: 10px; padding: 6px 6px 6px 14px; background: #1A150D;
      border-bottom: 1px solid #45381F; touch-action: none; user-select: none; cursor: grab; }
    .head .title { flex: 1; font-family: Cinzel, Georgia, serif; font-weight: 700; font-size: 20px; color: #EDC87A; letter-spacing: .04em; }
    .ghost { background: transparent; border: 1px solid #8C662F; color: #EDC87A; padding: 0 14px; }
    .ghost:hover { background: #2A2216; }
    .main { background: #C99E52; border: 1px solid #EDC87A; color: #0B0A08; font-weight: 700; padding: 0 20px; }
    .main:hover { background: #EDC87A; }
    button:disabled { opacity: .45; cursor: default; }
    .body { overflow-y: auto; scrollbar-color: #8C662F #141109; padding: 12px 14px 14px; display: flex; flex-direction: column; gap: 12px; overscroll-behavior: contain; }
    .body > div:empty { display: none; }
    .muted { color: #A08C68; font-size: 16px; }
    .line { display: flex; align-items: center; gap: 10px; }
    .bar { height: 10px; border-radius: 6px; background: #2A2216; overflow: hidden; }
    .bar > div { height: 100%; background: #C99E52; width: 0; }
    .box { border: 1px solid #45381F; background: #100E0A; border-radius: 6px; padding: 12px; display: flex; flex-direction: column; gap: 8px; }
    h2 { margin: 0; font-family: Cinzel, Georgia, serif; font-size: 20px; color: #EDC87A; font-weight: 700; }
    h3 { margin: 4px 0 0; font-size: 18px; color: #EDC87A; font-weight: 600; }
    ul { margin: 0; padding-left: 22px; } li { margin: 4px 0; }
    a { color: #EDC87A; text-decoration: underline; display: inline-flex; align-items: center; min-height: 44px; }
    .opt { display: flex; gap: 12px; width: 100%; text-align: left; padding: 10px 12px; background: #141109; border: 1px solid #45381F; align-items: flex-start; }
    .opt.on { background: #2A2216; border-color: #C99E52; }
    .opt:disabled { opacity: .5; }
    .mark { flex: none; width: 24px; height: 24px; margin-top: 2px; border: 2px solid #C99E52; background: #0B0A08; display: grid; place-items: center; }
    .mark.radio { border-radius: 50%; } .mark.box { border-radius: 4px; }
    .opt.on .mark::after { content: ""; width: 12px; height: 12px; background: #EDC87A; border-radius: inherit; }
    .opt .txt { display: flex; flex-direction: column; gap: 4px; min-width: 0; }
    .opt .oname { font-weight: 600; }
    .opt .desc { color: #CDBE9F; font-size: 16px; white-space: pre-wrap; overflow-wrap: anywhere; }
    .opt .tag { color: #A08C68; font-size: 15px; }
    canvas { max-width: 100%; border-radius: 6px; display: block; }
    .rule { color: #A08C68; font-size: 16px; }
    .warnline { color: #E0A33A; font-size: 16px; }
    .row { display: flex; gap: 10px; flex-wrap: wrap; justify-content: flex-end; }
    .row .spacer { flex: 1; }
  `;
  root.appendChild(css);

  const wrap = el("div", "wrap");
  root.appendChild(wrap);

  // Pill (collapsed)
  const pill = el("button", "pill");
  pill.type = "button";
  pill.setAttribute("aria-label", "Show Turbine panel");
  const pillDot = el("span", "dot");
  const pillName = el("span", "name", "Turbine");
  const pillShort = el("span", "short");
  pill.append(pillDot, pillName, pillShort);

  // Card (open)
  const card = el("div", "card");
  card.setAttribute("role", "dialog");
  card.setAttribute("aria-label", "Turbine");
  const head = el("div", "head");
  const headDot = el("span", "dot");
  const headTitle = el("span", "title", "Turbine");
  const hideBtn = el("button", "ghost", "Hide");
  hideBtn.type = "button";
  head.append(headDot, headTitle, hideBtn);
  const body = el("div", "body");
  const secConn = el("div");
  const secStatus = el("div");
  const secAdded = el("div");
  const secChoices = el("div");
  const secSignin = el("div");
  body.append(secConn, secSignin, secStatus, secAdded, secChoices);
  card.append(head, body);

  function el(tag, cls, text) {
    const e = document.createElement(tag);
    if (cls) e.className = cls;
    if (text !== undefined) e.textContent = text;
    return e;
  }

  function button(text, cls, onTap) {
    const b = el("button", cls, text);
    b.type = "button";
    b.addEventListener("click", (ev) => {
      ev.stopPropagation();
      onTap();
    });
    return b;
  }

  // ---------- Dragging (touch and mouse), position saved ----------
  let pos = { right: 16, bottom: 16 };
  let dragged = false;

  function applyPos() {
    const r = host.getBoundingClientRect();
    const maxRight = Math.max(0, window.innerWidth - Math.max(r.width, 60));
    const maxBottom = Math.max(0, window.innerHeight - Math.max(r.height, 44));
    pos.right = Math.min(Math.max(0, pos.right), maxRight);
    pos.bottom = Math.min(Math.max(0, pos.bottom), maxBottom);
    host.style.setProperty("right", pos.right + "px", "important");
    host.style.setProperty("bottom", pos.bottom + "px", "important");
  }

  function makeDraggable(handle) {
    let start = null;
    handle.addEventListener("pointerdown", (ev) => {
      if (ev.button !== 0) return;
      if (ev.target !== handle && ev.target.closest && ev.target.closest("button") && ev.target.closest("button") !== handle) return;
      start = { x: ev.clientX, y: ev.clientY, right: pos.right, bottom: pos.bottom, id: ev.pointerId };
      dragged = false;
    });
    handle.addEventListener("pointermove", (ev) => {
      if (!start || ev.pointerId !== start.id) return;
      const dx = ev.clientX - start.x;
      const dy = ev.clientY - start.y;
      if (!dragged && Math.hypot(dx, dy) < 8) return;
      if (!dragged) {
        dragged = true;
        try {
          handle.setPointerCapture(ev.pointerId);
        } catch (e) {
          /* ignore */
        }
      }
      pos.right = start.right - dx;
      pos.bottom = start.bottom - dy;
      applyPos();
    });
    const end = (ev) => {
      if (!start || ev.pointerId !== start.id) return;
      start = null;
      if (dragged) chrome.storage.local.set({ panelPos: pos });
    };
    handle.addEventListener("pointerup", end);
    handle.addEventListener("pointercancel", end);
  }
  makeDraggable(pill);
  makeDraggable(head);
  window.addEventListener("resize", applyPos);

  pill.addEventListener("click", () => {
    if (dragged) {
      dragged = false;
      return;
    }
    setOpen(true);
  });
  hideBtn.addEventListener("click", () => setOpen(false));
  card.addEventListener("keydown", (ev) => {
    if (ev.key === "Escape") setOpen(false);
  });

  function setOpen(v) {
    open = v;
    chrome.storage.local.set({ panelOpen: v });
    render(true);
    requestAnimationFrame(applyPos);
  }

  // ---------- Images from data URLs (decoded here, so the page's image rules don't block them) ----------
  const bitmaps = new Map();
  function drawImage(canvas, dataUrl) {
    const show = (bmp) => {
      const w = Math.min(bmp.width, 400);
      const h = Math.round((bmp.height * w) / bmp.width);
      canvas.width = w;
      canvas.height = h;
      canvas.getContext("2d").drawImage(bmp, 0, 0, w, h);
    };
    if (bitmaps.has(dataUrl)) {
      const b = bitmaps.get(dataUrl);
      if (b) show(b);
      else canvas.remove();
      return;
    }
    try {
      const comma = dataUrl.indexOf(",");
      const mime = dataUrl.slice(5, dataUrl.indexOf(";"));
      const bin = atob(dataUrl.slice(comma + 1).replace(/\s+/g, ""));
      const bytes = new Uint8Array(bin.length);
      for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
      createImageBitmap(new Blob([bytes], { type: mime }))
        .then((bmp) => {
          bitmaps.set(dataUrl, bmp);
          if (canvas.isConnected) show(bmp);
        })
        .catch(() => {
          bitmaps.set(dataUrl, null);
          canvas.remove();
        });
    } catch (e) {
      bitmaps.set(dataUrl, null);
      canvas.remove();
    }
  }

  // ---------- Rendering ----------
  const sigs = { conn: "", signin: "", status: "", added: "", choices: "" };

  function level() {
    if (state.conn === "refused") return "warn";
    if (state.conn !== "connected") return "";
    if (state.choices) return "busy";
    return state.status ? state.status.level : "ok";
  }

  function shortText() {
    if (state.conn === "unpaired") return "Not paired";
    if (state.conn === "refused") return "Pair again";
    if (state.conn !== "connected") return "Not connected";
    if (state.choices) return "Needs your choice";
    const s = state.status;
    if (s && s.level === "busy" && typeof s.progress === "number") return Math.round(s.progress * 100) + "%";
    if (s && s.text) return s.text;
    return "Ready";
  }

  function onState(msg) {
    state = msg;
    let wantOpen = false;
    if (state.added && state.added.at !== seenAdded) {
      seenAdded = state.added.at;
      wantOpen = true;
    }
    const ck = state.choices ? state.choices.sessionId + ":" + state.choices.stepIndex + ":" + JSON.stringify(state.choices.groups).length : "";
    if (ck !== seenChoices) {
      seenChoices = ck;
      waiting = false;
      if (state.choices) wantOpen = true;
    }
    // Turbine waits for the player to do something on the page (press Slow download): the panel opens to say so.
    const wk = state.status && state.status.level === "wait" ? state.status.text : "";
    if (wk !== seenWait) {
      seenWait = wk;
      if (wk) wantOpen = true;
    }
    // Turbine asked for the Nexus sign-in: the panel opens to say what happens.
    const sk = state.signin ? state.signin.text : "";
    if (sk !== seenSignin) {
      seenSignin = sk;
      if (sk) wantOpen = true;
    }
    if (state.conn === "connected" && lastHref === "") reportPage();
    // The banner and the key reading work whether the panel is open or not.
    const g = state.conn === "connected" ? state.signin : null;
    const bsig = g ? JSON.stringify(g) : "";
    if (bsig !== bannerSig) {
      bannerSig = bsig;
      renderBanner(g);
    }
    readKeyWhenAsked(g);
    if (wantOpen && !open) {
      open = true;
      chrome.storage.local.set({ panelOpen: true });
    }
    render(false);
  }

  function render(force) {
    if (force) for (const k of Object.keys(sigs)) sigs[k] = "";
    const lv = level();
    pillDot.className = "dot " + lv;
    headDot.className = "dot " + lv;
    pillShort.textContent = shortText();
    if (open && pill.isConnected) pill.remove();
    if (!open && card.isConnected) card.remove();
    if (open && !card.isConnected) wrap.appendChild(card);
    if (!open && !pill.isConnected) wrap.appendChild(pill);
    if (!open) return;
    const top = body.scrollTop;
    renderConn();
    renderSignin();
    renderStatus();
    renderAdded();
    renderChoices();
    body.scrollTop = top;
  }

  function renderConn() {
    const sig = state.conn + "|" + (state.error || "") + "|" + (state.lastError || "");
    if (sig === sigs.conn) return;
    sigs.conn = sig;
    secConn.replaceChildren();
    let text = null;
    if (state.conn === "unpaired") text = "This browser is not paired with Turbine yet. In Turbine, open Settings, then Getting mods, and choose Pair again.";
    else if (state.conn === "refused") text = (state.error ? state.error + " " : "") + "In Turbine, open Settings, then Getting mods, and choose Pair again.";
    else if (state.conn === "offline" || state.conn === "connecting") text = "Turbine is not running. Open Turbine and this panel connects by itself. Downloads still go to your Downloads folder.";
    if (text) {
      const box = el("div", "box");
      box.append(el("div", "warnline", text));
      secConn.append(box);
    }
    if (state.lastError && state.conn === "connected") {
      const box = el("div", "box");
      box.append(el("div", "warnline", state.lastError), rowOf(button("OK", "ghost", () => post({ kind: "dismiss" }))));
      secConn.append(box);
    }
  }

  // Sign in with my browser: what is happening, and a button to send the sign-in now (the player's own press).
  function renderSignin() {
    const g = state.conn === "connected" ? state.signin : null;
    const sig = g ? JSON.stringify(g) : "";
    if (sig === sigs.signin) return;
    sigs.signin = sig;
    secSignin.replaceChildren();
    if (!g) return;
    const box = el("div", "box");
    box.append(el("h2", "", g.ok ? "Signed in. Go back to Turbine" : "Turbine is waiting for you to sign in to Nexus Mods"));
    box.append(el("div", "", g.text));
    if (g.back) box.append(el("div", "muted", g.back));
    if (g.pending && g.phase !== "expired") box.append(rowOf(button("I'm signed in. Send it to Turbine", "main", () => post({ kind: "signin.send" }))));
    secSignin.append(box);
  }

  // ---------- The sign-in banner across the top ----------
  const bannerHost = document.createElement("turbine-banner");
  bannerHost.style.cssText = "all:initial!important;position:fixed!important;z-index:2147483647!important;left:0;right:0;top:0;display:block!important;";
  const bannerRoot = bannerHost.attachShadow({ mode: "closed" });
  const bannerCss = document.createElement("style");
  bannerCss.textContent = `
    :host { all: initial; }
    * { box-sizing: border-box; }
    .ban { display: flex; align-items: center; gap: 14px; flex-wrap: wrap; padding: 10px 18px; background: #0B0A08; border-bottom: 3px solid #C99E52;
      box-shadow: 0 4px 18px rgba(0,0,0,.55); font: 18px/1.35 Barlow, "Segoe UI", Roboto, Ubuntu, Arial, sans-serif; color: #EADFC6; }
    .ban.ok { border-bottom-color: #7FA35A; } .ban.warn { border-bottom-color: #E0A33A; }
    .name { font-family: Cinzel, Georgia, serif; font-weight: 700; color: #EDC87A; letter-spacing: .04em; }
    .words { flex: 1; min-width: 240px; display: flex; flex-direction: column; }
    .title { font-weight: 700; color: #EDC87A; font-size: 20px; }
    .sub { color: #CDBE9F; font-size: 16px; }
    .dot { width: 16px; height: 16px; border-radius: 50%; flex: none; background: #C99E52; }
    .ok .dot { background: #7FA35A; } .warn .dot { background: #E0A33A; }
    button { font: inherit; cursor: pointer; min-height: 44px; min-width: 44px; border-radius: 6px; padding: 0 18px; }
    button:focus-visible { outline: 3px solid #EDC87A; outline-offset: 2px; }
    .main { background: #C99E52; border: 1px solid #EDC87A; color: #0B0A08; font-weight: 700; }
    .ghost { background: transparent; border: 1px solid #8C662F; color: #EDC87A; }
  `;
  bannerRoot.appendChild(bannerCss);
  let bannerHidden = "";
  let bannerSig = "";

  function renderBanner(g) {
    bannerRoot.querySelectorAll(".ban").forEach((n) => n.remove());
    const phase = g ? g.phase : "";
    if (!g || phase === "expired" || bannerHidden === phase) {
      if (bannerHost.parentNode) bannerHost.parentNode.removeChild(bannerHost);
      return;
    }
    const warn = phase === "failed" || phase === "key-missing" || phase === "key-failed";
    const ban = el("div", "ban" + (g.ok && !warn ? " ok" : warn ? " warn" : ""));
    ban.setAttribute("role", "status");
    const words = el("div", "words");
    words.append(el("span", "title", g.ok ? "Signed in. Go back to Turbine" : "Turbine is waiting for you to sign in to Nexus Mods"));
    words.append(el("span", "sub", g.ok ? (phase === "ok" ? g.back : g.text) : g.text));
    ban.append(el("span", "dot"), el("span", "name", "Turbine"), words);
    if (g.pending) ban.append(button("I'm signed in. Send it to Turbine", "main", () => post({ kind: "signin.send" })));
    if (phase === "key-missing" || phase === "key-failed") ban.append(button("Try again", "main", () => post({ kind: "key.start" })));
    ban.append(button("Hide", "ghost", () => {
      bannerHidden = phase;
      renderBanner(g);
    }));
    bannerRoot.appendChild(ban);
    if (!bannerHost.parentNode) document.documentElement.appendChild(bannerHost);
  }

  // ---------- The Personal API Key, when the player asked for it ----------
  let keyTry = 0;
  let keyTimer = null;

  function readKeyWhenAsked(g) {
    const asked = !!g && g.phase === "key-reading" && P.isKeyPage(location.href);
    if (!asked) {
      if (keyTimer) clearTimeout(keyTimer);
      keyTimer = null;
      keyTry = 0;
      return;
    }
    if (keyTimer) return;
    // The page draws its parts after it loads: look once a second, for up to 12 seconds.
    const look = () => {
      keyTimer = null;
      const r = TurbineKeyFind.findPersonalKey(document);
      keyTry += 1;
      if (r.key) post({ kind: "key.found", key: r.key });
      else if (keyTry >= 12) post({ kind: "key.missing", why: r.why });
      else keyTimer = setTimeout(look, 1000);
    };
    keyTimer = setTimeout(look, 600);
  }

  function renderStatus() {
    const s = state.conn === "connected" ? state.status : null;
    const sig = s ? JSON.stringify(s) : "";
    if (sig === sigs.status) return;
    sigs.status = sig;
    secStatus.replaceChildren();
    if (!s) {
      if (state.conn === "connected") secStatus.append(el("div", "muted", "Connected. Mods you download from Nexus go into Turbine."));
      return;
    }
    const line = el("div", "line");
    const d = el("span", "dot " + s.level);
    line.append(d, el("span", "", s.text));
    secStatus.append(line);
    if (s.game) secStatus.append(el("div", "muted", "Game: " + s.game));
    if (typeof s.progress === "number") {
      const bar = el("div", "bar");
      const fill = el("div");
      fill.style.width = Math.round(s.progress * 100) + "%";
      bar.setAttribute("role", "progressbar");
      bar.setAttribute("aria-valuenow", String(Math.round(s.progress * 100)));
      bar.append(fill);
      secStatus.append(bar);
    }
  }

  function rowOf(...items) {
    const r = el("div", "row");
    r.append(...items);
    return r;
  }

  function renderAdded() {
    const a = state.added;
    const sig = a ? String(a.at) : "";
    if (sig === sigs.added) return;
    sigs.added = sig;
    secAdded.replaceChildren();
    if (!a) return;
    const box = el("div", "box");
    box.append(el("h2", "", "Added to " + a.game));
    box.append(el("div", "", a.mod));
    if (a.notes && a.notes.length) {
      const ul = el("ul");
      for (const n of a.notes) ul.append(el("li", "", n));
      box.append(ul);
    }
    if (a.needs && a.needs.length) {
      box.append(el("h3", "", "Also needs"));
      const ul = el("ul");
      for (const n of a.needs) {
        const li = el("li");
        if (n.url) {
          const link = el("a", "", n.name);
          link.href = n.url;
          link.target = "_blank";
          link.rel = "noopener noreferrer";
          li.append(link);
        } else li.textContent = n.name;
        ul.append(li);
      }
      box.append(ul);
    }
    box.append(rowOf(button("OK", "ghost", () => post({ kind: "dismiss" }))));
    secAdded.append(box);
  }

  let waitTimer = null;
  function startWaiting() {
    waiting = true;
    if (waitTimer) clearTimeout(waitTimer);
    // If Turbine never answers, give the buttons back after a while.
    waitTimer = setTimeout(() => {
      waiting = false;
      render(false);
    }, 15000);
  }

  const RULES = {
    one: "Pick one",
    atmostone: "Pick one or none",
    atleastone: "Pick one or more",
    any: "Pick any you want",
    all: "All of these are included",
  };

  function renderChoices() {
    const m = state.conn === "connected" ? state.choices : null;
    if (!m) {
      choiceState = null;
      choiceKey = "";
      if (sigs.choices !== "") {
        sigs.choices = "";
        secChoices.replaceChildren();
      }
      return;
    }
    const key = m.sessionId + ":" + m.stepIndex + ":" + JSON.stringify(m.groups);
    if (key !== choiceKey) {
      choiceKey = key;
      choiceState = C.create(m);
    }
    const sig = key + "|" + waiting + "|" + JSON.stringify(C.selectedIds(choiceState));
    if (sig === sigs.choices) return;
    sigs.choices = sig;
    const focusedId = root.activeElement && root.activeElement.dataset ? root.activeElement.dataset.opt : null;
    secChoices.replaceChildren();

    const box = el("div", "box");
    box.append(el("h2", "", m.modName ? "Install " + m.modName : "Install options"));
    box.append(el("div", "muted", "Step " + (m.stepIndex + 1) + " of " + m.stepCount + (m.stepTitle ? ": " + m.stepTitle : "")));

    m.groups.forEach((g, gi) => {
      if (g.title) box.append(el("h3", "", g.title));
      box.append(el("div", "rule", RULES[g.kind] || RULES.any));
      const radio = C.isRadio(choiceState, gi);
      for (const o of g.options) {
        const on = C.isSelected(choiceState, gi, o.id);
        const can = C.canToggle(choiceState, gi, o.id);
        const b = el("button", "opt" + (on ? " on" : ""));
        b.type = "button";
        b.dataset.opt = gi + ":" + o.id;
        b.setAttribute("role", radio ? "radio" : "checkbox");
        b.setAttribute("aria-checked", on ? "true" : "false");
        if (o.state === "disabled" || waiting) b.disabled = true;
        else if (!can) b.setAttribute("aria-disabled", "true");
        const mark = el("span", "mark " + (radio ? "radio" : "box"));
        const txt = el("span", "txt");
        txt.append(el("span", "oname", o.name));
        if (o.state === "required") txt.append(el("span", "tag", "Always included"));
        if (o.state === "disabled") txt.append(el("span", "tag", "Not available with your setup"));
        if (o.description) txt.append(el("span", "desc", o.description));
        if (o.image) {
          const c = el("canvas");
          c.setAttribute("aria-hidden", "true");
          txt.append(c);
          drawImage(c, o.image);
        }
        b.append(mark, txt);
        b.addEventListener("click", (ev) => {
          ev.stopPropagation();
          if (C.toggle(choiceState, gi, o.id)) render(false);
        });
        box.append(b);
      }
    });

    for (const p of C.problems(choiceState)) box.append(el("div", "warnline", p.text));
    if (waiting) box.append(el("div", "muted", "Waiting for Turbine..."));

    const cancel = button("Cancel", "ghost", () => {
      startWaiting();
      post({ kind: "choice.cancel", sessionId: m.sessionId });
      render(false);
    });
    const spacer = el("span", "spacer");
    const items = [cancel, spacer];
    if (C.canGoBack(choiceState)) {
      const back = button("Back", "ghost", () => {
        startWaiting();
        post({ kind: "choice.back", sessionId: m.sessionId });
        render(false);
      });
      back.disabled = waiting;
      items.push(back);
    }
    const next = button(C.isLastStep(choiceState) ? "Install" : "Next", "main", () => {
      if (!C.isValid(choiceState)) return;
      startWaiting();
      post({ kind: "choice", sessionId: m.sessionId, step: m.stepIndex, selected: C.selectedIds(choiceState) });
      render(false);
    });
    next.disabled = waiting || !C.isValid(choiceState);
    cancel.disabled = waiting;
    items.push(next);
    box.append(rowOf(...items));
    secChoices.append(box);

    if (focusedId) {
      const again = secChoices.querySelector('[data-opt="' + CSS.escape(focusedId) + '"]');
      if (again) again.focus();
    }
  }

  // ---------- Start ----------
  chrome.storage.local.get(["panelPos", "panelOpen"]).then((s) => {
    if (s.panelPos && typeof s.panelPos.right === "number" && typeof s.panelPos.bottom === "number") pos = s.panelPos;
    open = s.panelOpen === true;
    // Attached to <html>, outside the page's own body, so the site's scripts never see it in their tree.
    document.documentElement.appendChild(host);
    render(true);
    applyPos();
    connectPort();
  });
})();
