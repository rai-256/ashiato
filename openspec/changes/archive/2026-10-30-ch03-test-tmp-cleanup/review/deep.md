# CH03 deep-review（問いの独立検査）

検査対象: `openspec/changes/ch03-test-tmp-cleanup/deep-questions.json`（Q1〜Q7、全問 `kind: open`・推奨あり。`ask_wizard.py` は rc=0「止める 0 / 仮でよい 7」）。
入力: `docs/changes/CH03.md`（依頼の逐語）・`premises.md`・`docs/briefs/CH03.html`・repo の実物（2026-10-23、HEAD 4234669c / origin/main f0c6cf14）・走っている 4 本のブランチ・`/tmp` の実物。

## 手順ごとの記録（問いの一覧を見る前にやり直したもの）

- **手順 1（要件どうしの衝突）**: `satisfies: []`。要件は無く、依頼の中で食い違う組を探した。依頼 2「**テストのコード**で使ったら落ちる」と Q3 の推奨「main も含む木の全体」が範囲で食い違う → R12
- **手順 2（扉の幅）**: 該当なし（確かめた範囲: CH03.md の front matter に doors・requires・capability が無い。ブリーフも「要件 0 件・面なし」）
- **手順 3（新たに立つ一方通行）**: 消すのは試験が作った一時ファイルだけ。外に出すものも列の型・保存形式も無い。依存（`tempfile`）の追加は外せば戻る。一方通行が立つのは Q7 の選択肢 2（名前の頭で /tmp を消す台本）だけで、選択肢に書かれていない → R4
- **手順 4（日常に影響する選択）**: CI の時間（Q6 の選択肢 2 にある）・殺された試験の残り（R5）・失敗した試験の中身を後から覗けるか（R10）・並走する worktree が /tmp の固定名を取り合う（R11）
- **手順 5（既存コードが依頼を満たしていない箇所）**: grep と git で確かめた。走っている 4 本の数え方が古い `main` に対して取られていた（R1）、書き直しが走っている枝と同じファイルに当たる（R2）、Rust の手伝いの形と早すぎる drop（R8）、「消していない」の前提が一部違う（R9）、Kotlin の手伝いと property の初期化（R14）
- **手順 6（過去の取りこぼし）**: `python3 scripts/upstream_misses.py report` の観点を 1 つずつ当てた。当てはまったもの:
  - 「検証コマンドが成立しない（11 件）」「凍結前に verify-run で空走させる」→ Q6 の推奨の検証が、走らずに空のまま通りうる（R7）
  - 「他 Story の決定の波及」→ Q5 が検査の失敗しか見ておらず、本文の衝突を見ていない（R2）
  - 「既存コードの振る舞いを確かめずに置いた」→ Q1 の context「消していない」（R9）
  - 「1 件見つかったら同じ形を機械で横展開する」→ 検査の書き方の兄弟（R6）、漏らしている手伝いの数え漏れ `st04`（R3・R14）
  - 「見積もり・根拠を実物で測っていない」→ Q7 の「計 2.4G」の内訳（R3）
  - 時計・保存場所・書き直せない記録・画面寸法の観点は該当なし（時刻も手元の状態も扱わない change）
- **手順 7（族の網羅）**: 「落ちても消える」の族（panic・assert の失敗 / 殺された・timeout・abort / 消さずに残す口）→ R5・R6・R10。「/tmp を食い潰す・取り合う」の族（試験の漏れ / bash の固定名 / harness の python の漏れ）→ R3・R11
- **手順 8（前提の確かさ）**: `premises.md` はある。`loss` の載った決定は無い（全問 B）ので、確かさ 4 未満に `loss` が載る違反は無い。ただし確かさ 4 と書いた行が古い基準で測られていて結果が誤り（R1）、grep 1 回きりを 4 と書いている（R13）、Q6 の推奨に要る前提の行が無い（R7）

---

