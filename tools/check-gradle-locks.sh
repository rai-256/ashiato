#!/usr/bin/env bash
set -euo pipefail

script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
root=$(cd "$script_dir/.." && pwd)

usage() {
  echo "使い方: $0 [--self-test | --same-release-as <前> <後>]" >&2
  exit 2
}

# 全部の解決できる構成を解き直す。`:app:dependencies` が :app の構成（detekt・試験・計測テストの基盤を含む）を全部解き、
# AGP が組み立ての途中で作る `androidApis` だけは `compileDebugKotlin` が解く（design D11 の「単独では解けない構成」）。
# 組み立てと試験は走らせない —— CI の android の job が自分で走らせる（2026-11-09 実測: 空にしたロックから
# この 2 つで書き直すと、今のロックとバイト単位で一致・48 秒。前の assemble ×3 + 単体試験は 175 秒。review/code.md R18）。
# 足りない構成が出れば、ロックからその構成が消えて「一致しない」で落ちる（黙って通ることはない）。
resolve_all() {
  local tree=$1
  "$tree/collector-android/gradlew" -p "$tree/collector-android" --no-daemon --no-configuration-cache \
    --write-locks -q :app:dependencies :app:compileDebugKotlin >/dev/null
}

# 片付けは終了時の 1 つの trap が持つ。関数の local を trap から読むと、関数が失敗で抜けた後の trap では
# 変数が無く（set -u で止まり）、作業場所と一時ディレクトリが残る（review/code.md R12）。
cleanup_trees=()
cleanup_dirs=()
cleanup() {
  local status=$? t d
  for t in "${cleanup_trees[@]}"; do
    git -C "$root" worktree remove --force "$t" >/dev/null 2>&1 || true
  done
  for d in "${cleanup_dirs[@]}"; do
    rm -rf -- "$d"
  done
  exit "$status"
}
trap cleanup EXIT

# 一時ディレクトリを作って片付けの一覧に載せる（$(…) の中で載せると subshell で消えるので、名前で受ける）
new_tmp() {
  local dir
  dir=$(mktemp -d)
  cleanup_dirs+=("$dir")
  printf -v "$1" '%s' "$dir"
}

add_tree() {
  cleanup_trees+=("$1")
  git -C "$root" worktree add --detach -q "$1" "$2"
}

check_fresh() {
  local tmp tree
  new_tmp tmp
  tree="$tmp/tree"
  add_tree "$tree" HEAD
  resolve_all "$tree"
  if ! git -C "$tree" diff --quiet -- '*.lockfile' || [[ -n "$(git -C "$tree" ls-files --others --exclude-standard -- '*.lockfile')" ]]; then
    echo "ロックファイルが現在の宣言の解決と一致しない" >&2
    git -C "$tree" status --short -- '*.lockfile' >&2
    return 1
  fi
  echo "== Scenario: ロックは今の宣言の解決と一致する"
}

make_release_init() {
  cat >"$1" <<'GROOVY'
gradle.afterProject { project, state ->
    if (state.failure == null && project.path == ":app") {
        project.tasks.register("ashiatoPrintReleaseModules") {
            doLast {
                def modules = project.configurations.getByName("releaseRuntimeClasspath").incoming.resolutionResult.allComponents
                    .findAll { it.id instanceof org.gradle.api.artifacts.component.ModuleComponentIdentifier }
                    .collect { "${it.moduleVersion.group}:${it.moduleVersion.name}:${it.moduleVersion.version}" }
                    .toSet().sort()
                modules.each { println(it) }
            }
        }
    }
}
GROOVY
}

