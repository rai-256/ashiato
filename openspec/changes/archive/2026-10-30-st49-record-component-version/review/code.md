# ST49 code review

## final review（d90ef3d..42ac63b）

final reviewer（`superpowers:requesting-code-review` の `code-reviewer.md`）の指摘。Critical 0 / Important 3 / Minor 13。
fix は 1 回（2b92476..d658b89）、scoped re-review 1 回で R1〜R8・R13 は全部 ADDRESSED、新しい Critical / Important なし。R9〜R12 は直さない（理由は各 R）。
Task ごとの ⚠️ は final reviewer が全体で確かめ、解消済み（Task 3 の `st49_mig_down_and_up_again`・clippy、Task 6 の D9、Task 8 の Scenario 名、Task 11 の本数ほか）か code-verify の席の担当（RED の証跡・`-- --list` の本数）に振り分けた。

## R1. `tools/check-openapi.sh` の `client_version` の除外が Task 9 の後も残り、注記（「Task 9 で外す」）と食い違う
- 成果物: tools/check-openapi.sh
- 根拠: check-openapi.sh:23-25（/ingest）・70-71（drops）/ Task 3 ⚠️・Task 6 F6 / `IngestRequest.kt` は `client_version` を既に持つ
- kind: technical
- 処置: fixed 9.2 —— /ingest・drops の両方で除外を外し注記を直した。`DropReportContractTest` の CONTRACT-FIELDS に `client_version` を足し素材を版つきの下書きにした（原文の鍵の比較からは除く）（2bfa158）。`./tools/check-openapi.sh` rc=0（破棄の報告の欄名 OK（12 欄））

## R2. `docs/handoff/ST29.md` に足した節が「衛星の資格情報はまだ無い・ST49 が先に入る」と書くが、ST29 は archive 済みで `Caller::Satellite => Internal` は既にある
- 成果物: docs/handoff/ST29.md
- 根拠: base d90ef3d の web_session.rs:45 / openspec/changes/archive/2026-10-27-st29-satellite-credentials / lib.rs:1337 / Task 11 F1（「どの版の欄にも書かない」とも読める）
- kind: technical
- 処置: fixed D9 —— 節を決定の記録に書き直した（衛星は `Writer::Internal` に振り分け済み。collector_version / web_version には書かず、server_version は S-01 が入れる）（26ae2b1）

## R3. `tools/check-build-stamp.sh` が CI からも `docs/testing.md` からも呼ばれていない（印の作り直しと `.dirty` の 7 本の Scenario を担保するのはこの台本だけ）
- 成果物: docs/testing.md
- 根拠: `.github/workflows/ci.yml`・`docs/testing.md` に名前が無い / evidence.jsonl は android 節だけ / tasks.md は CI への組み込みを求めていない（計画側の抜け）
- kind: technical
- 処置: fixed 1.2 —— `docs/testing.md` §1 の表に `tools/check-build-stamp.sh tool|server|windows|android` の行（担保する範囲・走らせ方・CI 未組み込みで重い・印の作り方を触ったら手元で走らせる）を足した（26ae2b1）。`tools/check-shared-appends.sh` rc=0。CI の job への組み込みは tasks.md の範囲外なので change に出す（PR 本文の未処置に挙げる）

## R4. C-01 の組み立てが `ASHIATO_BUILD_COMMIT` を読まず git だけを見る（D5 は C-01 も「環境変数 → git → unknown」）
- 成果物: collector-android/app/build.gradle.kts
- 根拠: build.gradle.kts:222 / design.md D5（commit の出所・「C-01 と V-01 は同じ規則」）
- kind: technical
- 処置: fixed D5 —— `providers.environmentVariable("ASHIATO_BUILD_COMMIT")`（trim・空でなければ）→ `git rev-parse HEAD` → `unknown` の順にした（e14a737）。`check-build-stamp.sh android` に環境変数ありと空の 2 場合を足し rc=0（d658b89）

## R5. 試験の小さな穴（状態符号・負の側・空振り）
- 成果物: crates/server/src/component_version_tests.rs, crates/collector-windows/src/runtime.rs
- 根拠: `st49_ingest_server_version_is_build_stamp`・`st49_dedup_*` が状態符号と accepted を見ない（Task 3 F1/F3）/ 画面のログインで /drops・/media/missing に送った版が NULL になる試験が無い（Task 6 F3）/ `st49_c02_heartbeat_has_no_version` が `send()` の後に `unwrap_or_default` で読み空振りしうる（Task 8 F2）
- kind: technical
- 処置: fixed 6.1 —— 3 本に `StatusCode::OK` と `accepted == true`、負の側 `st49_ledger_web_session_reports_drop_client_version`（`Writer::Web` を Collector 扱いに壊すと rc=101 で落ちることを確認）、heartbeat の試験は送れない transport で積んで `send()` の前に `unwrap()` で読む形に（b2f2311）。`cargo test -p ashiato-server st49_` 45 passed・`st49_c02_heartbeat_has_no_version` rc=0・clippy `-D warnings` rc=0

