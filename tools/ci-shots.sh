#!/bin/bash
# The build's screenshot and log checks, one part per CI job so they run side by side:
#   tools/ci-shots.sh 1280x800 | 1920x1080 | x11 | nexus
# Each part installs the Flatpak bundle the build made (TurbineModManager.flatpak in the working folder), takes its
# pictures (printed into the log as "SHOT <name> <base64>" too) and fails on any check that fails.
set -eo pipefail
MODE=${1:?"tools/ci-shots.sh 1280x800|1920x1080|x11|nexus [shard shards]"}
# 0.6: the pictures of one size split over parallel jobs: this job takes its share (shard of shards). Each picture
# goes to a shard by its run time (tools/ci-shards.txt, measured; a guess by its kind otherwise), the longest first
# onto the least loaded shard, the same way in every job; checks only read pictures their own shard took.
SHARD=${2:-0}
SHARDS=${3:-1}
case "$MODE" in
  1280x800|1920x1080) SIZES=$MODE ;;
  x11|nexus) SIZES="" ;;
  *) echo "unknown part $MODE"; exit 2 ;;
esac
flatpak install --user -y --noninteractive --reinstall TurbineModManager.flatpak > /dev/null
# X11 checks need Xvfb in the build container.
if { [ "$MODE" = x11 ] || [ "$MODE" = nexus ]; } && ! command -v Xvfb > /dev/null; then
  (dnf install -y xorg-x11-server-Xvfb xprop > /dev/null 2>&1 || (apt-get update > /dev/null 2>&1 && apt-get install -y xvfb x11-utils > /dev/null 2>&1)) || true
fi
if [ "$MODE" = x11 ]; then
# Qt WebEngine is inside the app (from the BaseApp), with its helper process.
if web=$(flatpak run --command=sh io.github.ravenor907.TurbineModManager -c 'ls /app/lib/libQt6WebEngineQuick.so* /app/bin/QtWebEngineProcess && ls -d /app/lib/qml/QtWebEngine' 2>&1); then
  echo "::notice title=Qt WebEngine in the Flatpak::$(echo "$web" | tr '\n' ' ' | cut -c1-240)"
else
  echo "::error title=Qt WebEngine missing from the Flatpak::$web"; exit 1
fi
# The programs the engine runs from the runtime: curl (downloads) and bsdtar (7z and rar).
for tool in curl bsdtar; do
  if v=$(flatpak run --command=$tool io.github.ravenor907.TurbineModManager --version 2>&1 | head -n1); then
    echo "::notice title=$tool in the Flatpak::$v"
  else
    echo "::error title=$tool missing from the Flatpak::$v"; exit 1
  fi
done
# openssl carries Nexus single sign-on (app/src/nexus_sso.rs). Without it, sign-in stays on the browser helper.
if v=$(flatpak run --command=openssl io.github.ravenor907.TurbineModManager version 2>&1 | head -n1); then
  echo "::notice title=openssl in the Flatpak::$v"
else
  echo "::warning title=openssl missing from the Flatpak (Nexus single sign-on can't turn on)::$v"
