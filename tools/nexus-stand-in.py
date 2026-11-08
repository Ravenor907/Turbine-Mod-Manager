#!/usr/bin/env python3
"""A stand-in for a Nexus Mods file page, for the build's checks of Nexus inside Turbine. Not Nexus: nothing here talks
to Nexus or copies its pages; it only has the same moves a file page makes.

  python3 tools/nexus-stand-in.py --port 47800 --log standin.log --zip "<a mod zip>" --dupe-zip "<another mod zip>"

Serves on 127.0.0.1 only:
  /<domain>/mods/<id>?tab=files&file_id=<n>  a file page: "Slow download" and "Mod Manager Download" buttons that each
      wait 5 seconds after the press (the page reports the press to /log first), then go to the file
      (/files/TurbineTest-Bigger_Pockets-1.0.3.zip, sent as an attachment over about 3 seconds) or to an nxm:// link;
      a link to a setup program (refused by Turbine), a link to "Dupe Test 1.0.zip" (a name already in the game's
      folder), a link to the description page and a text field. Everything sits at a fixed place on the page.
  ...&auto=1  the same file page, which also tries everything by itself with no press (as an ad or a hostile page
      would): it goes to an nxm:// link from a script, a frame inside it goes to one and loads one as an image, it
      opens a pop-up and it starts a download. Turbine must take none of them.
  /frame  that frame.
  /<domain>/mods/<id>?tab=description  a second page with a link back.
  /files/<name>  the files above.
  /log?event=<what>  the page's own report of a press; /mark?name=<what>  a mark from the check.
Every request is logged with its time and the names of the cookies it carried. The first visit gets a cookie, so a
second Turbine run can show it kept it.
"""

import argparse
import html
import secrets
import sys
import threading
import time
import urllib.parse
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

LOCK = threading.Lock()


AUTO = """
  <iframe class="at" style="left:640px; top:300px; width:200px; height:120px; border:0;" src="/frame"></iframe>
<script>
  const soon = () => String(Math.floor(Date.now() / 1000) + 3600);
  setTimeout(() => location.assign("nxm://skyrimspecialedition/mods/1/files/88?key=abc&expires=" + soon() + "&user_id=1"), 1500);
  setTimeout(() => window.open("/skyrimspecialedition/mods/1?tab=description"), 2500);
  setTimeout(() => location.assign("/files/TurbineTest-Bigger_Pockets-1.0.3.zip"), 3500);
</script>"""

FRAME = """<!doctype html><html><body style="margin:0">
<img src="nxm://skyrimspecialedition/mods/1/files/66?key=abc&expires=4102444800&user_id=1" alt="">
<script>
  setTimeout(() => location.assign("nxm://skyrimspecialedition/mods/1/files/77?key=abc&expires=" + String(Math.floor(Date.now() / 1000) + 3600) + "&user_id=1"), 2000);
</script></body></html>"""


def page(domain, mod_id, file_id, auto=False):
    """The file page. Places are fixed (CSS pixels from the page's top left) so the check can put the pointer there."""
    nxm = f"nxm://{domain}/mods/{mod_id}/files/{file_id}?key=abc&expires=EXPIRES&user_id=1"
    return f"""<!doctype html>
<html><head><meta charset="utf-8"><title>Stand-in file page {mod_id}</title>
<style>
  body {{ margin: 0; font: 16px sans-serif; background: #f4f1ea; color: #222; }}
  .at {{ position: absolute; box-sizing: border-box; }}
  button.at {{ font-size: 18px; }}
  a.at {{ display: block; width: 220px; height: 24px; line-height: 24px; }}
</style></head>
<body>
  <h1 class="at" style="left:40px; top:20px; margin:0;">Stand-in file page: {html.escape(domain)} mod {mod_id}, file {file_id}</h1>
  <p class="at" style="left:40px; top:80px; margin:0;">The build's stand-in for a Nexus file page.</p>
  <button id="slow" class="at" style="left:40px; top:160px; width:240px; height:56px;">Slow download</button>
  <button id="mmd" class="at" style="left:320px; top:160px; width:280px; height:56px;">Mod Manager Download</button>
  <div id="wait" class="at" style="left:40px; top:236px;"></div>
  <a id="exe" class="at" style="left:40px; top:300px;" href="/files/setup.exe">Get the setup program</a>
  <a id="dupe" class="at" style="left:40px; top:360px;" href="/files/Dupe%20Test%201.0.zip">Download again</a>
  <a id="desc" class="at" style="left:40px; top:420px; width:140px;" href="/{domain}/mods/{mod_id}?tab=description">Description</a>
  <input id="field" class="at" style="left:40px; top:480px; width:300px; height:40px;" placeholder="Search">
<script>
  function wait(what, then) {{
    fetch("/log?event=" + what + "-pressed");
    let n = 5;
    const w = document.getElementById("wait");
    w.textContent = "Your download starts in " + n + " seconds";
    const t = setInterval(() => {{
      n -= 1;
      if (n > 0) {{ w.textContent = "Your download starts in " + n + " seconds"; return; }}
      clearInterval(t);
      w.textContent = "";
      then();
    }}, 1000);
  }}
  document.getElementById("slow").onclick = () => wait("slow", () => location.assign("/files/TurbineTest-Bigger_Pockets-1.0.3.zip"));
  document.getElementById("mmd").onclick = () => wait("mmd", () => location.assign("{nxm}".replace("EXPIRES", String(Math.floor(Date.now() / 1000) + 3600))));
</script>{AUTO if auto else ""}
</body></html>"""


