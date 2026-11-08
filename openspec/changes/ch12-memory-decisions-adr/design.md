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

**比べる元そのものも固定する**（`review/spec.md` R9）: `--verbatim` は先に、いまの `docs/changes/CH12.md` の節の sha256 が、依頼を登録したときの節の値
`7e1b0a5c755c8b19fcb7c138db6f2f87897d96baa46f90b3bc889ba872e58b94` と一致することを見る。違えば `error:` で落とす。値は台本に定数で書く。
節の取り出し方: `## 依頼（逐語）` の行から、次の `## ` で始まる行の手前までの行（末尾の空行も含む）を、各行の後に改行 1 つを付けて並べたバイト列（243 行）。
上流が登録の commit `ff9a78dd7` の節と、いまの節の両方で同じ値になることを確かめた。commit の SHA でなく中身の値にするのは、枝が rebase されると SHA が履歴から消えうるから。
こうしないと、ADR と節を一緒に書き換えたときにどの検証も落ちない。

### D2（仮・C2）. 足すのは 6 ファイル・`CLAUDE.md` の 1 行・検証の台本 1 本。台本は CI に載せない（Q3）

C2 は「足すのは 6 ファイルと `CLAUDE.md` の 1 行だけ」だった（`premises.md` の C2 に、この段で改めたことを書き足した。`review/spec.md` R11）。spec の Scenario は印をコードの拡張子のファイルからしか拾わない（`scripts/check_scenarios.py`）ので、
印を持つ検証の台本 `tools/test-adr-docs.sh` を 1 本足す。**`.github/workflows/` には載せない**（Q3 の推奨「この change の検証で 1 回だけ見る」）。
コード・移行・既存の台本・CI は変えない。

反転条件: Q3 の答えが「足す」なら、`.github/workflows/ci.yml` の chain job に `./tools/test-adr-docs.sh` の 1 行を足す（`--verbatim` と `--backlog` は載せない。前者は D1、後者は CI にメモリが無い）。

### D3（仮）. 食い違いは直さずに PR 本文で挙げる（Q1）

ADR の本文と repo の今が食い違う箇所は、`premises.md` の表にある 8 つ（Q1 の context の 3 つと、`review/deep.md` R3 の 5 つ）。
加えて、README と 3 つの ADR が参照する harness2 の `docs/adr/` が、まだ harness2 の main に無いこと。これらを `docs/briefs/CH12-pr.md` に一覧で挙げる。ADR には書き足さない。

**後で見つける置き場**（`review/deep.md` R4・`review/spec.md` R3）: PR 本文は merge の後に流れていくので、同じ一覧を `docs/changes/CH12.md` の
`## ADR と repo の今の食い違い（直していない）` の節にも置く（「依頼（逐語）」の外。archive の後も repo に残る）。ADR の手入れの change を起こすとき、ここから拾う。
手入れの change はこの change では起こさない（本人が一覧を見て決める）。

反転条件: Q1 の答えが「(1)(2) の ADR の『影響』の末尾に『その後』の 1 行を足す」なら、その 2 件の ADR だけ末尾に 1 行を足し、D1 の逐語の一致はその 2 件について「逐語の塊が先頭からそのまま入っている」に緩める。

### D4（仮）. 完了の判定は repo の中のコマンドで確かめ、harness2 の側を待たない（Q2）

`tools/test-adr-docs.sh --backlog` が harness2 の `adr_backlog.py` を走らせる。どこにも手元のパスを書かずに、次の順で探す:

1. 環境変数 `ADR_BACKLOG`（台本のパス）が設定されていれば、それだけを使う。指す先が無ければ次へ落ちずに rc=127（`review/spec.md` R6）
2. `.harness2/scripts/adr_backlog.py`（harness2 の main に入れば、ここで見つかる）
3. `.harness2` の指す repo の worktree（`git -C "$(readlink -f .harness2)" worktree list --porcelain` の順）のうち、`scripts/adr_backlog.py` を持つ最初のもの

