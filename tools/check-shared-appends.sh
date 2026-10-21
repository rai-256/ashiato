#!/usr/bin/env bash
# Story の PR（feat/st*）と change の PR（feat/ch*）が、並走する Story と取り合う共有の本体に書き足していないことを見る（2026-10-15）。
#
# 実測 2026-10-15: open の ST08・ST14・ST17・ST47 は main とは衝突しないのに、**どの 2 本を合わせても衝突**し、
# 確認バッチに 1 本ずつしか入らなかった。どれも「同じファイルの末尾に 1 節（1 行）足す」衝突だった:
# `crates/server/src/lib.rs` の `MIGRATIONS` 配列（要素数の行も）・`tools/smoke.sh`・`tools/recording/recording.json`。
# 書き足す先を Story ごとの新しいファイルにして、取り合う場所そのものを無くした:
#   移行の順            → migrations/ORDER の末尾に 1 行（.gitattributes の merge=union。途中は変えない）
#   縦串・錠・偽データ  → tools/smoke.d/ ・ tools/check-immutable.d/ ・ tools/seed.d/ に 1 ファイル
#   録画のシナリオ      → tools/recording/scenarios.d/ に 1 ファイル
# ここはその約束を機械で守らせる。本体を直すのは Story でなく chore / fix の PR。
#
# 本体の途中の節の中身を直すのは通す（ST47 が書庫の節に自分の検査を足す、など。毎回は衝突しない）。落とすのは
#   - smoke.sh / check-immutable.sh / seed.sh の末尾（Story ごとの節を読む口と、後ろから 2 番目の空でない行より後ろ）に触れた変更。
#     読む口のかたまりを一字一句そのまま別の位置へ動かすだけの変更は通す
#   - 同じ 3 本の途中に、新しい Story の節の見出し（`# ==== ST<NN>` / `echo "== ST<NN>:` / `echo "== 41. …（ST<NN>…）"`）を足した変更
#     （同じ ST 番号の見出しの言い直しは通す）
#   - recording.json にシナリオの id か waive の鍵を足した変更・tools/recording/README.md のシナリオの表に行を足した変更
#   - migrations/ORDER の途中を変えた・消した変更（末尾への追記だけ通す）
#
#   tools/check-shared-appends.sh [<branch>]   既定は GITHUB_HEAD_REF、無ければ今のブランチ。比べる先は origin/main との分岐点
#   tools/check-shared-appends.sh --self-test  わざと書き足した木で NG になることを確かめる
# rc: 0 = 守っている（feat/st* / feat/ch* でなければ何も見ない）/ 1 = 書き足した / 2 = 比べられない
set -euo pipefail

SCRIPTS=(tools/smoke.sh tools/check-immutable.sh tools/seed.sh)
REC=tools/recording/recording.json
RECMD=tools/recording/README.md
ORDER=migrations/ORDER
MARK='Story ごとの節（tools/'   # 本体の末尾の「Story ごとの節を読む口」の見出し

# 読む口のかたまり（見出しの行 〜 `for … done` の `done`）の行番号を「<見出し> <done>」で出す。無ければ何も出さない
mouth_span() {
  awk -v m="$MARK" '!s && index($0, m) { s = NR }
    s && !e && $0 !~ /^[[:space:]]*#/ && $0 ~ /(^|[[:space:];])done[[:space:]]*$/ { e = NR }
    END { if (s && e) print s, e }'
}

