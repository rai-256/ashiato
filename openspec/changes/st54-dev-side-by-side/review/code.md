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
- 処置: fixed 2.1 （c70b4d3e）

## R2. LINEAGE の `rewritten` の 1 列目を、2 つの検査が違う名前で読んでいる（名前と中身を両方変えた移行はどう書いても通らない）
- 成果物: tools/check-nightly-migrations.sh
- 根拠: tools/check-nightly-migrations.sh:83 は旧名で `lineage_has_rewritten` を引く。tools/check-migrations.sh:95-99 は `rewritten` の 1 列目が今の ORDER にあることを求める。ORDER={b,c} で `renamed a b`+`rewritten a c` は check-migrations rc=1、`renamed a b`+`rewritten b c` は check-nightly-migrations が NG / Task 5 ⚠️
- kind: technical
- 処置: fixed 5.1 （c70b4d3e）

## R3. サーバの実バイナリ試験が呼び出しごとに空の target から依存ごと組み直す
- 成果物: crates/server/tests/server_startup.rs
- 根拠: server_startup.rs:32-57 の `custom_server_binary_with_version` を 12 か所が呼び、毎回新しい tempdir を `CARGO_TARGET_DIR` にして `cargo build`。CI の時間とメモリを食い、Task 11 のタイムアウトの原因とみられる
- kind: technical
- 処置: fixed 2.1 （c70b4d3e）

## R4. 開発版の migrate で空の DB を作る試験が spec の THEN の半分（全行の `-dev.`・続く起動の `/healthz` 200）を見ていない
- 成果物: crates/server/tests/server_startup.rs / crates/server/src/st54_tests.rs
- 根拠: server_startup.rs:1017-1033 は最初の 1 行だけを見る（Task 3 F7）。st54_tests.rs:342 `st54_empty_database_becomes_dev` は UPDATE で書いた値を assert するだけで自明に通る（Task 3 F8）
- kind: technical
- 処置: fixed 3.1 （c70b4d3e）

## R5. 退避の後のログが開発版でも「置き場は既定 backups/pre-migrate」と出る
- 成果物: crates/server/src/lib.rs
- 根拠: crates/server/src/lib.rs:394 / Task 2 F6
- kind: technical
- 処置: fixed 2.1 （c70b4d3e）

## R6. 系統の知らせを消すのに失敗すると、`notified_at` を先に消したまま戻り、以後は消し直さない
- 成果物: crates/collector-windows/src/channel_notice.rs
- 根拠: channel_notice.rs:61-66（`take()` の後に `dismiss_channel` が Err で return）/ Task 7 F4
- kind: technical
- 処置: fixed 7.1 （c70b4d3e）

## R7. 携帯の系統の違いだけの行に「（未送信 0 日分を携帯に残しています）」が付く
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/MainActivity.kt
- 根拠: MainActivity.kt:396-398・470（days=0 固定の notice.line で描く）/ Task 9 F16
- kind: technical
- 処置: fixed 9.2 （c70b4d3e）

## R8. 印 nightly の ORDER を取り出せない Scenario が rc しか見ていない
- 成果物: tools/check-nightly-migrations.sh
- 根拠: check-nightly-migrations.sh:152 の self-test が expect 1 だけで「印 nightly の migrations/ORDER を取り出せない」を assert しない / Task 5 F6
- kind: technical
- 処置: fixed 5.1 （c70b4d3e）

## R9. 移行の台帳の公開関数に doc コメントが無い
- 成果物: crates/server/src/migration_ledger.rs
- 根拠: `apply_migrations_recorded_with_lineage`・`compare_with_lineage`・`startup_refusal_with_lineage`・`LineageKind` / Task 4 F8・F9
- kind: technical
- 処置: fixed 4.1 （c70b4d3e）

## R10. 系統を何度も求め直している（D1 の「1 回求めて持ち物にする」とずれる）
- 成果物: crates/server/src/lib.rs / crates/collector-windows/src/main.rs / config.rs
- 根拠: lib.rs で `channel(env!("ASHIATO_VERSION"))` が 4 か所、収集アプリは main.rs:14 と config.rs:101 / Task 2 F7・F8、Task 6 F7
- kind: technical
- 処置: fixed 2.1 （c70b4d3e）

## R11. 見出しの知らない値の扱いが Rust と Kotlin で違う
- 成果物: crates/collector-windows/src/sender.rs / ContractGate.kt
- 根拠: reviewer は sender.rs:85-91 が `beta` を `None` にして確かめを素通りすると読んだ / Task 7 F6
- kind: technical
- 処置: rejected: sender.rs:85-91 は `and_then` の後に `.or(Some(Channel::Official))` が付き、`beta` は `None` → `Some(Official)`、見出し無しも `Some(Official)`。ContractGate.kt:59-61（知らない値は Official）と一致し、D14（仮）の「開発版とそれ以外の 2 つ」どおり。`""` の枝は `.or` と重なるだけで振る舞いは同じ

## R12. 系統が違う間、送る契機のたびに `channel_mismatch` のログを出し、間引かない
- 成果物: crates/collector-windows/src/runtime.rs
- 根拠: runtime.rs:832-842 / Task 7 F5
- kind: technical
- 処置: fixed 7.1 （c70b4d3e）

## R13. 中身の重なる単体試験と名前の整理（channel_notice の 13 本・`st54_red_tests`・404 の複製）
- 成果物: crates/collector-windows/src/channel_notice.rs / sender.rs / runtime.rs
- 根拠: channel_notice.rs:164-271（404 / 読めない 200 / 401 の 3 本は同じ）/ sender.rs の mod 名 `st54_red_tests` / runtime.rs:586 `st54_channel_runtime_notices_are_independent` は 404 の試験の複製で、系統の通知を消しても契約の通知が消えないことを見ていない / Task 7 F7・F9
- kind: technical
- 処置: fixed 7.1 （c70b4d3e）

