#!/bin/sh
# Asks Nexus Mods' public GraphQL (v2) what Turbine asks it, with no API key: a search per game domain (each game's
# own mods), the fields Get mods, the mod page and the update check read, and the schema they rely on. Prints what
# came back so the build log shows whether keyless browsing works today. Never fails the build by itself.
set -u
api=https://api.nexusmods.com/v2/graphql
ask() {   # ask <label> <json body>
  out=$(curl -sS --max-time 30 -w '\nHTTP %{http_code}' -H 'Content-Type: application/json' \
    -H 'Application-Name: TurbineModManager' -H 'Application-Version: 0.51' -d "$2" "$api" 2>&1)
  code=$(printf '%s' "$out" | tail -n1)
  body=$(printf '%s' "$out" | sed '$d')
  echo "== $1 ($code)"
  printf '%s\n' "$body" | cut -c1-6000
}
for d in skyrimspecialedition cyberpunk2077 witcher3 kingdomcomedeliverance kingdomcomedeliverance2; do
  ask "search $d" '{"query":"query($f: ModsFilter){ mods(filter: $f, count: 3, sort: [{endorsements:{direction:DESC}}]) { totalCount nodes { modId name game { domainName } version updatedAt thumbnailUrl } } }","variables":{"f":{"op":"AND","gameDomainName":[{"value":"'"$d"'","op":"EQUALS"}]}}}'
done
ask "query fields" '{"query":"{ __schema { queryType { fields { name args { name type { name kind ofType { name kind ofType { name } } } } } } } }"}'
ask "ModsFilter" '{"query":"{ __type(name: \"ModsFilter\") { inputFields { name type { name kind ofType { name } } } } }"}'
ask "Mod" '{"query":"{ __type(name: \"Mod\") { fields { name type { name kind ofType { name kind ofType { name } } } } } }"}'
ask "ModsSort" '{"query":"{ __type(name: \"ModsSort\") { inputFields { name } } }"}'
ask "Game" '{"query":"{ __type(name: \"Game\") { fields { name type { name kind ofType { name } } } } }"}'
ask "Category" '{"query":"{ __type(name: \"Category\") { fields { name type { name kind ofType { name } } } } }"}'
ask "ModPage" '{"query":"{ __type(name: \"ModPage\") { fields { name type { name kind ofType { name } } } } }"}'
ask "ModFile" '{"query":"{ __type(name: \"ModFile\") { fields { name type { name kind ofType { name } } } } }"}'
ask "legacyModsByDomain" '{"query":"{ legacyModsByDomain(ids: [{gameDomain: \"skyrimspecialedition\", modId: 32444}, {gameDomain: \"skyrimspecialedition\", modId: 12604}]) { nodes { modId name version updatedAt } } }"}'
ask "categories" '{"query":"{ categories(gameId: 1704) { id name } }"}'
ask "facets" '{"query":"query($f: ModsFilter){ mods(filter: $f, count: 1, facets: {categoryName: []}) { facetsData } }","variables":{"f":{"gameDomainName":[{"value":"skyrimspecialedition","op":"EQUALS"}]}}}'
ask "mod pictures" '{"query":"{ mod(gameId: 1704, modId: 3863) { name pictureUrl thumbnailUrl modCategory { name } } }"}'
exit 0
