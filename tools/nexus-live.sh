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
post() {   # post <label> <file with the JSON body> [strict]
  out=$(curl -sS --max-time 30 -w '\nHTTP %{http_code}' -H 'Content-Type: application/json' \
    -H 'Application-Name: TurbineModManager' -H 'Application-Version: 0.51' --data-binary "@$2" "$api" 2>&1)
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
# Other forms of Trending's date (said only, so a change on Nexus's side shows which form still works).
if [ -f "$dir/search-witcher3.json" ]; then
  day=$(grep -o '"updatedAt":\[{"value":"[^"]*"' "$dir/search-witcher3.json" | sed 's/.*"value":"//; s/"$//')
  if [ -n "$day" ]; then
    iso=$(date -u -d "@$day" +%Y-%m-%d 2>/dev/null || echo 1970-01-01)
    sed "s/\"value\":\"$day\"/\"value\":\"${iso}\"/" "$dir/search-witcher3.json" > /tmp/nexus-probe-day.json
    post "probe: updatedAt as a plain day" /tmp/nexus-probe-day.json
  fi
fi
# Search per game domain: each game's own mods.
for d in cyberpunk2077 kingdomcomedeliverance kingdomcomedeliverance2; do
  printf '%s' '{"query":"query($f: ModsFilter){ mods(filter: $f, count: 3) { totalCount nodes { modId name game { domainName } } } }","variables":{"f":{"gameDomainName":[{"value":"'"$d"'","op":"EQUALS"}]}}}' > /tmp/nexus-search-$d.json
  post "search $d" /tmp/nexus-search-$d.json
done
# The query names Nexus offers (for a mod's picture gallery, if it has one).
printf '%s' '{"query":"{ __schema { queryType { fields { name } } } }"}' > /tmp/nexus-schema.json
post "query names" /tmp/nexus-schema.json
# Pictures of a mod (for flipping on the mod page): the media query's arguments and what it returns.
printf '%s' '{"query":"{ __schema { queryType { fields { name args { name type { name kind ofType { name kind ofType { name } } } } type { name kind ofType { name } } } } } }"}' > /tmp/nexus-schema2.json
out=$(curl -sS --max-time 30 -H 'Content-Type: application/json' -H 'Application-Name: TurbineModManager' -H 'Application-Version: 0.51' --data-binary @/tmp/nexus-schema2.json "$api" 2>&1)
echo "== media query"
printf '%s' "$out" | grep -o '{"name":"media","args":\[[^]]*\][^}]*}[^}]*}' | head -c 2000; echo
for t in MediaSearchFilter MediaUnionPage ImageCategory; do
  printf '%s' '{"query":"{ __type(name: \"'"$t"'\") { kind fields { name type { name kind ofType { name } } } inputFields { name type { name kind ofType { name } } } enumValues { name } } }"}' > /tmp/nexus-type.json
  post "type $t" /tmp/nexus-type.json
done
exit $failed
