#!/usr/bin/env bash
# 送信契約の版の上げ忘れの検査（ST50 / design D7）。
# 封筒の形の写し docs/contract/envelope.json を版と一緒に持ち、次を見る:
#   1. 写しの S-01 の部分が docs/openapi.json（4 つの型の properties と required）から作り直したものと同じ
#   2. S-01・携帯・PC の版の定数が写しの version と同じ（D12）
#   3. 最後の配った印（タグ v*。ベータ版を含む）の写しと比べ、欄の追加・削除・型・必須の変化があるのに
#      version が印の版より大きくなければ落とす。印が 0 個なら比べない。印があるのに写しを取り出せなければ落とす
#   4. 配った印の版にある封筒の例（crates/server/tests/fixtures/contract/）が今の木で変わっている・消えていれば落とす
#   5. docs/collector-contract.md の「## 送信契約の版の履歴」の表に今の version の行が無ければ落とす
#   6. 写しの収集アプリ 2 本の欄が、同じ口の S-01 の欄に全部あること（収集アプリの欄 ⊆ S-01 の欄。無ければ
#      S-01 は `unknown_field` で断り、行は手元に残り続ける。final review R4）
# 収集アプリ 2 本の欄のずれは、それぞれの単体試験（St50EnvelopeShapeTest・st50_envelope_shape）が落とす。
# CI は rust の job の check-released-migrations.sh の隣に置く（印を取り寄せているのはその job だけ）。
# 使い方: check-contract-version.sh [--write | --self-test]
#   --write   写しを作り直す（手元で gradle と cargo を走らせて収集アプリ 2 本の欄を書き出す）
set -euo pipefail
cd "$(dirname "$0")/.."
ROOT="$PWD"

COPY=docs/contract/envelope.json
OPENAPI=docs/openapi.json
SERVER_CONST=crates/server/src/contract_version.rs
WINDOWS_CONST=crates/collector-windows/src/contract.rs
ANDROID_CONST=collector-android/app/src/main/kotlin/dev/ashiato/collector/IngestRequest.kt
CONTRACT_DOC=docs/collector-contract.md
FIXTURES=crates/server/tests/fixtures/contract

