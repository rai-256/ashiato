#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
expected_agpl_sha256="0d96a4ff68ad6d4b6f1f30f713b18d5184912ba8dd389f86aa7710db079abcb0"
expected_text="Additional permission under GNU AGPL version 3 section 7

If you modify this Program, or any covered work, by linking or combining it with the Google Play services libraries listed below (any version of them, but not a modified version), containing parts covered by the terms of the Android Software Development Kit License, the licensors of this Program grant you additional permission to convey the resulting work. Corresponding Source for a non-source form of such a combination shall not include the source code for the parts of those libraries used."

fail() { echo "NG: $*" >&2; exit 1; }

check_license() {
  local tree="$1" ref license
  ref="$tree/LICENSES/AdditionRef-ashiato-play-services.txt"
  license="$tree/LICENSE"
  [[ -f "$ref" ]] || fail "追加の許可の正本が無い"
  [[ -f "$license" ]] || fail "LICENSE が無い"

  local n
  n="$(wc -l < "$ref")"
  if ! head -n "$n" "$license" | cmp -s - "$ref"; then
    fail "LICENSE と正本が違う"
  fi
  if ! sed -n '1,/^Listed libraries:$/p' "$ref" | sed '$d' | cmp -s - <(printf '%s\n\n' "$expected_text"); then
    fail "追加の許可の文が仕様と違う"
  fi

  local actual
  actual="$(tail -n +$((n + 3)) "$license" | sha256sum | awk '{print $1}')"
  [[ "$actual" == "$expected_agpl_sha256" ]] || fail "AGPL の全文が違う (sha256: $actual)"

  local listed
  listed="$(sed -n '/^Listed libraries:$/,/^$/p' "$ref" | sed '1d;/^$/d')"
  [[ "$(printf '%s\n' "$listed" | wc -l)" -eq 4 ]] || fail "名指しのライブラリ数が違う"
  grep -qFx -- '- com.google.android.gms:play-services-location' <<<"$listed" || fail "location が無い"
  grep -qFx -- '- com.google.android.gms:play-services-base' <<<"$listed" || fail "base が無い"
  grep -qFx -- '- com.google.android.gms:play-services-basement' <<<"$listed" || fail "basement が無い"
  grep -qFx -- '- com.google.android.gms:play-services-tasks' <<<"$listed" || fail "tasks が無い"
  grep -qF 'play-services-ads' <<<"$listed" && fail "ads が名指しされている"
  grep -qF 'play-services-measurement' <<<"$listed" && fail "measurement が名指しされている"

  local readme="$tree/README.md"
  grep -q 'AdditionRef-ashiato-play-services' "$readme" || fail "README に正本名が無い"
  grep -q '参考訳' "$readme" || fail "README に参考訳の記載が無い"
  grep -q '正本は英文' "$readme" || fail "README に正本が英文である記載が無い"
}

self_test() {
  local work tree
  work="$(mktemp -d "${TMPDIR:-/tmp}/check-spdx.XXXXXX")"
  trap 'rm -rf "$work"' RETURN
  tree="$work/tree"
  mkdir -p "$tree/LICENSES"
  cp "$root/LICENSES/AdditionRef-ashiato-play-services.txt" "$tree/LICENSES/"
  cp "$root/LICENSE" "$tree/LICENSE"
  cp "$root/README.md" "$tree/README.md"

  echo "== Scenario: 追加の許可の正本は、深掘りで決めた英文と同じ"
  check_license "$tree"
  echo "== Scenario: LICENSE の先頭は追加の許可の正本と同じ"
  check_license "$tree"

  echo "== Scenario: LICENSE の許可の文を正本と変えると、許諾の文の検査が落ちる"
  sed -i 's/but not a modified version//' "$tree/LICENSE"
  if (check_license "$tree") >/dev/null 2>&1; then return 1; fi
  cp "$root/LICENSE" "$tree/LICENSE"

  echo "== Scenario: LICENSE の AGPL の全文を変えると、許諾の文の検査が落ちる"
  sed -i '$s/$/ changed/' "$tree/LICENSE"
  if (check_license "$tree") >/dev/null 2>&1; then return 1; fi
  cp "$root/LICENSE" "$tree/LICENSE"

  echo "== Scenario: README のライセンスの節は、追加の許可と正本の言葉を示す"
  sed -i '/参考訳/d' "$tree/README.md"
  if (check_license "$tree") >/dev/null 2>&1; then return 1; fi
  cp "$root/README.md" "$tree/README.md"

  echo "== Scenario: 許可の文の名指しは、位置のライブラリとそれが引く 3 つだけ"
  check_license "$tree"
}

case "${1:-license}" in
  license) check_license "$root" ;;
  --self-test) self_test ;;
  *) echo "使い方: $0 license|--self-test" >&2; exit 2 ;;
esac
