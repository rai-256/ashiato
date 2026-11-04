# ST54 code review

## final review（8184a7d..3b119b6）

final reviewer（`superpowers:requesting-code-review` の `code-reviewer.md`）の指摘。Critical 1 / Important 3 / Minor 多数。判定は「With fixes」。
reviewer が手元で走らせたもの: `python3 scripts/check_scenarios.py . st54-dev-side-by-side` rc=0（ST54 の警告 0）、一時ディレクトリで `tools/check-migrations.sh --dir` に LINEAGE の 2 通りを通す実験。cargo test の全体と gradle は走らせていない。
Task ごとの ⚠️ は final reviewer が全体で確かめた。成り立ったのは Task 3・11（DB の競合 → R1）、Task 5（rewritten の引き方 → R2）、Task 3 F7・F8（→ R4）。
解消済み・当たらない: Task 1（系統は `ashiato_build_stamp::channel` 1 か所）、Task 2（st54_bin_ 試験は置かれた）、Task 4・9（印は spec と一致、check_scenarios OK）、Task 9（7 日前の知らせに系統の文を添えている）、Task 10（package-linux は release.yml と同じ手順。`sign-apk.sh:41-43` が指紋を比べる）。
申し送り（ST51 premises 9・ST52 Q4/Q11・R11・R26）は満たされている。

## R1. `st54_bin_channel_environment_does_not_override_build_channel` が共有の DB を開発版の DB に変え、サーバ試験の結果が実行順で変わる
- 成果物: crates/server/tests/server_startup.rs
- 根拠: crates/server/tests/server_startup.rs:60-123。この 1 本だけ使い捨ての DB（st48 の Rig）でなく素の `DATABASE_OWNER_URL` / `DATABASE_URL` に開発版の migrate を当て、錠 `MIGRATING` も取らず、決め打ちの `127.0.0.1:18788` へ繋ぐ。正式版の `migrate_as_owner()` が先なら exit 4、後なら以後の正式版の試験が exit 4。Task 11 の報告の「channel_mismatch で失敗」と符合 / Task 3・11 ⚠️
- kind: technical
- 処置: (fix 後に記入)

## R2. LINEAGE の `rewritten` の 1 列目を、2 つの検査が違う名前で読んでいる（名前と中身を両方変えた移行はどう書いても通らない）
- 成果物: tools/check-nightly-migrations.sh
- 根拠: tools/check-nightly-migrations.sh:83 は旧名で `lineage_has_rewritten` を引く。tools/check-migrations.sh:95-99 は `rewritten` の 1 列目が今の ORDER にあることを求める。ORDER={b,c} で `renamed a b`+`rewritten a c` は check-migrations rc=1、`renamed a b`+`rewritten b c` は check-nightly-migrations が NG / Task 5 ⚠️
- kind: technical
- 処置: (fix 後に記入)

## R3. サーバの実バイナリ試験が呼び出しごとに空の target から依存ごと組み直す
- 成果物: crates/server/tests/server_startup.rs
- 根拠: server_startup.rs:32-57 の `custom_server_binary_with_version` を 12 か所が呼び、毎回新しい tempdir を `CARGO_TARGET_DIR` にして `cargo build`。CI の時間とメモリを食い、Task 11 のタイムアウトの原因とみられる
- kind: technical
- 処置: (fix 後に記入)

## R4. 開発版の migrate で空の DB を作る試験が spec の THEN の半分（全行の `-dev.`・続く起動の `/healthz` 200）を見ていない
- 成果物: crates/server/tests/server_startup.rs / crates/server/src/st54_tests.rs
- 根拠: server_startup.rs:1017-1033 は最初の 1 行だけを見る（Task 3 F7）。st54_tests.rs:342 `st54_empty_database_becomes_dev` は UPDATE で書いた値を assert するだけで自明に通る（Task 3 F8）
- kind: technical
- 処置: (fix 後に記入)

## R5. 退避の後のログが開発版でも「置き場は既定 backups/pre-migrate」と出る
- 成果物: crates/server/src/lib.rs
- 根拠: crates/server/src/lib.rs:394 / Task 2 F6
- kind: technical
- 処置: (fix 後に記入)

## R6. 系統の知らせを消すのに失敗すると、`notified_at` を先に消したまま戻り、以後は消し直さない
- 成果物: crates/collector-windows/src/channel_notice.rs
- 根拠: channel_notice.rs:61-66（`take()` の後に `dismiss_channel` が Err で return）/ Task 7 F4
- kind: technical
- 処置: (fix 後に記入)

## R7. 携帯の系統の違いだけの行に「（未送信 0 日分を携帯に残しています）」が付く
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/MainActivity.kt
- 根拠: MainActivity.kt:396-398・470（days=0 固定の notice.line で描く）/ Task 9 F16
- kind: technical
- 処置: (fix 後に記入)

## R8. 印 nightly の ORDER を取り出せない Scenario が rc しか見ていない
- 成果物: tools/check-nightly-migrations.sh
- 根拠: check-nightly-migrations.sh:152 の self-test が expect 1 だけで「印 nightly の migrations/ORDER を取り出せない」を assert しない / Task 5 F6
- kind: technical
- 処置: (fix 後に記入)

