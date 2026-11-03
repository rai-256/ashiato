#!/usr/bin/env bash
set -euo pipefail

root="${CHECK_GUARDS_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"
die() { echo "NG: $*" >&2; return 1; }
repo="${GITHUB_REPOSITORY:-rai-256/ashiato2}"

# 印 v* の削除と付け替えを拒むルールセットがあるか（final review R4）。
# `rules/tags/<印>` の API は無いので、ルールセットの一覧と各ルールセットの詳細を読む。
# 数えるのは target=tag・enforcement=active・include に refs/tags/v*（か ~ALL）があり、exclude に当たらないもの。
# 当たるルールセットの規則を合わせて、deletion と update（または non_fast_forward）がそろえば通る。
rules() {
  local list id detail types="" ids
  list="$(gh api "repos/$repo/rulesets?per_page=100" 2>/dev/null)" || { die 'ルールセットの一覧を読めない'; return 1; }
  ids="$(python3 -c 'import json,sys
for r in json.load(sys.stdin):
    if r.get("target") == "tag" and r.get("enforcement") == "active":
        print(r["id"])' <<<"$list")" || { die 'ルールセットの一覧の形が違う'; return 1; }
  for id in $ids; do
    detail="$(gh api "repos/$repo/rulesets/$id" 2>/dev/null)" || { die "ルールセット $id を読めない"; return 1; }
    types+="$(python3 -c 'import json,sys
r = json.load(sys.stdin)
ref = (r.get("conditions") or {}).get("ref_name") or {}
inc, exc = ref.get("include") or [], ref.get("exclude") or []
covers = lambda pats: any(p in ("~ALL", "refs/tags/v*", "refs/tags/*") for p in pats)
if r.get("target") == "tag" and r.get("enforcement") == "active" and covers(inc) and not covers(exc):
    for rule in r.get("rules") or []:
        print(rule.get("type", ""))' <<<"$detail")"$'\n'
  done
  grep -qx deletion <<<"$types" || { die '印 v* の削除を拒む規則が無い'; return 1; }
  grep -Eqx 'update|non_fast_forward' <<<"$types" || { die '印 v* の付け替えを拒む規則が無い'; return 1; }
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
  local head_ref='ref: \${{ github.event.pull_request.head.sha || github.sha }}'
  for job in package-linux package-windows; do
    job_block "$ci" "$job" | grep -q "$head_ref" || die "ci の $job が配布物を組む commit を checkout しない"
  done
  local artifacts_block
  artifacts_block="$(job_block "$ci" package-artifacts)"
  grep -q 'uses: actions/checkout@' <<<"$artifacts_block" || die 'ci の package-artifacts に checkout が無い'
  local first_checkout
  first_checkout="$(awk '/uses: actions\/checkout@/{n++; if (n == 1) print; next} n == 1 {print} n == 2 {exit}' <<<"$artifacts_block")"
  ! grep -q 'ref:' <<<"$first_checkout" || die 'package-artifacts の作業 checkout に ref がある'
  grep -A4 -q "path: built" <<<"$artifacts_block" || die 'package-artifacts に built checkout が無い'
  grep -A4 -q "$head_ref" <<<"$artifacts_block" || die 'package-artifacts の built checkout が commit を指定しない'
  grep -q 'version.sh source built/Cargo.toml' <<<"$artifacts_block" || die 'package-artifacts の版番号が built からでない'
  grep -q -- '--expect-license built/LICENSE' <<<"$artifacts_block" || die 'package-artifacts の LICENSE が built からでない'
  if grep -qE 'build.sh[[:space:]]+apk|sign-apk.sh' "$nightly"; then die 'nightly に APK 組み立てまたは署名がある'; return 1; fi
  job_block "$release" plan | grep -q 'version.sh plan' && job_block "$release" plan | grep -q 'fetch-depth: 0' || die 'release の判定 job が shallow checkout'
  job_block "$nightly" plan | grep -q 'version.sh nightly-plan' && job_block "$nightly" plan | grep -q 'fetch-depth: 0' || die 'nightly の判定 job が shallow checkout'
  # 配る組み立ての重い自己試験は package-linux（Rust・Java・node・Android SDK がそろう job）で走らせる（code_verify R29）
  local heavy
  for heavy in check-artifacts.sh sign-apk.sh make-key.sh; do
    job_block "$ci" package-linux | grep -q "tools/release/$heavy --self-test" || die "ci の package-linux で $heavy の自己試験が走らない"
  done
  # 署名の台本にも期待の指紋を渡す（鍵の取り違えを署名の前に止める。code_verify R26）
  [[ "$(job_block "$release" publish | grep -c 'ASHIATO_EXPECT_CERT_SHA256: \${{ vars.ASHIATO_CERT_SHA256 }}\|ASHIATO_EXPECT_CERT_SHA256: "\${{ vars.ASHIATO_CERT_SHA256 }}"')" -ge 2 ]] || die 'release の署名または検査に期待の指紋が渡らない'
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
  *rulesets/[0-9]*) id="${!#}"; id="${id##*/}"; cat "${GH_RULESETS_DIR:?}/$id.json" ;;
  *rulesets*) cat "${GH_RULESETS_DIR:?}/list.json" ;;
  *environments/release*) cat "${GH_ENV_RELEASE_JSON:?}" ;;
  *environments/nightly*) cat "${GH_ENV_NIGHTLY_JSON:?}" ;;
  *) exit 1 ;;
