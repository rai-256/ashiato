#!/usr/bin/env bash
# CH12 の ADR の形を機械で確かめる台本。
# README の一覧と docs/adr/ の対応、順序、ファイル名、本文の形、出所、
# CLAUDE.md / AGENTS.md の案内、CI の chain job、追跡済みの範囲を確かめる。
# メモリにしか無かった判断を repo の正本へ移した CH12 で、後から ADR を足しても
# 同じ形を保つために要る。引数なしは CI の chain job が走らせ、--self-test は
# 使い捨ての木の壊し方を、--verbatim は CH12 の依頼との逐語一致を、--backlog は
# 手元のメモリの残りを調べる口である。CI の chain job は
# `./tools/check-adr.sh --self-test && ./tools/check-adr.sh` を走らせる。
# --verbatim は CH12 の archive 後、ADR の手入れで落ちうる（D1）。
set -uo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
# 子を起こすときは絶対パスで呼ぶ（cd の後に相対の "$0" は見つからない。review/code.md R8）
SELF="$ROOT/tools/$(basename "$0")"
cd "$ROOT"

ADR_DIR=${ADR_DIR:-docs/adr}
CLAUDE_FILE=${ADR_CLAUDE_MD:-CLAUDE.md}
AGENTS_FILE=${ADR_AGENTS_MD:-AGENTS.md}
CI_FILE=${ADR_CI_YML:-.github/workflows/ci.yml}
# 追跡の検査が見る git の木と、網の名前の検査の例外の置き場（自己検査が使い捨ての木へ向ける）
GIT_ROOT=${ADR_GIT_ROOT:-.}
ALLOW_FILE=${ADR_PRIVATE_ALLOW:-tools/check-private.allow}
errors=0

error() {
  echo "error: $*" >&2
  errors=1
}

count_lines() {
  awk 'END { print NR + 0 }' "$1"
}

section_lines() {
  local file=$1 start=$2
  awk -v start="$start" '
    $0 ~ ("^" start) { in_section=1; next }
    in_section && /^## / { exit }
    in_section { print }
  ' "$file"
}

LIST_FILE=
FILES_FILE=
INDEX_OK=0

# README の一覧と docs/adr/ のファイル名を 1 回だけ読む。Scenario ごとの検査はこの結果を見る
load_index() {
  local readme="$ADR_DIR/README.md" line
  [ -d "$ADR_DIR" ] || { error "$ADR_DIR が無い"; return; }
  [ -f "$readme" ] || { error "$readme が無い"; return; }
  LIST_FILE=$(mktemp)
  FILES_FILE=$(mktemp)
  # 一覧の節の `- ` で始まる行は全部項目として数え、形の違う行は error で名指す（review/code.md R6）
  while IFS= read -r line; do
    if [[ "$line" =~ ^-\ \[[0-9]{4}-[0-9]{2}-[0-9]{2}\ .+\]\([^\)]+\)$ ]]; then
      printf '%s\n' "$line" >>"$LIST_FILE"
    else
      error "README の一覧の項目の形が違う: $line"
    fi
  done < <(awk '
    /^## 一覧（新しい順）$/ { in_list=1; next }
    in_list && /^## / { exit }
    in_list && /^- / { print }
  ' "$readme")
  find "$ADR_DIR" -maxdepth 1 -type f -name '*.md' ! -name README.md -printf '%f\n' | sort -r > "$FILES_FILE"
  INDEX_OK=1
}

unload_index() {
  rm -f "$LIST_FILE" "$FILES_FILE"
}

listed_names() {
  sed -E 's/^- \[[^]]+\]\(([^)]+)\)$/\1/' "$LIST_FILE"
}

check_correspondence() {
  [ "$INDEX_OK" = 1 ] || return
  local listed_count file_count
  listed_count=$(count_lines "$LIST_FILE")
  file_count=$(count_lines "$FILES_FILE")
  if [ "$listed_count" -ne "$file_count" ]; then
    error "README の一覧 ($listed_count 件) と $ADR_DIR の ADR ($file_count 件) が 1 対 1 でない"
  fi
  [ "$(listed_names)" = "$(cat "$FILES_FILE")" ] || error "README の一覧と $ADR_DIR のファイル名が一致しない"
}

check_list_titles() {
  [ "$INDEX_OK" = 1 ] || return
  local line date title name h1 adate
  while IFS= read -r line; do
    [ -n "$line" ] || continue
    date=$(printf '%s\n' "$line" | sed -E 's/^- \[([0-9-]+) .*/\1/')
    title=$(printf '%s\n' "$line" | sed -E 's/^- \[[0-9]{4}-[0-9]{2}-[0-9]{2} (.*)\]\([^)]*\)$/\1/')
    name=$(printf '%s\n' "$line" | sed -E 's/^- \[[^]]+\]\(([^)]+)\)$/\1/')
    [ -f "$ADR_DIR/$name" ] || continue
    h1=$(sed -n '1s/^# //p' "$ADR_DIR/$name")
    adate=$(sed -n 's/^- 日付: //p' "$ADR_DIR/$name")
    [ "$title" = "$h1" ] || error "$name: README の題と見出しが違う"
    [ "$date" = "$adate" ] || error "$name: README の日付と ADR の日付が違う"
  done < "$LIST_FILE"
}