## R1. 走っている 4 本の数え方が、古いローカルの `main`（2026-10-15）に対して取られていて誤り。新しく足しているのは st09 が 7 か所・st13 が 4 か所だけで、ch01・ch02 は 0 か所
- 成果物: openspec/changes/ch03-test-tmp-cleanup/deep-questions.json（Q5 の question・context）/ openspec/changes/ch03-test-tmp-cleanup/premises.md（4 行目「走っている 4 本も…」）
- 根拠: `git rev-parse main` = c6109b7f（2026-10-15 23:44）、`origin/main` = f0c6cf14（HEAD はこれを含む）。`git diff <base>...<branch> -- '*.rs' '*.kt' | grep '^+' | grep -cE 'temp_dir\(\)|createTempDirectory|createTempFile'` を数えると、base が `main` なら 26 / 23 / 19 / 19（premises の「19〜26 行」をそのまま再現した）、base が `origin/main` なら **st09 = 7・st13 = 4・ch01 = 0・ch02 = 0**。中身は、st09: `MediaCaptureInstrumentedTest.kt`・`MediaPremisesInstrumentedTest.kt`（androidTest）・`MediaCapabilityTest.kt`・`MediaRecordSizeTest.kt`・`MediaScanFixtures.kt`・`OriginalLedgerTest.kt`・`crates/server/src/media_tests.rs`。st13: `archive_scan_interval_tests.rs` に 2 か所・`archive_tests.rs` に 1 か所・`tests/server_startup.rs` に 1 か所
- kind: premise
- 提案: Q5 の「4 本は…検査に落ちます」を「st09 と st13 の 2 本が落ちる（ch01・ch02 は足していない）」に直す。premises の 4 行目は `origin/main...<branch>` で数え直して書き直す（`/tmp/ashiato-scan-interval-*` 26 個は st13 から出ている、と足す）。数え直すと選択肢 2 と 3 の手間の比較も変わる
- 処置: fixed deep-questions.json —— `origin/main...<branch>` で数え直した（st09 7・st13 4・ch01/ch02 0。手元で再現）。Q4（旧 Q5）を「st09 と st13」に直し、premises.md の行を書き直して基準を冒頭に書いた

## R2. 抜け: Q5 は検査に落ちることだけを見ていて、CH03 の書き直しが走っている枝と**同じファイル**に当たる（rebase で本文が衝突する）ことが context に無い
- 成果物: openspec/changes/ch03-test-tmp-cleanup/deep-questions.json（Q5）
- 根拠: CH03 が書き直すファイル（`grep -rlE 'temp_dir\(\)' crates` と `grep -rlE 'createTempDirectory|createTempFile' collector-android/app/src`）と `git diff --name-only origin/main...<branch>` の重なり: st13 が `archive_flow_tests.rs`（+758 行）と `archive_tests.rs`（+162 行）、st09 が `archive_tests.rs`・`export_file_tests.rs`、ch02 が `export_file_tests.rs`（`export_volume_one_run_reports_time_and_memory` の中）。CH03 が `tempfile` を足す `crates/server/Cargo.toml` は st09・st13 も直している。CI に検査を足す `.github/workflows/ci.yml` は st09 が直している。Q6 で試験の JVM の一時フォルダを向けるなら `collector-android/app/build.gradle.kts` は ch01 が直している
- kind: technical
- 提案: Q5 の context に、検査に落ちる枝（R1）とは別に「本文が衝突しうる枝とファイル」を足す。選択肢 3（4 本の merge の後に有効にする）でも衝突は避けられないことを detail に書く。merge の順は問いにしない（`.harness2/docs/flow-gates.md:245`「merge の順序を design に書かない。後から merge する側が追従する」）
- 処置: fixed deep-questions.json —— Q4 の context に本文が衝突しうる枝とファイルを足し、選択肢 3 の detail に「本文の衝突は残る」と書いた。merge の順は問いにしない。premises.md に行を足した

