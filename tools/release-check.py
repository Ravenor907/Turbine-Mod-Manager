#!/usr/bin/env python3
"""0.6: the release rules in .github/workflows/build.yml, checked on every push (the release itself runs rarely).

A release ships exactly what the green runs of its commit built and checked: the Flatpak and the Decky plugin from
the reused run, each with its checksum, after the green full builds a release needs (REQUIRED_GREEN_BUILDS); the plugin's packages are pinned by its lockfile."""
import glob
import re
import sys

wf = open(".github/workflows/build.yml").read()
problems = []

def need(text, why):
    if text not in wf:
        problems.append(why)

need("pnpm install --frozen-lockfile", "the Decky job must install from the committed lockfile")
if "--no-frozen-lockfile" in wf:
    problems.append("the Decky job may not resolve new package versions")
# 0.63 (owner): one green full build of the commit, a single named number at the top of the workflow.
REQUIRED_GREEN_BUILDS = 1
need(f"\nenv:\n  REQUIRED_GREEN_BUILDS: {REQUIRED_GREEN_BUILDS}\n", f"the workflow must set REQUIRED_GREEN_BUILDS to {REQUIRED_GREEN_BUILDS} near its top")
if wf.count("REQUIRED_GREEN_BUILDS:") != 1:
    problems.append("REQUIRED_GREEN_BUILDS must be set in exactly one place")
need('if [ "$greens" -lt "$REQUIRED_GREEN_BUILDS" ]', "a release needs REQUIRED_GREEN_BUILDS green full builds of its commit")
# 0.62: an attempt made by "Re-run failed jobs" carries the passed jobs over; it is not a green run of everything.
need('select(.run_attempt != $a)', "only full attempts may count as green builds (not re-runs of the failed jobs alone)")
if 'carried' in wf and wf.index('[ "$carried" = 0 ] || continue') > wf.index('greens=$((greens + 1))'):
    problems.append("the full-attempt check must come before a green run is counted")
rel = wf[wf.index("  release:"):]
# Both files come from the reused green run: one download step by their exact artifact ids, and the reuse
# step looks up an id for each of the two files.
if "run-id: ${{ needs.reuse.outputs.run }}" not in rel or "artifact-ids: ${{ needs.reuse.outputs.ids }}" not in rel:
    problems.append("the release must take both the Flatpak and the Decky zip from the reused green run")
need("for name in TurbineModManager.flatpak Turbine-Decky.zip; do", "the reuse step must find both the Flatpak and the Decky zip")
if "No checksum beside" not in rel or "exit 1" not in rel.split("No checksum beside")[1][:80]:
    problems.append("a missing checksum must stop the release")
if "::warning title=No checksum" in rel:
    problems.append("a missing checksum only warns")
# 0.64: a commit that changes only the paths a push ignores reuses the green build of the commit before it. The walk
# back must ignore exactly what the push trigger ignores (and nothing else), and must run before the green runs are
# counted.
top = wf[:wf.index("jobs:")]
ignored = re.findall(r'^\s+- "([^"]+)"', top[top.index("paths-ignore:"):top.index("workflow_dispatch:")], re.M) if "paths-ignore:" in top else []
if ignored != ["site/**", "README.md", "**/*.md", ".github/workflows/pages.yml"]:
    problems.append(f"the push trigger's paths-ignore changed ({ignored}): change the release's walk back over ignored commits (its case line) to match, then this list")
walk = 'case "$f" in site/*|*.md|.github/workflows/pages.yml) ;; *) only_ignored=0 ;; esac'
if walk not in wf:
    problems.append("the release must walk back over commits that change only ignored paths (site/, *.md, pages.yml) to the build they reuse")
elif wf.index(walk) > wf.index("greens=0"):
    problems.append("the walk back over ignored commits must come before the green builds are counted")
if 'sha="$built"' not in wf:
    problems.append("the release must look for green builds of the commit the walk back found")