check_list_order() {
  [ "$INDEX_OK" = 1 ] || return
  local expected
  expected=$(listed_names)
  [ "$expected" = "$(printf '%s\n' "$expected" | sort -r)" ] || error "README の一覧がファイル名の新しい順でない"
}

check_filenames() {
  [ "$INDEX_OK" = 1 ] || return
  local name
  while IFS= read -r name; do
    [[ "$name" =~ ^[0-9]{12}-[a-z0-9]+(-[a-z0-9]+)*\.md$ ]] || error "$name: ファイル名が時刻と英小文字 slug の形でない"
  done < "$FILES_FILE"
}

check_shape() {
  local name file count h date_line status_line source_line background_line
  while IFS= read -r name; do
    [ "$name" = README.md ] && continue
    file="$ADR_DIR/$name"
    [ -f "$file" ] || continue
    [ "$(sed -n '1{/^# /p;}' "$file" | wc -l)" -eq 1 ] || error "$name: 1 行目の題が無い"
    # 項目の行は形を問わず数え（形の違う 2 行目を見落とさない。review/code.md R6）、値の形は別に見る
    count=$(grep -c '^- 日付:' "$file" || true); [ "$count" -eq 1 ] || error "$name: 日付の行が $count 個"
    count=$(grep -c '^- 状態:' "$file" || true); [ "$count" -eq 1 ] || error "$name: 状態の行が $count 個"
    count=$(grep -c '^- 出所: ' "$file" || true); [ "$count" -eq 1 ] || error "$name: 出所の行が $count 個"
    grep -Eq '^- 日付: [0-9]{4}-[0-9]{2}-[0-9]{2}$' "$file" || error "$name: 日付の値が YYYY-MM-DD でない"
    grep -Eq '^- 状態: (採用|置き換え済み（→ [^）]+）)$' "$file" || error "$name: 状態の値が「採用」か「置き換え済み（→ …）」でない"
    local previous=0 current
    for h in '## 背景' '## 決定' '## 退けた案' '## 反転の条件' '## 影響'; do
      count=$(grep -Exc "$h" "$file" || true)
      [ "$count" -eq 1 ] || error "$name: $h の見出しが $count 個"
      current=$(grep -n -m1 -E "^$h$" "$file" | cut -d: -f1)
      if [ -n "$current" ] && [ "$current" -le "$previous" ]; then error "$name: 見出しの順序が違う"; fi
      previous=${current:-$previous}
    done
    # 日付・状態・出所の 3 行はこの順で、`## 背景` より前にある（README の「形」。review/code.md R7）
    date_line=$(grep -n -m1 '^- 日付:' "$file" | cut -d: -f1)
    status_line=$(grep -n -m1 '^- 状態:' "$file" | cut -d: -f1)
    source_line=$(grep -n -m1 '^- 出所: ' "$file" | cut -d: -f1)
    background_line=$(grep -n -m1 -Ex '## 背景' "$file" | cut -d: -f1)
    if [ -n "$date_line" ] && [ -n "$status_line" ] && [ -n "$source_line" ] && [ -n "$background_line" ]; then
      if ! [ "$date_line" -lt "$status_line" ] || ! [ "$status_line" -lt "$source_line" ] ||
        ! [ "$source_line" -lt "$background_line" ]; then
        error "$name: 日付・状態・出所の行が、この順で ## 背景 より前にない"
      fi
    fi
  done < <(find "$ADR_DIR" -maxdepth 1 -type f -name '*.md' -printf '%f\n' | sort)
}

check_claude() {
  local claude_section count
  claude_section=$(section_lines "$CLAUDE_FILE" '### 文書とハーネス' 2>/dev/null || true)
  count=$(printf '%s\n' "$claude_section" | awk -F'|' '$2 ~ /^[[:space:]]*`docs\/adr\/[[:space:]]*`[[:space:]]*$/ && $3 ~ /Story をまたぐ判断の記録/ && $4 ~ /する/ { count++ } END { print count + 0 }')
  [ "$count" -eq 1 ] || error "$CLAUDE_FILE: 文書の表から docs/adr/ に辿る行が $count 個"
}

check_agents() {
  local agents_section count
  agents_section=$(section_lines "$AGENTS_FILE" '## 実装と計画の規約' 2>/dev/null || true)
  count=$(printf '%s\n' "$agents_section" | grep -Ec '`docs/adr/`.*Story をまたぐ判断|Story をまたぐ判断.*`docs/adr/`' || true)
  [ "$count" -eq 1 ] || error "$AGENTS_FILE: Story をまたぐ判断から docs/adr/ に辿る行が $count 個"
}

