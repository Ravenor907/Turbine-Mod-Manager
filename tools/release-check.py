#!/usr/bin/env python3
"""0.6: the release rules in .github/workflows/build.yml, checked on every push (the release itself runs rarely).

A release ships exactly what the green runs of its commit built and checked: the Flatpak and the Decky plugin from
the reused run, each with its checksum, after three green runs; the plugin's packages are pinned by its lockfile."""
import sys

wf = open(".github/workflows/build.yml").read()
problems = []

def need(text, why):
    if text not in wf:
        problems.append(why)

need("pnpm install --frozen-lockfile", "the Decky job must install from the committed lockfile")
if "--no-frozen-lockfile" in wf:
    problems.append("the Decky job may not resolve new package versions")
need('if [ "$greens" -lt 3 ]', "a release needs three green runs of its commit")
need("Turbine-Decky.zip.sha256", "the Decky zip needs its checksum beside it")
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
decky = wf[wf.index("  decky:"):wf.index("  vendor:")]
if "needs.reuse.outputs.run == ''" not in decky:
    problems.append("the Decky job must not build again for a release that reuses a green run")

for p in problems:
    print(f"::error title=Release rules::{p}")
print(f"Release rules: {len(problems)} problems")
sys.exit(1 if problems else 0)
