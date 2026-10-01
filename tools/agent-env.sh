# harness2 が agent（implementer / reviewer / 証跡の取り直し）を起こすたびに source し、export の差分を渡す。
# agent に道具の場所を探させないため（実測 2026-09-24 ST06: fresh な implementer が毎回 JAVA_HOME / robolectric を
# grep し、見つけるまで gradle を試し直していた）。人間が手で `. ./tools/agent-env.sh` しても同じものが入る。
#
# - Android の道具の場所（tools/android-env.sh）
# - ASHIATO_ROBOLECTRIC_JARS: app/build.gradle.kts がこれを見て Robolectric をオフラインで動かす
#   （オンラインだと $HOME 直下のロックを掴み、sandbox の中で落ちる。2026-09-21 実測）
# - HARNESS_CODEX_WRITABLE_DIRS: Codex の sandbox で書けるようにする道具のキャッシュ（$HOME にあり、既定では書けない。
#   2026-09-21 実測）。harness2 が実在するものだけを --add-dir に足す
# - Codex のときだけ GRADLE_OPTS に daemon を使わない指定: daemon は sandbox の制限を持ったまま生き残り、
#   外のビルドを壊す（2026-09-21 実測）。harness2 は source するときに HARNESS_EXECUTOR（codex / claude / harness）を見せる
#
# Robolectric の android-all の jar は ~/.m2 からハードリンクで集める（実体は増えない）。**毎回集め直す** ——
# 「置き場が空のときだけ」だと、テストが @Config(sdk = …) で新しい版を要求しても足されず、
# `Path is not a file: …android-all-instrumented-14-….jar` で落ちた（2026-09-26 実測。ST06 の code-verify R30）。
# 1 本も無ければ変数を立てない —— 黙ってオフラインにすると「jar が無い」で落ち、原因が分からなくなる。
#
# 以前は harness2 の effects.py が持っていた（2026-10-01 に harness2 をプロジェクト非依存にしたとき移した）。
# 人間が手で source しても害は無い（HARNESS_EXECUTOR が無ければ GRADLE_OPTS は触らない）。
# source されるので exit しない・失敗で止めない。

_here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
[ -f "$_here/android-env.sh" ] && . "$_here/android-env.sh"

_jars="${ASHIATO_ROBOLECTRIC_JARS:-$HOME/.m2/robolectric-jars}"
if mkdir -p "$_jars" 2>/dev/null; then
  while IFS= read -r -d '' _j; do
    _n="$(basename "$_j")"
    [ "$(dirname "$_j")" = "$_jars" ] && continue
    [ -e "$_jars/$_n" ] || ln "$_j" "$_jars/$_n" 2>/dev/null || cp "$_j" "$_jars/$_n" 2>/dev/null || true
  done < <(find "$HOME/.m2" -name 'android-all*.jar' -type f -print0 2>/dev/null)
  if compgen -G "$_jars/*.jar" >/dev/null; then
    export ASHIATO_ROBOLECTRIC_JARS="$_jars"
  fi
fi
export HARNESS_CODEX_WRITABLE_DIRS="$HOME/.cargo:$HOME/.gradle:$HOME/.android"
if [ "${HARNESS_EXECUTOR:-}" = codex ]; then
  export GRADLE_OPTS="${GRADLE_OPTS:+$GRADLE_OPTS }-Dorg.gradle.daemon=false"
fi

unset _here _jars _j _n
true
