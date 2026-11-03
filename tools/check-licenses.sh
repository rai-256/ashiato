#!/usr/bin/env bash
# Rust・Node の依存の許諾が AGPL-3.0 と両立するかを機械で確かめる。
set -euo pipefail
cd "$(dirname "$0")/.."

SELF_LICENSE='AGPL-3.0-only WITH AdditionRef-ashiato-play-services'
ALLOW="${ALLOW_LICENSES:-MIT,MIT-0,Apache-2.0,BSD-2-Clause,BSD-3-Clause,ISC,Unicode-3.0,Unicode-DFS-2016,Zlib,CC0-1.0,MPL-2.0,0BSD,Apache-2.0 WITH LLVM-exception,CDLA-Permissive-2.0,BlueOak-1.0.0,CC-BY-4.0,Python-2.0}"

check_section() {
  local section=$1 input=$2
  python3 - "$ALLOW" "$SELF_LICENSE" "$section" "$input" <<'PY'
import json
import os
import re
import sys

allow = {item.strip() for item in sys.argv[1].split(",")}
self_license = sys.argv[2]
section = sys.argv[3]
input_path = sys.argv[4]

def allowed(expr):
    """SPDX の AND/OR 式を評価する（WITH は 1 つの原子として扱う）。"""
    tokens = re.findall(r"\(|\)|[A-Za-z0-9.\-+]+(?:\s+WITH\s+[A-Za-z0-9.\-]+)?|AND|OR", expr)
    position = 0
    def parse_or():
        nonlocal position
        value = parse_and()
        while position < len(tokens) and tokens[position] == "OR":
            position += 1
            value = parse_and() or value
        return value
    def parse_and():
        nonlocal position
        value = parse_atom()
        while position < len(tokens) and tokens[position] == "AND":
            position += 1
            value = parse_atom() and value
        return value
    def parse_atom():
        nonlocal position
        if position < len(tokens) and tokens[position] == "(":
            position += 1
            value = parse_or()
            if position < len(tokens) and tokens[position] == ")":
                position += 1
            return value
        token = tokens[position] if position < len(tokens) else ""
        position += 1
        return token in allow
    return bool(tokens) and parse_or()

def check_package(license_value, own):
    license_value = str(license_value or "").strip()
    if own and license_value == self_license:
        return True
    return bool(license_value) and allowed(license_value.replace("/", " OR "))

bad = []
count = 0
if section == "rust":
    with open(input_path, encoding="utf-8") as stream:
        metadata = json.load(stream)
    members = set(metadata.get("workspace_members", []))
    for package in metadata.get("packages", []):
        count += 1
        if not check_package(package.get("license"), package.get("id") in members):
            bad.append((package.get("name", ""), package.get("license") or "(許諾の記載が無い)"))
elif section == "node":
    root = input_path
    parent_package = os.path.join(os.path.dirname(root), "package.json")
    packages = []
    if os.path.isfile(parent_package):
        with open(parent_package, encoding="utf-8") as stream:
            packages.append(json.load(stream))
    if not os.path.isdir(root):
        print(f"  NG node_modules が無い: {root}（この検査は空振りしている）")
        sys.exit(1)
    for directory, _, filenames in os.walk(root):
        if "package.json" not in filenames:
            continue
        try:
            with open(os.path.join(directory, "package.json"), encoding="utf-8") as stream:
                packages.append(json.load(stream))
        except (OSError, ValueError):
            continue
    root_name = packages[0].get("name") if packages else None
    for package in packages:
        if "name" not in package or "version" not in package:
            continue
        count += 1
        license_value = package.get("license", "")
        if isinstance(license_value, dict):
            license_value = license_value.get("type", "")
        if not check_package(license_value, package["name"] == root_name):
            bad.append((package["name"], license_value or "(許諾の記載が無い)"))
else:
    raise SystemExit(f"未知の節: {section}")

for name, license_value in bad:
    print(f"  NG {name}: {license_value}")
print(f"  {count} 件を確認 / 不許可 {len(bad)} 件")
sys.exit(1 if bad else 0)
PY
}

run_rust() {
  local metadata=$1
  if [[ -z $metadata ]]; then
    metadata=$(mktemp "$work_tmp/meta.XXXXXX.json")
    cargo metadata --format-version 1 --all-features >"$metadata"
  fi
  echo "== Rust の依存"
  check_section rust "$metadata"
}

run_node() {
  echo "== Node の依存"
  check_section node "${ASHIATO_LICENSES_NODE_ROOT:-web/node_modules}"
}

