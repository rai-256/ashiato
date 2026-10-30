# ST51 code review

## final review（f0b45f3..3331639）

final reviewer（`superpowers:requesting-code-review` の `code-reviewer.md`）の指摘。Critical 0 / Important 2 / Minor 13。
reviewer が手元で走らせたもの: `cargo test -p ashiato-server --lib` 1760 passed / 0 failed、`:app:testDebugUnitTest`（St49OutboxTest 7 本・St51HeartbeatVersionTest 3 本）BUILD SUCCESSFUL、
`openspec validate st51-update-history --strict` valid、`HX_CHANGE=st51-update-history python3 scripts/check_scenarios.py .` rc=0。
fix は 1 回（9dac25a..05bb619。指摘の外に `cargo fmt --check` を通す整形 4941dd8 を 1 つ）、scoped re-review 1 回で R1〜R4・R6〜R13 は全部 ADDRESSED、新しい Critical / Important なし。R5・R14・R15 は直さない（理由は各 R）。
Task ごとの ⚠️ は final reviewer が全体で確かめ、全部 VERIFIED-OK（Task 9 の D14: `coverage*.rs` / `outage*.rs` に `collector_version` / `update_history` の語は 0 件、
Task 8 の ST49 の Scenario: ADDED（改名）+ REMOVED で外し ST49 の試験も消してある、Task 4 の `St51HeartbeatVersionTest` は実在し送った body の `client_version` を見る、
Task 2 の失敗の作り方と `record_startup` の配線（`lib.rs:4201`）、Task 1/6 のビューの試験、印の名前）。
並走の change で `device-collection` / `desktop-collection` を触るのは st50-contract-version だけで、改名した Requirement には触れない（grep で確認）。

## R1. `st51_coverage_outage_still_shown` が Scenario の GIVEN（途切れの間に更新の履歴の行と版の変化がある）を作れていない
- 成果物: crates/server/src/version_change_tests.rs
- 根拠: version_change_tests.rs:717 付近 —— `core.update_history` の行を `at` を指定せずに INSERT（`DEFAULT now()` = 実時刻で途切れの区間の外）、版の変化も区間の始まりの点 / Task 9 F1（未修正）
- kind: technical
- 処置: fixed 9.1 —— 更新の履歴の行の at を 2026-10-19T06:00Z に明示し、版の変化は同じ端末の別の C-01 ソースの 10-19 12:00 の v2 で区間の中に作る（位置のソースを区間に置くと途切れが終わるため）。区間の中にあることと last_alive_at を assert（ed415bd8）

## R2. `heartbeatKeepsVersionFromAddTime` の「入れ替わった後の送り手」が v2 を使っておらず、`assertTrue(v2 != v1)` が何も確かめていない
- 成果物: collector-android/app/src/test/kotlin/dev/ashiato/collector/St51HeartbeatVersionTest.kt
- 根拠: St51HeartbeatVersionTest.kt:244 付近 `Outbox.inDir(dir, HeartbeatRequest.serializer()) {}` / Task 8 F1（未修正）
- kind: technical
- 処置: fixed 8.2 —— 入れ替わった後の置き場を `Outbox.inDir(dir, …, v2)` にし、送った body が v1 のままを見る。空の assert は削除。読み出し時に v2 を付ける形に壊すと落ちることを確認（872a4d59）

## R3. ST58 への申し送りの「表がまだ無い回は試みも結果も書かず警告だけ」が実装と合わない（実装は表を作った後に結果の行を書く）
- 成果物: docs/handoff/ST58.md
- 根拠: docs/handoff/ST58.md:27 / `st51_bin_migrate_writes_compiled_stamp`（applied の 1 行）/ spec の Scenario「警告を出し、結果の 1 行だけを残す」
- kind: technical
- 処置: fixed 10.1 —— 初回は試みの行を書かず警告、表を作った後は結果の行（applied / failed、from は空）を書き、その回の退避には表が無いので戻すと消える、に書き直した（e1f56c53）

## R4. 警告の種別 `update_history_unwritten` が 2 か所に直書きで、試験は種別でなく `#[cfg(test)]` のスレッドローカルの理由の語を見ている
- 成果物: crates/server/src/update_history.rs
- 根拠: update_history.rs:470 / lib.rs:4201（`pool.acquire()` 失敗の分岐は `why` も持たない）/ Task 2 F1
- kind: technical
- 処置: fixed 2.1 —— `update_history::UNWRITTEN_KIND` に集約し、lib.rs の acquire 失敗も `warn_unwritten("acquire", …)` を通す。試験が種別そのものを取り込む形は並走の試験との取り合いで見送り、種別を取り違える経路は構造でなくした（6d0e12c8）

