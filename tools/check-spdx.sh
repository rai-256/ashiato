#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
expected_agpl_sha256="0d96a4ff68ad6d4b6f1f30f713b18d5184912ba8dd389f86aa7710db079abcb0"
expected_text="Additional permission under GNU AGPL version 3 section 7

If you modify this Program, or any covered work, by linking or combining it with the Google Play services libraries listed below (any version of them, but not a modified version), containing parts covered by the terms of the Android Software Development Kit License, the licensors of this Program grant you additional permission to convey the resulting work. Corresponding Source for a non-source form of such a combination shall not include the source code for the parts of those libraries used."

fail() { echo "NG: $*" >&2; exit 1; }

expected_header="AGPL-3.0-only WITH AdditionRef-ashiato-play-services"
marker='SPDX-License-'"Identifier:"
check_headers() {
  local tree="${1:-$root}" bad=0 meta mode path value
  # 空白を含むパスを切らないよう、NUL 区切りで読み、パスはタブの後ろを丸ごと取る
  while IFS= read -r -d '' meta; do
    mode="${meta%% *}"
    path="${meta#*$'\t'}"
    [[ "$mode" == 120000 ]] && continue
    if [[ ! -r "$tree/$path" ]]; then
      echo "NG: $path を読めない" >&2
      bad=1
      continue
    fi
    value="$(head -n 25 "$tree/$path" | LC_ALL=C awk -v marker="$marker" '
      index($0, marker) {
        prefix = substr($0, 1, index($0, marker) - 1)
        if (prefix !~ /^[[:space:]]*(\/\/!?|#|--|<!--|@rem|\/\*|\*)?[[:space:]]*$/) next
        value = substr($0, index($0, marker) + length(marker) + 1)
        sub(/[[:space:]]*(-->|\*\/)[[:space:]]*$/, "", value)
        print value
        exit
      }')"
    [[ -n "$value" ]] || continue
    case "$path" in
      collector-android/gradlew|collector-android/gradlew.bat)
        allowed=Apache-2.0 ;;
      migrations/202609211400_browser_history_record_id.sql|\
      migrations/202609211400_browser_history_record_id.down.sql|\
      migrations/202610181200_media_capture.sql)
        allowed=AGPL-3.0-only ;;
      *)
        allowed="$expected_header" ;;
    esac
    if [[ "$value" != "$allowed" ]]; then
      echo "NG: $path $value (許す値: $allowed)" >&2
      bad=1
    elif [[ "$allowed" != "$expected_header" ]]; then
      echo "  例外: $path $value"
    fi
  done < <(git -C "$tree" ls-files -s -z)
  return "$bad"
}

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
  sed -i 's/grant you additional permission/grant you an additional permission/g' "$tree/LICENSES/AdditionRef-ashiato-play-services.txt" "$tree/LICENSE"
  if output="$(check_license "$tree" 2>&1)"; then return 1; fi
  grep -qF '追加の許可の文が仕様と違う' <<<"$output"
  cp "$root/LICENSES/AdditionRef-ashiato-play-services.txt" "$tree/LICENSES/AdditionRef-ashiato-play-services.txt"
  cp "$root/LICENSE" "$tree/LICENSE"
  echo "== Scenario: LICENSE の先頭は追加の許可の正本と同じ"
  check_license "$tree"

  echo "== Scenario: LICENSE の許可の文を正本と変えると、許諾の文の検査が落ちる"
  sed -i 's/but not a modified version//' "$tree/LICENSE"
  if output="$(check_license "$tree" 2>&1)"; then return 1; fi
  grep -qF 'LICENSE と正本が違う' <<<"$output"
  cp "$root/LICENSE" "$tree/LICENSE"

  echo "== Scenario: LICENSE の AGPL の全文を変えると、許諾の文の検査が落ちる"
  sed -i '$s/$/ changed/' "$tree/LICENSE"
  if output="$(check_license "$tree" 2>&1)"; then return 1; fi
  grep -qF 'AGPL の全文が違う' <<<"$output"
  cp "$root/LICENSE" "$tree/LICENSE"

  echo "== Scenario: README のライセンスの節は、追加の許可と正本の言葉を示す"
  sed -i '/参考訳/d' "$tree/README.md"
  if output="$(check_license "$tree" 2>&1)"; then return 1; fi
  grep -qF 'README に参考訳の記載が無い' <<<"$output"
  cp "$root/README.md" "$tree/README.md"

  echo "== Scenario: 許可の文の名指しは、位置のライブラリとそれが引く 3 つだけ"
  check_license "$tree"

  echo "== Scenario: 決めた許諾の表示でないソースがあれば、表示の検査が落ちる"
  git -C "$tree" init -q
  git -C "$tree" config user.email test@example.invalid
  git -C "$tree" config user.name test
  printf '%s\n' '// SPDX-License-Identifier: GPL-3.0-only' > "$tree/bad.rs"
  git -C "$tree" add bad.rs
  if output="$(check_headers "$tree" 2>&1)"; then return 1; fi
  grep -qF 'bad.rs' <<<"$output"
  git -C "$tree" rm -qf bad.rs

  printf '%s\n' '/* SPDX-License-Identifier: GPL-3.0-only */' > "$tree/block.rs"
  git -C "$tree" add block.rs
  if output="$(check_headers "$tree" 2>&1)"; then return 1; fi
  grep -qF 'block.rs' <<<"$output"
  git -C "$tree" rm -qf block.rs

  echo "== Scenario: 表示の検査は symlink の先を読まない"
  outside="$work/outside.rs"
  printf '%s\n' '// SPDX-License-Identifier: GPL-3.0-only' > "$outside"
  ln -s "$outside" "$tree/external.rs"
  git -C "$tree" add external.rs
  check_headers "$tree"

  echo "== Scenario: 例外の一覧のファイルは、元の表示のままで表示の検査を通る"
  for file in collector-android/gradlew collector-android/gradlew.bat \
    migrations/202609211400_browser_history_record_id.sql \
    migrations/202609211400_browser_history_record_id.down.sql \
    migrations/202610181200_media_capture.sql; do
    mkdir -p "$tree/$(dirname "$file")"
    value=AGPL-3.0-only
    [[ "$file" == collector-android/* ]] && value=Apache-2.0
    printf '%s\n' "// SPDX-License-Identifier: $value" > "$tree/$file"
    git -C "$tree" add "$file"
  done
  output="$(check_headers "$tree")"
  [[ "$(grep -c '^  例外:' <<<"$output")" -eq 5 ]]

  echo "== Scenario: 空白を含むパスのファイルも、表示の検査が読む"
  printf '%s\n' '// SPDX-License-Identifier: GPL-3.0-only' > "$tree/with space.rs"
  git -C "$tree" add "with space.rs"
  if output="$(check_headers "$tree" 2>&1)"; then return 1; fi
  grep -qF 'NG: with space.rs GPL-3.0-only' <<<"$output"
  printf '%s\n' "// SPDX-License-Identifier: $expected_header" > "$tree/with space.rs"
  git -C "$tree" add "with space.rs"
  # 窓は先頭 25 行（D5・Q11）。26 行目の表示は見ない（窓を広げると落ちる。code_verify R18）
  { for _ in $(seq 25); do printf '%s\n' '// filler'; done; printf '%s\n' '// SPDX-License-Identifier: GPL-3.0-only'; } > "$tree/line26.rs"
  git -C "$tree" add line26.rs
  check_headers "$tree" >/dev/null || { echo '26 行目の表示を見た（窓が先頭 25 行でない）' >&2; return 1; }
  git -C "$tree" rm -qf line26.rs

  echo "== Scenario: 今の木は、表示の検査を rc=0 で通る"
  # $(…) の中では errexit が引き継がれず失敗を飲み込むので、別のプロセスで走らせて rc を見る
  if ! "${BASH_SOURCE[0]}" headers >"$work/root.out" 2>"$work/root.err"; then
    cat "$work/root.err" >&2
    echo "NG: 今の木で表示の検査が落ちる" >&2
    return 1
  fi
  output="$(cat "$work/root.out")"
  [[ "$(grep -c '^  例外:' <<<"$output")" -eq 5 ]]
  grep -qFx '  例外: collector-android/gradlew Apache-2.0' <<<"$output"
  grep -qFx '  例外: collector-android/gradlew.bat Apache-2.0' <<<"$output"
  grep -qFx '  例外: migrations/202609211400_browser_history_record_id.sql AGPL-3.0-only' <<<"$output"
  grep -qFx '  例外: migrations/202609211400_browser_history_record_id.down.sql AGPL-3.0-only' <<<"$output"
  grep -qFx '  例外: migrations/202610181200_media_capture.sql AGPL-3.0-only' <<<"$output"

  echo "== Scenario: 例外の一覧のファイルでも、許す値と違う表示なら表示の検査が落ちる"
  sed -i 's/AGPL-3.0-only/GPL-3.0-only/' "$tree/migrations/202610181200_media_capture.sql"
  if output="$(check_headers "$tree" 2>&1)"; then return 1; fi
  grep -qF 'migrations/202610181200_media_capture.sql' <<<"$output"

  echo "== Scenario: API の契約の写しは、部品と同じ許諾の式を持つ"
  actual_identifier="$(python3 -c 'import json, sys; print(json.load(open(sys.argv[1]))["info"]["license"]["identifier"])' "$root/docs/openapi.json")"
  [[ "$actual_identifier" == "$expected_header" ]]
}

case "${1:-all}" in
  all) check_license "$root"; check_headers "$root" ;;
  license) check_license "$root" ;;
  headers) check_headers "$root" ;;
  --self-test) self_test ;;
  *) echo "使い方: $0 [license|headers|--self-test]" >&2; exit 2 ;;
esac
