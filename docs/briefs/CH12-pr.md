# CH12 — メモリにしか無かった判断を ADR に置く（上流）

> 第 1 回の深掘りは**推奨の答えで先に**書いた（2026-10-20 の流れ）。本人の答え（2026-11-08）は Q1・Q2 が推奨のまま、Q3・Q4 が推奨と違い、`spec --answers Q3,Q4` で写した。

## 何をするか

メモリ（commit されない）にしか無かった ashiato2 に固有の判断 5 件を、`docs/adr/` に ADR として置き、README で書き方と一覧を示す。
本文は依頼（`docs/changes/CH12.md` の「依頼（逐語）」）のまま 1 字も変えない。`CLAUDE.md` の「文書とハーネス」の表に `docs/adr/` の 1 行を足す。
`AGENTS.md` の「実装と計画の規約」にも「Story をまたぐ判断は `docs/adr/` に書く」1 行の指示を足す（Q4）。
ADR の形の検査 `tools/check-adr.sh` を 1 本足し、CI の chain job の 1 行で PR ごとに自己検査と本番を走らせる（Q3）。

## 深掘り（第 1 回。4 問とも B）

- Q1 ADR の本文と repo の今の食い違い → (推奨のまま) 逐語のまま置き、PR 本文と `docs/changes/CH12.md` の節で挙げる（D3（仮））。一覧は下
- Q2 完了の判定の確かめ方（harness2 の `adr_backlog.py` が main に無い）→ (推奨のまま) repo の中のコマンドで確かめ、待たない。`adr_backlog.py` は手元で、メモリの置き場を明示し、4 つのメモリ名が残りに無いことで見る（D4（仮））
- Q3 ADR の形を CI で毎回見る検査 → **本人の選択（推奨と違う）: 足す**（`tools/check-adr.sh` と CI の chain job の 1 行。D2（仮））。見出しの形・`AGENTS.md` の行・CI の行の Scenario と、見出しの欠けた ADR が落ちる自己検査を足した
- Q4 agent への案内 → **本人の選択（推奨と違う）: `AGENTS.md` にも 1 行の指示を足す**（D7（仮））

A（止める問い）は無い。

## 仮決め（design.md の（仮））

- D2（仮）・D7（仮）: 上の Q3・Q4 の本人の答え。D3（仮）・D4（仮）: Q1・Q2 の推奨のまま。反転条件は design.md の各 D
- D5（仮）: 検証の出力に、公開されていないメモリの名前と説明を出さない（4 つのメモリ名の有無だけ）。失わない側を選んだ仮として台帳 `provisional.jsonl` に載せた（`review/deep.md` R5 の `loss: exported`）

聞かずに決めた既定（`premises.md` の C1〜C4 → design の D1・D2・D5・D6）と、AI の既定 D8（capability を立て `skip_specs` を使わない）・D9（台本の形）。

## ADR の本文と repo の今が食い違う箇所（直していない。依頼どおり）

同じ一覧を `docs/changes/CH12.md` の `## ADR と repo の今の食い違い（直していない）` にも置いた（PR 本文は merge の後に流れていくため）。

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
- 依頼の逐語との一致と `adr_backlog.py` の残りの判定は CH12 の受け入れの 1 回の判定で、正典に置かない（D1・D8）

## tasks

4 Task・5 項目。Scenario は 13 本（どれも `tools/check-adr.sh` が印を出す。引数なし 11 本・`--self-test` 2 本）。`--backlog`（完了の判定の 4.）は手元のメモリと harness2 の台本が要り、CI では走らない。無ければ環境の欠落で止まる（空の合格にしない）。ADR を渡さない対照で 4 つとも数えられることを先に確かめる。

## レビュー

- 深掘りの問いの独立検査（`review/deep.md`）: 6 件。R3・R4 は「fixed」と書かれていたが `deep-questions.json` に写っていなかった（Q1 の context は「ほかは repo と一致した」のまま）。
  答えがまだ 0 件なので、spec の段で Q1 の context を 8 つに直し、推奨の文面を「PR 本文と CH12.md の節で挙げる」に合わせた（問いの向きと推奨の意味は変えていない）。処置の形も機械の受ける形に直した
- spec-review（`review/spec.md`）: 12 件（technical 11・defer 1）。全部 fixed（仮 6 件）。人間に返すものは無い。
  R2 は reviewer が premise としたが、決定の前提が崩れたのではなく前の段の写し漏れなので technical に読み替えて直した。
  主な直し: `--backlog` が中身を見ずに通る経路を塞いだ（正の対照・rc と JSON の形・環境変数の指す先が無ければ落ちる・出力の全行の形）、`--backlog` を正典から外した、逐語の比べる元を登録の commit に固定した、自己検査の壊し方を 6 つにした
- 答えの写し（`review/answers.md`）: 下記
- 走っている Story との重なり: `CLAUDE.md`・`docs/adr/`・`tools/check-adr.sh` を触る枝は無い。`AGENTS.md` は ch10（`/dev/kvm` の節）、`.github/workflows/ci.yml` は ch10・st56（rust・android の job）も触るが、触る節が違い同じ行を取り合わない

🤖 Generated with [Claude Code](https://claude.com/claude-code)
