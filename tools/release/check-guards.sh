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
    # 空行と注釈の行は手順を切らない（YAML では同じ手順の中。再レビュー）
    if not line.strip() or line.lstrip().startswith("#"): continue
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

# 番号付けの workflow の手順を許す形に限る（spec: 公開する手順と印 v* を作る手順を持たない。code-verify R2）。
# 禁止の並びでなく、使ってよい action と、run の各コマンドの先頭の語を許す一覧で見る。
# gh・git などは run に直に書かない（印は number.sh tag、置くのは number.sh place だけが作る）
number_steps() {
  python3 - "$1" <<'PY'
import re, sys
try:
    import yaml
except ImportError:
    print("number.yml を読むのに python3 の yaml が要る"); sys.exit(1)
ALLOWED_USES = {"actions/checkout", "actions/upload-artifact", "actions/download-artifact",
                "actions/setup-java", "actions/setup-node", "dtolnay/rust-toolchain"}
ALLOWED_CMDS = {"tools/release/version.sh", "tools/release/number.sh", "tools/release/build.sh",
                "echo", "sed", "tee", "mkdir", "find", "cp", "[["}
ALLOWED_STEP_KEYS = {"id", "name", "if", "uses", "with", "run", "shell", "env"}
# 前が / でも拾う（/usr/bin/gh）。前が . や英数字なら別の語（number.sh の sh）
FORBIDDEN = re.compile(r"(?<![\w.-])(gh|git|curl|wget|eval|exec|source|xargs|bash|sh|python3?|node|npx|jq)(?![\w./-])")
ASSIGN = re.compile(r"""^[A-Za-z_][A-Za-z0-9_]*=("[^"]*"?|'[^']*'?|\S*)\s*""")
# & の 1 つ（裏で走らせる）・改行・find の -exec の後も、次のコマンドの先頭とみなす（code.md R11）
SPLIT = re.compile(r"\|\||&&|(?<![<>])&(?!>)|;|\||`|\n|(?<!\S)-(?:exec|execdir|ok|okdir)(?!\S)|\b(?:then|if|else|elif|fi|do|done|while|for)\b")
bad = []
def check_cmd(text, where):
    subs = []
    def take(m):
        subs.append(m.group(1)); return "X"
    prev = None
    while prev != text:
        prev, text = text, re.sub(r"\$\(([^()]*)\)", take, text)
    for inner in subs:
        check_cmd(inner, where)
    if FORBIDDEN.search(text):
        bad.append(f"{where}: 直に呼んではいけないコマンド: {text.strip()}")
    for seg in SPLIT.split(text):
        seg = seg.strip().lstrip("!").strip()
        while (m := ASSIGN.match(seg)):
            seg = seg[m.end():]
        if not seg or re.fullmatch(r"[\s\"'\\]*", seg) or seg.startswith("]]") or seg.startswith("#"):
            continue
        word = seg.split()[0]
        if word not in ALLOWED_CMDS:
            bad.append(f"{where}: 許していないコマンド {word!r}: {text.strip()}")
# 字面の行でなく YAML として読む（flow 形式の step・複数行の plain scalar も同じ文字列になる。code.md R11）
try:
    with open(sys.argv[1], encoding="utf-8") as f:
        doc = yaml.safe_load(f)
except yaml.YAMLError as e:
    print(f"number.yml を YAML として読めない: {e}"); sys.exit(1)
jobs = doc.get("jobs") if isinstance(doc, dict) else None
if not isinstance(jobs, dict):
    print("number.yml に jobs が無い"); sys.exit(1)
# shell と env は step のほか job・workflow の単位にも置ける（defaults.run.shell・env）。どの単位でも同じ規則で見る（code.md R12）
def check_shell(shell, where):
    if shell != "bash":
        bad.append(f"{where}: shell が bash でない: {shell}")
def check_env(env, where):
    if env is None:
        return
    if not isinstance(env, dict):
        bad.append(f"{where}: env が対応表でない"); return
    for key in env:
        if key in ("BASH_ENV", "ENV"):
            bad.append(f"{where}: 殻が先に読むファイルを指す env {key}")
def check_defaults(defaults, where):
    if defaults is None:
        return None
    if not isinstance(defaults, dict) or not isinstance(defaults.get("run", {}), dict):
        bad.append(f"{where}: defaults が対応表でない"); return None
    run = defaults.get("run") or {}
    if "shell" in run:
        check_shell(run["shell"], f"{where} の defaults.run")
        return run["shell"]
    return None
workflow_shell = check_defaults(doc.get("defaults"), "number.yml の workflow")
check_env(doc.get("env"), "number.yml の workflow")
for jname, job in jobs.items():
    if not isinstance(job, dict):
        bad.append(f"{jname}: job が対応表でない"); continue
    job_shell = check_defaults(job.get("defaults"), f"number.yml {jname} の job") or workflow_shell
    check_env(job.get("env"), f"number.yml {jname} の job")
    if "uses" in job:
        bad.append(f"{jname}: 別の workflow を呼ぶ job: {job['uses']}")
    steps = job.get("steps") or []
    if not isinstance(steps, list):
        bad.append(f"{jname}: steps が並びでない"); continue
    for n, step in enumerate(steps, 1):
        where = f"number.yml {jname} の {n} 番目の step"
        if not isinstance(step, dict):
            bad.append(f"{where}: step が対応表でない"); continue
        for key in step:
            if key not in ALLOWED_STEP_KEYS:
                bad.append(f"{where}: 許していない鍵 {key!r}")
        check_env(step.get("env"), where)
        if "uses" in step:
            name = str(step["uses"]).split("@")[0]
            if name not in ALLOWED_USES:
                bad.append(f"{where}: 許していない action: {step['uses']}")
        if "shell" in step:
            check_shell(step["shell"], where)
        if "run" in step:
            # shell を書かない run は bash -e（pipefail なし）で走り、パイプの左が落ちても緑になる。
            # どの run も、step・job・workflow のどこかで shell: bash を持つ（code.md R13）
            if step.get("shell", job_shell) is None:
                bad.append(f"{where}: run の shell が bash と書かれていない（書かないと pipefail なしで走る）")
            for line in str(step["run"]).splitlines():
                if line.strip() and not line.strip().startswith("#"):
                    check_cmd(line, where)
for b in bad:
    print(b)
sys.exit(1 if bad else 0)
PY
}