esac
EOF
  chmod +x "$fakegh"
  export PATH="$work:$PATH"
  # ルールセットの偽の配布元（一覧 list.json と詳細 <id>.json）。GITHUB_REF_NAME は渡さない（要らない）
  rulesets() {
    local dir="$work/rulesets-$1" target="$2" enforcement="$3" include="$4" rules_json="$5"
    mkdir -p "$dir"
    printf '[{"id":7,"name":"tags","target":"%s","enforcement":"%s"}]\n' "$target" "$enforcement" >"$dir/list.json"
    printf '{"id":7,"target":"%s","enforcement":"%s","conditions":{"ref_name":{"include":[%s],"exclude":[]}},"rules":%s}\n' \
      "$target" "$enforcement" "$include" "$rules_json" >"$dir/7.json"
    printf '%s' "$dir"
  }
  local both='[{"type":"deletion"},{"type":"update"}]' dir
  echo "== Scenario: 印の規則が入っていなければ、公開しない"
  printf '[]\n' >"$work/empty-list.json"; mkdir -p "$work/rulesets-none"; cp "$work/empty-list.json" "$work/rulesets-none/list.json"
  if env -u GITHUB_REF_NAME GH_RULESETS_DIR="$work/rulesets-none" PATH="$work:$PATH" "$0" rules --repo fake/repo >/dev/null 2>&1; then die 'ルールセット無しを通した'; fi
  dir="$(rulesets delete-only tag active '"refs/tags/v*"' '[{"type":"deletion"}]')"
  if env -u GITHUB_REF_NAME GH_RULESETS_DIR="$dir" PATH="$work:$PATH" "$0" rules --repo fake/repo >/dev/null 2>&1; then die '付け替えの規則無しを通した'; fi
  dir="$(rulesets branch branch active '"refs/heads/main"' "$both")"
  if env -u GITHUB_REF_NAME GH_RULESETS_DIR="$dir" PATH="$work:$PATH" "$0" rules --repo fake/repo >/dev/null 2>&1; then die '枝のルールセットを印の規則とみなした'; fi
  dir="$(rulesets other-tags tag active '"refs/tags/release-*"' "$both")"
  if env -u GITHUB_REF_NAME GH_RULESETS_DIR="$dir" PATH="$work:$PATH" "$0" rules --repo fake/repo >/dev/null 2>&1; then die 'v* に当たらないルールセットを通した'; fi
  dir="$(rulesets disabled tag disabled '"refs/tags/v*"' "$both")"
  if env -u GITHUB_REF_NAME GH_RULESETS_DIR="$dir" PATH="$work:$PATH" "$0" rules --repo fake/repo >/dev/null 2>&1; then die '止めてあるルールセットを通した'; fi
  echo "== Scenario: 印の規則が入っていれば、公開の前の確かめを通る"
  dir="$(rulesets good tag active '"refs/tags/v*"' "$both")"
  env -u GITHUB_REF_NAME GH_RULESETS_DIR="$dir" PATH="$work:$PATH" "$0" rules --repo fake/repo >/dev/null
  # --repo は前にも後にも置ける。non_fast_forward も付け替えを拒む規則として数える
  env -u GITHUB_REF_NAME GH_RULESETS_DIR="$dir" PATH="$work:$PATH" "$0" --repo fake/repo rules >/dev/null
  dir="$(rulesets nff tag active '"~ALL"' '[{"type":"deletion"},{"type":"non_fast_forward"}]')"
  env -u GITHUB_REF_NAME GH_RULESETS_DIR="$dir" PATH="$work:$PATH" "$0" rules --repo fake/repo >/dev/null
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
  # ci の code 判定が共有の台本を呼ばなくなれば、workflows の検査が落ちる（final review R19）
  cp "$root/.github/workflows/release.yml" "$root/.github/workflows/nightly.yml" "$fake_root/.github/workflows/"
  sed 's#tools/release/changes.sh code#tools/release/not-changes.sh code#' "$root/.github/workflows/ci.yml" >"$fake_root/.github/workflows/ci.yml"
  if CHECK_GUARDS_ROOT="$fake_root" "$0" workflows >/dev/null 2>&1; then die 'YAML の code 判定が台本を呼ばないのを通した'; fi
  echo "== Scenario: PR の配布物は、PR の head の commit から組まれ、同じ commit の期待値で検査される"
  local ci_saved="$fake_root/.github/workflows/ci.yml"
  if sed '/ref: \${{ github.event.pull_request.head.sha || github.sha }}/d' "$ci_saved" >"$work/ci-missing-linux.yml" && cp "$work/ci-missing-linux.yml" "$ci_saved" && CHECK_GUARDS_ROOT="$fake_root" "$0" workflows >/dev/null 2>&1; then die 'Linux package の ref 欠落を通した'; fi
  cp "$root/.github/workflows/ci.yml" "$ci_saved"
  if sed '/package-windows:/,/^  [A-Za-z0-9_-]*:/{/ref: \${{ github.event.pull_request.head.sha || github.sha }}/d}' "$ci_saved" >"$work/ci-missing-windows.yml" && cp "$work/ci-missing-windows.yml" "$ci_saved" && CHECK_GUARDS_ROOT="$fake_root" "$0" workflows >/dev/null 2>&1; then die 'Windows package の ref 欠落を通した'; fi
  cp "$root/.github/workflows/ci.yml" "$ci_saved"
  if sed '/package-artifacts:/,/^  [A-Za-z0-9_-]*:/{/with:/{n;/ref:/d;}}' "$ci_saved" >"$work/ci-artifacts-ref.yml" && cp "$work/ci-artifacts-ref.yml" "$ci_saved" && CHECK_GUARDS_ROOT="$fake_root" "$0" workflows >/dev/null 2>&1; then die 'package-artifacts の作業 checkout の ref を通した'; fi
  cp "$root/.github/workflows/ci.yml" "$ci_saved"
  if sed '/path: built/d' "$ci_saved" >"$work/ci-missing-built.yml" && cp "$work/ci-missing-built.yml" "$ci_saved" && CHECK_GUARDS_ROOT="$fake_root" "$0" workflows >/dev/null 2>&1; then die 'built checkout の欠落を通した'; fi
  cp "$root/.github/workflows/ci.yml" "$ci_saved"
  if sed 's/--expect-license built\/LICENSE//' "$ci_saved" >"$work/ci-missing-license.yml" && cp "$work/ci-missing-license.yml" "$ci_saved" && CHECK_GUARDS_ROOT="$fake_root" "$0" workflows >/dev/null 2>&1; then die 'built LICENSE 欠落を通した'; fi
  cp "$root/.github/workflows/ci.yml" "$ci_saved"
  if sed 's#version.sh source built/Cargo.toml#version.sh source#' "$ci_saved" >"$work/ci-missing-version.yml" && cp "$work/ci-missing-version.yml" "$ci_saved" && CHECK_GUARDS_ROOT="$fake_root" "$0" workflows >/dev/null 2>&1; then die 'built 版番号欠落を通した'; fi
  cp "$root/.github/workflows/ci.yml" "$ci_saved"
  workflow
  echo "OK: rules・Environment・workflow の守りを検査した"
}

[[ "${1:-}" == --self-test ]] && { self_test; exit 0; }
usage() { echo "使い方: $0 <rules|environments|workflows> [--repo <owner/name>]（--repo は前後どちらでもよい）" >&2; exit 2; }
kind=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --repo) [[ $# -ge 2 && -n "$2" ]] || usage; repo="$2"; shift 2 ;;
    rules|environments|workflows) [[ -z "$kind" ]] || usage; kind="$1"; shift ;;
    *) usage ;;
  esac
done
case "$kind" in rules) rules;; environments) environments;; workflows) workflow;; *) usage;; esac
