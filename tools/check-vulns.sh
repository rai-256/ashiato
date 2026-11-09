#!/usr/bin/env bash
set -u

# 何を見るか: Cargo.lock、web/package-lock.json、携帯の全 gradle.lockfile。
# なぜ要るか: CH11 で開発用を含む全依存の OSV 勧告と知らせを CI で止めるため。
# 使い方:
# | 呼び方 | 内容 |
# |---|---|
# | check-vulns.sh | 全体の検査。勧告か表の行の問題があれば rc=1、検査できなければ rc=2 |
# | check-vulns.sh --base <commit> | PR の検査。base に無かった組だけを数え、表の足した・変えた行を出す。rc は全体の検査と同じ |
# | check-vulns.sh allow-ghsas | dependency-review に渡す許可の GHSA の一覧（表と osv.dev の別名から）。組めなければ rc=2 |
# | check-vulns.sh nightly <dir> | 毎晩の検査。問題があれば <dir> に材料を書いて rc=1、検査できなければ rc=2（材料は書かない） |
# | check-vulns.sh file-issues <dir> | <dir> の材料（nightly と check-pg-minor.sh --findings）を issue にする。記録できなければ rc=2 |
# | check-vulns.sh github-settings | GitHub の依存の設定を API で整えて読み返す。権限が無ければ rc=2、合わなければ rc=1 |
# | check-vulns.sh --self-test | 自己試験 |
# 例外: osv-scanner.toml の [[IgnoredVulns]] に ID、90 日以内の期限、届かない: 理由 または 上げる予定: 見込み。
# 網失敗の実測（HTTPS_PROXY=http://127.0.0.1:9）: rc=2、`error: 検査できなかった（osv.dev への問い合わせ: ...）`。

set -o pipefail
readonly VERSION="v2.7.0"
readonly SHA256="160c6253e2922e9e0a6a9771c5c4c40a4b3c6090b34a3cdd5e80c96cc2e4218b"
# 認証の要らない releases/download から取る（api.github.com は認証なしだと 1 IP あたり 60 回/時）。中身は SHA256 で固定する。
readonly ASSET_URL="https://github.com/google/osv-scanner/releases/download/${VERSION}/osv-scanner_linux_amd64"
readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly ROOT="${CHECK_VULNS_ROOT:-$(cd "$SCRIPT_DIR/.." && pwd)}"

scanner_path() {
  local cache_root="${XDG_CACHE_HOME:-$HOME/.cache}/ashiato2/osv-scanner-${VERSION}"
  local path="${OSV_SCANNER_BIN:-$cache_root/osv-scanner_linux_amd64}"
  if [[ -z "${OSV_SCANNER_BIN:-}" && ! -x "$path" ]]; then
    mkdir -p "$(dirname "$path")" || return 2
    curl --fail --location --retry 3 --retry-all-errors --max-time 300 --silent --show-error \
      --output "$path" "$ASSET_URL" || {
      echo "error: 検査できなかった（osv-scanner の取り寄せ）" >&2; return 2;
    }
    chmod 755 "$path" || return 2
  fi
  [[ -f "$path" ]] || { echo "error: 検査できなかった（osv-scanner の場所）" >&2; return 2; }
  local actual
  actual="$(sha256sum "$path" | awk '{print $1}')" || return 2
  if [[ "$actual" != "$SHA256" ]]; then
    echo "error: 検査できなかった（sha256）" >&2
    return 2
  fi
  printf '%s\n' "$path"
}

github_settings() {
  local response status settings_root="${GH_SETTINGS_ROOT:-$ROOT}" rc
  echo "== Scenario: 設定を API で読み返す"

  response="$(gh api --include -X PUT repos/rai-256/ashiato/vulnerability-alerts 2>&1)"
  rc=$?
  status="$(sed -nE 's/^HTTP[^ ]* ([0-9]{3}).*/\1/p' <<<"$response" | tail -n 1)"
  if [[ "$status" == 401 || "$status" == 403 ]]; then
    echo "error: 検査できなかった（管理者の権限が要る）" >&2
    return 2
  fi
  if [[ $rc -ne 0 || "$status" != 204 ]]; then
    echo "error: 検査できなかった（vulnerability-alerts: HTTP ${status:-unknown}, gh rc=$rc）" >&2
    return 1
  fi
  echo "vulnerability-alerts: 204"

  response="$(gh api --include repos/rai-256/ashiato/dependency-graph/sbom 2>&1)"
  rc=$?
  status="$(sed -nE 's/^HTTP[^ ]* ([0-9]{3}).*/\1/p' <<<"$response" | tail -n 1)"
  if [[ "$status" != 200 || $rc -ne 0 ]]; then
    if [[ "$status" == 401 || "$status" == 403 ]]; then
      echo "error: 検査できなかった（管理者の権限が要る）" >&2
      return 2
    fi
    response="$(gh api --include -X PATCH repos/rai-256/ashiato -f 'security_and_analysis[dependency_graph][status]=enabled' 2>&1)"
    rc=$?
    status="$(sed -nE 's/^HTTP[^ ]* ([0-9]{3}).*/\1/p' <<<"$response" | tail -n 1)"
    if [[ "$status" == 401 || "$status" == 403 ]]; then
      echo "error: 検査できなかった（管理者の権限が要る）" >&2
      return 2
    fi
    if [[ $rc -ne 0 || "$status" != 200 ]]; then
      echo "error: 検査できなかった（security_and_analysis: HTTP ${status:-unknown}, gh rc=$rc）" >&2
      return 1
    fi
    response="$(gh api --include repos/rai-256/ashiato/dependency-graph/sbom 2>&1)"
    rc=$?
    status="$(sed -nE 's/^HTTP[^ ]* ([0-9]{3}).*/\1/p' <<<"$response" | tail -n 1)"
  fi
  if [[ "$status" == 401 || "$status" == 403 ]]; then
    echo "error: 検査できなかった（管理者の権限が要る）" >&2
    return 2
  fi
  if [[ $rc -ne 0 || "$status" != 200 ]]; then
    echo "error: 検査できなかった（dependency-graph/sbom: HTTP ${status:-unknown}, gh rc=$rc）" >&2
    return 1
  fi
  echo "dependency-graph/sbom: 200"

  response="$(gh api --include repos/rai-256/ashiato/automated-security-fixes 2>&1)"
  rc=$?
  status="$(sed -nE 's/^HTTP[^ ]* ([0-9]{3}).*/\1/p' <<<"$response" | tail -n 1)"
  if [[ "$status" == 401 || "$status" == 403 ]]; then
    echo "error: 検査できなかった（管理者の権限が要る）" >&2
    return 2
  fi
  if [[ $rc -ne 0 ]]; then
    echo "error: 検査できなかった（automated-security-fixes: HTTP ${status:-unknown}, gh rc=$rc）" >&2
    return 1
  fi
  if ! grep -Eq '"enabled"[[:space:]]*:[[:space:]]*false' <<<"$response"; then
    echo "error: 検査できなかった（automated-security-fixes が有効）" >&2
    return 1
  fi
  echo "automated-security-fixes: enabled=false"

  if [[ -e "$settings_root/.github/dependabot.yml" ]]; then
    echo "error: 検査できなかった（dependabot.yml がある）" >&2
    return 1
  fi
  echo "dependabot.yml: 無い"
}

