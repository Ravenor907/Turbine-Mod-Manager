#!/bin/sh
# Sends Nexus Mods' public GraphQL (v2) the exact requests Turbine sends, with no API key (the bodies come from the
# engine: engine/tests/complexity.rs writes them to $1), and prints what came back, so the build log shows whether
# keyless browsing, the mod page, requirements, files and update checks work today. A refused query (complexity,
# unknown field) is said as a warning. Never fails the build by itself.
set -u
dir=${1:-/tmp/nexus-queries}
api=https://api.nexusmods.com/v2/graphql
post() {   # post <label> <file with the JSON body>
  out=$(curl -sS --max-time 30 -w '\nHTTP %{http_code}' -H 'Content-Type: application/json' \
    -H 'Application-Name: TurbineModManager' -H 'Application-Version: 0.51' --data-binary "@$2" "$api" 2>&1)
  code=$(printf '%s' "$out" | tail -n1)
  body=$(printf '%s' "$out" | sed '$d')
  echo "== $1 ($code)"
  printf '%s\n' "$body" | cut -c1-3000
  case "$body" in
    *'"errors"'*) echo "::warning title=Nexus refused $1::$(printf '%s' "$body" | grep -o '"message":"[^"]*"' | head -n1)" ;;
    *'"data"'*) echo "::notice title=Nexus answered $1 without a key::$code" ;;
    *) echo "::warning title=Nexus didn't answer $1::$code" ;;
  esac
}
for f in "$dir"/*.json; do
  post "$(basename "$f" .json)" "$f"
done
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
for t in Media MediaPage Image ImageType MediaFilter; do
  printf '%s' '{"query":"{ __type(name: \"'"$t"'\") { kind fields { name type { name kind ofType { name } } } inputFields { name type { name kind ofType { name } } } enumValues { name } } }"}' > /tmp/nexus-type.json
  post "type $t" /tmp/nexus-type.json
done
exit 0
