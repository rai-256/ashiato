#!/usr/bin/env bash
set -euo pipefail

script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
root=$(cd "$script_dir/.." && pwd)

usage() {
  echo "使い方: $0 [--self-test | --same-release-as <前> <後>]" >&2
  exit 2
}

resolve_all() {
  local tree=$1
  "$tree/collector-android/gradlew" -p "$tree/collector-android" --no-daemon --no-configuration-cache \
    --write-locks -q detekt :app:assembleDebug :app:assembleRelease :app:assembleDebugAndroidTest :app:testDebugUnitTest \
    :app:dependencies --configuration releaseRuntimeClasspath
}

cleanup_worktree() {
  local tree=$1 tmp=$2 status=$3
  git -C "$root" worktree remove --force "$tree" >/dev/null 2>&1 || true
  rm -rf -- "$tmp"
  return "$status"
}

check_fresh() {
  local tmp tree
  tmp=$(mktemp -d)
  tree="$tmp/tree"
  trap 'status=$?; cleanup_worktree "$tree" "$tmp"; trap - EXIT; exit "$status"' EXIT
  git -C "$root" worktree add --detach -q "$tree" HEAD
  resolve_all "$tree"
  if ! git -C "$tree" diff --quiet -- '*.lockfile' || [[ -n "$(git -C "$tree" ls-files --others --exclude-standard -- '*.lockfile')" ]]; then
    echo "ロックファイルが現在の宣言の解決と一致しない" >&2
    git -C "$tree" status --short -- '*.lockfile' >&2
    return 1
  fi
  echo "== Scenario: ロックは今の宣言の解決と一致する"
  cleanup_worktree "$tree" "$tmp" "$?"
  trap - EXIT
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
  tmp=$(mktemp -d)
  before_tree="$tmp/before"
  after_tree="$tmp/after"
  trap 'status=$?; git -C "$root" worktree remove --force "$before_tree" >/dev/null 2>&1 || true; git -C "$root" worktree remove --force "$after_tree" >/dev/null 2>&1 || true; rm -rf -- "$tmp"; trap - EXIT; exit "$status"' EXIT
  git -C "$root" worktree add --detach -q "$before_tree" "$before"
  git -C "$root" worktree add --detach -q "$after_tree" "$after"
  init="$tmp/release-init.gradle"
  make_release_init "$init"
  before_modules="$tmp/before.modules"
  after_modules="$tmp/after.modules"
  "$before_tree/collector-android/gradlew" -p "$before_tree/collector-android" --no-daemon --no-configuration-cache -q -I "$init" :app:ashiatoPrintReleaseModules | sort -u >"$before_modules"
  "$after_tree/collector-android/gradlew" -p "$after_tree/collector-android" --no-daemon --no-configuration-cache -q -I "$init" :app:ashiatoPrintReleaseModules | sort -u >"$after_modules"
  diff -u "$before_modules" "$after_modules"
  git -C "$root" worktree remove --force "$before_tree" >/dev/null 2>&1 || true
  git -C "$root" worktree remove --force "$after_tree" >/dev/null 2>&1 || true
  rm -rf -- "$tmp"
  trap - EXIT
}

self_test() {
  local tmp copied output
  tmp=$(mktemp -d)
  trap 'status=$?; rm -rf -- "$tmp"; trap - EXIT; exit "$status"' EXIT
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
  rm -rf -- "$tmp"
  trap - EXIT
}

case "${1:-}" in
  "") check_fresh ;;
  --self-test) self_test ;;
  --same-release-as) shift; check_same_release "$@" ;;
  *) usage ;;
esac