# main への push の CI の差分の基準（ST61 D8）。changes の job が、前に成功した main の push の run の commit を
# changes.sh base で確かめて BASE に使うことを、手順の単位で見る（code-verify R1）
ci_changes_base() {
  local ci="$1" block prev
  block="$(job_block "$ci" changes)"
  grep -qxF '    permissions: { contents: read, actions: read }' <<<"$block" || { die 'ci の changes の job が run を読む権限（actions: read）を持たない'; return 1; }
  prev="$(awk '/^      - id: prev$/{f=1; print; next} f && /^      - /{exit} f{print}' <<<"$block")"
  [[ -n "$prev" ]] || { die 'ci の changes の job に前に成功した run を探す手順（id: prev）が無い'; return 1; }
  grep -qxF "        if: github.event_name == 'push'" <<<"$prev" || { die 'ci の前に成功した run を探す手順が main への push で走らない'; return 1; }
  if grep -qE '^        (continue-on-error|if):' <<<"$(grep -vxF "        if: github.event_name == 'push'" <<<"$prev")"; then die 'ci の前に成功した run を探す手順に別の条件がある'; return 1; fi
  grep -qE '^          PREV_SUCCESS_SHA="\$\(gh api "repos/\$\{REPO\}/actions/workflows/ci\.yml/runs\?branch=main&event=push&status=success&' <<<"$prev" \
    || { die 'ci が main の push の成功した run だけを探さない'; return 1; }
  grep -qxF '          export PREV_SUCCESS_SHA' <<<"$prev" || { die 'ci が前に成功した run の commit を changes.sh base に渡さない'; return 1; }
  grep -qxF '          echo "base=$(tools/release/changes.sh base)" >> "$GITHUB_OUTPUT"' <<<"$prev" || { die 'ci が差分の基準を changes.sh base で決めない'; return 1; }
  grep -qxF '          BASE: ${{ github.event.pull_request.base.sha || steps.prev.outputs.base }}' <<<"$block" || { die 'ci の差分の基準が、前に成功した run の commit でない'; return 1; }
}

# 番号付けの workflow（ST61 D10・D12）。条件・権限・順番待ち・取り出しを字面でなく job の単位で見る
number_workflow() {
  local file="$1" job block
  [[ -f "$file" ]] || { die 'number.yml が無い'; return 1; }
  grep -qE '^  workflow_run:' "$file" || { die 'number.yml が workflow_run で起きない'; return 1; }
  grep -qE '^    workflows: \[ci\]' "$file" || { die 'number.yml が CI の workflow の終わりで起きない'; return 1; }
  grep -qE '^    types: \[completed\]' "$file" || { die 'number.yml が終わった run だけで起きない'; return 1; }
  grep -qE '^    branches: \[main\]' "$file" || { die 'number.yml が main の run だけで起きない'; return 1; }
  block="$(job_block "$file" number)"
  # 3 つの条件を && で結んだ形そのものを見る（含むかだけでは || で結んだ形も通る。code.md R21）
  grep -qxF "    if: github.event.workflow_run.event == 'push' && github.event.workflow_run.conclusion == 'success' && github.event.workflow_run.head_repository.full_name == github.repository" <<<"$block" \
    || { die '番号付けが「この repo の枝の、push の、成功した run」だけで起きない（3 つを && で結んだ if でない）'; return 1; }
  # 組は判定の job にだけ置き、workflow の単位には置かない。取り消す設定は持たない
  if grep -qE '^concurrency:' "$file"; then die '同時に走らせない組が workflow の単位にある'; return 1; fi
  { grep -qE '^    concurrency:' <<<"$block" && grep -qE '^      group: number$' <<<"$block"; } || { die '判定の job に同時に走らせない組が無い'; return 1; }
  grep -qE '^      cancel-in-progress: false$' <<<"$block" || { die '判定の job が走っているものを取り消さないと書いていない'; return 1; }
  if grep -q 'cancel-in-progress: true' "$file"; then die '番号付けが走っているものを取り消す'; return 1; fi
  # 書く権限は印を作る job と置く job だけ。workflow の既定は読むだけ
  awk '/^permissions:$/{getline; print; exit}' "$file" | grep -qE '^  contents: read$' || { die 'number.yml の既定の権限が読むだけでない'; return 1; }
  [[ "$(grep -c 'contents: write' "$file")" == 2 ]] || { die '書く権限が 2 つの job だけにない'; return 1; }
  grep -q 'contents: write' <<<"$block" || { die '印を作る job に書く権限が無い'; return 1; }
  job_block "$file" place | grep -q 'contents: write' || { die '置く job に書く権限が無い'; return 1; }
  if grep -qE 'actions/cache|^[[:space:]]+cache:|restore-keys|cache-dependency-path' "$file"; then die '番号付けが CI のキャッシュを使う'; return 1; fi
  if grep -qE 'gh release (edit|delete)|--draft=false|--clobber|publish-nightly|refs/tags/v|sign-apk' "$file"; then die '番号付けが公開・上書き・印 v* の手順を持つ'; return 1; fi
  local not_allowed
  not_allowed="$(number_steps "$file")" || { die "番号付けの手順が許す形でない（公開・印 v* を作れる）: $not_allowed"; return 1; }
  # 組み立ては印を作る job の後。終わった run の commit を履歴を全部持つ形で取り出す
  for job in build-linux build-windows; do
    block="$(job_block "$file" "$job")"
    grep -qE '^    needs: number$' <<<"$block" || { die "$job が印を作る job の後にない"; return 1; }
    grep -q 'ref: \${{ github.event.workflow_run.head_sha }}' <<<"$block" || { die "$job が終わった run の commit を取り出さない"; return 1; }
    grep -q 'fetch-depth: 0' <<<"$block" || { die "$job が履歴を全部持つ形で取り出さない"; return 1; }
  done
  block="$(job_block "$file" number)"
  { grep -q 'ref: \${{ github.event.workflow_run.head_sha }}' <<<"$block" && grep -q 'fetch-depth: 0' <<<"$block"; } || { die '判定の job が終わった run の commit を全部の履歴で取り出さない'; return 1; }
  # 置く job は gh と GH_TOKEN だけで書く。書く権限の token を作業木（.git/config）に残さない（code.md R18）
  job_block "$file" place | grep -qxF '          persist-credentials: false' || { die '置く job が書く権限の token を作業木に残す（persist-credentials: false でない）'; return 1; }
  if job_block "$file" place | grep -q 'persist-credentials: true'; then die '置く job が書く権限の token を作業木に残す'; return 1; fi
  grep -q 'ASHIATO_APK_ALSO_UNSIGNED' <<<"$(job_block "$file" build-linux)" || { die '番号付けの APK に署名していない APK が足されない'; return 1; }
}