## R3. 前提の誤り: Q7 と premises の「試験の残り…計 2.4G」は内訳が違う。ashiato の試験の残りは約 428M で、約 1.6G は harness2 の python の試験が残した `tmpXXXXXXXX` が 15,834 個。`st04<数字>` の 2,856 個も表に無い
- 成果物: openspec/changes/ch03-test-tmp-cleanup/deep-questions.json（Q7 の question・context）/ openspec/changes/ch03-test-tmp-cleanup/premises.md（5 行目「いまの /tmp の残り」）
- 根拠: `du -shc /tmp/{ashiato-*,outbox*,st0*,st09-media*,heartbeat*,clock*,counters*,legacy*,hb*,both*,broken-outbox*,originals*}` = 428M。`ls -A /tmp | grep -cE '^tmp[a-z0-9_]{8}$'` = 15,834、合計 1.6G。中身は `openspec`・`.git`・`origin.git` などで、harness2 の試験の形（harness2 にはもう `tests/_tmp.py` と `tests/test_tests_use_tmp_helper.py` があり、今日作られたものは `find /tmp -maxdepth 1 -name 'tmp????????' -newermt 2026-10-23` で 0 件）。`st04*` 2,856 個は `TestOutbox.kt:52` の `TestStores`（`Files.createTempDirectory("st04")`）から出ている
- kind: premise
- 提案: Q7 の context を「2.5G のうち ashiato の試験の残りは約 0.4G。約 1.6G は harness2 の試験の古い残りで、CH03 では減らない」に直す。premises の表に `st04*`・`tmp????????` の行を足す。推奨はこのままでも、人間が「CH03 で /tmp の問題が片付く」と読まないようにする
- 処置: fixed deep-questions.json —— Q6（旧 Q7）を ashiato の残り 約 0.45G に直し、`st04*` を足した（手元で 2,855・15,834 を再現）。note に「harness2 の約 1.6G は CH03 の外」と書いた。premises.md の行も直した

## R4. 不可逆の記述が無い: Q7 の選択肢 2（名前の頭と古さで消す台本）は、試験と関係の無い同じ頭のファイルまで消しうる。消したものは戻らない
- 成果物: openspec/changes/ch03-test-tmp-cleanup/deep-questions.json（Q7 の選択肢 2）/ premises.md の冒頭「失われるもの（loss）のある決定は無い」
- 根拠: 試験の頭は汎用の語（`clock`・`legacy`・`counters`・`both`・`hb`・`heartbeat`・`outbox`・`st`）。`/tmp` にはいま、試験ではないセッションの作業ファイル `st33_res.py`・`st33_ot.py`・`st41-list.txt`・`st29go`・`st29chk.mjs` がある（`ls -d /tmp/st*`）。Kotlin の `createTempDirectory("st04")` は `st04<数字>` になるので、頭を `st` で拾うとこれらが当たる
- kind: irreversible
- loss: discarded
- 提案: 選択肢 2 の detail に「頭が汎用の語なので、試験でないファイル（例: /tmp/st33_res.py）も消しうる。消したものは戻らない」と書く。失わない側（選択肢 1）が推奨で、高くつかず範囲も越えないので、B のままでよい。premises の冒頭の「loss のある決定は無い」は「推奨の側には無い」に直す
- 処置: escalated —— Q6 の選択肢 2 に `irreversible`（試験でないファイルも消しうる）を書き、premises.md の冒頭を「推奨の側には無い」に直した。失わない側（選択肢 1）が推奨で、高くつかず範囲も越えないので、問いは B のまま本人に見せる

## R5. 族の抜け: 「落ちても消える」は panic と assert の失敗には効くが、試験が**殺された**とき（timeout・Ctrl-C・OOM・Gradle の worker が kill された）には Drop も `TemporaryFolder.after()` も走らない。その残りをどう扱うかが一覧に無い
- 成果物: openspec/changes/ch03-test-tmp-cleanup/deep-questions.json（Q1・Q2・Q7）
- 根拠: premises の「TempDir は drop で消える…abort でなければ panic でも走る」は unwind の場合だけ。`scripts/quiet-run` や verify の timeout で落ちた cargo / gradle は SIGKILL で終わり、デストラクタも JVM の後始末も走らない。`/tmp/tmp.*`（bash の mktemp）7 個のうち 6 個は trap のある台本の形（`migrations tools`）で、殺された run の残りと読める
- kind: daily
- 提案: Q7 に「殺された run の残りは、再起動か手で消すまで残ってよいか」を 1 行で足すか、B の問いを 1 つ足す（選択肢の例: 残ってよい / 試験の一時フォルダを run ごとの 1 つの根（worktree の `target/` の下など）にまとめて、殺されても 1 か所を消せば済むようにする）。推奨を必ず付ける
- 処置: escalated —— 問い Q7（B。推奨: 残ってよい）を足した。premises.md の TempDir の行に SIGKILL では走らないことを書いた