self_test() {
  local test_root cargo_meta node_root
  test_root=$(mktemp -d)
  trap 'rm -rf "$test_root"' RETURN
  cargo_meta="$test_root/meta.json"
  node_root="$test_root/web/node_modules"
  mkdir -p "$node_root/allowed" "$test_root/web"
  cat >"$test_root/web/package.json" <<'JSON'
{"name":"ashiato-web","version":"0.0.0","license":"AGPL-3.0-only WITH AdditionRef-ashiato-play-services"}
JSON
  cat >"$node_root/allowed/package.json" <<'JSON'
{"name":"allowed-package","version":"1.0.0","license":"MIT"}
JSON
  echo "== Scenario: 他人の部品の AGPL の許諾は、許諾の検査が落とす"
  cat >"$cargo_meta" <<'JSON'
{"packages":[{"id":"path+fake#evil-agpl","name":"evil-agpl","license":"AGPL-3.0-only"}],"workspace_members":[]}
JSON
  local output
  if output=$(ASHIATO_LICENSES_CARGO_META="$cargo_meta" "$0" rust 2>&1); then
    echo "  self-test NG: 他人の AGPL を通した" >&2; return 1
  fi
  grep -q "evil-agpl" <<<"$output" || return 1
  echo "== Scenario: 自分の部品は、決めた許諾の式で許諾の検査を通る"
  cat >"$cargo_meta" <<'JSON'
{"packages":[{"id":"path+fake#ashiato-server","name":"ashiato-server","license":"AGPL-3.0-only WITH AdditionRef-ashiato-play-services"}],"workspace_members":["path+fake#ashiato-server"]}
JSON
  ASHIATO_LICENSES_CARGO_META="$cargo_meta" ASHIATO_LICENSES_NODE_ROOT="$node_root" "$0" rust node >/dev/null
  echo "  self-test: 自分の部品の名前で、決めた式でない許諾を持つものが落ちる"
  cat >"$cargo_meta" <<'JSON'
{"packages":[{"id":"path+fake#ashiato-server","name":"ashiato-server","license":"AGPL-3.0-only"}],"workspace_members":["path+fake#ashiato-server"]}
JSON
  if output=$(ASHIATO_LICENSES_CARGO_META="$cargo_meta" "$0" rust 2>&1); then
    echo "  self-test NG: 自分の部品の不正な式を通した" >&2; return 1
  fi
  grep -q "ashiato-server" <<<"$output" || return 1
  echo "== Scenario: 許可の一覧は、AGPL-3.0-only を外したほかは前と同じ"
  [[ $ALLOW == "MIT,MIT-0,Apache-2.0,BSD-2-Clause,BSD-3-Clause,ISC,Unicode-3.0,Unicode-DFS-2016,Zlib,CC0-1.0,MPL-2.0,0BSD,Apache-2.0 WITH LLVM-exception,CDLA-Permissive-2.0,BlueOak-1.0.0,CC-BY-4.0,Python-2.0" ]] || return 1
  [[ $ALLOW != *AGPL-3.0-only* ]] || return 1
  echo "== Scenario: node_modules が無ければ、画面の節は通さずに落ちる"
  if output=$(ASHIATO_LICENSES_NODE_ROOT="$test_root/missing-node-modules" "$0" node 2>&1); then
    echo "  self-test NG: 無い node_modules を通した" >&2; return 1
  fi
  grep -q "node_modules が無い" <<<"$output" || return 1
  echo "self-test OK"
}

if [[ ${1:-} == --self-test ]]; then
  self_test
  exit 0
fi

work_tmp=${TMPDIR:-}
if [[ -z $work_tmp ]]; then
  work_tmp=$(mktemp -d)
else
  work_tmp=$(mktemp -d "$work_tmp/check-licenses.XXXXXX")
fi
trap 'rm -rf "$work_tmp"' EXIT

sections=()
if (($# == 0)); then sections=(rust node android); else sections=("$@"); fi
fail=0
for section in "${sections[@]}"; do
  case $section in
    rust) run_rust "${ASHIATO_LICENSES_CARGO_META:-}" || fail=1 ;;
    node) run_node || fail=1 ;;
    android)
      echo "== Android の依存"
      echo "  NG 対象が無い（Android の節は Task 4 で実装する）"
      fail=1
      ;;
    *) echo "使い方: $0 [rust|node|android]… | --self-test" >&2; exit 2 ;;
  esac
done
if ((fail == 0)); then echo "ライセンス OK"; else echo "ライセンス NG" >&2; exit 1; fi