workflow() {
  local ci="$root/.github/workflows/ci.yml" release="$root/.github/workflows/release.yml" nightly="$root/.github/workflows/nightly.yml"
  [[ -f "$ci" && -f "$release" && -f "$nightly" ]] || die 'workflow が不足している'
  number_workflow "$root/.github/workflows/number.yml" || return 1
  grep -q 'tools/release/changes.sh code' "$ci" || die 'ci が共有の code 判定を使っていない'
  # 変わったものの一覧をパイプで判定へ渡さない。大きな一覧では grep -q が先に終わり、左の printf が SIGPIPE で落ち、
  # pipefail が「触れていない」と読んでコードの job を飛ばす（code.md R28）。<<< で渡す
  if grep -qE '"\$files"[[:space:]]*\|' "$ci"; then die 'ci が変わったものの一覧をパイプで判定へ渡す（大きな一覧で SIGPIPE が偽になる）'; return 1; fi
  grep -qF 'tools/release/changes.sh code <<<"$files"' "$ci" || { die 'ci が変わったものの一覧を <<< で code 判定へ渡さない'; return 1; }
  grep -qF 'tools/release/changes.sh deps <<<"$files"' "$ci" || { die 'ci が変わったものの一覧を <<< で deps 判定へ渡さない'; return 1; }
  ci_changes_base "$ci" || return 1
  job_block "$ci" package-linux | grep -q 'if: needs.changes.outputs.code == '\''true'\''' || die 'Linux package が code=true でない'
  job_block "$ci" package-windows | grep -q 'if: needs.changes.outputs.code == '\''true'\''' || die 'Windows package が code=true でない'
  job_block "$ci" package-artifacts | grep -q 'if: needs.changes.outputs.code == '\''true'\''' || die 'artifact package が code=true でない'
  grep -q 'upload-artifact' "$ci" || die 'ci に upload-artifact が無い'
  grep -q 'cron: "0 15 \* \* \*"' "$nightly" || die 'nightly の cron が違う'
  job_block "$nightly" publish | grep -q "ASHIATO_NIGHTLY_PUBLISH == 'on'" || die 'nightly publish の条件が無い'
  job_block "$release" plan | grep -q 'version.sh plan' || die 'release の plan が無い'
  job_block "$nightly" plan | grep -q 'version.sh nightly-plan' || die 'nightly の nightly-plan が無い'
  # 判定の step（id: plan）は shell: bash で走らせる。書かないと bash -e（pipefail なし）で走り、
  # version.sh が出力の後に落ちても tee の 0 で緑になる（code.md R13・R25・R27・R32）
  local plan_name plan_file plan_step
  for plan_name in nightly release; do
    plan_file="$root/.github/workflows/$plan_name.yml"
    plan_step="$(job_block "$plan_file" plan | awk '/^      - id: plan$/{f=1; print; next} f && /^      - /{exit} f{print}')"
    [[ -n "$plan_step" ]] || { die "$plan_name の判定 job に id: plan の step が無い"; return 1; }
    grep -qxF '        shell: bash' <<<"$plan_step" || { die "$plan_name の判定の step（id: plan）の shell が bash と書かれていない（書かないと pipefail なしで走る）"; return 1; }
  done
  job_block "$release" package-linux | grep -qE 'ASHIATO_APK_UNSIGNED(:[[:space:]]*|=)"?1' || die 'release の APK が unsigned でない'
  job_block "$nightly" package-linux | grep -q 'tools/release/build.sh apk' || die 'nightly の組み立て job に APK が無い'
  job_block "$nightly" publish | grep -q 'tools/release/publish-nightly.sh' || die 'nightly の公開 job が公開手順を使っていない'
  [[ "$(grep -c 'tools/release/publish-nightly.sh' "$nightly")" == "$(job_block "$nightly" publish | grep -c 'tools/release/publish-nightly.sh')" ]] || die 'nightly の公開手順が publish 以外から呼ばれる'
  job_block "$nightly" publish | grep -qE '^[[:space:]]+name: nightly[[:space:]]*$' || die 'nightly の公開 job に Environment が無い'
  job_block "$nightly" publish | grep -q 'ASHIATO_EXPECT_DEV_CERT_SHA256: \${{ vars.ASHIATO_CERT_SHA256 }}' || die 'nightly の開発版指紋が Environment 変数から渡らない'
  if grep -qE 'sign-apk.sh' "$nightly"; then die 'nightly の workflow に署名の論理がある'; return 1; fi
  # D11: 組み立てと公開の job は、判定が返した最新の番号の commit を取り出し、印 nightly もその commit へ動かす
  local nightly_job
  for nightly_job in package-linux package-windows publish; do
    job_block "$nightly" "$nightly_job" | grep -q 'ref: "\${{ needs.plan.outputs.commit }}"' || die "nightly の $nightly_job が判定の commit を取り出さない"
  done
  job_block "$nightly" plan | grep -q 'commit: \${{ steps.plan.outputs.commit }}' || die 'nightly の判定 job が commit を outputs に出さない'
  job_block "$nightly" plan | grep -qF "grep -E '^(build|publish|version|commit)='" || die 'nightly の判定 job が commit を出力へ写さない'
  job_block "$nightly" publish | grep -q 'publish-nightly.sh .*needs.plan.outputs.commit' || die 'nightly の公開が印 nightly を判定の commit へ動かさない'
  # 番号を持たない組み立ては 0.0.x だけを名乗る（D6）。更新の試験の版と、配る形の job に渡す版番号を見る
  local spec_line spec_versions spec_v
  spec_line="$(job_block "$ci" package-windows | grep -E 'for spec in ' || true)"
  # 項目の名前に依らず、版の形の値（数字.数字.数字…）を全部拾う。0.0.0 は番号を持たない版でもない（code.md R22）
  spec_versions="$(grep -oE '(^|[^0-9A-Za-z.])[0-9]+\.[0-9]+\.[0-9]+[0-9A-Za-z.+-]*' <<<"$spec_line" | sed -E 's/^[^0-9]//')"
  [[ -n "$spec_versions" ]] || die '更新の試験の組み立ての版が読めない'
  for spec_v in $spec_versions; do
    [[ "$spec_v" == 0.0.[1-9]* ]] || die "CI の更新の試験の組み立てが 0.0.<N≥1> でない版を名乗る: $spec_v"
  done
  local given_v
  while IFS= read -r given_v; do
    [[ "$given_v" == 0.0.[1-9]* || "$given_v" == '$v' ]] || die "配る形の組み立ての job が番号の形の版番号を渡す: $given_v"
  done < <(grep -oE 'ASHIATO_BUILD_VERSION(:[[:space:]]*|=)"?[^"[:space:],}]*' "$ci" | sed -E 's/^ASHIATO_BUILD_VERSION(:[[:space:]]*|=)"?//')
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
  echo "== Scenario: 開発版の作り直しの workflow は、携帯の APK を組み、公開の job の中でだけ開発版の鍵で署名する"
  grep -q 'tools/release/build.sh apk' "$root/.github/workflows/nightly.yml" || die 'nightly に APK の手順が無い'
  job_block "$root/.github/workflows/nightly.yml" publish | grep -q 'tools/release/publish-nightly.sh' || die '公開手順が無い'
  job_block "$root/.github/workflows/nightly.yml" publish | grep -q 'ASHIATO_EXPECT_DEV_CERT_SHA256: \${{ vars.ASHIATO_CERT_SHA256 }}' || die '開発版の指紋が無い'
  echo "== Scenario: 開発版の作り直しは、日本時間の 0:00 に起きる"
  grep -q 'cron: "0 15 \* \* \*"' "$root/.github/workflows/nightly.yml" || die 'cron が違う'
  echo "== Scenario: 公開の設定が無ければ、印 nightly を動かす手順は走らない"
  grep -q "ASHIATO_NIGHTLY_PUBLISH == 'on'" "$root/.github/workflows/nightly.yml" || die '公開条件が無い'
  export GH_ENV_RELEASE_JSON="$work/release-env.json"
  local fake_root="$work/tree"
  mkdir -p "$fake_root/.github/workflows"
  cp "$root/.github/workflows/ci.yml" "$root/.github/workflows/release.yml" "$root/.github/workflows/nightly.yml" "$root/.github/workflows/number.yml" "$fake_root/.github/workflows/"
  sed '/tools\/release\/build.sh apk/d' "$root/.github/workflows/nightly.yml" >"$fake_root/.github/workflows/nightly.yml"
  if CHECK_GUARDS_ROOT="$fake_root" "$0" workflows >/dev/null 2>&1; then die 'APK の無い偽 nightly を通した'; fi
  sed '/tools\/release\/build.sh apk/a\      - run: tools/release/sign-apk.sh' "$root/.github/workflows/nightly.yml" >"$fake_root/.github/workflows/nightly.yml"
  if CHECK_GUARDS_ROOT="$fake_root" "$0" workflows >/dev/null 2>&1; then die '組み立て job の署名を通した'; fi
  sed '/ASHIATO_EXPECT_DEV_CERT_SHA256/d' "$root/.github/workflows/nightly.yml" >"$fake_root/.github/workflows/nightly.yml"
  if CHECK_GUARDS_ROOT="$fake_root" "$0" workflows >/dev/null 2>&1; then die '開発版指紋の無い偽 nightly を通した'; fi
  # Environment の名前が違えば、environments の検査（nightly の branch 規則）の外で走る（code-verify R28）
  sed 's/^\([[:space:]][[:space:]]*\)name: nightly$/\1name: nightly-open/' "$root/.github/workflows/nightly.yml" >"$fake_root/.github/workflows/nightly.yml"
  if CHECK_GUARDS_ROOT="$fake_root" "$0" workflows >/dev/null 2>&1; then die 'Environment の名前が違う偽 nightly を通した'; fi
  echo "== Scenario: 開発版の作り直しの workflow は、判定が返した commit を組む"
  sed 's/ref: "\${{ needs.plan.outputs.commit }}", //' "$root/.github/workflows/nightly.yml" >"$fake_root/.github/workflows/nightly.yml"
  if CHECK_GUARDS_ROOT="$fake_root" "$0" workflows >/dev/null 2>&1; then die 'ref の無い偽 nightly を通した'; fi
  sed 's/ "\${{ needs.plan.outputs.commit }}"$//' "$root/.github/workflows/nightly.yml" >"$fake_root/.github/workflows/nightly.yml"
  if CHECK_GUARDS_ROOT="$fake_root" "$0" workflows >/dev/null 2>&1; then die '印 nightly の先に commit を渡さない偽 nightly を通した'; fi
  sed '/^      commit: /d' "$root/.github/workflows/nightly.yml" >"$fake_root/.github/workflows/nightly.yml"
  if CHECK_GUARDS_ROOT="$fake_root" "$0" workflows >/dev/null 2>&1; then die 'commit を outputs に出さない偽 nightly を通した'; fi
  sed 's/|version|commit)=/|version)=/' "$root/.github/workflows/nightly.yml" >"$fake_root/.github/workflows/nightly.yml"
  if CHECK_GUARDS_ROOT="$fake_root" "$0" workflows >/dev/null 2>&1; then die 'commit を出力へ写さない偽 nightly を通した'; fi
  # 判定の step の shell: bash を外した偽 nightly（code.md R27）
  sed '/^      - id: plan$/,/^        run: /{/^        shell: bash$/d}' "$root/.github/workflows/nightly.yml" >"$fake_root/.github/workflows/nightly.yml"
  ! cmp -s "$root/.github/workflows/nightly.yml" "$fake_root/.github/workflows/nightly.yml" || die 'shell: bash を外した偽 nightly が元と同じ'
  if CHECK_GUARDS_ROOT="$fake_root" "$0" workflows >/dev/null 2>&1; then die '判定の step に shell: bash の無い偽 nightly を通した'; fi
  cp "$root/.github/workflows/ci.yml" "$root/.github/workflows/nightly.yml" "$fake_root/.github/workflows/"
  # 判定の step の shell: bash を外した偽 release（code.md R32）
  sed '/^      - id: plan$/,/^        run: /{/^        shell: bash$/d}' "$root/.github/workflows/release.yml" >"$fake_root/.github/workflows/release.yml"
  ! cmp -s "$root/.github/workflows/release.yml" "$fake_root/.github/workflows/release.yml" || die 'shell: bash を外した偽 release が元と同じ'
  if CHECK_GUARDS_ROOT="$fake_root" "$0" workflows >/dev/null 2>&1; then die '判定の step に shell: bash の無い偽 release を通した'; fi
  if sed 's/contents: write/contents: read/' "$root/.github/workflows/release.yml" >"$fake_root/.github/workflows/release.yml" && CHECK_GUARDS_ROOT="$fake_root" "$0" workflows >/dev/null 2>&1; then die 'release publish の権限欠落を通した'; fi
  cp "$root/.github/workflows/release.yml" "$fake_root/.github/workflows/release.yml"
  echo "== Scenario: CI の更新の試験の組み立ては、0.0.x だけを名乗る"
  CHECK_GUARDS_ROOT="$root" "$0" workflows >/dev/null 2>&1 || die '本物の ci.yml が 0.0.x の検査を通らない'
  if sed 's/0\.0\.2:old/0.4.0:old/' "$root/.github/workflows/ci.yml" >"$fake_root/.github/workflows/ci.yml" && ! cmp -s "$root/.github/workflows/ci.yml" "$fake_root/.github/workflows/ci.yml" && CHECK_GUARDS_ROOT="$fake_root" "$0" workflows >/dev/null 2>&1; then die '0.4.0 の更新の試験の組み立てを通した'; fi
  ! cmp -s "$root/.github/workflows/ci.yml" "$fake_root/.github/workflows/ci.yml" || die '偽の ci.yml（0.4.0）が元と同じ'
  # 別の名前の項目・0.0.0 も見逃さない（code.md R22）
  local spec_variant
  for spec_variant in '0.4.0:extra' '0.0.0:zero'; do
    sed "s/0\.0\.2:old /0.0.2:old $spec_variant /" "$root/.github/workflows/ci.yml" >"$fake_root/.github/workflows/ci.yml"
    ! cmp -s "$root/.github/workflows/ci.yml" "$fake_root/.github/workflows/ci.yml" || die "偽の ci.yml（$spec_variant）が元と同じ"
    if CHECK_GUARDS_ROOT="$fake_root" "$0" workflows >/dev/null 2>&1; then die "更新の試験の組み立ての版 $spec_variant を通した"; fi
  done
  cp "$root/.github/workflows/ci.yml" "$fake_root/.github/workflows/ci.yml"
  echo "== Scenario: 配る形の組み立ての job は、番号を版番号に渡さない"
  if sed '/^  package-linux:$/,/^    steps:$/ s/^    steps:$/    env: { ASHIATO_BUILD_VERSION: 0.2.0 }\n    steps:/' "$root/.github/workflows/ci.yml" >"$fake_root/.github/workflows/ci.yml" && CHECK_GUARDS_ROOT="$fake_root" "$0" workflows >/dev/null 2>&1; then die '番号の形の版番号を渡す偽の ci.yml を通した'; fi
  grep -q 'ASHIATO_BUILD_VERSION: 0.2.0' "$fake_root/.github/workflows/ci.yml" || die '偽の ci.yml（0.2.0）に版番号が入っていない'
  cp "$root/.github/workflows/ci.yml" "$fake_root/.github/workflows/ci.yml"
  # シェルの形（VAR=値 の前置きと $GITHUB_ENV への書き込み）でも番号を渡せない
  if sed 's|ASHIATO_BUILD_VERSION="\$v" tools/release/package.sh|ASHIATO_BUILD_VERSION=0.2.0 tools/release/package.sh|' "$root/.github/workflows/ci.yml" >"$fake_root/.github/workflows/ci.yml" && CHECK_GUARDS_ROOT="$fake_root" "$0" workflows >/dev/null 2>&1; then die '= の形で番号の版番号を渡す偽の ci.yml を通した'; fi
  grep -q 'ASHIATO_BUILD_VERSION=0.2.0' "$fake_root/.github/workflows/ci.yml" || die '偽の ci.yml（= の形）に版番号が入っていない'
  if sed '/^  package-linux:$/,/^    steps:$/ s/^    steps:$/    steps:\n      - run: echo ASHIATO_BUILD_VERSION=0.2.0 >> "$GITHUB_ENV"/' "$root/.github/workflows/ci.yml" >"$fake_root/.github/workflows/ci.yml" && CHECK_GUARDS_ROOT="$fake_root" "$0" workflows >/dev/null 2>&1; then die 'GITHUB_ENV へ番号の版番号を書く偽の ci.yml を通した'; fi
  grep -q 'ASHIATO_BUILD_VERSION=0.2.0 >> ' "$fake_root/.github/workflows/ci.yml" || die '偽の ci.yml（GITHUB_ENV）に版番号が入っていない'
  cp "$root/.github/workflows/ci.yml" "$fake_root/.github/workflows/ci.yml"
  echo "== Scenario: 公開の job は、署名の前に承認者を確かめる"
  # approvals の行が無いときは、理由（NG の文）を出して落ちる（pipefail で黙って落ちない。final review R7）
  local no_approvals_output
  if sed '/tools\/release\/check-guards.sh approvals/d' "$root/.github/workflows/release.yml" >"$work/release-no-approvals.yml" && cp "$work/release-no-approvals.yml" "$fake_root/.github/workflows/release.yml" && no_approvals_output="$(CHECK_GUARDS_ROOT="$fake_root" "$0" workflows 2>&1)"; then die '承認者の確かめ無しを通した'; fi
  grep -qF 'NG: release publish が署名前に承認者を確かめない' <<<"$no_approvals_output" || die "承認者の確かめ無しで NG の文を出さない: $no_approvals_output"
  if awk '/check-guards.sh approvals/{line=$0; next} /sign-apk.sh/{print; print line; next} {print}' "$root/.github/workflows/release.yml" >"$work/release-swapped.yml" && cp "$work/release-swapped.yml" "$fake_root/.github/workflows/release.yml" && CHECK_GUARDS_ROOT="$fake_root" "$0" workflows >/dev/null 2>&1; then die '署名前でない承認者の確かめを通した'; fi
  cp "$root/.github/workflows/release.yml" "$fake_root/.github/workflows/release.yml"
  # 字面が残っていても、走らない・失敗を飲む承認の手順は数えない（code_verify R15・/code-review R26）
  local variant
  for variant in comment or-true if-false continue-on-error blank-continue-on-error; do
    case "$variant" in
      comment) sed 's#^      - run: tools/release/check-guards.sh approvals$#      \# - run: tools/release/check-guards.sh approvals#' ;;
      or-true) sed 's#^\(      - run: tools/release/check-guards.sh approvals\)$#\1 || true#' ;;
      if-false) sed 's#^      - run: tools/release/check-guards.sh approvals$#      - if: false\n        run: tools/release/check-guards.sh approvals#' ;;
      continue-on-error) sed 's#^      - run: tools/release/check-guards.sh approvals$#      - run: tools/release/check-guards.sh approvals\n        continue-on-error: true#' ;;
      blank-continue-on-error) sed 's#^      - run: tools/release/check-guards.sh approvals$#      - run: tools/release/check-guards.sh approvals\n\n        continue-on-error: true#' ;;
    esac <"$root/.github/workflows/release.yml" >"$fake_root/.github/workflows/release.yml"
    ! cmp -s "$root/.github/workflows/release.yml" "$fake_root/.github/workflows/release.yml" || die "偽の release.yml（$variant）が元と同じ"
    if CHECK_GUARDS_ROOT="$fake_root" "$0" workflows >/dev/null 2>&1; then die "走らない承認者の確かめ（$variant）を通した"; fi
  done
  cp "$root/.github/workflows/release.yml" "$fake_root/.github/workflows/release.yml"
  # ci の code 判定が共有の台本を呼ばなくなれば、workflows の検査が落ちる（final review R19）
  cp "$root/.github/workflows/release.yml" "$root/.github/workflows/nightly.yml" "$fake_root/.github/workflows/"
  sed 's#tools/release/changes.sh code#tools/release/not-changes.sh code#' "$root/.github/workflows/ci.yml" >"$fake_root/.github/workflows/ci.yml"
  if CHECK_GUARDS_ROOT="$fake_root" "$0" workflows >/dev/null 2>&1; then die 'YAML の code 判定が台本を呼ばないのを通した'; fi
  # 変わったものの一覧をパイプで判定へ渡す偽の ci.yml（code.md R28）
  local pipe_variant
  for pipe_variant in code deps has; do
    case "$pipe_variant" in
      code) sed "s#tools/release/changes.sh code <<<\"\$files\"#printf '%s\\n' \"\$files\" | tools/release/changes.sh code#" ;;
      deps) sed "s#tools/release/changes.sh deps <<<\"\$files\"#printf '%s\\n' \"\$files\" | tools/release/changes.sh deps#" ;;
      has) sed "s#grep -qE \"\$1\" <<<\"\$files\"#printf '%s\\n' \"\$files\" | grep -qE \"\$1\"#" ;;
    esac <"$root/.github/workflows/ci.yml" >"$fake_root/.github/workflows/ci.yml"
    ! cmp -s "$root/.github/workflows/ci.yml" "$fake_root/.github/workflows/ci.yml" || die "パイプで渡す偽の ci.yml（$pipe_variant）が元と同じ"
    if CHECK_GUARDS_ROOT="$fake_root" "$0" workflows >/dev/null 2>&1; then die "変わったものの一覧をパイプで渡す偽の ci.yml（$pipe_variant）を通した"; fi
  done
  cp "$root/.github/workflows/ci.yml" "$fake_root/.github/workflows/ci.yml"
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
  echo "== Scenario: 取り消された run の後の文書だけの merge でも、コードの job が走る"
  # ci.yml の差分の基準の配線（ST61 D8）を 1 つずつ外した偽の ci.yml は、workflows の検査が落とす（code-verify R1）
  local ci_variant
  for ci_variant in before no-actions-read no-success if-false continue-on-error commented-export base-all; do
    case "$ci_variant" in
      before) sed 's/steps\.prev\.outputs\.base }}/github.event.before }}/' ;;
      no-actions-read) sed 's/^    permissions: { contents: read, actions: read }$/    permissions: { contents: read }/' ;;
      no-success) sed 's/&status=success//' ;;
      if-false) sed "s/^        if: github.event_name == 'push'\$/        if: false/" ;;
      continue-on-error) sed "s/^        if: github.event_name == 'push'\$/&\n        continue-on-error: true/" ;;
      commented-export) sed 's/^          export PREV_SUCCESS_SHA$/          # export PREV_SUCCESS_SHA/' ;;
      base-all) sed 's#echo "base=$(tools/release/changes.sh base)"#echo "base=all"#' ;;
    esac <"$root/.github/workflows/ci.yml" >"$ci_saved"
    ! cmp -s "$root/.github/workflows/ci.yml" "$ci_saved" || die "偽の ci.yml（$ci_variant）が元と同じ"
    if CHECK_GUARDS_ROOT="$fake_root" "$0" workflows >/dev/null 2>&1; then die "差分の基準の配線を外した偽の ci.yml（$ci_variant）を通した"; fi
  done
  cp "$root/.github/workflows/ci.yml" "$ci_saved"
  # 番号付けの workflow（ST61 D10・D12）。条件を 1 つ外した偽の number.yml は workflows の検査が落とす
  local nf="$fake_root/.github/workflows/number.yml" nreal="$root/.github/workflows/number.yml"
  number_mutant() { # <名前> <sed の式>…（元と同じなら偽の作り損ね）
    local name="$1"; shift
    sed "$@" "$nreal" >"$nf"
    ! cmp -s "$nreal" "$nf" || die "偽の number.yml（$name）が元と同じ"
    if CHECK_GUARDS_ROOT="$fake_root" "$0" workflows >/dev/null 2>&1; then die "偽の number.yml（$name）を通した"; fi
    cp "$nreal" "$nf"
  }
  CHECK_GUARDS_ROOT="$fake_root" "$0" workflows >/dev/null || die '本物の number.yml を落とした'
  echo "== Scenario: main の push の成功した run だけが番号付けを起こす"
  number_mutant no-push -e "s/github.event.workflow_run.event == 'push' && //"
  number_mutant no-success -e "s/github.event.workflow_run.conclusion == 'success' && //"
  number_mutant no-same-repo -e "s/ && github.event.workflow_run.head_repository.full_name == github.repository//"
  number_mutant no-main -e '/^    branches: \[main\]/d'
  number_mutant not-ci -e 's/^    workflows: \[ci\]/    workflows: [nightly]/'
  # 3 つの条件を || で結んだ if（code.md R21）
  number_mutant or-if -e "s/'push' \&\& github.event.workflow_run.conclusion/'push' || github.event.workflow_run.conclusion/"
  echo "== Scenario: 番号付けの workflow は、前の組み立てを使い回さず、書く権限をその job にだけ持つ"
  number_mutant write-in-build -e '/^  build-linux:/,/^  build-windows:/s/^    runs-on: ubuntu-24.04/    runs-on: ubuntu-24.04\n    permissions:\n      contents: write/'
  number_mutant no-write-in-place -e '/^  place:/,$s/contents: write/contents: read/'
  number_mutant cache-restore -e 's#^      - uses: actions/setup-node@\(.*\)v6$#      - uses: actions/cache@x\n      - uses: actions/setup-node@\1v6#'
  number_mutant gradle-cache -e 's/^          java-version: "21"/          java-version: "21"\n          cache: gradle/'
  number_mutant publish-step -e 's#^      - run: tools/release/number.sh place.*#&\n      - run: gh release edit --draft=false#'
  number_mutant build-before-number -e '/^  build-windows:/,/^  place:/s/^    needs: number$/    needs: []/'
  # 禁止の並びに当たらない公開・印 v* の手順も、許す形の外なので落とす（code-verify R2）
  number_mutant gh-release-create -e 's#^      - run: tools/release/number.sh place.*#&\n      - run: gh release create "v${{ needs.number.outputs.number }}" --target "${{ github.event.workflow_run.head_sha }}" number/all/*#'
  number_mutant git-tag-push -e 's#^      - run: tools/release/number.sh place.*#&\n      - run: git tag "v${{ needs.number.outputs.number }}" \&\& git push origin "v${{ needs.number.outputs.number }}"#'
  number_mutant git-in-block -e 's#^          if \[\[ -n "$number" \]\]; then tools/release/number.sh tag#          git tag v9 \&\& git push origin v9\n&#'
  number_mutant gh-in-subst -e 's#^          echo "number=$number" >> "$GITHUB_OUTPUT"#          echo "x=$(gh release create v1)" >> "$GITHUB_OUTPUT"\n&#'
  number_mutant find-exec-gh -e 's#-exec cp {} number/all/#-exec gh release upload v1 {} #'
  # 書き方を変えた公開の step（code.md R11。末尾の place の job に足す）: flow 形式・複数行の plain scalar・& の後・find -exec の絶対パス
  number_mutant flow-step -e '$a\      - { run: "gh release create v1" }'
  number_mutant plain-multiline -e '$a\      - run: echo a' -e '$a\          && gh release create v1'
  number_mutant background-abs -e '$a\      - run: echo a & /usr/bin/gh release create v1'
  number_mutant find-exec-abs -e '$a\      - run: find . -maxdepth 0 -exec /usr/bin/gh release create v1 \\;'
  number_mutant gh-release-action -e 's#^      - uses: actions/download-artifact@\(.*\)#      - uses: softprops/action-gh-release@v2\n&#'
  number_mutant shell-python -e '0,/^        shell: bash$/s//        shell: python {0}/'
  echo "== Scenario: 番号付けの判定が止まると、番号付けの run が落ちる"
  # shell を書かない run は bash -e（pipefail なし）で走り、version.sh number が落ちても tee の 0 で緑になる（code.md R13）
  number_mutant no-shell-decide -e '/^      - id: decide$/,/^      - id: complete$/{/^        shell: bash$/d}'
  number_mutant no-shell-complete -e '/^      - id: complete$/,/^  build-linux:$/{/^        shell: bash$/d}'
  number_mutant no-shell-place -e '/^  place:$/,${/^        shell: bash$/d}'
  echo "== Scenario: 置く job は、書く権限の token を作業木に残さない"
  number_mutant place-persist -e '/^  place:$/,${s/persist-credentials: false/persist-credentials: true/}'
  # shell と env を workflow・job の単位に置いた偽の number.yml（code.md R12）
  number_mutant defaults-workflow -e 's/^jobs:$/defaults: { run: { shell: "python {0}" } }\n&/'
  number_mutant defaults-job -e '/^  place:$/a\    defaults:\n      run:\n        shell: "gh release create v1 {0}"'
  number_mutant env-workflow -e 's/^jobs:$/env: { BASH_ENV: .\/evil }\n&/'
  number_mutant env-job -e '/^  place:$/a\    env:\n      ENV: ./evil'
  echo "== Scenario: 番号付けは、CI と別の、取り消さない 1 本ずつの流れにある"
  number_mutant no-concurrency -e '/^    concurrency:/,/^      cancel-in-progress: false/d'
  number_mutant cancel -e 's/cancel-in-progress: false/cancel-in-progress: true/'
  echo "== Scenario: 番号付けは、終わった run の commit を全部の履歴で取り出す"
  number_mutant shallow-number -e '/^  number:/,/^  build-linux:/s/fetch-depth: 0/fetch-depth: 1/'
  number_mutant shallow-build -e '/^  build-windows:/,/^  place:/s/fetch-depth: 0/fetch-depth: 1/'
  number_mutant main-head -e '/^  build-linux:/,/^  build-windows:/{/^          ref: /d}'
  echo "== Scenario: 番号付けを起こさない run は、待っている番号付けを取り消さない"
  # 組を workflow の単位に置いた偽の number.yml（job の組は外す）
  awk '/^    concurrency:/{skip=3} skip>0{skip--; next} /^permissions:$/{print "concurrency:\n  group: number\n  cancel-in-progress: false"} {print}' "$nreal" >"$nf"
  ! cmp -s "$nreal" "$nf" || die '組を workflow の単位に置いた偽の number.yml が元と同じ'
  if CHECK_GUARDS_ROOT="$fake_root" "$0" workflows >/dev/null 2>&1; then die '組を workflow の単位に置いた偽の number.yml を通した'; fi
  cp "$nreal" "$nf"
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