check_ci() {
  local body count
  body=$(awk '
    /^  chain:/ { in_chain=1; next }
    in_chain && /^  [A-Za-z0-9_-]+:/ { exit }
    in_chain { print }
  ' "$CI_FILE" 2>/dev/null || true)
  count=$(printf '%s\n' "$body" | grep -Fxc '      - run: ./tools/check-adr.sh --self-test && ./tools/check-adr.sh' || true)
  [ "$count" -eq 1 ] || error "$CI_FILE: chain job の ADR 検査の行が $count 個"
}

TRACKED_OK=0

# CH12 で置いた 6 つが追跡されていること。後から足す ADR は数を問わない（review/code.md R1・R9）
check_tracked() {
  local files name ok=1
  local expected=(
    docs/adr/README.md
    docs/adr/202610030000-worktree-port-allocation.md
    docs/adr/202610080000-foundation-for-satellites.md
    docs/adr/202610110000-pure-crate-split.md
    docs/adr/202610110001-ui-surface-only.md
    docs/adr/202610110002-stricter-lint.md
  )
  files=$(git -C "$GIT_ROOT" ls-files docs/adr 2>/dev/null)
  [ -n "$files" ] || { error "docs/adr/ がまだ追跡されていない"; return; }
  for name in "${expected[@]}"; do
    printf '%s\n' "$files" | grep -Fxq "$name" || { error "$name が追跡されていない"; ok=0; }
  done
  TRACKED_OK=$ok
}

check_private_scope() {
  if [ ! -f "$ALLOW_FILE" ]; then
    error "$ALLOW_FILE が無い"
  elif grep -Eq '^docs/adr/' "$ALLOW_FILE"; then
    error "$ALLOW_FILE に docs/adr/ の例外がある"
  fi
  [ "$TRACKED_OK" = 1 ] || error "docs/adr/ の 6 つが追跡されていないので、網の名前の検査が docs/adr/ を見ない"
  if [ "${ADR_SELF_TEST_CHILD:-0}" != 1 ]; then
    tools/check-private.sh || errors=1
  fi
}

check_sources() {
  local name memory
  declare -A sources=(
    [202610030000-worktree-port-allocation.md]='worktree-port-allocation-followup.md prefer-collision-free-allocation-over-checks.md'
    [202610080000-foundation-for-satellites.md]='foundation-for-satellites-premise.md'
    [202610110000-pure-crate-split.md]='lint-purity-ui-refresh-plan.md'
    [202610110001-ui-surface-only.md]='lint-purity-ui-refresh-plan.md'
    [202610110002-stricter-lint.md]='lint-purity-ui-refresh-plan.md'
  )
  for name in "${!sources[@]}"; do
    while IFS= read -r memory; do
      [ -n "$memory" ] || continue
      grep -F -- "- 出所:" "$ADR_DIR/$name" | grep -Fq -- "$memory" ||
        error "$name: 出所に $memory が無い"
    done < <(printf '%s\n' ${sources[$name]})
  done
}

backlog_names=(
  worktree-port-allocation-followup.md
  prefer-collision-free-allocation-over-checks.md
  foundation-for-satellites-premise.md
  lint-purity-ui-refresh-plan.md
)

find_backlog_script() {
  if [ "${ADR_BACKLOG+x}" = x ]; then
    [ -f "$ADR_BACKLOG" ] || { echo 'error: ADR_BACKLOG の台本が無い' >&2; return 127; }
    BACKLOG_SCRIPT=$ADR_BACKLOG
    return 0
  fi

  if [ -f "$ROOT/.harness2/scripts/adr_backlog.py" ]; then
    BACKLOG_SCRIPT=$ROOT/.harness2/scripts/adr_backlog.py
    return 0
  fi

  local harness_root worktree
  harness_root=$(readlink -f "$ROOT/.harness2" 2>/dev/null || true)
  if [ -n "$harness_root" ] && [ -d "$harness_root" ]; then
    while IFS= read -r worktree; do
      [ -n "$worktree" ] || continue
      if [ -f "$worktree/scripts/adr_backlog.py" ]; then
        BACKLOG_SCRIPT=$worktree/scripts/adr_backlog.py
        return 0
      fi
    done < <(git -C "$harness_root" worktree list --porcelain 2>/dev/null | sed -n 's/^worktree //p')
  fi

  echo 'error: adr_backlog.py の台本が無い' >&2
  return 127
}

find_memory_dir() {
  if [ "${ADR_MEMORY+x}" = x ]; then
    [ -d "$ADR_MEMORY" ] || { echo 'error: ADR_MEMORY の置き場が無い' >&2; return 127; }
    MEMORY_DIR=$ADR_MEMORY
    return 0
  fi

  local main_repo repo_key
  main_repo=$(git -C "$ROOT" worktree list --porcelain 2>/dev/null | sed -n 's/^worktree //p' | sed -n '1p')
  [ -n "$main_repo" ] || { echo 'error: ADR_MEMORY の置き場が無い' >&2; return 127; }
  repo_key=$(printf '%s' "$main_repo" | LC_ALL=C sed 's/[^A-Za-z0-9]/-/g')
  MEMORY_DIR="${HOME:-}/.claude/projects/$repo_key/memory"
  [ -d "$MEMORY_DIR" ] || { echo 'error: ADR_MEMORY の置き場が無い' >&2; return 127; }
}

# rc: 0 = 読めた / 1 = 正の対照で名前が欠けた / 2 = JSON の配列でない。
# production では、4 つの名前のうち残りにあるものを 1 行ずつ標準出力に出す（それ以外は出さない）
backlog_json_check() {
  local json_file=$1 mode=$2
  shift 2
  python3 -I - "$json_file" "$mode" "$@" <<'PY' 2>/dev/null
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
mode = sys.argv[2]
names = sys.argv[3:]
try:
    value = json.loads(path.read_text(encoding="utf-8"))
except (OSError, json.JSONDecodeError):
    raise SystemExit(2)
if not isinstance(value, list) or any(not isinstance(row, dict) for row in value):
    raise SystemExit(2)
found = {row.get("memory") for row in value}
if mode == "positive":
    raise SystemExit(0 if set(names) <= found else 1)
for name in names:
    if name in found:
        print(name)
PY
}

backlog() {
  local tmp empty_dir positive_output production_output positive_rc production_rc check_rc name remaining
  tmp=$(mktemp -d)
  trap 'rm -rf "$tmp"' RETURN

  find_backlog_script || return $?
  find_memory_dir || return $?

  empty_dir=$(mktemp -d "$tmp/empty.XXXXXX")
  positive_output="$tmp/positive.json"
  python3 -I "$BACKLOG_SCRIPT" --memory "$MEMORY_DIR" --adr "$empty_dir" --json >"$positive_output" 2>"$tmp/positive.err"
  positive_rc=$?
  if [ "$positive_rc" -gt 1 ] || ! backlog_json_check "$positive_output" positive "${backlog_names[@]}" >/dev/null; then
    # 文面は D5 の `error: …無い` の形（review/code.md R4）
    echo 'error: --adr を空にした正の対照の残りに、4 つのメモリが無い' >&2
    return 1
  fi

  production_output="$tmp/production.json"
  python3 -I "$BACKLOG_SCRIPT" --memory "$MEMORY_DIR" --adr "$ADR_DIR" --json >"$production_output" 2>"$tmp/production.err"
  production_rc=$?
  if [ "$production_rc" -gt 1 ]; then
    echo 'error: adr_backlog.py の結果が無い' >&2
    return 1
  fi
  remaining="$tmp/remaining"
  backlog_json_check "$production_output" production "${backlog_names[@]}" >"$remaining"
  check_rc=$?
  [ "$check_rc" -eq 0 ] || { echo 'error: adr_backlog.py の結果に JSON の配列が無い' >&2; return 1; }

  # 名前ごとに有無を出す（1 つが残っても、ほかの 3 つを「残りにある」と言わない。review/code.md R3）
  for name in "${backlog_names[@]}"; do
    if grep -Fxq "$name" "$remaining"; then
      printf '%s: 残りにある\n' "$name"
    else
      printf '%s: 残りに無い\n' "$name"
    fi
  done
  if [ -s "$remaining" ]; then
    echo 'NG backlog'
    return 1
  fi
  echo 'OK backlog'
  return 0
}

run_main() {
  load_index
  echo 'Scenario: README の一覧と docs/adr/ の ADR のファイルが 1 対 1 で対応する'
  check_correspondence
  echo 'Scenario: 一覧の日付と題は ADR の日付と見出しに一致する'
  check_list_titles
  echo 'Scenario: 一覧はファイル名の新しい順に並ぶ'
  check_list_order
  echo 'Scenario: ADR のファイル名は時刻と英小文字の slug の形を持つ'
  check_filenames
  unload_index
  echo 'Scenario: ADR は決まった項目と見出しをこの順に持つ'
  check_shape
  echo 'Scenario: CH12 で置いた README と 5 件の ADR がある'
  check_tracked
  echo 'Scenario: CLAUDE.md の文書の表から docs/adr/ に辿れる'
  check_claude
  echo 'Scenario: AGENTS.md の規約から docs/adr/ に辿れる'
  check_agents
  echo 'Scenario: CI の chain job が ADR の形の検査を走らせる'
  check_ci
  echo 'Scenario: 5 件の ADR の出所に、移したメモリのファイル名がある'
  check_sources
  echo 'Scenario: docs/adr/ を含む追跡ファイルで、網の名前の検査が通る'
  check_private_scope
  return "$errors"
}

verbatim() {
  local source=docs/changes/CH12.md expected=7e1b0a5c755c8b19fcb7c138db6f2f87897d96baa46f90b3bc889ba872e58b94 actual tmp
  tmp=$(mktemp -d)
  trap 'rm -rf "$tmp"' RETURN
  awk '/^## 依頼（逐語）$/{on=1} on && NR>1 && /^## / && $0 != "## 依頼（逐語）"{exit} on{print}' "$source" > "$tmp/section"
  actual=$(sha256sum "$tmp/section" | awk '{print $1}')
  if [ "$actual" != "$expected" ]; then
    error "$source: 依頼（逐語）の sha256 が違う"
    return 1
  fi
  python3 -I - "$tmp/section" "$ADR_DIR" <<'PY'
import pathlib, re, sys
source = pathlib.Path(sys.argv[1]).read_text()
adr = pathlib.Path(sys.argv[2])
section = []
for line in source.splitlines():
    section.append(line[2:] if line.startswith('> ') else '' if line == '>' else line)
text = '\n'.join(section) + '\n'
blocks = re.findall(r'^=== ファイル: ([^=]+) ===\n(.*?)(?=^=== ファイル: |\Z)', text, re.M | re.S)
failed = False
if len(blocks) != 6:
    print(f'error: 依頼（逐語）のファイル塊が {len(blocks)} つで、6 つでない', file=sys.stderr)
    failed = True
names = [name.strip() for name, _ in blocks]
if len(set(names)) != len(names):
    print('error: 依頼（逐語）のファイル名に重複がある', file=sys.stderr)
    failed = True
for name, body in blocks:
    wanted = body.strip('\n') + '\n'
    path = adr / name.strip()
    if not path.is_file():
        print(f'error: {path} が無い', file=sys.stderr)
        failed = True
        continue
    # 改行は変換せずに読む（CRLF を LF と同じに見ない。review/code.md R10）
    with open(path, encoding='utf-8', newline='') as f:
        actual = f.read()
    if actual == wanted:
        continue
    expected_lines = wanted.splitlines()
    actual_lines = actual.splitlines()
    first_difference = next(
        (index for index in range(max(len(expected_lines), len(actual_lines)))
         if (expected_lines[index] if index < len(expected_lines) else None)
         != (actual_lines[index] if index < len(actual_lines) else None)),
        None,
    )
    if first_difference is None:
        # 行は同じで、末尾の改行か改行の形（CRLF）だけが違う（review/code.md R10）
        print(f'error: {path}: 末尾の改行か改行の形が依頼（逐語）と一致しない', file=sys.stderr)
        failed = True
        continue
    line_number = first_difference + 1
    expected_line = expected_lines[first_difference] if first_difference < len(expected_lines) else '<EOF>'
    actual_line = actual_lines[first_difference] if first_difference < len(actual_lines) else '<EOF>'
    print(f'error: {path}: {line_number} 行目が依頼（逐語）と一致しない', file=sys.stderr)
    print(f'  依頼: {expected_line}', file=sys.stderr)
    print(f'  ADR: {actual_line}', file=sys.stderr)
    failed = True
if failed:
    sys.exit(1)
PY
}

# 使い捨ての木の子を、その木へ向けて走らせる
run_child() {
  local dir=$1
  shift
  ADR_SELF_TEST_CHILD=1 ADR_DIR="$dir/docs/adr" ADR_CLAUDE_MD="$dir/CLAUDE.md" ADR_AGENTS_MD="$dir/AGENTS.md" \
    ADR_CI_YML="$dir/.github/workflows/ci.yml" ADR_GIT_ROOT="$dir" ADR_PRIVATE_ALLOW="$dir/tools/check-private.allow" \
    "$SELF" "$@"
}

# 壊し方を 1 つ当てる。$1 = 壊し方、$2 = 木、$3 = 対象の ADR
break_tree() {
  local case_name=$1 dir=$2 target=$3 adr upper
  adr="$dir/docs/adr"
  case "$case_name" in
    list-missing) sed -i '0,/^- \[/ { /^- \[/d; }' "$adr/README.md" ;;
    list-swapped) python3 -I - "$adr/README.md" <<'PY'
