# CH12 design — メモリにしか無かった判断を ADR に置く

> 第 1 回の深掘りの推奨の答え（Q1〜Q4）と、聞かずに決めた既定（`premises.md` の C1〜C4。ここでは D1・D2・D5 と D6、印「C」）で先に書いた。
> 本人の答えが推奨と違えば、その Q の D（仮）を反転条件のとおりに書き直す。

## Context

動機は `proposal.md` の Why。いまの形（`premises.md` の表。2026-11-08 の repo、`origin/main` = 0adef6712）:

- `docs/adr/` は無い。ADR の本文 6 つは `docs/changes/CH12.md` の「依頼（逐語）」に `> ` 付きの引用で置いてある（この段で上半分を書き直したので、素の写しは無くなった）
- 出所に挙げた 4 つのメモリは手元の `~/.claude/projects/-home-yosis-dev-ashiato2/memory/` に実在する。CI の runner には無い
- harness2 の `scripts/adr_backlog.py` は、この repo の `.harness2`（harness2 の main の checkout）には無い。harness2 の手元の worktree の 1 つ（枝 `docs/adr-backfill`）にだけある。
  この台本はメモリの置き場を cwd のパスから決め、無ければ rc=0 で終わる（空の合格。`review/deep.md` R1）。`--memory DIR`・`--adr DIR`（重ねてよい）・`--json` を受ける
- `tools/check-private.sh` は追跡ファイル全部を見る（CI の chain job と pre-commit）。`tools/check-private.allow` で意図した例外を外せる
- `CLAUDE.md` の「文書とハーネス」の表は `docs/`・`openspec/`・`.harness2`・symlink・`AGENTS.md` の 5 行

## Goals / Non-Goals

**Goals:**
- 依頼の 6 ファイルを逐語のまま `docs/adr/` に置き、`CLAUDE.md` の表から辿れるようにする（spec `decision-records` の 1 つ目の Requirement）
- 移した 4 つのメモリが ADR の出所に名指しされ、harness2 の数える台本の残りから外れることを確かめる（2 つ目）
- 網の名前・私設 IP を含めない（3 つ目）

**Non-Goals:**
- ADR の本文の手入れ（食い違いの直し・「その後」の追記）（Q1。D3（仮））
- ADR の形を CI で毎回見ること（Q3。D2（仮））
- agent に ADR を書かせる指示（Q4。D7（仮））
- harness2 の ADR 24 件と `adr_backlog.py` を harness2 の main に入れること（この repo の外。Q2）
- ハーネスのメモリ（19 件）を ADR にすること（harness2 の側の仕事）

## Decisions

### D1（C1）. 6 ファイルは依頼の逐語のまま置き、逐語との一致を 1 回だけ機械で見る

比べる元は `docs/changes/CH12.md` の `## 依頼（逐語）` の節（次の `## ` 見出しの手前まで）。各行の行頭の `> ` を外し、`>` だけの行は空行にする。
塊は `=== ファイル: <名前> ===` の行の次の行から、次の `=== ファイル:` の行の手前か節の終わりまで。塊の先頭と末尾の空行を落とし、末尾の改行を 1 つにそろえたものが、
`docs/adr/<名前>` の中身（末尾の改行を 1 つにそろえたもの）とバイト単位で一致すること。塊は 6 つで、名前も 6 つちょうど。

この一致は `tools/test-adr-docs.sh --verbatim` が見る。**spec の Scenario にしない**（正典に置かない）: README 自身が「決定が後で変わったら、状態を置き換え済みにして新しい ADR を足す」と
書いており、後の ADR の手入れで本文は変わってよい。逐語の一致は CH12 の受け入れの 1 回の判定で、常に成り立つ性質ではない。
`--verbatim` は CH12 の archive の後に走らせると、手入れの後では落ちうる（台本の冒頭のコメントにそう書く）。

### D2（仮・C2）. 足すのは 6 ファイル・`CLAUDE.md` の 1 行・検証の台本 1 本。台本は CI に載せない（Q3）

C2 は「足すのは 6 ファイルと `CLAUDE.md` の 1 行だけ」だった。spec の Scenario は印をコードの拡張子のファイルからしか拾わない（`scripts/check_scenarios.py`）ので、
印を持つ検証の台本 `tools/test-adr-docs.sh` を 1 本足す。**`.github/workflows/` には載せない**（Q3 の推奨「この change の検証で 1 回だけ見る」）。
コード・移行・既存の台本・CI は変えない。