## R5. migrate の失敗の 1 行を `tracing::error!` と `eprintln!` の両方に出している
- 成果物: crates/server/src/lib.rs
- 根拠: lib.rs:126-129 / Task 2 F2
- kind: technical
- 処置: rejected: 重複ではない。`run_migrate` の `tracing_subscriber::fmt()` は writer 未指定で標準出力、`eprintln!` は標準エラーで、それぞれ 1 行。ST48 の `mismatch_report` と同じ形（final reviewer が確認、re-review も R8 の判定で同じ読み）

## R6. design D15 の関数の形（`record_migrate(conn, stamp, outcome)`・`record_startup(conn, stamp)`）と実装（`attempt`・`versions` の引数、`apply_migrations_recorded` の新設）がずれている
- 成果物: openspec/changes/st51-update-history/design.md
- 根拠: design.md D15 / update_history.rs:637 / Task 2 F3
- kind: technical
- 処置: fixed D15 —— 決定は変えず、`record_attempt` / `record_migrate(…, attempt, outcome)` / `record_startup(…, versions)` / `apply_migrations_recorded(…, stamp: Option<&str>)` の形と理由（錠を持つ同じ接続で書く）を書いた（9bb9e1ee）

## R7. Rust の語「`None`」が契約の説明文（docs/openapi.json）に漏れている
- 成果物: crates/server/src/heartbeat.rs
- 根拠: heartbeat.rs:33・247 → docs/openapi.json:3591 / Task 4 F2
- kind: technical
- 処置: fixed 4.1 —— 「欄の無い要求は『不明』」に直し docs/openapi.json を再生成。`./tools/check-openapi.sh` rc=0（81cf6c12）。ST09 由来の `network` の同じ書き方は範囲外

## R8. ST48 の判定の緩め方が粗い（`kind=` の行が `error: kind=migrate_failed ` で始まれば、後ろに `db_ahead` などが付いても通る）
- 成果物: crates/server/tests/server_startup.rs
- 根拠: server_startup.rs:817-823 / Task 3 F2
- kind: technical
- 処置: fixed 2.3 —— 食い違いの種別が標準エラーに無いこと・`kind=` の行が 1 本・語の並びが `failure_line` の形に限られることを見る（43edf7dd）

## R9. `st51_bin_startup_row_on_swap` が `h.len() == 3` と前置きの中身に決め打ちしている
- 成果物: crates/server/tests/server_startup.rs
- 根拠: server_startup.rs:1306 / Task 3 F1
- kind: technical
- 処置: fixed 3.2 —— 起動の前の行を取り、`before + 1` と前の行が変わらないことで比べる（43edf7dd）

## R10. `st51_coverage_same_with_or_without_version` の `/outages` が空どうしの比較で、途切れの判定の側を見ていない
- 成果物: crates/server/src/version_change_tests.rs
- 根拠: version_change_tests.rs:690・729 / Task 9 F2・F3
- kind: technical
- 処置: fixed 9.1 —— 入力を 10-19 03:00 / 09:00 にして 27 時間の途切れを出し、位置のソースの途切れ 1 件と last_alive_at 09:00 を見る。版の有無の比較はそのまま（ed415bd8）

## R11. `segmentOutbox` の KDoc「stamp を渡した置き場だけが版を書く」が古い（生存信号も emitter を通して版を持つ）
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/LocationService.kt
- 根拠: LocationService.kt:324 / Task 8 F2
- kind: technical
- 処置: fixed 8.1 —— 生存信号の置き場は stamp を渡さず `HeartbeatEmitter` が積むときに版を入れる、を KDoc に足した（167d31f1）

## R12. `file_note` がファイル名の文字列で分岐し、`VIEW_FILES` / `CATALOG` と名前を二重に持つ
- 成果物: crates/server/src/export/doc.rs
- 根拠: export/doc.rs:54 / Task 6 F1
- kind: technical
- 処置: fixed 6.1 —— `export::UPDATE_HISTORY_FILE` / `COLLECTOR_VERSION_CHANGES_FILE` を置き、CATALOG・VIEW_FILES・`file_note`・dump.rs の並べ方が同じ定数を引く。説明の文を項目の欄へ移す形は CATALOG の 17 項目すべてに欄が要り並走の Story と衝突するので見送り（05bb6190）

## R13. 所有者の TRUNCATE が「何かの DB の誤り」しか見ず、アプリの役割の INSERT は件数の増加だけで足した行を読み戻していない
- 成果物: crates/server/src/update_history_tests.rs
- 根拠: update_history_tests.rs:257・292 / Task 1 F1・F2
- kind: technical
- 処置: fixed 1.2 —— 所有者の TRUNCATE は門の文言と P0001、アプリの役割は 42501 を見る。INSERT は `RETURNING seq` の行をアプリの役割で読み戻す（46aaf094）