import pathlib, sys
p = pathlib.Path(sys.argv[1]); lines = p.read_text().splitlines(True)
i = next(i for i, x in enumerate(lines) if x.startswith('- ['))
j = next(j for j in range(i + 1, len(lines)) if lines[j].startswith('- ['))
lines[i], lines[j] = lines[j], lines[i]; p.write_text(''.join(lines))
PY
      ;;
    list-bad-form) sed -i '0,/^- \[/s/^- \[/- [2026-1-08 x](202611080000-x.md)\n- [/' "$adr/README.md" ;;
    title) sed -i '0,/^# /s/^# /# changed /' "$adr/$target" ;;
    h1-missing) sed -i '1s/^# //' "$adr/$target" ;;
    date) sed -i 's/^- 日付: .*/- 日付: 1999-01-01/' "$adr/$target" ;;
    date-extra) sed -i '/^- 日付: /p' "$adr/$target" ;;
    date-form) sed -i 's/^\(- 日付: .*\)$/\1 頃/' "$adr/$target" ;;
    readme-missing) rm "$adr/README.md" ;;
    adr-dir-missing) rm -rf "$adr" ;;
    source) sed -i '0,/^- 出所: / { /^- 出所: /d; }' "$adr/$target" ;;
    source-name) sed -i '/^- 出所: /s/worktree-port-allocation-followup\.md//' "$adr/$target" ;;
    source-moved) python3 -I - "$adr/$target" <<'PY'