## R6. detekt の baseline に、この差分が生んだ import 順の違反（`LocationService.kt` の `KSerializer`）を足して隠している
- 成果物: collector-android/config/detekt-baseline.xml
- 根拠: Task 9 F1 / app/build.gradle.kts:173-174（新しい違反だけで落とす方針）
- kind: technical
- 処置: fixed 9.2 —— import を `android.util.Log` の直後へ移し baseline の 1 行を消した。組み直しで当たらなくなった `DropReportContractTest` の 2 行も消した（68f9703）。`:app:detekt` rc=0・単体 31 本 failed 0

## R7. export の新しい試験 5 本が `ashiato-export-file-*` を一時フォルダに残す
- 成果物: crates/server/src/export_component_version_tests.rs
- 根拠: evidence.jsonl の tmp_left / Task 7 F4（`HARNESS_TMP_LEAK=fail` に切り替えると落ちる）
- kind: technical
- 処置: fixed 7.1 —— 返した zip を包む `Exported`（Drop で `ashiato-export-file-` の置き場だけ消す）と `export_here` に 5 本を寄せた（8d2050d）。実行の後に新しい置き場は増えていない。共有の `export_once` を使うほかの試験の残りは ST49 より前からある（Task 11 F2）

## R8. `tools/check-build-stamp.sh` の注記と決め打ち（windows 節は rlib を読むのに「bin を strings」、版番号を `"0."*` に決め打ち、android 節の失敗の理由が gradle の失敗にならない）
- 成果物: tools/check-build-stamp.sh
- 根拠: check-build-stamp.sh:5・583・android 節の `read <<<"$(android_stamps …)"` / Task 8 F1・F3・Task 9 F2
- kind: technical
- 処置: fixed 1.2 —— 注記を rlib に、版番号は `Cargo.toml` の `[workspace.package]` version / `versionName` と比べ、式は版番号を逃がして組む。android 節は `got="$(android_stamps …)" || exit 1` で gradle の失敗の理由で止まる（d658b89）。4 節とも rc=0

## R9. `git status` が失敗すると印が「きれい」になる（Rust・web・gradle の 3 つとも）
- 成果物: crates/build-stamp/src/lib.rs, web/build-stamp.ts, collector-android/app/build.gradle.kts
- 根拠: Task 10 F1 / final reviewer は 3 つで揃っていることを確認
- kind: technical
- 処置: rejected: 3 つ（Rust・web・gradle）で揃った扱いで、`git status` だけが落ちて `rev-parse` が通る場面は実地で起きにくい。直すなら 3 つの印の意味を同時に変える（`.dirty` に倒すか `unknown` にするか）判断になり、この fix の範囲を越える。印は「その組み立ての申告」で、誤っていても記録の内容は失われない

## R10. 環境変数の commit の形を検査しない（`ci.yml:76` の `ci-<run id>` は D5 の正規表現に合わない印になる）
- 成果物: crates/build-stamp/src/lib.rs, web/build-stamp.ts
- 根拠: Task 10 F4 / `st49_stamp_env_commit_wins` / design.md D5（ST48 の D8（仮）と同じ順）
- kind: technical
- 処置: rejected: D5 は commit の出所を ST48 の D8（仮）と同じ順に定め、環境変数の値を形で検査しない（`st49_stamp_env_commit_wins` が固定）。`ci.yml` の `ci-<run id>` は CI の組み立てだけで配らない。形を検査するなら ST48 の台帳の `build_commit` と同時に変える必要がある

## R11. `crates/server/build.rs` の `commit_of` が組んだ印を分解して commit を取り出す（環境変数の値が `.dirty` / `.debug` で終わると ST48 の台帳と食い違う）
- 成果物: crates/server/build.rs
- 根拠: build.rs:426-430 / Task 1 F2
- kind: technical
- 処置: rejected: 環境変数に `.dirty` / `.debug` で終わる値や `+` を含む値を渡す運用は無い（CI は `ci-<run id>`、手元は git）。`ashiato-build-stamp` に commit を返す関数を公開すれば消える差で、実害が出たら直す

## R12. 滞在の「原文が変わったか」を Rust（`e.raw != raw`）と SQL（`raw IS DISTINCT FROM $2`）の 2 か所で判定している
- 成果物: crates/server/src/stay_store.rs
- 根拠: stay_store.rs:600・981 / Task 5 F1
- kind: technical
- 処置: rejected: いまは 2 か所の判定が一致し（`e.raw != raw` と `raw IS DISTINCT FROM $2` はどちらも原文の値の比較）、D2（仮）の試験（5.1）が両方の結果を見ている。bool を bind して 1 か所に寄せるのは整理で、振る舞いは変わらない

## R13. `DropLedger.openIndex` の版の違う下書きを閉じる loop は `load()` が先に閉じるので当たらない
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/DropReport.kt
- 根拠: DropReport.kt:210 / Task 9 F4
- kind: technical
- 処置: fixed 9.3 —— loop を除き `indexOfFirst` の 1 行にした。版の違う下書きを閉じるのは `load()` だけ（a94db3e）。`*DropReport*`・`*DropLedger*`・`*St49*` 緑