fi
fi
mkdir -p shots home empty data samples
python3 tools/fake-home.py "$PWD/home"
# Recorded answers instead of the internet: the engine's samples, then the app's own.
cp engine/tests/data/*.json engine/tests/data/*.html samples/
cp tools/selftest-data/* samples/
fail=0
hung=0
# The shard plan: every picture name this run can take, with its weight, from a dry pass over this script.
guess() {   # seconds a picture takes, by its kind
  case "$1" in
    inputcheck*) echo 70 ;;
    browsertest*|browserpersist*|regress*) echo 200 ;;
    controls*|legendcheck*|sheetcheck*|presscount*|nexusguard*|inlinebrowser*|sizeinputs*|coopcheck*|editbutton*|heldcheck*|mmdguard*|passthrough*|golden*|abcheck*|barbuttons*|findfrom*|menufit*|startcheck*|bookcheck*|cooppw*|keycheck*) echo 28 ;;
    findpop*|fullscroll*|listbar*|ac*|modsettings*|coopsettings*|gameupdate*|ersaves*) echo 19 ;;
    loot*|nxm*|getmodsback*) echo 13 ;;
    *) echo 9 ;;
  esac
}
declare -A ASSIGN
if [ "$SHARDS" -gt 1 ]; then
  names=$(grep -oE '^\s*(SHOOT_ENV="[^"]*" )?shoot "[^"]+"' "$0" | sed -E 's/.*shoot "([^"]+)"/\1/' | sed -E 's/\$size/'"$MODE"'/g; s/\$\{p\/modpage\/modpage-\}/modpage-x/g' | grep -v "$( [ "$MODE" = 1280x800 ] && echo 1920x1080 || echo 1280x800 )" | grep -v '\$' | sort -u)
  declare -A W
  for n in $names; do W[$n]=$(guess "$n"); done
  if [ -f tools/ci-shards.txt ]; then while read -r n w; do [ -n "$n" ] && [ "${n:0:1}" != "#" ] && W[$n]=$w; done < tools/ci-shards.txt; fi
  declare -a LOAD; for ((k = 0; k < SHARDS; k++)); do LOAD[$k]=0; done
  for n in $(for k in "${!W[@]}"; do echo "${W[$k]} $k"; done | sort -k1,1nr -k2,2 | awk '{print $2}'); do
    best=0; for ((k = 1; k < SHARDS; k++)); do [ "${LOAD[$k]}" -lt "${LOAD[$best]}" ] && best=$k; done
    ASSIGN[$n]=$best; LOAD[$best]=$(( LOAD[$best] + W[$n] ))
  done
  echo "Shard $SHARD of $SHARDS: planned load ${LOAD[$SHARD]} s of $(( $(IFS=+; echo "${LOAD[*]}") )) s"
fi
mine() {   # is this picture this shard's?
  [ "$SHARDS" -le 1 ] && return 0
  local b=${ASSIGN[$1]:-}
  [ -z "$b" ] && b=$(( $(printf %s "$1" | cksum | cut -d' ' -f1) % SHARDS ))
  [ "$b" = "$SHARD" ]
}
RAN=" "
ran() { [[ "$RAN" == *" $1 "* ]]; }
shoot() {   # shoot <name> <WxH> <page> <home> [more arguments]
  name=$1 size=$2 page=$3 home=$4
  shift 4
  mine "$name" || return 0
  RAN="$RAN$name "
  local t0=$SECONDS
  out="$PWD/shots/$name.png"
  # Each picture gets its own mod store, so imports never pile up between pictures.
  timeout 60 dbus-run-session -- flatpak run --env=QT_QPA_PLATFORM=offscreen --env=QT_QUICK_BACKEND=software $SHOOT_ENV \
    --env=TURBINE_HOME="$home" --env=TURBINE_DATA="$PWD/data/$name" \
    --filesystem="$PWD/shots" --filesystem="$home" --filesystem="$PWD/data" --filesystem="$PWD/samples:ro" \
    io.github.ravenor907.TurbineModManager --screenshot="$out" --size="$size" --page="$page" --selftest-data="$PWD/samples" "$@" > "shots/$name.log" 2>&1 \
    || { code=$?; echo "::warning title=Turbine stopped with code $code::$name"; [ $code -eq 124 ] && hung=$((hung + 1)); }
  if [ ! -s "$out" ]; then echo "::error title=No screenshot::$name"; cat "shots/$name.log"; tail -n 40 "$PWD/data/$name/turbine-mod-manager/activity.log" 2>/dev/null || true; fail=1; fi
  # Three pictures that never came: stop here rather than wait a minute for every other one.
  if [ "$hung" -ge 3 ]; then echo "::error title=Turbine hangs::three pictures timed out"; exit 1; fi
  if grep -E "qrc:|\.qml:[0-9]+|TypeError|ReferenceError" "shots/$name.log"; then echo "::error title=QML problem::$name"; fail=1; fi
  # The engine answers "not ready" while parts of it are unfinished; say so without failing the build.
  if grep -q "not yet implemented" "shots/$name.log"; then echo "::warning title=Engine not finished::$name"; fi
  echo "SHOTTIME $name $(( SECONDS - t0 ))"
}
need() {   # need <shot> <title> <grep -E pattern>
  ran "$1" || return 0
  if grep -Eq "$3" "$PWD/data/$1/turbine-mod-manager/activity.log" 2>/dev/null; then echo "::notice title=$2::$(grep -Eh "$3" "$PWD/data/$1/turbine-mod-manager/activity.log" | head -n1 | cut -c1-200)"
  else echo "::error title=$2::not found: $3"; (grep -h "check\|shot\|Sheet\|focus\|Rail " "$PWD/data/$1/turbine-mod-manager/activity.log" 2>/dev/null | tail -n 60 || true); fail=1; fi
}
dl="$PWD/home/Downloads"
skyrim=steam:489830
# A copy of the home for pictures whose mod store sits in the game's library.
fresh() { rm -rf "$PWD/fh/$1" && mkdir -p "$PWD/fh" && cp -a "$PWD/home" "$PWD/fh/$1"; }
for size in $SIZES; do
  for page in game settings details perf notice; do shoot "$page-$size" "$size" "$page" "$PWD/home"; done
  shoot "nogames-$size" "$size" game "$PWD/empty"
  shoot "mods-skyrim-$size" "$size" mods "$PWD/home" --game=$skyrim \
    "--selftest-import=$dl/Brighter Lanterns 1.2.zip,$dl/Lantern Glow SKSE 2.0.zip"
  shoot "mods-valheim-$size" "$size" mods "$PWD/home" --game=steam:892970 "--selftest-import=$dl/TurbineTest-Bigger_Pockets-1.0.3.zip"
  shoot "mods-stardew-$size" "$size" mods "$PWD/home" --game=gog:1453375253 "--selftest-import=$dl/Quiet Rooster 1.4.zip"
  shoot "menu-$size" "$size" menu "$PWD/home" --game=$skyrim "--selftest-import=$dl/Brighter Lanterns 1.2.zip"
  shoot "gamemenu-$size" "$size" gamemenu "$PWD/home" --game=$skyrim
  shoot "check-$size" "$size" check "$PWD/home" --game=$skyrim "--selftest-import=$dl/Lantern Glow SKSE 2.0.zip"
  shoot "add-$size" "$size" add "$PWD/home" --game=$skyrim
  shoot "loader-$size" "$size" loader "$PWD/home" --game=$skyrim "--selftest-import=$dl/Lantern Glow SKSE 2.0.zip"
  # Labs mode and Seasonal (neither is saved from a picture).
  for page in about labs perfcheck; do shoot "$page-$size" "$size" "$page" "$PWD/home"; done
  # 0.54: Labs > Activity log (after a game page's worth of lines).
  shoot "activity-$size" "$size" activity "$PWD/home" --game=$skyrim
  need "activity-$size" "0.58: A on Settings, Labs, Activity log opens it ($size)" "Activity log sheet open after A on its Settings row: true"
  need "activity-$size" "0.58: every Settings action row has an action ($size)" "Settings rows check: [0-9]+ action rows, 0 without an action"
  shoot "report-$size" "$size" report "$PWD/home" --game=$skyrim
  shoot "autumn-$size" "$size" mods "$PWD/home" --game=steam:892970 --season=autumn "--selftest-import=$dl/TurbineTest-Bigger_Pockets-1.0.3.zip"
  shoot "snow-$size" "$size" settings "$PWD/home" --season=snow
  # 0.3: Get Mods, details, installer choices, load order, profiles, accounts, updates.
  lethal=steam:1966720
  shoot "browse-$size" "$size" browse "$PWD/home" --game=$lethal
  shoot "moddetails-$size" "$size" moddetails "$PWD/home" --game=$lethal
  shoot "fomod-$size" "$size" fomod "$PWD/home" --game=$skyrim "--selftest-import=$dl/Lush Meadows 1.0.zip"
  shoot "loadorder-$size" "$size" loadorder "$PWD/home" --game=$skyrim
  shoot "profiles-$size" "$size" profiles "$PWD/home" --game=$skyrim
  shoot "accounts-$size" "$size" accounts "$PWD/home"
  shoot "updates-$size" "$size" updates "$PWD/home"
  # LOOT sorting with the recorded sample masterlist (on a copy of the home, since sorting writes plugins.txt).
  for page in lootsort lootnotes; do
    rm -rf "$PWD/lhome" && cp -a "$PWD/home" "$PWD/lhome"
    shoot "$page-$size" "$size" $page "$PWD/lhome" --game=$skyrim
  done
  shoot "versions-$size" "$size" versions "$PWD/home" --game=$lethal
  # Nexus: requirements ("Have it" from a mod recorded as Address Library), other files, a free account, a link.
  have="--selftest-import=$dl/Brighter Lanterns 1.2.zip|nexus|skyrimspecialedition:32444:1|1.0"
  shoot "nexusdetails-$size" "$size" nexusdetails "$PWD/home" --game=$skyrim "$have"
  shoot "nexusversions-$size" "$size" nexusversions "$PWD/home" --game=$skyrim
  shoot "nexusfree-$size" "$size" nexusfree "$PWD/home" --game=$skyrim "$have"
  shoot "nxm-$size" "$size" nxm "$PWD/home" --game=$skyrim \
    "nxm://skyrimspecialedition/mods/12604/files/200?key=abc123&expires=4102444800&user_id=1234567"
  shoot "code-$size" "$size" code "$PWD/home" --game=$lethal \
    "--selftest-import=$dl/TurbineTest-Bigger_Pockets-1.0.3.zip|thunderstore|TurbineTest-Bigger_Pockets|1.0.3"
  # The browser: Settings > Accounts > Browser, the no-browser warning, the helper's steps, the status card.
  for page in browser nobrowser helper overlay overlaygm addbrowser; do shoot "$page-$size" "$size" $page "$PWD/home"; done
  # 0.55: Settings, Getting mods in Game Mode (Steam's Game Mode is what SteamGamepadUI=1 says).
  SHOOT_ENV="--env=SteamGamepadUI=1" shoot "accountsgm-$size" "$size" accountsgm "$PWD/home"
  need "accountsgm-$size" "R5: Game Mode's Getting mods: signed in or not first, page size, faster downloads; no own-browser rows ($size)" "Getting mods rows \(Game Mode true\): nexusSign, zoom, (keyFold|pasteKey, keyPage, nexusKey, keyWhat, signOut)$"
  if grep -Eq "Getting mods rows \(Game Mode true\):.*(, helper|, nxmHandler|, overlay|, browser(,|$))" "$PWD/data/accountsgm-$size/turbine-mod-manager/activity.log" 2>/dev/null; then
    echo "::error title=10: own-browser rows aren't folded in Game Mode ($size)::"; fail=1; fi
  # Cyberpunk 2077 and The Witcher 3: their mods and the check before Play; install notes after adding.
  cp2077=steam:1091500
  w3=steam:292030
  shoot "mods-cyberpunk-$size" "$size" mods "$PWD/home" --game=$cp2077 "--selftest-import=$dl/Quiet Streets CET 1.0.zip,$dl/Neon Signs REDmod 1.1.zip"
  shoot "check-cyberpunk-$size" "$size" check "$PWD/home" --game=$cp2077 "--selftest-import=$dl/Quiet Streets CET 1.0.zip,$dl/Neon Signs REDmod 1.1.zip"
  # The end of the check, scrolled there by wheel steps (REDmod's --launcher-skip in a fixed-width font).
  shoot "checkend-cyberpunk-$size" "$size" checkend "$PWD/home" --game=$cp2077 "--selftest-import=$dl/Quiet Streets CET 1.0.zip,$dl/Neon Signs REDmod 1.1.zip"
  shoot "check-witcher-$size" "$size" check "$PWD/home" --game=$w3 "--selftest-import=$dl/Better Torches 1.0.zip,$dl/Faster Roach 2.0.zip"
  shoot "notes-witcher-$size" "$size" mods "$PWD/home" --game=$w3 "--selftest-import=$dl/Faster Roach 2.0.zip"
  # 0.51: Kingdom Come: Deliverance I and II (a mod folder, a bare pak; a mod named for the other game).
  fresh "kcd-$size"; h="$PWD/fh/kcd-$size"
  shoot "mods-kcd1-$size" "$size" mods "$h" --game=steam:379430 "--selftest-import=$h/Downloads/Better Lockpicking 1.0.zip,$h/Downloads/Unlimited Saving 3.0.zip"
  shoot "mods-kcd2-$size" "$size" mods "$h" --game=steam:1771300 "--selftest-import=$h/Downloads/Herb Picking II 1.5.zip,$h/Downloads/Fast Travel KCD1 2.0.zip"
  shoot "check-kcd2-$size" "$size" check "$h" --game=steam:1771300 "--selftest-import=$h/Downloads/Herb Picking II 1.5.zip,$h/Downloads/Fast Travel KCD1 2.0.zip"
  # The controller pointer: shown over the mod list and clicked on the second mod's ON pill through the same
  # mouse events the right stick and RT make; wheel steps (the left stick) on the 2,000 row list.
  shoot "pointer-$size" "$size" pointer "$PWD/home" --game=$skyrim \
    "--selftest-import=$dl/Brighter Lanterns 1.2.zip,$dl/Lantern Glow SKSE 2.0.zip"
  shoot "pointerscroll-$size" "$size" pointerscroll "$PWD/home"
  # Downloads caught with no setup: the question in Turbine and over the browser; Get Mods without a key.
  shoot "caught-$size" "$size" caught "$PWD/home" --game=$cp2077
  shoot "caughtgm-$size" "$size" caughtgm "$PWD/home" --game=$cp2077
  shoot "nexusnokey-$size" "$size" nexusnokey "$PWD/home" --game=$skyrim
  # 0.5: Get mods without a key (grid, the fallback, a mod page free and Premium, back to the same search),
  # a game with no mods, Removed mods, a new mod arriving, opening Nexus, the rail, Look and the waiting pill.
  # Each of these gets its own copy of the home (the mod store sits in the game's library).
  fresh "getmods-$size"; h="$PWD/fh/getmods-$size"
  shoot "getmods-start-$size" "$size" getmods "$h" --game=$skyrim "--selftest-import=$h/Downloads/Brighter Lanterns 1.2.zip|nexus|skyrimspecialedition:1:1|1.2"
  shoot "getmods-fallback-$size" "$size" getmodsfallback "$PWD/home" --game=$skyrim
  for p in modpagefree modpagepremium; do
    fresh "$p-$size"; h="$PWD/fh/$p-$size"
    shoot "${p/modpage/modpage-}-$size" "$size" $p "$h" --game=$w3 "--selftest-import=$h/Downloads/Better Torches 1.0.zip|nexus|witcher3:2109:1|1.0"
  done
  shoot "getmods-back-$size" "$size" getmodsback "$PWD/home" --game=$skyrim
  for p in emptygame emptygamemods; do
    fresh "$p-$size"; h="$PWD/fh/$p-$size"
    shoot "${p/gamemods/game-mods}-$size" "$size" $p "$h" --game=$w3 "--selftest-import=$h/Downloads/Better Torches 1.0.zip"
  done
  fresh "removed-$size"; h="$PWD/fh/removed-$size"
  shoot "removed-$size" "$size" removed "$h" --game=$w3 "--selftest-import=$h/Downloads/Better Torches 1.0.zip,$h/Downloads/All Quest Objectives On Map-5-1-0-5-1699999999.zip"
  fresh "newmod-$size"; h="$PWD/fh/newmod-$size"
  shoot "newmod-$size" "$size" newmod "$h" --game=$w3 "--selftest-import=$h/Downloads/All Quest Objectives On Map-5-1-0-5-1699999999.zip"
  shoot "opening-$size" "$size" opening "$PWD/home" --game=$w3
  shoot "opening8s-$size" "$size" opening8s "$PWD/home" --game=$w3
  shoot "slowhint-$size" "$size" slowhint "$PWD/home" --game=$w3
  shoot "rail-$size" "$size" rail "$PWD/home" --game=$skyrim
  shoot "tvhidden-$size" "$size" tvhidden "$PWD/home"
  shoot "overlaywait-$size" "$size" overlaywait "$PWD/home"
  # 0.54 controls: the beginner tutorial's four cards, the quick wheel (Up then Right: Sort), a removed mod
  # dimmed in its row, the rail's quick menu, and the browser buttons under a mod page's button.
  for n in 1 2 3 4; do shoot "tutorial$n-$size" "$size" "tutorial$n" "$PWD/home"; done
  for p in wheel ghost; do
    fresh "$p-$size"; h="$PWD/fh/$p-$size"
    shoot "$p-$size" "$size" $p "$h" --game=$skyrim "--selftest-import=$h/Downloads/Brighter Lanterns 1.2.zip,$h/Downloads/Lantern Glow SKSE 2.0.zip"
  done
  shoot "railmenu-$size" "$size" railmenu "$PWD/home" --game=$skyrim
  # 0.55: the Nexus page's frame over a mod page (the page itself is drawn in the X11 run below).
  fresh "browserframe-$size"; h="$PWD/fh/browserframe-$size"
  shoot "browserframe-$size" "$size" browserframe "$h" --game=$skyrim
  fresh "inlinebrowser-$size"; h="$PWD/fh/inlinebrowser-$size"
  shoot "inlinebrowser-$size" "$size" inlinebrowser "$h" --game=$w3 "--selftest-import=$h/Downloads/Better Torches 1.0.zip|nexus|witcher3:2109:1|1.0"
done
if [ "$MODE" = 1280x800 ]; then
# 0.54 controls, the acceptance checks: pad presses through the same path a controller's take.
# A new home for each (not a copy of the shared one, whose mods earlier pictures turned on and off): both mods start on.
for p in controls legendcheck sheetcheck inputcheck startcheck keycheck; do
  rm -rf "$PWD/fh/$p"; python3 tools/fake-home.py "$PWD/fh/$p" > /dev/null; h="$PWD/fh/$p"
  shoot "$p-1280x800" 1280x800 $p "$h" --game=$skyrim "--selftest-import=$h/Downloads/Brighter Lanterns 1.2.zip,$h/Downloads/Lantern Glow SKSE 2.0.zip"
done
fresh "nexusguard"; h="$PWD/fh/nexusguard"
shoot "nexusguard-1280x800" 1280x800 nexusguard "$h" --game=$w3 "--selftest-import=$h/Downloads/Better Torches 1.0.zip|nexus|witcher3:2109:1|1.0"
# A new home: by now the shared one has Witcher 3 mods from earlier pictures.
rm -rf "$PWD/fh/presscount"; python3 tools/fake-home.py "$PWD/fh/presscount" > /dev/null; h="$PWD/fh/presscount"
shoot "presscount-1280x800" 1280x800 presscount "$h" --game=$w3 "--presscount-file=$h/Downloads/Better Torches 1.0.zip"
c=controls-1280x800
need $c "2: LB/RB in Settings keep the game" "Settings LB/RB: section .*, game unchanged"
need $c "7: RT is one click with the pointer" "Pointer RT: one click"
need $c "7: LT opens the quick menu" 'Pointer LT: sheet "modmenu"'
need $c "7: after a D-pad press the triggers page" 'After a D-pad press: pointer hidden, LT row 2 to 1, RT to 2, sheet ""'
need $c "R3 with the pointer hidden is A" "R3 hidden: pointer stays hidden, same as A"
need $c "R3 with the pointer showing clicks" "Left click \(R3\)"
need $c "8: toggle keeps the row" "Toggle: focus on row 2"
need $c "8: a menu closes back to its row" "Menu modmenu closed: focus on row 2"
need $c "8: tabs keep the row" "Tabs round trip: tab 1 and back to 0, row 2"
need $c "8: a game round trip keeps tab and row" "Game round trip: back on .*, tab 0, .*: the same mod"
need $c "Quick wheel: Up then Right" "Wheel: slot 1 Updates"
need $c "Quick wheel held" 'Wheel held, Left, let go: sheet pick "Games"'
need $c "Quick wheel: the middle is neutral, A closes" "Wheel neutral: a tap of View leaves it open with slot -1, A Close, B Close; A closes it: true"
need $c "Quick wheel: the stick at rest picks nothing" "Wheel stick: pushed Right picks slot 2, at rest slot -1; D-pad Up picks slot 0 and it stays; B closes it: true"
need $c "Quick wheel: View let go with nothing picked closes" 'Wheel held, nothing pushed, let go: it was open, now closed, sheet ""'
need $c "Quick wheel: a click on the middle closes" "Wheel middle clicked: it was open, now closed"
need $c "Uninstall keeps the row" "Uninstall: row 1 is ghost"
need $c "Put back" "Put back: .* is back"
# 5: no sheet opened without a press (in the scripted pages and when Nexus is slow to show).
for p in controls-1280x800 presscount-1280x800 nexusguard-1280x800 opening8s-1280x800; do
  hits=$(grep -h "opened with no press" "$PWD/data/$p/turbine-mod-manager/activity.log" 2>/dev/null || true)
  if [ -n "$hits" ]; then echo "::error title=5: a sheet opened with no press ($p)::$(echo "$hits" | head -n1 | cut -c1-200)"; fail=1; fi
done
need opening8s-1280x800 "No sheet when Nexus is slow" "Still in front 8 seconds after Open: Open in Steam's browser and Always use Steam's browser are under the button"
need nexusguard-1280x800 "3: three A presses, one browser" "Nexus guard: 3 presses of A, 1 browser launch,"
need legendcheck-1280x800 "4: the legend is binds()" "Legend check: [0-9]+ contexts, 0 problems"
need sheetcheck-1280x800 "6: A highlighted, B cancel, Start nothing" "Sheet rule: [0-9]+ sheets, 0 problems"
# 0.62: Start. A tap is Settings and back, a hold let go early is nothing, 3 seconds is the Play choice (or Play, by
# the setting), A on Play plays, Ctrl+Enter is the hold, a menu's Start never plays, the bar's hint is a button.
s=startcheck-1280x800
need $s "Start: a tap opens Settings" "Start check: a tap opens Settings, legend .*START Close$"
need $s "Start: a tap again goes back" "Start check: a tap again goes back to My mods$"
need $s "Start: B goes back" "Start check: B from Settings goes back to My mods$"
need $s "Start: let go early does nothing, the ring was filling" "Start check: let go at 1.5 s: the ring was showing at [3-8][0-9]%, then page game, sheet \"\", Play asked 0 times$"
need $s "Start: held 3 seconds shows the Play choice" "Start check: held 3 seconds: nothing at 2.3 s \(sheet \"\"\), then Play options, focus Play with mods, Play asked 0 times$"
need $s "Start: B closes the Play choice" "Start check: B closes the Play choice: sheet \"\"$"
need $s "Start: the setting in Settings, Look" "Start check: Hold Start does: row there, A changed it to last, the bar says hold: Play choice then hold: Play$"
need $s "Start: Play my last choice plays after 3 seconds" "Start check: Play my last choice: held 3 seconds, Play asked 1 times$"
need $s "Start: A on Play still plays" "Start check: A on Play: Play asked 1 times$"
need $s "Start: Ctrl+Enter is the hold" "Start check: Ctrl\+Enter: Play options, setting choice$"
need $s "Start: never plays from a menu" "Start check: Start on the mod menu: sheet \"modmenu\", page game, Play asked 0 times, legend ([^S]|S[^T])*$"
need $s "Start: the bar's hint is a 44 px button" "Start check: a click on the Start hint \(([4-9][4-9]|[5-9][0-9]|[1-9][0-9][0-9]) px tall\): Settings$"
need $s "Start: the bar with the hint, nothing overlaps" "Bar check \(startcheck, .* 0 problems \| .*start, wheel, Play"
need $s "Start checks" "Start check done: 0 problems"
# 0.62: the keyboard and the mouse drive every screen, and the bar's hints follow the last input used.
y=keycheck-1280x800
need $y "Keys: arrows move the ring" "Key check: Down and Up: row 2 then row 1, last input keys$"
need $y "Keys: the hints say the keys" "Key check: the hints after a key: Enter, Esc, X, Ctrl\+F, PgUp, PgDn$"
need $y "Keys: Enter is A" "Key check: Enter is A: the mod turned (on|off)$"
need $y "Keys: Menu and Shift+F10 open options, Esc and Backspace go back" "Key check: Menu key: sheet \"modmenu\", Esc: \"\", Shift\+F10: \"modmenu\", Backspace: \"\"$"
need $y "Keys: Ctrl+F and / find" "Key check: Ctrl\+F: sheet \"findpop\", Esc: \"\", /: \"findpop\", Esc: \"\"$"
need $y "Keys: Tab turns the page" "Key check: Tab: Get mods$"
need $y "Keys: Get mods by arrows and Enter" "Key check: Get mods: Right to card 2, Down to card [0-9]+, Enter opens the mod's page$"
need $y "Keys: Esc leaves the mod's page" "Key check: Esc from the mod's page: back on Get mods$"
need $y "Keys: Page Down, Page Up and Shift+Tab turn pages" "Key check: Page Down: tab 2, Page Up: tab 1, Shift\+Tab: tab 0$"
need $y "Keys: F1 is Help" "Key check: F1: settings Help, Esc: game$"
need $y "Keys: Ctrl+= and Ctrl+- change the size" "Key check: Ctrl\+=: 100% to 110%, Ctrl\+-: back to 100%$"
need $y "Keys: P is Start, V the wheel" "Key check: P: settings, P again: game; V: the wheel open, Right picks slot 2, Esc: closed$"
need $y "Keys: Ctrl+Enter is the Play choice" "Key check: Ctrl\+Enter: pick Play options, Esc: sheet \"\"$"
need $y "Keys: Ctrl+Z is the toast's Undo" "Key check: the toast says Ctrl\+Z Undo; Ctrl\+Z: [1-9] of [1-9] back on$"
need $y "Mouse: hover outlines, the hints say the mouse" "Key check: the mouse over a row: last input mouse, the row is outlined, hints Click, Esc, Right-click, Ctrl\+F, PgUp, PgDn$"
need $y "Mouse: a click is A" "Key check: a click on a row is A: row 2 turned (on|off)$"
need $y "Mouse: right-click is options, Back is B" "Key check: a right-click on row 1: sheet \"modmenu\" for row 1; the Back button: sheet \"\"$"
need $y "Mouse: right-click a card, Back twice" "Key check: a right-click on card 2: sheet \"cardmenu\"; Back: sheet \"\"; Back again: My mods$"
need $y "Mouse: Ctrl and the wheel change the size" "Key check: the wheel alone: 100%; Ctrl and the wheel up: 110%, down: 100%$"
need $y "The hints follow the last input" "Key check: hints after a key: Enter, Esc, X, Ctrl\+F, PgUp, PgDn; after a controller press: A, B, X, Y, LB, RB$"
need $y "The bar with the keyboard's names: nothing overlaps" "Bar check \(keycheck, .* 0 problems"
need $y "One sign-in status" "Key check: sign-in status: Settings says Nexus Mods: (Signed in, Turbine's browser is signed in|Not signed in, Turbine's browser is not signed in)$"
need $y "Key and mouse checks" "Key check done: 0 problems"
# 0.62: Desktop Mode with no controller is normal: a line in the log, never a toast or a line in Settings.
need $y "No controller in Desktop Mode: logged, nothing shown" "No controller in Desktop Mode: (logged once, no toast|a controller is seen, no toast)"
hits=$(grep -n -E "No controller reaches Turbine|padHelp" app/qml/*.qml || true)
if [ -n "$hits" ]; then echo "::error title=The no controller message is back::$(echo "$hits" | head -n1 | cut -c1-200)"; fail=1; fi
# 0.62: Get mods as a page of a book: no game list, the page edges, Left and Right at the ends of a row, LB and RB,
# a held D-pad, clicks on the edges, and the page-turn itself.
fresh "bookcheck"; h="$PWD/fh/bookcheck"
shoot "bookcheck-1280x800" 1280x800 bookcheck "$h" --game=$skyrim "--selftest-import=$h/Downloads/Brighter Lanterns 1.2.zip|nexus|skyrimspecialedition:1:1|1.2"
k=bookcheck-1280x800
need $k "Book: My mods keeps the game list" "Book check: My mods: the game list is there, page edges none$"
need $k "Book: RB turns forward to Get mods" "Book check: RB: Get mods comes in from the right, turning true$"
need $k "Book: no game list on Get mods, the edges name the pages" "Book check: Get mods: the game list is gone, left edge My mods \(([4-9][4-9]|[5-9][0-9]|[1-9][0-9][0-9]) px\), right edge Load order$"
need $k "Book: Left inside a row moves the ring" "Book check: Left inside the row: card 1, still Get mods$"
need $k "Book: Left from the first column turns back" "Book check: Left from the first column: My mods comes in from the left over Get mods \(still showing\), zone main$"
need $k "Book: the turn ends, the game list is back" "Book check: The turn done: My mods, Get mods put away, the game list back$"
need $k "Book: Right inside a row moves the ring" "Book check: Right inside the row: card 2, still Get mods$"
need $k "Book: Right from the last column turns forward" "Book check: Right from the last column: Load order comes in from the right$"
need $k "Book: LB turns back from Load order" "Book check: LB from Load order: Get mods comes in from the left$"
need $k "Book: a held D-pad stops at the edge" "Book check: Left held 1.5 s: card 1, the page stayed$"
need $k "Book: a click on the left edge" "Book check: A click on the left edge: My mods$"
need $k "Book: a click on the right edge" "Book check: A click on the right edge: Load order$"
need $k "Book: the bar on Get mods, nothing overlaps" "Bar check \(bookcheck, .* 0 problems"
need $k "Book: B is My mods" "Book check: The bar on Get mods: .*B My mods"
need $k "Book checks" "Book check done: 0 problems, [1-9][0-9]* page turns shown"
# 0.54 fixes: holds need a real press, a click on a row is A, the removed row, RT and R3, the tutorial, hidden games.
i=inputcheck-1280x800
need $i "Holds: a click never deletes" "Hold check: a click on Hold A to delete: only the highlight moved"
need $i "Holds: R3 never deletes" "Hold check: R3 on Hold A to delete: no hold"
need $i "Holds stop in the background" "Hold check: A held, then Turbine in the background: the hold started, stopped"
need $i "Holds stop when the controller goes" "Hold check: A held, then the controller went away: stopped"
need $i "Holds stop when the sheet changes" "Hold check: A held, then the sheet changed: stopped"
need $i "Holds still work when held" "Hold check: A really held for 2 seconds: Delete for good ran"
need $i "Reset: a tap stops" "Hold check: a tap of A on Reset game files: stopped at the release"
need $i "Reset: B stops it" "Hold check: A held on Reset, then B closed the menu: stopped"
need $i "Reset: a click never resets" "Hold check: a click on Reset game files: no hold"
need $i "Reset: R3 never resets" "Hold check: R3 on Reset game files: no hold"
need $i "RT and R3 held apart" "RT and R3: R3 let go while RT holds: still held; RT let go: let go; RT let go while R3 holds: still held; R3 let go: let go; presses sent on to the interface: 0"
need $i "A click on a mod row is A" "Click check: a click on a mod row's name: turned it (on|off)"
need $i "A click on a Settings row is A" "Click check: a click on a Settings choice row: changed it"
need $i "Rows are 44 px or more" "Click check: a mod row is ([4-9][4-9]|[5-9][0-9]|[1-9][0-9][0-9]) px tall"
need $i "Move with the removed row" "Ghost check: Move with the removed row showing moves .*, the focused mod"
need $i "A as the removed row goes" "Ghost check: A just as the removed row went: put back, the mod that slid up untouched"
need $i "Tutorial waits" "Tutorial check: after the tutorial: sheet \"oldhelper\""
need $i "Show hidden games" "Hidden check: every game hidden, the rail shows Show hidden games"
need $i "Press twice: no repeat toast" "Press twice check: first A on Delete for good: the button says it, no toast"
need $i "Add from Downloads: Y is Find" "Find check: Add from Downloads Y Find \(This game's files, All files\), All files showing"
need $i "Toast action on R3" "Toast check: Turn every mod off shows (R3|F) Undo"
need $i "R3's step is in the legend while the toast shows" "Toast check: the legend while it shows: A [^·]* · B [^·]* · R3 Undo"
need $i "R3 leaves the legend with the toast" "Toast check: the legend after it: A [^·]* · B [^·]*( · [XY] [^·]*)*$"
need $i "Y is Find under a toast" "Toast check: Y with Undo showing: Find opened, Undo kept"
need $i "R3 undoes" "Toast check: R3 with Undo showing: [1-9][0-9]* mods? back on of [1-9]"
need $i "Input checks" "Input check: 0 problems"
fi
for size in $SIZES; do
  need inlinebrowser-$size "No inline browser buttons: Nexus opens in Turbine ($size)" "Inline browser buttons: none \(Nexus open in Turbine\), sheet \"\""
done
if [ "$MODE" = 1280x800 ]; then
need presscount-1280x800 "Presses to a mod and Play" "Presses from the rail to a mod added and Play: "
fi
if [ "$MODE" = x11 ]; then
# 10. The 0.54 regression: a helper report with no finish, a refused finish, and the Slow download hint.
fresh "regress"; h="$PWD/fh/regress"
timeout 90 dbus-run-session -- flatpak run --env=QT_QPA_PLATFORM=offscreen --env=QT_QUICK_BACKEND=software \
  --env=TURBINE_HOME="$h" --env=TURBINE_DATA="$PWD/data/regress" \
  --filesystem="$PWD/shots" --filesystem="$h" --filesystem="$PWD/data" --filesystem="$PWD/samples:ro" \
  io.github.ravenor907.TurbineModManager --screenshot="$PWD/shots/regress-1280x800.png" --size=1280x800 --page=regress --selftest-data="$PWD/samples" \
  --game=$skyrim "--regress-downloads=$h/Downloads" > shots/regress.log 2>&1 || echo "::warning title=The regression run stopped with code $?::"
r=regress
need $r "10: the Slow download hint is shown" "Status \(hint\): Sign in to Nexus if it asks"
need $r "10: a download status replaces the hint" "Status \(busy\): Downloading in the browser: Regress Three 1.0.zip"
need $r "10: a report with no finish holds the watcher back" "Regress One 1.0.zip: the browser helper is reporting this download"
need $r "10: ... for at most 20 seconds" "The browser didn't report Regress One 1.0.zip finished; adding it from Downloads"
need $r "10: a refused finish falls back to the watcher" "Finished download not taken: .*the Downloads watcher takes Regress Two 1.0.zip instead"
added=$(grep -c "Added to .*Skyrim Special Edition: " "$PWD/data/$r/turbine-mod-manager/activity.log" || true)
if [ "$added" -ge 2 ]; then echo "::notice title=10: both files added::$added added"; else echo "::error title=10: the files weren't both added::$added added"; (grep -h "catch\|bridge" "$PWD/data/$r/turbine-mod-manager/activity.log" | tail -n 40 || true); fail=1; fi
fi
if [ "$MODE" = 1280x800 ]; then
# 0.51: Get mods for Cyberpunk (its own mods, categories), who wins a shared file, the closed-early prompt,
# the next requirement for a free account, and the bar over My mods.
shoot "getmods-cyberpunk-1280x800" 1280x800 getmods "$PWD/home" --game=steam:1091500
fresh "overlaps"; h="$PWD/fh/overlaps"
shoot "overlaps-1280x800" 1280x800 overlaps "$h" --game=$skyrim "--selftest-import=$h/Downloads/Brighter Lanterns 1.2.zip,$h/Downloads/Lantern Colors 1.1.zip"
fresh "quickexit"; h="$PWD/fh/quickexit"
shoot "quickexit-1280x800" 1280x800 quickexit "$h" --game=$skyrim "--selftest-import=$h/Downloads/Brighter Lanterns 1.2.zip,$h/Downloads/Lantern Colors 1.1.zip"
shoot "chain-1280x800" 1280x800 chain "$PWD/home" --game=steam:292030
fresh "listbar"; h="$PWD/fh/listbar"
shoot "listbar-1280x800" 1280x800 listbar "$h" --game=$skyrim "--selftest-import=$h/Downloads/Brighter Lanterns 1.2.zip,$h/Downloads/Lantern Colors 1.1.zip,$h/Downloads/Lantern Glow SKSE 2.0.zip"
need listbar-1280x800 "My mods: Y opens the pop-up with Find, Show, Sort" "Find pop-up \\(My mods\\): field, Show 6, Sort 3$"
need listbar-1280x800 "My mods pop-up legend (0.59: B is Done, X Clear all)" "My mods pop-up legend: A Choose · B Done · X Clear all · Y Keyboard$"
if ran "getmods-cyberpunk-1280x800"; then
log="$PWD/data/getmods-cyberpunk-1280x800/turbine-mod-manager/activity.log"
if grep -q "Get mods for Cyberpunk 2077: .* first Cyber Engine Tweaks" "$log"; then echo "::notice title=Get mods per game::$(grep -h 'Get mods for' "$log" | head -n1 | cut -c1-200)"
else echo "::error title=Cyberpunk's Get mods didn't show Cyberpunk mods::"; (grep -h "Get mods\|net" "$log" | tail -n 10 || true); fail=1; fi
fi
if ran "getmods-start-1280x800"; then
log="$PWD/data/getmods-start-1280x800/turbine-mod-manager/activity.log"
if grep -q "categories" "$log"; then echo "::notice title=Get mods categories::$(grep -h 'Get mods for' "$log" | head -n1 | cut -c1-200)"
else echo "::error title=No categories in Get mods::"; fail=1; fi
fi
if ran "overlaps-1280x800"; then
log="$PWD/data/overlaps-1280x800/turbine-mod-manager/activity.log"
if grep -q "Mod menu: .*Loses 1 file to Lantern Colors" "$log"; then echo "::notice title=Who wins::$(grep -h 'Mod menu:' "$log" | head -n1 | cut -c1-200)"
else echo "::error title=The mod menu didn't say which mod wins the shared file::"; (grep -h "Mod menu\|shot" "$log" | tail -n 5 || true); fail=1; fi
fi
if ran "quickexit-1280x800"; then
log="$PWD/data/quickexit-1280x800/turbine-mod-manager/activity.log"
if grep -q "closed after 12 seconds: offering a way back" "$log"; then echo "::notice title=Closed early::offered"
else echo "::error title=No closed-early prompt::"; fail=1; fi
fi
fi
for size in $SIZES; do
  if ran "check-kcd2-$size"; then
  log="$PWD/data/check-kcd2-$size/turbine-mod-manager/activity.log"
  if grep -Eq "KCD1.* is made for Kingdom Come: Deliverance(;|$)" "$log"; then
    echo "::notice title=KCD II check $size::$(grep -h 'Check before Play' "$log" | head -n1 | cut -c1-200)"
  else echo "::error title=KCD II check didn't flag the first game's mod ($size)::"; (grep -h "Check before Play\|Add " "$log" | tail -n 5 || true); fail=1; fi
  fi
done
if [ "$MODE" = 1280x800 ]; then
# Controls: Play options, R3 Find on My mods, Select several.
for p in playoptions findmods selectmods; do
  fresh "$p-1280x800"; h="$PWD/fh/$p-1280x800"
  shoot "$p-1280x800" 1280x800 $p "$h" --game=$skyrim \
    "--selftest-import=$h/Downloads/Brighter Lanterns 1.2.zip,$h/Downloads/Lantern Glow SKSE 2.0.zip"
done
if ran "playoptions-1280x800"; then
log="$PWD/data/playoptions-1280x800/turbine-mod-manager/activity.log"
if grep -q "Play options open: true" "$log"; then echo "::notice title=Play options::Play options opened"
else echo "::error title=Play options didn't open::"; (grep -h "shot\|action" "$log" | tail -n 10 || true); fail=1; fi
fi
# 0.52: A on Before you play runs the highlighted button (Not now must not play); the mod and game menus
# offer the missing script extender; KCD II rows say a first-game mod may not work; Witcher's clash offers
# turning off the newer mod; Interface size stops at the largest that fits.
fresh "checkfocus"; h="$PWD/fh/checkfocus"
shoot "checkfocus-1280x800" 1280x800 checkfocus "$h" --game=$skyrim "--selftest-import=$h/Downloads/Lantern Glow SKSE 2.0.zip"
fresh "menuskse"; h="$PWD/fh/menuskse"
shoot "menuskse-1280x800" 1280x800 menuskse "$h" --game=$skyrim "--selftest-import=$h/Downloads/Brighter Lanterns 1.2.zip,$h/Downloads/Lantern Glow SKSE 2.0.zip"
if ran "checkfocus-1280x800"; then
log="$PWD/data/checkfocus-1280x800/turbine-mod-manager/activity.log"
if grep -q "Check before Play: Not now" "$log" && ! grep -q "Check before Play: Play anyway" "$log" && grep -q 'After A: sheet ""' "$log"; then
  echo "::notice title=Before you play::$(grep -h 'Check focus on\|After A' "$log" | head -n2 | tr '\n' ' ' | cut -c1-200)"
else echo "::error title=A on Before you play didn't run the highlighted button::"; (grep -h "Check\|shot" "$log" | tail -n 10 || true); fail=1; fi
fi
if ran "menuskse-1280x800"; then
log="$PWD/data/menuskse-1280x800/turbine-mod-manager/activity.log"
if grep -q "Mod menu: Get SKSE" "$log" && grep -Eq "Game menu: .*(Install|Get) SKSE" "$log"; then
  echo "::notice title=SKSE way back::$(grep -h 'Mod menu:' "$log" | head -n1 | cut -c1-200)"
else echo "::error title=No way back to SKSE in the mod or game menu::"; (grep -h "menu:" "$log" | tail -n 4 || true); fail=1; fi
fi
fi
for size in $SIZES; do
  if ran "mods-kcd2-$size"; then
  log="$PWD/data/mods-kcd2-$size/turbine-mod-manager/activity.log"
  if grep -q "Row warnings: .*Fast Travel KCD1: Made for KCD" "$log"; then echo "::notice title=KCD II row warning $size::$(grep -h 'Row warnings' "$log" | head -n1 | cut -c1-200)"
  else echo "::error title=KCD II row has no warning for the first game's mod ($size)::"; (grep -h "Row warnings\|Add " "$log" | tail -n 4 || true); fail=1; fi
  fi
  if ran "check-witcher-$size"; then
  log="$PWD/data/check-witcher-$size/turbine-mod-manager/activity.log"
  if grep -q "Check before Play buttons: .*Turn off Faster Roach" "$log"; then echo "::notice title=Witcher clash $size::$(grep -h 'Check before Play buttons' "$log" | head -n1 | cut -c1-200)"
  else echo "::error title=Witcher clash doesn't offer turning off the newer mod ($size)::"; (grep -h "Check before Play" "$log" | tail -n 4 || true); fail=1; fi
  fi
done
# 0.53: Game Mode's browser through Steam: with no shortcuts.vdf the check finds nothing and says so, and
# Add Chrome to Steam shows.
for size in $SIZES; do
  if ran "addbrowser-$size"; then
  log="$PWD/data/addbrowser-$size/turbine-mod-manager/activity.log"
  if grep -q "Recheck: no Chrome-family browser among Steam's non-Steam games" "$log" && grep -q "Showing Add Chrome to Steam" "$log"; then
    echo "::notice title=Add Chrome to Steam $size::$(grep -h 'Recheck:' "$log" | head -n1 | cut -c1-200)"
  else echo "::error title=Add Chrome to Steam didn't show ($size)::"; (grep -h "browser\|shot" "$log" | tail -n 6 || true); fail=1; fi
  fi
done
# 0.54: Labs > Activity log shows the log's lines.
for size in $SIZES; do
  if ran "activity-$size"; then
  log="$PWD/data/activity-$size/turbine-mod-manager/activity.log"
  if grep -Eq "Activity log open: [1-9][0-9]* lines" "$log"; then echo "::notice title=Activity log $size::$(grep -h 'Activity log open' "$log" | head -n1)"
  else echo "::error title=The activity log sheet showed no lines ($size)::"; (grep -h "labs\|shot" "$log" | tail -n 6 || true); fail=1; fi
  fi
  # Opening a file page on a free account says to press Slow download.
  if ran "slowhint-$size"; then
  log="$PWD/data/slowhint-$size/turbine-mod-manager/activity.log"
  if grep -q "Status: press Slow download on the page" "$log"; then echo "::notice title=Slow download hint $size::shown"
  else echo "::error title=Opening a Nexus file page didn't say to press Slow download ($size)::"; (grep -h "catch\|browser" "$log" | tail -n 6 || true); fail=1; fi
  fi
done
# Interface size 150% on the handheld size; Desktop and TV layouts at 1920 x 1080.
# 0.58: Interface size 50% to 150% with no cap: each input makes it smaller (D-pad Left, the - button clicked with the
# pointer, A's list; B cancels the list); the buttons are at least 44 pixels.
if [ "$MODE" = 1280x800 ]; then
shoot "sizeinputs-1280x800" 1280x800 sizeinputs "$PWD/home"
n=sizeinputs-1280x800
need $n "Size: the first press after opening Settings changes nothing (the owner's Nexus zoom)" "Size first Left after Settings opened: [0-9]+% to [0-9]+%, unchanged"
need $n "Size: D-pad Left makes it smaller" "Size by D-pad Left: [0-9]+% to [0-9]+%, smaller"
need $n "Size: the - button makes it smaller" "Size by the - button \(pointer click\): [0-9]+% to [0-9]+%, smaller"
need $n "Size: the + button makes it bigger" "Size by the \+ button \(pointer click\): [0-9]+% to [0-9]+%, bigger"
need $n "Size: A opens the list of sizes" "Size A: the list of sizes, on [0-9]+%.*, 11 sizes"
need $n "Size: B cancels the list" "Size list B: [0-9]+% to [0-9]+%, unchanged"
need $n "Size: A's list makes it smaller" "Size by A's list: [0-9]+% to [0-9]+%, smaller"
need $n "Size: D-pad Right makes it bigger" "Size by D-pad Right: [0-9]+% to [0-9]+%, bigger"
need $n "Size: 50% to 150% in 11 steps" "Size range: 50% to 150% in 11 steps"
need $n "Size: buttons of 44 pixels or more" "Size buttons: - (4[4-9]|[5-9][0-9]|1[0-9][0-9]) x"
if grep -q "PROBLEM" "$PWD/data/$n/turbine-mod-manager/activity.log"; then echo "::error title=Interface size check::$(grep -h PROBLEM "$PWD/data/$n/turbine-mod-manager/activity.log" | head -n3 | tr '\n' ' ' | cut -c1-300)"; fail=1; fi
# 0.58: the bottom bar in a 1024 pixel wide window (the owner's Desktop Mode picture), both layouts.
shoot "mods-1024x700" 1024x700 mods "$PWD/home" --game=$skyrim "--selftest-import=$dl/Brighter Lanterns 1.2.zip"
shoot "mods-desktop-1024x700" 1024x700 mods "$PWD/home" --game=$skyrim --layout=desktop "--selftest-import=$dl/Brighter Lanterns 1.2.zip"
need mods-1024x700 "Bar at 1024 wide: nothing overlaps Play or the edge" "Bar check \(mods, 1024x700, size [0-9]+%\): [0-9]+ items, 0 problems"
need mods-desktop-1024x700 "Bar at 1024 wide, Desktop layout" "Bar check \(mods, 1024x700, size [0-9]+%\): [0-9]+ items, 0 problems"
rm -rf "$PWD/fh/modsempty-1024"; python3 tools/fake-home.py "$PWD/fh/modsempty-1024" > /dev/null; h="$PWD/fh/modsempty-1024"
shoot "modsempty-1024x700" 1024x700 modsempty "$h" --game=$skyrim
need modsempty-1024x700 "Empty My mods card at 1024 wide" "No mods card: No mods for Skyrim"
fi
# 0.59: the start-up splash, mid-way and after; the first frame isn't later with it.
for size in $SIZES; do
  shoot "splashmid-$size" "$size" splashmid "$PWD/home"
  shoot "splashafter-$size" "$size" splashafter "$PWD/home"
  need "splashmid-$size" "Splash: the icon shows while it plays ($size)" "Splash check \(splashmid\): shown true, icon opacity 0\.[1-9]|Splash check \(splashmid\): shown true, icon opacity 1"
  need "splashafter-$size" "Splash: gone after about 1.2 s ($size)" "Splash check \(splashafter\): shown false"
  need "splashafter-$size" "Splash: played through ($size)" "Splash done \(played\) after (1[0-9]{3}|[5-9][0-9]{2}) ms"
  a=$(grep -ho "First frame drawn [0-9]* ms" "$PWD/data/game-$size/turbine-mod-manager/activity.log" 2>/dev/null | head -n1 | grep -o "[0-9]*" || true)
  b=$(grep -ho "First frame drawn [0-9]* ms" "$PWD/data/splashafter-$size/turbine-mod-manager/activity.log" 2>/dev/null | head -n1 | grep -o "[0-9]*" || true)
  echo "::notice title=First frame with and without the splash ($size)::without ${a:-?} ms, with ${b:-?} ms"
  if [ -n "$a" ] && [ -n "$b" ] && [ "$b" -gt $((a + 400)) ]; then echo "::warning title=The splash made the first frame later ($size)::$a ms to $b ms"; fi
done
# 0.6: every sheet and menu ends above the bottom bar (its last row too) at 50%, 100% and 150%, and at 1024 wide.
for size in $SIZES $( [ "$MODE" = 1280x800 ] && echo 1024x700 ); do
  for u in 50 100 150; do
    n="menufit-u$u-$size"
    rm -rf "$PWD/fh/$n"; python3 tools/fake-home.py "$PWD/fh/$n" > /dev/null; hm="$PWD/fh/$n"
    shoot "$n" "$size" menufit "$hm" --game=$skyrim --uisize=$u "--selftest-import=$hm/Downloads/Brighter Lanterns 1.2.zip,$hm/Downloads/Lantern Glow SKSE 2.0.zip"
    need "$n" "Menus end above the bar ($size, $u%)" "Menu fit \\([0-9]+x[0-9]+, size [0-9]+%\\): [0-9]+ menus, 0 problems"
  done
done
# 0.58: every size reflows instead of clipping: pictures at 50% and 150%, each with the bar check.
for size in $SIZES; do
  for u in 50 150; do
    shoot "mods-u$u-$size" "$size" mods "$PWD/home" --game=$skyrim --uisize=$u "--selftest-import=$dl/Brighter Lanterns 1.2.zip,$dl/Lantern Glow SKSE 2.0.zip"
    shoot "getmods-u$u-$size" "$size" getmods "$PWD/home" --game=$skyrim --uisize=$u
    shoot "settings-u$u-$size" "$size" settings "$PWD/home" --uisize=$u
    shoot "gamemenu-u$u-$size" "$size" gamemenu "$PWD/home" --game=$skyrim --uisize=$u
    need "mods-u$u-$size" "Bar at $u% ($size): nothing overlaps" "Bar check \(mods, $size, size $u%\): [0-9]+ items, 0 problems"
    need "getmods-u$u-$size" "Get mods bar at $u% ($size)" "Bar check \(getmods, $size, size $u%\): [0-9]+ items, 0 problems"
  done
  # 0.58: the one-time Launch Options step for a me3 game (A copies the exact text), and the launch check's sheet.
  shoot "launchsetup-$size" "$size" launchsetup "$PWD/home" --game=$skyrim
  need "launchsetup-$size" "Launch Options step: the exact text ($size)" "Launch Options step for Elden Ring: needs \"if \\[ -x .*/steam-launch/turbine-launch.sh.* %command%; else %command%; fi\""
  shoot "notstarted-$size" "$size" notstarted "$PWD/home" --game=$skyrim
  need "notstarted-$size" "Launch check: didn't start, with the fixes ($size)" "Launch check sheet: .* didn't start \| Play without mods; Activity log; Why won't it start\?"
  rm -rf "$PWD/fh/modsempty-$size"; python3 tools/fake-home.py "$PWD/fh/modsempty-$size" > /dev/null; h="$PWD/fh/modsempty-$size"
  shoot "modsempty-$size" "$size" modsempty "$h" --game=$skyrim
  need "modsempty-$size" "Empty My mods: the centered card ($size)" "No mods card: No mods for Skyrim"
  need "modsempty-$size" "Empty My mods: A Get mods, X Add from Downloads ($size)" "No mods card: .* \| legend A Get mods · B Games · X Add from Downloads"