## R14. 携帯の系統の違いの通知が ST50 の通知のチャンネル `contract` に載る
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/ContractNotifier.kt
- 根拠: ChannelMismatchNotifier のチャンネル / Task 9 F10
- kind: technical
- 処置: rejected: design D12（仮）が「ST50 と同じ通知のチャンネル `contract`、別の通知の id」と決めている。設計どおり

## R15. 携帯の自分の系統の読み口が 2 つあり、`channelMismatchHeadline()` の既定引数が BuildConfig を直接読む
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/ContractGate.kt / ContractNotifier.kt
- 根拠: ContractGate.kt:33 の `localChannel` と `buildLocalChannel()` の重複、ContractNotifier.kt:325 の既定引数（試験が差し替えられない）/ Task 9 F15
- kind: technical
- 処置: fixed 9.2 （c70b4d3e）

## R16. 系統の違いの後に 404 が来ても `channelMismatch` が残る
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/ContractGate.kt
- 根拠: ContractGate.kt:235 付近の `NoEndpoint` の枝
- kind: technical
- 処置: rejected: openspec/changes/st54-dev-side-by-side/specs/distribution/spec.md:344 は 404 を「系統の判定に使わず」と決め、design.md:165（D12（仮））は知らせを「同じ系統のサーバが答えた契機で消す」と決めている。404 で系統の違いを消すのは 404 を系統の判定に使うことになる。spec の Scenario「契約の問い合わせが 404 なら、携帯の収集アプリは系統の違いの知らせでなく更新を促す」の GIVEN は系統の違いを持たない状態で、この端は当たらない

## R17. publish-nightly.sh が復元した鍵ファイルを使わず、check-artifacts に渡す指紋の名前が DEV と揃わない
- 成果物: tools/release/publish-nightly.sh / .github/workflows/nightly.yml
- 根拠: publish-nightly.sh:24-29、nightly.yml:83-84 が同じ変数を 2 つの名前で渡す（draft.sh:75 は DEV の名前）/ Task 10 F8
- kind: technical
- 処置: fixed 10.3 （c70b4d3e）

## R18. 名前を変えた exe の一時ディレクトリを repo の根に作る
- 成果物: tools/release/build.sh
- 根拠: tools/release/build.sh:84（`$TMPDIR` でない）/ Task 6 F9
- kind: technical
- 処置: fixed 6.2 （c70b4d3e）

## R19. LINEAGE の `removed` の行を完全一致の grep で読み、行末のコメントや空白で外れる
- 成果物: tools/check-nightly-migrations.sh
- 根拠: Task 5 F3
- kind: technical
- 処置: fixed 5.1 （c70b4d3e）

## R20. `toast_names.rs` の試験が本体より前にある
- 成果物: crates/collector-windows/src/toast_names.rs
- 根拠: toast_names.rs:1 / Task 6 F13
- kind: technical
- 処置: fixed 6.1 （c70b4d3e）

## R21. ブランチの Rust が CI の `cargo fmt --all --check` と `cargo clippy --workspace --all-targets -- -D warnings` を通らない
- 成果物: crates/collector-windows/src/config.rs・main.rs・runtime.rs / crates/server/src/lib.rs・migration_ledger.rs・st54_tests.rs
- 根拠: .github/workflows/ci.yml:77-78。c70b4d3e で `cargo fmt --all --check` が crates/collector-windows/src/main.rs:22 で差分、clippy は config.rs:244-345（試験の unwrap_used）・main.rs:226-233・runtime.rs:1366・lib.rs:4439・migration_ledger.rs:615・st54_tests.rs:29 で error（.superpowers/sdd/logs/20261104-212325-clippy.log）。どれも ST54 が触ったファイル。fix の席が「範囲の外」とした分を controller が確かめて足した
- kind: technical
- 処置: fixed 2.1 （8490cfad）

## code-verify（8184a7d..8490cfa）

独立の席が実行して確かめた。作業ツリーのコードは触っていない（手 7 だけ作業ツリーで一時的に壊して戻した。前後の `git status --porcelain` と `git diff` は一致）。
手 2・4 の変異は、作業ツリーを写した使い捨ての木（`.superpowers/sdd/tmp/cv/mut`。`ASHIATO_DB_PORT=55554` でこの worktree の DB を指す）で 1 本ずつ壊して走らせ、戻した。

### 申告と実測

申告: tasks.md の `[x]` は 20/23（未チェックは 8.1 Windows・12.1・12.2 人間の確認）。検証コマンドは `[x]` の 20 件で 35 本（2.1 と 3.1 の 1 本目の `cargo test -p ashiato-server st54_` は同じコマンドで下限だけが `≥7` / `≥13` と違うので、`≥13` で 1 回に走らせた。実行は 34 回）。