## R9. 移行の台帳の公開関数に doc コメントが無い
- 成果物: crates/server/src/migration_ledger.rs
- 根拠: `apply_migrations_recorded_with_lineage`・`compare_with_lineage`・`startup_refusal_with_lineage`・`LineageKind` / Task 4 F8・F9
- kind: technical
- 処置: (fix 後に記入)

## R10. 系統を何度も求め直している（D1 の「1 回求めて持ち物にする」とずれる）
- 成果物: crates/server/src/lib.rs / crates/collector-windows/src/main.rs / config.rs
- 根拠: lib.rs で `channel(env!("ASHIATO_VERSION"))` が 4 か所、収集アプリは main.rs:14 と config.rs:101 / Task 2 F7・F8、Task 6 F7
- kind: technical
- 処置: (fix 後に記入)

## R11. 見出しの知らない値の扱いが Rust と Kotlin で違う
- 成果物: crates/collector-windows/src/sender.rs / ContractGate.kt
- 根拠: reviewer は sender.rs:85-91 が `beta` を `None` にして確かめを素通りすると読んだ / Task 7 F6
- kind: technical
- 処置: rejected: sender.rs:85-91 は `and_then` の後に `.or(Some(Channel::Official))` が付き、`beta` は `None` → `Some(Official)`、見出し無しも `Some(Official)`。ContractGate.kt:59-61（知らない値は Official）と一致し、D14（仮）の「開発版とそれ以外の 2 つ」どおり。`""` の枝は `.or` と重なるだけで振る舞いは同じ

## R12. 系統が違う間、送る契機のたびに `channel_mismatch` のログを出し、間引かない
- 成果物: crates/collector-windows/src/runtime.rs
- 根拠: runtime.rs:832-842 / Task 7 F5
- kind: technical
- 処置: (fix 後に記入)

## R13. 中身の重なる単体試験と名前の整理（channel_notice の 13 本・`st54_red_tests`・404 の複製）
- 成果物: crates/collector-windows/src/channel_notice.rs / sender.rs / runtime.rs
- 根拠: channel_notice.rs:164-271（404 / 読めない 200 / 401 の 3 本は同じ）/ sender.rs の mod 名 `st54_red_tests` / runtime.rs:586 `st54_channel_runtime_notices_are_independent` は 404 の試験の複製で、系統の通知を消しても契約の通知が消えないことを見ていない / Task 7 F7・F9
- kind: technical
- 処置: (fix 後に記入)

## R14. 携帯の系統の違いの通知が ST50 の通知のチャンネル `contract` に載る
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/ContractNotifier.kt
- 根拠: ChannelMismatchNotifier のチャンネル / Task 9 F10
- kind: technical
- 処置: rejected: design D12（仮）が「ST50 と同じ通知のチャンネル `contract`、別の通知の id」と決めている。設計どおり

## R15. 携帯の自分の系統の読み口が 2 つあり、`channelMismatchHeadline()` の既定引数が BuildConfig を直接読む
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/ContractGate.kt / ContractNotifier.kt
- 根拠: ContractGate.kt:33 の `localChannel` と `buildLocalChannel()` の重複、ContractNotifier.kt:325 の既定引数（試験が差し替えられない）/ Task 9 F15
- kind: technical
- 処置: (fix 後に記入)

## R16. 系統の違いの後に 404 が来ても `channelMismatch` が残る
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/ContractGate.kt
- 根拠: ContractGate.kt:235 付近の `NoEndpoint` の枝
- kind: technical
- 処置: rejected: openspec/changes/st54-dev-side-by-side/specs/distribution/spec.md:344 は 404 を「系統の判定に使わず」と決め、design.md:165（D12（仮））は知らせを「同じ系統のサーバが答えた契機で消す」と決めている。404 で系統の違いを消すのは 404 を系統の判定に使うことになる。spec の Scenario「契約の問い合わせが 404 なら、携帯の収集アプリは系統の違いの知らせでなく更新を促す」の GIVEN は系統の違いを持たない状態で、この端は当たらない

## R17. publish-nightly.sh が復元した鍵ファイルを使わず、check-artifacts に渡す指紋の名前が DEV と揃わない
- 成果物: tools/release/publish-nightly.sh / .github/workflows/nightly.yml
- 根拠: publish-nightly.sh:24-29、nightly.yml:83-84 が同じ変数を 2 つの名前で渡す（draft.sh:75 は DEV の名前）/ Task 10 F8
- kind: technical
- 処置: (fix 後に記入)

## R18. 名前を変えた exe の一時ディレクトリを repo の根に作る
- 成果物: tools/release/build.sh
- 根拠: tools/release/build.sh:84（`$TMPDIR` でない）/ Task 6 F9
- kind: technical
- 処置: (fix 後に記入)

## R19. LINEAGE の `removed` の行を完全一致の grep で読み、行末のコメントや空白で外れる
- 成果物: tools/check-nightly-migrations.sh
- 根拠: Task 5 F3
- kind: technical
- 処置: (fix 後に記入)

## R20. `toast_names.rs` の試験が本体より前にある
- 成果物: crates/collector-windows/src/toast_names.rs
- 根拠: toast_names.rs:1 / Task 6 F13
- kind: technical
- 処置: (fix 後に記入)
