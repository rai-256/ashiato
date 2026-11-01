#!/usr/bin/env bash
set -euo pipefail

root="${CHECK_GUARDS_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"
die() { echo "NG: $*" >&2; return 1; }
repo="${GH_REPOSITORY:-rai-256/ashiato2}"

rules() {
  local json
  json="$(gh api "repos/$repo/rules/tags/v*" 2>/dev/null)" || die "印 v* の規則を読めない"
  grep -Eq '"deletion"[[:space:]]*:[[:space:]]*true|"type"[[:space:]]*:[[:space:]]*"deletion"' <<<"$json" || { die "印 v* の削除を拒む規則が無い"; return 1; }
  grep -Eq '"update"[[:space:]]*:[[:space:]]*true|"type"[[:space:]]*:[[:space:]]*"update"' <<<"$json" || { die "印 v* の付け替えを拒む規則が無い"; return 1; }
}

environments() {
  local release nightly
  release="$(gh api "repos/$repo/environments/release/deployment-branch-policies" 2>/dev/null)" || { die 'release Environment の規則を読めない'; return 1; }
  nightly="$(gh api "repos/$repo/environments/nightly/deployment-branch-policies" 2>/dev/null)" || { die 'nightly Environment の規則を読めない'; return 1; }
  grep -Eq '"name"[[:space:]]*:[[:space:]]*"v\*"' <<<"$release" || { die 'release Environment の対象が v* でない'; return 1; }
  grep -Eq '"name"[[:space:]]*:[[:space:]]*"main"' <<<"$nightly" || { die 'nightly Environment の対象が main でない'; return 1; }
  [[ "$(grep -Eo '"name"[[:space:]]*:[[:space:]]*"[^"]+"' <<<"$release" | wc -l)" == 1 ]] || { die 'release Environment が開きすぎている'; return 1; }
  [[ "$(grep -Eo '"name"[[:space:]]*:[[:space:]]*"[^"]+"' <<<"$nightly" | wc -l)" == 1 ]] || { die 'nightly Environment が開きすぎている'; return 1; }
}

workflow() {
  local ci="$root/.github/workflows/ci.yml" release="$root/.github/workflows/release.yml" nightly="$root/.github/workflows/nightly.yml"
  [[ -f "$ci" && -f "$release" && -f "$nightly" ]] || die 'workflow が不足している'
  grep -q "code.*==.*true" "$ci" || die 'ci に code の出し分けが無い'
  grep -qE '^  package(-linux|-windows|-artifacts)?:|package.sh' "$ci" || die 'ci に package が無い'
  grep -q 'upload-artifact' "$ci" || die 'ci に upload-artifact が無い'
  grep -q 'cron: "0 15 \* \* \*"' "$nightly" || die 'nightly の cron が違う'
  grep -q "ASHIATO_NIGHTLY_PUBLISH == 'on'" "$nightly" || die 'nightly publish の条件が無い'
  grep -q 'version.sh plan' "$release" || die 'release の plan が無い'
  grep -q 'version.sh nightly-plan' "$nightly" || die 'nightly の nightly-plan が無い'
  grep -q 'ASHIATO_APK_UNSIGNED=1' "$release" || die 'release の APK が unsigned でない'
  if grep -qE 'build.sh[[:space:]]+apk|sign-apk.sh' "$nightly"; then die 'nightly に APK 組み立てまたは署名がある'; return 1; fi
  grep -q 'fetch-depth: 0' "$release" || die 'release の判定 job が shallow checkout'
  grep -q 'fetch-depth: 0' "$nightly" || die 'nightly の判定 job が shallow checkout'
  [[ "$(grep -h -c 'contents: write' "$release" "$nightly" | awk '{s += $1} END {print s+0}')" == 2 ]] || die '書く権限が publish 以外にもある'
}

code_change_requires_package() {
  printf '%s\n' "$@" | grep -vE '^(docs/|openspec/)|\.md$' | grep -q .
}

self_test() {
  local work fakegh
  work="$(mktemp -d "${TMPDIR:-/tmp}/release-guards.XXXXXX")"
  trap "rm -rf '$work'" EXIT
  fakegh="$work/gh"; cat >"$fakegh" <<'EOF'
#!/usr/bin/env bash
case "$*" in
  *rules/tags*) cat "${GH_RULES_JSON:?}" ;;
  *environments/release*) cat "${GH_ENV_RELEASE_JSON:?}" ;;
  *environments/nightly*) cat "${GH_ENV_NIGHTLY_JSON:?}" ;;
  *) exit 1 ;;
esac
EOF
  chmod +x "$fakegh"
  export PATH="$work:$PATH"
  echo "== Scenario: 印の規則が入っていなければ、公開しない"
  printf '{"deletion":false,"update":false}\n' >"$work/bad.json"
  if PATH="$work:$PATH" GH_RULES_JSON="$work/bad.json" gh="$fakegh" "$0" rules --repo fake/repo >/dev/null 2>&1; then die '規則無しを通した'; fi
  echo "== Scenario: 印の規則が入っていれば、公開の前の確かめを通る"
  printf '{"deletion":true,"update":true}\n' >"$work/good.json"
  PATH="$work:$PATH" GH_RULES_JSON="$work/good.json" "$0" rules --repo fake/repo >/dev/null
  printf '{"branch_policies":[{"name":"v*"}]}\n' >"$work/release-env.json"
  printf '{"branch_policies":[{"name":"main"}]}\n' >"$work/nightly-env.json"
  echo "== Scenario: 鍵の置き場は、印と main のそれぞれにだけ開いている"
  export GH_ENV_RELEASE_JSON="$work/release-env.json" GH_ENV_NIGHTLY_JSON="$work/nightly-env.json"
  repo=fake/repo
  PATH="$work:$PATH" environments >/dev/null
  local saved="$root/.github/workflows/release.yml"
  cp "$saved" "$work/release.yml"
  printf '{"branch_policies":[{"name":"v*"},{"name":"main"}]}\n' >"$work/release-env-too-open.json"
  export GH_ENV_RELEASE_JSON="$work/release-env-too-open.json"
  if PATH="$work:$PATH" environments >/dev/null 2>&1; then die 'Environment の開きすぎを通した'; fi
  echo "== Scenario: コードを触った PR では、配る形の組み立てが走る"
  code_change_requires_package crates/server/src/lib.rs || die 'コード変更を見落とした'
  grep -q "if: needs.changes.outputs.code == 'true'" "$root/.github/workflows/ci.yml" || die 'code=true の package 条件が無い'
  echo "== Scenario: 文書だけの PR では、配る形の組み立てを飛ばす"
  if code_change_requires_package docs/readme.md openspec/changes/x/spec.md; then die '文書変更をコード変更と判定した'; fi
  grep -q "code=false" "$root/.github/workflows/ci.yml" || die '文書だけの code=false が無い'
  echo "== Scenario: 開発版の作り直しの workflow は、ST54 まで携帯の APK を組む手順を持たない"
  if grep -qE 'build.sh[[:space:]]+apk|sign-apk.sh' "$root/.github/workflows/nightly.yml"; then die 'nightly に APK の手順がある'; fi
  echo "== Scenario: 開発版の作り直しは、日本時間の 0:00 に起きる"
  grep -q 'cron: "0 15 \* \* \*"' "$root/.github/workflows/nightly.yml" || die 'cron が違う'
  echo "== Scenario: 公開の設定が無ければ、印 nightly を動かす手順は走らない"
  grep -q "ASHIATO_NIGHTLY_PUBLISH == 'on'" "$root/.github/workflows/nightly.yml" || die '公開条件が無い'
  export GH_ENV_RELEASE_JSON="$work/release-env.json"
  local fake_root="$work/tree"
  mkdir -p "$fake_root/.github/workflows"
  cp "$root/.github/workflows/ci.yml" "$root/.github/workflows/release.yml" "$root/.github/workflows/nightly.yml" "$fake_root/.github/workflows/"
  printf '\n      - run: tools/release/build.sh apk out\n' >>"$fake_root/.github/workflows/nightly.yml"
  if CHECK_GUARDS_ROOT="$fake_root" "$0" workflows >/dev/null 2>&1; then die 'APK のある偽 nightly を通した'; fi
  workflow
  echo "OK: rules・Environment・workflow の守りを検査した"
}

[[ "${1:-}" == --self-test ]] && { self_test; exit 0; }
kind="${1:-}"; shift || true
if [[ "$kind" == --repo ]]; then kind=rules; set -- --repo "$repo" "$@"; fi
while [[ $# -gt 0 ]]; do case "$1" in --repo) repo="$2"; shift 2;; *) die "不明な引数: $1";; esac; done
case "$kind" in rules) rules;; environments) environments;; workflows) workflow;; *) echo "使い方: $0 <rules|environments|workflows> [--repo <owner/name>]" >&2; exit 2;; esac
