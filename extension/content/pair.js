// Content script for Turbine's pairing page, http://127.0.0.1:<port>/pair#<token>.
// Hands the port and token to the service worker (which checks and stores them), then says so on the page.
/* global chrome, TurbineProtocol */
(function () {
  "use strict";
  const pairing = TurbineProtocol.parsePairingUrl(location.href);
  if (!pairing) return;
  const pageUrl = location.href;
  // Keep the token out of the address bar and history.
  history.replaceState(null, "", location.pathname + location.search);

  function show(text, ok) {
    let box = document.getElementById("turbine-paired");
    if (!box) {
      box = document.createElement("div");
      box.id = "turbine-paired";
      box.setAttribute("role", "status");
      box.style.cssText =
        "position:fixed;left:50%;top:24px;transform:translateX(-50%);z-index:2147483647;padding:16px 28px;" +
        "background:#0B0A08;color:#EDC87A;border:2px solid #C99E52;border-radius:6px;font:700 24px/1.3 Barlow,'Segoe UI',Arial,sans-serif;" +
        "box-shadow:0 6px 24px rgba(0,0,0,.6);text-align:center;max-width:90vw;";
      document.documentElement.appendChild(box);
    }
    box.textContent = text;
    box.style.borderColor = ok ? "#C99E52" : "#C8553D";
    document.documentElement.dataset.turbinePaired = ok ? "yes" : "no";
  }

  chrome.runtime.sendMessage({ kind: "pair", pageUrl }).then(
    (res) => show(res && res.ok ? "Paired with Turbine. You can close this tab." : "Pairing did not work. Choose Pair again in Turbine.", !!(res && res.ok)),
    () => show("Pairing did not work. Reload this page or choose Pair again in Turbine.", false)
  );
})();