メモリの置き場は、環境変数 `ADR_MEMORY` が設定されていればそれだけ（指す先が無ければ rc=127）、設定されていなければこの repo の main の checkout（`git worktree list --porcelain` の 1 つめ）のパスから、
`adr_backlog.py` と同じ規則（英数字以外を `-` に）で `~/.claude/projects/<それ>/memory` を組む。worktree のパス（`…-up-ch12` など）から組まない（R1 の空の合格）。
走らせ方は `python3 -I <台本> --memory <置き場> --adr docs/adr --json`。`--adr` には `docs/adr` だけを渡す（4 つのメモリはこの repo の ADR が名指す。harness2 の ADR は要らない）。
判定（`review/spec.md` R4。「名前が出力に無い」だけでは、台本が落ちても・数える対象から外れても通るので、次の 3 つを全部見る）:

1. **正の対照**: 同じ置き場で `--adr <空の mktemp -d>` を走らせ、4 つの名前が**すべて**残りに出ること（数える対象に 4 つが在る）。出なければ落とす
2. 本番（`--adr docs/adr`）の rc が 0 か 1 で、標準出力が JSON の配列として読めること（rc=1 はハーネスのメモリが残る限り普通）。でなければ落とす
3. 本番の配列の各要素のファイル名に、4 つの名前のどれも無いこと

台本かメモリの置き場のどちらかが見つからなければ rc=127 と `error: …が無い` の 1 行で終える。verify-run は rc=127 を環境の欠落（BLOCKED_INFRA）と読み、FAIL にしない（`.harness2/scripts/evidence.py:437-440`）。

反転条件: Q2 の答えが「harness2 の ADR が main に入るのを待つ」なら、探し方を 2. だけにし、この change は harness2 の merge まで `--backlog` の検証で止まる。

### D5（仮・C4）. 検証の出力には、4 つのメモリ名の有無だけを出す

`adr_backlog.py` の標準出力（残りのメモリ 1 件ごとのファイル名と description）と標準エラーは、`tools/test-adr-docs.sh` の中で一時の場所に受けて捨て、外に出さない（`review/spec.md` R5）。出すのは 4 行
（`<メモリ名>: 残りに無い` か `<メモリ名>: 残りにある`）と、結果の 1 行だけ。verify-run は出力の末尾 20 行を証跡（commit される）に入れ、この repo は公開なので、
ハーネスのメモリの名前と説明が外に出る経路になる（`review/deep.md` R5）。4 つの名前は既に `docs/changes/CH12.md` に公開されている。
出してよい行の形は 4 つだけ: `Scenario: …` は使わない（`--backlog` は正典に Scenario を持たない。D8）ので、`<4 つの名前のどれか>: 残りに(無い|ある)`・`OK …` / `NG …` の結果の 1 行・`error: …`。
`--self-test` が `--backlog` を子として走らせ、標準出力と標準エラーの全行がこの形のどれかに当たることを見る（メモリか台本が無い環境では子が rc=127 で、形だけを見る）。

（仮）である理由: 失わない側（出さない）を選んだ。反転条件は、メモリの残りを公開してよいと本人が決めたとき（その場合も一覧を出す理由は無いので、変える見込みは小さい）。
台帳 `provisional.jsonl` に載せる（`review/deep.md` R5 の `loss: exported`）。

### D6（C3）. 外に出ることは問わない

ADR の本文は、依頼の逐語として既に公開の origin にある（`premises.md` の 1 行目）。新しく出るのは `CLAUDE.md` の 1 行と検証の台本だけで、どちらも私的なものを含まない。

### D7（仮）. agent への案内は `CLAUDE.md` の表の 1 行だけ（Q4）

行は `| \`docs/adr/\` | Story をまたぐ判断の記録 | する |`。置き場は `docs/` の行の直後（`docs/` の下の特別な置き場なので、親の行の次に読める）。
いつ ADR を書くかの規則の正本は `docs/adr/README.md` の 1 か所で、`AGENTS.md` には書かない。

