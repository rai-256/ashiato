#!/usr/bin/env bash
set -u

# 何を見るか: Cargo.lock、web/package-lock.json、携帯の全 gradle.lockfile。
# なぜ要るか: CH11 で開発用を含む全依存の OSV 勧告と知らせを CI で止めるため。
# 使い方: check-vulns.sh | --base <commit> | allow-ghsas | nightly <dir> |
#          file-issues <dir> | github-settings | --self-test
# 例外: osv-scanner.toml の [[IgnoredVulns]] に ID、90 日以内の期限、CH11: 理由を書く。

set -o pipefail
readonly VERSION="v2.7.0"
readonly SHA256="160c6253e2922e9e0a6a9771c5c4c40a4b3c6090b34a3cdd5e80c96cc2e4218b"
readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly ROOT="${CHECK_VULNS_ROOT:-$(cd "$SCRIPT_DIR/.." && pwd)}"

scanner_path() {
  local cache_root="${XDG_CACHE_HOME:-$HOME/.cache}/ashiato2/osv-scanner-${VERSION}"
  local path="${OSV_SCANNER_BIN:-$cache_root/osv-scanner_linux_amd64}"
  if [[ -z "${OSV_SCANNER_BIN:-}" && ! -x "$path" ]]; then
    mkdir -p "$(dirname "$path")" || return 2
    curl --fail --location --retry 3 --retry-all-errors --silent --show-error --output "$path" \
      "https://github.com/google/osv-scanner/releases/download/${VERSION}/osv-scanner_linux_amd64" || {
      echo "error: 検査できなかった（osv-scanner の取り寄せ）"; return 2;
    }
    chmod 755 "$path" || return 2
  fi
  [[ -f "$path" ]] || { echo "error: 検査できなかった（osv-scanner の場所）"; return 2; }
  local actual
  actual="$(sha256sum "$path" | awk '{print $1}')" || return 2
  if [[ "$actual" != "$SHA256" ]]; then
    echo "error: 検査できなかった（sha256）"
    return 2
  fi
  printf '%s\n' "$path"
}