## R14. 同じ印で版の集合だけが違う組み立て（`.dirty` の開発ビルドで移行を足した場合）は行を残さない
- 成果物: crates/server/src/update_history.rs
- 根拠: design.md D16（試みの行も結果の行も「版が違うとき」だけ）
- kind: technical
- 処置: rejected: design D16（試みも結果も「版が違うとき」だけ書く）どおりの帰結で、同じ印で版の集合だけ違う組み立ては `.dirty` の開発ビルドでしか起きない。記録にとどめる

## R15. agreed-test-changes.md に ST49 の試験の削除と改名（`heartbeatHasNoVersion` → `recordOutboxCarriesStamp`、`st49_c02_heartbeat_has_no_version` の削除）が無い
- 成果物: openspec/changes/st51-update-history/agreed-test-changes.md
- 根拠: tasks.md の Global Constraints（ST49 の試験を消して置き換える）/ git diff の削除
- kind: technical
- 処置: rejected: 一覧に書かなくても通る。ST49 の Scenario は change の spec の `## REMOVED Requirements` で名指しされており、`check_agreed_tests.py` はそれを合意した変更として通す（agreed-test-changes.md は合意の外の書き換えのための一覧）

## code-verify（HEAD aaa4906f。独立検証）

席: code-verify（申告と実態のずれ）。**tasks.md の全項目の検証コマンドを HEAD で走らせ直した**（2026-10-30 11:10〜。台本 `/tmp/st51cv/run.sh`、結果 `/tmp/st51cv/log/`）。
変異と探りは作業ツリーの外の複製（`/tmp/st51cv/ashiato2-st51`。HEAD を rsync した独立の git、`CARGO_TARGET_DIR` も別）で当て、作業ツリーのコードは触っていない（前後の `git status --porcelain` と `git diff` が一致）。
複製の cargo は `ASHIATO_DB_PORT=55551`・`COMPOSE_PROJECT_NAME=ashiato2-st51`（この木の db）に繋いだ。`tools/check-immutable.sh`（終わりに `docker compose down -v`）は、この木の DB を消さないよう、複製の別の compose project（`st51cv-imm`・port 55560・別の subnet）でだけ走らせた。
`plan_fix.py pending` → `[]`、`plan_fix.py check` → rc=0。`git merge-tree --write-tree origin/main HEAD`（origin/main de2b737b。HEAD より 11 commit 先）→ rc=0（衝突なし）。

### 申告: 20/20（`- [x]` 20・`- [ ]` 0）・検証コマンド一覧

`CT <絞り>` = `bash -c 'set -a; . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server <絞り> && test "$(… -- --list | grep -c ": test$")" -ge <下限>'`（tasks.md の本文どおり）。本数は quiet-run の全文ログの `test result` から写した。

| 項目 | 検証コマンド（tasks.md の本文どおり） | 実測（HEAD aaa4906f） | 一致 |
|---|---|---|---|
| 1.1 | `check-migrations.sh` / `cargo build -p ashiato-server` | rc=0 / rc=0 | 一致 |
| 1.2 | `CT st51_form_ (≥3)` / `CT st51_rows_ (≥2)` / `CT app_role_tests (≥1)` | 3 passed・list 3 / 2 passed・list 2 / 3 passed | 一致 |
| 1.3 | `check-immutable.sh` / `check-shared-appends.sh` | rc=0（複製・`書き換え禁止 OK`、ST51 の節の OK 6 行）/ rc=0（`共有の本体 OK`） | 一致 |
| 2.1 | build / `CT st51_migrate_ (≥19)` | rc=0 / 19 passed・list 19 | 一致 |
| 2.2 | `CT st51_migrate_ (≥19)` / `CT migration_ledger_ (≥20)` | 19 passed / 23 passed・list 23 | 一致 |
| 2.3 | `CT --test server_startup st51_bin_migrate_ (≥5)` | 5 passed・list 5 | 一致 |
| 3.1 | build / `CT --test server_startup st51_bin_startup_ (≥3)` | rc=0 / 3 passed・list 3 | 一致 |
| 3.2 | 同上 / `CT --test server_startup st48_ (≥10)` | 3 passed / 24 passed・list 24 | 一致 |
| 4.1 | `check-openapi.sh` / `jq -e … has("client_version")` / `CT st51_hb_ (≥7)` | rc=0 / rc=0 / 7 passed・list 7 | 一致 |
| 4.2 | `CT st51_hb_ (≥7)` | 7 passed | 一致 |
| 5.1 | `CT st51_view_ (≥13)` | 14 passed・list 14 | 一致 |
| 6.1 | `CT st51_export_ (≥5)` / `CT export_ (≥130)` | 5 passed / 191 passed・list 191（114.6 秒） | 一致 |
| 6.2 | `CT st51_export_ (≥5)` / `CT export_file_excludes_ledgers (≥1)` | 5 passed / 1 passed | 一致 |
| 7.1 | `cargo test -p ashiato-collector-windows --lib st51_c02_ (≥4)` / `--lib` | 4 passed・list 4 / 291 passed | 一致 |
| 7.2 | `--lib st51_c02_ (≥4)` | 4 passed | 一致 |
| 8.1 | gradle `*St51HeartbeatVersionTest*`（XML の testcase ≥3）/ `*Heartbeat*`（≥20） | BUILD SUCCESSFUL・3 / BUILD SUCCESSFUL・32 | 一致 |
| 8.2 | gradle `*St51HeartbeatVersionTest*`（≥3） | 同上 | 一致 |
| 9.1 | `CT st51_coverage_ (≥2)` / `CT outage_ (≥40)` | 2 passed / 60 passed・list 60 | 一致 |
| 10.1 | `check_chain.py .` | rc=0（`chain: OK (0 件 / 未回収 0 件 / warn 0 件)`） | 一致 |
| 10.2 | `check_scenarios.py . st51-update-history` / `check-migrations.sh` / `check-shared-appends.sh` | rc=0（`scenarios: OK`。Scenario 2206 件 / 担保あり 2205 / 人間の確認待ち 1 —— 待ちの 1 件は ST51 の外）/ rc=0 / rc=0 | 一致 |

