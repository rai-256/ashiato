# CH03 の前提（実物の調査）

外の振る舞いに依る前提は、Rust の標準ライブラリと `tempfile`・JUnit4 の `TemporaryFolder`・Gradle の試験の JVM の一時フォルダだけ。
**推奨の側に失われるものは無い**（消すのは試験が作った一時ファイルだけ。製品のデータにも置き場にも触らない）。失うものがあるのは
Q6 の選択肢 2（名前の頭で /tmp を消す台本。試験でない同じ頭のファイルも消しうる）だけで、推奨ではない。
repo と `/tmp` の実物は 2026-10-23 に数えた。**走っている枝との比べは `origin/main`（f0c6cf14）を基準にする**（worktree のローカル `main` は 10/15 のまま古い。review/deep.md R1）。

| 前提 | 何が依るか | 確かめ方 | 結果 | 出所 | 確かさ |
|---|---|---|---|---|---|
| `crates/` の `std::env::temp_dir()` は全部が試験のコード（`*_tests.rs`・`tests/`・`#[cfg(test)]` の中）で、製品のコードには 0 か所 | 検査を `crates/` 全体に当ててよいか（Q1）・「製品の置き場の決め方を変えない」 | `grep -rn 'temp_dir()' crates` の 20 ファイル・46 か所を、ファイルの `#[cfg(test)]` の行と目で突き合わせた | 全部が試験の側。製品の側は 0。下流で検査の `--self-test` と本体の走りが落ちる形で見れば 4 になる | 実物（repo） | 3 |
| Rust の漏れ方は 3 通り: 一度も消さない（書き出しの `tmp_dir()`。`export_tests.rs:16`、呼び出し 57 か所。`/tmp/ashiato-export-*` は 1 個 約 4M）/ 成功したときだけ消す（`archive_tests.rs` の `remove_dir_all(root).unwrap()` など。落ちると残る）/ もう Drop で消す（`archive_flow_tests.rs:36` の `Inbox`、`tests/runtime_windows.rs` の自前の `TempDir`） | C1 の書き換えの形 | 読んだ（review/deep.md R9） | 上のとおり | 実物（repo） | 3 |
| Rust の手伝い（`tmp_dir()`・`runtime.rs:855` の `cfg()`・`history/ledger.rs:183`）は `PathBuf` を返す。そのまま `TempDir` に替えると、その行で drop して消え、書き出しは置き場を作り直す（`export.rs` の `ensure_dir`）ので、試験は落ちずに確かめる経路が変わり、作り直した置き場が漏れる | C1・Q5（検査の grep では拾えない漏れ） | 読んだ（review/deep.md R8） | 手伝いは `TempDir`（か、それを持つ型）を返す形に変える | 実物（repo） | 3 |
| `collector-android` の `Files.createTempDirectory` / `createTempFile` は `app/src/test` の 10 ファイルだけ（`main` は 0。`androidTest` も main の上では 0、st09 が 2 か所足している） | 検査の範囲（Q1） | `grep -rlE 'createTempDirectory\|createTempFile' collector-android` | 10 ファイル。共通の手伝い（`TestOutbox.kt` の `testOutbox()`・`testHeartbeatOutbox()`・`TestStores`（`createTempDirectory("st04")`）、`AppUsageFixtures.kt`）は関数の中で作り、呼び出しは 17 ファイル・49 か所 | 実物（repo） | 3 |
| Kotlin の 4 つの試験（`SegmentStoreTest`・`OutboxStoreTest`・`ElapsedClockTest`・`SegmentMigrationTest`）は `private val dir = Files.createTempDirectory(…)` を property の初期化で作る。`TemporaryFolder.newFolder()` は rule の `before()` の前に呼ぶと `IllegalStateException` | C2 の書き換えの形 | 読んだ・公式文書 | `lazy` か `@Before` に移す | 実物（repo）・公式文書（https://junit.org/junit4/javadoc/latest/org/junit/rules/TemporaryFolder.html） | 2 |
| 手書きの一時フォルダを使ってよい（漏れでない）所がある | 検査の逃げ道（Q1） | `export_file_tests.rs:619` を読んだ | `std::env::temp_dir()` を**読むだけ**（書き出しが一時フォルダに漏れていないことを確かめる試験。R10）。ほかに `config.rs:157`・`autostart.rs:75` は絶対パスの文字列を作るだけで、何も作らない | 実物（repo） | 3 |
| 走っている枝のうち、同じ形の手書きの一時フォルダを足しているのは st09 と st13 だけ | 検査を main に入れた後、どの枝が rebase で落ちるか（Q4） | `git diff origin/main...<branch> -- crates collector-android` の追加行を数えた | st09 = 7（`MediaCaptureInstrumentedTest.kt`・`MediaPremisesInstrumentedTest.kt` の androidTest 2・`MediaCapabilityTest.kt`・`MediaRecordSizeTest.kt`・`MediaScanFixtures.kt`・`OriginalLedgerTest.kt`・`media_tests.rs`）、st13 = 4（`archive_scan_interval_tests.rs` 2・`archive_tests.rs` 1・`tests/server_startup.rs` 1）、ch01・ch02 = 0 | 実物（git） | 3 |
| CH03 が書き直すファイルは、走っている枝も直している | rebase の本文の衝突（Q4） | `git diff --name-only origin/main...<branch>` と突き合わせた（review/deep.md R2） | st13: `archive_flow_tests.rs`（+758 行）・`archive_tests.rs`。st09: `archive_tests.rs`・`export_file_tests.rs`・`.github/workflows/ci.yml`。ch02: `export_file_tests.rs`。`crates/server/Cargo.toml` は st09・st13、`collector-android/app/build.gradle.kts` は ch01 | 実物（git） | 3 |
| いまの `/tmp` の残り | Q6 | `ls -d /tmp/<頭>* \| wc -l`・`du -shc` | ashiato の試験の残りは約 0.45G（`ashiato-export*` 196・`outbox*` 5,145・`st04*` 2,855・`st09-media*` 2,221・`heartbeat*` 789 など。`seg*` は誰かが消して 0）。ほかに約 1.6G は harness2 の python の試験の古い残り `tmpXXXXXXXX` 15,834 個で、CH03 では減らない（harness2 はもう直していて、今日できたものは 0）。/tmp 全体は 15% | 実物（/tmp） | 5 |
| /tmp には試験でないセッションの作業ファイルが、試験と同じ頭で並んでいる（`st33_res.py`・`st41-list.txt`・`st29go` など） | Q6 の選択肢 2 の失うもの | `ls -d /tmp/st*` | 有る | 実物（/tmp） | 5 |
| Gradle の試験の JVM は、指定しなければ `java.io.tmpdir` = `/tmp` を使う | Kotlin の漏れ先・Q5 | `/tmp/seg*` が `SegmentStoreTest` の `createTempDirectory("seg")` から 264 個できていた（依頼の実測）。`app/build.gradle.kts` に `tmpdir` の指定は無い | /tmp に出る | 実物（依頼の実測・repo） | 5 |
| OpenJDK の Unix の `java.io.tmpdir` は `TMPDIR` を読まない。試験の JVM へは `build.gradle.kts` の `unitTests.all` で `systemProperty("java.io.tmpdir", …)` を渡す必要がある | Q5 | 知識 | 下流の最初の Task が 1 回走らせて確かめる | 知識 | 1 |
| `std::env::temp_dir()` と `tempfile` は `TMPDIR` を読む | Q5 | 公式文書 | Unix では `TMPDIR`、無ければ `/tmp` | 公式文書（https://doc.rust-lang.org/std/env/fn.temp_dir.html 、https://docs.rs/tempfile/latest/tempfile/fn.env_temp_dir.html） | 2 |
| `testDebugUnitTest` は `UP-TO-DATE` で rc=0 のまま走らないことがある | Q5（走らずに「空」で通る） | `collector-android/app/build.gradle.kts:54-60` の注記 | `--rerun` を付けないと走らないことがある | 実物（repo） | 3 |
| `tempfile::TempDir` は drop で中身ごと消える。試験が panic しても（unwind の間に）drop が走る。SIGKILL（timeout・Ctrl-C の強制終了・OOM）では走らない | C1・Q7 | 公式文書。repo の `Cargo.toml` に `panic = "abort"` の profile は無い | drop で消す。abort・kill では残る | 公式文書（https://docs.rs/tempfile/latest/tempfile/struct.TempDir.html）・repo | 2 |
| `tempfile` はまだ依存に無い（`Cargo.lock` に無い）。license は MIT OR Apache-2.0 で `check-licenses.sh` の許可に入る | C1 | `grep tempfile Cargo.lock`・`check-licenses.sh` の `ALLOW` | 依存に無い。MIT・Apache-2.0 は許可の中 | 実物（repo）・公式文書（crates.io） | 2 |
| JUnit4 の `TemporaryFolder`（`@get:Rule`）は試験が落ちても後で消す。Robolectric の runner でも rule は効く | C2 | 公式文書 | `after()` で再帰的に消す（失敗でも） | 公式文書（上の URL） | 2 |

