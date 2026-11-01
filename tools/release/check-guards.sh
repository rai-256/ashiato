#!/usr/bin/env bash
set -euo pipefail

root="${CHECK_GUARDS_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"
die() { echo "NG: $*" >&2; return 1; }
repo="${GITHUB_REPOSITORY:-rai-256/ashiato2}"

rules() {
  local json tag="${GITHUB_REF_NAME:-}"
  if [[ -z "$tag" ]]; then
    die '公開する印が無い'
    return 1
  fi
  json="$(gh api "repos/$repo/rules/tags/$tag" 2>/dev/null)" || die "印 $tag の規則を読めない"
  grep -Eq '"type"[[:space:]]*:[[:space:]]*"deletion"' <<<"$json" || die "印 $tag の削除を拒む規則が無い"
  grep -Eq '"type"[[:space:]]*:[[:space:]]*"(update|non_fast_forward)"' <<<"$json" || die "印 $tag の付け替えを拒む規則が無い"
}

environments() {
  local release nightly
  release="$(gh api "repos/$repo/environments/release/deployment-branch-policies" 2>/dev/null)" || { die 'release Environment の規則を読めない'; return 1; }
  nightly="$(gh api "repos/$repo/environments/nightly/deployment-branch-policies" 2>/dev/null)" || { die 'nightly Environment の規則を読めない'; return 1; }
  if ! python3 -c 'import json,sys
kind, expected = sys.argv[1:]
items = json.load(sys.stdin).get("branch_policies", [])
if len(items) != 1 or items[0].get("type") != kind or items[0].get("name") != expected:
    raise SystemExit(1)' tag 'v*' <<<"$release"; then
    die 'release Environment の対象が tag v* だけでない'
    return 1
  fi
  if ! python3 -c 'import json,sys
kind, expected = sys.argv[1:]
items = json.load(sys.stdin).get("branch_policies", [])
if len(items) != 1 or items[0].get("type") != kind or items[0].get("name") != expected:
    raise SystemExit(1)' branch main <<<"$nightly"; then
    die 'nightly Environment の対象が branch main だけでない'
    return 1
  fi
}

job_block() {
  local file="$1" job="$2"
  awk -v job="$job" '
    $0 == "  " job ":" { found=1; print; next }
    found && $0 ~ /^  [A-Za-z0-9_-]+:/ { exit }
    found { print }
  ' "$file"
}

workflow() {
  local ci="$root/.github/workflows/ci.yml" release="$root/.github/workflows/release.yml" nightly="$root/.github/workflows/nightly.yml"
  [[ -f "$ci" && -f "$release" && -f "$nightly" ]] || die 'workflow が不足している'
  grep -q 'tools/release/changes.sh code' "$ci" || die 'ci が共有の code 判定を使っていない'
  job_block "$ci" package-linux | grep -q 'if: needs.changes.outputs.code == '\''true'\''' || die 'Linux package が code=true でない'
  job_block "$ci" package-windows | grep -q 'if: needs.changes.outputs.code == '\''true'\''' || die 'Windows package が code=true でない'
  job_block "$ci" package-artifacts | grep -q 'if: needs.changes.outputs.code == '\''true'\''' || die 'artifact package が code=true でない'
  grep -q 'upload-artifact' "$ci" || die 'ci に upload-artifact が無い'
  grep -q 'cron: "0 15 \* \* \*"' "$nightly" || die 'nightly の cron が違う'
  job_block "$nightly" publish | grep -q "ASHIATO_NIGHTLY_PUBLISH == 'on'" || die 'nightly publish の条件が無い'
  job_block "$release" plan | grep -q 'version.sh plan' || die 'release の plan が無い'
  job_block "$nightly" plan | grep -q 'version.sh nightly-plan' || die 'nightly の nightly-plan が無い'
  job_block "$release" package-linux | grep -qE 'ASHIATO_APK_UNSIGNED(:[[:space:]]*|=)"?1' || die 'release の APK が unsigned でない'
  if grep -qE 'build.sh[[:space:]]+apk|sign-apk.sh' "$nightly"; then die 'nightly に APK 組み立てまたは署名がある'; return 1; fi
  job_block "$release" plan | grep -q 'version.sh plan' && job_block "$release" plan | grep -q 'fetch-depth: 0' || die 'release の判定 job が shallow checkout'
  job_block "$nightly" plan | grep -q 'version.sh nightly-plan' && job_block "$nightly" plan | grep -q 'fetch-depth: 0' || die 'nightly の判定 job が shallow checkout'
  [[ "$(job_block "$release" publish | grep -c 'contents: write')" == 1 ]] || die 'release publish の書く権限が無い'
  [[ "$(job_block "$nightly" publish | grep -c 'contents: write')" == 1 ]] || die 'nightly publish の書く権限が無い'
  [[ "$(grep -h -c 'contents: write' "$release" "$nightly" | awk '{s += $1} END {print s+0}')" == 2 ]] || die '書く権限が publish 以外にもある'
}