# S-01 の 4 つの型の欄ごとの JSON の型（昇順）と必須。型が無い欄（任意の JSON）は ["any"]
S01_JQ='
def norm: if has("type") then ((.type | if type == "array" then . else [.] end) | sort) else ["any"] end;
.components.schemas as $s
| ["IngestRequest", "HeartbeatRequest", "DropReportRequest", "MediaMissingRequest"]
| map({key: ., value: ($s[.] | (.required // []) as $r | .properties
    | with_entries(.key as $k | {key: $k, value: {type: (.value | norm), required: any($r[]; . == $k)}}))})
| from_entries'

# 欄ごとに 1 行 "<部>.<口>.<欄>=<型と必須>" にする（差分の欄の名前を出すため）
LINES_JQ='
def lines(p): to_entries[] | .key as $g | .value | to_entries[] | "\(p).\($g).\(.key)=\(.value | tojson)";
(.s01 | lines("s01")), (.android | lines("android")), (.windows | lines("windows"))'

# 収集アプリの口 → S-01 の型。S-01 に無い欄を "<部>.<口>.<欄>" で出す（知らない口は全部の欄を出す）
SUBSET_JQ='
{"ingest": "IngestRequest", "heartbeat": "HeartbeatRequest", "drops": "DropReportRequest", "media_missing": "MediaMissingRequest"} as $m
| . as $c | ("android", "windows") as $app | ($c[$app] // {}) | to_entries[] | .key as $g
| ($c.s01[$m[$g] // ""] // {}) as $srv | .value | keys[] | select(. as $f | $srv | has($f) | not)
| "\($app).\($g).\(.)"'

S01_LINES_JQ='to_entries[] | .key as $g | .value | to_entries[] | "\($g).\(.key)=\(.value | tojson)"'

const_of() { sed -n 's/^[[:space:]]*\(pub \)\{0,1\}const \(val \)\{0,1\}CONTRACT_VERSION: [A-Za-z0-9]* = \([0-9][0-9]*\).*/\3/p' "$1" | head -1; }

# 差分の欄の名前（"<部>.<口>.<欄>" の重複なし）
# diff は差があると rc=1 を返す（errexit と pipefail で台本が無言で止まらないよう吸う）
drift_names() { { diff <(printf '%s\n' "$1") <(printf '%s\n' "$2") || true; } | sed -n 's/^[<>] \([^=]*\)=.*/\1/p' | sort -u; }

# 今いる git の木を検査する。0: 通る / 1: 落ちる
check_tree() {
  local bad=0 ver
  if [ ! -f "$COPY" ]; then
    echo "  NG 封筒の形の写し $COPY が無い（tools/check-contract-version.sh --write で作る）"
    return 1
  fi
  ver="$(jq -r '.version' "$COPY")"
  case "$ver" in '' | *[!0-9]*) echo "  NG 写しの version が整数でない: $ver"; return 1 ;; esac

  # 1. 写しの S-01 の部分
  local s01_now s01_copy
  s01_now="$(jq -S -c "$S01_JQ" "$OPENAPI")"
  s01_copy="$(jq -S -c '.s01' "$COPY")"
  if [ "$s01_now" != "$s01_copy" ]; then
    echo "  NG 写しの S-01 の部分が $OPENAPI とずれている（--write で作り直す）: $(drift_names \
      "$(jq -r "$S01_LINES_JQ" <<<"$s01_now")" "$(jq -r "$S01_LINES_JQ" <<<"$s01_copy")" | tr '\n' ' ')"
    bad=1
  fi

  # 2. 3 つの版の定数
  local name file v
  for name in "S-01:$SERVER_CONST" "PC:$WINDOWS_CONST" "携帯:$ANDROID_CONST"; do
    file="${name#*:}"
    v="$(const_of "$file")"
    if [ "$v" != "$ver" ]; then
      echo "  NG ${name%%:*}の CONTRACT_VERSION（${v:-読めない}。$file）が写しの version（$ver）と違う"
      bad=1
    fi
  done

  # 5. 版の履歴の行
  if ! awk -v v="$ver" '
      /^## 送信契約の版の履歴/ { in_sec = 1; next }
      /^## / { in_sec = 0 }
      in_sec && $0 ~ "^\\|[[:space:]]*" v "[[:space:]]*\\|" { found = 1 }
      END { exit found ? 0 : 1 }' "$CONTRACT_DOC"; then
    echo "  NG $CONTRACT_DOC の「## 送信契約の版の履歴」の表に今の版（$ver）の行が無い"
    bad=1
  fi

  # 6. 収集アプリの欄 ⊆ S-01 の欄
  local extra
  extra="$(jq -r "$SUBSET_JQ" "$COPY")"
  if [ -n "$extra" ]; then
    echo "  NG 収集アプリが送る欄が S-01 に無い（unknown_field で断られる）: $(printf '%s' "$extra" | tr '\n' ' ')"
    bad=1
  fi

  # 3・4. 配った印
  local tags tag latest old old_ver
  tags="$(git tag --list 'v[0-9]*' --sort=-creatordate)"
  if [ -z "$tags" ]; then
    echo "配った印が 0 個（タグ v* が無い）: 印との比較はしない"
  else
    latest="$(printf '%s\n' "$tags" | head -1)"
    if ! old="$(git show "$latest:$COPY" 2>/dev/null)"; then
      echo "  NG 最後の印 $latest の $COPY を取り出せない（タグの取り寄せが足りない、または印の版に写しが無い）"
      bad=1
    else
      old_ver="$(jq -r '.version' <<<"$old")"
      local names
      names="$(drift_names "$(jq -r "$LINES_JQ" <<<"$old")" "$(jq -r "$LINES_JQ" "$COPY")")"
      if [ -n "$names" ] && ! { [[ "$old_ver" =~ ^[0-9]+$ ]] && [ "$ver" -gt "$old_ver" ]; }; then
        echo "  NG 印 $latest（版 $old_ver）から欄の追加・削除・型・必須が変わっているのに版が上がっていない（今 $ver）: $(printf '%s' "$names" | tr '\n' ' ')"
        bad=1
      fi
    fi
    for tag in $tags; do
      local f
      while IFS= read -r f; do
        [ -n "$f" ] || continue
        if ! git diff --quiet "$tag" HEAD -- "$f"; then
          echo "  NG 封筒の例 $f が印 $tag の版から変わっている・消えている（直さずに新しい版の例を足す）"
          bad=1
        fi
      done < <(git ls-tree -r --name-only "$tag" -- "$FIXTURES" 2>/dev/null)
    done
  fi
  [ "$bad" -eq 0 ] && echo "送信契約の版 OK（版 $ver）" || echo "送信契約の版 NG"
  return "$bad"
}

# 写しを作り直す。収集アプリ 2 本の欄は試験の直列化から書き出す
write_copy() {
  local tmp ver
  tmp="$(mktemp -d "${TMPDIR:-/tmp}/envelope.XXXXXX")"
  trap 'rm -rf "$tmp"' RETURN
  ver="$(const_of "$SERVER_CONST")"
  [ -n "$ver" ] || { echo "S-01 の CONTRACT_VERSION を読めない: $SERVER_CONST"; return 1; }
  ASHIATO_ENVELOPE_OUT="$tmp/android.json" ./collector-android/gradlew -p collector-android \
    :app:cleanTestDebugUnitTest :app:testDebugUnitTest --tests '*St50EnvelopeShapeTest*'
  ASHIATO_ENVELOPE_OUT="$tmp/windows.json" cargo test -p ashiato-collector-windows --lib contract::tests::st50_envelope_shape -- --exact
  [ -s "$tmp/android.json" ] && [ -s "$tmp/windows.json" ] || { echo "収集アプリの欄が書き出されなかった"; return 1; }
  mkdir -p "$(dirname "$COPY")"
  jq -S --argjson v "$ver" --slurpfile a "$tmp/android.json" --slurpfile w "$tmp/windows.json" \
    "{version: \$v, s01: ($S01_JQ), android: \$a[0], windows: \$w[0]}" "$OPENAPI" >"$COPY"
  echo "写しを作り直した: $COPY（版 $ver）"
}

case "${1:-}" in
  --write) write_copy; exit $? ;;
  --self-test) ;;
  '') check_tree; exit $? ;;
  *) echo "使い方: $0 [--write | --self-test]"; exit 2 ;;
