# CH03 コードのレビュー（ブランチ全体）

## final review（1954021..3fa435a）

席: `superpowers:requesting-code-review` の `code-reviewer.md`（review package `.superpowers/sdd/tasks/review-19540213..3fa435ae.diff`）。
判定: **With fixes**（Critical なし・Important 2・Minor 多数）。reviewer は差分を複数パスで読み、残り約 30 ファイルは機械で見た
（D1 の型の破り 0 件・property 初期化の `tmp.newFolder()` 0 件・製品のファイルの hunk は全部 `#[cfg(test)]` の中、例外は R13 の 1 行）。
reviewer が手元で走らせたもの: `tools/check-tmp-hygiene.sh --self-test` rc=0 / `tools/tmp-leak-check.sh --self-test` rc=0 /
8.3 の `check-test-names.sh`（読むだけの形）を 3fa435a で rc=0（rust 1805→1806・kotlin 626→628、消えた印・名前なし）。

fix は 1 回（0aa6ca29..ed9abba6、報告 `.superpowers/sdd/ch03-final-fix-report.md`）、scoped re-review 1 回（`re-review-prompt.md`、package `.superpowers/sdd/tasks/review-0aa6ca29..ed9abba6.diff`）:
R1〜R11 ADDRESSED、R12 は 8.3 の機械の証跡が無い分だけ残る（R17 に分けた。premise で finish が処置する）。新しい Critical/Important なし。

finish への申し送り（harness2 側）: deep.md の答えが「答え: …（本人の選択）」の形で `provisional.py` の `answered()` が読む「本人の答え」の形でないため、`--follows Q5` が通らなかった（D6 は --keeps/--cost で載せた）。

Task ごとの ⚠️ の振り分け: 「証跡が commit 前の作業木で取られた」（T1・T3・T6・T7）は、最終のコードでの 8.2・8.4 の PASS と reviewer の self-test 2 本が置き換える（取り直しが要るのは 8.3 だけ → R12）。
T1 の「`--rerun` で古い XML が混ざるか」は、Gradle の Test task が結果のフォルダを空にしてから書くので害なし。T4 の Windows で走ることは PR の CI の `collector-windows-runtime` が見る。

## R1. spec の SHALL・完了の判定・D6 の見出しが「worktree の中の空のフォルダ」のままで、実装（根は `/tmp`、落ちた回は `target/tmp-leak/` へ移す）と食い違う
- 成果物: openspec/changes/ch03-test-tmp-cleanup/specs/test-hygiene/spec.md:84 / docs/changes/CH03.md:75 / openspec/changes/ch03-test-tmp-cleanup/design.md:94
- 根拠: e80dd71c が D6（仮）の根を実測（collector-windows 283 本が /tmp で 5.7 秒・target/ で 63〜90 秒、`history_large_fetch_keeps_observing_foreground` が毎回落ちる）で `/tmp/ashiato-tmp-leak-*` に替え、design の本文だけ直した。archive すると正典に偽の SHALL が残る。本人の Q5 の答えは「空のフォルダへ向けて」で worktree を名指ししていない。Q6（/tmp を食わない）は落ちた回を target へ移すことで保つ
- kind: conflict
- 処置: fixed D6 仮 —— spec.md:84 の SHALL・docs/changes/CH03.md:75・design.md の D6 見出しと判定の段落・proposal.md:22 を「/tmp の下の専用の空のフォルダ（落ちた回・残りがあった回は target/tmp-leak/ へ移して残す）」に（d6f6ecad）。provisional.jsonl に D6（--keeps/--cost。`--follows Q5` は deep.md に「本人の答え」の行が無く check が通らない）。openspec validate --strict rc=0・check_scenarios.py rc=0。go.json:298 と docs/briefs/CH03-issue.md は第 1 回の時点の記録として残す

## R2. 履歴の試験の多くが、取得のスレッド（HistoryWorker は drop で join しない）を動かしたまま終わり、`_tmp` が消えた後に置き場を作り直して漏れうる
- 成果物: crates/collector-windows/src/runtime.rs（`history_heartbeat_reports_not_queued_at_runtime`・`history_heartbeat_counts_unwritable_sent_version_as_not_queued`・`history_heartbeat_reports_unreadable_sent_version`・`history_heartbeat_counts_panicked_read_as_not_queued`・`fetch_and_send` を使う試験）
- 根拠: `tick_until_history_idle` を呼ぶのは runtime.rs:3001・3056・3162・3244 の 4 か所だけ。crates/collector-windows/src/history/fetch.rs:28 に join する Drop が無い。CI の rust job が漏れの検査になったので、間欠的な漏れが間欠的な CI の赤になる。T4 F1
- kind: technical
- 処置: fixed 4.1 —— runtime.rs の history_runtime が試験だけの HistoryRuntime（Deref/DerefMut）を返し、drop で取得の別スレッドの終わりを最長 20 秒待つ（panic しない）。待つ判定は collect.rs の #[cfg(test)] threads_finished()。fetch.rs は不変（7683490e）。clippy --tests rc=0、`tools/tmp-leak-check.sh rust -p ashiato-collector-windows` を 5 回続けて rc=0（283 本・残り無し）。直す前の漏れの再現はしていない

## R3. `--no-run` が落ちると `/tmp` に空の根が残る
- 成果物: tools/tmp-leak-check.sh:59
- 根拠: `cargo test --no-run "$@" || { rc=$?; rm -f "$out"; return "$rc"; }` は根を消さない。この change が止める `/tmp` の残りそのもの。T1 F3 の残り
- kind: technical
- 処置: fixed 1.3 —— tools/tmp-leak-check.sh の --no-run 失敗の道で `rm -rf "$root"`（1f911304）。存在しないクレートで rc=101・根なしを確認