**手 1（固定値の再計算）: 該当なし（一致）。** `st51_hb_hash_is_pinned` の `86330ea5…061df` を python の `hashlib.sha256` で、`content_hash` の作り（長さ 8 バイト big-endian + 中身 を `pinned-source` / `1757000000×10^6` の i64 big-endian / `{"alive":true}` の順）から計算し直して一致。
`content_hash` の本体はこの change で変わっていない（`git diff f0b45f34 HEAD -- crates/server/src/heartbeat.rs` は欄と `component_version` を足すだけ）。`st51_form_fixed` の 10 列の名前・型・NULL の可否は `specs/distribution/spec.md:171` の表と一致（実装の出力を写しただけの値ではない）。

**手 2（ガードを壊す）:** 複製で 1 か所ずつ壊した。
- 移行の TRUNCATE の門（`update_history_no_truncate`）を作らない → `check-immutable.sh` rc=1（`NG owner から更新の履歴が書き換えられた: TRUNCATE`）/ `st51_rows_cannot_be_rewritten` FAILED
- `grants.sql` の `REVOKE UPDATE, DELETE ON core.update_history FROM ashiato_app;` を消す → `check-immutable.sh` rc=0（app の 3 行とも `OK app からの拒否`）/ `st51_rows_` 2 passed（**R18**）
- `tools/check-immutable.d/202610300900_st51-update-history.sh` を空にする → `check-immutable.sh` rc=0（`書き換え禁止 OK`）。節を空にしても本体は落ちない（ST23 R17 と同じ形。ST23 は自分の入口で節の OK 行を数えて塞いだ）。
  ただし同じ Scenario は `st51_rows_cannot_be_rewritten` も持ち、門を外すとそちらが落ちるので、Scenario の担保は失われない。指摘にはしない

**手 3（Scenario と test）:** `check_scenarios.py` rc=0。この change の 73 本は全部印があり、置き場は tasks.md の名指しどおり（grep で 73 本を 1 本ずつ引いた）。
主張の階層を見た範囲 —— 退避の前に試みの行が入っていることは本物の `tools/pg_dump-docker.sh` の退避を空の DB へ `pg_restore` して読む、標準出力と標準エラーの両方に `st51-secret-value` が無いことを見る、
後着の信号は発信の時刻と受信の時刻を分けて置く、生存信号の版は「積んだ印 A・送る印 D」で送った body の欄と原文の両方を見る（C-01・C-02）—— は THEN を観測している。
ずれは起動の書き込み（`record_startup`）の 2 本に出た: 「同じ版の変化を 2 つ同時に書いても、行は 1 つ」（**R16**）と「当てる処理の途中で…」の警告（**R17**）。

**手 4（本人の決定）:** 複製で値・分岐を 1 つずつ変え、関係する試験を走らせた。
- 落ちる（固定されている）: Q6 試みの行を退避の後へ動かす（`st51_migrate_attempt_committed_before_backup`・`st51_bin_migrate_attempt_survives_restore`・`st51_bin_migrate_backup_failed_row`）/
  Q4 並びを受信の時刻にする（`st51_view_late_queued_no_false_change`）・最初の信号も変化に数える（8 本）・版なしを並びに残す（`st51_view_versionless_between_different_versions`）・部品で分けない（`st51_view_split_by_component`）/
  D7 前の版に試み・落ちた行も数える（lib 8 本・bin 2 本）/ D16（仮）試みを同じ版でも書く（`st51_migrate_same_stamp_no_row`・`st51_migrate_two_migrates_pairs_intact`）/
  C-02 欄の無い行を `null` で送る（`st51_c02_old_heartbeat_sent_without_version`）・原文に版を入れない（`st51_c02_heartbeat_keeps_enqueued_version`）/ C-01 原文に版を入れない（`St51HeartbeatVersionTest` 3 本中 2 本 FAILED）。
  Q1 の誤りの本文を出さない・落ちた版と SQLSTATE は `st51_bin_migrate_failed_exit1_no_value` が出力と行の両方で見ている。Q5（格子・途絶の判定を変えない）は Task 9 の 2 本と、`coverage*.rs` / `outage*.rs` に新しい語が無いこと（final reviewer の確認）