| 項目 | 検証コマンド（要点） | 申告 | 実測 |
|---|---|---|---|
| 1.1 | `cargo test -p ashiato-build-stamp st54_` + `--list ≥1` | rc=0 | rc=0（1 本）一致 |
| 2.1 | `cargo test -p ashiato-server st54_` + `--list ≥7` / `check-openapi.sh` / `cargo test … st50_` | rc=0 | rc=0 / rc=0 / rc=0（st50_ は 42 本）一致 |
| 3.1 | `st54_` `--list ≥13`（26 本）/ `--test server_startup st54_bin_` `≥2`（9 本）/ `migration_ledger`（24 本）/ `--test server_startup st48_` | rc=0 | 4 本とも rc=0 一致 |
| 3.2 | `docs/handoff/ST58.md` の 4 つの grep | rc=0 | rc=0 一致 |
| 4.1 | `st54_lineage` `≥3`（3 本） | rc=0 | rc=0 一致 |
| 4.2 | `check-migrations.sh --self-test` / 本番 | rc=0 | rc=0 / rc=0 一致 |
| 5.1 | `check-nightly-migrations.sh --self-test` / 本番 | rc=0 | rc=0 / rc=0 一致（本番は印 `nightly` が無く「印なし」で通る。手 2） |
| 5.2 | ci.yml の grep / actionlint | rc=0 | rc=0 / rc=0 一致 |
| 6.1 | `ashiato-collector-windows st54_` `≥10`（35 本）/ 全体 | rc=0 | rc=0（全体 361 本）一致 |
| 6.2 | `build.sh --self-test` | rc=0 | rc=0 一致 |
| 7.1 | `st54_channel` `≥14`（24 本）/ `st50_`（36 本） | rc=0 | rc=0 / rc=0 一致 |
| 9.1 | `check-artifacts.sh --self-test-android` | rc=0 | rc=0 一致 |
| 9.2 | `ChannelGateTest` XML `≥10` / `--tests *St50*`（訂正後。41 本） | rc=0 | rc=0 / rc=0 一致 |
| 9.3 | `tools/android-side-by-side.sh`（エミュレータ） | rc=0 | rc=0 一致（「正式版の未送信 1 件を確認した」） |
| 10.1 | `check-artifacts.sh --self-test` | rc=0 | rc=0 一致 |
| 10.2 | `publish-nightly.sh --self-test` / `sign-apk.sh --self-test` | rc=0 | rc=0 / rc=0 一致 |
| 10.3 | `check-guards.sh --self-test` / `workflows` / actionlint nightly.yml | rc=0 | 3 本とも rc=0 一致 |
| 10.4 | `draft.sh --self-test` | rc=0 | rc=0 一致 |
| 11.1 | `docs/release.md` と `CLAUDE.md` の grep | rc=0 | rc=0 一致 |
| 11.2 | `check_scenarios.py . st54-dev-side-by-side` / 消した 3 つの印の grep | rc=0 | rc=0（`scenarios: OK`）/ rc=0 一致 |
| 8.1 | Windows で `--test runtime_windows st54_` | 未チェック | 未実行（Linux の席。試験 `st54_side_by_side_autostart_and_aumid` は `tests/runtime_windows.rs:54` にある） |

申告と実測の rc はすべて一致した。ずれは下の R22〜R28（主張の階層と試験の階層の違い）。

### 手 1: 固定値を独立に再計算する

該当なし（確かめた範囲: ST54 の差分の `+` 行で 32 桁以上の 16 進・指紋の形の literal を grep。出たのは `actions/setup-java@c5195efe… # v4` と試験の入力の偽の commit `0000…`（期待値ではない）だけ。前者は `git ls-remote https://github.com/actions/setup-java 'refs/tags/v4*'` で `refs/tags/v4.7.1` と一致した）。

### 手 2: ガードをわざと壊す

| 壊したもの（使い捨ての木） | 検査 | 結果 |
|---|---|---|
| 印 `nightly` を打ち、最後の移行の名前を変え LINEAGE なし | `check-nightly-migrations.sh` | rc=1「nightly の版にあるが、今の ORDER に無い」 |
| 同じに `renamed` を足す | 同上 | rc=0（通るべき） |
| さらに中身を 1 行変える | 同上 | rc=1「中身が nightly の版から変わっている（直しの移行を足して rewritten を書く）」 |
| nightly.yml から `build.sh apk` を消す | `check-guards.sh workflows` | rc=1 |
| nightly.yml から `ASHIATO_EXPECT_DEV_CERT_SHA256` を消す | 同上 | rc=1 |
| nightly.yml の publish から `environment:` / `name: nightly` を消す | 同上 | rc=1 |
| Environment の名前を `nightly-open` にする | 同上 | **rc=0**（→ R28） |
| LINEAGE に `removed <名前>   # コメント` | `check-migrations.sh` / `cargo build -p ashiato-server` | rc=0 / **rc=101**（→ R27） |

検査の外側: `git ls-remote --tags origin nightly` は 0 行（rc=0）。印 `nightly` はまだ無く、CI の `check-nightly-migrations.sh` は初回の公開まで「印なし rc=0」で通る。spec どおり（印が無ければ通る）だが、Q11 の検査はいま何も比べていない。

## R22. 「開発版のサーバの画面のログインは、開発版の名前のクッキーだけを使う」の印は「だけ」を観測していない。開発版が `ashiato_session` も受け付けても全部緑
- 成果物: crates/server/src/st54_tests.rs / crates/server/src/web_session.rs
- 根拠: spec（specs/distribution/spec.md:642-645）は「**その値を** `ashiato_session` の名前で付けて 401」。st54_tests.rs:258-268 は値 `wrong-channel`（DB に無い印）を付けて 401 を見るので、名前でなく値で落ちている。変異: web_session.rs:199 を `(k == cookie_name(channel) || k == COOKIE_NAME)` にして `cargo test -p ashiato-server --lib st54_` → 17 passed / 0 failed、`--lib session` → 32 passed / 0 failed。また（読みの所見。変異は未実施）Set-Cookie も `headers()[SET_COOKIE]`（先頭 1 本）の `starts_with` だけで、`ashiato_session=` を書かないことは見ていない
- kind: technical
- 提案: ログインで得た印の値を `ashiato_session=<同じ値>` で付けて 401 を見る。`get_all(SET_COOKIE)` に `ashiato_session=` が無いことも見る
- 処置: fixed 2.1 （st54_dev_web_login_uses_dev_cookie が、ログインで得た印の値を `ashiato_session` の名前で付けて 401、`get_all(SET_COOKIE)` に `ashiato_session=` が無いことを見る。web_session.rs の照合に `|| k == "ashiato_session"` を足す変異で 1 failed）