done
if [ "$MODE" = 1920x1080 ]; then
shoot "desktop-1920x1080" 1920x1080 mods "$PWD/home" --game=$skyrim --layout=desktop \
  "--selftest-import=$dl/Brighter Lanterns 1.2.zip,$dl/Lantern Glow SKSE 2.0.zip"
shoot "desktopgrid-1920x1080" 1920x1080 desktopgrid "$PWD/home" --game=$skyrim --layout=desktop
shoot "tv-1920x1080" 1920x1080 tv "$PWD/home" --game=$skyrim --layout=tv --external-display
fi
# The 0.5 checks in the activity logs.
for size in $SIZES; do
  if ran "getmods-fallback-$size"; then
  log="$PWD/data/getmods-fallback-$size/turbine-mod-manager/activity.log"
  if grep -q "Nexus browse without a key failed" "$log"; then
    echo "::notice title=Keyless fallback $size::$(grep -h 'Nexus browse without a key failed' "$log" | head -n1 | cut -c1-200)"
  else
    echo "::error title=No fallback line in the activity log ($size)::"; (tail -n 20 "$log" || true); fail=1
  fi
  fi
  if ran "getmods-back-$size"; then
  log="$PWD/data/getmods-back-$size/turbine-mod-manager/activity.log"
  if grep -q 'Back from the mod page: search "lantern", card 5' "$log"; then
    echo "::notice title=Get mods back $size::same search, card 5"
  else
    echo "::error title=B didn't come back to the same search and card ($size)::"; (grep -h "shot\|action" "$log" | tail -n 20 || true); fail=1
  fi
  fi
  if ran "newmod-$size"; then
  log="$PWD/data/newmod-$size/turbine-mod-manager/activity.log"
  if grep -q "New mod All Quest Objectives On Map" "$log"; then
    echo "::notice title=New mod $size::$(grep -h 'New mod ' "$log" | head -n1 | cut -c1-160)"
  else
    echo "::error title=The new mod wasn't named cleanly or focused ($size)::"; (grep -h "New mod\|Add " "$log" | tail -n 10 || true); fail=1
  fi
  fi