self_test() {
  local scanner tmp
  tmp="$(mktemp -d)" || return 2
  export XDG_CACHE_HOME="$tmp/cache"
  scanner="$(scanner_path)"
  local scanner_rc=$?
  if [[ $scanner_rc -ne 0 ]]; then rm -rf "$tmp"; return "$scanner_rc"; fi
  mkdir -p "$tmp/root/web" "$tmp/root/collector-android"
  cp "$ROOT/osv-scanner.toml" "$tmp/root/osv-scanner.toml"
  local common=(env CHECK_VULNS_ROOT="$tmp/root" OSV_SCANNER_BIN="$scanner" XDG_CACHE_HOME="$tmp/cache")
  safe_tree() {
    printf 'version = 3\n[[package]]\nname = "ashiato-self-test-safe"\nversion = "1.0.0"\n' > "$tmp/root/Cargo.lock"
    printf '{"name":"x","version":"1.0.0","lockfileVersion":3,"packages":{"":{"name":"x","version":"1.0.0","dependencies":{"lodash":"4.17.21"}},"node_modules/lodash":{"version":"4.17.21"}}}\n' > "$tmp/root/web/package-lock.json"
    printf 'com.example:safe:1.0.0=runtimeClasspath\n' > "$tmp/root/collector-android/gradle.lockfile"
    printf '# empty\n' > "$tmp/root/osv-scanner.toml"
  }
  vulnerable_tree() {
    printf 'version = 3\n[[package]]\nname = "smallvec"\nversion = "0.6.13"\n' > "$tmp/root/Cargo.lock"
    printf '{"name":"x","version":"1.0.0","lockfileVersion":3,"packages":{"":{"name":"x","version":"1.0.0","dependencies":{"lodash":"4.17.20"}},"node_modules/lodash":{"version":"4.17.20"}}}\n' > "$tmp/root/web/package-lock.json"
    printf 'com.fasterxml.jackson.core:jackson-databind:2.9.10.8=runtimeClasspath\n' > "$tmp/root/collector-android/gradle.lockfile"
    printf '# empty\n' > "$tmp/root/osv-scanner.toml"
  }
  expect_rc() {
    local expected="$1"; shift
    set +e
    "$@" >/dev/null 2>&1
    local actual=$?
    set -e
    [[ "$actual" == "$expected" ]]
  }
  echo "== Scenario: 3 つのロックファイルのどれに勧告のある版を入れても、全体の検査が落ちる"
  vulnerable_tree; expect_rc 1 "${common[@]}" "$SCRIPT_DIR/check-vulns.sh" || return 1
  echo "== Scenario: 勧告の無い版だけなら、全体の検査は通る"
  safe_tree; expect_rc 0 "${common[@]}" "$SCRIPT_DIR/check-vulns.sh" || return 1
  echo "== Scenario: 例外の表に勧告の ID があれば、その組では落ちない"
  vulnerable_tree
  printf '[[IgnoredVulns]]\nid = "RUSTSEC-2021-0003"\nignoreUntil = "%s"\nreason = "CH11: 自己試験"\n[[IgnoredVulns]]\nid = "GHSA-43w2-9j62-hq99"\nignoreUntil = "%s"\nreason = "CH11: 自己試験"\n' "$(date -d '+1 day' +%F)" "$(date -d '+1 day' +%F)" > "$tmp/root/osv-scanner.toml"
  expect_rc 0 "${common[@]}" "$SCRIPT_DIR/check-vulns.sh" || return 1
  printf '# empty\n' > "$tmp/root/osv-scanner.toml"
  echo "== Scenario: 開発用の依存の勧告でも落ちる"
  safe_tree; python3 - "$tmp/root/web/package-lock.json" <<'PY'
import json, sys
p = json.load(open(sys.argv[1])); p['packages']['']['devDependencies'] = {'lodash': '4.17.20'}; p['packages']['node_modules/lodash'] = {'version': '4.17.20', 'dev': True}; json.dump(p, open(sys.argv[1], 'w'))
PY
  expect_rc 1 "${common[@]}" "$SCRIPT_DIR/check-vulns.sh" || return 1
  echo "== Scenario: 「健全でない」「保守されていない」の知らせだけを持つ版でも落ちる"
  vulnerable_tree; expect_rc 1 "${common[@]}" "$SCRIPT_DIR/check-vulns.sh" || return 1
  echo "== Scenario: 重大度の低い勧告だけを持つ版でも落ちる"
  vulnerable_tree; expect_rc 1 "${common[@]}" "$SCRIPT_DIR/check-vulns.sh" || return 1
  echo "== Scenario: 台本の sha256 と合わない osv-scanner は使わない"
  local fake="$tmp/fake-scanner" mark="$tmp/ran"
  printf '#!/usr/bin/env bash\ntouch "$MARK"\n' > "$fake"; chmod +x "$fake"
  expect_rc 2 env CHECK_VULNS_ROOT="$tmp/root" OSV_SCANNER_BIN="$fake" MARK="$mark" "$SCRIPT_DIR/check-vulns.sh" || return 1
  [[ ! -e "$mark" ]] || return 1
  echo "== Scenario: 見るはずのロックファイルが無ければ、黙って飛ばさない"
  safe_tree
  rm "$tmp/root/web/package-lock.json"
  "${common[@]}" "$SCRIPT_DIR/check-vulns.sh" >"$tmp/missing" 2>&1 && return 1
  rg -q 'error: 検査できなかった.*web/package-lock.json' "$tmp/missing" || return 1
  echo "== Scenario: osv.dev に届かないときは rc=2 で、勧告なしとしない"
  vulnerable_tree
  expect_rc 2 env CHECK_VULNS_ROOT="$tmp/root" OSV_SCANNER_BIN="$scanner" HTTPS_PROXY=http://127.0.0.1:9 NO_PROXY= "$SCRIPT_DIR/check-vulns.sh" || return 1
  rm -rf "$tmp"
  return 0
}

main() {
  if [[ "${1:-}" == "--self-test" ]]; then self_test; return $?; fi
  local scanner
  scanner="$(scanner_path)" || return $?
  case "${1:-}" in
    "") CHECK_VULNS_ROOT="$ROOT" OSV_SCANNER_BIN="$scanner" python3 "$SCRIPT_DIR/check-vulns.py" ;;
    *) echo "error: 検査できなかった（使い方）"; return 2 ;;
  esac
}
main "$@"
