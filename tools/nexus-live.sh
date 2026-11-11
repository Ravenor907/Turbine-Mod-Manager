#!/bin/sh
# Sends Nexus Mods' public GraphQL (v2) the exact requests Turbine sends, with no API key (the bodies come from the
# engine: engine/tests/complexity.rs writes them to $1), and prints what came back, so the build log shows whether
# keyless browsing, the mod page, requirements, files and update checks work today. A GraphQL "errors" answer to one
# of Turbine's own requests (complexity, an unknown field, a filter Nexus can't read) fails the build, because every
# player would get the same answer; Nexus being down or slow (no answer at all) is only a warning.
set -u
failed=0
dir=${1:-/tmp/nexus-queries}
api=https://api.nexusmods.com/v2/graphql
version=$(cat "$(dirname "$0")/../VERSION")
post() {   # post <label> <file with the JSON body> [strict]
  out=$(curl -sS --max-time 30 -w '\nHTTP %{http_code}' -H 'Content-Type: application/json' \
    -H 'Application-Name: TurbineModManager' -H "Application-Version: $version" --data-binary "@$2" "$api" 2>&1)
  code=$(printf '%s' "$out" | tail -n1)
  body=$(printf '%s' "$out" | sed '$d')
  echo "== $1 ($code)"
  printf '%s\n' "$body" | cut -c1-3000
  case "$body" in
    *'"errors"'*)
      if [ "${3:-}" = strict ]; then
        echo "::error title=Nexus refused Turbine's request $1::$(printf '%s' "$body" | grep -o '"message":"[^"]*' | head -n1 | cut -c1-300)"
        failed=1
      else
        echo "::warning title=Nexus refused $1::$(printf '%s' "$body" | grep -o '"message":"[^"]*' | head -n1 | cut -c1-300)"
      fi ;;
    *'"totalCount":0,'*)
      # A search of Turbine's that finds nothing (a filter Nexus reads but matches nothing with) fails too.
      case "$1" in
        search*) if [ "${3:-}" = strict ]; then echo "::error title=Nexus found nothing for Turbine's $1::$code"; failed=1; else echo "::warning title=Nexus found nothing for $1::$code"; fi ;;
        *) echo "::notice title=Nexus answered $1 without a key::$code" ;;
      esac ;;
    *'"data"'*) echo "::notice title=Nexus answered $1 without a key::$code" ;;
    *) echo "::warning title=Nexus didn't answer $1::$code" ;;
  esac
}
n=0
for f in "$dir"/*.json; do
  [ -f "$f" ] || continue
  n=$((n + 1))
  post "$(basename "$f" .json)" "$f" strict
done
if [ "$n" -eq 0 ]; then echo "::error title=Nexus live check::No request bodies from the engine in $dir"; failed=1; fi
# Nexus's public game list (Mods for any game matches names against it). A warning when it is gone: Turbine
# keeps its cached copy.
code=$(curl -sS --max-time 60 -o /tmp/nexus-games.json -w '%{http_code}' -H 'Application-Name: TurbineModManager' \
  -H "Application-Version: $version" https://data.nexusmods.com/file/nexus-data/games.json 2>/dev/null || echo 000)
if [ "$code" = 200 ] && grep -q '"domain_name"' /tmp/nexus-games.json; then
  echo "::notice title=Nexus game list::$(grep -o '"domain_name"' /tmp/nexus-games.json | wc -l) games, $(wc -c < /tmp/nexus-games.json) bytes"
else
  echo "::warning title=Nexus game list::HTTP $code"
fi
exit $failed
