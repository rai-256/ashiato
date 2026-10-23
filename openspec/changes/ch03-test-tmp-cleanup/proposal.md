# CH03 — テストの一時ファイルの後始末

> 第 1 回の深掘りの答えを待たずに、推奨の答え（`deep.md` の `（答え待ち・推奨: …）`）で先に書いた（2026-10-20 の流れ）。
> 本人の答えが推奨と違えば `spec --answers` が写し直す。
> `docs/changes/CH03.md` の「価値」「壊してはいけないもの」「完了の判定」も、この proposal・design D7・tasks に合わせてこの段で書いた（review/spec.md R3・R15）。

## Why

テストが一時フォルダ（`/tmp`）に作ったファイル・フォルダを消さず、16G の tmpfs を満杯にした（2026-10-23 実測。`docs/changes/CH03.md`）。
満杯のあいだ、並走する Story / change の検証が「空きなし」で落ちた。漏れ方は 3 通り（一度も消さない / 通ったときだけ消す / もう Drop で消す。`premises.md`）で、
どれも手書きの一時フォルダ（Rust の `std::env::temp_dir()`、Kotlin の `Files.createTempDirectory`）から始まる。直しても、手で書く道が開いている限り同じ漏れが戻る。

## What Changes

- **試験の一時フォルダを、通っても落ちても消える作りにする。** Rust は `tempfile::TempDir`（dev-dependency）を試験の最後まで持ち、手伝い（書き出しの `tmp_dir()` など）は
  `PathBuf` でなく `TempDir`（か、それを持つ型）を返す。Kotlin は JUnit4 の `@get:Rule` の `TemporaryFolder` にし、共通の手伝い（`testOutbox()`・`testHeartbeatOutbox()`・`TestStores`・
  `AppUsageFixtures`）はフォルダを引数で受け取る（premises の C1・C2）
- **手書きの一時フォルダを落とす検査 `tools/check-tmp-hygiene.sh`（`--self-test` 付き）を足し、CI の `chain` job と commit 前の検査に掛ける。**
  当てるのは Rust の `crates/` の全体（製品のコードも）と Kotlin の `collector-android/app/src/test`（Q1 の推奨）。作る書き方の兄弟と、後始末を外す口も落とす（Q2 の推奨）。
  落ちたときの文言に何を使えばよいかを書き、漏れでない使い方は行の印 `tmp-ok: <理由>` で通す。木の全体に当てる（Q4 の推奨。走っている st09・st13 は rebase の後の gate → fix で直す）
- **試験を走らせるたびに一時フォルダの残りを測る `tools/tmp-leak-check.sh`（`--self-test` 付き）を足す。** CI の rust job の `cargo test --workspace` と android job の
  `:app:testDebugUnitTest` をこの台本から走らせ、一時フォルダを worktree の中の空のフォルダへ向け、終わった後に空であることと走った試験が 1 本以上であることを見る（Q5 の推奨）。
  Kotlin の試験の JVM へは、環境変数があるときだけ `build.gradle.kts` から `java.io.tmpdir` を渡す
- 何を使えばよいかを `docs/testing.md` に 1 節で書く（C3）。`docs/flow-gates.md` の検査の表に 1 行足す
- **振る舞いを変えないもの**: 試験の期待値・`// Scenario:` の印・試験の名前（C4）、製品のコードの置き場の決め方（依頼の「触らない」）。
  書き換えるのは一時フォルダの作り方と後始末だけ。既存の Scenario は、その印を持つ試験がそのまま通ることで固定される（CI と `check_scenarios.py`）
- **変わってしまう振る舞い（1 つ）**: 失敗した試験の一時フォルダも消え、後から中身を覗けなくなる。残す口は持たない（Q8 の推奨）
- やらないこと: bash の道具と `/tmp` の固定名（Q3）、いま `/tmp` にある残りの掃除（Q6）、殺された試験（SIGKILL）の残り（Q7）。design の Non-Goals

