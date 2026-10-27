# CH03 コードのレビュー（ブランチ全体）

## final review（1954021..3fa435a）

席: `superpowers:requesting-code-review` の `code-reviewer.md`（review package `.superpowers/sdd/tasks/review-19540213..3fa435ae.diff`）。
判定: **With fixes**（Critical なし・Important 2・Minor 多数）。reviewer は差分を複数パスで読み、残り約 30 ファイルは機械で見た
（D1 の型の破り 0 件・property 初期化の `tmp.newFolder()` 0 件・製品のファイルの hunk は全部 `#[cfg(test)]` の中、例外は R13 の 1 行）。
reviewer が手元で走らせたもの: `tools/check-tmp-hygiene.sh --self-test` rc=0 / `tools/tmp-leak-check.sh --self-test` rc=0 /
8.3 の `check-test-names.sh`（読むだけの形）を 3fa435a で rc=0（rust 1805→1806・kotlin 626→628、消えた印・名前なし）。

Task ごとの ⚠️ の振り分け: 「証跡が commit 前の作業木で取られた」（T1・T3・T6・T7）は、最終のコードでの 8.2・8.4 の PASS と reviewer の self-test 2 本が置き換える（取り直しが要るのは 8.3 だけ → R12）。
T1 の「`--rerun` で古い XML が混ざるか」は、Gradle の Test task が結果のフォルダを空にしてから書くので害なし。T4 の Windows で走ることは PR の CI の `collector-windows-runtime` が見る。

## R1. spec の SHALL・完了の判定・D6 の見出しが「worktree の中の空のフォルダ」のままで、実装（根は `/tmp`、落ちた回は `target/tmp-leak/` へ移す）と食い違う
- 成果物: openspec/changes/ch03-test-tmp-cleanup/specs/test-hygiene/spec.md:84 / docs/changes/CH03.md:75 / openspec/changes/ch03-test-tmp-cleanup/design.md:94
- 根拠: e80dd71c が D6（仮）の根を実測（collector-windows 283 本が /tmp で 5.7 秒・target/ で 63〜90 秒、`history_large_fetch_keeps_observing_foreground` が毎回落ちる）で `/tmp/ashiato-tmp-leak-*` に替え、design の本文だけ直した。archive すると正典に偽の SHALL が残る。本人の Q5 の答えは「空のフォルダへ向けて」で worktree を名指ししていない。Q6（/tmp を食わない）は落ちた回を target へ移すことで保つ
- kind: conflict
- 処置: （fix の後に書く）

## R2. 履歴の試験の多くが、取得のスレッド（HistoryWorker は drop で join しない）を動かしたまま終わり、`_tmp` が消えた後に置き場を作り直して漏れうる
- 成果物: crates/collector-windows/src/runtime.rs（`history_heartbeat_reports_not_queued_at_runtime`・`history_heartbeat_counts_unwritable_sent_version_as_not_queued`・`history_heartbeat_reports_unreadable_sent_version`・`history_heartbeat_counts_panicked_read_as_not_queued`・`fetch_and_send` を使う試験）
- 根拠: `tick_until_history_idle` を呼ぶのは runtime.rs:3001・3056・3162・3244 の 4 か所だけ。crates/collector-windows/src/history/fetch.rs:28 に join する Drop が無い。CI の rust job が漏れの検査になったので、間欠的な漏れが間欠的な CI の赤になる。T4 F1
- kind: technical
- 処置: （fix の後に書く）

## R3. `--no-run` が落ちると `/tmp` に空の根が残る
- 成果物: tools/tmp-leak-check.sh:59
- 根拠: `cargo test --no-run "$@" || { rc=$?; rm -f "$out"; return "$rc"; }` は根を消さない。この change が止める `/tmp` の残りそのもの。T1 F3 の残り
- kind: technical
- 処置: （fix の後に書く）

## R4. Android の本数に skipped が入り、全部が飛ばされても 0 本で落ちない
- 成果物: tools/tmp-leak-check.sh（`count_android_xml`）
- 根拠: JUnit XML の `tests` を数える。`Ch03TmpdirProbeTest` は環境変数が無いと飛ぶので、まさにその形。T1 F2
- kind: technical
- 処置: （fix の後に書く）

## R5. self-test が android の道（`count_android_xml`）を通らない
- 成果物: tools/tmp-leak-check.sh（`--self-test`）
- 根拠: self-test は `TMP_LEAK_CMD` の rust の形だけ。XML の数え方は 8.2 の本物の走りまで誰も確かめない。T1 F1・T8 F4
- kind: technical
- 処置: （fix の後に書く）