import pathlib, sys
p = pathlib.Path(sys.argv[1]); lines = p.read_text().splitlines(True)
i = next(i for i, x in enumerate(lines) if x.startswith('- 出所: '))
moved = lines.pop(i); p.write_text(''.join(lines) + '\n' + moved)
PY
      ;;
    status-value) sed -i 's/^- 状態: 採用$/- 状態: 検討中/' "$adr/$target" ;;
    status-extra) sed -i '/^- 状態: 採用$/a - 状態: 検討中' "$adr/$target" ;;
    heading-swap) sed -i 's/^## 背景$/## TMP-SWAP/; s/^## 決定$/## 背景/; s/^## TMP-SWAP$/## 決定/' "$adr/$target" ;;
    decision) sed -i '0,/^## 決定$/ { /^## 決定$/d; }' "$adr/$target" ;;
    filename) mv "$adr/$target" "$adr/${target%.md}-BAD.md" ;;
    upper-name)
      upper=$(printf '%s' "${target#*-}" | tr 'a-z' 'A-Z')
      upper="${target%%-*}-${upper%.MD}.md"
      mv "$adr/$target" "$adr/$upper"
      sed -i "s/($target)/($upper)/" "$adr/README.md"
      ;;
    untracked) git -C "$dir" rm -q --cached "docs/adr/$target" ;;
    untracked-all) git -C "$dir" rm -rq --cached docs/adr ;;
    claude) sed -i '/`docs\/adr\/`/d' "$dir/CLAUDE.md" ;;
    agents) sed -i '/Story をまたぐ判断.*`docs\/adr\/`/d' "$dir/AGENTS.md" ;;
    ci) sed -i '/check-adr\.sh --self-test/d' "$dir/.github/workflows/ci.yml" ;;
    allow-adr) echo 'docs/adr/README.md' >>"$dir/tools/check-private.allow" ;;
    allow-missing) rm "$dir/tools/check-private.allow" ;;
    next-adr)
      cat >"$adr/209912310000-sample-next.md" <<'MD'