## Capabilities

### New Capabilities

- `test-hygiene`: 試験が一時フォルダに残さないこと・手書きの一時フォルダを検査が落とすこと・CI が残りを測ること。製品の振る舞いではなく、
  並走する Story と change が同じ機械の上で検証を回し続けられるための、開発の道具の約束（いま openspec/specs に道具の capability は無い。道具の capability を初めて立てる）。
  schema は「tooling は `skip_specs`」と言うが、採らない: 失敗した試験のフォルダが消える・検査が書き方を断る・CI が残りで落ちる、は外から見える振る舞いで、
  Scenario（18 本）を `check_scenarios.py` が印に縛り、正典に残る。`skip_specs` では tasks の検証だけが残り、archive の後に誰も固定しない。
  change の capability は `docs/stories/INDEX.md` に足さない（story-upstream の change の規則）

### Modified Capabilities

（無し）

## Impact

- コード（試験の側だけ）: Rust は `temp_dir()` を持つ 20 ファイル・46 か所（`crates/server` 5 ファイル、`crates/collector-windows` 15 ファイル。うち `tests/runtime_windows.rs` は Windows でだけ組まれる）と、
  `tmp_dir()` の呼び出し 57 か所（`export_tests.rs`・`export_file_tests.rs`・`export_job_tests.rs`）。Kotlin は `app/src/test` の 10 ファイルと、手伝いの呼び出し 26 ファイル・60 行（`TestStores(` などを含む）
- 製品のコード: 変えない（`temp_dir()` は 0 か所。`config.rs:157`・`autostart.rs:75`・`migrate_backup.rs:207` は `#[cfg(test)]` の中）
- 依存: `tempfile`（MIT OR Apache-2.0。`tools/check-licenses.sh` の許可の中）を `crates/server` と `crates/collector-windows` の `[dev-dependencies]` に足す
- 道具と CI: 新しいファイル `tools/check-tmp-hygiene.sh`・`tools/tmp-leak-check.sh`。`.github/workflows/ci.yml`（rust・android・chain の 3 job）・`tools/pre-commit.sh`・
  `collector-android/app/build.gradle.kts`（`unitTests.all` に 1 つ）・`docs/testing.md`・`docs/flow-gates.md` を直す
- `docs/changes/CH03.md` の front matter の `capability: []` はそのままにする（`test-hygiene` は新しく、走っている Story と重ならない。CH02 の D9 と同じ）
- **走っている Story・change との重なり**（`origin/main` = f0c6cf14 との差分。`premises.md`）:
  - **検査に落ちる**: st09（7 か所。うち androidTest の 2 か所は Q1 の推奨で検査の外）と st13（4 か所）。rebase の後、それぞれの gate → fix が直す（落ちたときの文言が何を使えばよいかを書く）。ch01・ch02 は 0
  - **コンパイルが通らない**: Kotlin の手伝いの引数が変わる（design D2）ので、st09（`testOutbox()`・`testHeartbeatOutbox()`・`TestStores()` の既定の引数・`TestStores(segmentBytes = 300)`）と st13（`testHeartbeatOutbox()`）は
    rebase の後に試験が組めない。Rust の `Inbox` は形を残す（st13 の `Inbox::new(` 15 か所・欄 16 か所は壊れない。design D1）。直しは検査と同じ fix で、commit 前の検査が追跡ファイル全体を見るので 1 つの commit にまとめる（design D5）
  - **本文が衝突しうるファイル**: `archive_flow_tests.rs`・`archive_tests.rs`（st13）、`archive_tests.rs`・`export_file_tests.rs`・`.github/workflows/ci.yml`（st09）、`export_file_tests.rs`（ch02）、
    `crates/server/Cargo.toml`（st09・st13）、`collector-android/app/build.gradle.kts`・`docs/testing.md`（ch01）。後から merge する側が rebase で追従する。merge の順は決めない。走っている Story へ差し戻さない