## code_verify（7da9d42。2026-10-28）

### 申告と実測

申告: tasks.md 25/25 `[x]`。検証コマンドは各 Task の本文（`CT` / `CW` / `CS` / `GT` / `VT` / `STAMP` と 11.2 の全体）。

| 項目 | 検証コマンド（要点） | 実測 |
|---|---|---|
| 1.1 | `CS st49_stamp_`（7 本以上） | 一致（`--list` 7・7 passed・rc=0） |
| 1.2 / 8.1 / 9.1 | `./tools/check-build-stamp.sh tool` / `server` / `windows` / `android` | 一致（4 節とも rc=0。7 本の `== Scenario:` が全部 OK） |
| 1.3 | `cargo build -p ashiato-server` / `CT migration_ledger_`（20 本以上） | 一致（`--list` 23） |
| 2.1 | `./tools/check-migrations.sh` | 一致（rc=0） |
| 2.2 | `CT st49_gate_`（9）/ `CT st49_cols_`（1）/ `CT dedup_tests`（35） | 一致（9 / 1 / 35） |
| 2.3 | `./tools/check-immutable.sh` / `./tools/check-shared-appends.sh` | 一致（両方 rc=0。ST49 の節 `== ST49: …` の 6 行が OK） |
| 3.1 / 4.1 | 先に `CT st49_ingest_collector` / `CT st49_update_versions` が rc≠0 を確かめる | **証跡なし**（evidence.jsonl に 3.1・4.1 の行が無い）。手 2 の壊しで代わりに確かめた: `Writer::split` の `Collector` 腕を `(None, None)` → `st49_ingest_collector_version_and_server_version` が component_version_tests.rs:522 で panic / `apply_external_update` の UPDATE の 3 列を据え置き → `st49_update_versions_move_and_history_keeps_old` が :844 で panic（3 passed 1 failed） |
| 3.2 | `CT st49_ingest_`（8）/ `CT st49_dedup_`（1）/ `st49_hash`（1）/ `hash_is_pinned`（1） | 一致（8 / 1 / 1 / 2） |
| 3.3 / 6.2 | `check-openapi.sh` / `jq … has("client_version")` ×3 / `grep -c client_version docs/collector-contract.md` ≥ 3 | 一致（rc=0・true・5）。`--bin openapi` で作り直した契約と `docs/openapi.json` の diff 無し |
| 4.2 | `CT st49_update_`（4）/ `CT dedup_tests`（35） | 一致 |
| 5.2 | `CT st49_stay_`（4）/ `CT stay_tests`（65） | 一致（4 / 65） |
| 6.2 | `CT st49_ledger_`（9）/ `drops_tests`（15）/ `media_missing_tests`（6）/ `deletion_tests`（26）/ `feelings_remind`（8） | 一致（10 / 15 / 6 / 27 / 8） |
| 7.2 | `CT st49_export_`（6）/ `CT export_`（130） | 一致（6 / 182） |
| 8.1 | `CW st49_c02_`（5）/ `cargo clippy -p ashiato-collector-windows --target x86_64-pc-windows-gnu -D warnings` | 一致（5 passed・clippy rc=0） |
| 9.2 / 9.3 | `GT St49OutboxTest`（5）/ `GT St49DropDraftTest`（2）/ `:app:testDebugUnitTest` 全部 | 一致（5 / 2・全 634 本 failures 0） |
| 10.1 | `VT buildStamp.test.ts` / `npx vitest run` / `npm run build` | 一致（9 passed / 509 passed / rc=0） |
| 10.2 | `cd web && npm run test:e2e -- st49-client-version` | 一致（1 passed・rc=0） |
| 11.1 | handoff の grep / `check-shared-appends.sh` | 一致（rc=0） |
| 11.2 | fmt / clippy workspace / `cargo test -p ashiato-server` / `CS st49_` / 検査 4 本 / `tools/smoke.sh` / `check_scenarios.py` | 一致（fmt rc=0・clippy rc=0・1593 passed 1 ignored・7・4 本 rc=0・smoke rc=0「縦串 OK」・scenarios: OK） |

### 手 1: 固定値を独立に再計算する

`hash_is_pinned` / `st49_hash_ignores_client_version` の `39d0ebc5…5df1` を python の `hashlib.sha256` で計算し直した（各項目を 8 バイトのビッグエンディアンの長さ＋中身。入力は `"test"`・`1757000000×10^6` µs の i64 BE・`{"v":"x"}`）。**一致**。この値は ST49 より前（22544b25）からあり、ST49 は鍵を変えていない。

### 手 2: ガードをわざと壊す

壊すたびに戻し、前後で `git status --porcelain` と `git diff` が空のままであることを確かめた。