## R6. 族の抜け: Q3 の検査が拾う書き方は 4 つだけで、同じ漏れ方の兄弟の書き方と、後始末を外す口を問うていない
- 成果物: openspec/changes/ch03-test-tmp-cleanup/deep-questions.json（Q3）
- 根拠: Q3 の対象は `std::env::temp_dir()` / `Files.createTempDirectory` / `createTempFile` / `File.createTempFile` の文字列。拾えない兄弟: `use std::env;` の後の `env::temp_dir()`、`kotlin.io.createTempDir()`（非推奨だが使える）、`System.getProperty("java.io.tmpdir")`、`"/tmp/…"` の直書き（いま `archive_tests.rs:121` に文字列だけある）。後始末を外す口: `tempfile::TempDir::keep()` / `into_path()`、`NamedTempFile::keep()` / `persist()`、`@Rule` / `@get:Rule` を付けずに `TemporaryFolder().create()` を手で呼ぶ形。先例として harness2 には「試験が tmp の手伝いを使っているか」を見る `tests/test_tests_use_tmp_helper.py` がある
- kind: technical
- 提案: Q3 の context に兄弟の書き方と外す口を並べ、どこまで拾うかを選択肢に入れる（B。推奨を付ける）。何を使えばよいかの規約は `docs/testing.md` にまだ 1 行も無い（`grep -n 'TempDir\|TemporaryFolder\|tempdir' docs/testing.md` は 0 件）ので、検査の文言と同じことを `docs/testing.md` に書くかは design の既定（C）で足りる
- 処置: fixed deep-questions.json —— 問い Q2（B。推奨: 兄弟の全部と外す口も落とす）を足した。規約を docs/testing.md に書くことは C3（premises.md）

## R7. Q6 の推奨の検証は、試験が走らないまま「フォルダは空」で通りうる。Gradle の試験の JVM の一時フォルダの向け方も premises に無い
- 成果物: openspec/changes/ch03-test-tmp-cleanup/deep-questions.json（Q6）/ openspec/changes/ch03-test-tmp-cleanup/premises.md
- 根拠: (a) `collector-android/app/build.gradle.kts:54-60` の注記どおり `testDebugUnitTest` は `UP-TO-DATE` で rc=0 のまま走らないことがある（走らなければフォルダは空で通る）。(b) server の試験は DB（`testdb.rs`）が要り、DB が無いとフォルダを作る前に落ちる。(c) OpenJDK の Unix の既定の `java.io.tmpdir` は `/tmp` 固定で、`TMPDIR` を読まない（知識、確かさ 1）。台本から試験の JVM の先を変えるには `build.gradle.kts` の `unitTests.all` で `systemProperty("java.io.tmpdir", …)` を環境変数から渡すような変更が要る。この行は premises に無く、ch01 も同じファイルを直している（R2）。(d) `TMPDIR` を cargo 全体に向けると、rustc や Robolectric が自分の一時ファイルをそこに置き、「空」の判定がそれで落ちうる。upstream_misses の「検証コマンドが成立しない（11 件）」と同じ型
- kind: technical
- 提案: Q6 の context に (a)〜(d) を書き、推奨の detail に「走った試験の数が 1 以上であることも見る（Gradle は `--rerun`、cargo は passed の合計）」を足す。premises に「試験の JVM の `java.io.tmpdir` を台本から向けられる」の行を足し、下流の最初の Task で 1 回走らせて確かさ 4 にする
- 処置: fixed deep-questions.json —— Q5（旧 Q6）の context に (a)〜(d) を書き、推奨に「走った試験の数が 1 以上」「cargo は --no-run の後に TMPDIR」「Gradle は --rerun」を足した。premises.md に java.io.tmpdir（確かさ 1）と UP-TO-DATE の行を足した