## R4. Android の本数に skipped が入り、全部が飛ばされても 0 本で落ちない
- 成果物: tools/tmp-leak-check.sh（`count_android_xml`）
- 根拠: JUnit XML の `tests` を数える。`Ch03TmpdirProbeTest` は環境変数が無いと飛ぶので、まさにその形。T1 F2
- kind: technical
- 処置: fixed 1.3 —— count_android_xml が tests - skipped を合計（1f911304）

## R5. self-test が android の道（`count_android_xml`）を通らない
- 成果物: tools/tmp-leak-check.sh（`--self-test`）
- 根拠: self-test は `TMP_LEAK_CMD` の rust の形だけ。XML の数え方は 8.2 の本物の走りまで誰も確かめない。T1 F1・T8 F4
- kind: technical
- 処置: fixed 1.3 —— XML の置き場を TMP_LEAK_XML_DIR で差し替え、self-test に android の 3 本（全部 skipped→rc=1、skipped を引いて 4 本→rc=0、残り→rc=1）。引き算を外すと 2 本 FAIL を確認（1f911304）

## R6. docs/testing.md の `tools/tmp-leak-check.sh <試験の命令>` が実際の使い方と違う
- 成果物: docs/testing.md:358
- 根拠: 実際は `rust [cargo test の引数…]` / `android [gradle の引数…]` / `--self-test`（tools/tmp-leak-check.sh:4-6）。書いてあるとおりに打つと使い方のエラーで落ちる。T7 F1
- kind: technical
- 処置: fixed 7.3 —— docs/testing.md の使い方を rust/android/--self-test・根は /tmp・落ちた回は target/tmp-leak/・skipped を引いた 0 本で落ちる、に（1f911304）

## R7. check-tmp-hygiene.sh が未知の引数を黙って本番の検査として走らせる
- 成果物: tools/check-tmp-hygiene.sh:509
- 根拠: `--selftest` の打ち間違いが rc=0 で通る。T7 F3
- kind: technical
- 処置: fixed 7.1 —— 引数なし=本体・--self-test=self-test・それ以外は usage rc=2。self-test に --selftest の case（1f911304）

## R8. 作業木で消して stage していない追跡ファイルで check-tmp-hygiene.sh が traceback で rc=1
- 成果物: tools/check-tmp-hygiene.sh:491
- 根拠: `open()` が FileNotFoundError。pre-commit で違反と区別できない。T7 F4
- kind: technical
- 処置: fixed 7.1 —— 作業木に無い追跡ファイルを飛ばす。self-test に一時の git の木で消す case、飛ばしを外すと traceback で FAIL を確認（1f911304）

## R9. 通ったときだけ消す `remove_file(path).unwrap()` が残る
- 成果物: crates/server/src/archive_tests.rs:624
- 根拠: `TempDir` の中なので要らず、assert より前にあり、Global Constraints の「通ったときだけ消す」形。T3 F1
- kind: technical
- 処置: fixed 3.1 —— 行を削除（2c88e8ed）。該当 1 本 rc=0

## R10. 前の書き方の名残（要らない `create_dir_all`、DB のパスを作って捨てる `temp_db()`）
- 成果物: crates/collector-windows/src/heartbeat.rs:303 / crates/collector-windows/src/outage_notice.rs:1391 / crates/collector-windows/src/history/read.rs:1058・1242
- 根拠: T4 F2・F3。振る舞いは変わらない
- kind: technical
- 処置: fixed 4.1 —— heartbeat.rs・outage_notice.rs の要らない create_dir_all を削除、read.rs に scratch() -> TempDir を置き temp_db() も使う（2c88e8ed）

## R11. 未送信の置き場が env の `dir` の下に移ったことが KDoc に無い
- 成果物: collector-android/app/src/test/kotlin/dev/ashiato/collector/AppUsageFixtures.kt:309（RollupTestEnv も同じ）
- 根拠: 後で `dir` を列挙する試験が子フォルダを拾う。T5 F1
- kind: technical
- 処置: fixed 5.1 —— AppUsageFixtures.kt の UsageTestEnv.outbox と RollupImportTest.kt の RollupTestEnv.outbox の KDoc（2c88e8ed・571854cd で detekt の空行）

## R12. 8.3 の証跡が最終 HEAD で取られていない
- 成果物: openspec/changes/ch03-test-tmp-cleanup/evidence.jsonl
- 根拠: 最後の 8.3 PASS は d798b111。その後に main の merge（751422b4）と試験の書き換え（65314fa2・f5e99cc3）。T8 F2
- kind: technical
- 処置: fixed 8.2 —— 571854cd で verify-run 8.1・8.2（rust 1788・android 629、残り無し）・8.4（detekt 含む）が PASS（ed9abba6 は証跡だけ）。8.3 は機械の証跡が取れないので R17 に分けた（571854cd で手で走らせて rc=0、rust 1805→1806・kotlin 626→628、消えた印・名前なし）

## R13. 「製品のコードは変えない」「assert を変えない」の例外 2 つを PR 本文で宣言する
- 成果物: crates/server/src/media.rs:568（`// tmp-ok:` のコメント 1 行）/ crates/server/src/day_tests.rs:1447（`st25_reasons_before_collection_started` の日付と assert_eq!、理由は agreed-test-changes.md:3）
- 根拠: どちらも振る舞い・期待値の意味は変わらない。T8 F1・T8 ⚠️
- kind: technical
- 処置: rejected: 欠陥ではない。`media.rs` は Q1（製品のコードも検査の対象）の逃げ道の印で振る舞いは同じ、`day_tests.rs` は agreed-test-changes.md に合意の記録がある。finish の PR 本文に 8.3 の例外として 2 つとも挙げる

## R14. 印が 1 行下の違反も覆う（`@Rule` の判定が「同じ行か 1 行上」だけ）
- 成果物: tools/check-tmp-hygiene.sh:463
- 根拠: T7 F2
- kind: technical
- 処置: rejected: spec の本文 openspec/changes/ch03-test-tmp-cleanup/specs/test-hygiene/spec.md:34「印と規則の注釈は、その行と同じ行か 1 行上にあるときだけ数える」どおりの実装（tools/check-tmp-hygiene.sh:44 の near()）。違反が 2 行続く形は木に無い（`tools/check-tmp-hygiene.sh` 本体 rc=0）。Kotlin の宣言を構文で読むのは D3（仮）の「置き場で範囲を決め、構文を読まない」に反する