code_change_requires_package() {
  printf '%s\n' "$@" | "$root/tools/release/changes.sh" code
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
  printf '[{"type":"deletion"}]\n' >"$work/bad.json"
  if PATH="$work:$PATH" GH_RULES_JSON="$work/bad.json" GITHUB_REF_NAME=v1.2.3 "$0" rules --repo fake/repo >/dev/null 2>&1; then die '規則無しを通した'; fi
  echo "== Scenario: 印の規則が入っていれば、公開の前の確かめを通る"
  printf '[{"type":"deletion"},{"type":"update"}]\n' >"$work/good.json"
  PATH="$work:$PATH" GH_RULES_JSON="$work/good.json" GITHUB_REF_NAME=v1.2.3 "$0" rules --repo fake/repo >/dev/null
  printf '{"branch_policies":[{"type":"tag","name":"v*"}]}\n' >"$work/release-env.json"
  printf '{"branch_policies":[{"type":"branch","name":"main"}]}\n' >"$work/nightly-env.json"
  echo "== Scenario: 鍵の置き場は、印と main のそれぞれにだけ開いている"
  export GH_ENV_RELEASE_JSON="$work/release-env.json" GH_ENV_NIGHTLY_JSON="$work/nightly-env.json"
  repo=fake/repo
  PATH="$work:$PATH" environments >/dev/null
  local saved="$root/.github/workflows/release.yml"
  cp "$saved" "$work/release.yml"
  printf '{"branch_policies":[{"type":"tag","name":"v*"},{"type":"branch","name":"main"}]}\n' >"$work/release-env-too-open.json"
  export GH_ENV_RELEASE_JSON="$work/release-env-too-open.json"
  if PATH="$work:$PATH" environments >/dev/null 2>&1; then die 'Environment の開きすぎを通した'; fi
  printf '{"branch_policies":[{"type":"branch","name":"main"},{"type":"branch","name":"develop"}]}\n' >"$work/nightly-env-too-open.json"
  export GH_ENV_RELEASE_JSON="$work/release-env.json" GH_ENV_NIGHTLY_JSON="$work/nightly-env-too-open.json"
  if PATH="$work:$PATH" environments >/dev/null 2>&1; then die 'nightly Environment の開きすぎを通した'; fi
  echo "== Scenario: コードを触った PR では、配る形の組み立てが走る"
  code_change_requires_package crates/server/src/lib.rs || die 'コード変更を見落とした'
  grep -q 'tools/release/changes.sh code' "$root/.github/workflows/ci.yml" || die '共有の code 判定が無い'
  echo "== Scenario: 文書だけの PR では、配る形の組み立てを飛ばす"
  if code_change_requires_package docs/readme.md openspec/changes/x/spec.md; then die '文書変更をコード変更と判定した'; fi
  if grep -q 'grep -vE' "$root/.github/workflows/ci.yml"; then die 'code 判定の正本が YAML に残っている'; fi
  if sed 's#tools/release/changes.sh code#tools/release/not-changes.sh code#' "$root/.github/workflows/ci.yml" | grep -q 'tools/release/changes.sh code'; then die 'YAML の code 判定を検査できない'; fi
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
  if sed 's/contents: write/contents: read/' "$root/.github/workflows/release.yml" >"$fake_root/.github/workflows/release.yml" && CHECK_GUARDS_ROOT="$fake_root" "$0" workflows >/dev/null 2>&1; then die 'release publish の権限欠落を通した'; fi
  workflow
  echo "OK: rules・Environment・workflow の守りを検査した"
}

[[ "${1:-}" == --self-test ]] && { self_test; exit 0; }
kind="${1:-}"; shift || true
if [[ "$kind" == --repo ]]; then kind=rules; set -- --repo "$repo" "$@"; fi
while [[ $# -gt 0 ]]; do case "$1" in --repo) repo="$2"; shift 2;; *) die "不明な引数: $1";; esac; done
case "$kind" in rules) rules;; environments) environments;; workflows) workflow;; *) echo "使い方: $0 <rules|environments|workflows> [--repo <owner/name>]" >&2; exit 2;; esac