done
# 0.55: the nxm scheme and WebEngine are set up before the application exists, in every run.
for p in $( [ -n "$SIZES" ] && echo "game-$SIZES settings-$SIZES" ); do
  if ran "$p"; then
  log="$PWD/data/$p/turbine-mod-manager/activity.log"
  a=$(grep -n "nxm scheme registered" "$log" | head -n1 | cut -d: -f1); b=$(grep -n "WebEngine ready" "$log" | head -n1 | cut -d: -f1)
  if [ -n "$a" ] && [ -n "$b" ] && [ "$a" -lt "$b" ]; then echo "::notice title=WebEngine set up ($p)::nxm scheme line $a, WebEngine ready line $b"
  else echo "::error title=WebEngine wasn't set up before the app ($p)::"; (grep -h "web\]" "$log" | head -n 10 || true); fail=1; fi
  fi
done
if [ "$MODE" = x11 ]; then
# Words that must never be on screen any more.
hits=$(grep -n -E "No controller found|Play original|\"Accounts\"|RECENTLY REMOVED" app/qml/*.qml || true)
if [ -n "$hits" ]; then echo "::error title=Old words still in the interface::"; echo "$hits"; fail=1; fi
fi
# The pointer's events really reached Turbine's interface: the click turned the mod off, the wheel scrolled.
for size in $SIZES; do
  if ran "pointer-$size"; then
  log="$PWD/data/pointer-$size/turbine-mod-manager/activity.log"
  if grep -q "Left click (RT)" "$log" && grep -Eq "Turn (on|off) " "$log"; then
    echo "::notice title=Pointer click $size::$(grep -Eh 'Turn (on|off) ' "$log" | head -n1 | cut -c1-160)"
  else
    echo "::error title=Pointer click didn't reach the mod ($size)::"; (grep -h "pad\|action" "$log" | tail -n 20 || true); fail=1
  fi
  fi
  if ran "pointerscroll-$size"; then
  log="$PWD/data/pointerscroll-$size/turbine-mod-manager/activity.log"
  moved=$(grep -ho "the list moved [0-9]* px" "$log" 2>/dev/null | grep -o "[0-9][0-9]*" | head -n1 || true)
  if [ -n "$moved" ] && [ "$moved" -gt 0 ]; then
    echo "::notice title=Pointer wheel $size::The list moved $moved px"
  else
    echo "::error title=Wheel steps didn't scroll the list ($size)::"; (grep -h "pad" "$log" | tail -n 20 || true); fail=1
  fi
  fi
done
if [ "$MODE" = x11 ]; then
# Mod Manager Download links: the installed Flatpak says Turbine opens nxm:// links.
loc=$(flatpak info --user --show-location io.github.ravenor907.TurbineModManager || true)
desk="$loc/export/share/applications/io.github.ravenor907.TurbineModManager.desktop"
if grep -q "^MimeType=.*x-scheme-handler/nxm" "$desk" && grep -q "^Exec=.*%u" "$desk"; then
  echo "::notice title=nxm handler exported::$(grep -h '^Exec=' "$desk")"
else
  echo "::error title=nxm handler not exported::$desk"; (cat "$desk" 2>&1 || true); fail=1
fi
# The browser helper is in the Flatpak (without its tests).
ext=$(flatpak run --command=sh io.github.ravenor907.TurbineModManager -c 'ls /app/share/turbine-mod-manager/extension' 2>&1 | tr '\n' ' ')
echo "Browser helper in the Flatpak: $ext"
case "$ext" in *manifest.json*) ;; *) echo "::error title=Browser helper missing::$ext"; fail=1 ;; esac
case "$ext" in *test*) echo "::error title=Browser helper ships its tests::$ext"; fail=1 ;; esac
# X11 (what SteamOS uses for Turbine now): the main window draws, and the Game Mode overlay window gets
# GAMESCOPE_EXTERNAL_OVERLAY. Needs Xvfb in the build container; said as a warning when it can't be had.
if ! command -v Xvfb > /dev/null; then
  (dnf install -y xorg-x11-server-Xvfb xprop > /dev/null 2>&1 || (apt-get update > /dev/null 2>&1 && apt-get install -y xvfb x11-utils > /dev/null 2>&1)) || true
fi
if command -v Xvfb > /dev/null; then
  Xvfb :77 -screen 0 1280x800x24 > /dev/null 2>&1 &
  xvfb=$!
  sleep 2
  for page in game overlaygm; do
    out="$PWD/shots/x11-$page-1280x800.png"
    rm -rf "$PWD/data/x11-$page"
    DISPLAY=:77 timeout 60 dbus-run-session -- flatpak run --socket=x11 --env=QT_QPA_PLATFORM=xcb --env=QT_QUICK_BACKEND=software \
      --env=TURBINE_HOME="$PWD/home" --env=TURBINE_DATA="$PWD/data/x11-$page" \
      --filesystem="$PWD/shots" --filesystem="$PWD/home" --filesystem="$PWD/data" --filesystem="$PWD/samples:ro" \
      io.github.ravenor907.TurbineModManager --screenshot="$out" --size=1280x800 --page=$page --selftest-data="$PWD/samples" > "shots/x11-$page.log" 2>&1
    if [ ! -s "$out" ]; then echo "::error title=No screenshot under X11::$page"; cat "shots/x11-$page.log"; fail=1; fi
    if grep -E "qrc:|\.qml:[0-9]+|TypeError|ReferenceError" "shots/x11-$page.log"; then echo "::error title=QML problem under X11::$page"; fail=1; fi
  done
  mark=$(grep -h "GAMESCOPE_EXTERNAL_OVERLAY" "$PWD"/data/x11-overlaygm/turbine-mod-manager/activity.log || true)
  echo "Overlay mark: $mark"
  case "$mark" in *"set to 1 (done)"*) echo "::notice title=Game Mode overlay::$mark" ;; *) echo "::error title=Overlay window not marked under X11::$mark"; fail=1 ;; esac
  kill $xvfb 2>/dev/null || true
else
  echo "::warning title=No Xvfb::The X11 check didn't run"
fi
fi
if [ "$MODE" = nexus ]; then
# 0.55: Nexus inside Turbine, against the stand-in page (tools/nexus-stand-in.py) under X11, WebEngine drawing
# without the GPU. The full run at both sizes, then a second run with the same profile (signed in stays).
if command -v Xvfb > /dev/null; then
  Xvfb :78 -screen 0 1920x1080x24 > /dev/null 2>&1 &
  xvfb=$!
  sl="$PWD/shots/standin.log"
  python3 tools/nexus-stand-in.py --port 47800 --log "$sl" --zip "$PWD/home/Downloads/Brighter Lanterns 1.2.zip" --dupe-zip "$PWD/home/Downloads/Lantern Glow SKSE 2.0.zip" > shots/standin.out 2>&1 &
  standin=$!
  sleep 2
  sky="The Elder Scrolls V Skyrim Special Edition"
  bshoot() {   # bshoot <name> <WxH> <page> <home> <data> [more arguments]
    name=$1 size=$2 page=$3 home=$4 bdata=$5
    shift 5
    RAN="$RAN$name "
    DISPLAY=:78 timeout 260 dbus-run-session -- flatpak run --socket=x11 --env=QT_QPA_PLATFORM=xcb --env=QT_QUICK_BACKEND=software \
      --env=QTWEBENGINE_CHROMIUM_FLAGS=--disable-gpu $BROWSER_ENV \
      --env=TURBINE_HOME="$home" --env=TURBINE_DATA="$bdata" \
      --filesystem="$PWD/shots" --filesystem="$home" --filesystem="$PWD/data" --filesystem="$PWD/samples:ro" \
      io.github.ravenor907.TurbineModManager --screenshot="$PWD/shots/$name.png" --size="$size" --page="$page" --selftest-data="$PWD/samples" \
      --browser-test=http://127.0.0.1:47800/ "$@" > "shots/$name.log" 2>&1 || echo "::warning title=$name stopped with code $?::"
    if [ ! -s "$PWD/shots/$name.png" ]; then echo "::error title=No screenshot::$name"; tail -n 60 "shots/$name.log"; fail=1; fi
    if grep -E "qrc:|\.qml:[0-9]+|TypeError|ReferenceError" "shots/$name.log"; then echo "::error title=QML problem::$name"; fail=1; fi
  }
  # Chromium refuses its own sandbox when run as root, and the build container runs everything as root: there (only
  # there) it runs without it. Players never run Turbine as root.
  BROWSER_ENV=""
  if [ "$(id -u)" = 0 ]; then BROWSER_ENV="--env=QTWEBENGINE_DISABLE_SANDBOX=1"; echo "::notice title=Chromium sandbox::off in this check only (the build container runs as root)"; fi
  # Both sizes here: this job is the only one with the stand-in page.
  for size in 1280x800 1920x1080; do
    n="browsertest-$size"
    rm -rf "$PWD/fh/$n" "$PWD/data/$n"; python3 tools/fake-home.py "$PWD/fh/$n" > /dev/null; h="$PWD/fh/$n"
    mkdir -p "$h/Downloads/Game Mods/$sky" && cp "$h/Downloads/Lantern Colors 1.1.zip" "$h/Downloads/Game Mods/$sky/Dupe Test 1.0.zip"
    bshoot "$n" "$size" browsertest "$h" "$PWD/data/$n" --game=$skyrim
    log="$PWD/data/$n/turbine-mod-manager/activity.log"
    if ran "$n"; then
    # Chromium's own sandbox inside the Flatpak: if the page never loaded, say so and try once without it.
    if ! grep -q "Browser check: the file page loaded in Turbine" "$log" 2>/dev/null && [ -z "$BROWSER_ENV" ]; then
      echo "::warning title=The Nexus page didn't load ($size)::trying again with QTWEBENGINE_DISABLE_SANDBOX=1 (only in this check)"
      grep -h " web \| browser " "$log" | tail -n 20 || true
      BROWSER_ENV="--env=QTWEBENGINE_DISABLE_SANDBOX=1"
      rm -rf "$PWD/fh/$n" "$PWD/data/$n"; python3 tools/fake-home.py "$PWD/fh/$n" > /dev/null
      mkdir -p "$h/Downloads/Game Mods/$sky" && cp "$h/Downloads/Lantern Colors 1.1.zip" "$h/Downloads/Game Mods/$sky/Dupe Test 1.0.zip"
      bshoot "$n" "$size" browsertest "$h" "$PWD/data/$n" --game=$skyrim
    fi
    fi
    need $n "2: WebEngine set up before the app ($size)" "WebEngine ready"
    need $n "Nexus page loaded in Turbine ($size)" "Browser check: the file page loaded in Turbine"
    need $n "9: legend, no field: Y Keyboard and R stick Pointer, no LT ($size)" "Browser legend \(no field\): A Click · B [^·]* · X Options · Y Keyboard · RS Pointer, 0 problems"
    need $n "9: legend, a field ($size)" "Browser legend \(a field\): .*Y Keyboard.*, 0 problems"
    if grep -Eq "Browser legend .*LT " "$log"; then echo "::error title=9: LT Menu is still in the legend ($size)::"; fail=1; fi
    need $n "7: A after a click is a click ($size)" "Browser check: A after a click is a click: yes"
    need $n "7: A presses Enter only after the D-pad ($size)" "Browser check: A after the D-pad presses Enter: yes"
    need $n "7: A after the pointer clicked is a click again ($size)" "Browser check: A after the pointer clicked is a click again: yes"
    need $n "9: the quick menu has at most 6 rows ($size)" "Browser check: the quick menu has [1-6] rows"
    tt=$(grep -h "Browser check: touch targets" "$log" | head -n1)
    small=$(echo "$tt" | grep -oE "[0-9]+x[0-9]+" | awk -Fx '$1 < 44 || $2 < 44' | head -n1)
    if [ -n "$tt" ] && [ -z "$small" ]; then echo "::notice title=9: touch targets 44 px or more ($size)::${tt#*touch targets }"
    else echo "::error title=9: a touch target under 44 px ($size)::$tt"; fail=1; fi
    need $n "9: the Added note clears ($size)" "Browser check: the Added note cleared at the next click: yes"
    need $n "9: the no-key note on its own page leaves A and B alone ($size)" "Browser check: the no-key note on its own file page leaves A Click and B (Close|Back)"
    need $n "10: no-key on another page: B Not now ($size)" "Browser check: the no-key note for another file's page: A Slow download page, B Not now"
    need $n "9: the pointer hides when the page closes ($size)" "Browser check: the pointer is hidden after the close: yes"
    need $n "9: A just after the auto close is ignored ($size)" "Press a ignored: the Nexus page closed by itself"
    need $n "9: ... and starts nothing ($size)" "Browser check: A just after the auto close started nothing: yes"
    need $n "2: a Nexus-hosted download after a press is a sure match ($size)" "Download: Dupe Test 1.0 \(2\).zip into Game Mods/$sky \(from 127.0.0.1, after your press: a sure match\)"
    need $n "8: B goes back a page ($size)" "Browser check: B from a second page went back one page"
    need $n "7: an exe is refused ($size)" "Download refused \(127.0.0.1\): Turbine only takes zip, 7z or rar mods"
    need $n "7: a second file of a name gets (2) ($size)" "Download: Dupe Test 1.0 \(2\).zip into Game Mods/$sky"
    need $n "1: the initiator is checked in C++ ($size)" "Mod Manager Download link from http://127.0.0.1:47800: initiator checked"
    need $n "4: Mod Manager Download comes to Turbine, which asks ($size)" "Mod Manager Download link from the browser \(scheme handler, from http://127.0.0.1:47800\): Turbine asks before it downloads: nxm://skyrimspecialedition/mods/1/files/2"
    need $n "M6: the question names mod and file, opens on Not now, page input off ($size)" "Browser check: Mod Manager Download asks: Get .* \(.*\) for .*\?, focus Not now, A Not now, B Not now, page input off"
    need $n "M6: A at once on the question is ignored ($size)" "Browser check: A at once on the question: ignored"
    need $n "M6: Right then A after a second gets it ($size)" "Browser check: Right then A after a second gets it: yes"
    need $n "L1: two taps on Get it don't get it ($size)" "Browser check: two taps on Get it: still asking"
    need $n "L2: a page alert or confirm doesn't replace the question ($size)" "Browser check: a page alert and confirm while asking: the question stays"
    need $n "4: and its file goes into Game Mods ($size)" "Test run: pretending to download nxm://skyrimspecialedition/mods/1/files/2.* into Game Mods/$sky"
    need $n "4: the page stays on the file page ($size)" "after Mod Manager Download the page is still the file page: yes"
    need $n "8: B on the first page closes, focus on Get it ($size)" "B on the first page closed it; focus on Get it"
    need $n "3: Slow download, added, page closed, focus on Play ($size)" "after the add the page closed by itself; focus on Play; My mods lists it"
    f="$h/Downloads/Game Mods/$sky/TurbineTest-Bigger_Pockets-1.0.3.zip"
    if [ -s "$f" ]; then echo "::notice title=3: the file is in Game Mods ($size)::$f"; else echo "::error title=3: the file isn't in Game Mods ($size)::"; ls -laR "$h/Downloads/Game Mods" || true; fail=1; fi
    adds=$(grep -c "Start: Add TurbineTest-Bigger_Pockets-1.0.3.zip" "$log" || true)
    if [ "$adds" = "1" ]; then echo "::notice title=3: added once ($size)::no second add from the watcher"; else echo "::error title=3: added $adds times ($size)::"; fail=1; fi
    presses=$(grep -c "Browser press [0-9]* (" "$log" || true); clicks=$(grep -c "Browser click [0-9]* (" "$log" || true)
    if [ "$presses" -gt 0 ] && [ "$presses" = "$clicks" ]; then echo "::notice title=5: one click per press ($size)::$presses presses, $clicks clicks"
    else echo "::error title=5: clicks and presses differ ($size)::$presses presses, $clicks clicks"; grep -h "Browser press\|Browser click" "$log" | head -n 40; fail=1; fi
    # The sign-in pass-through: a helper's answer for another request is refused whole; the real one puts only
    # Nexus's cookie into Turbine's browser, never logs its value, and the cookie is kept on disk with the profile.
    need $n "Sign-in: an answer for another request is refused ($size)" "Sign-in from the browser refused: the sign-in answer is for another request"
    need $n "Sign-in: only Nexus's cookie is taken ($size)" "Sign-in from the browser: 1 Nexus cookie\(s\) put into Turbine's browser \(turbine_signin_check\); 3 for other sites or unreadable, dropped"
    need $n "Sign-in: Turbine says it is signed in ($size)" "Signed in to Nexus \(from the player's browser\)"
    if grep -rq "valueNeverLogged7f3" "$PWD/data/$n/turbine-mod-manager/activity.log" "shots/$n.log"; then echo "::error title=Sign-in: a cookie value was logged ($size)::"; fail=1
    else echo "::notice title=Sign-in: no cookie value in the logs ($size)::"; fi
    kept=$(grep -rla "turbine_signin_check" "$PWD/data/$n" 2>/dev/null | grep -v "activity.log" | head -n1 || true)
    if [ -n "$kept" ]; then echo "::notice title=Sign-in: the cookie is in the profile on disk ($size)::${kept#$PWD/}"
    else echo "::error title=Sign-in: the cookie isn't in the profile on disk ($size)::"; find "$PWD/data/$n" -maxdepth 4 | head -n 40; fail=1; fi
    if grep -rlaE "other_site_check|look_alike_check" "$PWD/data/$n" 2>/dev/null | grep -qv "activity.log"; then echo "::error title=Sign-in: another site's cookie got in ($size)::"; fail=1; fi
    if grep -rlaE "cf_turbine_check|cfValueNeverKept9" "$PWD/data/$n" 2>/dev/null | grep -qv "activity.log"; then echo "::error title=Sign-in: a Cloudflare cookie got in ($size)::"; fail=1
    else echo "::notice title=Sign-in: no Cloudflare cookie in Turbine's browser ($size)::"; fi
    # 6: signed links and the account number never go into the log whole.
    leak=$(grep -Eh "user_id=|[?&]key=|[?&]md5=|expires=[0-9]" "$log" 2>/dev/null || true)
    if [ -n "$leak" ]; then echo "::error title=6: a signed link or user_id is in the log ($size)::"; echo "$leak" | head -n 5 | cut -c1-200; fail=1
    else echo "::notice title=6: no signed links or user_id in the log ($size)::"; fi
    # 6: the profile folder (it holds the sign-in) is the player's alone.
    bdir=$(find "$PWD/data/$n" -maxdepth 3 -type d -name browser | head -n1)
    perm=$( [ -n "$bdir" ] && stat -c %a "$bdir" || echo none)
    if [ "$perm" = 700 ]; then echo "::notice title=6: the profile folder is 0700 ($size)::${bdir#$PWD/}"; else echo "::error title=6: the profile folder is $perm, not 700 ($size)::$bdir"; fail=1; fi
    for tag in page downloads added nokey; do
      if [ -s "$PWD/shots/$n-$tag.png" ]; then echo "::notice title=11: picture $tag ($size)::$n-$tag.png"; else echo "::error title=11: no picture $tag ($size)::"; fail=1; fi
    done
  done
  # 3: Nexus's wait is the page's: at least 5 seconds between the press and the file.
  python3 - "$sl" <<'PY' || fail=1
import sys
lines = [l.split() for l in open(sys.argv[1]) if l.strip()]
bad = 0
press = [float(l[0]) for l in lines if "event=slow-pressed" in l[2]]
files = [float(l[0]) for l in lines if l[2].startswith("/files/TurbineTest") and len(l) == 4]
for p in press:
    nxt = [f for f in files if f > p]
    gap = (nxt[0] - p) if nxt else -1
    print(f"::notice title=3: Slow download wait::{gap:.1f} s from the press to the file request")
    if gap < 5:
        bad = 1
        print("::error title=3: the file came sooner than 5 seconds after the press::")
if not press:
    print("::error title=3: the stand-in never saw a Slow download press::")
    bad = 1
# 4: the stand-in never gets an nxm request. 5: nothing pressed while nobody pressed anything.
if any("nxm" in l[2] for l in lines):
    print("::error title=4: the stand-in got an nxm request::"); bad = 1
s = [float(l[0]) for l in lines if "name=idle-start" in l[2]]
e = [float(l[0]) for l in lines if "name=idle-end" in l[2]]
if s and e:
    quiet = [l for l in lines if s[0] < float(l[0]) < e[0] and ("-pressed" in l[2] or l[2].startswith("/files/"))]
    print(f"::notice title=5: 30 seconds with no input::{len(quiet)} button requests, {e[0] - s[0]:.0f} s")
    if quiet: bad = 1; print("::error title=5: a button request came with no input::")
else:
    print("::error title=5: the idle marks are missing::"); bad = 1
sys.exit(bad)
PY
  # 6: a second run with the same profile sends the stand-in's cookie back (signed in stays signed in).
  curl -s "http://127.0.0.1:47800/mark?name=persist-run" > /dev/null
  h="$PWD/fh/browsertest-1280x800"
  bshoot browserpersist-1280x800 1280x800 browserpersist "$h" "$PWD/data/browsertest-1280x800" --game=$skyrim
  first=$(sed -n '/name=persist-run/,$p' "$sl" | grep "/skyrimspecialedition/mods/" | head -n1)
  case "$first" in *standin_session*) echo "::notice title=6: signed in stays signed in::$first" ;; *) echo "::error title=6: the second run didn't send the cookie back::$first"; fail=1 ;; esac
  # 1: a page that tries everything by itself, with no press: nothing is downloaded or turned on.
  n=browserpersist-1280x800
  plog="$PWD/data/browsertest-1280x800/turbine-mod-manager/activity.log"
  after=$(sed -n '/Browser check: second run loaded/,$p' "$plog" 2>/dev/null || true)
  pneed() { hit=$(printf '%s\n' "$after" | grep -Eh "$2" | head -n1 | cut -c1-200 || true)
            if [ -n "$hit" ]; then echo "::notice title=$1::$hit"
            else echo "::error title=$1::not found: $2"; printf '%s\n' "$after" | grep -h " web \| browser \| check " | tail -n 40 || true; fail=1; fi; }
  pneed "1: a script's nxm link with no press is refused" "Mod Manager Download link refused: nxm://skyrimspecialedition/mods/1/files/88 came with no press of yours"
  pneed "1: a frame's nxm link is refused (main frame only)" "Mod Manager Download link refused \(nxm://skyrimspecialedition/mods/1/files/77\): it came from a frame"
  before=$(printf '%s\n' "$after" | sed -n '1,/Browser check: no press, nothing downloaded/p')
  if printf '%s\n' "$before" | grep -q "Page newwindow"; then echo "::error title=1: a pop-up opened with no press::"; fail=1
  elif printf '%s\n' "$before" | grep -q "Page popupblocked"; then echo "::notice title=1: a pop-up with no press is refused::Page popupblocked"
  else echo "::notice title=1: a pop-up with no press didn't open::Chromium stopped it before Turbine saw it"; fi
  pneed "1: a download with no press is refused" "Download refused \(127.0.0.1\): Turbine only takes a download you start yourself"
  pneed "1: nothing downloaded or turned on with no press" "Browser check: no press, nothing downloaded: yes"
  pneed "1b: going to an outside site drops the press" "The page left Nexus \(http://127.0.0.1:47801/bounce\)"
  pneed "1b: the bounce back to Nexus's Mod Manager Download page is refused" "Mod Manager Download link refused: nxm://skyrimspecialedition/mods/1/files/55 came with no press of yours"
  pneed "1b: nothing downloaded after the bounce" "Browser check: after a bounce off Nexus and back, nothing downloaded: yes"
  pneed "1c (M5): another mod's link asks first: Not now and no input for the first second, then the ring on Get it (I17)" "Browser check: another mod's link asks: Get .* \(.*\) for .*\?, at first A Not now, B Not now, input off; after the quiet second A Get it, B Not now"
  pneed "1c (M5): ... logged as asking" "Turbine asks before it downloads: nxm://skyrimspecialedition/mods/2/files/57"
  pneed "1c (H1c): a tap on an outside page asks first" "Browser check: a tap off Nexus asks: Get .* \(.*\) for .*\?, at first A Not now, B Not now, input off; after the quiet second A Get it, B Not now"
  pneed "1c (H1c): ... logged as asking" "Turbine asks before it downloads: nxm://skyrimspecialedition/mods/1/files/56"
  pneed "1c: Not now downloads nothing" "Browser check: after Not now, nothing downloaded: yes"
  asked=$(printf '%s\n' "$after" | grep -E "pretending to download nxm://skyrimspecialedition/mods/(1/files/56|2/files/57)" || true)
  if [ -n "$asked" ]; then echo "::error title=1c: a link that asked downloaded anyway::"; fail=1; fi
  got=$(printf '%s\n' "$after" | grep -E "from the browser \(scheme handler, from [^)]*\): Turbine asks before it downloads: nxm://.*files/(55|66|77|88)|pretending to download nxm://skyrimspecialedition/mods/1/files/(55|66|77|88)" || true)
  if [ -n "$got" ]; then
    echo "::error title=1: a link with no press got through::"; fail=1; fi
  img=$(printf '%s\n' "$after" | grep -h "files/66" | head -n1 | cut -c1-200 || true)
  echo "::notice title=1: the frame's nxm image::${img:-never reached Turbine (Chromium blocked it)}"
  pneed "10 (I10): closing midway says the download goes on, with Watch it" "Still downloading 1 mod. It.s added by itself."
  pneed "10: ... with Getting 1 mod at the top and Nexus downloads in the game menu" "Browser check: closed while downloading; at the top: Getting 1 mod; game menu has Nexus downloads: yes"
  pneed "10: ... and it lands" "Browser check: the download went on after the close: .*(added|asked)"
  pneed "10: ... and the screen doesn't jump to it when it lands" "New mod from a download that went on after Nexus closed: the focus stays where it is"
  echo "Stand-in log:"; cat "$sl" | cut -c1-200
  kill $standin $xvfb 2>/dev/null || true
else
  echo "::error title=No Xvfb::the Nexus page checks didn't run"; fail=1
fi
fi
# 0.56, Mods for any game: Lantern Hollow (Unreal, anti-cheat, two Nexus games with its name) and Brass Pier (Data and
# Mods folders, one exact Nexus match), with Nexus's game list from a recorded copy.
NG="--env=TURBINE_NEXUS_GAMES=$PWD/samples/nexus_games_list.json"
lh=steam:3999901
bp=steam:3999902
for size in $SIZES; do
  SHOOT_ENV="$NG" shoot "basicgame-$size" "$size" basicgame "$PWD/home" --game=$bp
  need "basicgame-$size" "Any game: first visit card ($size)" "First visit card: Mods for Brass Pier"
  need "basicgame-$size" "Any game: an exact Nexus name is sure ($size)" "Nexus game for Brass Pier: brasspier"
  SHOOT_ENV="$NG" shoot "anticheat-$size" "$size" basicgame "$PWD/home" --game=$lh
  need "anticheat-$size" "Any game: anti-cheat card ($size)" "Anti-cheat card: Lantern Hollow \(Easy Anti-Cheat\)"
  need "anticheat-$size" "Any game: a subtitle asks ($size)" "Nexus game for Lantern Hollow: ask \(2 candidates\)"
  SHOOT_ENV="$NG" shoot "nexuspick-$size" "$size" nexuspick "$PWD/home" --game=$lh
  need "nexuspick-$size" "Any game: Which game on Nexus ($size)" "Which game on Nexus: Lantern Hollow \(2 candidates\)"
  SHOOT_ENV="$NG" shoot "basicdetails-$size" "$size" details "$PWD/home" --game=$bp
  fresh "place-$size"; h="$PWD/fh/place-$size"
  SHOOT_ENV="$NG" shoot "place-$size" "$size" place "$h" --game=$bp "--selftest-import=$h/Downloads/Odd Layout 1.0.zip"
  need "place-$size" "Any game: Where do these files go? ($size)" "Asked where files go: Odd Layout"
  fresh "overwrite-$size"; h="$PWD/fh/overwrite-$size"
  SHOOT_ENV="$NG" shoot "overwrite-$size" "$size" overwrite "$h" --game=$bp "--selftest-import=$h/Downloads/Pier Fix 1.0.zip"
  need "overwrite-$size" "Any game: the question about the game's files ($size)" "Asked about files Brass Pier already has: Pier Fix.*Data/pier.txt"
  fresh "basicmenu-$size"; h="$PWD/fh/basicmenu-$size"
  SHOOT_ENV="$NG" shoot "basicmenu-$size" "$size" basicmenu "$h" --game=$bp "--selftest-import=$h/Downloads/Brass Lamps 1.0.zip"
  need "basicmenu-$size" "Any game: data-folder rule ($size)" "Placed by rule data-folder"
  need "basicmenu-$size" "Any game: Change where it goes ($size)" "Mod menu: .*Change where it goes"
  fresh "checkbasic-$size"; h="$PWD/fh/checkbasic-$size"
  SHOOT_ENV="$NG" shoot "checkbasic-$size" "$size" check "$h" --game=$lh "--selftest-import=$h/Downloads/Wand Glow 1.0.zip"
  # 0.59 (I4): anti-cheat in a game with no data file means no mods (the unreal-paks rule is in the engine tests).
  need "checkbasic-$size" "I4: an unlisted anti-cheat game refuses mods ($size)" "needs its anti-cheat to play at all"
done
# 0.56 and 0.57, layout: a renamed mod, the bars hidden with the details panel; no top bar, the game's name, tabs
# and Play in the bottom bar; the Y pop-up on Get mods; Get mods scrolled to full screen and back.
for size in $SIZES; do
  shoot "tucked-$size" "$size" tucked "$PWD/home" --game=$skyrim "--selftest-import=$dl/Brighter Lanterns 1.2.zip,$dl/Lantern Glow SKSE 2.0.zip"
  need "tucked-$size" "Rename ($size)" "Renamed .* to Lanterns of the Road"
  need "tucked-$size" "A short list keeps the bars ($size)" "Short list keeps the bars: true"
  need "tucked-$size" "The bottom bar names the game and has Play, no top bar ($size)" "Bottom bar: Skyrim SE \\| [^|]* \\| tabs (3|caps only) \\| Play shown \\| top bar none"
  need "tucked-$size" "The bars hide going down ($size)" "Bars hidden: true; panel: true"
  shoot "findpop-$size" "$size" findpop "$PWD/home" --game=$skyrim
  need "findpop-$size" "Get mods: Y opens the pop-up ($size)" "Find pop-up \\(Get mods\\): field, Sort 5, Category"
  need "findpop-$size" "Get mods pop-up: Sort and Category buttons, B Done, X Clear all ($size)" "Get mods pop-up: Sort: Trending/.*; Category: All/.* \\| legend A Choose · B Done · X Clear all · Y Keyboard \\| sheet findpop"
  need "findpop-$size" "Get mods: no search row, no top bar ($size)" "Get mods page: search row none, top bar none"
  shoot "fullscroll-$size" "$size" fullscroll "$PWD/home" --game=$skyrim
  need "fullscroll-$size" "Get mods: down to full screen ($size)" "Full screen: bars hidden true, rail hidden"
  need "fullscroll-$size" "Get mods: Up brings the bars back ($size)" "Full screen, Up: bars hidden false"
  need "fullscroll-$size" "Get mods: Down hides them again ($size)" "Full screen, Down again: bars hidden true"
  shoot "why-$size" "$size" why "$PWD/home" --game=$skyrim "--selftest-import=$dl/Lantern Glow SKSE 2.0.zip"
  need "why-$size" "Why won't it start ($size)" "Why won't it start: .*(isn't installed|Try it without mods)"
done
# 0.57, anti-cheat games (a home of their own): Elden Ring asks once before Play with mods; its mod goes into the
# Turbine-owned me3 profile and never the game folder; Before you play says me3 comes next; Halo's guide; Destiny 2
# (online only) refuses mods.
for size in $SIZES; do
  rm -rf "$PWD/fh/ac-$size"; python3 tools/fake-home.py "$PWD/fh/ac-$size" --anticheat > /dev/null; h="$PWD/fh/ac-$size"
  er=steam:1245620
  shoot "acask-$size" "$size" acask "$h" --game=$er "--selftest-import=$h/Downloads/Golden Armor 1.0.zip"
  need "acask-$size" "Anti-cheat: Elden Ring is played through me3 ($size)" "Anti-cheat rules: me3, ask true, off false, me3 ready false, 1 mods; bar: Elden Ring \\| .* · Mods offline$"
  need "acask-$size" "I2: one question before the first modded Play, A plays ($size)" "Anti-cheat sheet: msg Play Elden Ring with mods\\? \\| buttons Play with mods, offline, Not now$"
  shoot "acdeploy-$size" "$size" acdeploy "$h" --game=$er "--selftest-import=$h/Downloads/Golden Armor 1.0.zip"
  need "acdeploy-$size" "Anti-cheat: Before you play says me3 downloads now, not as a warning ($size)" "Check before Play: .*ok Anti-cheat: off while mods are on \\(offline only\\).*ok me3 downloads now"
  g="$h/.local/share/Steam/steamapps/common/ELDEN RING"
  if ran "acdeploy-$size"; then
  inprofile=$(find "$h" "$PWD/data/acdeploy-$size" -path "*me3-profile/mods/parts/am_f_9000.partsbnd.dcx" 2>/dev/null | head -n1)
  if [ -n "$inprofile" ] && [ ! -e "$g/Game/parts/am_f_9000.partsbnd.dcx" ] && [ "$(find "$g" -type f | wc -l)" -eq 4 ]; then
    echo "::notice title=Anti-cheat: the mod is in the me3 profile, the game folder untouched ($size)::$inprofile"
  else echo "::error title=Anti-cheat: the mod isn't only in the me3 profile ($size)::profile: ${inprofile:-none}"; find "$g" -type f; fail=1; fi
  fi
  shoot "acguide-$size" "$size" acguide "$h" --game=steam:976730
  need "acguide-$size" "Anti-cheat: Halo's own switch in the game menu ($size)" "Game menu: .*How to play Halo MCC with mods"
  need "acguide-$size" "Anti-cheat: Halo's guide ($size)" "Anti-cheat guide: Halo MCC"
  shoot "acrefuse-$size" "$size" acrefuse "$h" --game=steam:1085660 "--selftest-import=$h/Downloads/Guardian Colors 1.0.zip"
  need "acrefuse-$size" "Anti-cheat: an online-only game refuses mods ($size)" "needs its anti-cheat to play at all"
  need "acrefuse-$size" "Anti-cheat: the refusal on screen ($size)" "Refused: tabs 1, status Needs its anti-cheat: no mods, Play shown"
  # 0.58: Seamless Co-op settings (its own copy; the session password first), Play Seamless Co-op in the bar, and
  # Elden Ring's saves by kind.
  shoot "coopsettings-$size" "$size" coopsettings "$h" --game=$er "--selftest-import=$h/Downloads/Seamless Co-op 2.0.1.zip|loader|seamless|2.0.1"
  need "coopsettings-$size" "Seamless Co-op settings in the game menu ($size)" "Seamless in: true; game menu: .*Co-op settings"
  need "coopsettings-$size" "Seamless Co-op settings: the password first, toggles, lists and numbers ($size)" "Seamless page: Seamless Co-op settings \| first cooppassword \| cooppassword:text, allow_invaders:toggle, overhead_player_display:list, default_boot_master_volume:number, enemy_health_scaling:number"
  need "coopsettings-$size" "I3: co-op is chosen at Play, no separate button ($size)" "Co-op chosen at Play: true"
  need "coopsettings-$size" "The bar with Co-op: nothing overlaps, Play is the one button ($size)" "Bar check \(coopbar, .* 0 problems \| .*wheel, Play"
  # 0.6 (I3): the Play choice, the password step, the remembered choice, settings from three places, saved edits.
  rm -rf "$PWD/fh/coop-$size"; python3 tools/fake-home.py "$PWD/fh/coop-$size" --anticheat > /dev/null; hc="$PWD/fh/coop-$size"
  shoot "coopcheck-$size" "$size" coopcheck "$hc" --game=$er "--selftest-import=$hc/Downloads/Seamless Co-op 2.0.1.zip|loader|seamless|2.0.1"
  n="coopcheck-$size"
  need $n "I3: Play asks with, without, co-op and its settings ($size)" "Co-op choice: Play with mods; Play Seamless Co-op; Play without mods; Co-op settings \\| focus Play with mods \\| Y Co-op settings"
  need $n "Seamless Co-op loads only with its own Play: said at the Play choice and on its row ($size)" "Co-op only: the Play choice says \"Seamless Co-op loads only with Play Seamless Co-op. A plays the highlighted one.\"; Play with mods: Solo, [0-9]+ mods? on. Seamless Co-op stays out.; its row in My mods: Loads only with Play Seamless Co-op$"
  need $n "I3: no password: its settings open on it with the keyboard, nothing started ($size)" "Co-op password step: picked Play Seamless Co-op, sheet settingsedit with the keyboard \\(ask\\), focus cooppassword, waiting true, Play asked 0 times"
  need $n "I3: with the password typed, co-op goes on ($size)" "Co-op after the password: sheet .*, waiting false, Play asked 1 times, co-op true"
  need $n "I3: the choice is remembered ($size)" "Co-op choice remembered: focus Play Seamless Co-op, Play says Co-op"
  need $n "I3: settings from the Play choice ($size)" "Co-op settings from the Play choice \\(Y\\): Seamless Co-op settings"
  need $n "I3: settings first in the game menu ($size)" "Game menu first row: Co-op settings"
  need $n "I3: settings from the game menu ($size)" "Co-op settings from the game menu: Seamless Co-op settings"
  need $n "I3: settings from the mod's X menu ($size)" "Co-op settings from the mod's X menu: Seamless Co-op settings"
  need $n "I12: edits save by themselves ($size)" "Co-op edits saved by themselves: 0 waiting"
  need $n "Co-op tile: asked once after the install, one press, the restart said first ($size)" "Co-op tile question: asked by itself 0 times; after the install: asked, Add a Co-op tile to Steam \\| Restart Steam and add it; Not now; B: sheet \"\"; a second time: not asked$"
  need $n "Co-op tile: its row on the Co-op settings page ($size)" "Co-op settings page: row Co-op tile in Steam \\[Add\\], A Add, A opens: Add a Co-op tile to Steam$"
  if ran "coopcheck-$size"; then
  ini=$(find "$hc" "$PWD/data/$n" -name ersc_settings.ini -not -path "*Downloads*" 2>/dev/null | xargs grep -l "dragon" 2>/dev/null | head -n1)
  if [ -n "$ini" ] && grep -q "^cooppassword = dragon" "$ini" && grep -q "^enemy_health_scaling = 50" "$ini" && grep -Eq "^allow_invaders = [01]" "$ini"; then echo "::notice title=I3: the ini has the password, the toggle and the number ($size)::$ini"
  else echo "::error title=I3: the ini doesn't have the edits ($size)::${ini:-no ersc_settings.ini with the password}"; (find "$hc" "$PWD/data/$n" -name ersc_settings.ini -exec sh -c 'echo "$1"; cat "$1"' _ {} \; 2>/dev/null | head -n 40); fail=1; fi
  fi
  # 0.62 (the owner's device): Seamless Co-op never starts with an empty session password. Every way a co-op start
  # comes, with the password empty: nothing starts and Co-op settings open on the password with the keyboard.
  rm -rf "$PWD/fh/cooppw-$size"; python3 tools/fake-home.py "$PWD/fh/cooppw-$size" --anticheat > /dev/null; hp="$PWD/fh/cooppw-$size"
  shoot "cooppw-$size" "$size" cooppw "$hp" --game=$er "--selftest-import=$hp/Downloads/Seamless Co-op 2.0.1.zip|loader|seamless|2.0.1"
  n="cooppw-$size"
  stop="nothing started, sheet settingsedit with the keyboard \\(ask\\), focus cooppassword, waiting true$"
  need $n "Co-op password: empty at the start ($size)" "Co-op password check: at the start the password is empty$"
  need $n "Co-op password: the Play choice stops for it ($size)" "Co-op password check: The Play choice: $stop"
  need $n "Co-op password: Start held stops for it ($size)" "Co-op password check: Start held \\(last choice coop\\): $stop"
  need $n "Co-op password: Play options stops for it ($size)" "Co-op password check: Play options \\(wheel slot Play options\\): $stop"
  need $n "Co-op password: the Co-op tile and Steam's menu stop for it ($size)" "Co-op password check: The Co-op tile or Steam's menu: $stop"
  need $n "Co-op password: the Play job itself stops for it ($size)" "Co-op password check: The Play job stopped it itself: 1 times$"
  need $n "Co-op password: the Play job opens its settings ($size)" "Co-op password check: The Play job: $stop"
  need $n "Co-op password: the Play job says why ($size)" "Seamless Co-op not started: no session password"
  need $n "Co-op password: B and B leave nothing waiting ($size)" "Co-op password check: The tile, then B and B: sheet \"\", waiting false, Play asked 0 times$"
  need $n "Co-op password: two quotes are empty ($size)" "Co-op password check: A password of two quotes: nothing started, the password counts as empty$"
  need $n "Co-op password checks ($size)" "Co-op password check done: 0 problems"
  # The same from outside the window, as the Co-op tile and the Steam menu plugin ask: nothing is launched, the
  # answer says the password is missing, and the tile leaves a request for Turbine to open on the game.
  if ran "$n"; then
    for how in "" "--from-tile"; do
      out=$(timeout 60 dbus-run-session -- flatpak run --env=QT_QPA_PLATFORM=offscreen --env=TURBINE_HOME="$hp" --env=TURBINE_DATA="$PWD/data/$n" \
        --filesystem="$hp" --filesystem="$PWD/data" io.github.ravenor907.TurbineModManager --play-coop "$er" --json $how 2>&1 || true)
      if echo "$out" | grep -q '"password":false' && ! echo "$out" | grep -q '"launch"'; then echo "::notice title=Co-op password: --play-coop ${how:-from the Steam menu} starts nothing ($size)::$(echo "$out" | tail -n1 | cut -c1-200)"
      else echo "::error title=Co-op password: --play-coop ${how:-from the Steam menu} didn't stop for the empty password ($size)::$(echo "$out" | tail -n3 | tr '\n' ' ' | cut -c1-300)"; fail=1; fi
    done
    need $n "Co-op password: the tile opens Turbine on the game first ($size)" "Co-op tile: Turbine opens on steam:1245620 first"
  fi
  shoot "ersaves-$size" "$size" ersaves "$h" --game=$er
  need "ersaves-$size" "Elden Ring saves: normal, modded and co-op apart ($size)" "Saves by kind: solo 1 files, [0-9]+ backups; modded 1 files, [0-9]+ backups; coop 1 files, [0-9]+ backups"
done
# 0.58: Edit settings on a mod (Right steps the number, Start saves; the edited copy goes into the game), and the
# game update warning (A plays without mods until they update).
for size in $SIZES; do
  fresh "modsettings-$size"; h="$PWD/fh/modsettings-$size"
  shoot "modsettings-$size" "$size" modsettings "$h" --game=$skyrim "--selftest-import=$h/Downloads/Lantern Glow SKSE 2.0.zip"
  need "modsettings-$size" "Edit settings in the mod menu ($size)" "Mod menu: .*Edit settings"
  need "modsettings-$size" "Edit settings: the number setting ($size)" "Settings page: settingsedit \| [0-9]+ rows \| focus Glow number 1.5"
  need "modsettings-$size" "Edit settings: the first Right after opening changes nothing ($size)" "First Right after Edit settings opened: nothing changed"
  need "modsettings-$size" "Edit settings: Right steps it ($size)" "After Right: 1.6, 1 change; legend A Type · B Back · X Options"
  need "modsettings-$size" "Edit settings: saved, password-safe log ($size)" "Edit settings saved: .*LanternGlow.ini: Glow 1.5 to 1.6"
  need "modsettings-$size" "Edit settings: the page shows your version after Save ($size)" "After Save: sheet settingsedit, 0 changes, edited true"
  need "modsettings-$size" "Edit settings: X options switch text and list ($size)" "Edit settings X: Options, first Show as text; then as text, back as a list"
  if ran "modsettings-$size"; then
  ini=$(find "$h" -name LanternGlow.ini -path "*Data/SKSE/Plugins/*" -not -path "*turbine-mod-manager*" 2>/dev/null | head -n1)
  if [ -n "$ini" ] && grep -q "Glow=1.6" "$ini"; then echo "::notice title=Edit settings: the edited copy is in the game ($size)::$ini"
  else echo "::error title=Edit settings: the game doesn't have the edited value ($size)::${ini:-no LanternGlow.ini}: $(cat "$ini" 2>/dev/null)"; fail=1; fi
  fi
  # 0.6 (I8): Edit settings on the mod's panel, by a tap and by Right then A; none for a mod without settings.
  # 0.6: outcome checks by real presses and clicks (not pictures): A and B on every screen, nothing through a pop-up,
  # and the golden path from an empty game to Play and back.
  rm -rf "$PWD/fh/abcheck-$size"; python3 tools/fake-home.py "$PWD/fh/abcheck-$size" > /dev/null; h="$PWD/fh/abcheck-$size"
  shoot "abcheck-$size" "$size" abcheck "$h" --game=$skyrim "--selftest-import=$h/Downloads/Lantern Glow SKSE 2.0.zip,$h/Downloads/Brighter Lanterns 1.2.zip"
  need "abcheck-$size" "A selects and B goes back one step on every screen ($size)" "A and B \([0-9]+x[0-9]+\): 1[0-9] screens, 0 problems"
  if ran "abcheck-$size"; then grep -h "A and B, .*PROBLEM" "$PWD/data/abcheck-$size/turbine-mod-manager/activity.log" | head -n 5 | sed 's/^/::error title=A and B::/' || true; fi
  rm -rf "$PWD/fh/passthrough-$size"; python3 tools/fake-home.py "$PWD/fh/passthrough-$size" > /dev/null; h="$PWD/fh/passthrough-$size"
  shoot "passthrough-$size" "$size" passthrough "$h" --game=$skyrim "--selftest-import=$h/Downloads/Lantern Glow SKSE 2.0.zip,$h/Downloads/Brighter Lanterns 1.2.zip"
  need "passthrough-$size" "No click goes through a pop-up ($size)" "Pass-through \([0-9]+x[0-9]+\): [1-9][0-9]* clicks, 0 problems"
  if ran "passthrough-$size"; then grep -h "Pass-through, .*PROBLEM" "$PWD/data/passthrough-$size/turbine-mod-manager/activity.log" | head -n 5 | sed 's/^/::error title=Pass-through::/' || true; fi
  if ran "passthrough-$size"; then grep -h "Pass-through, .*skipped" "$PWD/data/passthrough-$size/turbine-mod-manager/activity.log" | cut -c1-200 | sed 's/^/::notice title=Pass-through skipped::/' || true; fi
  rm -rf "$PWD/fh/golden-$size"; python3 tools/fake-home.py "$PWD/fh/golden-$size" > /dev/null; h="$PWD/fh/golden-$size"
  # A clean game for the journey: without the fixture whose master is missing on purpose (other pictures test that).
  find "$h" -name "Lantern Paths.esp" -delete; find "$h" -name plugins.txt -exec sed -i '/Lantern Paths/d' {} +
  shoot "golden-$size" "$size" golden "$h" --game=$skyrim
  need "golden-$size" "Journey: X on an empty game opens Add from Downloads ($size)" "Journey add: sheet add, [1-9][0-9]* files, A Add"
  need "golden-$size" "Journey: A adds the mod, on ($size)" "Journey added: [^,]+ on, sheet none"
  need "golden-$size" "Journey: Start held shows the Play choice ($size)" "Journey Start held: Play options, focus Play with mods"
  need "golden-$size" "Journey: A plays it ($size)" "Journey played: Play started .* with [1-9] mods on"
  need "golden-$size" "Journey: A turns it off and on ($size)" "Journey A off and on: .* on again"
  need "golden-$size" "Journey: Uninstall leaves the row with Put back ($size)" "Journey uninstall: the row says .* removed, A Put back"
  need "golden-$size" "Journey: A puts it back ($size)" "Journey put back: .* is back in My mods"
  if ran "golden-$size" && ! grep -q "Journey put back" "$PWD/data/golden-$size/turbine-mod-manager/activity.log" 2>/dev/null; then
    grep -h "engine\|action\|Journey\|Sheet" "$PWD/data/golden-$size/turbine-mod-manager/activity.log" | tail -n 30 | sed 's/^/::notice title=golden log::/' || true; fi
  # 0.6 (break M1, M2): the Mod Manager Download question's guard, timed from its first frame.
  rm -rf "$PWD/fh/mmdguard-$size"; python3 tools/fake-home.py "$PWD/fh/mmdguard-$size" > /dev/null; h="$PWD/fh/mmdguard-$size"
  shoot "mmdguard-$size" "$size" mmdguard "$h" --game=$skyrim
  need "mmdguard-$size" "M8: at first Not now, input off, A ignored ($size)" "Guard at first: A Not now, B Not now, input off; A at [0-9]+ ms: ignored"
  need "mmdguard-$size" "M2: a press made in the first second but handled late is ignored ($size)" "Guard late press: made at 500 ms, handled at [0-9]+ ms: ignored"
  need "mmdguard-$size" "M8: after the quiet second the ring is on Get it ($size)" "Guard after the quiet second: ring on Get it, A Get it"
  need "mmdguard-$size" "M1: Left at about one second keeps Not now ($size)" "Guard Left at about 1 s: ring on Not now at [0-9]+ ms, A Not now"
  need "mmdguard-$size" "M1: then A gets nothing ($size)" "Guard A then: closed, nothing downloaded"
  need "mmdguard-$size" "N3: an A made before the ring moved, handled after, is Not now ($size)" "Guard stalled A: made at 1100 ms, ring on Get it, handled at [0-9]+ ms: Not now, nothing downloaded"
  # 0.6: Mods off from Steam's menu: the note in My mods and its Turn them back on (A, and a tap).
  rm -rf "$PWD/fh/heldcheck-$size"; python3 tools/fake-home.py "$PWD/fh/heldcheck-$size" > /dev/null; h="$PWD/fh/heldcheck-$size"
  shoot "heldcheck-$size" "$size" heldcheck "$h" --game=$skyrim "--selftest-import=$h/Downloads/Lantern Glow SKSE 2.0.zip,$h/Downloads/Brighter Lanterns 1.2.zip"
  need "heldcheck-$size" "Mods off from Steam's menu shows in My mods with its way back ($size)" "Held note: row 1 held \| Mods off from Steam's menu: 2 mods turned off \| legend A Turn them back on"
  need "heldcheck-$size" "Turn them back on by A ($size)" "Held back on by A: 2 of 2 on, note gone"
  need "heldcheck-$size" "Turn them back on by a tap ($size)" "Held back on by a tap: 2 of 2 on, note gone"
  if ran "heldcheck-$size" && ! grep -q "Held back on by a tap: 2 of 2 on" "$PWD/data/heldcheck-$size/turbine-mod-manager/activity.log" 2>/dev/null; then
    grep -h "engine\|action\|quick\|Held" "$PWD/data/heldcheck-$size/turbine-mod-manager/activity.log" | tail -n 25 | sed 's/^/::notice title=heldcheck log::/' || true; fi
  fresh "editbutton-$size"; h="$PWD/fh/editbutton-$size"
  shoot "editbutton-$size" "$size" editbutton "$h" --game=$skyrim "--selftest-import=$h/Downloads/Lantern Glow SKSE 2.0.zip,$h/Downloads/Brighter Lanterns 1.2.zip"
  if grep -q "Edit button: the panel doesn't fit here" "$PWD/data/editbutton-$size/turbine-mod-manager/activity.log" 2>/dev/null; then echo "::notice title=I8: no panel at $size::the row's X menu has Edit settings"; else
  need "editbutton-$size" "I8: a tap on Edit settings opens that mod's ($size)" "Edit button tapped: sheet settingsedit for Lantern Glow"
  need "editbutton-$size" "I8: Right then A opens it ($size)" "Edit button by Right then A: sheet settingsedit for Lantern Glow"
  need "editbutton-$size" "I8: no button for a mod without settings ($size)" "Edit button for a mod without settings \\(.*\\): none"
  fi
  fresh "gameupdate-$size"; h="$PWD/fh/gameupdate-$size"
  shoot "gameupdate-$size" "$size" gameupdate "$h" --game=$skyrim "--selftest-import=$h/Downloads/Lantern Glow SKSE 2.0.zip"
  need "gameupdate-$size" "Game update warning names the mods that may break ($size)" "Game update warning: .* build 99 to 100; may break: Lantern Glow"
  need "gameupdate-$size" "Game update warning offers Play without mods until they update ($size)" "Update sheet: msg .* updated \| Play without mods until they update, Keep my mods on"
done
if [ "$MODE" = 1280x800 ]; then
  fresh placestart; h="$PWD/fh/placestart"
  SHOOT_ENV="$NG" shoot placestart 1280x800 placestart "$h" --game=$bp "--selftest-import=$h/Downloads/Odd Layout 1.0.zip"
  need placestart "Any game: Start puts them here" "Placed by you: the game folder \(Odd Layout"
  fresh overwriteskip; h="$PWD/fh/overwriteskip"; g="$h/.local/share/Steam/steamapps/common/Brass Pier"
  SHOOT_ENV="$NG" shoot overwriteskip 1280x800 overwriteskip "$h" --game=$bp "--selftest-import=$h/Downloads/Pier Fix 1.0.zip"
  need overwriteskip "Any game: Skip those files" "Skipped existing .*Data/pier.txt"
  need overwriteskip "Any game: the game's program is never placed" "Refused BrassPier.exe"
  if ran "overwriteskip"; then
  if [ "$(cat "$g/Data/pier.txt")" = "the game's own pier" ] && [ ! -s "$g/BrassPier.exe" ] && [ -f "$g/Data/pier2.txt" ]; then
    echo "::notice title=Any game: the game's own files are untouched::pier.txt and BrassPier.exe as they were; pier2.txt added"
  else echo "::error title=Any game: the game's own files changed::"; ls -la "$g" "$g/Data"; fail=1; fi
  fi
fi
if [ "$MODE" = 1920x1080 ]; then
  # 5: no sheet opened without a press when Nexus is slow to show.
  hits=$(grep -h "opened with no press" "$PWD/data/opening8s-1920x1080/turbine-mod-manager/activity.log" 2>/dev/null || true)
  if [ -n "$hits" ]; then echo "::error title=5: a sheet opened with no press (opening8s-1920x1080)::$(echo "$hits" | head -n1 | cut -c1-200)"; fail=1; fi
fi
# 0.58: every picture's bottom bar: no hint under Play or another hint, nothing past the edge, the game's name whole.
bars=$(grep -h "Bar check .* items, [1-9][0-9]* problems" "$PWD"/data/*/turbine-mod-manager/activity.log 2>/dev/null || true)
nbars=$(cat "$PWD"/data/*/turbine-mod-manager/activity.log 2>/dev/null | grep -c "Bar check " || true)
if [ -n "$bars" ]; then
  echo "::error title=Bottom bar problems ($(echo "$bars" | wc -l) of $nbars pictures)::$(echo "$bars" | head -n 3 | cut -c1-400 | tr '\n' ' ')"
  echo "$bars" | head -n 20
  fail=1
else echo "::notice title=Bottom bar::$nbars pictures checked, no overlaps"; fi
ls -la shots
# Also into the log, so the pictures can be checked where artifacts can't be downloaded.
for f in shots/*.png; do echo "SHOT $(basename "$f") $(base64 -w0 "$f")"; done
exit $fail