## R15. ci.yml への掛かりの検査が ci.yml 全体を見るだけで job 単位でない
- 成果物: tools/tmp-leak-check.sh:118
- 根拠: T8 F3
- kind: technical
- 処置: rejected: 掛かりの検査（tools/tmp-leak-check.sh:136-143）は裸の `cargo test --workspace` / `gradlew :app:testDebugUnitTest` の段が無いことと、`tmp-leak-check.sh rust` / `android` の段が有ることを見て、掛け忘れを捕まえる。job の移し替えは .github/workflows/ci.yml の diff として PR のレビューに出る。YAML を job 単位で読むには pyyaml が要るが tools/ と .github/ のどこも使っていない（`grep -rn "import yaml" tools .github` 0 件）ので、足す費用に見合わない

## R16. `archive_scan_interval_change_takes_effect_without_restart` が負荷で揺れる
- 成果物: crates/server/src/archive_scan_interval_tests.rs
- 根拠: log 20261027-190711 で「走査が 3 回に届かない: 2」。CH03 の前からある。T8 F5
- kind: technical
- 処置: rejected: CH03 の不具合ではない。`git diff origin/main...HEAD -- crates/server/src/archive_scan_interval_tests.rs` は crates/server/src/archive_scan_interval_tests.rs:231-235 の一時フォルダの作り方と後始末の行だけで、時計・間隔・期待値は変えていない。CI の rust job が台本を通すようになって CI の赤の元になりうるので、PR 本文の「未処置の申し送り」に別の change の起票として挙げる

## R17. 8.3 の検証コマンドの入口が evidence.py の CMD_LIKE に当たらず、verify-run が走らせられない
- 成果物: openspec/changes/ch03-test-tmp-cleanup/tasks.md:149（8.3 の「検証:」の `openspec/changes/ch03-test-tmp-cleanup/check-test-names.sh`）
- 根拠: scripts/evidence.py:67 の CMD_LIKE の頭（`./` `tools/` `scripts/` …）に `openspec/` が無い。`scripts/verify-run 8.3` rc=2（本文に検証コマンドが無い）、`python3 scripts/evidence.py unverified openspec/changes/ch03-test-tmp-cleanup --task 8` → `["8.3"]`。最後の 8.3 の記録は d798b111 の executor manual。scoped re-review が R12 のこの分を NOT ADDRESSED とした。試す候補: `verify-run 8.3 --candidate './openspec/changes/ch03-test-tmp-cleanup/check-test-names.sh'`（`./` は CMD_LIKE に当たる）
- kind: premise
- 処置: escalated —— `scripts/verify-run 8.3 --candidate './openspec/changes/ch03-test-tmp-cleanup/check-test-names.sh'` は「本文の検証コマンドがいまのコードで FAIL した証跡が無い」で走らない。今の入口は evidence.py:67 の CMD_LIKE に当たらず FAIL の証跡も作れないので、plan_fix の機械の道（旧の FAIL → 新の PASS）が成立しない。原因は harness2 の evidence.py。deep.md Q9（第 2 回）。台本そのものは HEAD で手で rc=0（rust 1805→1806・kotlin 626→628、名前の集合でも欠け無し）

## code_verify（HEAD 4dec22bc。独立検証）

席: `code_verify`（手 1〜7）。上の R1〜R17 とは別に、実行して確かめたことだけを書く。作業ツリーは壊す前と後で
`git status --porcelain` と `git diff` が同じ（確かめ済み）。571854cd..HEAD の差分は `evidence.jsonl` と `review/code.md` だけ（`git diff --stat` で実測）。

### 申告: 25/25（`- [x]` 25 件。1.x 5・2.x 3・3.x 3・4.x 2・5.x 2・6.x 2・7.x 4・8.x 4）

| 項目 | 検証コマンド（要点） | 実測 | 一致 |
|---|---|---|---|
| 1.1 | `cargo tree … tempfile v3`（2 crate） | rc=0 | 一致 |
| 1.2 | `grep ASHIATO_TEST_TMPDIR / java.io.tmpdir build.gradle.kts` | rc=0 | 一致 |
| 1.3 | `tools/tmp-leak-check.sh --self-test` | rc=0 | 一致（本文の根の置き場は古い → R22） |
| 1.4 | Ch03TmpdirProbeTest を `ASHIATO_TEST_TMPDIR` 付きで | `LK android` の走りの XML で `tests="1" skipped="0"` | 一致（等価の走り） |
| 1.5 | `tmp-leak-check.sh rust -p ashiato-collector-windows --lib` + 名前の判定 | rc=0 | 一致（rc=1 の道は壊れている → R22） |
| 2.1 | `NL export_ 99` / `NL migrate_backup 4`（`CT` は 8.2 の全体の走りに含む） | 167 本 / 6 本、rc=0 | 一致 |
| 2.2 | `NL ch03_tmp_dir 1` | 1 本、rc=0。`tmp_dir` に `.disable_cleanup(true)` を足すと FAILED（export_tests.rs:41） | 一致 |
| 2.3 | `LK rust -p ashiato-server export_` | rc=0、167 本、残り無し | 一致 |
| 3.1 | `NL archive_tests 73` | 76 本、rc=0 | 一致 |
| 3.2 | `NL archive_flow_tests 105` | 120 本、rc=0 | 一致 |
| 3.3 | `LK rust -p ashiato-server archive` | rc=0、240 本、残り無し | 一致 |
| 4.1 | `--list` ≥283 / `LK rust -p ashiato-collector-windows --lib` | 283 本 / rc=0、283 本、残り無し | 一致 |
| 4.2 | `cargo clippy -p ashiato-collector-windows --all-targets --target x86_64-pc-windows-gnu -- -D warnings` | rc=0 | 一致 |
| 5.1 | grep が 0 件 / 単体試験の全部 | rc=0 / `LK android` で 629 本が通った | 一致 |
| 5.2 | Ch03TmpRuleTest の XML がある | `TEST-dev.ashiato.collector.Ch03TmpRuleTest.xml` がある | 一致 |
| 6.1 | grep が 0 件 | rc=0 | 一致 |
| 6.2 | `LK android` | rc=0、629 本、残り無し | 一致 |
| 7.1〜7.3 | `tools/check-tmp-hygiene.sh --self-test` / 本体 | rc=0 / rc=0 | 一致（守りの外側 → R18・R19） |
| 7.4 | docs の grep | rc=0 | 一致 |
| 8.1 | `tools/tmp-leak-check.sh --self-test` | rc=0 | 一致 |
| 8.2 | `LK rust` / `LK android` | **rust は 2 回走らせて 2 回とも rc=101**（`archive_scan_interval_change_takes_effect_without_restart` だけが落ちた。残した根は空）/ android は rc=0 | **不一致**（→ R21） |
| 8.3 | `check-test-names.sh` | rc=0（rust 1805→1806・kotlin 626→628）。verify-run では走らない（R17） | 一致（守りの穴 → R20） |
| 8.4 | check_scenarios / fmt+clippy / detekt / log-private+shared-appends | rc=0 / rc=0 / rc=0 / rc=0 | 一致 |