## R23. 「版の記録の行が 1 つも無い DB には、どちらの系統の migrate も当てる」の 2 本の印は、spec の WHEN（開発版の migrate）を一度も走らせていない
- 成果物: crates/server/src/st54_tests.rs / crates/server/tests/server_startup.rs / crates/server/src/migration_ledger.rs
- 根拠: spec:518-521 は「開発版の組み立てで migrate → 4 でなく、行の版番号が `-dev.`」。st54_tests.rs:405-417 と server_startup.rs:1091-1100 はどちらも行を消した後に**正式版**の migrate を当て、版番号を見ない。変異: migration_ledger.rs:366 の行 0 の枝を `Ok(Some(Channel::Official))` にして `--lib st54_` 17 passed、`--test server_startup st54_bin_` 9 passed（全部緑）。同じ変異の下で、正式版で当てて行を消した DB に開発版の bin で migrate する一時の試験を足すと `assertion left != right failed: error: kind=channel_mismatch` で落ち、変異を戻すと通った（試験は消した）
- kind: technical
- 提案: 行を消した DB に開発版の bin で migrate し、rc≠4 と全行の `-dev.` を見る（逆向きの正式版は今の試験のまま残す）
- 処置: fixed 3.1 （st54_empty_ledger_allows_both_channels と st54_bin_empty_ledger_allows_either_channel が、正式版で当てて行を消した DB に開発版の migrate を当てる。bin は rc≠4 と全行の `-dev.` も見る。行 0 の枝を `Some(Official)` にする変異で両方 1 failed）

## R24. D3 の「版の記録の**最初の行**（`applied_at` の順、同じなら名前の順）で系統を決める」を、値を変えても全部通る
- 成果物: crates/server/src/migration_ledger.rs
- 根拠: migration_ledger.rs:368-370 の `ORDER BY applied_at, name` を `ORDER BY applied_at DESC, name DESC`（最後の行）にして `--lib st54_` 17 passed・`--lib migration_ledger` 24 passed・`--test server_startup st54_bin_` 9 passed。どの試験の DB も全行が同じ系統なので、どの行を読んでも同じになる
- kind: technical
- 提案: 最初の行と後の行で版番号の系統が違う DB（行を直接書く）を作り、最初の行の系統で判定することを見る
- 処置: fixed 3.1 （st54_first_ledger_row_decides_channel: 最初の行だけ系統の違う DB を作り、名前の順と `applied_at` の順の両方を見る。`ORDER BY applied_at DESC, name DESC` と `name DESC` の変異でそれぞれ 1 failed）

## R25. 携帯の「系統の違いを通知で知らせる」は、送る契機（Drainer）から通知への配線をどの試験も通らない。配線を消しても 125 本緑
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/Retention.kt / LocationService.kt / ChannelGateTest.kt
- 根拠: ChannelGateTest.kt の `mismatch has persisted line and separate notification` は `ChannelMismatchNotifier(...).update(true)` を試験が自分で呼ぶ。ChannelGateTest の `Drainer` は `onChannelMismatch` を渡していない。`grep -rln "onChannelMismatch\|ChannelMismatchNotifier\|channelMismatch" collector-android/app/src/test collector-android/app/src/androidTest` は ChannelGateTest.kt だけ。変異: Retention.kt:287-289 の `runCatching { onChannelMismatch(guard.gate.channelMismatch) }` を消して `:app:testDebugUnitTest --tests '*Channel*' '*Drainer*' '*Retention*' '*LocationService*' '*St50*' '*Contract*'` → rc=0、13 クラス 125 本緑。Q13 の答え（知らせる）は、気づかずに 90 日の上限で捨てられる（discarded）のを防ぐための決定で、その知らせが出なくても試験は気づかない
- kind: technical
- 提案: ChannelGateTest の Drainer に本物と同じ `onChannelMismatch = { notifier.update(it) }` を渡し、`drainer.tick()` だけで通知の id が出ることを見る（LocationService の組み立てを試験から引ければなお良い）
- 処置: fixed 9.2 （ChannelGateTest の Drainer に本物と同じ `onChannelMismatch = { notifier.update(it) }` を渡し、試験は通知の係を自分で呼ばない。Retention.kt の配線を消す変異で 14 本中 2 failed）

## R26. 「待ち受けと退避の置き場を明示で渡せば、開発版でもその値を使う」は純粋関数の段でしか見ておらず、開発版の migrate が `MIGRATE_BACKUP_DIR` を無視しても全部緑（実データの退避が別の置き場に書かれた）
- 成果物: crates/server/src/st54_tests.rs / crates/server/src/migrate_backup.rs / crates/server/tests/server_startup.rs
- 根拠: st54_tests.rs:273-283 は `bind_for` / `dir_for` に値を渡すだけ。spec:647-650 の WHEN は環境変数を渡して読む。変異: migrate_backup.rs:33 を `std::env::var("MIGRATE_BACKUP_DIR").ok().filter(|_| channel != Channel::Dev)` にして `--lib st54_` 17 passed・`--test server_startup st54_bin_` 9 passed。そのとき開発版の bin の migrate は Rig の置き場でなく `crates/server/backups/pre-migrate-dev/pre-migrate-20261104T131459Z.dump`（git の無視の下）に試験 DB の dump を書いた（片付けた）。`BIND` は `st54_bin_channel_environment_does_not_override_build_channel` が 18788 が使用中のときだけ渡すので、空いている手元では観測されない
- kind: technical
- 提案: 開発版の bin の migrate に `MIGRATE_BACKUP_DIR=<使い捨て>` を渡して dump がそこに 1 つ増えることを見る。`BIND` は常に空いた port を渡す回を 1 本足す
- 処置: fixed 2.1 （st54_bin_explicit_bind_and_backup_dir_win_for_dev: 開発版の bin の migrate が渡した `MIGRATE_BACKUP_DIR` に退避を 1 つ書き、渡した `BIND`（空いた port）で待ち受けることを見る。migrate_backup.rs の変異で 1 failed）