| 壊したもの | 実行 | 結果 |
|---|---|---|
| `deletion.rs` の削除の台帳の INSERT から `server_version` を外す | `cargo test -p ashiato-server --lib st49_ledger_inserts_name_server_version` | 落ちた（rc=101「server_version を名指さない INSERT: ["deletion.rs: INSERT INTO core.deletion_ledger"]」） |
| 移行の `require_version_or_ledger` の `content_changed` から版の 3 行を外す | `./tools/check-immutable.sh` | 落ちた（rc=1。`NG 前の版の履歴なしで版が書き換えられた` ×3・`NG 拒まれたのに版が変わっている`） |
| `ingest.rs` の Scenario の印を 1 字変える | `python3 scripts/check_scenarios.py . st49-record-component-version` | 落ちた（rc=1「担保の無い Scenario」1 件） |
| `IngestRequest.kt` の `@SerialName("client_version")` を外す | `./tools/check-openapi.sh` | 落ちた（rc=1「契約とKotlin側の欄名がずれている」） |
| `Writer::split` の `Collector` を `(None, None)` | `CT st49_ingest_collector` | 落ちた（上の表の 3.1） |
| S-01 の `STAMP_SOURCES` を `["crates/server/src"]` に狭めた clone（commit 済み） | その clone で `./tools/check-build-stamp.sh server` | **落ちなかった**（rc=0）→ R16 |
| C-01 の原本の欠けの溜めの `BuildConfig.COMPONENT_VERSION` を外す | `:app:testDebugUnitTest` 全部 | **落ちなかった**（634 本 failures 0）→ R15 |
| C-01 の `DropLedger(version = BuildConfig.COMPONENT_VERSION)` を `null` | `:app:testDebugUnitTest` 全部 | **落ちなかった**（634 本 failures 0）→ R15 |

検査の外側: 書き出しから `deletion_ledger` と `media_missing` が外れているのは ST49 より前からの `Class::Exclude`（export.rs:565・583）で、版の列の落ちではない。

### 手 3: Scenario と test を突き合わせる

`python3 scripts/check_scenarios.py . st49-record-component-version` rc=0（scenarios: OK）。65 本の印の先を 1 本ずつ見た。
主張の階層とテストの階層が食い違うのは、C-01 の「原本の欠け・破棄の報告に積む時点の版を書く」（device-collection の Requirement の SHALL 1 文目）。
これに対応する Scenario 2 本（「破棄の報告は、作った収集アプリの版と…」「原本の欠けは、作った収集アプリの版と…」）は S-01 の HTTP の試験で担保されていて、C-01 の配線はどの試験も見ていない（R15）。
ほかの Scenario は、印の先の試験が Scenario の主張（どの列に・どの値が入るか、送った本文、生成された `BuildConfig.java` の印）を直接見ている。

### 手 4: 本人の決定が test で固定されているか

- Q1 の形（`<版番号>+<40 桁>[.dirty][.debug]`・並び順）: `st49_stamp_dirty_debug` などが完全一致で固定している。版番号は `check-build-stamp.sh` が `Cargo.toml` / `versionName` と比べ、e2e は `web/package.json` と比べる
- Q1 の「`.dirty` は**その部品のソース**の未 commit の変更」: **値を変えても全部通る**。S-01 の `crates/server/build.rs:15` の `STAMP_SOURCES` を `crates/server/src` だけにしても `check-build-stamp.sh server` は rc=0。台本が `.dirty` を作るために触るファイルは部品ごとに 1 つだけ（`crates/server/src/lib.rs`・`crates/collector-windows/src/lib.rs`・`…/collector/Config.kt`）（R16）
- Q2（S-01 の版をすべての記録に）: 8 表の INSERT を `st49_ledger_inserts_name_server_version` が文字列で縛る（手 2 で落ちることを確認）。衛星の表は ST39・ST40・ST46 の handoff にある（`docs/handoff/ST39.md:51`・`ST40.md:10`・`ST46.md:3`）
- Q3（破棄の報告・原本の欠けは収集アプリの版と S-01 の版、台帳は S-01 の版）: S-01 の側は表ごとの試験で固定されている。C-01 の側の焼き込みは固定されていない（R15）
- Q4（画面の版と S-01 の版）: `st49_ingest_web_session_goes_to_web_version` と e2e が固定している
- Q5（書き出しの説明「今の中身」）: `st49_export_doc_says_current_content` が固定している

### 手 5: tasks の `[x]` と実体

上の表のとおり、25 項目の検証コマンドは全部実在して rc=0、本数の下限も満たす（`cargo test --test` の型は使っていない）。
足りないのは 3.1・4.1 の「先に落ちることを確かめる」の証跡だけで、手 2 の壊しで代わりに確かめた（どちらも落ちた）。

### 手 6: 隙間

- 既存の移行 `202610231200_event_end_time.sql` を書き換えたため、ST41 を当て済みの DB では起動のたびに警告が出続ける（R14。実際に起こして確かめた）
- 古い S-01 が `client_version` を読み飛ばして版だけが落ちる間（S-01 の `IngestRequest` に `deny_unknown_fields` は無い）は `docs/handoff/ST50.md:3` に申し送り済み。`docs/stories/ST50.md` も「知らない欄を黙って捨てる」を受け持っている（指摘にしない）
- C-01 の `SegmentStore` は行を書き直さない（追記と `.acked` だけ）。型に通して書き直して版の欄・知らない欄を落とす経路は無い
- `.down.sql` で版の値が失われることは、冒頭のコメントと design の Migration Plan 3 に書いてある（指摘にしない）