確かさ 1〜2 で止めた前提は、どれも下流の最初の Task が走らせて確かめる（`TMPDIR` / `java.io.tmpdir` を空のフォルダに向けて試験を走らせ、後で空であること —— Q5 の検証そのもの）。
外れたら落ちる形なので、上流ではここで止める。

## 聞かずに決めるもの（C。spec の段が design の D 番号に写す）

依頼が「どの手段にするかは決めてよい」と任せている（review/deep.md R15）。異論があれば、どの問いの補足欄にでも番号で書く。

- **C1. Rust は `tempfile::TempDir`（dev-dependency）。** 手伝いは `PathBuf` ではなく `TempDir`（か、それを持つ型）を返し、試験は最後まで持ち続ける
  （早すぎる drop で経路が変わらないように。上の表の 3 行目）。もう Drop で消している `archive_flow_tests` の `Inbox`・`runtime_windows` の自前の型は、
  `TempDir` に寄せる（同じ手段に揃え、検査の逃げ道を増やさない）。`TempDir` は Unix で 0700 のフォルダを作る（`create_dir_all` は 0755）。
  権限を見る試験は自分で権限を付け直す（いま当たる試験は無い）
- **C2. Kotlin は JUnit4 の `TemporaryFolder`（`@get:Rule`）。** 共通の手伝い（`testOutbox()`・`testHeartbeatOutbox()`・`TestStores`・`AppUsageFixtures`）は
  フォルダを引数で受け取る形に変える。property の初期化で作っている 4 つの試験は `lazy` か `@Before` に移す
- **C3. 何を使えばよいかを `docs/testing.md` に 1 節で書く**（検査のエラーの文言と同じこと。いまは 1 行も無い）
- **C4. 試験の期待値・`// Scenario:` の印・試験の名前は変えない。** 書き換えるのは一時フォルダの作り方と後始末だけ（依頼の「触らない」）