- **固定されていない**: D6「導くときは欄を先に読む」（**R19**）、D10 の REVOKE（**R18**）

**手 5（`[x]` と実体）:** 20 件とも本文の検証コマンドが実在し rc=0。`--test server_startup` は `crates/server/tests/server_startup.rs`、`--lib` は collector-windows の lib が実在。件数つきは全部 `-- --list`（gradle は XML）で下限以上。
1.1 の「`.down.sql` は逆順に消し、版の記録の自分の行を消す」は検証コマンド（`check-migrations.sh` は存在と記載の静的検査）が観測しないので、一時の database（`st51cv_down`。終わりに DROP）で ORDER の 37 版を当て → 版の記録に自分の行と別の行を置き → `.down.sql` rc=0 → 表・ビュー・生存信号の `collector_version` の列が無く、版の記録は別の行だけ残る → 前進側を当て直して rc=0、を確かめた。

**手 6（隙間）:** 端末の時計が更新の後に戻ると、版の変化の読み口が更新 1 回を「戻した」と「上げた」の 2 行に見せる（**R20**）。記録は失われない（生存信号は残り、ビューは導くだけ）が、全記録の書き出しの `collector_version_changes.jsonl` に無かった版の戻りが出る。
退避から戻した後の書き直しは `docs/handoff/ST58.md` の ST51 の節が持ち、`docs/stories/ST58.md`（requires に ST51）が受ける。`check_chain.py` は未回収 0 件。

**手 7（機械が承認した訂正）: 該当なし**（`plan_fix.py pending` → `[]`）。

## R16. Scenario「同じ版の変化を 2 つ同時に書いても、行は 1 つ」は起動の書き込みを 2 つ走らせる主張だが、印の試験は migrate の結果の書き込みを呼んでおり、起動の書き込みの直列を外しても全部緑
- 成果物: crates/server/src/update_history_tests.rs / crates/server/src/update_history.rs
- 根拠: spec の WHEN は「印 B の**起動の書き込み**を 2 つ同時に走らせる」。`st51_migrate_concurrent_one_row`（update_history_tests.rs:563）は `record_migrate(…, Attempt::Unwritten, …)` を 2 つ走らせ、`record_startup` を呼ばない。
  複製で `startup_tx` の `pg_try_advisory_xact_lock` を `_shared` にし、更新の履歴の鍵（`HISTORY_KEY`）を取らない形にした → `cargo test -p ashiato-server st51_` 52 passed・`--test server_startup st51_` 8 passed（全部緑）。
  同じ変異の上で `record_startup(B)` を 2 つ `tokio::join!` で 10 回走らせる探り → `起動の書き込み 2 つで B の行が 2 行`（FAILED）。元のコードでは同じ探りが ok
- kind: technical
- 提案: `st51_migrate_concurrent_one_row` を `record_startup` 2 つにする（または Scenario の印をもう 1 本、起動の書き込みの試験に置く）。結果の書き込みの直列は別の試験で残してよい
- 処置: fixed 2.2 —— `st51_migrate_concurrent_one_row` を `record_startup` 2 つの同時呼び出しにし、版を替えて 10 回、毎回 1 行だけ増えることを見る（結果の書き込みの直列は印なしの `st51_migrate_concurrent_results_one_row` に残した）。起動の取引から更新の履歴の鍵を外すと FAILED（update_history_tests.rs:586）になることを確かめた

## R17. Scenario「当てる処理の途中で同じ組み立ての S-01 が起動しても、試みと結果の組は崩れない」の THEN「種別 `update_history_unwritten` の警告を示し」を試験が見ていない
- 成果物: crates/server/src/update_history_tests.rs / crates/server/src/update_history.rs
- 根拠: `st51_migrate_startup_between_attempt_and_result`（update_history_tests.rs:677）は起動の書き込みを `on_other_thread` で走らせ、待たずに返ることと行の並びだけを見る（警告の口 `unwritten_seen()` はスレッドローカルで、別のスレッドの警告は読めない）。
  複製で `record_startup` の `Ok(Some("migrate_running")) => {}`（錠が取れないときだけ警告を出さない）にした → `st51_` lib 52 passed・bin 8 passed（全部緑）
- kind: technical
- 提案: 起動の書き込みが書かなかった理由を返り値で返す（`record_startup` が `Option<&'static str>` を返す、など）か、同じスレッドで呼んで `unwritten_seen()` に `migrate_running` があることを見る
- 処置: fixed D15 —— `record_startup` が書かなかった理由の語を返す（`Option<&'static str>`。design D15 を更新）。`st51_migrate_startup_between_attempt_and_result` は別のスレッドの起動の返り値が `Some("migrate_running")` であることを見る

