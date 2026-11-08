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
cd "$ROOT"

ADR_DIR=${ADR_DIR:-docs/adr}
CLAUDE_FILE=${ADR_CLAUDE_MD:-CLAUDE.md}
AGENTS_FILE=${ADR_AGENTS_MD:-AGENTS.md}
CI_FILE=${ADR_CI_YML:-.github/workflows/ci.yml}
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

check_index_and_files() {
  local readme="$ADR_DIR/README.md"
  [ -f "$readme" ] || { error "$readme が無い"; return; }
  [ -d "$ADR_DIR" ] || { error "$ADR_DIR が無い"; return; }

  local list_file files_file
  list_file=$(mktemp)
  files_file=$(mktemp)
  trap 'rm -f "$list_file" "$files_file"' RETURN
  awk '
    /^## 一覧（新しい順）$/ { in_list=1; next }
    in_list && /^## / { exit }
    in_list && /^- \[[0-9]{4}-[0-9]{2}-[0-9]{2} / { print }
  ' "$readme" | sed -E 's/^-[[:space:]]+\[[^]]+\]\([^)]*\)$/&/' > "$list_file"
  find "$ADR_DIR" -maxdepth 1 -type f -name '*.md' ! -name README.md -printf '%f\n' | sort -r > "$files_file"

  local listed_count file_count
  listed_count=$(count_lines "$list_file")
  file_count=$(count_lines "$files_file")
  if [ "$listed_count" -ne "$file_count" ]; then
    error "README の一覧 ($listed_count 件) と $ADR_DIR の ADR ($file_count 件) が 1 対 1 でない"
  fi

  local expected actual name title date line
  expected=$(sed -E 's/^- \[[^]]+\]\(([^)]+)\)$/\1/' "$list_file")
  actual=$(cat "$files_file")
  if [ "$expected" != "$actual" ]; then
    error "README の一覧と $ADR_DIR のファイル名が一致しない"
  fi

  while IFS= read -r line; do
    [ -n "$line" ] || continue
    date=$(printf '%s\n' "$line" | sed -E 's/^- \[([0-9-]+) .*/\1/')
    title=$(printf '%s\n' "$line" | sed -E 's/^- \[[0-9]{4}-[0-9]{2}-[0-9]{2} (.*)\]\([^)]*\)$/\1/')
    name=$(printf '%s\n' "$line" | sed -E 's/^- \[[^]]+\]\(([^)]+)\)$/\1/')
    [ -f "$ADR_DIR/$name" ] || continue
    local h1 adate
    h1=$(sed -n '1s/^# //p' "$ADR_DIR/$name")
    adate=$(sed -n 's/^- 日付: //p' "$ADR_DIR/$name")
    [ "$title" = "$h1" ] || error "$name: README の題と見出しが違う"
    [ "$date" = "$adate" ] || error "$name: README の日付と ADR の日付が違う"
  done < "$list_file"

  local sorted
  sorted=$(printf '%s\n' "$expected" | sort -r)
  [ "$expected" = "$sorted" ] || error "README の一覧がファイル名の新しい順でない"

  while IFS= read -r name; do
    [[ "$name" =~ ^[0-9]{12}-[a-z0-9]+(-[a-z0-9]+)*\.md$ ]] || error "$name: ファイル名が時刻と英小文字 slug の形でない"
  done < "$files_file"
  rm -f "$list_file" "$files_file"
  trap - RETURN
}

check_shape() {
  local name file count h
  while IFS= read -r name; do
    [ "$name" = README.md ] && continue
    file="$ADR_DIR/$name"
    [ -f "$file" ] || continue
    [ "$(sed -n '1{/^# /p;}' "$file" | wc -l)" -eq 1 ] || error "$name: 1 行目の題が無い"
    count=$(grep -Ec '^- 日付: [0-9]{4}-[0-9]{2}-[0-9]{2}$' "$file" || true); [ "$count" -eq 1 ] || error "$name: 日付の行が $count 個"
    count=$(grep -Ec '^- 状態: (採用|置き換え済み（→ [^）]+）)$' "$file" || true); [ "$count" -eq 1 ] || error "$name: 状態の行が $count 個"
    count=$(grep -Ec '^- 出所: ' "$file" || true); [ "$count" -eq 1 ] || error "$name: 出所の行が $count 個"
    local previous=0 index=0
    for h in '## 背景' '## 決定' '## 退けた案' '## 反転の条件' '## 影響'; do
      count=$(grep -Exc "$h" "$file" || true)
      [ "$count" -eq 1 ] || error "$name: $h の見出しが $count 個"
      index=$((index + 1))
      local current
      current=$(grep -n -m1 -E "^$h$" "$file" | cut -d: -f1)
      if [ -n "$current" ] && [ "$current" -le "$previous" ]; then error "$name: 見出しの順序が違う"; fi
      previous=${current:-$previous}
    done
  done < <(find "$ADR_DIR" -maxdepth 1 -type f -name '*.md' -printf '%f\n' | sort)
}

