# CH12 — メモリにしか無かった判断を ADR に置く

> 第 1 回の深掘りの答えを待たずに、推奨の答え（`deep.md` の `（答え待ち・推奨: …）`）で先に書いた（2026-10-20 の流れ）。
> 本人の答えが推奨と違えば、`spec --answers` がその問いだけを写す。`docs/changes/CH12.md` の「価値」「壊してはいけないもの」「完了の判定」も、この段で推奨の答えに合わせて書いた。

## Why

ashiato2 に固有の判断のうち 5 件（worktree のポートの割り当て・衛星の基盤が主という前提・純粋な crate の分割・画面の作り直しは表面だけ・lint を一段厳しく）は、
Claude のメモリ（`~/.claude/projects/…/memory/`）にしか無い。メモリは commit されず、ほかの席や Codex からは見えず、消えれば戻らない。
本人の依頼（2026-11-08）「adr は早めに欲しい、失われていくので」に応えて、判断の正本を repo の中に移す。

## What Changes

- `docs/adr/` に README（ADR の書き方と一覧）と ADR 5 件を、`docs/changes/CH12.md` の「依頼（逐語）」の各塊のまま置く（1 字も変えない。design D1）
- `CLAUDE.md` の「文書とハーネス」の表に `docs/adr/` の 1 行を足す（何か: Story をまたぐ判断の記録。書き込み: する）。`AGENTS.md` は変えない（Q4 の推奨。D7（仮））
- 検証の台本 `tools/test-adr-docs.sh` を 1 本足す。索引と実ファイルの対応・出所・`CLAUDE.md` の表・`tools/check-private.sh` を見て、`--verbatim` で依頼の逐語との一致を、`--backlog` で harness2 の `adr_backlog.py` の残りを見る。**CI には載せない**（Q3 の推奨。D2（仮））
- ADR の本文と repo の今が食い違う箇所（8 つ）と、ADR が参照する harness2 の ADR がまだ harness2 の main に無いことは、直さずに PR 本文で挙げる（Q1 の推奨。D3（仮））
- 完了の判定は repo の中のコマンドで確かめ、harness2 の側を待たない。`adr_backlog.py` は手元で、メモリの置き場を明示して走らせ、4 つのメモリ名が残りに無いことだけを見る（Q2 の推奨。D4（仮）・D5）
- **やらないこと**: ADR の本文の手入れ（食い違いの直し・「その後」の追記）、ADR の形を CI で毎回見る検査、agent への「ADR を書け」の指示、harness2 の ADR 24 件の push や PR、コード・移行・既存の台本・CI の変更

## Capabilities

### New Capabilities

- `decision-records`: Story をまたぐ判断の記録（ADR）を `docs/adr/` に 1 件 1 ファイルで置き、README の一覧と `CLAUDE.md` の表から辿れ、メモリから移した判断は出所にメモリのファイル名を名指しし、網の名前・私設 IP を含まないこと。
  CH03 の `test-hygiene`・CH10 の `emulator-startup` と同じく、製品ではなく開発の約束。
  schema の「docs は `skip_specs`」は採らない: 索引と実ファイルの対応・出所の名指しは外から確かめられる性質で、台本の Scenario の印に縛られ、archive の後も正典が固定する（D8）。
  依頼の逐語との一致は CH12 の受け入れの 1 回だけの判定で、正典には置かない（README が後の「置き換え済み」への書き換えを認めているので、常に成り立つ性質ではない。D1）。
  change の capability は `docs/stories/INDEX.md` に足さない

### Modified Capabilities

（無い。正典の spec に文書の置き場を扱うものは無い）

## Impact

- 文書: `docs/adr/` に 6 ファイルを足す。`CLAUDE.md` の表に 1 行（`docs/` の行の直後）。`docs/changes/CH12.md` の「価値」「壊してはいけないもの」「完了の判定」（この段で書いた。「依頼（逐語）」の節は変えない）
- 道具: `tools/test-adr-docs.sh` を足す（新しいファイル）。既存の `tools/` の台本と `.github/workflows/` は変えない
- 製品のコード・API・移行・画面: 変えない。依存も足さない
- 外に出るもの: 本文は既に公開の origin にある（`premises.md` の 1 行目）。新しく出るのは `CLAUDE.md` の 1 行と台本だけ。検証の出力と PR 本文には、公開されていないメモリの名前と説明を出さない（D5）
- harness2 との関係: README と 3 つの ADR が harness2 の `docs/adr/`（題名で 3 件）を参照するが、harness2 の ADR 24 件と `scripts/adr_backlog.py` は harness2 の main に無く、push も PR も無い（`premises.md` の 2 行目）。この change はそれを待たない（D4（仮））。参照の行き先がまだ無いことは PR 本文に書く
- **走っている Story・change との重なり**（`origin/main` = 0adef6712 との差分）: st53・st56・ch10 と、`feat/st*`・`feat/ch*` の枝はどれも `CLAUDE.md`・`docs/adr/`・`tools/test-adr-docs.sh` に触らない（`git diff --name-only origin/main...<枝>` で確かめた）。ch10 は `AGENTS.md` を触るが、この change は `AGENTS.md` に触らない。走っている Story へ差し戻すものは無い
