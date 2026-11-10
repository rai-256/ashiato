# CH12 — メモリにしか無かった判断を ADR に置く

> 第 1 回の深掘りの答えを待たずに、推奨の答え（`deep.md` の `（答え待ち・推奨: …）`）で先に書いた（2026-10-20 の流れ）。
> 本人の答えが推奨と違えば、`spec --answers` がその問いだけを写す。`docs/changes/CH12.md` の「価値」「壊してはいけないもの」「完了の判定」も、この段で推奨の答えに合わせて書いた。
> 第 1 回の答え（2026-11-08）: Q1・Q2 は推奨のまま、Q3・Q4 は推奨と違う。Q3（ADR の形の検査を CI で毎回見る）・Q4（`AGENTS.md` にも 1 行）を写した。

## Why

ashiato2 に固有の判断のうち 5 件（worktree のポートの割り当て・衛星の基盤が主という前提・純粋な crate の分割・画面の作り直しは表面だけ・lint を一段厳しく）は、
Claude のメモリ（`~/.claude/projects/…/memory/`）にしか無い。メモリは commit されず、ほかの席や Codex からは見えず、消えれば戻らない。
本人の依頼（2026-11-08）「adr は早めに欲しい、失われていくので」に応えて、判断の正本を repo の中に移す。

## What Changes

- `docs/adr/` に README（ADR の書き方と一覧）と ADR 5 件を、`docs/changes/CH12.md` の「依頼（逐語）」の各塊のまま置く（1 字も変えない。design D1）
- `CLAUDE.md` の「文書とハーネス」の表に `docs/adr/` の 1 行を足す（何か: Story をまたぐ判断の記録。書き込み: する）。`AGENTS.md` の「実装と計画の規約」にも「Story をまたぐ判断は `docs/adr/` に書く」1 行の指示を足す（Q4 の本人の答え。D7（仮））
- ADR の形の検査 `tools/check-adr.sh` を 1 本足す。索引と実ファイルの対応・並び・ファイル名の形・見出しの形・出所・`CLAUDE.md` の表・`AGENTS.md` の行・CI の行・`tools/check-private.sh` を見て、`--verbatim` で依頼の逐語との一致を、`--backlog` で harness2 の `adr_backlog.py` の残りを見る。
  **CI の chain job に 1 行で載せ、PR ごとに自己検査と本番を走らせる**（`--verbatim` と `--backlog` は載せない。Q3 の本人の答え。D2（仮））
- ADR の本文と repo の今が食い違う箇所（8 つ）と、ADR が参照する harness2 の ADR がまだ harness2 の main に無いことは、直さずに PR 本文と `docs/changes/CH12.md` の節で挙げる（Q1 の推奨。D3（仮））
- 完了の判定は repo の中のコマンドで確かめ、harness2 の側を待たない。`adr_backlog.py` は手元で、メモリの置き場を明示して走らせ、4 つのメモリ名が残りに無いことだけを見る（Q2 の推奨。D4（仮）・D5）
- **やらないこと**: ADR の本文の手入れ（食い違いの直し・「その後」の追記）、harness2 の ADR 24 件の push や PR、コード・移行・既存の台本の変更、CI の chain job の 1 行のほかの変更

## Capabilities

### New Capabilities

- `decision-records`: Story をまたぐ判断の記録（ADR）を `docs/adr/` に 1 件 1 ファイルで置き、README の一覧と `CLAUDE.md` の表から辿れ、メモリから移した判断は出所にメモリのファイル名を名指しし、網の名前・私設 IP を含まないこと。
  CH03 の `test-hygiene`・CH10 の `emulator-startup` と同じく、製品ではなく開発の約束。
  schema の「docs は `skip_specs`」は採らない: 索引と実ファイルの対応・出所の名指しは外から確かめられる性質で、台本の Scenario の印に縛られ、archive の後も正典が固定する（D8）。
  依頼の逐語との一致と、harness2 の `adr_backlog.py` の残りの判定は、CH12 の受け入れの 1 回だけの判定で、正典には置かない（前者は README が後の「置き換え済み」への書き換えを認めているので常には成り立たず、後者は手元のメモリと harness2 の main に無い台本に依る。D1・D8）。
  change の capability は `docs/stories/INDEX.md` に足さない

### Modified Capabilities

（無い。正典の spec に文書の置き場を扱うものは無い）

## Impact

- 文書: `docs/adr/` に 6 ファイルを足す。`CLAUDE.md` の表に 1 行（`docs/` の行の直後）。`AGENTS.md` の「実装と計画の規約」の導入の段落の直後に 1 行。`docs/changes/CH12.md` の「価値」「壊してはいけないもの」「完了の判定」「ADR と repo の今の食い違い」（この段で書いた。「依頼（逐語）」の節は変えない）
- 道具・CI: `tools/check-adr.sh` を足す（新しいファイル）。`.github/workflows/ci.yml` の chain job に 1 行（`./tools/check-private.sh` の直後）。既存の `tools/` の台本と、ci.yml のほかの段・ほかの job は変えない。
  後から ADR を足す PR は、README の形と一覧に合わないと CI で落ちる（Q3 の意図）
- 製品のコード・API・移行・画面: 変えない。依存も足さない
- 外に出るもの: 本文は既に公開の origin にある（`premises.md` の 1 行目）。新しく出るのは `CLAUDE.md`・`AGENTS.md`・ci.yml の 1 行ずつと台本だけ。検証の出力と PR 本文には、公開されていないメモリの名前と説明を出さない（D5）
- harness2 との関係: README と 3 つの ADR が harness2 の `docs/adr/`（題名で 3 件）を参照するが、harness2 の ADR 24 件と `scripts/adr_backlog.py` は harness2 の main に無く、push も PR も無い（`premises.md` の 2 行目）。この change はそれを待たない（D4（仮））。参照の行き先がまだ無いことは PR 本文に書く
- **走っている Story・change との重なり**（`origin/main` = 0adef6712 との差分）: st53・st56・ch10 と、`feat/st*`・`feat/ch*` の枝はどれも `CLAUDE.md`・`docs/adr/`・`tools/check-adr.sh` に触らない（`git diff --name-only origin/main...<枝>` で確かめた）。
  `AGENTS.md` は ch10（`feat/ch10-emulator-start-timeout`）が `/dev/kvm` の節に 1 行を足し、この change は「実装と計画の規約」の導入に 1 行を足す。`.github/workflows/ci.yml` は ch10 が rust job と st55 の段を、st56 が rust job と android の job を変え、この change は chain job に 1 行を足す。
  どれも触る節が違い、同じ行を取り合わない。gate が main に rebase するので、後に入る側が追従する。走っている Story へ差し戻すものは無い