check_guidance() {
  local claude_section agents_section count
  claude_section=$(section_lines "$CLAUDE_FILE" '### 文書とハーネス' 2>/dev/null || true)
  count=$(printf '%s\n' "$claude_section" | awk -F'|' '$2 ~ /^[[:space:]]*`docs\/adr\/[[:space:]]*`[[:space:]]*$/ && $3 ~ /Story をまたぐ判断の記録/ && $4 ~ /する/ { count++ } END { print count + 0 }')
  [ "$count" -eq 1 ] || error "$CLAUDE_FILE: 文書の表から docs/adr/ に辿る行が $count 個"

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

check_tracked() {
  local files count expected name
  expected=(
    docs/adr/README.md
    docs/adr/202610030000-worktree-port-allocation.md
    docs/adr/202610080000-foundation-for-satellites.md
    docs/adr/202610110000-pure-crate-split.md
    docs/adr/202610110001-ui-surface-only.md
    docs/adr/202610110002-stricter-lint.md
  )
  files=$(git ls-files docs/adr)
  count=$(printf '%s\n' "$files" | sed '/^$/d' | wc -l)
  [ "$count" -eq 6 ] || { error "docs/adr/ がまだ追跡されていない"; return; }
  for name in "${expected[@]}"; do
    printf '%s\n' "$files" | grep -Fxq "$name" || error "$name が追跡されていない"
  done
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

backlog_json_check() {
  local json_file=$1 mode=$2
  shift 2
  python3 -I - "$json_file" "$mode" "$@" <<'PY' >/dev/null 2>&1
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
mode = sys.argv[2]
names = set(sys.argv[3:])
try:
    value = json.loads(path.read_text(encoding="utf-8"))
except (OSError, json.JSONDecodeError):
    raise SystemExit(2)
if not isinstance(value, list) or any(not isinstance(row, dict) for row in value):
    raise SystemExit(2)
found = {row.get("memory") for row in value}
if mode == "positive":
    raise SystemExit(0 if names <= found else 1)
raise SystemExit(1 if found & names else 0)
PY
}

backlog() {
  local tmp empty_dir positive_output production_output positive_rc production_rc check_rc name
  local backlog_has_target=0
  tmp=$(mktemp -d)
  trap 'rm -rf "$tmp"' RETURN

  find_backlog_script || return $?
  find_memory_dir || return $?

  empty_dir=$(mktemp -d "$tmp/empty.XXXXXX")
  positive_output="$tmp/positive.json"
  python3 -I "$BACKLOG_SCRIPT" --memory "$MEMORY_DIR" --adr "$empty_dir" --json >"$positive_output" 2>"$tmp/positive.err"
  positive_rc=$?
  if [ "$positive_rc" -gt 1 ] || ! backlog_json_check "$positive_output" positive "${backlog_names[@]}"; then
    echo 'error: --adr の空の正の対照で 4 つのメモリを数えられない' >&2
    return 1
  fi

  production_output="$tmp/production.json"
  python3 -I "$BACKLOG_SCRIPT" --memory "$MEMORY_DIR" --adr "$ADR_DIR" --json >"$production_output" 2>"$tmp/production.err"
  production_rc=$?
  if [ "$production_rc" -gt 1 ]; then
    echo 'error: adr_backlog.py の結果が読めない' >&2
    return 1
  fi
  backlog_json_check "$production_output" production "${backlog_names[@]}"
  check_rc=$?
  [ "$check_rc" -ne 2 ] || { echo 'error: adr_backlog.py の結果が JSON の配列でない' >&2; return 1; }
  backlog_has_target=$check_rc

  for name in "${backlog_names[@]}"; do
    if [ "$backlog_has_target" -eq 0 ]; then
      printf '%s: 残りに無い\n' "$name"
    else
      printf '%s: 残りにある\n' "$name"
    fi
  done
  if [ "$backlog_has_target" -eq 0 ]; then
    echo 'OK backlog'
    return 0
  fi
  echo 'NG backlog'
  return 1
}

run_main() {
  echo 'Scenario: README の一覧と docs/adr/ の ADR のファイルが 1 対 1 で対応する'
  check_index_and_files
  echo 'Scenario: 一覧の日付と題は ADR の日付と見出しに一致する'
  check_index_and_files
  echo 'Scenario: 一覧はファイル名の新しい順に並ぶ'
  check_index_and_files
  echo 'Scenario: ADR のファイル名は時刻と英小文字の slug の形を持つ'
  check_index_and_files
  echo 'Scenario: ADR は決まった項目と見出しをこの順に持つ'
  check_shape
  echo 'Scenario: CH12 で置いた README と 5 件の ADR がある'
  check_tracked
  echo 'Scenario: CLAUDE.md の文書の表から docs/adr/ に辿れる'
  check_guidance
  echo 'Scenario: AGENTS.md の規約から docs/adr/ に辿れる'
  check_guidance
  echo 'Scenario: CI の chain job が ADR の形の検査を走らせる'
  check_ci
  echo 'Scenario: 5 件の ADR の出所に、移したメモリのファイル名がある'
  check_sources
  echo 'Scenario: docs/adr/ を含む追跡ファイルで、網の名前の検査が通る'
  check_tracked
  if grep -Eq '^docs/adr/' tools/check-private.allow; then
    error 'tools/check-private.allow に docs/adr/ の例外がある'
  fi
  if [ "${ADR_SELF_TEST_CHILD:-0}" != 1 ]; then
    tools/check-private.sh || errors=1
  fi
  trap - RETURN
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
  python3 - "$tmp/section" "$ADR_DIR" <<'PY'
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
    actual = path.read_text()
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

self_test() {
  local base tmp case_rc expected_message backlog_test fake_backlog output line
  [ -d "$ADR_DIR" ] || { error "self-test の基準となる $ADR_DIR が無い"; return 1; }
  tmp=$(mktemp -d)
  trap 'rm -rf "$tmp"' RETURN
  base="$tmp/base"
  mkdir -p "$base/docs/adr" "$base/.github/workflows"
  cp -a "$ADR_DIR/." "$base/docs/adr/"
  cp "$CLAUDE_FILE" "$base/CLAUDE.md"
  cp "$AGENTS_FILE" "$base/AGENTS.md"
  cp "$CI_FILE" "$base/.github/workflows/ci.yml"
  base_output="$tmp/base-output"
  if ! ADR_SELF_TEST_CHILD=1 ADR_DIR="$base/docs/adr" ADR_CLAUDE_MD="$base/CLAUDE.md" ADR_AGENTS_MD="$base/AGENTS.md" ADR_CI_YML="$base/.github/workflows/ci.yml" "$0" >"$base_output" 2>&1; then
    error "self-test の基準の木で本番検査が通らない"
    sed -n '/error:/p' "$base_output" >&2
  fi
  for case_name in list-missing list-swapped title source filename claude decision agents ci; do
    case_dir="$tmp/$case_name"
    cp -a "$base" "$case_dir"
    target=$(find "$case_dir/docs/adr" -maxdepth 1 -type f -name '20*.md' -printf '%f\n' | sort | head -n1)
    case "$case_name" in
      list-missing) sed -i '0,/^- \[/ { /^- \[/d; }' "$case_dir/docs/adr/README.md" ;;
      list-swapped) python3 - "$case_dir/docs/adr/README.md" <<'PY'
import pathlib
p = pathlib.Path(__import__('sys').argv[1]); lines = p.read_text().splitlines(True)
i = next(i for i, x in enumerate(lines) if x.startswith('- [')); j = next(j for j in range(i+1, len(lines)) if lines[j].startswith('- ['))
lines[i], lines[j] = lines[j], lines[i]; p.write_text(''.join(lines))
PY
        ;;
      title) sed -i '0,/^# /s/^# /# changed /' "$case_dir/docs/adr/$target" ;;
      source) echo 'Scenario: 出所の行が無い ADR は落ちる'; sed -i '0,/^- 出所: / { /^- 出所: /d; }' "$case_dir/docs/adr/$target" ;;
      filename) mv "$case_dir/docs/adr/$target" "$case_dir/docs/adr/${target%.md}-BAD.md" ;;
      claude) sed -i '/`docs\/adr\/`/d' "$case_dir/CLAUDE.md" ;;
      decision) echo 'Scenario: 決まった見出しの欠けた ADR は落ちる'; sed -i '0,/^## 決定$/ { /^## 決定$/d; }' "$case_dir/docs/adr/$target" ;;
      agents) sed -i '/Story をまたぐ判断.*`docs\/adr\/`/d' "$case_dir/AGENTS.md" ;;
      ci) sed -i '/check-adr\.sh --self-test/d' "$case_dir/.github/workflows/ci.yml" ;;
    esac
    case "$case_name" in
      list-missing) expected_message='README の一覧' ;;
      list-swapped) expected_message='一覧がファイル名の新しい順でない' ;;
      title) expected_message='README の題と見出しが違う' ;;
      source) expected_message="$target: 出所の行が 0 個" ;;
      filename) expected_message='ファイル名が一致しない' ;;
      claude) expected_message='CLAUDE.md: 文書の表から docs/adr/ に辿る行が 0 個' ;;
      decision) expected_message='## 決定 の見出しが 0 個' ;;
      agents) expected_message='AGENTS.md: Story をまたぐ判断から docs/adr/ に辿る行が 0 個' ;;
      ci) expected_message='ci.yml: chain job の ADR 検査の行が 0 個' ;;
    esac
    output="$case_dir/output"
    if ADR_SELF_TEST_CHILD=1 ADR_DIR="$case_dir/docs/adr" ADR_CLAUDE_MD="$case_dir/CLAUDE.md" ADR_AGENTS_MD="$case_dir/AGENTS.md" ADR_CI_YML="$case_dir/.github/workflows/ci.yml" "$0" >"$output" 2>&1; then
      error "self-test の壊し方 $case_name で検査が落ちなかった"
    fi
    grep -Fq "$expected_message" "$output" || error "self-test の壊し方 $case_name が期待する error を出さなかった: $expected_message"
  done

  backlog_test="$tmp/backlog"
  mkdir -p "$backlog_test"
  if ADR_BACKLOG="$backlog_test/missing.py" "$0" --backlog >"$tmp/backlog-missing.out" 2>"$tmp/backlog-missing.err"; then
    error 'self-test の ADR_BACKLOG 欠落を通した'
  else
    case_rc=$?
    [ "$case_rc" -eq 127 ] || error "self-test の ADR_BACKLOG 欠落が rc=127 でない: $case_rc"
    grep -Fq 'error: ADR_BACKLOG の台本が無い' "$tmp/backlog-missing.err" ||
      error 'self-test の ADR_BACKLOG 欠落が台本の error を出さない'
  fi

  fake_backlog="$backlog_test/empty.py"
  cat >"$fake_backlog" <<'PY'