self_test() {
  local scanner tmp
  tmp="$(cd "$(mktemp -d)" && pwd)" || return 2
  trap "rm -rf -- '$tmp'" EXIT
  export XDG_CACHE_HOME="$tmp/cache"
  scanner="$(scanner_path)"
  local scanner_rc=$?
  if [[ $scanner_rc -ne 0 ]]; then return "$scanner_rc"; fi
  mkdir -p "$tmp/root/web" "$tmp/root/collector-android"
  git -C "$tmp/root" init -q
  cp "$ROOT/osv-scanner.toml" "$tmp/root/osv-scanner.toml"
  local common=(env CHECK_VULNS_ROOT="$tmp/root" OSV_SCANNER_BIN="$scanner" XDG_CACHE_HOME="$tmp/cache")
  safe_tree() {
    mkdir -p "$tmp/root/web" "$tmp/root/collector-android"
    printf 'version = 3\n[[package]]\nname = "ashiato-self-test-safe"\nversion = "1.0.0"\n' > "$tmp/root/Cargo.lock"
    printf '{"name":"x","version":"1.0.0","lockfileVersion":3,"packages":{"":{"name":"x","version":"1.0.0","dependencies":{"ms":"2.1.3"}},"node_modules/ms":{"version":"2.1.3","resolved":"https://registry.npmjs.org/ms/-/ms-2.1.3.tgz"}}}\n' > "$tmp/root/web/package-lock.json"
    printf 'com.example:safe:1.0.0=runtimeClasspath\n' > "$tmp/root/collector-android/gradle.lockfile"
    printf '# empty\n' > "$tmp/root/osv-scanner.toml"
    git -C "$tmp/root" add Cargo.lock web/package-lock.json collector-android/gradle.lockfile osv-scanner.toml
  }
  vulnerable_tree() {
    mkdir -p "$tmp/root/web" "$tmp/root/collector-android"
    printf 'version = 3\n[[package]]\nname = "ansi_term"\nversion = "0.12.1"\n' > "$tmp/root/Cargo.lock"
    printf '{"name":"x","version":"1.0.0","lockfileVersion":3,"packages":{"":{"name":"x","version":"1.0.0","dependencies":{"lodash":"4.17.20"}},"node_modules/lodash":{"version":"4.17.20","resolved":"https://registry.npmjs.org/lodash/-/lodash-4.17.20.tgz"}}}\n' > "$tmp/root/web/package-lock.json"
    printf 'com.fasterxml.jackson.core:jackson-databind:2.9.10.8=runtimeClasspath\n' > "$tmp/root/collector-android/gradle.lockfile"
    printf '# empty\n' > "$tmp/root/osv-scanner.toml"
    git -C "$tmp/root" add Cargo.lock web/package-lock.json collector-android/gradle.lockfile osv-scanner.toml
  }
  assert_self_test_dependencies_absent() {
    ! grep -Eq 'name = "ansi_term"' "$ROOT/Cargo.lock" || return 1
    ! grep -Eq '"lodash"' "$ROOT/web/package-lock.json" || return 1
    ! grep -REq --include='*.lockfile' '(^|:)jackson-databind:' "$ROOT/collector-android" || return 1
    ! grep -Eq '"node-forge"' "$ROOT/web/package-lock.json" || return 1
  }
  assert_self_test_dependencies_absent || return 1
  # 表の頭のひな形は、検査が受け付ける理由の形を示す（ひな形どおりに書いた行が落ちないように）
  grep -Eq '^# reason = "(届かない|上げる予定): ' "$ROOT/osv-scanner.toml" || return 1
  ! grep -Eq '^# reason = "CH[0-9]+: ' "$ROOT/osv-scanner.toml" || return 1
  expect_rc() {
    local expected="$1"; shift
    "$@" >"$tmp/actual" 2>&1
    local actual=$?
    [[ "$actual" == "$expected" ]]
  }
  exception_config() {
    printf '%s\n' "[[IgnoredVulns]]" "id = \"RUSTSEC-2021-0139\"" "$1" > "$tmp/root/osv-scanner.toml"
    git -C "$tmp/root" add osv-scanner.toml
  }
  echo "== Scenario: 理由の無い行で落ちる"
  vulnerable_tree
  exception_config "ignoreUntil = \"$(date -u -d '+1 day' +%F)\""
  expect_rc 1 "${common[@]}" "$SCRIPT_DIR/check-vulns.sh" || return 1
  grep -Eq '理由が無い.*RUSTSEC-2021-0139' "$tmp/actual" || return 1
  echo "== Scenario: 期限の無い行で落ちる"
  vulnerable_tree
  exception_config 'reason = "届かない: 自己試験"'
  expect_rc 1 "${common[@]}" "$SCRIPT_DIR/check-vulns.sh" || return 1
  grep -Eq '期限が無い.*RUSTSEC-2021-0139' "$tmp/actual" || return 1
  # 日付として読めない期限も、spec の 6 種の「期限が無い」で落とす（review/code.md R30）
  vulnerable_tree
  exception_config 'ignoreUntil = "2026-13-01"'$'\n''reason = "届かない: 自己試験"'
  expect_rc 1 "${common[@]}" "$SCRIPT_DIR/check-vulns.sh" || return 1
  grep -Eq '期限が無い.*RUSTSEC-2021-0139' "$tmp/actual" || return 1
  echo "== Scenario: 期限の切れた行で落ちる"
  vulnerable_tree
  exception_config "ignoreUntil = \"2000-01-01\""$'\n''reason = "届かない: 自己試験"'
  expect_rc 1 "${common[@]}" "$SCRIPT_DIR/check-vulns.sh" || return 1
  grep -Eq '期限切れ.*RUSTSEC-2021-0139' "$tmp/actual" || return 1
  echo "== Scenario: 期限が 90 日より先の行で落ちる"
  vulnerable_tree
  exception_config "ignoreUntil = \"$(date -u -d '+91 days' +%F)\""$'\n''reason = "届かない: 自己試験"'
  expect_rc 1 "${common[@]}" "$SCRIPT_DIR/check-vulns.sh" || return 1
  grep -Eq '期限が 90 日を越える.*RUSTSEC-2021-0139' "$tmp/actual" || return 1
  echo "== Scenario: 期限が 90 日ちょうどの行は通る"
  vulnerable_tree
  printf '{"name":"x","version":"1.0.0","lockfileVersion":3,"packages":{"":{"name":"x","version":"1.0.0","dependencies":{"lodash":"4.17.21"}},"node_modules/lodash":{"version":"4.17.21"}}}\n' > "$tmp/root/web/package-lock.json"
  printf 'com.example:safe:1.0.0=runtimeClasspath\n' > "$tmp/root/collector-android/gradle.lockfile"
  exception_config "ignoreUntil = \"$(date -u -d '+90 days' +%F)\""$'\n''reason = "届かない: 自己試験"'
  expect_rc 0 "${common[@]}" "$SCRIPT_DIR/check-vulns.sh" || return 1
  echo "== Scenario: 理由が決まった形で始まらない行で落ちる"
  vulnerable_tree
  exception_config "ignoreUntil = \"$(date -u -d '+1 day' +%F)\""$'\n''reason = "問題ない"'
  expect_rc 1 "${common[@]}" "$SCRIPT_DIR/check-vulns.sh" || return 1
  grep -Eq '理由の形が違う.*RUSTSEC-2021-0139' "$tmp/actual" || return 1
  echo "== Scenario: 同じ ID の行が 2 つあれば、後の正しい行で前の悪い行を隠せない"
  vulnerable_tree
  exception_config "ignoreUntil = \"2000-01-01\""$'\n''reason = "届かない: 自己試験"'$'\n''[[IgnoredVulns]]'$'\n''id = "RUSTSEC-2021-0139"'$'\n'"ignoreUntil = \"$(date -u -d '+1 day' +%F)\""$'\n''reason = "届かない: 自己試験"'
  expect_rc 1 "${common[@]}" "$SCRIPT_DIR/check-vulns.sh" || return 1
  grep -Eq 'ID が重複.*RUSTSEC-2021-0139' "$tmp/actual" || return 1
  echo "== Scenario: 引用符の無い日付の期限も、文字列の日付と同じに検査する"
  vulnerable_tree
  exception_config "ignoreUntil = 2000-01-01"$'\n''reason = "届かない: 自己試験"'
  expect_rc 1 "${common[@]}" "$SCRIPT_DIR/check-vulns.sh" || return 1
  grep -Eq '期限切れ.*RUSTSEC-2021-0139' "$tmp/actual" || return 1
  ! grep -Eq 'Traceback' "$tmp/actual" || return 1
  echo "== Scenario: 勧告がもう出ない行は残せない"
  safe_tree
  exception_config "ignoreUntil = \"$(date -u -d '+1 day' +%F)\""$'\n''reason = "届かない: 自己試験"'
  expect_rc 1 "${common[@]}" "$SCRIPT_DIR/check-vulns.sh" || return 1
  grep -Eq '勧告がもう出ない.*RUSTSEC-2021-0139' "$tmp/actual" || return 1
  echo "== Scenario: 3 つのロックファイルのどれに勧告のある版を入れても、全体の検査が落ちる"
  vulnerable_tree; expect_rc 1 "${common[@]}" "$SCRIPT_DIR/check-vulns.sh" || return 1
  grep -Eq 'ansi_term.*RUSTSEC-2021-0139' "$tmp/actual" || return 1
  grep -Eq 'lodash.*GHSA-35jh-r3h4-6jhm' "$tmp/actual" || return 1
  grep -Eq 'jackson-databind.*GHSA-57j2-w4cx-62h2' "$tmp/actual" || return 1
  echo "== Scenario: 勧告の無い版だけなら、全体の検査は通る"
  safe_tree; expect_rc 0 "${common[@]}" "$SCRIPT_DIR/check-vulns.sh" || return 1
  echo "== Scenario: 例外の表に勧告の ID があれば、その組では落ちない"
  safe_tree
  printf 'version = 3\n[[package]]\nname = "ansi_term"\nversion = "0.12.1"\n' > "$tmp/root/Cargo.lock"
  printf '[[IgnoredVulns]]\nid = "RUSTSEC-2021-0139"\nignoreUntil = "%s"\nreason = "届かない: 自己試験"\n' "$(date -u -d '+1 day' +%F)" > "$tmp/root/osv-scanner.toml"
  git -C "$tmp/root" add Cargo.lock osv-scanner.toml
  expect_rc 0 "${common[@]}" "$SCRIPT_DIR/check-vulns.sh" || return 1
  printf '# empty\n' > "$tmp/root/osv-scanner.toml"
  echo "== Scenario: 開発用の依存の勧告でも落ちる"
  safe_tree; python3 - "$tmp/root/web/package-lock.json" <<'PY'
import json, sys
p = json.load(open(sys.argv[1])); p['packages']['']['dependencies'] = {'lodash': '4.17.20'}; p['packages']['']['devDependencies'] = {'lodash': '4.17.20'}; p['packages']['node_modules/lodash'] = {'version': '4.17.20', 'resolved': 'https://registry.npmjs.org/lodash/-/lodash-4.17.20.tgz', 'dev': True}; json.dump(p, open(sys.argv[1], 'w'))
PY
  expect_rc 1 "${common[@]}" "$SCRIPT_DIR/check-vulns.sh" || return 1
  grep -Eq 'lodash.*GHSA-35jh-r3h4-6jhm' "$tmp/actual" || return 1
  echo "== Scenario: 「健全でない」「保守されていない」の知らせだけを持つ版でも落ちる"
  safe_tree; printf 'version = 3\n[[package]]\nname = "ansi_term"\nversion = "0.12.1"\n' > "$tmp/root/Cargo.lock"; git -C "$tmp/root" add Cargo.lock
  expect_rc 1 "${common[@]}" "$SCRIPT_DIR/check-vulns.sh" || return 1
  grep -Eq 'RUSTSEC-2021-0139' "$tmp/actual" || return 1
  echo "== Scenario: 重大度の低い勧告だけを持つ版でも落ちる"
  safe_tree
  local low_scanner="$tmp/low-scanner"
  printf '%s\n' '#!/usr/bin/env bash' "printf '%s\\n' '{\"results\":[{\"packages\":[{\"package\":{\"ecosystem\":\"npm\",\"name\":\"low-only\",\"version\":\"1.0.0\"},\"groups\":[{\"ids\":[\"GHSA-low-only\"],\"aliases\":[],\"max_severity\":\"3.7\"}]}]}]}'" 'exit 1' > "$low_scanner"
  chmod +x "$low_scanner"
  expect_rc 1 env CHECK_VULNS_ROOT="$tmp/root" OSV_SCANNER_BIN="$low_scanner" python3 "$SCRIPT_DIR/check-vulns.py" || return 1
  grep -Eq 'low-only.*GHSA-low-only.*severity=3\.7' "$tmp/actual" || return 1
  echo "== Scenario: 台本の sha256 と合わない osv-scanner は使わない"
  local fake="$tmp/fake-scanner" mark="$tmp/ran"
  printf '#!/usr/bin/env bash\ntouch "$MARK"\n' > "$fake"; chmod +x "$fake"
  expect_rc 2 env CHECK_VULNS_ROOT="$tmp/root" OSV_SCANNER_BIN="$fake" MARK="$mark" "$SCRIPT_DIR/check-vulns.sh" || return 1
  [[ ! -e "$mark" ]] || return 1
  mkdir -p "$tmp/fake-cache/ashiato2/osv-scanner-${VERSION}"
  cp "$fake" "$tmp/fake-cache/ashiato2/osv-scanner-${VERSION}/osv-scanner_linux_amd64"
  chmod +x "$tmp/fake-cache/ashiato2/osv-scanner-${VERSION}/osv-scanner_linux_amd64"
  rm -f "$mark"
  expect_rc 2 env CHECK_VULNS_ROOT="$tmp/root" XDG_CACHE_HOME="$tmp/fake-cache" MARK="$mark" "$SCRIPT_DIR/check-vulns.sh" || return 1
  grep -Eq 'sha256' "$tmp/actual" || return 1
  [[ ! -e "$mark" ]] || return 1
  echo "== Scenario: 見るはずのロックファイルが無ければ、黙って飛ばさない"
  safe_tree
  rm "$tmp/root/web/package-lock.json"
  "${common[@]}" "$SCRIPT_DIR/check-vulns.sh" >"$tmp/missing" 2>&1; [[ $? -eq 2 ]] || return 1
  grep -Eq 'error: 検査できなかった.*web/package-lock.json' "$tmp/missing" || return 1
  echo "== Scenario: osv.dev に届かないときは rc=2 で、勧告なしとしない"
  vulnerable_tree
  expect_rc 2 env CHECK_VULNS_ROOT="$tmp/root" OSV_SCANNER_BIN="$scanner" HTTPS_PROXY=http://127.0.0.1:9 NO_PROXY= "$SCRIPT_DIR/check-vulns.sh" || return 1
  grep -Eq 'error: 検査できなかった（osv.dev への問い合わせ' "$tmp/actual" || return 1
  git -C "$tmp/root" config user.email self-test@example.invalid
  git -C "$tmp/root" config user.name self-test
  safe_tree; git -C "$tmp/root" commit -qm base
  vulnerable_tree
  echo "== Scenario: 勧告のある版を足した PR は落ちる"
  expect_rc 1 env CHECK_VULNS_ROOT="$tmp/root" OSV_SCANNER_BIN="$scanner" "$SCRIPT_DIR/check-vulns.sh" --base HEAD || return 1
  grep -Eq '新しく持ち込んだ: [1-9][0-9]* 件' "$tmp/actual" || return 1
  grep -Eq 'lodash.*GHSA-35jh-r3h4-6jhm' "$tmp/actual" || return 1
  echo "== Scenario: base にもある勧告は、版を変えても新しいと数えない"
  git -C "$tmp/root" add . && git -C "$tmp/root" commit -qm vulnerable-base
  printf '%s\n' '{"name":"x","version":"1.0.0","lockfileVersion":3,"packages":{"":{"name":"x","version":"1.0.0","dependencies":{"lodash":"4.17.19"}},"node_modules/lodash":{"version":"4.17.19","resolved":"https://registry.npmjs.org/lodash/-/lodash-4.17.19.tgz"}}}' > "$tmp/root/web/package-lock.json"
  git -C "$tmp/root" add web/package-lock.json && expect_rc 0 env CHECK_VULNS_ROOT="$tmp/root" OSV_SCANNER_BIN="$scanner" "$SCRIPT_DIR/check-vulns.sh" --base HEAD || return 1
  grep -Eq '新しく持ち込んだ: 0 件' "$tmp/actual" || return 1
  echo "== Scenario: base にロックファイルも例外の表も無ければ、今の木の組と行は全部新しい"
  git -C "$tmp/root" rm -q Cargo.lock collector-android/gradle.lockfile osv-scanner.toml
  git -C "$tmp/root" commit -qm no-base-files
  safe_tree
  printf 'com.fasterxml.jackson.core:jackson-databind:2.9.10.8=runtimeClasspath\n' > "$tmp/root/collector-android/gradle.lockfile"
  python3 - "$scanner" "$tmp/root/collector-android/gradle.lockfile" "$tmp/root/osv-scanner.toml" "$(date -u -d '+1 day' +%F)" <<'PY'
import json, subprocess, sys
data = subprocess.run([sys.argv[1], 'scan', 'source', '--lockfile', sys.argv[2], '--format', 'json', '--all-vulns', '--all-packages', '--config', '/dev/null'], capture_output=True, text=True).stdout
ids = []
for result in json.loads(data).get('results', []):
    for package in result.get('packages', []):
        for group in package.get('groups', []):
            ids.extend(group.get('ids', []))
with open(sys.argv[3], 'w') as output:
    for ident in sorted(set(ids)):
        output.write(f'[[IgnoredVulns]]\nid = "{ident}"\nignoreUntil = "{sys.argv[4]}"\nreason = "届かない: 自己試験"\n')
PY
  git -C "$tmp/root" add Cargo.lock web/package-lock.json collector-android/gradle.lockfile osv-scanner.toml
  expect_rc 0 env CHECK_VULNS_ROOT="$tmp/root" OSV_SCANNER_BIN="$scanner" "$SCRIPT_DIR/check-vulns.sh" --base HEAD || return 1
  grep -Eq '新しく持ち込んだ: 0 件' "$tmp/actual" || return 1
  grep -Eq '== 例外の表で足した・変えた行' "$tmp/actual" || return 1
  grep -Eq 'GHSA-57j2-w4cx-62h2' "$tmp/actual" || return 1
  echo "== Scenario: base の commit を取り寄せられなければ比べない"
  expect_rc 2 env CHECK_VULNS_ROOT="$tmp/root" OSV_SCANNER_BIN="$scanner" "$SCRIPT_DIR/check-vulns.sh" --base does-not-exist || return 1
  grep -Eq '^error: 検査できなかった' "$tmp/actual" || return 1
  echo "== Scenario: 足した行と期限を延ばした行が一覧に出る"
  safe_tree
  printf '%s\n' '{"name":"x","version":"1.0.0","lockfileVersion":3,"packages":{"":{"name":"x","version":"1.0.0","dependencies":{"lodash":"4.17.20"}},"node_modules/lodash":{"version":"4.17.20","resolved":"https://registry.npmjs.org/lodash/-/lodash-4.17.20.tgz"}}}' > "$tmp/root/web/package-lock.json"
  python3 - "$scanner" "$tmp/root/web/package-lock.json" "$tmp/root/osv-scanner.toml" "$(date -u -d '+1 day' +%F)" <<'PY'
import json, subprocess, sys
data = subprocess.run([sys.argv[1], 'scan', 'source', '--lockfile', sys.argv[2], '--format', 'json', '--all-vulns', '--all-packages', '--config', '/dev/null'], capture_output=True, text=True).stdout
ids = []
for result in json.loads(data).get('results', []):
    for package in result.get('packages', []):
        for group in package.get('groups', []): ids.extend(group.get('ids', []))
with open(sys.argv[3], 'w') as output:
    for ident in sorted(set(ids)):
        if ident != 'GHSA-35jh-r3h4-6jhm':
            output.write(f'[[IgnoredVulns]]\nid = "{ident}"\nignoreUntil = "{sys.argv[4]}"\nreason = "届かない: A"\n')
PY
  git -C "$tmp/root" add web/package-lock.json osv-scanner.toml && git -C "$tmp/root" commit -qm exception-base
  sed -i "0,/$(date -u -d '+1 day' +%F)/s//$(date -u -d '+2 days' +%F)/" "$tmp/root/osv-scanner.toml"
  printf '%s\n' '[[IgnoredVulns]]' 'id = "GHSA-35jh-r3h4-6jhm"' "ignoreUntil = \"$(date -u -d '+1 day' +%F)\"" 'reason = "届かない: B"' >> "$tmp/root/osv-scanner.toml"
  expect_rc 0 env CHECK_VULNS_ROOT="$tmp/root" OSV_SCANNER_BIN="$scanner" "$SCRIPT_DIR/check-vulns.sh" --base HEAD || return 1
  grep -Eq "GHSA-29mw-wpgm-hmr9.*$(date -u -d '+2 days' +%F)" "$tmp/actual" || return 1
  grep -Eq 'GHSA-35jh-r3h4-6jhm' "$tmp/actual" || return 1
  echo "== Scenario: 例外の表を変えていなければ「なし」と出る"
  git -C "$tmp/root" checkout -q -- osv-scanner.toml
  expect_rc 0 env CHECK_VULNS_ROOT="$tmp/root" OSV_SCANNER_BIN="$scanner" "$SCRIPT_DIR/check-vulns.sh" --base HEAD || return 1
  grep -Eq '== 例外の表で足した・変えた行' "$tmp/actual" && grep -Eq 'なし' "$tmp/actual" || return 1
  echo "== Scenario: PR の検査でも、表の行の検査が走る"
  sed -i "s/$(date -u -d '+1 day' +%F)/2000-01-01/" "$tmp/root/osv-scanner.toml"
  expect_rc 1 env CHECK_VULNS_ROOT="$tmp/root" OSV_SCANNER_BIN="$scanner" "$SCRIPT_DIR/check-vulns.sh" --base HEAD || return 1
  grep -Eq '期限切れ' "$tmp/actual" || return 1
  grep -Eq '^新しく持ち込んだ: ' "$tmp/actual" || return 1
  grep -Eq '^== 例外の表で足した・変えた行$' "$tmp/actual" || return 1
  echo "== Scenario: PR の検査でも、osv.dev に届かなければ rc=2"
  git -C "$tmp/root" checkout -q -- osv-scanner.toml
  expect_rc 2 env CHECK_VULNS_ROOT="$tmp/root" OSV_SCANNER_BIN="$scanner" HTTPS_PROXY=http://127.0.0.1:9 NO_PROXY= "$SCRIPT_DIR/check-vulns.sh" --base HEAD || return 1
  grep -Eq '^error: 検査できなかった' "$tmp/actual" || return 1
  echo "== Scenario: 許可の一覧は例外の表と別名から組む"
  printf '%s\n' '[[IgnoredVulns]]' 'id = "RUSTSEC-2020-0071"' "ignoreUntil = \"$(date -u -d '+1 day' +%F)\"" 'reason = "届かない: 自己試験"' '[[IgnoredVulns]]' 'id = "GHSA-35jh-r3h4-6jhm"' "ignoreUntil = \"$(date -u -d '+1 day' +%F)\"" 'reason = "届かない: 自己試験"' > "$tmp/root/osv-scanner.toml"
  expect_rc 0 env CHECK_VULNS_ROOT="$tmp/root" "$SCRIPT_DIR/check-vulns.sh" allow-ghsas || return 1
  [[ "$(cat "$tmp/actual")" == 'GHSA-wcg3-cvx6-7396,GHSA-35jh-r3h4-6jhm' ]] || return 1
  echo "== Scenario: 別名を問い合わせられなければ、許可の一覧を出さない"
  expect_rc 2 env HTTPS_PROXY=http://127.0.0.1:9 NO_PROXY= CHECK_VULNS_ROOT="$tmp/root" "$SCRIPT_DIR/check-vulns.sh" allow-ghsas || return 1
  ! grep -Eq '^GHSA-' "$tmp/actual" || return 1
  echo "== Scenario: 依存に関わるファイルを変えた PR で走り、それ以外では走らない"
  for path in Cargo.lock web/package-lock.json collector-android/app/gradle.lockfile osv-scanner.toml tools/check-vulns.sh; do
    printf '%s\n' "$path" | "$ROOT/tools/release/changes.sh" deps || return 1
  done
  if printf '%s\n' .github/workflows/ci.yml | "$ROOT/tools/release/changes.sh" deps; then return 1; fi
  local deps_path
  for deps_path in tools/check-vulns.py tools/check-vulns-issues.py tools/check-vulns.sh tools/check-pg-minor.sh tools/release/changes.sh; do
    printf '%s\n' "$deps_path" | "$ROOT/tools/release/changes.sh" deps || return 1
  done
  if printf '%s\n' docs/x.md | "$ROOT/tools/release/changes.sh" deps; then return 1; fi
  if printf '%s\n' crates/server/src/lib.rs docs/x.md | "$ROOT/tools/release/changes.sh" deps; then return 1; fi
  if printf '%s\n' docs/x.md | "$ROOT/tools/release/changes.sh" code; then return 1; fi
  printf '%s\n' crates/server/src/lib.rs | "$ROOT/tools/release/changes.sh" code || return 1
  local fake_gh="$tmp/gh" gh_log="$tmp/gh.log" gh_body_dir="$tmp/gh-bodies"
  mkdir -p "$gh_body_dir"
  cat >"$fake_gh" <<'GH'
#!/usr/bin/env bash
set -u
printf '%s\n' "$*" >>"$GH_LOG"
body=""
limit=30
for ((i = 1; i <= $#; i++)); do
  if [[ "${!i}" == --body && $((i + 1)) -le $# ]]; then
    j=$((i + 1)); body="${!j}"
  fi
  if [[ "${!i}" == --limit && $((i + 1)) -le $# ]]; then
    j=$((i + 1)); limit="${!j}"
  fi
done
case "$1 ${2:-}" in
  "label list") [[ "${GH_LABEL_EXISTS:-}" == 1 ]] && printf '[{"name":"脆弱性"}]\n' || printf '[]\n' ;;
  "label create") printf '[]\n' ;;
  "issue list") if [[ -n "${GH_MANY_ISSUES:-}" ]]; then
      # gh と同じく、既定では 30 件までしか返さない
      for ((k = 1; k <= GH_MANY_ISSUES && k <= limit; k++)); do printf '{"number":%d,"title":"脆弱性: npm pkg%d","body":"勧告: GHSA-pkg%d"}\n' "$k" "$k" "$k"; done | python3 -c 'import json,sys; print(json.dumps([json.loads(l) for l in sys.stdin]))'
    elif [[ "${GH_TABLE_ISSUE:-}" == 1 ]]; then printf '[{"number":11,"title":"脆弱性: 例外の表","body":"勧告: A-1\\n理由: 期限切れ"}]\n'
    elif [[ "${GH_PG_ISSUE:-}" == 1 ]]; then printf '[{"number":10,"title":"脆弱性: PostgreSQL 17 の minor の遅れ","body":"同梱: 17.6\\n最新: 17.11"}]\n'; elif [[ "${GH_ONLY_EXISTING:-}" == 1 ]]; then printf '[{"number":7,"title":"脆弱性: npm lodash","body":"勧告: GHSA-35jh-r3h4-6jhm"}]\n'; elif [[ "${GH_OPEN_ISSUE:-}" == 1 ]]; then printf '[{"number":7,"title":"脆弱性: npm lodash","body":"勧告: GHSA-35jh-r3h4-6jhm\\n勧告: GHSA-29mw-wpgm-hmr9\\n勧告: GHSA-r5fr-rjxr-66jc\\n勧告: GHSA-xxjr-mmjv-4gpg"},{"number":8,"title":"脆弱性: crates.io ansi_term","body":"勧告: RUSTSEC-2021-0139"},{"number":9,"title":"脆弱性: Maven com.fasterxml.jackson.core:jackson-databind","body":"勧告: GHSA-57j2-w4cx-62h2"}]\n'; else printf '[]\n'; fi ;;
  "issue view") [[ "${GH_OPEN_ISSUE:-}" == 1 ]] && printf '{"comments":[]}\n' || printf '{"comments":[]}\n' ;;
  "issue create")
    n=$(find "$GH_BODY_DIR" -name 'create-*' | wc -l)
    printf '%s' "$body" >"$GH_BODY_DIR/create-$((n + 1))"
    exit 0 ;;
  "issue comment")
    n=$(find "$GH_BODY_DIR" -name 'comment-*' | wc -l)
    printf '%s' "$body" >"$GH_BODY_DIR/comment-$((n + 1))"
    exit 0 ;;
esac
GH
  chmod +x "$fake_gh"
  echo "== Scenario: 届かなかった夜は issue を立てない"
  vulnerable_tree
  printf 'com.example:safe:1.0.0=runtimeClasspath\n' >"$tmp/root/collector-android/gradle.lockfile"
  : >"$gh_log"
  rm -rf "$tmp/findings"
  local unreachable_rc=0
  PATH="$tmp:$PATH" GH_LOG="$gh_log" OSV_SCANNER_BIN="$scanner" CHECK_VULNS_ROOT="$tmp/root" HTTPS_PROXY=http://127.0.0.1:9 NO_PROXY= "$SCRIPT_DIR/check-vulns.sh" nightly "$tmp/findings" || unreachable_rc=$?
  [[ "$unreachable_rc" == 2 ]] || return 1
  local file_unreachable_rc=0
  PATH="$tmp:$PATH" GH_LOG="$gh_log" "$SCRIPT_DIR/check-vulns.sh" file-issues "$tmp/findings" || file_unreachable_rc=$?
  [[ "$file_unreachable_rc" == 2 ]] || return 1
  [[ ! -e "$tmp/findings/vulnerabilities.json" ]] || return 1
  ! grep -Eq 'issue (create|comment)' "$gh_log" || return 1
  echo "== Scenario: 落ちた夜に、依存ごとに 1 件の issue が立つ"
  vulnerable_tree
  printf 'com.example:safe:1.0.0=runtimeClasspath\n' >"$tmp/root/collector-android/gradle.lockfile"
  : >"$gh_log"
  PATH="$tmp:$PATH" GH_LOG="$gh_log" GH_BODY_DIR="$gh_body_dir" GH_LABEL_EXISTS=0 OSV_SCANNER_BIN="$scanner" CHECK_VULNS_ROOT="$tmp/root" "$SCRIPT_DIR/check-vulns.sh" nightly "$tmp/findings" || [[ $? -eq 1 ]] || return 1
  PATH="$tmp:$PATH" GH_LOG="$gh_log" GH_BODY_DIR="$gh_body_dir" GH_LABEL_EXISTS=0 "$SCRIPT_DIR/check-vulns.sh" file-issues "$tmp/findings" || return 1
  [[ "$(grep -Ec '^issue create ' "$gh_log")" == 2 ]] || return 1
  grep -Eq '^label create 脆弱性' "$gh_log" || return 1
  printf '%s' $'依存: ansi_term\n版: 0.12.1\n勧告: RUSTSEC-2021-0139\n重大度: 不明\nURL: https://osv.dev/vulnerability/RUSTSEC-2021-0139' | diff -u - "$gh_body_dir/create-1" || return 1
  printf '%s' $'依存: lodash\n版: 4.17.20\n勧告: GHSA-29mw-wpgm-hmr9\n重大度: 5.3\nURL: https://osv.dev/vulnerability/GHSA-29mw-wpgm-hmr9\n依存: lodash\n版: 4.17.20\n勧告: GHSA-35jh-r3h4-6jhm\n重大度: 7.2\nURL: https://osv.dev/vulnerability/GHSA-35jh-r3h4-6jhm\n依存: lodash\n版: 4.17.20\n勧告: GHSA-r5fr-rjxr-66jc\n重大度: 8.1\nURL: https://osv.dev/vulnerability/GHSA-r5fr-rjxr-66jc\n依存: lodash\n版: 4.17.20\n勧告: GHSA-xxjr-mmjv-4gpg\n重大度: 6.9\nURL: https://osv.dev/vulnerability/GHSA-xxjr-mmjv-4gpg' | diff -u - "$gh_body_dir/create-2" || return 1
  echo "== Scenario: 同じ依存の issue が開いていれば、立て直さない"
  printf '%s\n' '[{"kind":"vulnerability","ecosystem":"npm","name":"lodash","version":"4.17.20","id":"GHSA-35jh-r3h4-6jhm","severity":"HIGH"}]' >"$tmp/existing.json"
  mkdir -p "$tmp/existing-dir"
  cp "$tmp/existing.json" "$tmp/existing-dir/vulnerabilities.json"
  : >"$gh_log"
  PATH="$tmp:$PATH" GH_LOG="$gh_log" GH_OPEN_ISSUE=1 GH_ONLY_EXISTING=1 "$SCRIPT_DIR/check-vulns.sh" file-issues "$tmp/existing-dir"; local duplicate_rc=$?
  [[ "$duplicate_rc" == 0 ]] || return 1
  ! grep -Eq '^issue (create|comment) ' "$gh_log" || return 1
  echo "== Scenario: 開いた issue が 30 件を越えても、31 件目の依存の issue を立て直さない"
  printf '%s\n' '[{"kind":"vulnerability","ecosystem":"npm","name":"pkg31","version":"1.0.0","id":"GHSA-pkg31","severity":"HIGH"}]' >"$tmp/pkg31.json"
  : >"$gh_log"
  PATH="$tmp:$PATH" GH_LOG="$gh_log" GH_BODY_DIR="$gh_body_dir" GH_LABEL_EXISTS=1 GH_MANY_ISSUES=31 "$SCRIPT_DIR/check-vulns.sh" file-issues "$tmp/pkg31.json" || return 1
  ! grep -Eq '^issue (create|comment) ' "$gh_log" || return 1
  echo "== Scenario: 勧告が増えたときだけ、開いている issue に追記する"
  cat >"$tmp/finding.json" <<'JSON'
[{"kind":"vulnerability","ecosystem":"npm","name":"lodash","version":"4.17.20","id":"GHSA-35jh-r3h4-6jhm","severity":"HIGH"},{"kind":"vulnerability","ecosystem":"npm","name":"lodash","version":"4.17.20","id":"GHSA-new","severity":"LOW"}]
JSON
  : >"$gh_log"
  PATH="$tmp:$PATH" GH_LOG="$gh_log" GH_BODY_DIR="$gh_body_dir" GH_OPEN_ISSUE=1 "$SCRIPT_DIR/check-vulns.sh" file-issues "$tmp/finding.json" || return 1
  [[ "$(grep -Ec '^issue comment ' "$gh_log")" == 1 ]] || return 1
  printf '%s' '勧告: GHSA-new' | cmp -s - "$gh_body_dir/comment-1" || return 1
  echo "== Scenario: issue に書くのは決まった項目だけ"
  grep -Eq 'issue comment .*勧告: GHSA-new' "$gh_log" || return 1
  ! grep -Eq '届く|分析|見出し' "$gh_log" || return 1
  echo "== Scenario: 勧告が消えた依存の issue は閉じない"
  ! grep -Eq '^issue close ' "$gh_log" || return 1
  echo "== Scenario: 表の行の問題は、例外の表の 1 件の issue になる"
  safe_tree
  # A-3 は形の正しい行だが、勧告の無い木では当たらない（勧告がもう出ない。review/code.md R22）
  printf '%s\n' '[[IgnoredVulns]]' 'id = "A-1"' 'ignoreUntil = "2000-01-01"' 'reason = "届かない: 網"' '[[IgnoredVulns]]' 'id = "A-2"' 'ignoreUntil = "2099-01-01"' '[[IgnoredVulns]]' 'id = "A-3"' "ignoreUntil = \"$(date -u -d '+1 day' +%F)\"" 'reason = "届かない: 自己試験"' >"$tmp/root/osv-scanner.toml"
  git -C "$tmp/root" add osv-scanner.toml
  : >"$gh_log"
  rm -rf "$tmp/findings"
  local table_rc=0
  PATH="$tmp:$PATH" GH_LOG="$gh_log" OSV_SCANNER_BIN="$scanner" CHECK_VULNS_ROOT="$tmp/root" "$SCRIPT_DIR/check-vulns.sh" nightly "$tmp/findings" || table_rc=$?
  [[ "$table_rc" == 1 ]] || return 1
  PATH="$tmp:$PATH" GH_LOG="$gh_log" GH_BODY_DIR="$gh_body_dir" GH_OPEN_ISSUE=0 "$SCRIPT_DIR/check-vulns.sh" file-issues "$tmp/findings" || return 1
  [[ "$(grep -Ec '^issue create ' "$gh_log")" == 1 ]] || return 1
  grep -Eq '脆弱性: 例外の表' "$gh_log" && grep -Eq 'A-1' "$gh_log" && grep -Eq '期限切れ' "$gh_log" && grep -Eq 'A-2' "$gh_log" && grep -Eq '理由が無い' "$gh_log" || return 1
  grep -Eq 'A-3' "$gh_log" && grep -Eq '勧告がもう出ない' "$gh_log" || return 1
  echo "== Scenario: 例外の表の issue が開いていれば、新しい行の問題だけを追記する"
  : >"$gh_log"
  rm -f "$gh_body_dir"/comment-*
  PATH="$tmp:$PATH" GH_LOG="$gh_log" GH_BODY_DIR="$gh_body_dir" GH_TABLE_ISSUE=1 "$SCRIPT_DIR/check-vulns.sh" file-issues "$tmp/findings" || return 1
  ! grep -Eq '^issue create ' "$gh_log" || return 1
  [[ "$(grep -Ec '^issue comment 11 ' "$gh_log")" == 1 ]] || return 1
  printf '%s' $'勧告: A-2\n理由: 理由が無い\n勧告: A-3\n理由: 勧告がもう出ない' | diff -u - "$gh_body_dir/comment-1" || return 1
  echo "== Scenario: PostgreSQL の遅れは 1 件の issue になり、開いていれば立て直さない"
  # 材料は手で書かず、check-pg-minor.sh --findings が書いたものをそのまま file-issues に渡す（review/code.md R23）。
  # 置き場の親に . を含めて、パスの形で置き場の扱いが変わらないことも見る。
  local pg_edb="$tmp/pg-edb" pg_versions="$tmp/pg-versions.json" pg_findings="$tmp/pg.v1/findings" pg_date
  pg_date="$(date -u -d '-5 days' +%F)"
  mkdir -p "$pg_edb"
  printf 'version=17.6-1\nurl=file://%s/postgresql-17.6-1-windows-x64-binaries.zip\n' "$pg_edb" >"$tmp/pgsql.lock"
  printf '[{"major":"17","latestMinor":"11","relDate":"%s"}]\n' "$pg_date" >"$pg_versions"
  : >"$pg_edb/postgresql-17.11-1-windows-x64-binaries.zip"
  local pg_rc=0
  PGSQL_LOCK_FILE="$tmp/pgsql.lock" PG_VERSIONS_URL="file://$pg_versions" "$SCRIPT_DIR/check-pg-minor.sh" --findings "$pg_findings" >/dev/null 2>&1 || pg_rc=$?
  [[ "$pg_rc" == 1 && -f "$pg_findings/postgresql-minor.txt" ]] || return 1
  : >"$gh_log"
  rm -f "$gh_body_dir"/create-*
  PATH="$tmp:$PATH" GH_LOG="$gh_log" GH_BODY_DIR="$gh_body_dir" GH_OPEN_ISSUE=0 "$SCRIPT_DIR/check-vulns.sh" file-issues "$pg_findings" || return 1
  [[ "$(grep -Ec '^issue create ' "$gh_log")" == 1 ]] || return 1
  grep -Eq '^issue create --title 脆弱性: PostgreSQL 17 の minor の遅れ ' "$gh_log" || return 1
  printf '%s' "同梱: 17.6"$'\n'"最新: 17.11"$'\n'"EDB の zip: present"$'\n'"公開日: $pg_date" | diff -u - "$gh_body_dir/create-1" || return 1
  echo "== Scenario: PostgreSQL の issue が開いていれば、EDB の状態や公開日の行が違っても追記しない"
  : >"$gh_log"
  PATH="$tmp:$PATH" GH_LOG="$gh_log" GH_BODY_DIR="$gh_body_dir" GH_OPEN_ISSUE=1 GH_PG_ISSUE=1 "$SCRIPT_DIR/check-vulns.sh" file-issues "$pg_findings" || return 1
  ! grep -Eq '^issue (create|comment) ' "$gh_log" || return 1
  echo "== Scenario: PostgreSQL の issue が開いたまま新しい minor が出たら、追記する"
  printf '[{"major":"17","latestMinor":"12","relDate":"%s"}]\n' "$pg_date" >"$pg_versions"
  : >"$pg_edb/postgresql-17.12-1-windows-x64-binaries.zip"
  pg_rc=0
  PGSQL_LOCK_FILE="$tmp/pgsql.lock" PG_VERSIONS_URL="file://$pg_versions" "$SCRIPT_DIR/check-pg-minor.sh" --findings "$pg_findings" >/dev/null 2>&1 || pg_rc=$?
  [[ "$pg_rc" == 1 ]] || return 1
  : >"$gh_log"
  rm -f "$gh_body_dir"/comment-*
  PATH="$tmp:$PATH" GH_LOG="$gh_log" GH_BODY_DIR="$gh_body_dir" GH_OPEN_ISSUE=1 GH_PG_ISSUE=1 "$SCRIPT_DIR/check-vulns.sh" file-issues "$pg_findings" || return 1
  ! grep -Eq '^issue create ' "$gh_log" || return 1
  printf '%s' "同梱: 17.6"$'\n'"最新: 17.12"$'\n'"EDB の zip: present"$'\n'"公開日: $pg_date" | diff -u - "$gh_body_dir/comment-1" || return 1
  return 0
}