反転条件: Q4 の答えが「AGENTS.md にも足す」なら、`AGENTS.md` の「実装と計画の規約」に「Story をまたぐ判断は `docs/adr/` に書く（書き方は `docs/adr/README.md`）」の 1 行を足す。

### D8. capability `decision-records` を新しく立て、`skip_specs` は使わない

schema は docs の change に `skip_specs: true` を勧めるが、この repo の change で `skip_specs` を使った前例は無く、Requirement が 0 件の change がグラフ
（`check_scenarios`・`go_brief`・merge gate・archive）を通るかを確かめていない。また、索引と実ファイルの対応・出所の名指しは外から機械で確かめられる性質で、
CH10 の `emulator-startup`・CH09 の `test-timing` と同じく、開発の約束として正典に置ける。

正典に置かないもの（`review/spec.md` R10）: 依頼の逐語との一致（D1）と、`adr_backlog.py` の残りの判定（D4）。どちらも CH12 の受け入れの 1 回の判定で、
後者は手元にしか無いメモリと、harness2 の main に無い台本に依る。どちらも `docs/changes/CH12.md` の完了の判定と tasks の検証にだけ置く。

正典に置くもの（索引と実ファイルの 1 対 1・並び・ファイル名の形・`CLAUDE.md` の表・出所の名指し・網の名前）は、ADR を足すたびに成り立つべき性質。
ただし Q3 の推奨で台本を CI に載せないので、archive の後にこれを毎回確かめるものは無い（ADR を足した人が `tools/test-adr-docs.sh` を走らせるか、
書き忘れが起きたら D2 の反転条件で CI に載せる）。正典は「何が成り立つべきか」を残し、Q3 は「毎回見るかどうか」を決める —— 両者は食い違わない。

### D9. 検証の台本の形

`tools/test-adr-docs.sh`（bash。冒頭のコメントに何を確かめるか・なぜ要るか（CH12）・走らせ方の 4 つの口）:

- 引数なし: 1 つ目と 3 つ目の Requirement と、2 つ目の「出所に名指しがある」を見る。各検査の前に `echo "Scenario: <名前>"` を出す。見る `docs/adr/` は環境変数 `ADR_DIR`（既定 `docs/adr`）、`CLAUDE.md` は `ADR_CLAUDE_MD`（既定 `CLAUDE.md`）。
  `tools/check-private.sh` は repo の根で走らせる（`ADR_DIR` を変えても、check-private は repo の追跡ファイルを見る）。
  check-private は `git ls-files` の範囲しか見ないので、呼ぶ前に `git ls-files docs/adr` が README と 5 件の ADR を返すことを確かめ、返さなければ `error: docs/adr/ がまだ追跡されていない` で落とす（`review/spec.md` R8）
- `--self-test`: `mktemp -d` の使い捨ての木に `docs/adr/` と `CLAUDE.md` を写し、`ADR_DIR` と `ADR_CLAUDE_MD` をそこへ向けて壊す。壊し方は 6 つ（一覧の 1 項目を消す・一覧の 2 項目を入れ替える・ADR の 1 件の見出しを 1 字変える・ADR の 1 件の出所の行を消す・ADR の名前を大文字にする・`CLAUDE.md` の表の行を消す）で、
  どれでも検査が rc≠0 になることを確かめる（`review/spec.md` R7）。`trap` で木を消す。「出所の行が無い ADR は落ちる」の印はここで出す。
  `--backlog` の欠落の 2 つ（`ADR_BACKLOG` を無いパスに・`ADR_MEMORY` を無いフォルダにして、`ADR_BACKLOG` は使い捨ての木に置いた偽の台本に向ける）と、D5 の出力の形もここで見る。
  欠落の 2 つは `error:` の文面で、どちらの枝を通ったかを見る（R6）
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