## R18. design D10 の `REVOKE UPDATE, DELETE ON core.update_history FROM ashiato_app` を消しても、Rust の試験も `check-immutable.sh` も通る
- 成果物: crates/server/src/grants.sql / crates/server/src/update_history_tests.rs / tools/check-immutable.d/202610300900_st51-update-history.sh
- 根拠: 複製で grants.sql:20 の REVOKE を消す → `cargo test -p ashiato-server st51_rows_` 2 passed / `check-immutable.sh` rc=0（`OK app からの拒否: UPDATE …` `DELETE …`）。
  試験は app の UPDATE・DELETE を「門の拒否 **または** 42501」で通し（update_history_tests.rs:148-152）、節は app の失敗なら理由を見ない。どちらも門（trigger）が先に断るので、権限の層を観測していない
  final review R13 の fix（46aaf094「アプリの役割は 42501 を見る」）は TRUNCATE（update_history_tests.rs:169-171）にしか及んでいない。権限の検査は trigger より先に走るので、42501 だけを通せば REVOKE の有無を区別できる
- kind: technical
- 提案: app の UPDATE・DELETE は SQLSTATE 42501 だけを通す（門の拒否 P0001 では落とす）。または `has_table_privilege('ashiato_app','core.update_history','UPDATE')` が false であることを見る
- 処置: fixed D10 —— `st51_rows_cannot_be_rewritten` はアプリの UPDATE・DELETE を 42501 だけで通し、`check-immutable.d` の ST51 の節もアプリの拒否に `permission denied` を求める。grants.sql の REVOKE を消すと試験が FAILED（update_history_tests.rs:156）になることを確かめた。`check-immutable.sh` rc=0

## R19. design D6「導くときは欄を先に読む」が試験で固定されていない（原文を先に読む形にしても全部緑）
- 成果物: migrations/202610300900_update_history.sql / crates/server/src/version_change_tests.rs
- 根拠: 複製でビューの `COALESCE(NULLIF(collector_version,''), <原文の client_version>)` の順を入れ替えた → `cargo test -p ashiato-server st51_view_` 14 passed。
  欄と原文の両方に**違う**版を持つ信号を置く試験が無い（`st51_view_reads_raw_when_column_empty` は列が空の信号だけ）。受け口は欄と原文を別々に受け取り、同じ値かを確かめない（D6「形を検査しない」）ので、欄と原文に別の値を送る送り手があれば食い違う
- kind: technical
- 提案: 列に `0.1.0+<commit A>`・原文に `0.1.0+<commit B>` を持つ信号を並びに置き、変化が A で数えられることを見る 1 本を `st51_view_` に足す
- 処置: fixed D6 —— `st51_view_column_before_raw` を足した（欄 A・原文 B の信号で、変化が A で数えられることを見る）

## R20. 更新の後に端末の時計が戻ると、版の変化の読み口が更新 1 回を「B→A」「A→B」の 2 行に見せ、全記録の書き出しに無かった版の戻りが出る
- 成果物: migrations/202610300900_update_history.sql（`core.collector_version_change`）/ crates/server/src/export/dump.rs（`collector_version_changes.jsonl`）
- 根拠: 複製で探りを走らせた: 端末 d1 の C-01 に、版 A を発信 10:00、更新後の版 B を発信 09:30（時計が戻った。受信 10:31）と 10:30（受信 11:31）で置く → ビューの行は `B → A at 10:00` と `A → B at 10:30` の 2 行（`更新 1 回で変化が 2 行`）。
  Q4 の答え（発信の時刻の順）どおりの帰結で、spec にも design にも時計が戻る場合の Scenario は無い（S-01 の側は D7「書いた順。時計は戻りうる」で扱っている）。生存信号の行は残るのでビューを直せば出し直せる（失われない）
- kind: premise
- 提案: 本人の Q4 の答えの前提（端末の発信の時刻は戻らない）が崩れる場合として人間へ返す。今の振る舞いのままにするなら、時計が戻ったときの見え方を Scenario に書いて固定する。変えるなら並べ方（発信の時刻が前の信号より戻ったら受信の順で並べる、など）を Q4 の補足として決める
- 処置: escalated: deep.md 第 3 回 Q7（`deep-questions-r3.json`・`docs/briefs/ST51-deep-r3.html`）。失うものは無い（読み口はビュー）が、本人の Q4 の答えの前提が崩れる場合なので返した。今の見え方は印なしの試験 `st51_view_clock_back_after_update` が固定している

## security review（f0b45f3..aaa4906）

