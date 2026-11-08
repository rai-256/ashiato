# CH12 — メモリにしか無かった判断を ADR に置く（上流）

> 第 1 回の深掘りは**推奨の答えで先に**書いた（2026-10-20 の流れ）。本人の答えはまだ無い。推奨と違う答えが返ったら、`spec --answers` がその問いだけを写す。

## 何をするか

メモリ（commit されない）にしか無かった ashiato2 に固有の判断 5 件を、`docs/adr/` に ADR として置き、README で書き方と一覧を示す。
本文は依頼（`docs/changes/CH12.md` の「依頼（逐語）」）のまま 1 字も変えない。`CLAUDE.md` の「文書とハーネス」の表に `docs/adr/` の 1 行を足す。
確かめるための台本 `tools/test-adr-docs.sh` を 1 本足す（CI には載せない）。

## 深掘り（第 1 回。4 問とも B・推奨で先に書いた）

- Q1 ADR の本文と repo の今の食い違い → 推奨: 逐語のまま置き、PR 本文で挙げる（D3（仮））。一覧は下
- Q2 完了の判定の確かめ方（harness2 の `adr_backlog.py` が main に無い）→ 推奨: repo の中のコマンドで確かめ、待たない。`adr_backlog.py` は手元で、メモリの置き場を明示し、4 つのメモリ名が残りに無いことで見る（D4（仮））
- Q3 ADR の形を CI で毎回見る検査 → 推奨: 足さない（D2（仮））
- Q4 agent への案内 → 推奨: `CLAUDE.md` の表の 1 行だけ（D7（仮））

A（止める問い）は無い。

## 仮決め（design.md の（仮））

- D2（仮）・D3（仮）・D4（仮）・D7（仮）: 上の Q3・Q1・Q2・Q4 の推奨。反転条件は design.md の各 D

聞かずに決めた既定（`premises.md` の C1〜C4 → design の D1・D2・D5・D6）と、AI の既定 D8（capability を立て `skip_specs` を使わない）・D9（台本の形）。

## ADR の本文と repo の今が食い違う箇所（直していない。依頼どおり）

1. 衛星の ADR の「影響」は「要件の書き直しは未着手」と書くが、要件の目的は 2026-10-11 に ★ で書き直し済み（`docs/requirements.md:15-21`）。「接続は未決」も、FR-59 の ★ 2026-10-11 で「いったん MCP」（`docs/requirements.md:804-805`）。ADR は「メモリの記録の時点で」と断っている
2. 衛星の ADR の「進め方 1. 監査」は済んでいる（`docs/audit/2026-10-09-satellite-foundation.md`）
3. 表面の ADR の後、表面は 2026-10-12 に作り直された（`docs/ui-direction.md` の ★ 2026-10-12。骨格は変えていない）
4. lint の ADR は 1.99 の規則を足した日を 2026-10-12 と書くが、`Cargo.toml` の注記は「2026-10-11 実測」（足した commit の日付は 2026-10-12）
5. lint の ADR「Rust: pedantic」に対して、`crates/winsvc/Cargo.toml` に `[lints] workspace = true` が無い（pedantic も `unsafe_code = "forbid"` も受けていない）
6. lint・純粋 crate の ADR の「ST12・ST21（・ST08）の merge 後に行う」の条件は満たされている（3 本とも archive 済み）が、`Cargo.toml` の「まだ直していないだけ」の表は残っている。lint の ADR は今の状態を書いていない
7. ポートの ADR「規則はここだけに置く」に対して、`crates/installer/src/commands.rs:107-117` が開発用の番号の範囲を写しで持っている
8. ポートの ADR「割り当てられなければ非 0、55432 には落とさない」に対して、`tools/ports.sh` は git が答えないと 55432 に落ちる（`GIT_DIR=/nonexistent` で 55432・rc=0）

加えて、README と 3 つの ADR（ポート・純粋 crate・lint）が参照する harness2 の `docs/adr/` は、まだ harness2 の main に無い（push も PR も無い）。題名での参照なので、リンク切れにはならないが、harness2 の側が入るまで行き先が無い。

## capability

- 新しい capability `decision-records`（開発の約束。`test-hygiene`・`emulator-startup` と同じ扱い）。前倒しは無い。MODIFIED は無い
- 依頼の逐語との一致は CH12 の受け入れの 1 回の判定で、正典に置かない（D1）

## tasks

4 Task・5 項目。Scenario は 11 本（どれも `tools/test-adr-docs.sh` が印を出す）。`--backlog`（完了の判定の 4.）は手元のメモリと harness2 の台本が要り、CI では走らない。無ければ環境の欠落で止まる（空の合格にしない）。

## レビュー

- 深掘りの問いの独立検査（`review/deep.md`）: 6 件。処置はすべて fixed と書かれているが、R4（Q1 を「食い違いの一覧をどこへ残すか」に置き換える）は `deep-questions.json` に写っていない（問いは置き換える前のまま。R3 の「ほかは一致」の文も Q1 の context に残る）。この段は問いを書き換えない。食い違いの一覧は premises の 8 つ全部を上に挙げた
- spec-review（`review/spec.md`）: 下の結果を参照

🤖 Generated with [Claude Code](https://claude.com/claude-code)