# 読む口のかたまりの中身（見出しの行と、注記・空行でない行）。動かしただけかを比べる。注記の行は比べない
mouth_sig() {
  awk -v m="$MARK" '!s && index($0, m) { s = 1; print; next }
    s { if ($0 !~ /^[[:space:]]*(#|$)/) print
        if ($0 !~ /^[[:space:]]*#/ && $0 ~ /(^|[[:space:];])done[[:space:]]*$/) exit }'
}

# 後ろから 2 番目の空でない行の行番号（1 行しか無ければその行）
second_last_line() {
  awk '/[^[:space:]]/ { prev = last; last = NR } END { print (prev ? prev : last) + 0 }'
}

# Story の節の見出しを、同じ ST 番号の見出しが `-` 行に無いまま足した（=既存の見出しの言い直しでない）なら 0。本体のどこでも
heading_added() {   # heading_added <base> <file>
  git diff -U0 "$1" HEAD -- "$2" | awk '
    function heading(s) { return s ~ /^# *[=-][=-][=-]+.*ST[0-9][0-9]/ || s ~ /^echo "== (Scenario: )?ST[0-9][0-9]/ \
                                 || s ~ /^echo "== [0-9]+[a-z]?\. .*ST[0-9][0-9]/ }
    function stno(s) { match(s, /ST[0-9][0-9]/); return substr(s, RSTART, RLENGTH) }
    /^@@/ { inh = 1; next }
    !inh { next }
    /^-/ { l = substr($0, 2); if (heading(l)) old[stno(l)] = 1; next }
    /^\+/ { l = substr($0, 2); if (heading(l)) add[++n] = stno(l); next }
    END { for (i = 1; i <= n; i++) if (!(add[i] in old)) bad = 1; exit bad ? 0 : 1 }'
}

# 本体の途中の節を直すのは通す（その節を持つ Story どうしの本当の衝突は残るが、毎回は起きない）。落とすのは「末尾に節を足す」—— 守る範囲:
#   1. 読む口の見出しの直前の空でない行 〜 読む口のかたまりの `done`
#   2. 後ろから 2 番目の空でない行 〜 ファイルの最後（読む口が末尾にある木では `done` より後ろ。読む口を動かした木では最後の検査より後ろ）
# 読む口のかたまりを別の位置へ動かすだけ（見出しと `for … done` の行が基準の木と同じ）なら、かたまりを両方の木から抜いて比べる
tail_touched() {   # tail_touched <base> <file> → 0 = 末尾に触れた
  local tmp rc=0
  tmp="$(mktemp -d)"
  tail_touched_in "$tmp" "$1" "$2" || rc=$?
  rm -rf "${tmp:?}"
  return "$rc"
}

tail_touched_in() {   # tail_touched_in <作業の置き場> <base> <file>
  local tmp="$1" base="$2" f="$3" span s e hs he p lo2
  git show "$base:$f" > "$tmp/base" 2>/dev/null || return 1
  span="$(mouth_span < "$tmp/base")"
  [ -n "$span" ] || return 1     # 読む口が無い基準（古い分岐点）は判断しない —— main を取り込めば判断できる
  read -r s e <<<"$span"
  p="$(awk -v s="$s" 'NR < s && /[^[:space:]]/ { p = NR } END { print p + 0 }' "$tmp/base")"
  git show "HEAD:$f" > "$tmp/head" 2>/dev/null || : > "$tmp/head"
  hs=""
  span="$(mouth_span < "$tmp/head")"
  [ -z "$span" ] || read -r hs he <<<"$span"
  if [ -n "$hs" ] && [ "$(mouth_sig < "$tmp/base")" = "$(mouth_sig < "$tmp/head")" ]; then
    awk -v s="$s" -v e="$e" 'NR < s || NR > e' "$tmp/base" > "$tmp/b"
    awk -v s="$hs" -v e="$he" 'NR < s || NR > e' "$tmp/head" > "$tmp/h"
    lo2="$(second_last_line < "$tmp/b")"
    { diff -B -U0 "$tmp/b" "$tmp/h" || :; } | hunk_hits "$p" "$p" "$lo2"
  else
    lo2="$(second_last_line < "$tmp/base")"
    git diff -U0 "$base" HEAD -- "$f" | hunk_hits "$p" "$e" "$lo2"
  fi
}

hunk_hits() {   # hunk_hits <lo1> <hi1> <lo2> → 差分（-U0）の旧い側が [lo1,hi1] か [lo2,最後] に触れたら 0。挿入は「その行の後ろ」
  awk -v a="$1" -v b="$2" -v c="$3" '
    function inr(lo, hi) { return n > 0 ? (st <= hi && st + n - 1 >= lo) : (st >= lo && st <= hi) }
    /^@@/ { split($2, x, ","); st = substr(x[1], 2) + 0; n = (x[2] == "" ? 1 : x[2] + 0)
            if (inr(a, b) || inr(c, 1000000000)) hit = 1 }
    END { exit hit ? 0 : 1 }'
}

check() {   # check <branch> <base-ref>
  local branch="$1" base="$2" mb bad=0 f
  # Story（feat/st<NN>-）と change（feat/ch<NN>-。harness2 の scripts/sid.py）。どちらも並走して同じ末尾を取り合う
  case "$branch" in feat/st[0-9][0-9]*|feat/ch[0-9][0-9]*) ;; *) echo "共有の本体: $branch は Story / change のブランチでないので見ない"; return 0;; esac
  mb="$(git merge-base "$base" HEAD 2>/dev/null)" || { echo "  NG $base との分岐点が無い（比べられない）"; return 2; }
  for f in "${SCRIPTS[@]}"; do
    if tail_touched "$mb" "$f"; then
      echo "  NG $f の末尾に書き足した —— Story の節は ${f%.sh}.d/YYYYMMDDHHMM_st<NN>-<slug>.sh に新しいファイルで置く"
      bad=1
    fi
    if heading_added "$mb" "$f"; then
      echo "  NG $f の途中に Story の節の見出しを足した —— Story の節は ${f%.sh}.d/YYYYMMDDHHMM_st<NN>-<slug>.sh に新しいファイルで置く"
      bad=1
    fi
  done
  if git cat-file -e "$mb:$REC" 2>/dev/null && git cat-file -e "HEAD:$REC" 2>/dev/null; then
    local grew
    grew="$(python3 - "$mb" <<'PY'
import json, subprocess, sys
show = lambda ref: json.loads(subprocess.run(["git", "show", f"{ref}:tools/recording/recording.json"],
                                             capture_output=True, text=True, check=True).stdout)
old, new = show(sys.argv[1]), show("HEAD")
ids = lambda c: {s.get("id") for s in c.get("scenarios") or []}
print(" ".join(sorted(ids(new) - ids(old)) + sorted(set(new.get("waive") or {}) - set(old.get("waive") or {}))))
PY
)"
    if [ -n "$grew" ]; then
      echo "  NG $REC にシナリオか waive を足した（$grew）—— tools/recording/scenarios.d/YYYYMMDDHHMM_st<NN>.json に置く"
      bad=1
    fi
  fi
  if git diff -U0 "$mb" HEAD -- "$RECMD" | grep -qE '^\+\| `st'; then
    echo "  NG $RECMD のシナリオの表に行を足した —— 説明は scenarios.d のシナリオの note に置く"
    bad=1
  fi
  if git cat-file -e "$mb:$ORDER" 2>/dev/null; then
    local old new
    old="$(git show "$mb:$ORDER")"
    new="$(git show "HEAD:$ORDER" 2>/dev/null || true)"
    if [ "${new:0:${#old}}" != "$old" ]; then
      echo "  NG $ORDER の途中を変えた・消した —— 新しい版は末尾に 1 行足すだけ（並べ替え・差し込みは起動のたびの当て直しの順を変える）"
      bad=1
    fi
  fi
  [ "$bad" -eq 0 ] && echo "共有の本体 OK（$branch）" || { echo "共有の本体 NG（$branch）"; return 1; }
}

self_test() {
  local t; t="$(mktemp -d)"
  trap 'rm -rf "${t:?}"' RETURN
  (
    cd "$t"
    git init -q -b main . && git config user.email t@example.invalid && git config user.name t
    mkdir -p tools/recording migrations tools/smoke.d
    for f in "${SCRIPTS[@]}"; do
      printf 'echo 1\n# ---- ST01 の節\necho a\necho b\n\n# ==== %s%s.d/）\nfor part in x; do :; done\necho 終わり\n' "$MARK" "$(basename "${f%.sh}")" > "$f"
    done
    echo '{"scenarios": [{"id": "A"}], "waive": {}, "web_dir": "web"}' > "$REC"
    printf '# 説明\na\nb\n' > "$ORDER"
    git add -A && git commit -qm base
    git update-ref refs/remotes/origin/main HEAD
    expect() {  # expect <rc> <説明> <branch>
      local rc=0; check "$3" origin/main >/dev/null || rc=$?
      [ "$rc" = "$1" ] || { echo "  self-test NG: $2（rc=$rc、期待 $1）"; exit 1; }
      echo "  self-test ok: $2"
    }
    git checkout -qb feat/st01-ok && echo x > tools/smoke.d/202610150000_st01-x.sh && echo c >> "$ORDER"
    echo '[{"id": "B"}]' > tools/recording/x.json
    git add -A && git commit -qm ok
    expect 0 "節を新しいファイルに置き、ORDER の末尾に足すのは通す" feat/st01-ok
    git checkout -q main && git checkout -qb feat/st02-mid && sed -i 's/echo a/echo a2/' tools/smoke.sh \
      && sed -i 's/"web_dir": "web"/"web_dir": "web2"/' "$REC" && git commit -qam mid
    expect 0 "本体の途中の節を直す・recording.json のほかの値を直すのは通す" feat/st02-mid
    git checkout -q main && git checkout -qb feat/st03-tail && awk -v m="$MARK" 'index($0, m) { print "echo 足した節" } 1' tools/smoke.sh > s.tmp \
      && mv s.tmp tools/smoke.sh && git commit -qam tail
    expect 1 "読む口の直前（末尾）に節を足すと落とす" feat/st03-tail
    git checkout -q main && git checkout -qb feat/st04-end && echo more >> tools/seed.sh && git commit -qam end
    expect 1 "ファイルの最後に書き足すと落とす" feat/st04-end
    git checkout -q main && git checkout -qb feat/ch01-end && echo more >> tools/seed.sh && git commit -qam end
    expect 1 "change のブランチ（feat/ch*）が書き足しても落とす" feat/ch01-end
    git checkout -q main && git checkout -qb feat/st08-after-last && sed -i '/^echo b$/a echo 足した節' tools/check-immutable.sh \
      && git commit -qam after
    expect 1 "最後の節の直後（空行の手前）に足しても落とす" feat/st08-after-last
    git checkout -q main && git checkout -qb feat/st05-mid && printf '# 説明\na\nz\nb\n' > "$ORDER" && git commit -qam order
    expect 1 "ORDER の途中に差し込むと落とす" feat/st05-mid
    git checkout -q main && git checkout -qb feat/st06-rec \
      && echo '{"scenarios": [{"id": "A"}, {"id": "C"}], "waive": {}, "web_dir": "web"}' > "$REC" && git commit -qam rec
    expect 1 "recording.json にシナリオを足すと落とす" feat/st06-rec
    git checkout -q main && git checkout -qb feat/st07-waive \
      && echo '{"scenarios": [{"id": "A"}], "waive": {"ST07": "x"}, "web_dir": "web"}' > "$REC" && git commit -qam waive
    expect 1 "recording.json に waive を足すと落とす" feat/st07-waive
    git checkout -q main && git checkout -qb feat/st09-readme && echo '| `st09/x.rec.ts` | ST09 |' >> "$RECMD" && git add -A \
      && git commit -qm readme
    expect 1 "録画の README の表に行を足すと落とす" feat/st09-readme
    git checkout -q main && git checkout -qb chore/fix-smoke && echo more >> tools/smoke.sh && git commit -qam chore
    expect 0 "chore のブランチは本体を直してよい" chore/fix-smoke
    # --- 本体の途中への節の見出し（design D2）。読む口が末尾にある木（動かす前）
    git checkout -q main && git checkout -qb feat/st91-head-eq && sed -i '/^echo a$/a # ==== ST99: 新しい節' tools/smoke.sh && git commit -qam h1
    expect 1 "本体の途中に # ==== ST99 の見出しを足すと落とす" feat/st91-head-eq
    git checkout -q main && git checkout -qb feat/st92-head-echo && sed -i '/^echo a$/a echo "== ST99: 新しい節"' tools/seed.sh && git commit -qam h2
    expect 1 "本体の途中に echo \"== ST99: の見出しを足すと落とす" feat/st92-head-echo
    git checkout -q main && git checkout -qb feat/st93-head-num && sed -i '/^echo a$/a echo "== 45. 新しい節（ST99 の確かめ）"' tools/smoke.sh \
      && git commit -qam h3
    expect 1 "smoke.sh の途中に番号つきの見出し（echo \"== 45. …（ST99））を足すと落とす" feat/st93-head-num
    git checkout -q main && git checkout -qb feat/st94-head-fix && sed -i 's/^# ---- ST01 の節$/# ---- ST01 の節（言い直し）/' tools/smoke.sh && git commit -qam h4
    expect 0 "既存の小見出しの文言を直すのは通す" feat/st94-head-fix
    git checkout -q main && git checkout -qb feat/st95-gap && sed -i '/^for part in x/a echo 足した' tools/smoke.sh && git commit -qam gap
    expect 1 "動かす前の木で done と最後の判定の行の間に 1 行足すと落とす" feat/st95-gap
    # --- 読む口のかたまりを動かす（注記を直して見出しの直後へ）。動かすだけは通す
    git checkout -q main && git checkout -qb feat/st96-move \
      && printf 'echo 1\n# ==== %s%s.d/）\n# 注記を直した\nfor part in x; do :; done\n# ---- ST01 の節\necho a\necho b\n\necho 終わり\n' \
        "$MARK" check-immutable > tools/check-immutable.sh && git commit -qam move
    expect 0 "読む口のかたまりをそのまま前へ動かすだけなら通す" feat/st96-move
    git checkout -q main && git merge -q --ff-only feat/st96-move && git update-ref refs/remotes/origin/main HEAD
    # --- 読む口が途中にある木（動かした後）
    git checkout -qb feat/st97-after-mouth && sed -i '/^for part in x/a echo 足した節' tools/check-immutable.sh && git commit -qam am
    expect 1 "動かした後の木で読む口の直後に節を足すと落とす" feat/st97-after-mouth
    git checkout -q main && git checkout -qb feat/st98-after-end && echo more >> tools/check-immutable.sh && git commit -qam ae
    expect 1 "動かした後の木でファイルの最後に書き足すと落とす" feat/st98-after-end
    git checkout -q main && git checkout -qb feat/st99-after-mid && sed -i 's/^echo a$/echo a2/' tools/check-immutable.sh && git commit -qam mid2
    expect 0 "動かした後の木で戻しの並びの途中の行を直すのは通す" feat/st99-after-mid
    git checkout -q main && git checkout -qb feat/ch02-after-gap && sed -i '/^echo b$/a echo 足した' tools/check-immutable.sh && git commit -qam ag
    expect 1 "動かした後の木で最後の検査と最後の判定の行の間に 1 行足すと落とす" feat/ch02-after-gap
  )
}

if [ "${1:-}" = --self-test ]; then self_test; echo "self-test OK"; exit 0; fi
cd "$(dirname "$0")/.."
branch="${1:-${GITHUB_HEAD_REF:-$(git rev-parse --abbrev-ref HEAD)}}"
check "$branch" "${BASE_REF:-origin/main}"