手 1: 該当なし（確かめた範囲: CH03 には、期待値を直に書いて固定したテスト（hash など）が無い。tasks の本数の下限（`NL` の 99/4/1/73/105・collector-windows の 283）は `-- --list` で数え直し、どれも下限以上だった）。

手 2: R18・R19・R22。守っていることを確かめた壊し方: ci.yml の chain job から本体の行を消す → self-test rc=1。pre-commit の 23 行目を `|| true` にする → rc=1。`tmp_dir` を `disable_cleanup(true)` にする → panic の試験が FAILED。

手 3: `python3 scripts/check_scenarios.py . ch03-test-tmp-cleanup` は rc=0。18 本の印がどこにあるかを 1 本ずつ見た（Rust 1・Kotlin 1・tools の echo 16）。印の先が主張の階層で観測していないものは R19（commit 前に止める）だけ。「CI の試験の段は…台本を通してだけ走る」を ci.yml の全体で見ていて job ごとに見ていないことは、R15（rejected）と同じなので書かない。

手 4: 該当なし（確かめた範囲: Q1〜Q8 から取った値を、複製の上で 1 つずつ書き換えて self-test を走らせた。印の窓を 1 行上から 2 行上へ・理由の無い `tmp-ok:` を許す・Kotlin の範囲を `app/src/` 全体へ広げる・Kotlin を見ない・0 本で落とさない・skipped を引かない。6 つとも self-test が rc=1 で落ちた。Q3・Q6・Q7・Q8 は「しない」の決定で、それを破る台本は木に無い（`/tmp` を名前の頭で消す行は無い））。

手 6: 該当なし（確かめた範囲: 本人が外した範囲は Q7（殺された試験）と Q8（失敗した試験の中身）。殺された回の跡として、`/tmp/ashiato-tmp-leak-rust-20261027-190524-791032`（空。pid はもう居ない）と、`target/tmp-leak/out.*` の 14 個（10-24〜10-25 のもの）が残っていた。どちらも Q7 の範囲。落ちた回の根は `target/tmp-leak/` へ移り、`/tmp` には残らないことを実測した（R21 の 2 回とも））。

手 7: 8.3 の訂正 1 件（`correction-review.md`）。分岐点からある試験を消すと `check-test-names.sh` は rc=1 で落ちた。追加で試した壊し方で穴を見つけた → R20。

## R18. check-tmp-hygiene は「後始末を外す口」の `disable_cleanup(true)`・`keep(true)`・`TemporaryFolder.builder()` を素通りさせる
- 成果物: tools/check-tmp-hygiene.sh:32-36（`RUST` / `KOTLIN` / `FOLDER`）/ openspec/changes/ch03-test-tmp-cleanup/specs/test-hygiene/spec.md:31-32
- 根拠: 検査の python を `--root <一時の木>` で 1 行ずつ走らせた。`tempfile::Builder::new().disable_cleanup(true).tempdir()` は rc=0、`d.disable_cleanup(true)` は rc=0、`tempfile::Builder::new().keep(true).tempdir()` は rc=0、`TemporaryFolder.builder().build()`（規則なし）は rc=0、`std::mem::forget(d)` は rc=0。比べると `std::env::temp_dir()` は rc=1。tempfile 3.27.0（Cargo.lock）には `Builder::disable_cleanup`（lib.rs:490）と `Builder::keep(bool)`（lib.rs:497）がどちらもある。本人の Q2 の答えは「後始末を外す口も落とす」で、spec の列挙（`.keep()`・`into_path()`・`persist(`）と deep-questions.json:42 の兄弟の一覧からこれらが漏れた。もう 1 つ、Global Constraints が禁じる一時の値の形（`TempDir::new().unwrap().path().to_path_buf()`）も rc=0 で通る（木には 0 件。`tmp-leak-check` は、その path に書いたときだけ捕まえる）
- kind: technical
- 提案: `RUST` に `disable_cleanup\(true\)`・`\.keep\(true\)`、`FOLDER` に `TemporaryFolder\.builder\(` を足す（規則の判定も同じにする）。self-test の「書き方を変えても」の段と spec の列挙にも足す。`mem::forget` は製品のコードに正しい使い方が 2 つある（lib.rs:3705・3720）ので外したままでよい
- 処置: fixed 7.1 —— RUST に `disable_cleanup(true)`・`.keep(true)`、FOLDER に `TemporaryFolder.builder(` を足し、self-test の「書き方を変えても」に 3 つと builder の 1 つ、規則つき builder の通る形を足した（49f4da1f）。`mem::forget` は製品のコードに正しい使い方があるので外したまま