## R8. Rust の手伝いが `PathBuf` を返す形のまま TempDir に替えると、早すぎる drop で、試験が確かめている経路が気づかれずに変わる。問いの context に無い
- 成果物: openspec/changes/ch03-test-tmp-cleanup/deep-questions.json（Q1 の context・選択肢 1 の detail）
- 根拠: `crates/server/src/export_tests.rs:16` の `tmp_dir()` は `PathBuf` を返し、`export_tests.rs`・`export_file_tests.rs`・`export_job_tests.rs` の 57 か所から呼ばれる（`grep -rn 'tmp_dir(' crates/server/src | wc -l`）。ほかに `runtime.rs:855` の `cfg()`、`history/ledger.rs:183` の `temp_dir()` も同じ形。`tmp_dir("x").path().to_path_buf()` のように書くと TempDir はその行で消える。書き出しは置き場が無ければ作る（`export.rs:983` の `ensure_dir`）ので、試験は落ちずに「置き場がある」経路から「置き場が無い」経路に変わり、作り直された置き場は漏れる。Q3 の grep ではこれは拾えず、拾えるのは Q6 の台本だけ（推奨では CI に入らない）。もう 1 つ、`TempDir` は Unix で 0700 のフォルダを作るので、`create_dir_all` の 0755 と違う（いま当たる試験は無い。`export_tests.rs:142` は自分で 0755 にしている）
- kind: technical
- 提案: Q1 の context に「手伝いは TempDir（か、それを持つ型）を返す形に変える。早すぎる drop は試験を落とさずに経路を変える」と書く。Q6 で CI にしない推奨の理由（「漏れの予防は依頼 2 の検査が持っている」）は、この型の漏れには当たらないことを Q6 の選択肢 2 の detail に足す
- 処置: fixed premises.md —— C1 に「手伝いは TempDir（か、それを持つ型）を返す。0700 の権限」を書き、表に行を足した。Q5 の context に「早すぎる drop の漏れは grep で拾えない」と書き、推奨を CI の既存の段で測る形に変えた（試験を 2 回走らせない）

## R9. 前提の誤り: Q1 の context「45 か所で手書きし、消していない」は一部違う。成功したときは消している試験が多く、漏れの大半は書き出しの `tmp_dir()`（一度も消さない）と、失敗したときの経路
- 成果物: openspec/changes/ch03-test-tmp-cleanup/deep-questions.json（Q1 の context）/ docs/changes/CH03.md の「archive_flow_tests ほか」
- 根拠: `grep -rn 'std::env::temp_dir()' crates | wc -l` = 46。成功したときに消しているもの: `archive_tests.rs:504,549,893,909,921,1291,1334,1379,1414,1451,1503,1653`（`remove_dir_all(root).unwrap()`）、`outbox.rs:199,228,246,262`・`outage_notice.rs:503,512,623,649`・`history/locate.rs:249,301`（`.ok()`）。Drop で消しているもの: `archive_flow_tests.rs:36`（`impl Drop for Inbox`）、`tests/runtime_windows.rs:567,646`（自前の `TempDir`・`Cleanup`）。消していないもの: `export_tests.rs:16` の `tmp_dir()`（呼び出し 57 か所。`/tmp/ashiato-export-*` は 1 個が約 4M）
- kind: premise
- 提案: Q1 の context を「46 か所。成功したときは消すが失敗したときに残すものと、一度も消さないもの（書き出しの `tmp_dir()`）がある。`archive_flow_tests` と `runtime_windows` はもう Drop で消している」に直す。依頼の逐語（CH03.md）は直さない
- 処置: fixed premises.md —— 旧 Q1 は R15 で C1 に移したので、漏れ方の 3 通り（一度も消さない / 成功時だけ消す / もう Drop）と 46 か所を表に書いた