# 0.63: Turbine's own update check reads the checksum file beside the bundle (it no longer asks GitHub's API, whose
# hourly limit refused it), so the release must publish both checksum files.
for f in ("TurbineModManager.flatpak.sha256", "Turbine-Decky.zip.sha256"):
    if ("\n            " + f + "\n") not in rel:
        problems.append(f"the release must publish {f} beside its file: Turbine's updater checks the download against it")
decky = wf[wf.index("  decky:"):wf.index("  vendor:")]
if "needs.reuse.outputs.run == ''" not in decky:
    problems.append("the Decky job must not build again for a release that reuses a green run")

# 0.62: the app can't be built everywhere this script runs, and a function declared for the interface with no code
# behind it only showed in the Flatpak build (olderSeamlessAnswer). Every #[qinvokable] declared in the bridge
# (app/src/backend.rs) needs a function of the same name with a body somewhere in app/src, taking the same number of
# arguments and the same kind of self.

def params(text):
    """The parameter list of `fn name(...)` starting at text[0] == '(' (nesting counted), split at top level."""
    depth, out, cur = 0, [], ""
    for ch in text:
        if ch in "(<[":
            depth += 1
            if depth == 1 and ch == "(":
                continue
        elif ch in ")>]":
            depth -= 1
            if depth == 0 and ch == ")":
                break
        if ch == "," and depth == 1:
            out.append(cur.strip())
            cur = ""
        else:
            cur += ch
    if cur.strip():
        out.append(cur.strip())
    return out

bridge = open("app/src/backend.rs").read()
sources = {f: open(f).read() for f in sorted(glob.glob("app/src/*.rs"))}
declared = re.findall(r"#\[qinvokable\]\s*(?:#\[[^\]]*\]\s*)*fn\s+([a-z0-9_]+)\s*(\([^;{]*?\))\s*(?:->[^;{]*)?;", bridge)
if len(declared) < 50:
    problems.append(f"the bridge check found only {len(declared)} invokables in app/src/backend.rs: its pattern no longer fits")
pinned = lambda ps: bool(ps) and "Pin<&mut Self>" in ps[0]
for name, sig in declared:
    want = params(sig)
    shapes = []
    for f, text in sources.items():
        for m in re.finditer(r"\bfn\s+" + name + r"\s*(\([^;{]*?\))\s*(?:->[^;{]*)?\{", text):
            got = params(m.group(1))
            # Other things in the app may carry the same name (a free function, another type's method): only a
            # function that takes self counts.
            if got and "self" in got[0]:
                shapes.append((len(got), pinned(got), f))
    if not shapes:
        problems.append(f"{name} is declared for the interface in app/src/backend.rs but no function of that name exists in app/src")
    elif not any(n == len(want) and pin == pinned(want) for n, pin, _ in shapes):
        n, pin, f = shapes[0]
        problems.append(f"{name}: the bridge declares {len(want) - 1} arguments on {'Pin<&mut Self>' if pinned(want) else '&Self'}, the function in {f} takes {n - 1} on {'Pin<&mut Self>' if pin else '&self'}")

# And the other way round: every Turbine.<name>(...) the interface calls is declared in the bridge.
bridge_names = set(re.findall(r'#\[cxx_name\s*=\s*"([A-Za-z0-9_]+)"\]', bridge)) | set(re.findall(r"\bfn\s+([a-z0-9_]+)\s*\(", bridge))
camel = lambda n: re.sub(r"_([a-z0-9])", lambda m: m.group(1).upper(), n)
bridge_names |= {camel(n) for n in bridge_names}
for f in sorted(glob.glob("app/qml/*.qml")):
    for name in sorted(set(re.findall(r"\bTurbine\.([A-Za-z0-9_]+)\s*\(", open(f).read())) - bridge_names):
        problems.append(f"{f} calls Turbine.{name}(...), which the bridge in app/src/backend.rs doesn't declare")

for p in problems:
    print(f"::error title=Release rules::{p}")
print(f"Release rules: {len(problems)} problems")
sys.exit(1 if problems else 0)