## R19. 「検査は commit 前に走る」の self-test は pre-commit.sh の文字列を見るだけで、`[ -x … ]` の guard が偽でも緑
- 成果物: tools/pre-commit.sh:22-23 / tools/check-tmp-hygiene.sh（self-test の pre-commit の段）
- 根拠: 複製の木（tools と ci.yml を写した一時の git の木）で、pre-commit.sh:22 を `if [ -x tools/check-tmp-hygiene.NOPE ]` に書き換えると、`--self-test` は rc=0 のまま。Scenario「検査は CI の chain job と commit 前に走る」の THEN「落ちれば commit を止める」は振る舞いの主張で、テストは 23 行目の文字列が在ることしか見ていない（主張の階層とテストの階層が違う）。exec bit が落ちた checkout では、pre-commit が何も言わずに検査を飛ばす。比べると、23 行目を `|| true` にした壊し方は self-test が FAIL で捕まえた
- kind: technical
- 提案: self-test で pre-commit.sh を実際に走らせる（違反を 1 つ置いた一時の木で rc≠0 を見る）か、guard を外して `tools/check-tmp-hygiene.sh` が無ければ落とす
- 処置: fixed 7.1 —— self-test が一時の git の木で tools/pre-commit.sh を実際に走らせ、違反を stage すると rc=1 と違反の場所を見る（49f4da1f）

## R20. 8.3 の本数比較が総数なので、CH03 が足した試験の数（rust 1・kotlin 2）までは、既存の試験の `#[test]` を外しても通る
- 成果物: openspec/changes/ch03-test-tmp-cleanup/check-test-names.sh:12-16
- 根拠: 使い捨ての worktree（detached。ref は動かしていない）で、分岐点からある `broken_marker_is_an_error` の `#[test]` の 1 行だけを消して commit し（marker.rs:146。関数は残す）、`check-test-names.sh` を走らせた → `rust 1805 -> 1805`、rc=0。試験は 1 本走らなくなったのに緑。関数ごと消すと（手 7）、名前の集合の比較で rc=1。design D7 の「本数が減っていない」を総数で見ているため、増えた分だけ既存の試験を黙って無効にできる
- kind: technical
- 提案: `#[test]`・`#[tokio::test]`・`@Test` の直後の `fn` / `fun` の名前の集合を、merge-base と HEAD で比べる（`git grep -A1` か `-A2`）。そのうえで、merge-base の集合が HEAD に含まれることを見る
- 処置: fixed 8.3 —— check-test-names.sh が注釈の直後の `fn` / `fun` の名前の集合を分岐点と HEAD で比べ、分岐点の試験が HEAD で試験でなくなれば名前を挙げて rc=1（49f4da1f）。使い捨ての worktree で `broken_marker_is_an_error` の `#[test]` だけを消して commit → `rust 1805 -> 1805` のうえで「試験でなくなった関数: fn broken_marker_is_an_error」rc=1。worktree は消した

## R21. 8.2 の `LK rust` が HEAD で 2 回とも rc=101（R16 の揺れる試験。単独なら通り、残りも無い）
- 成果物: crates/server/src/archive_scan_interval_tests.rs:156-165（`wait_for_scans` が 25ms × 200 回）/ .github/workflows/ci.yml:75
- 根拠: `tools/tmp-leak-check.sh rust` を 2 回走らせた（ログ `.superpowers/sdd/logs/20261027-195741-leak.log`・`20261027-200321-leak.log`。load average 3.9）。2 回とも `archive_scan_interval_change_takes_effect_without_restart` が「走査が 6 回に届かない: 5」「走査が 5 回に届かない: 4」で落ちた（server lib は 1479 本が通り、落ちたのは 1 本）。target/tmp-leak/ へ移した根は空だった。`tmp-leak-check.sh rust -p ashiato-server --lib archive_scan_interval` は 2 回とも rc=0（5 本・残り無し）。CH03 の差分は一時フォルダの 2 か所だけ。cargo は落ちた試験の binary で止まるので、7 つの実行ファイルのうち 3 つは走っていない。8.2 の `[x]` は、手元では負荷によって再現できない。この試験を持っている Story / change は無い（`grep -rn archive_scan_interval docs/stories docs/changes` が 0 件。隙間に落ちている）
- kind: daily
- 提案: R16 の申し送りを、実際の change の起票にする（`hx change`）。そこにこの 2 回の実測を書く。CH03 の merge は止めない（CH03 が無くても、裸の `cargo test --workspace` で同じように落ちる）
- 処置: fixed D9 仮 —— design D9（仮）: この揺れる試験は CH03 で直さない。反転条件は分岐点の木で同じ負荷の全体の走りが 3 回とも通ること。別の change の起票は PR 本文の申し送りに書く（このセッションは `hx change` を起こさない）