## R10. 抜け（変えてよい振る舞い）: いまは失敗した試験の一時フォルダが残り、後から中身を覗ける。CH03 の後は消える。これを変えてよいか、残す口を持つかが問われていない
- 成果物: openspec/changes/ch03-test-tmp-cleanup/deep-questions.json（一覧に無い）
- 根拠: `.claude/skills/story-upstream/SKILL.md:50-51`「振る舞いを変えない change でも、変えてよい振る舞いが本当に無いかを問う」。一覧で「ついでに」を問うているのは Q4（bash）だけ。失敗した試験の残りを手掛かりにする道（下流の fix の agent が落ちた試験の書き出しを読むなど）は、TempDir / TemporaryFolder にすると無くなる。`grep -rnE 'KEEP_TMP|keep\(\)|into_path' crates collector-android/app/src` は 0 件で、いまは残す口も無い
- kind: daily
- 提案: B の問いを 1 つ足す（選択肢の例: 残す口を持たない。もう一度走らせれば再現する / 環境変数（例 `ASHIATO_KEEP_TEST_TMP=1`）のときだけ残す。残す口は R6 の検査の「外す口」の例外になる）。推奨を付ける
- 処置: escalated —— 問い Q8（B。推奨: 残す口を持たない）を足した

## R11. 族の抜け: /tmp を食い潰す族の兄弟として、並走する worktree が /tmp の**固定名**のファイルを取り合う形が Q4 の context に無い
- 成果物: openspec/changes/ch03-test-tmp-cleanup/deep-questions.json（Q4 の context）
- 根拠: `tools/stack.sh:102` `> /tmp/ashiato-seed.log`、`tools/smoke.sh:118` `> /tmp/ashiato-smoke.sql`、`tools/check-licenses.sh:13` `> /tmp/meta.json`、走っている tasks の検証 `openspec/changes/ch01-st17-review-fixes/tasks.md:15-17`・`openspec/changes/st13-scheduled-account-fetch/tasks.md:17` の `tee /tmp/ct.log`（`vt.log`・`et.log`）。量は増えないが、同時に走る 2 つの worktree が同じファイルを書き、検証の判定を取り違えうる。Q4 の context は `mktemp` の 23 か所しか数えていない（`grep -rn mktemp tools | wc -l` = 23 は合っている）
- kind: technical
- 提案: Q4 の context に固定名の 4 か所を足し、この change の範囲に入れるか別の change にするかを選択肢で分ける（B。推奨を付ける）。tasks の `/tmp/ct.log` は harness2 側の規約なので、外に出すなら置き場（harness2）も書く
- 処置: fixed deep-questions.json —— Q3（旧 Q4）の context に固定名の 4 か所を足し、選択肢 2 に「固定名 3 か所を mktemp + trap に直す」を入れた。tasks の /tmp/ct.log は harness2 側と書いた

## R12. 範囲の食い違い: 依頼 2 は「**テストのコード**で使ったら落ちる検査」だが、Q3 の推奨は製品のコード（`main`）と端末の上で走る `androidTest` も縛る。その帰結が detail に書かれていない
- 成果物: openspec/changes/ch03-test-tmp-cleanup/deep-questions.json（Q3 の選択肢 1）
- 根拠: docs/changes/CH03.md:29「テストのコードで使ったら落ちる検査」・:32「製品のコード（export.rs など）の置き場の決め方も変えない」。製品のコードにはいま 0 か所（`config.rs:140`・`autostart.rs:67` の `#[cfg(test)]` より後ろにあることを確かめた）なので今日の製品は変わらないが、推奨では、一時ファイルが要る製品のコードを書く後の Story は必ず `tmp-ok` を書くことになる。`androidTest` は端末の上で走り、ホストの /tmp には出ない。st09 は `androidTest` に 2 か所足している（R1）ので、推奨では st09 の fix で計測試験（エミュレータ）を走らせ直す手間が増える
- kind: conflict
- 提案: 選択肢 1 の detail に「依頼の『テストのコード』より広い。後の Story の製品のコードも縛る」「androidTest はホストの /tmp に出ないが、st09 の 2 か所は計測試験の走らせ直しになる」を書く。範囲を依頼の文言どおりにする選択肢（試験だけ、ただし判定はファイルの置き場で: `*_tests.rs`・`tests/`・`app/src/test`）と比べられるようにする
- 処置: escalated —— Q1（旧 Q3）の選択肢を 3 つにし（Rust 全体 + Kotlin test / + androidTest / 試験だけを解析で判定）、推奨の detail に「依頼の文言より広い・後の Story の製品のコードも縛る」「androidTest を外せば st09 は計測試験を走らせ直さない」を書いた