反転条件: Q3 の答えが「足す」なら、`.github/workflows/ci.yml` の chain job に `./tools/test-adr-docs.sh` の 1 行を足す（`--verbatim` と `--backlog` は載せない。前者は D1、後者は CI にメモリが無い）。

### D3（仮）. 食い違いは直さずに PR 本文で挙げる（Q1）

ADR の本文と repo の今が食い違う箇所は、`premises.md` の表にある 8 つ（Q1 の context の 3 つと、`review/deep.md` R3 の 5 つ）。
加えて、README と 3 つの ADR が参照する harness2 の `docs/adr/` が、まだ harness2 の main に無いこと。これらを `docs/briefs/CH12-pr.md` に一覧で挙げる。ADR には書き足さない。

反転条件: Q1 の答えが「(1)(2) の ADR の『影響』の末尾に『その後』の 1 行を足す」なら、その 2 件の ADR だけ末尾に 1 行を足し、D1 の逐語の一致はその 2 件について「逐語の塊が先頭からそのまま入っている」に緩める。

### D4（仮）. 完了の判定は repo の中のコマンドで確かめ、harness2 の側を待たない（Q2）

`tools/test-adr-docs.sh --backlog` が harness2 の `adr_backlog.py` を走らせる。どこにも手元のパスを書かずに、次の順で探す:

1. 環境変数 `ADR_BACKLOG`（台本のパス）があればそれ
2. `.harness2/scripts/adr_backlog.py`（harness2 の main に入れば、ここで見つかる）
3. `.harness2` の指す repo の worktree（`git -C "$(readlink -f .harness2)" worktree list --porcelain` の順）のうち、`scripts/adr_backlog.py` を持つ最初のもの

メモリの置き場は、環境変数 `ADR_MEMORY` があればそれ、無ければこの repo の main の checkout（`git worktree list --porcelain` の 1 つめ）のパスから、
`adr_backlog.py` と同じ規則（英数字以外を `-` に）で `~/.claude/projects/<それ>/memory` を組む。worktree のパス（`…-up-ch12` など）から組まない（R1 の空の合格）。
走らせ方は `python3 -I <台本> --memory <置き場> --adr docs/adr --json`。`--adr` には `docs/adr` だけを渡す（4 つのメモリはこの repo の ADR が名指す。harness2 の ADR は要らない）。
rc は見ない（ハーネスのメモリが残る限り rc=1）。JSON の各行のファイル名に 4 つの名前のどれかがあれば落とす。

台本かメモリの置き場のどちらかが見つからなければ rc=127 と `error: …が無い` の 1 行で終える。verify-run は rc=127 を環境の欠落（BLOCKED_INFRA）と読み、FAIL にしない（`.harness2/scripts/evidence.py:437-440`）。

反転条件: Q2 の答えが「harness2 の ADR が main に入るのを待つ」なら、探し方を 2. だけにし、この change は harness2 の merge まで `--backlog` の検証で止まる。

### D5（C4）. 検証の出力には、4 つのメモリ名の有無だけを出す

`adr_backlog.py` の出力（残りのメモリ 1 件ごとのファイル名と description）は、`tools/test-adr-docs.sh` の中で受けて捨て、外に出さない。出すのは 4 行
（`<メモリ名>: 残りに無い` か `<メモリ名>: 残りにある`）と、結果の 1 行だけ。verify-run は出力の末尾 20 行を証跡（commit される）に入れ、この repo は公開なので、
ハーネスのメモリの名前と説明が外に出る経路になる（`review/deep.md` R5）。4 つの名前は既に `docs/changes/CH12.md` に公開されている。

### D6（C3）. 外に出ることは問わない

ADR の本文は、依頼の逐語として既に公開の origin にある（`premises.md` の 1 行目）。新しく出るのは `CLAUDE.md` の 1 行と検証の台本だけで、どちらも私的なものを含まない。

### D7（仮）. agent への案内は `CLAUDE.md` の表の 1 行だけ（Q4）