## R27. LINEAGE の行末のコメントを、2 つのシェルの検査は読み飛ばして通すのに、`build.rs` は panic してビルドが落ちる
- 成果物: crates/server/build.rs / tools/check-migrations.sh / tools/check-nightly-migrations.sh
- 根拠: 使い捨ての木で `migrations/LINEAGE` に `removed 202001010000_gone   # 使わなくなった` を足し、`tools/check-migrations.sh` → 「マイグレーション OK」rc=0、`cargo build -p ashiato-server` → rc=101 `panicked at crates/server/build.rs:111:18: migrations/LINEAGE の行の形が違う`。check-migrations.sh:84 は `${line%%#*}` でコメントを外し、check-nightly-migrations.sh:177-180 の self-test は行末コメントつきの `removed` を「通る」と固定している（final review R19 の直し）。build.rs:103-111 は `#` で始まる行だけを外して `split_whitespace` するので 4 欄になる
- kind: technical
- 提案: build.rs でも `#` 以降を外してから欄を割る（3 つの読み手を同じ規則に揃え、build.rs の側にも行末コメントの試験を置く）
- 処置: fixed 4.1 （build.rs が行末の `#` 以降を外してから欄を割る。LINEAGE に `removed 202001010000_gone   # 使わなくなった` を足して `cargo build -p ashiato-server` rc=0 を確かめて戻した）

## R28. `check-guards.sh workflows` の「公開 job に Environment がある」は部分一致で、Environment の名前を変えても通る
- 成果物: tools/release/check-guards.sh
- 根拠: check-guards.sh:81 `job_block "$nightly" publish | grep -q 'name: nightly'`。使い捨ての木で nightly.yml の `name: nightly` を `name: nightly-open` にして `tools/release/check-guards.sh workflows` → rc=0。`environments` の検査（branch main だけ）が見るのは `nightly` の Environment なので、名前が違う Environment で走る publish はその守りの外に出る（ST54 が足した行。8184a7d の check-guards.sh に無い）
- kind: technical
- 提案: `name: nightly$`（行末まで）で照合し、self-test に名前を変えた偽を足す

### 手 3: Scenario と test の突き合わせ

`python3 scripts/check_scenarios.py . st54-dev-side-by-side` rc=0（ST54 の警告 0）。印の先を読んだ範囲: サーバ（st54_tests.rs 17 本・server_startup.rs の st54_bin_ 9 本）、PC（runtime.rs の st54_channel_ 15 本・config.rs の st54_ 8 本・toast_names.rs・channel_notice.rs）、携帯（ChannelGateTest 12 本）、release の self-test（rc とメッセージ）。

| Scenario | 印の先 | 判定 |
|---|---|---|
| 開発版のサーバの画面のログインは、開発版の名前のクッキーだけを使う | st54_dev_web_login_uses_dev_cookie | 不一致 → R22 |
| 版の記録の行が 1 つも無い DB には、どちらの系統の migrate も当てる | st54_empty_ledger_allows_both_channels / st54_bin_empty_ledger_allows_either_channel | 不一致 → R23 |
| 系統が違って送らない間、携帯の収集アプリは画面の行と通知で送り先を確かめるよう知らせる | ChannelGateTest `mismatch has persisted line…` | 通知は試験が自分で呼ぶ → R25 |
| 待ち受けと退避の置き場を明示で渡せば、開発版でもその値を使う | st54_explicit_bind_and_backup_dir_win_for_dev | 関数の段だけ → R26 |
| 空の DB に開発版の migrate を当てると、開発版の DB になる | unit は UPDATE で版を書き換えた上で見る。THEN の全部は st54_bin_dev_migrate_bootstraps_dev_db（全行の `-dev.`・`/healthz`）が持つ | 一致（bin に委ねている） |
| 系統の名前の環境変数を渡しても、系統は変わらない（PC の側） | config.rs st54_channel_environment_variable_is_not_read は系統を引数で渡し端末識別子を見る | 弱い（spec の THEN は自動起動の項目の名前）。main.rs:14 は `env!("ASHIATO_VERSION")` だけから系統を求め、`ASHIATO_CHANNEL` を読む口は無い（grep）ので指摘にしない |
| 系統の違いの知らせは、続く間 24 時間ごとに出し直す（PC） | st54_channel_runtime_notice_is_daily（0・23h・24h で 2 回）+ notice_state_is_persistent（23h で 1 回） | channel_notice.rs:76 の 24h を 23h にすると 2 本、25h にすると 2 本落ちた（23h30m では 24 本緑）。一致 |
| 送り先が開発版／正式版のサーバです（PC の文） | channel_notice.rs の st54_channel_mismatch_names_server… | 文を入れ替えると 1 本落ちた（360 passed / 1 failed）。一致 |
| 系統を答えない古いサーバは、正式版のサーバとみなす | st54_channel_runtime_old_server_is_official | sender.rs:91 の既定を `Channel::Dev` にすると `st54_` で 2 本落ちた（32 passed / 2 failed）。一致 |
| 上記以外の ST54 の印 | — | 読んだ範囲で主張と観測の階層は合っていた |

### 手 4: 本人の決定が test で固定されているか