見た経路:
- 端末 → `POST /heartbeat`（`lib.rs:4059`）→ `heartbeat_post` の頭の `authorize`（`lib.rs:3270`）→ `heartbeat_one` が `client_version` を bind で `core.heartbeat.collector_version` へ（`lib.rs:3180-3201`。SQL へ文字列連結しない）（4）
- `client_version` の値 → `normalize_component_version`（`ingest.rs:295`。U+0000 を除くだけ）→ 行の列と原文。ログ・応答には出ない（`git diff` の追加行の `tracing::`/`eprintln!` は 3 か所で、どれも種別・版の名前・SQLSTATE だけ）（3）
- `core.heartbeat` → ビュー `core.collector_version_change`（原文は `pg_input_is_valid` で確かめてから `->> 'client_version'` で読むだけ。動的 SQL なし）→ 書き出しの `VIEW_FILES`（`export/dump.rs:303-316`。ビュー名・列名は定数）→ 既存の合言葉付き ZIP（3）
- `run_migrate` / `run()` → `update_history::record_attempt` / `record_migrate` / `record_startup` → `core.update_history`（値は組み立ての印・ORDER の版の名前・SQLSTATE。外からの入力は入らない）（3）
- 書けないとき・落ちたとき → `warn_unwritten`（`update_history.rs` の `tracing::warn!`。固定の理由の語と SQLSTATE）/ `MigrateError::failure_line`（`migration_ledger.rs`。種別・版の名前・SQLSTATE。DB の誤りの本文を持たない）（3）
- 付与: `grants.sql` の末尾で `ashiato_app` から `core.update_history` の UPDATE・DELETE を外す。門（trigger）が所有者でも書き換え・切り詰めを断る（2。試験で外しても緑になる件は R18）
- 収集アプリ: C-01 `HeartbeatEmitter` が `BuildConfig.COMPONENT_VERSION`、C-02 `heartbeat::signal` が `build_stamp` を欄と原文へ。私的データは足していない。新しい送信先・待ち受け・依存の追加なし（`Cargo.toml`・`*.gradle.kts`・`package.json` の差分なし）（3）

安全の決め手: 版の値が外から入る唯一の新しい経路（生存信号の `client_version`）は、既存の `/heartbeat` の頭の `authorize` を必ず通る —— 確かさ 4。
一時の試験（`heartbeat_post` に `client_version: "evil"` を持つ 1 件を、token なし・誤った token で送る）を `cargo test -p ashiato-server zz_sec_probe` で走らせて消した:
`PROBE auth=false -> Some(401)` / `PROBE auth=true -> Some(401)`（誤った token）/ `PROBE rows heartbeat=0 view=0` / `1 passed`。

指摘なし

範囲外: 書き出しの各ファイルは利用者で絞らず全行を出す（`export/dump.rs` の `plans` の既定の分岐）。単一利用者の前提で既存の表と同じ扱いで、この差分の新しいファイルもそれに従うだけ。

## /code-review（f0b45f3..aaa4906）

見た: Claude Code の組み込みの `/code-review`（8 件）。写しは scripts/code_review_record.py

## R21. record_result treats 'never reached the version check' (run.attempt == None) the same as 'table absent' (Attempt::Unwritten), so errors raised before or during the ledger check still write a 'failed' row.
- 成果物: crates/server/src/migration_ledger.rs:324
- 根拠: If pg_advisory_lock fails, read_ledger_on returns a sqlx error, or describe_unknown fails while building the Ahead report, the error converts to MigrateError::Db. record_result then runs with attempt=Unwritten, and if core.update_history exists and the previous version differs from the stamp, it writes a 'failed' row (failed_version NULL). The design says nothing is written when the DB is ahead or unreadable, and here the check never passed. In the lock-failure case the write also happens without holding LOCK_KEY. Fix: skip record_result when run.attempt is None.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: 意図した振る舞い。design D8 の表の 7 行目「migrate の付与・錠・版の記録の読み書きの失敗（終了コード 1）→ 表を書けるなら `failed`」と、その下の「錠・版の記録の読みの失敗は版の検査より前なので、試みの行を伴わない」が、検査の前に落ちた回にも試みなしの `failed` を書くと決めている。ahead・読めない（終了コード 3）は `record_result` の先頭で返して書かない（migration_ledger.rs の `Err(MigrateError::Ahead { .. } | MigrateError::Unreadable(_)) => return`）

## R22. Deciding 'NotNeeded' by comparing only prev == stamp means a migrate run that does apply versions under an unchanged stamp leaves no attempt row and no result row.
- 成果物: crates/server/src/update_history.rs:560
- 根拠: build.rs gives every uncommitted build the same `<ver>+<commit>.dirty` stamp. A developer applies build B.dirty (history row: applied, B.dirty), adds a migration without committing, and runs migrate again. The stamp is still B.dirty, so record_attempt returns NotNeeded and record_migrate returns early. The newly applied version and any failed/backup_failed result never reach update_history.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: 意図した振る舞い。更新の履歴は組み立ての印の変わり目を残す表で（D2・D16（仮）「版の変わらない走らせ直しは、もともと版の変化ではない」）、同じ印の下で当てた版は ST48 の版の記録（`migration_ledger.applied`）が持つ。同じ印の走らせ直しの失敗も残したくなったときの反転条件は D16（仮）に書いてある。`.dirty` の印は手元の組み立てだけで、配る組み立ては commit から印を取る