check_same_release() {
  [[ $# -eq 2 ]] || usage
  local before=$1 after=$2 tmp init before_tree after_tree before_modules after_modules
  new_tmp tmp
  before_tree="$tmp/before"
  after_tree="$tmp/after"
  add_tree "$before_tree" "$before"
  add_tree "$after_tree" "$after"
  init="$tmp/release-init.gradle"
  make_release_init "$init"
  before_modules="$tmp/before.modules"
  after_modules="$tmp/after.modules"
  "$before_tree/collector-android/gradlew" -p "$before_tree/collector-android" --no-daemon --no-configuration-cache -q -I "$init" :app:ashiatoPrintReleaseModules | sort -u >"$before_modules"
  "$after_tree/collector-android/gradlew" -p "$after_tree/collector-android" --no-daemon --no-configuration-cache -q -I "$init" :app:ashiatoPrintReleaseModules | sort -u >"$after_modules"
  diff -u "$before_modules" "$after_modules"
}

# 片付けの自己試験: ロックが合わずに check_fresh が失敗で抜けても、作業場所と一時ディレクトリを残さない。
# 解決（resolve_all）をロックを書き換えるだけの関数に替えた写しを、使い捨ての git の木で走らせる。
self_test_cleanup_on_mismatch() {
  local tmp repo rc=0
  new_tmp tmp
  repo="$tmp/repo"
  mkdir -p "$repo/tools" "$repo/collector-android/app" "$tmp/t"
  echo 'a:b:1=runtimeClasspath' >"$repo/collector-android/app/gradle.lockfile"
  python3 -c '
import re, sys
s = open(sys.argv[1], encoding="utf-8").read()
stub = "resolve_all() {\n  echo \"c:d:2=runtimeClasspath\" >>\"$1/collector-android/app/gradle.lockfile\"\n}"
s, n = re.subn(r"resolve_all\(\) \{.*?\n\}", lambda m: stub, s, count=1, flags=re.S)
assert n == 1
open(sys.argv[2], "w", encoding="utf-8").write(s)
' "$script_dir/check-gradle-locks.sh" "$repo/tools/check-gradle-locks.sh"
  git -C "$repo" init -q
  git -C "$repo" add collector-android tools
  git -C "$repo" -c user.name=self-test -c user.email=self-test@example.invalid commit -qm self-test
  TMPDIR="$tmp/t" bash "$repo/tools/check-gradle-locks.sh" >"$tmp/out" 2>&1 || rc=$?
  [[ "$rc" == 1 ]] || { cat "$tmp/out" >&2; echo "self-test NG: 合わないロックで rc=$rc" >&2; return 1; }
  grep -q 'ロックファイルが現在の宣言の解決と一致しない' "$tmp/out" || { cat "$tmp/out" >&2; return 1; }
  ! grep -q 'unbound variable' "$tmp/out" || { cat "$tmp/out" >&2; return 1; }
  [[ "$(git -C "$repo" worktree list | wc -l)" == 1 ]] || { git -C "$repo" worktree list >&2; echo "self-test NG: 作業場所が残った" >&2; return 1; }
  [[ -z "$(ls -A "$tmp/t")" ]] || { ls -A "$tmp/t" >&2; echo "self-test NG: 一時ディレクトリが残った" >&2; return 1; }
  echo "== Scenario: ロックが合わずに落ちても、作業場所と一時ディレクトリを残さない"
}

self_test() {
  local tmp copied output
  new_tmp tmp
  copied="$tmp/collector-android"
  cp -a "$root/collector-android" "$copied"
  cp "$root/Cargo.toml" "$tmp/Cargo.toml"
  sed -i '/implementation("androidx.core:core-ktx:1.17.0")/a\    releaseImplementation("junit:junit:4.13.2")' "$copied/app/build.gradle.kts"
  if output=$("$copied/gradlew" -p "$copied" --no-daemon --no-configuration-cache -q :app:assembleRelease 2>&1); then
    echo "self-test NG: ロックに無い依存を通した" >&2
    return 1
  fi
  grep -Eq 'dependency lock|lock state|junit' <<<"$output" || { echo "$output" >&2; return 1; }
  echo "== Scenario: ロックに無い依存を足すと、組み立てが落ちる"
  cat >>"$copied/app/build.gradle.kts" <<'KOTLIN'

val selfTestUnlocked by configurations.creating
dependencies.add("selfTestUnlocked", "junit:junit:4.13.2")
tasks.register("resolveSelfTestUnlocked") {
    doLast { configurations.getByName("selfTestUnlocked").resolve() }
}
KOTLIN
  if output=$("$copied/gradlew" -p "$copied" --no-daemon --no-configuration-cache -q :app:resolveSelfTestUnlocked 2>&1); then
    echo "self-test NG: ロックに無い構成を通した" >&2
    return 1
  fi
  grep -Eq 'dependency lock|lock state|junit' <<<"$output" || { echo "$output" >&2; return 1; }
  echo "== Scenario: ロックに無い構成を解こうとすると落ちる"
  self_test_cleanup_on_mismatch
}

case "${1:-}" in
  "") check_fresh ;;
  --self-test) self_test ;;
  --same-release-as) shift; check_same_release "$@" ;;
  *) usage ;;
esac