esac

# ---- --self-test: 使い捨ての git の木で場合を走らせる ----
work="$(mktemp -d "${TMPDIR:-/tmp}/contract-version.XXXXXX")"
trap 'rm -rf "$work"' EXIT
fail=0
n=0

# 木を作る: 版 1 で何も食い違わない最初の commit。印は付けない
new_tree() {
  n=$((n + 1))
  T="$work/t$n"
  mkdir -p "$T/docs/contract" "$T/$(dirname "$SERVER_CONST")" "$T/$(dirname "$WINDOWS_CONST")" \
    "$T/$(dirname "$ANDROID_CONST")" "$T/$FIXTURES/v1" "$T/tools"
  # CI と同じ呼び方（別の bash・errexit が効く）で確かめるため、台本の写しを木に置く
  cp "$ROOT/tools/check-contract-version.sh" "$T/tools/"
  git -C "$T" init -q
  git -C "$T" config user.email t@example.com
  git -C "$T" config user.name t
  git -C "$T" config commit.gpgsign false
  cat >"$T/$OPENAPI" <<'EOF'
{"components": {"schemas": {
  "IngestRequest": {"required": ["id"], "properties": {"id": {"type": "string"}, "raw": {"type": ["string", "null"]}, "payload": {}}},
  "HeartbeatRequest": {"required": ["id"], "properties": {"id": {"type": "string"}}},
  "DropReportRequest": {"required": ["id"], "properties": {"id": {"type": "string"}}},
  "MediaMissingRequest": {"required": ["id"], "properties": {"id": {"type": "string"}}}}}}
EOF
  local fld='{"id": {"type": ["string"], "required": true}}'
  jq -S --argjson f "$fld" "{version: 1, s01: ($S01_JQ), android: {ingest: \$f, heartbeat: \$f, drops: \$f, media_missing: \$f}, windows: {ingest: \$f, heartbeat: \$f}}" \
    "$T/$OPENAPI" >"$T/$COPY"
  echo 'pub const CONTRACT_VERSION: u32 = 1;' >"$T/$SERVER_CONST"
  echo 'pub const CONTRACT_VERSION: u32 = 1;' >"$T/$WINDOWS_CONST"
  echo 'const val CONTRACT_VERSION: Int = 1' >"$T/$ANDROID_CONST"
  printf '# 契約\n\n## 送信契約の版の履歴\n\n| 版 | 最初の印 | 足した欄・変えたこと | 理由 |\n|---|---|---|---|\n| 1 | 未定 | 全部 | 最初の版 |\n\n## 次\n' >"$T/$CONTRACT_DOC"
  echo '{"id": "x"}' >"$T/$FIXTURES/v1/ingest.json"
  git -C "$T" add -A
  git -C "$T" commit -q -m base
}
commit_all() { git -C "$T" add -A && git -C "$T" commit -q -m "$1"; }
tag_beta() { git -C "$T" tag v0.1.0-beta.1; }
# 写しを jq で直す（"." で終わる式は無い。$1 = jq の式）
edit_copy() { jq -S "$1" "$T/$COPY" >"$T/copy.tmp" && mv "$T/copy.tmp" "$T/$COPY"; }
# S-01 と携帯の両方に欄 new_field を足す（S-01 に無い欄は検査 6 が落とす）
add_field_both() {
  jq '.components.schemas.IngestRequest.properties.new_field = {"type": "string"}' "$T/$OPENAPI" >"$T/o.tmp" && mv "$T/o.tmp" "$T/$OPENAPI"
  edit_copy '.s01.IngestRequest.new_field = {type: ["string"], required: false} | .android.ingest.new_field = {type: ["string"], required: false}'
}
# 版を $1 に上げる（3 つの定数・写し・履歴の行）
bump_all() {
  sed -i "s/= [0-9][0-9]*/= $1/" "$T/$SERVER_CONST" "$T/$WINDOWS_CONST" "$T/$ANDROID_CONST"
  edit_copy ".version = $1"
  sed -i "s/^| 1 | 未定 | 全部 | 最初の版 |\$/&\n| $1 | 未定 | 欄を足した | テスト |/" "$T/$CONTRACT_DOC"
}
# 期待: $1=0 通る / 1 落ちる（印の echo は呼ぶ側に literal で置く。check_scenarios.py が literal を拾う）
expect() {
  local want="$1" rc=0 out
  out="$(cd "$T" && bash tools/check-contract-version.sh 2>&1)" || rc=$?
  if [ "$rc" -ne "$want" ]; then
    echo "  FAIL: rc=$rc（期待 $want）"; printf '%s\n' "$out" | sed 's/^/    /'
    fail=1
  fi
}
# 落ちる場合は出力に $2 が含まれること（どの検査で落ちたかを見る）
expect_msg() {
  local out rc=0
  out="$(cd "$T" && bash tools/check-contract-version.sh 2>&1)" || rc=$?
  if [ "$rc" -ne 1 ] || ! printf '%s\n' "$out" | grep -qF -- "$1"; then
    echo "  FAIL: rc=$rc で「$1」が出ない"; printf '%s\n' "$out" | sed 's/^/    /'
    fail=1
  fi
}