def description(domain, mod_id):
    return f"""<!doctype html>
<html><head><meta charset="utf-8"><title>Stand-in description {mod_id}</title></head>
<body style="margin:0; font:16px sans-serif;">
  <h1 style="margin:20px 40px;">Description of mod {mod_id}</h1>
  <p style="margin:0 40px;"><a href="/{domain}/mods/{mod_id}?tab=files">Files</a></p>
</body></html>"""


class Handler(BaseHTTPRequestHandler):
    server_version = "StandIn/1"

    def log_message(self, fmt, *args):
        pass

    def note(self, extra=""):
        cookies = self.headers.get("Cookie", "")
        names = sorted({c.split("=", 1)[0].strip() for c in cookies.split(";") if "=" in c})
        line = f"{time.time():.3f} GET {self.path} cookies={','.join(names) or '-'}{(' ' + extra) if extra else ''}\n"
        with LOCK:
            with open(self.server.log_path, "a", encoding="utf-8") as f:
                f.write(line)

    def send_body(self, code, body, ctype="text/html; charset=utf-8", extra=None):
        data = body.encode("utf-8") if isinstance(body, str) else body
        self.send_response(code)
        self.send_header("Content-Type", ctype)
        self.send_header("Content-Length", str(len(data)))
        self.send_header("Cache-Control", "no-store")
        if "standin_session=" not in self.headers.get("Cookie", ""):
            self.send_header("Set-Cookie", f"standin_session={secrets.token_hex(8)}; Path=/; Max-Age=86400; SameSite=Lax")
        for k, v in (extra or {}).items():
            self.send_header(k, v)
        self.end_headers()
        return data

    def do_GET(self):
        self.note()
        url = urllib.parse.urlsplit(self.path)
        q = urllib.parse.parse_qs(url.query)
        parts = [p for p in url.path.split("/") if p]
        if url.path in ("/log", "/mark"):
            self.wfile.write(self.send_body(204, b""))
            return
        if url.path == "/frame":
            self.wfile.write(self.send_body(200, FRAME))
            return
        if len(parts) >= 3 and parts[1] == "mods" and parts[2].isdigit():
            domain, mod_id = parts[0], int(parts[2])
            if q.get("tab", [""])[0] == "description":
                self.wfile.write(self.send_body(200, description(domain, mod_id)))
            else:
                file_id = int(q.get("file_id", ["2"])[0] or 2) if q.get("file_id", ["2"])[0].isdigit() else 2
                self.wfile.write(self.send_body(200, page(domain, mod_id, file_id, q.get("auto", [""])[0] == "1")))
            return
        if len(parts) == 2 and parts[0] == "files":
            name = urllib.parse.unquote(parts[1])
            if name.endswith(".exe"):
                data = b"MZ" + b"\0" * 1022
                ctype = "application/octet-stream"
                secs = 0.0
            elif name in ("TurbineTest-Bigger_Pockets-1.0.3.zip", "Dupe Test 1.0.zip"):
                data = self.server.zip_bytes if name.startswith("TurbineTest") else self.server.dupe_bytes
                ctype = "application/zip"
                secs = 3.0 if name.startswith("TurbineTest") else 4.0
            else:
                self.wfile.write(self.send_body(404, "not here", "text/plain"))
                return
            head = self.send_body(200, data, ctype, {"Content-Disposition": f'attachment; filename="{name}"'})
            # Sent in pieces over a few seconds, so the Downloads tab can show one running.
            pieces = 20
            step = max(1, len(head) // pieces)
            for i in range(0, len(head), step):
                self.wfile.write(head[i:i + step])
                self.wfile.flush()
                time.sleep(secs / pieces)
            self.note(f"sent {name} ({len(head)} bytes)")
            return
        if url.path == "/":
            self.wfile.write(self.send_body(200, "<!doctype html><title>Stand-in</title><p>Stand-in</p>"))
            return
        self.wfile.write(self.send_body(404, "not here", "text/plain"))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--port", type=int, default=47800)
    ap.add_argument("--log", required=True)
    ap.add_argument("--zip", required=True)
    ap.add_argument("--dupe-zip", required=True)
    a = ap.parse_args()
    srv = ThreadingHTTPServer(("127.0.0.1", a.port), Handler)
    srv.log_path = a.log
    with open(a.zip, "rb") as f:
        srv.zip_bytes = f.read()
    with open(a.dupe_zip, "rb") as f:
        srv.dupe_bytes = f.read()
    open(a.log, "a").close()
    print(f"stand-in on http://127.0.0.1:{a.port}/", flush=True)
    try:
        srv.serve_forever()
    except KeyboardInterrupt:
        pass
    return 0


if __name__ == "__main__":
    sys.exit(main())