## R23. warn_unwritten repeats the SQLSTATE extraction that sqlstate_of, defined right below it, already does.
- 成果物: crates/server/src/update_history.rs:475
- 根拠: The `e.and_then(as_database_error).and_then(code).map(to_string)` chain is copied. If one copy changes (for example how a missing code is represented), the warning and the stored row will show SQLSTATE differently. Use `e.and_then(sqlstate_of).unwrap_or_default()` instead.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 2.2 —— `warn_unwritten` は `e.and_then(sqlstate_of)` で SQLSTATE を取る

## R24. record_attempt and record_migrate each contain the same three-arm table_exists match (no_table / db_error / continue).
- 成果物: crates/server/src/update_history.rs:606
- 根拠: The same branch, with its fixed warning words, exists twice, and startup_tx handles the same condition a third way through `?`. Changing a warning word or the behavior means editing every copy. A helper returning `Option<()>` (or folding the check into write_in_tx) would cover all of them.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 2.2 —— 表の有無の 3 分岐を `table_ready` にまとめ、`record_attempt`・`record_migrate` がそれを呼ぶ（起動の取引は理由の語を返す形なので `startup_tx` の `?` のまま）

## R25. The VIEW_FILES plan builder copies the default (`_`) branch of the Include plan builder (same SELECT/ORDER/count format strings).
- 成果物: crates/server/src/export/dump.rs:303
- 根拠: If the default branch's SELECT changes later (a filter, the trailing NULL::uuid, the count form), the view files will quietly diverge from the table files. Build both from one shared plan constructor that takes (file, view, columns).（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed D11 —— 全行を出すファイルの計画を `whole_plan` にまとめ、`Include` の既定の分岐と `VIEW_FILES` の両方がそれを呼ぶ。`export_` 191 passed

## R26. The view parses every heartbeat's raw text as jsonb twice (pg_input_is_valid, then raw::jsonb), and the export runs the whole window-function view twice (count(*) plan, then SELECT).
- 成果物: migrations/202610300900_update_history.sql:754
- 根拠: No row written before ST51 has the collector_version column set, so every legacy heartbeat goes to the JSON path. Each export does 2 full scans × 2 JSON parses per row, then a sort/window over the whole core.heartbeat table, inside the export transaction. The cost grows linearly with history size. A cheaper form is to count the streamed rows (or skip the separate count for views) and to parse once using a CASE on a pre-filter, or `jsonb_typeof` after a single validated cast in a lateral subquery.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: 再現はした（JSON の読みは欄が空の信号で 2 回・件数と本文で 2 回引く）が、直さない。件数と本文を別に引くのは書き出しの全部の表と同じ形（D11 の目録の件数）で、書き出しは利用者が手で始める操作。生存信号は 1 端末 1 日数十件（D5（仮）の費用）で、`pg_input_is_valid` は JSON でない原文で読み口を落とさないための門（Scenario「原文が JSON でない信号があっても、読み口は失敗しない」）

## R27. The export order for collector_version_changes has no unique tiebreak, unlike the view's own lag ordering (emitted_at, received_at, id).
- 成果物: crates/server/src/export/dump.rs:92
- 根拠: Two change rows can share user, component, device, emitted_at and received_at (for example, a batch of queued heartbeats from different logical sources received in one request). Postgres may then emit them in a different order on each export, so two exports of the same data produce different bytes in the file. The view doesn't expose id, so add logical_source (or expose id) as the final sort key.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed D11 —— `collector_version_changes.jsonl` の並びを行の全部の列（`logical_source`・`from_version`・`to_version` を足す）にした。並びが同じ行は中身も同じなので、書き出すたびに同じバイトになる

## R28. record_startup's non-blocking try-lock on LOCK_KEY also fails when a second S-01 process is running the same startup, so that case gets the warning 'migrate_running'.
- 成果物: crates/server/src/update_history.rs:643
- 根拠: Two server processes (for example a restart that overlaps with the old process, or two instances) start at once. One takes the xact lock on LOCK_KEY. The other's pg_try_advisory_xact_lock returns false and it logs update_history_unwritten why=migrate_running, although no migrate is running. Operators reading the warning are pointed at the wrong cause.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed D16 —— 起動の取引は当てる処理の錠を `pg_try_advisory_xact_lock_shared` で取る（design D16（仮）を更新）。migrate は同じ数を排他で持つので走っている間は取れず、起動どうしは互いを止めない。`st51_migrate_concurrent_one_row` が同時の 2 つの起動の返り値が両方 `None` であることを見る