new_tree; tag_beta
expect 0

new_tree; tag_beta
edit_copy '.android.ingest.new_field = {type: ["string"], required: false}'; commit_all add-android-field
echo "== Scenario: 収集アプリだけが欄を足して版を上げないと CI が落ちる"; expect_msg "android.ingest.new_field"

new_tree; tag_beta
edit_copy '.android.ingest.id.required = false'; commit_all required-changed
echo "== Scenario: 既存の欄の型や必須を変えて版を上げないと CI が落ちる"; expect_msg "android.ingest.id"
edit_copy '.android.ingest.id.required = true | .android.ingest.id.type = ["integer"]'; commit_all type-changed
expect_msg "android.ingest.id"

new_tree; tag_beta
add_field_both; bump_all 2; commit_all add-and-bump
echo "== Scenario: 版を上げれば欄を足しても通る"; expect 0

new_tree
add_field_both; commit_all add-android-field
echo "== Scenario: 配った印が無い間は欄を足しても通る"; expect 0

new_tree
edit_copy '.windows.heartbeat.extra = {type: ["string"], required: false}'; commit_all app-only-field
echo "== 収集アプリが S-01 に無い欄を送ると、印が無くても落ちる"; expect_msg "windows.heartbeat.extra"
bump_all 2; commit_all app-only-field-bumped
expect_msg "S-01 に無い"