# 次の判断の見本

- 日付: 2099-12-31
- 状態: 採用
- 出所: 自己検査

## 背景

見本。

## 決定

見本。

## 退けた案

記録なし

## 反転の条件

記録なし

## 影響

見本。
MD
      sed -i '0,/^- \[/s/^- \[/- [2099-12-31 次の判断の見本](209912310000-sample-next.md)\n- [/' "$adr/README.md"
      git -C "$dir" add docs/adr
      ;;
  esac
}

expected_error() {
  local case_name=$1 target=$2
  case "$case_name" in
    list-missing) echo 'が 1 対 1 でない' ;;
    list-swapped) echo '一覧がファイル名の新しい順でない' ;;
    list-bad-form) echo 'README の一覧の項目の形が違う: - [2026-1-08 x](202611080000-x.md)' ;;
    title) echo "$target: README の題と見出しが違う" ;;
    h1-missing) echo "$target: 1 行目の題が無い" ;;
    date) echo "$target: README の日付と ADR の日付が違う" ;;
    date-extra) echo "$target: 日付の行が 2 個" ;;
    date-form) echo "$target: 日付の値が YYYY-MM-DD でない" ;;
    readme-missing) echo 'docs/adr/README.md が無い' ;;
    adr-dir-missing) echo 'docs/adr が無い' ;;
    source) echo "$target: 出所の行が 0 個" ;;
    source-name) echo "$target: 出所に worktree-port-allocation-followup.md が無い" ;;
    source-moved) echo "$target: 日付・状態・出所の行が、この順で ## 背景 より前にない" ;;
    status-value) echo "$target: 状態の値が「採用」か「置き換え済み（→ …）」でない" ;;
    status-extra) echo "$target: 状態の行が 2 個" ;;
    heading-swap) echo "$target: 見出しの順序が違う" ;;
    decision) echo "$target: ## 決定 の見出しが 0 個" ;;
    filename) echo 'ファイル名が一致しない' ;;
    upper-name) echo 'ファイル名が時刻と英小文字 slug の形でない' ;;
    untracked-all) echo 'docs/adr/ がまだ追跡されていない' ;;
    untracked) echo "docs/adr/$target が追跡されていない"; echo '網の名前の検査が docs/adr/ を見ない' ;;
    claude) echo 'CLAUDE.md: 文書の表から docs/adr/ に辿る行が 0 個' ;;
    agents) echo 'AGENTS.md: Story をまたぐ判断から docs/adr/ に辿る行が 0 個' ;;
    ci) echo 'ci.yml: chain job の ADR 検査の行が 0 個' ;;
    allow-adr) echo 'check-private.allow に docs/adr/ の例外がある' ;;
    allow-missing) echo 'check-private.allow が無い' ;;
  esac
}