## R22. tasks 1.3・1.5 の本文が D6 の根の変更（`/tmp/ashiato-tmp-leak-*`）に追いついておらず、1.5 の rc=1 の道が古いフォルダを読む
- 成果物: openspec/changes/ch03-test-tmp-cleanup/tasks.md:44（1.3「根は `target/tmp-leak/<rust|android>-<日時>-<pid>`」）/ tasks.md の 1.5 の検証（`ls -dt target/tmp-leak/rust-*`）
- 根拠: 実装（tools/tmp-leak-check.sh:59）の根は `/tmp/ashiato-tmp-leak-<mode>-…` で、落ちた回は `target/tmp-leak/ashiato-tmp-leak-<mode>-…` に移る。`TMP_LEAK_CMD` で `ashiato-left` を 1 つ残し、1.5 の判定の式を走らせた → 台本は rc=1、判定の式は今の根でなく古い `target/tmp-leak/rust-20261027-175933-120534` を拾って rc=1。今の HEAD は rc=0 の道を通るので緑だが、「どちらでも、`ashiato` で始まらない残りがあれば落ちる」の半分は働かない。R1 は spec・design・CH03.md・proposal を直したが、tasks.md を直していない
- kind: technical
- 提案: 1.3 の根の記述と 1.5 の glob を `target/tmp-leak/ashiato-tmp-leak-rust-*` に直す（plan_fix の訂正として）
- 処置: fixed 1.5 —— 落ちた回の根の移し先を `target/tmp-leak/<rust|android>-<日時>-<乱数>`（頭の `ashiato-tmp-leak-` を外す）にして、1.3 の本文の置き場と 1.5 の glob `target/tmp-leak/rust-*` が実装と合う形にした（49f4da1f）。tasks.md は凍結なので直さない（根そのものが /tmp にあることは spec・design D6・docs/testing.md が書く）。`TMP_LEAK_CMD` で残りを置いた回で、移した先が `ls -dt target/tmp-leak/rust-* | head -1` に当たることを self-test が見る

## security review（19540213..4dec22bc）

見た経路:
- 差分の `crates/**/*.rs` の各 hunk の位置 → `mod tests` より前にあるのは `history/collect.rs:525` の `#[cfg(test)] impl`（`threads_finished`）と `media.rs:568` の `// tmp-ok:` のコメント 1 行だけ。API・取り込み・待ち受け・ログの経路は変わっていない（4: hunk の行番号を `mod tests` の行と比べる台本を走らせた）
- CI の `cargo test --workspace` / `gradlew :app:testDebugUnitTest` → `tools/tmp-leak-check.sh rust|android` → `TMPDIR` / `ASHIATO_TEST_TMPDIR` を `/tmp/ashiato-tmp-leak-<mode>-<秒>-<pid>` に向ける → 通れば `rm -rf`、落ちれば `target/tmp-leak/` へ `mv`（4: 下の R23 の台本で走らせた）
- `build.gradle.kts:67` `ASHIATO_TEST_TMPDIR` → 試験の JVM の `java.io.tmpdir` だけ。環境変数が無ければ何も変えない。本体の APK の組み立てには入らない（`android.testOptions.unitTests.all` の中）（2）
- `TMP_LEAK_CMD` → `bash -c`（tmp-leak-check.sh:65）。環境変数を握れる者はもう命令を走らせられるので新しい入口ではない。CI は設定しない（2）
- `tools/check-tmp-hygiene.sh` → `git ls-files` の名前を python の `open()` に渡すだけ。シェルにも正規表現にもファイルの中身を式として渡さない（2）
- 依存: `crates/collector-windows` の `[dev-dependencies] tempfile = "3"`。Cargo.lock は既にあった `tempfile 3.27.0`（server が使う同じもの）に解決し、新しい crate は入らない。試験の組み立てだけ（2）

安全の決め手: この差分が製品のコードに入れたのは `#[cfg(test)]` の関数 1 つとコメント 1 行だけで、外から届く入口（API・取り込み・画面・待ち受け）は 1 つも増えも変わりもしていない —— 確かさ 4。各ファイルの差分の hunk の新しい側の行番号を `mod tests` の行と比べる台本を走らせ、前に出たのは上の 2 か所だけだった。新しく届くようになった経路は、開発の道具（tmp-leak-check.sh）が共有の `/tmp` に名前の決まった根を作るところだけ（→ R23）

## R23. tmp-leak-check.sh の根は `/tmp` の決まった名前を `mkdir -p` で作るので、同じ機械の別の利用者が先に置いた symlink・フォルダへ試験の一時ファイルが書かれる
- 成果物: tools/tmp-leak-check.sh:59-61（`root="$base/ashiato-tmp-leak-$mode-$stamp-$$"` / `mkdir -p "$root"` / 空かどうかだけを見る）
- 根拠: 根の名前は日時（秒）と pid だけで決まり、`mkdir -p` は既にあるフォルダ・フォルダを指す symlink をそのまま受け入れる。持ち主も symlink かどうかも見ない。台本で再現した: サブシェルの `$BASHPID` と次の 3 秒分の日時から `/tmp/ashiato-tmp-leak-rust-<日時>-<pid>` の symlink を、別に作った空のフォルダ `/tmp/sec-evil-9ni1` へ先に張り、`TMP_LEAK_CMD='d=$(mktemp -d); echo secret-fixture > "$d/f"; …' exec tools/tmp-leak-check.sh rust` を走らせた → 台本は `根 /tmp/ashiato-tmp-leak-rust-20261027-201353-1688234` を名乗って進み、試験が書いた `tmp.ct6ATP6Rjb/f` は `/tmp/sec-evil-9ni1/` の下にできた。落ちた回の `mv` は symlink だけを `target/tmp-leak/` へ移し、中身は先に置いた側のフォルダに残った（確かさ 4）。差分の前は、試験の一時フォルダは `tempfile` / uuid の名前で `/tmp`（sticky）に直に作られていたので、この先回りは効かなかった。根の持ち主が他人なら、tempfile が中に作る 0700 のフォルダも名前の付け替えが効く。いま試験が書くのは fixture（`example.test` など）だけで、CI の runner も手元の WSL も利用者は 1 人なので、私的データの漏れには届かない（`loss: exported` は付けない）
- kind: technical
- 提案: 根を `root=$(mktemp -d "$base/ashiato-tmp-leak-$mode-$stamp-XXXXXX")` で作る（O_EXCL で新しく作り、0700。名前の頭は残るので docs/testing.md:60 の記述と `keep_root` はそのまま使える）。self-test に「同じ名前が先にあっても別の根を作る」を 1 本足す