### 手 7

該当なし（`python3 scripts/plan_fix.py pending openspec/changes/st49-record-component-version` → `[]`）。

## R14. 当て済みの ST41 の移行 `202610231200_event_end_time.sql` を書き換えたため、ST41 を ST49 より前に当てた DB は起動のたびに「指紋が違う」の警告を出し続ける。書き換えが効くのは、全部の版を当て直す試験の経路だけ
- 成果物: migrations/202610231200_event_end_time.sql / migrations/202610251000_record_component_version.sql
- 根拠: `git diff d90ef3d..HEAD -- migrations/202610231200_event_end_time.sql`（d690d95 が `event_version_live` を `IF NOT EXISTS` の DO で包み、md5 の許す値を 2 つ足した）/ 中身の sha256 は d90ef3d が `57264b68…0825`、HEAD が `1602f87d…3191` / 確認バッチの DB（`ashiato2-verify-db-1`）の `migration_ledger.applied` はこの版を `57264b68…` で記録している / 試した: worktree の DB で `target/release/ashiato-server migrate` を当て（rc=0）、記録の sha256 を `57264b68…` に書き換えてからもう 2 回 `migrate` を起こすと、2 回とも `WARN … 記録した指紋と中身が違う版がある（当て直さない） version="202610231200_event_end_time"`（rc=0）。台帳は書き直されないので警告は消えない / migration_ledger.rs:374-377 は記録のある版を当てないので、書き換えた本体は既存の DB では一度も走らない。走るのは raw で全部の版を当て直す `check-immutable.d` の `for m in MIGS` と、試験の `fresh_db` だけ（d690d95 の本文「check-immutable の 2 回目の当て」）/ ST49 の移行の 2 行目「`migrate()` は起動のたびに全版を当てる」も ST48 の後は正しくない
- kind: technical
- 提案: ST41 の移行は d90ef3d の中身に戻し、当て直す側（ST49 の `check-immutable.d` の節など）で `event_version_live` を落としてから当てるか、台帳を通して当てるかで、2 回目の当てを通す。戻せないなら、警告が出ることと、その理由を handoff と移行の注記に書く
- 処置: fixed D19 —— ST41 の移行は戻さない: `tools/check-immutable.sh` の 2 回目の当て（ST48 の D12 が保つ）は ST41 の本文そのものをこの版の上に当てるので、新しい版の側では通せない（戻すと `CREATE OR REPLACE VIEW` が列を減らす作り直しになり、md5 の検査も落ちる）。配った印（`v*`）より前の版は直してよい（ST48 Q1）、指紋の違いは警告だけ（ST48 D7（仮））の範囲。警告が出ることと理由・消し方（`STACK_RESET=1`）を ST49 の移行の冒頭と design D19 に書いた（d846989）。`./tools/check-migrations.sh` rc=0

## R15. C-01 が原本の欠けと破棄の報告に「積む時点の版」を書く配線を外しても、どの試験も落ちない
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/LocationService.kt / collector-android/app/src/test/kotlin/dev/ashiato/collector/St49OutboxTest.kt
- 根拠: LocationService.kt:564 の `segmentOutbox("media-missing", …, BuildConfig.COMPONENT_VERSION)` から印を外す → `./collector-android/gradlew -p collector-android :app:testDebugUnitTest` rc=0（634 本 failures 0）/ LocationService.kt:586 の `version = BuildConfig.COMPONENT_VERSION` を `null` にする → 同じく rc=0（634 本 failures 0）/ device-collection の Requirement の SHALL 1 文目は「C-01 が記録・破棄の報告・原本の欠けを未送信に積むとき、その時点の C-01 の組み立ての印を…書き込む」。対応する 2 本の Scenario は S-01 の HTTP の試験（`st49_ledger_drop_report_versions`・`st49_ledger_media_missing_versions`）にあり、C-01 の配線を見るのは `heartbeatHasNoVersion` の記録の溜め（`outboxForTest`）だけ。`St49DropDraftTest` は `DropLedger` に版を引数で渡すので、本物の配線は見ていない / 計測テストにも無い（`grep -rn "client_version\|COMPONENT_VERSION" collector-android/app/src/androidTest` 0 件）
- kind: technical
- 提案: `heartbeatHasNoVersion` と同じ `TestableLocationService` で、原本の欠けの溜めに積んだ行の `client_version` と、`ledger` の下書きの `clientVersion` が `BuildConfig.COMPONENT_VERSION` であることを見る試験を足す
- 処置: fixed 9.2 —— `St49OutboxTest.mediaMissingAndDropDraftCarryStamp`（`TestableLocationService` の原本の欠けの溜めに積んだ行の `client_version` と、`ledgerForTest` の下書きの `clientVersion` が `BuildConfig.COMPONENT_VERSION`）と覗き口 `mediaMissingOutboxForTest` を足した（d846989）。2 か所の配線を外すと落ち、戻すと通る。`:app:testDebugUnitTest` 636 本 failures 0・`:app:detekt` rc=0