## R13. 確かさの付け方: premises の 1〜4 行目は grep か読むだけで、外れたら落ちる台本ではないのに確かさ 4。4 行目はその上で結果が誤り（R1）
- 成果物: openspec/changes/ch03-test-tmp-cleanup/premises.md
- 根拠: `.claude/skills/story-upstream/SKILL.md:116-117` の段は「4 走らせた（本物のコードか材料に当てて、外れたら落ちる台本かテスト）/ 3 外れる場合が起きないことを 1 歩ずつ辿った」。1 行目（「全部が試験の側」）は `#[cfg(test)]` の行と目で突き合わせたもの、3 行目は `export_file_tests.rs:619` を読んだもの。`loss` の載った決定は無いので、今回は止める理由にはならない
- kind: premise
- 提案: 1〜4 行目は 3 に直す（4 行目は数え直して書き直す）。Q3 の検査の `--self-test` が「製品のコードに 0 か所」を落ちる形で見るなら、下流でそれを 4 の確かめ方にする
- 処置: fixed premises.md —— grep と目視の行を確かさ 3 に下げた（`/tmp` の実物と依頼の実測の行は 5 のまま）

## R14. Q2 の context に、Kotlin の手伝いの形の数え漏れと、TemporaryFolder を property の初期化で使えないことが無い。呼び出し側を書き換えずに済む案も選択肢に無い
- 成果物: openspec/changes/ch03-test-tmp-cleanup/deep-questions.json（Q2）
- 根拠: 手伝いは `TestOutbox.kt` の `testOutbox()`・`testHeartbeatOutbox()` のほかに `TestOutbox.kt:52` の `TestStores`（既定の引数が `createTempDirectory("st04")`。/tmp の `st04*` 2,856 個）がある。呼び出しは 17 ファイル・49 か所（`grep -rn 'testOutbox()\|testHeartbeatOutbox()\|AppUsageWorld(\|St04World(' … | wc -l`）。`SegmentStoreTest.kt:21`・`OutboxStoreTest.kt:22`・`ElapsedClockTest.kt:17`・`SegmentMigrationTest.kt:18` は `private val dir = Files.createTempDirectory(…)` を property の初期化で作る。JUnit4 の `TemporaryFolder.newFolder()` は rule の `before()` より前に呼ぶと `IllegalStateException`（「the temporary folder has not yet been created」。https://junit.org/junit4/javadoc/latest/org/junit/rules/TemporaryFolder.html）なので、`lazy` か `@Before` に移す書き換えが要る
- kind: technical
- 提案: Q2 の context に `TestStores`・49 か所・property の初期化の書き換えを足す。手伝いの呼び出しを変えずに済む案（JVM ごとに 1 つの根を作り、手伝いはその下に作り、JVM の終わりに根を消す）を選択肢に足すか、採らない理由を detail に書く
- 処置: fixed premises.md —— C2 に手伝い（`TestStores` を含む 17 ファイル・49 か所）はフォルダを引数で受け取る形、property の初期化の 4 つは `lazy` か `@Before` と書き、表に行を足した

## R15. 不要: Q1 と Q2 は依頼が「どの手段にするかは決めてよい」と AI に任せている。問いにせず design の既定（C）にしてよい
- 成果物: openspec/changes/ch03-test-tmp-cleanup/deep-questions.json（Q1・Q2）
- 根拠: docs/changes/CH03.md:27-28「Rust は落ちても消える仕組み（例: tempfile の TempDir の drop）、Kotlin は JUnit の TemporaryFolder か @After での削除。どの手段にするかは決めてよい」。どちらの選択肢も失うものが無く、範囲も越えない（`tempfile` は MIT OR Apache-2.0 で `tools/check-licenses.sh:9` の許可の中）
- kind: defer
- 提案: Q1・Q2 を外し、推奨の答えを `design.md` の D 番号に既定として書く。R8・R14 の注意（手伝いの形・早すぎる drop・property の初期化）は design の同じ D に移す。人間に見せる問いは Q3〜Q7（と足す問い）にする
- 処置: fixed premises.md —— 旧 Q1・Q2 を問いから外し、C1・C2（premises.md の「聞かずに決めるもの」）にした。問いの note に C の一覧を出し、異論は補足欄で受ける