| 決定 | 値 | 固定している試験 | 結果 |
|---|---|---|---|
| Q14 / D8 待ち受け | `127.0.0.1:18788` | st54_dev_defaults_are_separate | literal で固定 |
| Q14 クッキー | `ashiato_session_dev` | 同上・st54_dev_web_login_uses_dev_cookie | 名前は固定、「だけ」は R22 |
| Q14 退避 | `backups/pre-migrate-dev` | 同上・st54_backup_log_names_channel_default_dir | 既定は固定、明示の優先は R26 |
| Q14 DB のポート | 55433 | なし（`docs/release.md:110` の手順書だけ。コードに値が無い） | 試験の対象外 |
| Q14 実行ファイル | `ashiato-collector-windows-dev.exe` | build.sh --self-test | 固定 |
| Q7 名前 | `あしあと。開発版`・`ashiato 開発版`・差出人 `dev.ashiato.collector.dev`・`ashiato-collector-dev.cmd` | check-artifacts.sh --self-test-android・toast_names.rs・autostart.rs の試験 | 固定 |
| Q2 置き場と端末識別子 | `<置き場>/dev`・`-dev`・除外は渡された置き場 | config.rs の st54_ 8 本・runtime.rs:3072 | 固定 |
| Q3 / D3 系統の判定 | 最初の行の版番号 | なし | **値を変えても全部通る**（`ORDER BY` を逆にして 50 本緑）→ R24 |
| Q3 終了コード | 4・`error: kind=channel_mismatch` | st54_bin_* の `Some(4)`・`starts_with("error: kind=channel_mismatch\n")` | 固定 |
| Q8 説明文 | 1 行目「この開発版はサポートしません。」 | publish-nightly.sh --self-test | 固定 |
| Q12 両向きで送らない | — | st54_channel_runtime_dev_holds_official / official_holds_dev・ChannelGateTest の 2 本 | 固定 |
| Q13 / D12 知らせ・24 時間 | 24h | PC は 23h・25h に変えるとそれぞれ 2 本落ちる（23h30m は通る。粒度は 1 時間）。携帯は `ContractNotifier.REPEAT` を参照（ST50 の試験が固定） | 固定。携帯の配線は R25 |
| Q15 読む口は止めない | `GET /outages` | st54_channel_runtime_mismatch_still_polls_outages・ChannelGateTest `outage polling…` | 固定 |

### 手 5: tasks の `[x]` と実体

`[x]` の 20 件の 35 本（同じコマンドの 1 本をまとめて 34 回）をすべて走らせ、すべて rc=0（上の表）。件数の下限つきのものは `-- --list` の本数が下限以上（st54_ 26 ≥ 13、st54_bin_ 9 ≥ 2、st54_lineage 3 ≥ 3、PC st54_ 35 ≥ 10、st54_channel 24 ≥ 14、ChannelGateTest は XML で下限以上）。下限の無い `st50_`（サーバ 42 本・PC 36 本）・`migration_ledger`（24 本）・`*St50*`（41 本）も 0 本ではない。
`--test server_startup` は `crates/server/tests/server_startup.rs` が実在する。テスト名が挙がっていて実体の無いものは無かった。未チェックの 8.1 は試験が `tests/runtime_windows.rs:54` に置かれ、Windows の job を待つ（Linux では `#![cfg(windows)]` で組まれない）。

### 手 6: 隙間

新しい指摘なし（確かめた範囲）:
- 系統が違って送らない間の PC の未送信: `outbox.rs:4` が「上限も破棄も持たない」。捨てる経路は無い
- 携帯の未送信: 系統が違っても `Retention.kt` の 90 日 / 2 GB で捨てる。Q12 で本人が承知した損失で、気づく手がかりの知らせ（Q13）は R25 のとおり配線が試験の外
- PC の除外の登録: アプリは `exclusions.json` を書かない（`grep -rn "fn save\|fs::write" crates/collector-windows/src/exclusion.rs` は試験の行だけ）。開発版が渡された置き場を読む経路は runtime.rs:3072 と config.rs の試験が固定
- 開発版の DB: `docker-compose.yml` は名前つきの volume も bind mount も持たない（イメージの匿名 volume）。`-p ashiato-nightly` で container ごと分かれ、正式版のデータ領域を共有しない
- 退避からの戻し（ST58）: `docs/handoff/ST58.md` に終了コード 4 の節がある（3.2 rc=0）

### 手 7: 機械が承認した検証コマンドの訂正

`plan_fix.py pending` は 1 件（9.2 `2026-11-04T09:21:00+00:00`。`*ContractGate*` → `*St50*`）。
ContractGate.kt:64 の `HTTP_NOT_FOUND -> ContractAnswer.NoEndpoint` を `Transient` に変えて新しいコマンドを走らせると rc=1（41 tests completed, 6 failed。St50ContractNoticeTest の 404 の 3 本ほか）。戻した後の `git status --porcelain` と `git diff` は壊す前と同じ。`correction-review.md` に「落ちた」と書いた。`python3 scripts/plan_fix.py check openspec/changes/st54-dev-side-by-side` → `[OK]` rc=0。
- 処置: fixed 10.3 （check-guards.sh が `^[[:space:]]+name: nightly[[:space:]]*$` で照合し、self-test に名前を `nightly-open` に変えた偽を足した。照合を部分一致に戻すと self-test rc=1）

## security review（8184a7d..8490cfa）