main() {
  if [[ "${1:-}" == "--self-test" ]]; then self_test; return $?; fi
  if [[ "${1:-}" == "allow-ghsas" ]]; then
    CHECK_VULNS_ROOT="$ROOT" python3 "$SCRIPT_DIR/check-vulns.py" --allow-ghsas
    return $?
  fi
  if [[ "${1:-}" == "github-settings" ]]; then
    [[ $# == 1 ]] || { echo "error: 検査できなかった（使い方）" >&2; return 2; }
    github_settings
    return $?
  fi
  if [[ "${1:-}" == "nightly" || "${1:-}" == "file-issues" ]]; then
    [[ $# == 2 ]] || { echo "error: 検査できなかった（使い方）"; return 2; }
    if [[ "$1" == nightly ]]; then
      local scanner
      scanner="$(scanner_path)" || return $?
      CHECK_VULNS_ROOT="$ROOT" OSV_SCANNER_BIN="$scanner" python3 "$SCRIPT_DIR/check-vulns-issues.py" "$@"
    else
      CHECK_VULNS_ROOT="$ROOT" python3 "$SCRIPT_DIR/check-vulns-issues.py" "$@"
    fi
    return $?
  fi
  local scanner
  scanner="$(scanner_path)" || return $?
  case "${1:-}" in
    "") CHECK_VULNS_ROOT="$ROOT" OSV_SCANNER_BIN="$scanner" python3 "$SCRIPT_DIR/check-vulns.py" ;;
    --base) [[ $# == 2 ]] || { echo "error: 検査できなかった（使い方）"; return 2; }; CHECK_VULNS_ROOT="$ROOT" OSV_SCANNER_BIN="$scanner" python3 "$SCRIPT_DIR/check-vulns.py" --base "$2" ;;
    *) echo "error: 検査できなかった（使い方）"; return 2 ;;
  esac
}
main "$@"