# --backlog の子は偽の台本と使い捨てのメモリで走らせる（手元のメモリと harness2 に依らない。review/code.md R4）。
# 出してよい行は D5 の形だけ。偽の台本は秘密の印を stdout・stderr に出す
backlog_line_ok() {
  local line=$1
  local re='^(worktree-port-allocation-followup\.md|prefer-collision-free-allocation-over-checks\.md|foundation-for-satellites-premise\.md|lint-purity-ui-refresh-plan\.md):[[:space:]]残りに(無い|ある)$|^(OK|NG)[[:space:]]backlog$|^error: .*無い$'
  [[ "$line" =~ $re ]] && [[ "$line" != *SECRET* ]]
}

check_backlog_child() {
  local label=$1 want_rc=$2 out=$3 err=$4 rc=$5 line
  shift 5
  [ "$rc" -eq "$want_rc" ] || error "self-test の --backlog（$label）の rc が $want_rc でない: $rc"
  while IFS= read -r line; do
    backlog_line_ok "$line" || error "self-test の --backlog（$label）が公開してはいけない行を出した: $line"
  done < <(cat "$out" "$err")
  for line in "$@"; do
    # pipe に grep -q を繋ぐと、pipefail の下で cat の SIGPIPE が rc≠0 になる。ファイルを直に渡す
    grep -Fxqs -- "$line" "$out" "$err" ||
      error "self-test の --backlog（$label）が期待する行を出さなかった: $line"
  done
}

self_test() {
  local base tmp case_rc expected_message backlog_test fake_backlog output target case_dir case_name memory rc name line
  [ -d "$ADR_DIR" ] || { error "self-test の基準となる $ADR_DIR が無い"; return 1; }
  tmp=$(mktemp -d)
  trap 'rm -rf "$tmp"' RETURN
  base="$tmp/base"
  mkdir -p "$base/docs/adr" "$base/.github/workflows" "$base/tools"
  cp -a "$ADR_DIR/." "$base/docs/adr/"
  cp "$CLAUDE_FILE" "$base/CLAUDE.md"
  cp "$AGENTS_FILE" "$base/AGENTS.md"
  cp "$CI_FILE" "$base/.github/workflows/ci.yml"
  cp "$ALLOW_FILE" "$base/tools/check-private.allow"
  git -C "$base" init -q && git -C "$base" add docs/adr ||
    { error 'self-test の使い捨ての木を git に載せられない'; return 1; }
  if ! run_child "$base" >"$tmp/base-output" 2>&1; then
    error "self-test の基準の木で本番検査が通らない"
    sed -n '/error:/p' "$tmp/base-output" >&2
  fi
  target=$(find "$base/docs/adr" -maxdepth 1 -type f -name '20*.md' -printf '%f\n' | sort | head -n1)

  for case_name in list-missing list-swapped list-bad-form title h1-missing date date-extra date-form source source-name source-moved \
    status-value status-extra heading-swap decision filename upper-name untracked untracked-all claude agents ci allow-adr allow-missing \
    readme-missing adr-dir-missing; do
    case_dir="$tmp/$case_name"
    cp -a "$base" "$case_dir"
    case "$case_name" in
      source) echo 'Scenario: 出所の行が無い ADR は落ちる' ;;
      decision) echo 'Scenario: 決まった見出しの欠けた ADR は落ちる' ;;
    esac
    break_tree "$case_name" "$case_dir" "$target"
    expected_message=$(expected_error "$case_name" "$target")
    output="$case_dir/output"
    if run_child "$case_dir" >"$output" 2>&1; then
      error "self-test の壊し方 $case_name で検査が落ちなかった"
    fi
    while IFS= read -r line; do
      grep -Fq -- "$line" "$output" || error "self-test の壊し方 $case_name が期待する error を出さなかった: $line"
    done <<<"$expected_message"
  done

  # 正の対照: 形の正しい ADR を 1 件足して追跡しても通る（review/code.md R1）
  case_dir="$tmp/next-adr"
  cp -a "$base" "$case_dir"
  break_tree next-adr "$case_dir" "$target"
  if ! run_child "$case_dir" >"$case_dir/output" 2>&1; then
    error 'self-test の形の正しい ADR を 1 件足した木で検査が落ちた'
    sed -n '/error:/p' "$case_dir/output" >&2
  fi

  backlog_test="$tmp/backlog"
  mkdir -p "$backlog_test"
  if ADR_BACKLOG="$backlog_test/missing.py" "$SELF" --backlog >"$tmp/backlog-missing.out" 2>"$tmp/backlog-missing.err"; then
    error 'self-test の ADR_BACKLOG 欠落を通した'
  else
    case_rc=$?
    [ "$case_rc" -eq 127 ] || error "self-test の ADR_BACKLOG 欠落が rc=127 でない: $case_rc"
    grep -Fq 'error: ADR_BACKLOG の台本が無い' "$tmp/backlog-missing.err" ||
      error 'self-test の ADR_BACKLOG 欠落が台本の error を出さない'
  fi

  fake_backlog="$backlog_test/fake.py"
  cat >"$fake_backlog" <<'PY'
