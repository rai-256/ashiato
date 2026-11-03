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

approvals() {
  local reviewer="${reviewer_opt:-${ASHIATO_RELEASE_REVIEWER:-${repo%%/*}}}" details
  details="$(gh api "repos/$repo/environments/release" 2>/dev/null)" || { die 'release Environment の保護規則を読めない'; return 1; }
  if ! python3 -c 'import json,sys
reviewer = sys.argv[1]
data = json.load(sys.stdin)
rules = data.get("protection_rules") or []
required = [r for r in rules if r.get("type") == "required_reviewers"]
# API の形: reviewers[] = {"type": "User", "reviewer": {"login": ...}}（login は reviewer の下。/code-review R19）
if not required or not any(u.get("type") == "User" and (u.get("reviewer") or {}).get("login") == reviewer for r in required for u in (r.get("reviewers") or [])):
    raise SystemExit(1)' "$reviewer" <<<"$details"; then
    die "release Environment に承認者 $reviewer が無い"
    return 1
  fi
  if ! python3 -c 'import json,sys
if json.load(sys.stdin).get("can_admins_bypass") is not False:
    raise SystemExit(1)' <<<"$details"; then
    die 'release Environment で管理者が承認を飛ばせる'
    return 1
  fi
}

agent_credential() {
  local permissions
  permissions="$(gh api "repos/$repo" --jq .permissions 2>/dev/null)" || { die 'リポジトリの権限を読めない'; return 1; }
  if ! python3 -c 'import json,sys
p = json.load(sys.stdin)
if p.get("admin") is not False or p.get("maintain") is not False:
    raise SystemExit(1)' <<<"$permissions"; then
    die 'エージェントの資格情報に管理者または maintain の権限がある'
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

checkout_block() {
  local block="$1" wanted="$2"
  awk -v wanted="$wanted" '
    /^      - / {
      if ($0 ~ /^      - uses: actions\/checkout@/) {
        count++
        if (count > wanted) exit
        capture=(count == wanted)
      } else if (capture) {
        exit
      }
    }
    capture { print }
  ' <<<"$block"
}

# 公開の job の手順を割り、承認者の確かめの手順と署名の手順の番号を「<承認> <署名>」で出す（どちらも無ければ 0）。
# 字面でなく手順として見る: 注釈の行は数えず、承認の手順は run がちょうど `tools/release/check-guards.sh approvals` で、
# `if:` と `continue-on-error` を持たないものだけ（code_verify R15・/code-review R26）
publish_steps() {
  python3 -c 'import re,sys
steps=[]; cur=None
for line in sys.stdin.read().splitlines():
    if line.lstrip().startswith("#"): continue
    if re.match(r"      - ", line): cur=[line]; steps.append(cur)
    elif cur is not None and re.match(r"        \S", line): cur.append(line)
    elif cur is not None and not re.match(r"\s{9,}", line): cur=None
    elif cur is not None: cur.append(line)
approval=sign=0
for i, st in enumerate(steps, 1):
    keys=[re.sub(r"^      - ", "        ", l) for l in st]
    if not approval and any(re.fullmatch(r"        run: tools/release/check-guards\.sh approvals\s*", k) for k in keys) \
       and not any(re.match(r"        (if|continue-on-error):", k) for k in keys):
        approval=i
    if not sign and any("tools/release/sign-apk.sh" in k for k in keys):
        sign=i
print(approval, sign)'
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
    job_block "$ci" "$job" | grep -q "^[[:space:]]*$head_ref" || die "ci の $job が配布物を組む commit を checkout しない"
  done
  local approval_step sign_step
  # 無いときも 0 を出し、下の NG の文で落とす（pipefail で黙って落ちない。final review R7）
  read -r approval_step sign_step < <(job_block "$release" publish | publish_steps)
  [[ "$approval_step" -gt 0 && "$sign_step" -gt 0 && "$approval_step" -lt "$sign_step" ]] || die 'release publish が署名前に承認者を確かめない'
  local artifacts_block
  artifacts_block="$(job_block "$ci" package-artifacts)"
  grep -q 'uses: actions/checkout@' <<<"$artifacts_block" || die 'ci の package-artifacts に checkout が無い'
  local first_checkout built_checkout
  first_checkout="$(checkout_block "$artifacts_block" 1)"
  ! grep -q 'ref:' <<<"$first_checkout" || die 'package-artifacts の作業 checkout に ref がある'
  built_checkout="$(checkout_block "$artifacts_block" 2)"
  grep -q 'path: built' <<<"$built_checkout" || die 'package-artifacts に built checkout が無い'
  grep -q "^[[:space:]]*$head_ref" <<<"$built_checkout" || die 'package-artifacts の built checkout が commit を指定しない'
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
  *environments/release) cat "${GH_ENV_RELEASE_DETAILS_JSON:?}" ;;
  *environments/release*) cat "${GH_ENV_RELEASE_JSON:?}" ;;
  *environments/nightly*) cat "${GH_ENV_NIGHTLY_JSON:?}" ;;
  *repos/*) cat "${GH_REPO_JSON:?}" ;;
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
  printf '{"protection_rules":[],"can_admins_bypass":false}\n' >"$work/release-details-no-reviewer.json"
  printf '{"protection_rules":[{"type":"required_reviewers","reviewers":[{"type":"User","reviewer":{"login":"fake","id":1}}]}],"can_admins_bypass":true,"prevent_self_review":false}\n' >"$work/release-details-bypass.json"
  printf '{"protection_rules":[{"type":"required_reviewers","reviewers":[{"type":"User","reviewer":{"login":"fake","id":1}}]}],"can_admins_bypass":false,"prevent_self_review":false}\n' >"$work/release-details-good.json"
  printf '{"protection_rules":[{"type":"required_reviewers","reviewers":[{"type":"User","reviewer":{"login":"other","id":2}}]}],"can_admins_bypass":false}\n' >"$work/release-details-other.json"
  printf '{"admin":true,"maintain":false,"push":true}\n' >"$work/repo-admin.json"
  printf '{"admin":false,"maintain":false,"push":true}\n' >"$work/repo-agent.json"
  echo "== Scenario: 鍵の置き場は、印と main のそれぞれにだけ開いている"
  export GH_ENV_RELEASE_JSON="$work/release-env.json" GH_ENV_NIGHTLY_JSON="$work/nightly-env.json"
  repo=fake/repo
  PATH="$work:$PATH" environments >/dev/null
  echo "== Scenario: 承認者の無い配布用の Environment では、公開の前の確かめが落ちる"
  if GH_ENV_RELEASE_DETAILS_JSON="$work/release-details-no-reviewer.json" PATH="$work:$PATH" "$0" approvals --repo fake/repo --reviewer fake >/dev/null 2>&1; then die '承認者無しを通した'; fi
  echo "== Scenario: 管理者が承認を飛ばせる設定では、公開の前の確かめが落ちる"
  if GH_ENV_RELEASE_DETAILS_JSON="$work/release-details-bypass.json" PATH="$work:$PATH" "$0" approvals --repo fake/repo --reviewer fake >/dev/null 2>&1; then die '管理者の飛ばしを通した'; fi
  echo "== Scenario: 本人が承認者で管理者が飛ばせなければ、公開の前の確かめを通る"
  GH_ENV_RELEASE_DETAILS_JSON="$work/release-details-good.json" PATH="$work:$PATH" "$0" approvals --repo fake/repo --reviewer fake >/dev/null
  echo "== Scenario: 本人が承認者にいなければ、公開の前の確かめが落ちる"
  if GH_ENV_RELEASE_DETAILS_JSON="$work/release-details-other.json" PATH="$work:$PATH" "$0" approvals --repo fake/repo --reviewer fake >/dev/null 2>&1; then die '本人以外の承認者を通した'; fi
  echo "== Scenario: 管理者の資格情報では、エージェントの資格情報の確かめが落ちる"
  if GH_REPO_JSON="$work/repo-admin.json" PATH="$work:$PATH" "$0" agent-credential --repo fake/repo >/dev/null 2>&1; then die '管理者資格情報を通した'; fi
  echo "== Scenario: 管理者でない資格情報では、エージェントの資格情報の確かめを通る"
  GH_REPO_JSON="$work/repo-agent.json" PATH="$work:$PATH" "$0" agent-credential --repo fake/repo >/dev/null
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
  cp "$root/.github/workflows/ci.yml" "$root/.github/workflows/nightly.yml" "$fake_root/.github/workflows/"
  if sed 's/contents: write/contents: read/' "$root/.github/workflows/release.yml" >"$fake_root/.github/workflows/release.yml" && CHECK_GUARDS_ROOT="$fake_root" "$0" workflows >/dev/null 2>&1; then die 'release publish の権限欠落を通した'; fi
  cp "$root/.github/workflows/release.yml" "$fake_root/.github/workflows/release.yml"
  echo "== Scenario: 公開の job は、署名の前に承認者を確かめる"
  # approvals の行が無いときは、理由（NG の文）を出して落ちる（pipefail で黙って落ちない。final review R7）
  local no_approvals_output
  if sed '/tools\/release\/check-guards.sh approvals/d' "$root/.github/workflows/release.yml" >"$work/release-no-approvals.yml" && cp "$work/release-no-approvals.yml" "$fake_root/.github/workflows/release.yml" && no_approvals_output="$(CHECK_GUARDS_ROOT="$fake_root" "$0" workflows 2>&1)"; then die '承認者の確かめ無しを通した'; fi
  grep -qF 'NG: release publish が署名前に承認者を確かめない' <<<"$no_approvals_output" || die "承認者の確かめ無しで NG の文を出さない: $no_approvals_output"
  if awk '/check-guards.sh approvals/{line=$0; next} /sign-apk.sh/{print; print line; next} {print}' "$root/.github/workflows/release.yml" >"$work/release-swapped.yml" && cp "$work/release-swapped.yml" "$fake_root/.github/workflows/release.yml" && CHECK_GUARDS_ROOT="$fake_root" "$0" workflows >/dev/null 2>&1; then die '署名前でない承認者の確かめを通した'; fi
  cp "$root/.github/workflows/release.yml" "$fake_root/.github/workflows/release.yml"
  # 字面が残っていても、走らない・失敗を飲む承認の手順は数えない（code_verify R15・/code-review R26）
  local variant
  for variant in comment or-true if-false continue-on-error; do
    case "$variant" in
      comment) sed 's#^      - run: tools/release/check-guards.sh approvals$#      \# - run: tools/release/check-guards.sh approvals#' ;;
      or-true) sed 's#^\(      - run: tools/release/check-guards.sh approvals\)$#\1 || true#' ;;
      if-false) sed 's#^      - run: tools/release/check-guards.sh approvals$#      - if: false\n        run: tools/release/check-guards.sh approvals#' ;;
      continue-on-error) sed 's#^      - run: tools/release/check-guards.sh approvals$#      - run: tools/release/check-guards.sh approvals\n        continue-on-error: true#' ;;
    esac <"$root/.github/workflows/release.yml" >"$fake_root/.github/workflows/release.yml"
    ! cmp -s "$root/.github/workflows/release.yml" "$fake_root/.github/workflows/release.yml" || die "偽の release.yml（$variant）が元と同じ"
    if CHECK_GUARDS_ROOT="$fake_root" "$0" workflows >/dev/null 2>&1; then die "走らない承認者の確かめ（$variant）を通した"; fi
  done
  cp "$root/.github/workflows/release.yml" "$fake_root/.github/workflows/release.yml"
  # ci の code 判定が共有の台本を呼ばなくなれば、workflows の検査が落ちる（final review R19）
  cp "$root/.github/workflows/release.yml" "$root/.github/workflows/nightly.yml" "$fake_root/.github/workflows/"
  sed 's#tools/release/changes.sh code#tools/release/not-changes.sh code#' "$root/.github/workflows/ci.yml" >"$fake_root/.github/workflows/ci.yml"
  if CHECK_GUARDS_ROOT="$fake_root" "$0" workflows >/dev/null 2>&1; then die 'YAML の code 判定が台本を呼ばないのを通した'; fi
  echo "== Scenario: PR の配布物は、PR の head の commit から組まれ、同じ commit の期待値で検査される"
  local ci_saved="$fake_root/.github/workflows/ci.yml"
  if sed '/ref: \${{ github.event.pull_request.head.sha || github.sha }}/d' "$ci_saved" >"$work/ci-missing-linux.yml" && cp "$work/ci-missing-linux.yml" "$ci_saved" && CHECK_GUARDS_ROOT="$fake_root" "$0" workflows >/dev/null 2>&1; then die 'Linux package の ref 欠落を通した'; fi
  cp "$root/.github/workflows/ci.yml" "$ci_saved"
  if sed 's/^\([[:space:]]*\)\(ref: \${{ github.event.pull_request.head.sha || github.sha }}\)/\1# \2/' "$ci_saved" >"$work/ci-commented-ref.yml" && cp "$work/ci-commented-ref.yml" "$ci_saved" && CHECK_GUARDS_ROOT="$fake_root" "$0" workflows >/dev/null 2>&1; then die '注釈にした ref を通した'; fi
  cp "$root/.github/workflows/ci.yml" "$ci_saved"
  if sed '/package-windows:/,/^  [A-Za-z0-9_-]*:/{/ref: \${{ github.event.pull_request.head.sha || github.sha }}/d}' "$ci_saved" >"$work/ci-missing-windows.yml" && cp "$work/ci-missing-windows.yml" "$ci_saved" && CHECK_GUARDS_ROOT="$fake_root" "$0" workflows >/dev/null 2>&1; then die 'Windows package の ref 欠落を通した'; fi
  cp "$root/.github/workflows/ci.yml" "$ci_saved"
  awk '
    /^  package-artifacts:/ { in_artifacts=1 }
    in_artifacts && /^  [A-Za-z0-9_-]+:/ && $0 !~ /^  package-artifacts:/ { in_artifacts=0 }
    in_artifacts && /^      - uses: actions\/checkout@/ { checkout++ }
    in_artifacts && checkout == 1 && /with: \{ persist-credentials: false \}/ {
      print "        with:"
      print "          ref: ${{ github.event.pull_request.head.sha || github.sha }}"
      print "          persist-credentials: false"
      next
    }
    { print }
  ' "$ci_saved" >"$work/ci-artifacts-ref.yml" && cp "$work/ci-artifacts-ref.yml" "$ci_saved"
  if CHECK_GUARDS_ROOT="$fake_root" "$0" workflows >/dev/null 2>&1; then die 'package-artifacts の作業 checkout の ref を通した'; fi
  cp "$root/.github/workflows/ci.yml" "$ci_saved"
  if sed '/path: built/d' "$ci_saved" >"$work/ci-missing-built.yml" && cp "$work/ci-missing-built.yml" "$ci_saved" && CHECK_GUARDS_ROOT="$fake_root" "$0" workflows >/dev/null 2>&1; then die 'built checkout の欠落を通した'; fi
  cp "$root/.github/workflows/ci.yml" "$ci_saved"
  if awk '
    /^  package-artifacts:/ { job=1 }
    job && /^  [A-Za-z0-9_-]+:/ && $0 !~ /^  package-artifacts:/ { job=0 }
    job && /^      - uses: actions\/checkout@/ { checkout++ }
    job && checkout == 2 && /ref:/ { next }
    { print }
  ' "$ci_saved" >"$work/ci-missing-built-ref.yml" && cp "$work/ci-missing-built-ref.yml" "$ci_saved" && CHECK_GUARDS_ROOT="$fake_root" "$0" workflows >/dev/null 2>&1; then die 'built checkout の ref 欠落を通した'; fi
  cp "$root/.github/workflows/ci.yml" "$ci_saved"
  if sed 's/--expect-license built\/LICENSE//' "$ci_saved" >"$work/ci-missing-license.yml" && cp "$work/ci-missing-license.yml" "$ci_saved" && CHECK_GUARDS_ROOT="$fake_root" "$0" workflows >/dev/null 2>&1; then die 'built LICENSE 欠落を通した'; fi
  cp "$root/.github/workflows/ci.yml" "$ci_saved"
  if sed 's#version.sh source built/Cargo.toml#version.sh source#' "$ci_saved" >"$work/ci-missing-version.yml" && cp "$work/ci-missing-version.yml" "$ci_saved" && CHECK_GUARDS_ROOT="$fake_root" "$0" workflows >/dev/null 2>&1; then die 'built 版番号欠落を通した'; fi
  cp "$root/.github/workflows/ci.yml" "$ci_saved"
  workflow
  echo "OK: rules・Environment・workflow の守りを検査した"
}

[[ "${1:-}" == --self-test ]] && { self_test; exit 0; }
usage() { echo "使い方: $0 <rules|environments|workflows|approvals|agent-credential> [--repo <owner/name>] [--reviewer <login>]（--repo は前後どちらでもよい）" >&2; exit 2; }
kind=""
reviewer_opt=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --repo) [[ $# -ge 2 && -n "$2" ]] || usage; repo="$2"; shift 2 ;;
    --reviewer) [[ $# -ge 2 && -n "$2" ]] || usage; reviewer_opt="$2"; shift 2 ;;
    rules|environments|workflows|approvals|agent-credential) [[ -z "$kind" ]] || usage; kind="$1"; shift ;;
    *) usage ;;
  esac
done
case "$kind" in rules) rules;; environments) environments;; workflows) workflow;; approvals) approvals;; agent-credential) agent_credential;; *) usage;; esac