範囲外: 手で走らせた検証が `target/tmp-leak/out.*`（試験の出力の全文）を消さずに残す件は殺された回だけで、既に「手 6」と Q7 の範囲。
- 処置: fixed 1.3 —— 根を `mktemp -d "$base/ashiato-tmp-leak-$mode-$stamp-XXXXXX"` で作る（新しく作り 0700。49f4da1f）。self-test に「根は symlink でなく 0700」の 1 本。design D6・docs/testing.md も <pid> → <乱数>

## /code-review（1954021..4dec22b）

見た: Claude Code の組み込みの `/code-review`（10 件）。写しは scripts/code_review_record.py

## R24. /tmp に作った根（と target/tmp-leak/out.*）を消す trap が無いので、台本が中断されると根が /tmp に残る。
- 成果物: tools/tmp-leak-check.sh:57
- 根拠: CI のタイムアウト、Ctrl-C、quiet-run の打ち切りで `tools/tmp-leak-check.sh rust` が SIGINT/SIGTERM で止まる → /tmp/ashiato-tmp-leak-rust-<日時>-<pid> と試験が書いた中身が消されずに残る。CH03 が防ごうとしている /tmp の溜まりを台本自身が作る。`trap 'keep_root "$root"; rm -f "$out"' INT TERM` などが要る。（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 1.3 —— run() に INT / TERM の trap（out を消し、根を target/tmp-leak/ へ移して rc=130）。試験が終われば trap を外す（49f4da1f）。`TMP_LEAK_CMD='sleep 30'` を TERM で止めて rc=130・根が target 側へ移り /tmp に残らないことを手で確かめた

## R25. TemporaryFolder( の規則の有無を『同じ行か 1 行上に何かの @Rule があるか』で見ているので、別の規則の行に続く規則なしの TemporaryFolder() が通る。
- 成果物: tools/check-tmp-hygiene.sh:54
- 根拠: `@get:Rule val clock = FakeClockRule()` の次の行に `private val tmp = TemporaryFolder()` を書く → near(lines, i, RULE) が 1 行上の別の @get:Rule に当たって真になり、検査は通る。実際には規則が付かないので newFolder() は before() 前で落ちるか、手で create() すれば消されずに残る。（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 7.1 —— 1 行上の注釈は、その行が注釈だけ（`^\s*@(get:)?Rule\s*$`）のときだけ数える（ruled()）。self-test に `@get:Rule val clock = FakeClockRule()` の次の行の `TemporaryFolder()` が落ちる case（49f4da1f）

## R26. Rust の型が `.keep()`（引数なし）だけを拾うので、tempfile の `Builder::keep(true)`・`disable_cleanup(true)`・`std::mem::forget(tmp)` で消えない一時フォルダを作っても素通りする。
- 成果物: tools/check-tmp-hygiene.sh:32
- 根拠: `tempfile::Builder::new().prefix("x").keep(true).tempdir()` を試験に書く → `\.keep\(\)` に一致せず rc=0。TempDir は drop しても消えず、/tmp に残る（検査の『消えることが言語の仕組みで決まる書き方だけを許す』の抜け道）。（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 7.1 —— R18 と同じ直し（`disable_cleanup(true)`・`.keep(true)` を落とす）。`std::mem::forget` は製品のコード（lib.rs の 2 か所）に正しい使い方があるので型に入れない

## R27. 別スレッドが終わるまで待つ HistoryRuntime の Drop は history_runtime() 経由の試験にしか効かない。Runtime::new(...).with_history() を直に組む試験は、最後の tick_until_history_idle より前で落ちると待たずに TempDir を消す。
- 成果物: crates/collector-windows/src/runtime.rs:3106
- 根拠: history_large_fetch_keeps_observing_foreground（と history_slow_read_does_not_disturb_window・送り先が止まる試験）で途中の assert が落ちる → 巻き戻しで _tmp が消える。そのあと、まだ走っている読み・帳面の書き込みのスレッドが browser-history/ を create_dir_all で作り直し、一時フォルダが漏れる（R2 で塞いだはずの漏れが、落ちた回に限って残る）。（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 4.1 —— `Runtime::new(...).with_history()` を直に組む 3 本（history_large_fetch_keeps_observing_foreground・history_slow_read_does_not_disturb_window・送り先が止まる試験）も HistoryRuntime で包む（49f4da1f）

## R28. `release`（gate の Sender）が rt より先に宣言されているので、試験が落ちると HistoryRuntime::drop が放されない gate を 20 秒待ち、結局漏らす。
- 成果物: crates/collector-windows/src/runtime.rs:3266
- 根拠: history_heartbeat_at_startup_reports_previous_not_queued の assert_eq!(beats[0]["capturable"], false) が落ちる → 宣言の逆順で rt が release より先に drop される。読みのスレッドは gate.recv() で止まったままなので、threads_finished() は 4000×5ms 偽のまま。20 秒止まった後に『漏れうる』と出して抜け、そのあと release が drop されてスレッドが置き場へ書き、一時フォルダが残る。（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 4.1 —— gate の `release` を rt より後に束縛し直す（宣言の逆順で gate が先に放される。49f4da1f）。`history_heartbeat_at_startup_reports_previous_not_queued` の assert をわざと壊して走らせた → 0.02 秒で FAILED、残した根は空（20 秒待たず、漏れない）。壊した行は戻した

## R29. 外れの通知の問い合わせスレッド（OutagePoller）を試験の最後に待たないので、NoticeStore::save → atomic_write の create_dir_all が、消えた置き場を作り直しうる。
- 成果物: crates/collector-windows/src/runtime.rs:1246
- 根拠: outage_notice_poll_runs_once_per_clock_trigger_with_the_device_id などで、最後の契機に起こした poll_once のスレッドがまだ走っているうちに試験が終わる → _tmp が消えた後に save が `<tmp>/state/outage-notice.json` を作る（fsutil::atomic_write が親を create_dir_all する）→ ashiato-rt-* が /tmp に残り、tmp-leak-check が時々落ちる。テスト用の OutagePoller::wait() はあるのに呼んでいない。（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 4.1 —— outage_notice.rs に `#[cfg(test)] impl Drop for OutagePoller`（捨てるときに走っている問い合わせを待つ。製品の組み立てには入らない。49f4da1f）。rt は _tmp より後に束縛されるので、先に落ちて問い合わせを待ってから一時フォルダが消える。`tools/tmp-leak-check.sh rust -p ashiato-collector-windows --lib` rc=0（283 本・残り無し）