見た経路:
- `GET /contract`（認証なし・違う token・偽のクッキー 2 種）→ `contract_version.rs:58` の `authorize` → `Caller::ApiToken` の枝でだけ見出し `Ashiato-Channel` を足す。どちらの系統でも 401 で、見出しは付かない（4）
- 画面のログイン: `web_session.rs:192` `cookie_token` は自分の系統の名前だけを読み、印は `core.web_session` の sha256 で照合（DB は系統ごとに別）。127.0.0.1 ではポートが違ってもクッキーは両方へ届くが、反対の名前の印は読まない（3）
- 系統の決め方: サーバ `lib.rs:305` / PC `main.rs:14` / 携帯 `buildLocalChannel()` は組み立て時の版番号（`env!` / `BuildConfig`）だけから決め、実行時の環境変数・設定から変える口は無い（3）
- PC の送る契機 `runtime.rs:832` → 頭の `GET /contract` の系統が違えば `return`（記録・生存信号の `flush` に届かない）。送る口は `sender.rs:242` の 1 か所で、`send_at` 以外から呼ばれない（4）
- 携帯 `Drainer.tick`（`Retention.kt:248`）→ `records` / `beats` / `drops` / `missing` の `flush(answer)` と原本の `OriginalUploader.run(answer)`。本番の組み立て（`LocationService.kt:983`）は必ず `ContractGuard` を渡し、`answer == null`（全部送る）は試験だけ（3）
- PC の開発版の除外の登録: `config.rs:133` が渡された置き場の `exclusions.json` を指し、`main.rs:108`・`runtime.rs:330`（ブラウザの履歴）の両方がそれを読む。開発版で除外が外れる経路は無い（3）
- 開発版の署名: `nightly.yml` の publish job（Environment `nightly`）→ `publish-nightly.sh` が鍵を `$RUNNER_TEMP` に戻し、`sign-apk.sh` が指紋 `ASHIATO_EXPECT_DEV_CERT_SHA256` と照合してから署名し、終わりに消す。正式版の鍵は `release` Environment にだけ置く（`docs/release.md:30`）ので nightly の job からは読めない（2）
- 系統の違いのログ: PC `runtime.rs:846`・携帯 `Telemetry.line(…)`・サーバ `migration_ledger.rs` の報告は `dev` / `official` の固定語と版の名前だけ（3）
- ログの検査 `tools/check-log-private.sh:55` に足した許可語 `server` / `self` → 下の R29（4）

安全の決め手: 「送る契機は頭の `GET /contract` で系統を照合し、違えば何も送らない（PC は `send_at` の 1 か所、携帯は `Drainer.tick` の 1 か所）」—— 確かさ 4。`scripts/quiet-run cw -- cargo test -p ashiato-collector-windows --lib st54_channel` → `24 passed; 0 failed`（`st54_channel_runtime_dev_holds_official` / `official_holds_dev` は POST の数 0 を見る）。サーバ側の入口は、使い捨ての worktree で `st54_tests.rs` に試験を足して走らせた（`ASHIATO_DB_PORT=55554 cargo test -p ashiato-server --lib st54_sec_probe`）: Dev・Official のどちらも、認証なし・違う token・偽のクッキー（`ashiato_session=x; ashiato_session_dev=y`）が `status=401 channel_header=None`、`1 passed`。作業ツリーは触っていない（worktree は消した）。

## R29. ログの検査の許可語に `server` と `self` を足したが、使っている所は無い。`server = %base_url` のように送り先の URL をそのまま出しても検査を通る
- 成果物: tools/check-log-private.sh
- 根拠: `check-log-private.sh:55` の `ALLOWED` に `"server", "self"`（ST54 で足した）。`grep -rnE '(info|warn|error|debug|trace)!\([^)]*\b(server|self)\s*=' crates/*/src` は 0 件で、PC の `runtime.rs:846` は `telemetry::line` の `error` に `format!("server={} self={}")` を渡す形なので、構造化された欄の名前としては使っていない。使い捨ての worktree で `crates/collector-windows/src/main.rs:108` に `tracing::warn!(server = %cfg.base_url, self = %cfg.device_id, "probe");` を差し込み `tools/check-log-private.sh` → `check-log-private: OK` rc=0。同じ行の欄の名前を `url` に変えると「許可していないフィールド `url`」rc=1。足した 2 語は、検査が止めていた送り先の URL・端末識別子を通す抜け道になっている（確かさ 4）
- kind: technical
- 提案: `"server", "self"` を `ALLOWED` から外す（ST54 のコードは使っていない）。系統を構造化された欄で出す日が来たら、`channel` のように値の決まった名前で足す
- 処置: fixed 7.1 （`ALLOWED` から `server` / `self` を外した。外しても `tools/check-log-private.sh` rc=0 —— 使っている所は無い）

## /code-review（8184a7d..8490cfa）

見た: Claude Code の組み込みの `/code-review`（9 件）。写しは scripts/code_review_record.py

## R30. The LINEAGE parser in build.rs only skips lines that start with '#'. It does not strip a trailing '# comment', so a line that the two check scripts (check-migrations.sh / check-nightly-migrations.sh) accept makes the server build panic.
- 成果物: crates/server/build.rs:106
- 根拠: Someone writes the form that the check-nightly-migrations self-test explicitly allows (final review R19): ` removed 202001010000_a # 使わなくなった`. split_whitespace returns 4 fields, the match falls to `_ => panic!("migrations/LINEAGE の行の形が違う")`, and `cargo build` of the server fails even though both CI checks are green.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 4.1 （R27 と同じ直し）

## R31. On a channel mismatch the Android collector writes a `channel_mismatch` log line from every Sender flush (records, beats, drops, missing), from ContractGate.observe and from OriginalUploader, on every send cycle. The Windows collector was changed to log once per state change (R12); Android was not.
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/Sender.kt:261
- 根拠: A dev APK is pointed at an official server. On every drain cycle Drainer calls records/beats/drops/missing.flush(answer), and each one logs `channel_mismatch`. The gate and the uploader log it again, so about 6 identical lines per cycle repeat for as long as the mismatch lasts, burying logcat and the logs ST58 reads.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 9.2 （Sender・OriginalUploader は `channel_mismatch` を出さず、ContractGate が状態に入ったときに 1 回だけ出す（メモリの印。読める同じ系統の答えで戻す）。試験 `mismatch log is written once while it lasts`、印を立てない変異で 1 failed）