## R16. Q1 の「`.dirty` はその部品のソースの未 commit の変更」の「部品のソースの道」を狭めても、`check-build-stamp.sh` は通る
- 成果物: crates/server/build.rs / crates/collector-windows/build.rs / collector-android/app/build.gradle.kts / web/build-stamp.ts / tools/check-build-stamp.sh
- 根拠: `git clone --local` した木で `crates/server/build.rs:15` の `STAMP_SOURCES` を `["crates/server/src"]` にして commit し、その木で `./tools/check-build-stamp.sh server` → rc=0（`OK: …debug -> ….dirty.debug`）。`migrations/`・`crates/build-stamp`・`Cargo.lock` の変更が `.dirty` にならなくなっても通る / 台本が `.dirty` を作るために触るのは、部品ごとに 1 ファイルだけ（check-build-stamp.sh:93・135・183）。「ほかの部品だけが変わっても `.dirty` にならない」を本物の組み立てで見る節も無い（`st49_stamp_other_tree_is_not_dirty` は使い捨ての木で `stamp()` を呼ぶだけで、各部品の道の一覧を見ない）。C-02・C-01・V-01 の道は壊して走らせていない（台本の触り方は同じ形）
- kind: technical
- 提案: `server` の節で `migrations/ORDER` か `crates/build-stamp` を触って `.dirty`、`web/` を触って `.dirty` にならない、の 2 つを足す（C-02・C-01 の節も、もう一つの道と、ほかの部品の木で同じことをする）。もしくは各 `build.rs` の道の一覧を定数として試験から名指しで固定する
- 処置: fixed 1.2 —— `check-build-stamp.sh` の各節の終わりに、ほかの部品の木だけの変更で `.dirty` にならないこと（server: `web/`、windows・android: `crates/server`）と、部品の道の別の場所で `.dirty` になること（server: `migrations/ORDER`・`crates/build-stamp`、windows: `crates/build-stamp`）を足した（d846989）。4 節 rc=0。`STAMP_SOURCES` を `["crates/server/src"]` に狭めた clone で server 節が落ちることも確かめた（下の再レビュー）

## security review（d90ef3d..7da9d42）

見た経路:
- `POST /ingest` → `ingest`（lib.rs:1908 `Writer::of(authorize(..)?)`）→ `ingest_one` → `ingest_request` → `core.event` の `collector_version`/`web_version`/`server_version`（3）
- `POST /drops` → `drops_post`（drops.rs:380 `Writer::of(authorize(..)?)`）→ `drop_one` → `core.drop_report`。版を入れるのは `Writer::Collector` のときだけ（drops.rs:280）（3）
- `POST /media/missing` → `missing_post`（media_missing.rs:208）→ `missing_one` → `core.media_missing`。drops と同じ振り分け（3）
- 書き手の決め方: `Writer` は要求の欄からでなく `authorize` が返した `Caller` から決まる（lib.rs:1166）。衛星の資格情報は `authorize` が先に 403 で返す（lib.rs:653-654）ので、`Writer::Internal` に届くのは `store_one`（書庫の取り込み。worker.rs:1095 は `client_version: None`）だけ（3）
- 値の扱い: `client_version` は sqlx の bind だけで SQL に入る（lib.rs:1541・1882、drops.rs:310、media_missing.rs:162）。文字列の連結・パス・シェル・正規表現には入らない。U+0000 だけ除く（ingest.rs:295）。大きさは `/ingest` 系の既定の本文上限（axum の `DefaultBodyLimit`。lib.rs:3816 の上書きは原本の口だけ）に収まる（2）
- 削除との関係: 削除済みの行は `blocked_by_deleted` → `SkippedDeletedContent` で版も書き換えない（lib.rs:1565、試験 `st49_update_deleted_keeps_versions`）。版の列は外部の更新の前の版へ写すだけで、消した中身を戻す経路は足していない（3）
- 書き出し: 足したのは版の列だけ（export.rs:196-390）。出さない表（`Class::Exclude`）は変えていない（2）
- 組み立ての印: `crates/build-stamp` は `git` を固定の引数で `Command::new` する（lib.rs:85）。外から来る値は組み立ての環境変数 `ASHIATO_BUILD_COMMIT` だけ。画面の印は配る JS に入る（vite.config.ts:35）が、ST48 の時点で同じ commit をすでに DB の台帳に持つ（migration_ledger.rs:19）。記録の中身は入らない（2）
- 端末の置き場: C-01 `SegmentStore.stamped`・C-02 `outbox.rs` は積む行に版の 1 欄を足すだけで、ログに出すのは既存の `Log.w(TAG, it)`（読めない行の通知）のまま（2）
- 依存: 足したのはリポジトリの中の `ashiato-build-stamp`（path 依存。本体の依存は `std` だけ）と、その dev 依存の `tempfile`（すでに Cargo.lock にあるもの）。外の crate・npm・gradle の依存は増えていない（2）