## R6. docs/testing.md の `tools/tmp-leak-check.sh <試験の命令>` が実際の使い方と違う
- 成果物: docs/testing.md:358
- 根拠: 実際は `rust [cargo test の引数…]` / `android [gradle の引数…]` / `--self-test`（tools/tmp-leak-check.sh:4-6）。書いてあるとおりに打つと使い方のエラーで落ちる。T7 F1
- kind: technical
- 処置: （fix の後に書く）

## R7. check-tmp-hygiene.sh が未知の引数を黙って本番の検査として走らせる
- 成果物: tools/check-tmp-hygiene.sh:509
- 根拠: `--selftest` の打ち間違いが rc=0 で通る。T7 F3
- kind: technical
- 処置: （fix の後に書く）

## R8. 作業木で消して stage していない追跡ファイルで check-tmp-hygiene.sh が traceback で rc=1
- 成果物: tools/check-tmp-hygiene.sh:491
- 根拠: `open()` が FileNotFoundError。pre-commit で違反と区別できない。T7 F4
- kind: technical
- 処置: （fix の後に書く）

## R9. 通ったときだけ消す `remove_file(path).unwrap()` が残る
- 成果物: crates/server/src/archive_tests.rs:624
- 根拠: `TempDir` の中なので要らず、assert より前にあり、Global Constraints の「通ったときだけ消す」形。T3 F1
- kind: technical
- 処置: （fix の後に書く）

## R10. 前の書き方の名残（要らない `create_dir_all`、DB のパスを作って捨てる `temp_db()`）
- 成果物: crates/collector-windows/src/heartbeat.rs:303 / crates/collector-windows/src/outage_notice.rs:1391 / crates/collector-windows/src/history/read.rs:1058・1242
- 根拠: T4 F2・F3。振る舞いは変わらない
- kind: technical
- 処置: （fix の後に書く）

## R11. 未送信の置き場が env の `dir` の下に移ったことが KDoc に無い
- 成果物: collector-android/app/src/test/kotlin/dev/ashiato/collector/AppUsageFixtures.kt:309（RollupTestEnv も同じ）
- 根拠: 後で `dir` を列挙する試験が子フォルダを拾う。T5 F1
- kind: technical
- 処置: （fix の後に書く）

## R12. 8.3 の証跡が最終 HEAD で取られていない
- 成果物: openspec/changes/ch03-test-tmp-cleanup/evidence.jsonl
- 根拠: 最後の 8.3 PASS は d798b111。その後に main の merge（751422b4）と試験の書き換え（65314fa2・f5e99cc3）。T8 F2
- kind: technical
- 処置: （fix の後に書く）

## R13. 「製品のコードは変えない」「assert を変えない」の例外 2 つを PR 本文で宣言する
- 成果物: crates/server/src/media.rs:568（`// tmp-ok:` のコメント 1 行）/ crates/server/src/day_tests.rs:1447（`st25_reasons_before_collection_started` の日付と assert_eq!、理由は agreed-test-changes.md:3）
- 根拠: どちらも振る舞い・期待値の意味は変わらない。T8 F1・T8 ⚠️
- kind: technical
- 処置: rejected: 欠陥ではない。`media.rs` は Q1（製品のコードも検査の対象）の逃げ道の印で振る舞いは同じ、`day_tests.rs` は agreed-test-changes.md に合意の記録がある。finish の PR 本文に 8.3 の例外として 2 つとも挙げる

## R14. 印が 1 行下の違反も覆う（`@Rule` の判定が「同じ行か 1 行上」だけ）
- 成果物: tools/check-tmp-hygiene.sh:463
- 根拠: T7 F2
- kind: technical
- 処置: rejected: spec の Scenario と D4（仮）が定めた範囲（同じ行か 1 行上）どおり。違反が 2 行続く形は木に無く、Kotlin の宣言を構文で読むのは D3（仮）の「置き場で範囲を決め、構文を読まない」に反する

## R15. ci.yml への掛かりの検査が ci.yml 全体を見るだけで job 単位でない
- 成果物: tools/tmp-leak-check.sh:118
- 根拠: T8 F3
- kind: technical
- 処置: rejected: 掛かりの検査は「掛け忘れ」を捕まえるためのもので、job の移し替えは ci.yml の diff として PR のレビューに出る。YAML を job 単位で読む道具（pyyaml）は CI の chain job に無く、足す費用に見合わない

## R16. `archive_scan_interval_change_takes_effect_without_restart` が負荷で揺れる
- 成果物: crates/server/src/archive_scan_interval_tests.rs
- 根拠: log 20261027-190711 で「走査が 3 回に届かない: 2」。CH03 の前からある。T8 F5
- kind: technical
- 処置: rejected: CH03 の不具合ではない（この試験の期待値・時間は CH03 で変えていない）。CI の rust job が台本を通すようになって CI の赤の元になりうるので、PR 本文の「未処置の申し送り」に別の change の起票として挙げる