## R32. When the app is both waiting for a contract update and seeing a channel mismatch, the screen does not show the mismatch line. The only writer of `store.notice` (ContractNotifier.update → contractNotice()) never sets channelMismatch=true, and MainActivity only falls back to the mismatch line when `waitStore.notice` is null.
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/MainActivity.kt:396
- 根拠: A dev phone has old rows rejected with 426 (contract waiting), then its URL is changed to an official server. `waitStore.notice` is non-null with channelMismatch=false, so the screen shows only 「サーバを更新してください」. The 「送り先が正式版のサーバです」 line never appears, and the user goes to update the server instead of fixing the destination.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 9.2 （画面の行を `screenNotice(store)` に切り出し、待ちの写しに系統の違いを足す。試験 `screen line shows mismatch while contract waiting`、足さない変異で 1 failed）

## R33. read_contract_channel ends with `.or(Some(Official))`, which turns the `_ => None` arm for unknown header values back into Official. Android likewise maps every value other than "dev" to Official. An unknown or malformed `Ashiato-Channel` value is silently treated as the official channel.
- 成果物: crates/collector-windows/src/sender.rs:91
- 根拠: A future server (or a proxy that mangles the header) answers `Ashiato-Channel: staging`. The official PC/phone collectors treat it as official and send all records to it, and the dev collectors report 'official server' (a misleading message). The intended 'unknown → do not decide' behaviour (`_ => None`) is dead code.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: design.md:190（D14（仮））が「系統は 2 つ: 開発版（`dev`）と、それ以外（`official`）」と決め、spec の FR-106 ★ 2026-11-03 も同じ。知らない値を Official に読むのは設計どおりで、Android の ContractGate.kt:59-61 とも一致する（final review R11 と同じ）

## R34. In ChannelNotice the `failing` flag is set to true on the first toast failure and never reset. After one failure, no later show/dismiss failure is logged again (ContractNotice clears it on success).
- 成果物: crates/collector-windows/src/channel_notice.rs:112
- 根拠: A toast fails once (for example while the notification service is still starting after boot) and logs `channel_notice_skipped`, then works again. If it later breaks for good (for example the registry key is removed), no log line ever appears, so 'the notice is not showing' cannot be diagnosed.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 7.1 （出す・消す・書くが通ったら `failing` を戻す。試験 st54_channel_failure_is_logged_again_after_recovery、戻さない変異で 1 failed）

## R35. The dev collector now moves its state dir to `<ASHIATO_STATE_DIR>/dev` and its device ID to `<id>-dev`. Nothing carries over the outbox, marker or browser-history state that existing nightly users have in the old location.
- 成果物: crates/collector-windows/src/config.rs:125
- 根拠: A user who has been running the nightly PC collector since ST52 updates. outbox.jsonl in the old state_dir is never sent, the missing marker makes the stopped period look like a first launch, and the server sees the same PC as a new device `pc-1-dev`. This is the same 'left behind' problem R27 in this file warns about.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: 開発版の PC 版はまだ一度も配られていない —— `git ls-remote --tags origin 'nightly*'` は 0 行、`gh release list` も空。旧い置き場に未送信を持つ開発版の利用者はいない。置き場の分け方は本人の決定 Q2（design.md:36 の D2）

## R36. readContractAnswer already returns ChannelMismatch whenever the channel differs, yet ContractGate.observe, Sender.flush and OriginalUploader each also handle 'Version whose channel != localChannel' separately. That branch is effectively dead code duplicating the ChannelMismatch branch (6 copies of the same log statement).
- 成果物: collector-android/app/src/main/kotlin/dev/ashiato/collector/ContractGate.kt:216
- 根拠: Any change to the mismatch handling (for example deduplicating the log, as in the log-flood finding) has to be applied in 6 places, and missing one leaves the branches behaving differently. The only gain is a path for the case where readContractAnswer and the gate are given different expectedChannel values.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 9.2 （`ContractAnswer.Version` から系統を外し、系統の比べを `readContractAnswer` の 1 か所にした。Sender・OriginalUploader・ContractGate の Version の系統の枝と `localChannel` を消した）

## R37. The per-channel default listen address (dev 18788) only applies when BIND is unset. `.env.example` sets `BIND=127.0.0.1:18787` explicitly, so a setup that loads .env runs the dev server on the official port too.
- 成果物: crates/server/src/lib.rs:4434
- 根拠: Following docs/release.md §9, a user copies .env.example for the dev server and starts it with the env loaded. bind_for gives the explicit 18787 priority, so the dev server either collides with the official server and fails to start, or, if the official server is stopped, takes the official port and the official collectors connect to it.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 11.1 （サーバは `.env` を自分では読まず、明示の BIND が既定より優先されるのは spec の Scenario「待ち受けと退避の置き場を明示で渡せば…」どおり。docs/release.md §9 の 4 に「正式版の `.env` を読み込んだ環境では `BIND=127.0.0.1:18788` を渡すか外す」を足した）

## R38. The release notes use a repository-relative link `../../docs/release.md`. In a GitHub Release body that path does not resolve to a file in the repository, so the link is a 404.
- 成果物: tools/release/nightly-notes.md:5
- 根拠: On the nightly release page, the link 「詳しい手順は docs/release.md」 resolves to /<owner>/<repo>/docs/release.md, which is not a /blob/main/ path, so it is a 404 and users cannot reach the side-by-side setup steps.（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 10.2 （リンクを `https://github.com/rai-256/ashiato/blob/main/docs/release.md#9-開発版を正式版と並べて使う` にした）