安全の決め手: 版を書く 3 つの口（`/ingest` `/drops` `/media/missing`）では、書き手（`Writer`）を `authorize(..)?` の戻り値からしか作れない。資格情報が無いか偽物なら、本文を読む前に 401 で返る —— 確かさ 4。
`component_version_tests.rs` に一時的な試験を足して `scripts/quiet-run sec -- cargo test -p ashiato-server --lib sec_probe -- --nocapture` を走らせ、試験は消した（rc=0）。
3 つの口それぞれに「資格情報なし」「`Bearer wrong`」「偽の cookie」を送ると、9 本すべて `401 Unauthorized` だった。`client_version: "evil<script>"` を持つ行も、本文の印を持つ行も 0 件（`PROBE rows_with_marker=0 rows_with_version=0`）。衛星の資格情報（403）は走らせていない（lib.rs:653 の file:line まで）。

指摘なし

## /code-review（d90ef3d..7da9d42）

見た: Claude Code の組み込みの `/code-review`（8 件）。写しは scripts/code_review_record.py

## R17. This branch edits ST41's forward migration after it was merged (the view guard and the md5 lists). Every DB already holds a ledger row for that migration, and that row now records a sha256 that no longer matches the file.
- 成果物: migrations/202610231200_event_end_time.sql:44
- 根拠: Any dev, verify or integ DB that already applied 202610231200_event_end_time has its ST48 ledger row with the old sha256. On every later `migrate`, `warn_changed_fingerprints` logs '記録した指紋と中身が違う版がある' for this migration, and the edited text never runs on those DBs (only migrations missing from the ledger are applied). The edit only takes effect on fresh or pre-ledger DBs, while the warning becomes permanent noise on all the others. The fix belongs in the new migration, not in the old one.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: R14 と同じ事実（再現は R14 の根拠）。「新しい版の側で直す」はできない —— `tools/check-immutable.sh:34` の 2 回目の当ては `migrations/202610231200_event_end_time.sql` の本文をこの版の上に当てるので、ST41 の本文が後の版の列と関数を受けられないと落ちる。配った印より前の版の書き換えは ST48 Q1・D7（仮）が許す範囲で、警告と理由は R14 の処置で移行の冒頭と design D19 に書いた

## R18. `emit_rerun` passes `.git/index` to `rerun-if-changed`. Any write to the index anywhere in the repo then re-runs the build scripts and recompiles ashiato-server and collector-windows, even when the stamp does not change.
- 成果物: crates/build-stamp/src/lib.rs:48
- 根拠: Running `git add docs/x.md`, or `git status` from an IDE or a hook (it rewrites the index to refresh stat data), touches `.git/index`. The next `cargo test` re-runs build.rs and recompiles the whole server crate and everything that depends on it, although HEAD and the source directories are unchanged. The `.dirty` flag is already covered by watching the source directories, and commits by watching HEAD and the ref, so watching the index adds only cost.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed D6 —— 再現: `touch <git-dir>/index` の後の `cargo build -p ashiato-server` が `Compiling ashiato-server`（4 秒）。`index` を `rerun-if-changed` から外し、`st49_stamp_rerun_paths_follow_worktree_git_dir` で出さないことを固定した（d846989）。直した後は同じ手順で 0.13 秒・組み直しなし。design D6 の「印が同じなら組み直されない」の誤りも直した。`.dirty` は道と HEAD・参照で拾う（server 節 rc=0）

## R19. The doc comment says the commit falls back to `unknown` when git is unavailable, but `providers.exec` only ignores a non-zero exit code. If the `git` executable is missing, the exec throws and the Android build fails.
- 成果物: collector-android/app/build.gradle.kts:139
- 根拠: Build from a source tarball or in a CI or F-Droid container without git on PATH: `providers.exec { commandLine("git", ...) }` raises an ExecException while `buildCommit` / `buildDirty` are resolved, and `generateDebugBuildConfig` fails. Rust (`git()` returns None) and the web (try/catch) fall back to `unknown` in the same situation; C-01 does not.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed D5 —— 再現: git を除いた PATH で `./gradlew --no-daemon :app:generateDebugBuildConfig` が `A problem occurred starting process 'command 'git''` で rc=1。`providers.exec` を `ValueSource`（`GitOutput`。起動の失敗を `GradleException` で受けて空を返す）に替えた（d846989）。同じ手順で rc=0・`0.1.0+unknown.debug`、git がある場所では 40 桁の commit。android 節 rc=0

## R20. `commit_of` gets the ST48 ledger commit by re-parsing the formatted stamp string (split at the first '+', then trim the suffixes). The build-stamp crate never exposes the commit as a separate value.
- 成果物: crates/server/build.rs:94
- 根拠: If the workspace version ever carries build metadata (e.g. `0.1.0+rc1`), `split_once('+')` returns `rc1+<sha>` and `ASHIATO_BUILD_COMMIT` (written to `migration_ledger.applied.build_commit`) becomes wrong. `trim_end_matches` strips repeated suffixes, so an env commit ending in `.debug` or `.dirty` is also corrupted. Exposing the commit from `ashiato_build_stamp` (e.g. `stamp_parts()`) avoids the round-trip.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed D5 —— `ashiato_build_stamp::stamp_with_commit` が印と commit を組で返し、`crates/server/build.rs` の `commit_of` を消した（d846989）。`st49_stamp_with_commit_does_not_parse_the_stamp`（版番号 `0.1.0+rc1`・環境変数 `ci-1.dirty`）を足した。`cargo test -p ashiato-build-stamp` 8 passed