行は `| \`docs/adr/\` | Story をまたぐ判断の記録 | する |`。置き場は `docs/` の行の直後（`docs/` の下の特別な置き場なので、親の行の次に読める）。
いつ ADR を書くかの規則の正本は `docs/adr/README.md` の 1 か所で、`AGENTS.md` には書かない。

反転条件: Q4 の答えが「AGENTS.md にも足す」なら、`AGENTS.md` の「実装と計画の規約」に「Story をまたぐ判断は `docs/adr/` に書く（書き方は `docs/adr/README.md`）」の 1 行を足す。

### D8. capability `decision-records` を新しく立て、`skip_specs` は使わない

schema は docs の change に `skip_specs: true` を勧めるが、この repo の change で `skip_specs` を使った前例は無く、Requirement が 0 件の change がグラフ
（`check_scenarios`・`go_brief`・merge gate・archive）を通るかを確かめていない。また、索引と実ファイルの対応・出所の名指しは外から機械で確かめられる性質で、
CH10 の `emulator-startup`・CH09 の `test-timing` と同じく、開発の約束として正典に置ける。依頼の逐語との一致だけは正典に置かない（D1）。

### D9. 検証の台本の形

`tools/test-adr-docs.sh`（bash。冒頭のコメントに何を確かめるか・なぜ要るか（CH12）・走らせ方の 3 つの口）:

- 引数なし: 1 つ目と 3 つ目の Requirement と、2 つ目の「出所に名指しがある」を見る。各検査の前に `echo "Scenario: <名前>"` を出す。見る `docs/adr/` は環境変数 `ADR_DIR`（既定 `docs/adr`）、`CLAUDE.md` は `ADR_CLAUDE_MD`（既定 `CLAUDE.md`）。
  `tools/check-private.sh` は repo の根で走らせる（`ADR_DIR` を変えても、check-private は repo の追跡ファイルを見る）
- `--self-test`: `mktemp -d` の使い捨ての木に `docs/adr/` と `CLAUDE.md` を写し、`ADR_DIR` と `ADR_CLAUDE_MD` をそこへ向けて壊す。壊し方は 4 つ（一覧の 1 項目を消す・ADR の 1 件の出所の行を消す・ADR の名前を大文字にする・`CLAUDE.md` の表の行を消す）で、どれでも検査が rc≠0 になることを確かめる。`trap` で木を消す。
  「出所の行が無い ADR は落ちる」の印はここで出す
- `--verbatim`: D1
- `--backlog`: D4・D5。一時の出力は `mktemp` の場所（`/tmp` の固定名に書かない）
- 落ちるときは `error: ` で始まる行で、何が・どのファイルで違うかを出す

## Risks / Trade-offs

- [ADR の本文が repo の今と食い違ったまま入る（8 つ）] → 依頼の決定（「書き換えはしない」）。PR 本文に一覧で挙げ、人間が後で change を起こせるようにする（D3）
- [README と ADR からの harness2 の ADR への参照に、行き先がまだ無い] → 参照は題名で、パスのリンクではないので、リンク切れの検査には掛からない。harness2 の側が入れば解ける。PR 本文に書く
- [`--backlog` は手元でしか走らず、CI では確かめられない] → 完了の判定の 4. は手元の検証（verify-run）の証跡で確かめる。メモリが無い環境では BLOCKED_INFRA で止まり、空の合格にはしない（D4）
- [harness2 の worktree の探し方は、別の枝の古い `adr_backlog.py` を拾いうる] → 台本の入出力（`--memory`・`--adr`・`--json`）だけに依り、判定は 4 つのメモリ名の有無だけなので、版の違いに強い。`.harness2` に入れば 2. が先に当たる
- [台本を CI に載せないので、後の ADR で一覧の書き忘れが起きても止まらない] → Q3 の決定。起きたら change で CI に足す（D2 の反転条件）
- [`--verbatim` は ADR の手入れの後に落ちる] → CH12 の受け入れの口と台本のコメントに書く。正典の Scenario にはしない（D1）

## Migration Plan

文書と検証の台本を足すだけ。戻すときは `docs/adr/`・`tools/test-adr-docs.sh` を消し、`CLAUDE.md` の 1 行を消す。データ・移行・配布物に影響は無い。

## Open Questions

（無い）