#!/usr/bin/env python3
# adr_backlog.py の偽物: --adr のどの ADR にも名指されないメモリを残りとして JSON で出す。秘密の印も出す
import argparse, json, pathlib, sys
p = argparse.ArgumentParser()
p.add_argument('--memory'); p.add_argument('--adr'); p.add_argument('--json', action='store_true')
a = p.parse_args()
text = ''.join(f.read_text() for f in pathlib.Path(a.adr).glob('*.md'))
rows = [{'memory': m.name, 'description': 'SECRET-description'}
        for m in sorted(pathlib.Path(a.memory).glob('*.md')) if m.name != 'MEMORY.md' and m.name not in text]
print('SECRET-stderr', file=sys.stderr)
print(json.dumps(rows))
sys.exit(1 if rows else 0)
PY
  if ADR_BACKLOG="$fake_backlog" ADR_MEMORY="$backlog_test/missing-memory" "$SELF" --backlog >"$tmp/memory-missing.out" 2>"$tmp/memory-missing.err"; then
    error 'self-test の ADR_MEMORY 欠落を通した'
  else
    case_rc=$?
    [ "$case_rc" -eq 127 ] || error "self-test の ADR_MEMORY 欠落が rc=127 でない: $case_rc"
    grep -Fq 'error: ADR_MEMORY の置き場が無い' "$tmp/memory-missing.err" ||
      error 'self-test の ADR_MEMORY 欠落が置き場の error を出さない'
  fi

  memory="$backlog_test/memory"
  mkdir -p "$memory" "$backlog_test/empty-memory"
  for name in "${backlog_names[@]}" secret-other-memory.md; do
    printf -- '---\nname: x\n---\nSECRET-body\n' >"$memory/$name"
  done

  # 4 つとも ADR に名指されている → 4 行とも「残りに無い」と OK（ほかのメモリが残っていても）
  ADR_BACKLOG="$fake_backlog" ADR_MEMORY="$memory" ADR_DIR="$base/docs/adr" "$SELF" --backlog \
    >"$tmp/bl-ok.out" 2>"$tmp/bl-ok.err"
  rc=$?
  check_backlog_child '全部移した' 0 "$tmp/bl-ok.out" "$tmp/bl-ok.err" "$rc" \
    'foundation-for-satellites-premise.md: 残りに無い' 'lint-purity-ui-refresh-plan.md: 残りに無い' 'OK backlog'

  # 1 つだけ名指しを外す → その 1 つだけ「残りにある」と NG（review/code.md R3）
  case_dir="$tmp/bl-one"
  cp -a "$base" "$case_dir"
  sed -i 's/foundation-for-satellites-premise\.md//g' "$case_dir"/docs/adr/*.md
  ADR_BACKLOG="$fake_backlog" ADR_MEMORY="$memory" ADR_DIR="$case_dir/docs/adr" "$SELF" --backlog \
    >"$tmp/bl-one.out" 2>"$tmp/bl-one.err"
  rc=$?
  check_backlog_child '1 つ残る' 1 "$tmp/bl-one.out" "$tmp/bl-one.err" "$rc" \
    'foundation-for-satellites-premise.md: 残りにある' 'lint-purity-ui-refresh-plan.md: 残りに無い' \
    'worktree-port-allocation-followup.md: 残りに無い' 'NG backlog'

  # メモリを消した後（空の置き場）→ 正の対照で落ち、文面は D5 の形
  ADR_BACKLOG="$fake_backlog" ADR_MEMORY="$backlog_test/empty-memory" ADR_DIR="$base/docs/adr" "$SELF" --backlog \
    >"$tmp/bl-empty.out" 2>"$tmp/bl-empty.err"
  rc=$?
  check_backlog_child '空のメモリ' 1 "$tmp/bl-empty.out" "$tmp/bl-empty.err" "$rc" \
    'error: --adr を空にした正の対照の残りに、4 つのメモリが無い'

  return "$errors"
}

case "${1:-}" in
  '') run_main ;;
  --self-test) self_test ;;
  --verbatim) verbatim ;;
  --backlog) backlog ;;
  *) echo "error: unknown option: $1" >&2; exit 2 ;;
esac
