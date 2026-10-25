# ST29 code review（whole-branch）

## final review（4ee5f44..c940f70）

席: final reviewer（`superpowers:requesting-code-review` の `code-reviewer.md`）。入力は review package
`.superpowers/sdd/tasks/review-4ee5f442..c940f703.diff`、Task 1〜10 の指摘の記録の Minor / ⚠️、Task 7 の ledger の parked（F1）、`docs/handoff/ST29.md`。
判定: **With fixes**。D8（衛星の判定は最も外側で 1 回）・D6（値は応答に 1 回・保存は SHA-256 だけ）・D4 / D9（読み出しの記録は uuid だけ、CHECK と二重）・D14（全 route の試験は契約から取る）は design どおり。
申し送りの st17 C2・st33 C4（`/feelings` `/feelings/reminder` `/export*` を衛星の資格情報で断る）は `every_route_refuses_satellite` で閉じている。
parked の Task 7 F1（`export_ledger_migration_applies_twice`）は、ブランチの `export_ledger_tests.rs` の変更が main の df75d2c1 と同じ字面で、`git merge-tree` でも衝突が無い —— このブランチで追加の対応は要らない。
main との統合で衝突するのは `crates/server/src/lib.rs`（use と mod の塊）と `docs/openapi.json`（`src/bin/openapi.rs` で作り直す）だけ。

## R1. main と統合すると全 route の試験（この Story の完了の判定）が HEAD の口で落ちる
- 成果物: crates/server/src/satellite_routes_tests.rs:57-66（`contract_operations`）/ :157（本文の比較）
- 根拠: origin/main の `docs/openapi.json` の `/media/originals/{sha256}`（ST09）は `['put', 'head']`（`git show origin/main:docs/openapi.json` を python で確かめた）。`contract_operations()` は get/post/put/delete/patch 以外で `panic!("知らない method …")`。HEAD は本文が削られるので `body_string == "forbidden"` も落ちる（Task 6 F1 が一般論で指していたもの）
- kind: technical
- 提案: `head`（と `options`）を足し、path の項目の method でない key（`parameters` `summary`）は飛ばす。HEAD は状態の番号だけを見る
- 処置: fixed 6.1 — a3170df6。`head` / `options` を足し、method でない key は `continue`、`trace` だけ panic。`request()` は HEAD / OPTIONS に本文を付けない。HEAD は本文を見ない。`cargo test -p ashiato-server satellite_` → 42 passed。HEAD の分岐はこのブランチの契約に head の口が無いので、統合した木で初めて通る（PR 本文に挙げる）

## R2. 書き出しの `SATELLITE_COLUMNS` の `id` の説明が「衛星の識別子」で `identifier` と紛らわしい
- 成果物: crates/server/src/export.rs:327
- 根拠: 同じ表の `identifier` の説明が「衛星の識別子の文字列（逆ドメイン形）」。Task 7 F2
- kind: technical
- 処置: fixed 7.1 — a3170df6。「衛星の番号（uuid）」に。`export_satellite_tables_and_columns_are_classified` は緑

## Minor（merge 前に直さない。理由つきで ledger に park）

final reviewer が Minor とし、merge を止めないとしたもの。SDD の Final Review の「fix は 1 回だけ」に従い、R1・R2 の fix に入れなかった。
- `web/src/SatellitesPanel.tsx` の `revoke` が `busy` を立てない（Task 9 F3）。2 回目の DELETE も COALESCE で 204 —— 実害は小さい
- `SatellitesPanel.tsx` の確認の「やめる」が `destructiveStyle` を流用（Task 9 F4）。44 px のための流用で見た目の問題だけ
- `web_session.rs` の `session_get` が 403 から衛星と推して `decided` を読み直す（Task 3 F1）。いまの 403 は衛星だけ。衛星以外の 403 を足す Story で `decided` を先に match する形に
- `access_log.rs` の `AccessEntry.satellite` が `pub`（「`for_caller` からだけ埋まる」は DB の CHECK `access_log_satellite_ids` 頼み）
- `satellite_form_tests.rs` の Rust 側の突き合わせは識別子だけ。名前の上限 40 と Rust の定数の一致は web 側の試験（`satellites.test.tsx:187`）だけ（Task 2 F2）
- 移行の access_log の CHECK を当てるたびに DROP / ADD（D9 の指定どおり。表が大きくなると起動が遅くなる。Task 1 F4）
- `tools/check-immutable.d/202610241210_st29-satellite-credentials.sh` (b) の行の組み合わせが読み手に紛らわしい（Task 1 F3）。検査は通る
- `web/e2e/settings-export.spec.ts:144` の題が「最初の節は「全記録の書き出し」」のまま（中身は agreed-test-changes.md のとおり直してある。印の `// Scenario:` は正しい）
- `docs/handoff/ST29.md` の st28 Q4 に「ST44 へ移した」の追記が無い（中身は `docs/stories/ST44.md:14` にある）—— finish の PR 本文の「申し送り」で触れる
- `POST /satellites` で資格情報が無く本文が壊れていると 401 でなく 422（Task 4 F1。lib.rs の既存の口と同じ形）
- `tools/check-satellite-boundary.sh` のループ変数 `f` が `local` でない・self-test が `forbidden_in_deps` だけ（Task 8 F1・F2）

## fix（c940f70..a3170df）の scoped re-review

席: re-reviewer（SDD の `re-review-prompt.md`）。R1・R2 とも ADDRESSED、fix の diff に新たな Critical / Important は無し。
範囲外の所見: DB を新しく作った直後の 1 回目の `cargo test -p ashiato-server satellite_` で `satellite_routes_tests` の 2 本が `40P01 deadlock detected`
（`seed_credential` の INSERT と、別の接続が移行で `satellite_credential` に AccessExclusive を取りに来た。移行は `DROP TRIGGER IF EXISTS` を当てるたびに走らせる）。
続く 2 回は緑。同じプロセスの移行は `testdb::pool()` の OnceCell と助言ロックで直列なので、相手の接続は特定できていない —— CI の cold start で再現したら `ci_fix` で拾う。