## R21. Sender now sends the stored line's JSON as-is instead of re-encoding the typed item. That drops the normalization re-encoding used to do for lines written by older app versions, in every outbox (records, heartbeats, drops), not only the stamped one.
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/Sender.kt:178
- 根拠: A line written by an older C-01 that lacks a field the current type fills by default (for example heartbeat `blockers` or later-added nullable IngestRequest fields) used to go out with `"blockers":[]` or `null` filled in by `encodeDefaults=true`/`explicitNulls=true`. Now the bytes go out unchanged. It works today only because the server has `#[serde(default)]` on those fields. A future required-with-default field would be rejected as `malformed` for queued old lines, and nothing tests that.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed D3 仮 —— 行をそのまま送るのをやめ、型に通した 1 件に行だけが持つ欄を足して送る（`withUnknownFields`。同じ名前は型の側）（d846989）。知らない欄を残す D3（仮）の振る舞いは変えない。`St49OutboxTest.oldLineGetsCurrentDefaults`（`unit_system` を持たない古い行が `null` で埋まる）を足し、行をそのまま送る形に戻すと落ちることを確かめた。`unknownFieldSurvivesResend`・`oldLineIsSentWithoutVersion` は緑のまま

## R22. The rule 'store the collector version only for the API token' is written out twice as the same `match writer { Collector => req.component_version(), Web | Internal => None }` (drops.rs and media_missing.rs:143), next to two identical `component_version()` wrapper impls.
- 成果物: crates/server/src/drops.rs:280
- 根拠: A third collector-only endpoint, or a change in how `Writer` maps (e.g. satellites start counting as collectors), has to be updated in several places by hand. A `Writer::collector_version(Option<&str>) -> Option<String>` helper, or reusing `Writer::split(..).0`, would keep the D9 routing in one place next to `split`.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed D9 —— `Writer::collector_version`（`split` の 1 つ目）を足し、drops.rs・media_missing.rs の 2 つの `match` をそれに替えた（d846989）。`drops_tests` 15・`media_missing_tests` 6・`st49_` 45 passed

## R23. The only check that the build stamp is regenerated after commits and dirty edits (`tools/check-build-stamp.sh`) is documented as 'CI には未組み込み' and replaced by a rule to run it by hand ('触ったら手元で走らせる').
- 成果物: docs/testing.md:14
- 根拠: CLAUDE.md says: 「機械が判定できることは機械に決めさせる。規約に書く・人間に聞くで代替しない」. Someone who changes `build.rs`, `crates/build-stamp` or `app/build.gradle.kts` and forgets the manual run ships a stale or wrong stamp (for example the Gradle configuration cache keeping an old commit), and no CI job catches it.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 1.2 —— CI に `build-stamp` job（4 節）を足した。重いので `changes` に `stamp` の旗（`crates/build-stamp/`・2 つの `build.rs`・`app/build.gradle.kts`・この台本・`Cargo.*`・`.github/`）を足し、印の作り方を触ったときだけ走らせる。`docs/testing.md` の §1 の行と job の表を直した（d846989）。台本の clone は `--no-hardlinks` にした（置き場が別の filesystem だと `--local` の hardlink が落ちる。手元で再現）

## R24. The header comment says '`migrate()` は起動のたびに全版を当てる', but since ST48 `migrate` applies only versions missing from the ledger, and the server start (`run()`) does not call it.
- 成果物: migrations/202610251000_record_component_version.sql:2
- 根拠: A future author trusts the comment and writes or edits migrations to survive per-startup re-application (as this branch did to 202610231200). That causes needless edits to merged migrations and the fingerprint-mismatch warnings described above.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed D19 —— 移行の冒頭を「`migrate()` は版の記録に無い版だけを当てる（ST48）が、記録の無い古い DB には全版を当て、`check-immutable.sh` も 2 回当てる」に直した（d846989）

## 再レビュー（7da9d42..47f6eda。2026-10-29）

final の後の 3 席の指摘（R14〜R24）の直しを、範囲を限って 1 回見た（`pr-review-toolkit:code-reviewer`）。指摘なし。
R15・R16・R18〜R24 の処置は差分に入っている。R16 は `STAMP_SOURCES` を `crates/server/src` に狭めると server 節が落ちる（`ORDER` の `rerun-if-changed` で build script は走り直すが `.dirty` が付かない）。
R18 で拾えなくなるのは index だけで状態が変わる場面（`git rm --cached` など）で、design D6 に書いてある。
古いままだった注記 2 か所（design D6 の C-01 の `providers.exec` → `ValueSource`、`St49OutboxTest.kt` の頭書き）を直した。
`build-stamp` job の `stamp` の旗が `collector-android/gradle.properties`・`settings.gradle.kts`・各 crate の `Cargo.toml` を含まないことは気づきとして残す（今壊れる経路は無い）。