new_tree
echo 'pub const CONTRACT_VERSION: u32 = 2;' >"$T/$WINDOWS_CONST"; commit_all const-differs
echo "== Scenario: 3 つの版の値が食い違うと CI が落ちる"; expect_msg "PCの CONTRACT_VERSION"
echo 'const val CONTRACT_VERSION: Int = 3' >"$T/$ANDROID_CONST"; commit_all const-differs-2
expect_msg "携帯の CONTRACT_VERSION"

# 写しの S-01 の部分が docs/openapi.json とずれている（写しを直さずに型を足した）
new_tree
jq '.components.schemas.IngestRequest.properties.added = {"type": "string"}' "$T/$OPENAPI" >"$T/o.tmp" && mv "$T/o.tmp" "$T/$OPENAPI"; commit_all openapi-only
echo "== 写しの S-01 の部分が docs/openapi.json とずれていると落ちる"; expect_msg "IngestRequest.added"

new_tree; tag_beta
echo '{"id": "changed"}' >"$T/$FIXTURES/v1/ingest.json"; commit_all fixture-edit
echo "== Scenario: 配った版の封筒の例を書き換えると CI が落ちる"; expect_msg "封筒の例"
git -C "$T" rm -q "$FIXTURES/v1/ingest.json"; commit_all fixture-rm
expect_msg "封筒の例"

# 印はあるが、その印の版には写しが無い
new_tree
git -C "$T" rm -q "$COPY"; commit_all no-copy; tag_beta
git -C "$T" checkout -q HEAD~1 -- "$COPY"; commit_all copy-back
echo "== Scenario: 配った印の写しを取り出せないと CI が落ちる"; expect_msg "取り出せない"

new_tree; tag_beta
edit_copy 'del(.android.ingest.id)'; commit_all remove-field
echo "== Scenario: 欄を消して版を上げないと CI が落ちる"; expect_msg "android.ingest.id"

new_tree; tag_beta
edit_copy '.windows.heartbeat.id.type = ["integer"]'; commit_all type-changed
echo "== Scenario: ベータ版の印も配った印として比べる"; expect_msg "windows.heartbeat.id"

new_tree
sed -i '/^| 1 | /d' "$T/$CONTRACT_DOC"; commit_all no-history-row
echo "== Scenario: 版の履歴に今の版の行が無いと CI が落ちる"; expect_msg "版の履歴"

[ "$fail" -eq 0 ] && echo "check-contract-version self-test OK" || { echo "check-contract-version self-test NG"; exit 1; }