## R30. pre-commit でも git ls-files の一覧を作業木の中身で読むので、commit される staged の中身でなく作業木の中身を見ている。
- 成果物: tools/check-tmp-hygiene.sh:63
- 根拠: `std::env::temp_dir()` を含む版を git add した後、作業木だけで tempfile に直して stage し忘れる → 検査は作業木を読んで通り、違反のある中身が commit される。逆に、作業木にだけある書きかけの違反が、関係ない commit を止める。（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 7.1 —— check-tmp-hygiene.sh に `--staged`（`git cat-file --batch` で index の中身を読む）を足し、pre-commit.sh はそれを呼ぶ。CI の chain job は作業木（= commit した木）のまま。self-test に「作業木だけ直して stage し忘れた違反は止める」「stage していない作業木だけの違反は止めない」の 2 つ（49f4da1f）

## R31. スレッドを待つ処理を、試験専用の Deref の包み（HistoryRuntime）と tick_until_history_idle の 2 箇所で継ぎ足している。根は HistoryCollector / HistoryWorker が drop で join しないこと。
- 成果物: crates/collector-windows/src/runtime.rs:2235
- 根拠: 同じ競り合いが、包みを通らない試験（上の 3106 行）にも、tests/runtime_windows.rs の fetch_history_until（collector を待たずに TempDir を落とす）にも残る。HistoryCollector に drop で join する Drop（か、全経路が使う wait()）を持たせれば 1 箇所で済み、包み・待ちの手伝い・20 秒の輪をまとめて消せる。（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: 根を直すには HistoryCollector / HistoryWorker（crates/collector-windows/src/history/collect.rs:231・crates/collector-windows/src/history/fetch.rs:28 の製品のコード）に join する Drop を持たせることになり、製品の終わり方（アプリを閉じるときに読みを待つ）が変わる。CH03 は試験の一時フォルダの change で、製品の振る舞いを変えない（design D7・tasks の Global Constraints）。試験の側の待ちは HistoryRuntime（crates/collector-windows/src/runtime.rs:2220）1 か所に寄せ、直に組む試験も R27 で包んだ。tests/runtime_windows.rs の fetch_history_until は Windows の実行時試験で、tmp-leak-check の対象（--lib・Linux の CI）の外

## R32. archive_tests::tmp_dir が export_tests::tmp_dir（pub(crate)。prefix を作って TempDir を返す）とほぼ同じ実装の重複になっている。
- 成果物: crates/server/src/archive_tests.rs:10
- 根拠: 一時フォルダの作り方（prefix の規則・置き場）を変えるときに 2 箇所を揃えて直す必要があり、片方だけ直ると食い違う。crate::export_tests::tmp_dir を呼ぶか、testdb.rs の隣の共通の試験用モジュールへ 1 つにまとめられる。（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: rejected: 2 つは名前の頭が違う（export_tests.rs:20 は `ashiato-export-<name>-`、archive_tests.rs:12 は `ashiato-<name>-`）。export_tests::tmp_dir を呼ぶと書庫の試験の一時フォルダが `ashiato-export-archive-…` と名乗り、/tmp に残ったときに出所を読み違える。共通にするなら頭を引数に取る 5 行の手伝いになり、今の 2 つの 5 行と変わらない。名前の頭は Global Constraints の「名前の頭は残す」の対象

## R33. 落ちた回の根を毎回 target/tmp-leak/ へ移して残すだけで、上限も片付けも無いので、tmpfs の溜まりがディスク（target/）へ移るだけになる。
- 成果物: tools/tmp-leak-check.sh:49
- 根拠: 漏らす試験を直すまでの間に tmp-leak-check を何回も回す（verify の再試行など）→ 毎回 /tmp/ashiato-tmp-leak-* がファイルシステムをまたいで丸ごと複製され、target/tmp-leak/ が際限なく育つ（大きな一時ファイルを残す試験なら毎回数 GB）。古い根を消すか、件数に上限が要る。（確かさ 2）
- kind: technical
- 提案: （/code-review は提案を持たない。finish が直し方を決める）
- 処置: fixed 1.3 —— keep_root が移した後、種類ごとに新しい方から 5 個（KEEP_MAX）だけ残す（49f4da1f）。self-test で古い 6 個を置いて落ちた回を 1 つ足し、5 個になり一番古いものが消えることを見る。手元でも target/tmp-leak/rust-* が 5 個に収まった

## finish の fix（4dec22bc..49f4da1f）と scoped re-review

scoped re-review 1 回（独立の agent。package `.superpowers/sdd/tasks/review-4dec22bc..49f4da1f.diff`）: R18・R19・R21〜R30・R33 ADDRESSED、R31・R32 の rejected の理由は成り立つ、R20 PARTIAL。
fix が持ち込んだ Important 1 件（check-test-names.sh の本数の比べが、後ろに行が続いたため `set -e` の下で効かなくなった）は、`|| { …; exit 1; }` を足して直した（退行の戻しなので 2 回目の fix に数えない）。
park（Ruling。PR 本文へ）: R20 の残り —— 名前の集合は名前だけを比べるので、同じ名前の試験が別のファイルにある場合（HEAD で rust 6・kotlin 6）は片方の注釈を外しても通る。本数の比べが残りを覆う。Minor: trap が TERM でも 130 を返す・`--no-run` の失敗の道は trap を外さず return（すぐ終わるので害は無い）・1 行の `#[test] fn x()` を拾わない（木に無い）・R29 は `poll_outages` の `#[cfg(test)] wait()` と重ねがけ