#!/usr/bin/env python3
import json
print(json.dumps([]))
PY
  if ADR_BACKLOG="$fake_backlog" ADR_MEMORY="$backlog_test/missing-memory" "$0" --backlog >"$tmp/memory-missing.out" 2>"$tmp/memory-missing.err"; then
    error 'self-test の ADR_MEMORY 欠落を通した'
  else
    case_rc=$?
    [ "$case_rc" -eq 127 ] || error "self-test の ADR_MEMORY 欠落が rc=127 でない: $case_rc"
    grep -Fq 'error: ADR_MEMORY の置き場が無い' "$tmp/memory-missing.err" ||
      error 'self-test の ADR_MEMORY 欠落が置き場の error を出さない'
  fi

  if ADR_BACKLOG="$fake_backlog" ADR_MEMORY="$backlog_test" "$0" --backlog >"$tmp/backlog-positive-missing.out" 2>"$tmp/backlog-positive-missing.err"; then
    error 'self-test の空の --adr の正の対照を通した偽の台本を通した'
  fi

  output="$tmp/backlog-child.out"
  if "$0" --backlog >"$output" 2>"$tmp/backlog-child.err"; then
    :
  else
    case_rc=$?
    [ "$case_rc" -eq 127 ] || error "self-test の --backlog 子が予期せず失敗した: $case_rc"
  fi
  local backlog_line_re='^(worktree-port-allocation-followup\.md|prefer-collision-free-allocation-over-checks\.md|foundation-for-satellites-premise\.md|lint-purity-ui-refresh-plan\.md):[[:space:]]残りに(無い|ある)$|^(OK|NG)[[:space:]].*$|^error: .*無い$'
  while IFS= read -r line; do
    [[ "$line" =~ $backlog_line_re ]] ||
      error "self-test の --backlog 子が公開してはいけない行を出した: $line"
  done < <(cat "$output" "$tmp/backlog-child.err")

  return "$errors"
}

case "${1:-}" in
  '') run_main ;;
  --self-test) self_test ;;
  --verbatim) verbatim ;;
  --backlog) backlog ;;
  *) echo "error: unknown option: $1" >&2; exit 2 ;;
esac
