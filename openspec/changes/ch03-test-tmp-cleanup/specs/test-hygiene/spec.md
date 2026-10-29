## Purpose

試験が一時フォルダに作ったものを残さず、手書きの一時フォルダを検査が入る前に止め、CI が試験のたびに残りを測る。並走する Story と change が同じ機械の上で検証を回し続けられるための、開発の道具の約束。

## ADDED Requirements

### Requirement: 試験は自分が作った一時ファイルを、通っても落ちても残さない

THE SYSTEM SHALL サーバと PC の収集アプリの試験（`cargo test --workspace`）と、携帯の収集アプリの単体試験（`:app:testDebugUnitTest`）が一時フォルダに作ったファイルとフォルダを、
その試験が終わるまでに消す。試験が通ったときも、assert が外れた・panic した・例外が出たときも消す。
試験の過程が外から殺されたとき（SIGKILL・timeout・OOM・Gradle の worker の kill）の残りは、この約束の外とする。
導出元: CH03

#### Scenario: 一時フォルダを空のフォルダへ向けて試験を一通り走らせると、終わった後も空のまま
- **WHEN** Rust の試験（`cargo test --workspace`）と携帯の単体試験を、一時フォルダ（`TMPDIR` と、試験の JVM の `java.io.tmpdir`）を空のフォルダへ向けて走らせ、どちらも通る
- **THEN** 走り終えた後、そのフォルダは空である
- **AND** 走った試験の数は、Rust・携帯のそれぞれで 1 本以上である

#### Scenario: Rust の試験が panic しても、試験の一時フォルダは消える
- **WHEN** Rust の試験が、試験の手伝いから一時フォルダを受け取り、中にファイルを書いた後に panic する
- **THEN** panic が試験の外へ伝わった後、そのフォルダは在らない

#### Scenario: 携帯の単体試験が例外で落ちても、試験の一時フォルダは消える
- **WHEN** 携帯の単体試験が、試験の規則から一時フォルダを受け取り、中にファイルを書いた後に例外で落ちる
- **THEN** 試験の規則の後始末が済んだ後、そのフォルダは在らない

### Requirement: 手書きの一時フォルダは、検査が落とし、何を使えばよいかを示す

THE SYSTEM SHALL `tools/check-tmp-hygiene.sh` で、Rust の `crates/` の全体（試験も製品のコードも）と携帯の収集アプリの `app/src/test` を見て、
一時フォルダを手で作る・指す書き方と、後始末を外す書き方を、ファイルと行を挙げて rc=1 で落とす。
落とす書き方は、Rust の `temp_dir()`（`std::env::` の有無を問わない）・`"/tmp` で始まる文字列・`TempDir` と `NamedTempFile` の `keep()`・`into_path()`・`persist(`、
Kotlin の `createTempDirectory`・`createTempFile`・`createTempDir`・`java.io.tmpdir`・`"/tmp` で始まる文字列・`@Rule` か `@get:Rule` の付かない `TemporaryFolder(` とする。
落ちたときの文言には、Rust は `tempfile::TempDir` を試験の最後まで持つこと、Kotlin は `@get:Rule` の `TemporaryFolder` を使うこと、漏れでない使い方は行に `tmp-ok: <理由>` を書くことを示す。
印と規則の注釈は、その行と同じ行か 1 行上にあるときだけ数える。`tmp-ok:` の後に理由の無い印は印として数えない。携帯の `app/src/main` と `app/src/androidTest` は見ない。
この検査は CI の chain job と commit 前の検査（`tools/pre-commit.sh`）で走る。
導出元: CH03

#### Scenario: Rust の試験で一時フォルダを手で作ると、検査が落ち、TempDir を示す
- **WHEN** `crates/` の試験のコードに `std::env::temp_dir().join(…)` の行がある木で `tools/check-tmp-hygiene.sh` を走らせる
- **THEN** rc=1 で、そのファイルと行番号を挙げる
- **AND** 文言に `tempfile::TempDir` と `tmp-ok:` が出る

#### Scenario: 書き方を変えても、検査は素通りさせない
- **WHEN** `crates/` に `env::temp_dir()`・`"/tmp/x"`・`.keep()`・`.into_path()`・`.persist(` のどれか 1 つだけを持つ行がある木で走らせる
- **THEN** どの場合も rc=1 で、その行を挙げる

#### Scenario: 製品のコードの一時フォルダも、検査が落とす
- **WHEN** `crates/` の製品のコード（`#[cfg(test)]` の外）に `std::env::temp_dir()` の行がある木で走らせる
- **THEN** rc=1 で、その行を挙げる

#### Scenario: 携帯の単体試験で一時フォルダを手で作ると、検査が落ち、TemporaryFolder を示す
- **WHEN** `collector-android/app/src/test` に `Files.createTempDirectory`・`File.createTempFile`・`createTempDir()`・`System.getProperty("java.io.tmpdir")`・`"/tmp/x"`・規則の付かない `TemporaryFolder()`・2 行上にだけ `@get:Rule` のある `TemporaryFolder()` のどれか 1 つだけを持つ行がある木で走らせる
- **THEN** どの場合も rc=1 で、その行を挙げる
- **AND** 文言に `@get:Rule` の `TemporaryFolder` と `tmp-ok:` が出る

#### Scenario: 規則の付いた TemporaryFolder と tempfile は、検査を通る
- **WHEN** `crates/` に `tempfile::TempDir::new()` だけ、`app/src/test` に `@get:Rule val tmp = TemporaryFolder()` か、1 行上に `@get:Rule` のある `val tmp = TemporaryFolder()` だけを持つ木で走らせる
- **THEN** rc=0 である

#### Scenario: 理由を書いた tmp-ok の印が同じ行か 1 行上にあれば、検査を通る
- **WHEN** `std::env::temp_dir()` の行の同じ行に `// tmp-ok: 一時フォルダを読むだけ` がある木と、1 行上にある木で走らせる
- **THEN** どちらも rc=0 である

#### Scenario: 理由の無い tmp-ok の印と、2 行以上離れた印は、検査を通らない
- **WHEN** `std::env::temp_dir()` の行の同じ行に `// tmp-ok:` だけがある木と、2 行上に `// tmp-ok: 一時フォルダを読むだけ` がある木で走らせる
- **THEN** どちらも rc=1 である

#### Scenario: 携帯の main と androidTest は、検査の外
- **WHEN** `collector-android/app/src/main` と `collector-android/app/src/androidTest` にだけ `Files.createTempDirectory` の行がある木で走らせる
- **THEN** rc=0 である

#### Scenario: CH03 の後の木は、検査を通る
- **WHEN** CH03 を当てた木で `tools/check-tmp-hygiene.sh` を走らせる
- **THEN** rc=0 である

#### Scenario: 検査は CI の chain job と commit 前に走る
- **WHEN** `.github/workflows/ci.yml` の chain job と `tools/pre-commit.sh` を読む
- **THEN** chain job は `tools/check-tmp-hygiene.sh --self-test` と `tools/check-tmp-hygiene.sh` を走らせる
- **AND** `tools/pre-commit.sh` は `tools/check-tmp-hygiene.sh` が落ちれば commit を止める

### Requirement: CI は試験を走らせるたびに、一時フォルダに残ったものを測る

THE SYSTEM SHALL CI の rust job の `cargo test --workspace` と android job の `:app:testDebugUnitTest` を `tools/tmp-leak-check.sh` から走らせ、
一時フォルダを `/tmp` の下の専用の空のフォルダへ向け、試験の後にそのフォルダに残ったものがあれば、その名前を挙げて落とす（落ちた回・残りがあった回のフォルダは `target/tmp-leak/` へ移して残す）。
走った試験が 1 本も無いときも、フォルダが空でも落とす（`UP-TO-DATE` で試験が走らない・試験が始まる前に落ちる、を通さない）。
試験そのものが落ちたときは、試験の終了コードをそのまま返す。
導出元: CH03

#### Scenario: 試験の後に一時フォルダに残ったものがあれば、名前を挙げて落ちる
- **WHEN** `tools/tmp-leak-check.sh` が走らせた試験の命令が、一時フォルダにファイルを 1 つ残して rc=0 で終わる
- **THEN** 台本は rc=1 で、残ったファイルの名前を挙げる

#### Scenario: 試験が 1 本も走らなければ、一時フォルダが空でも落ちる
- **WHEN** `tools/tmp-leak-check.sh` が走らせた試験の命令が、試験を 1 本も走らせずに rc=0 で終わる
- **THEN** 台本は rc=1 で、走った試験が 0 本であることを挙げる

#### Scenario: 試験が落ちれば、台本も試験の終了コードで落ちる
- **WHEN** `tools/tmp-leak-check.sh` が走らせた試験の命令が rc=101 で終わる
- **THEN** 台本は rc=101 で終わる

#### Scenario: 試験が通り、何も残さなければ、台本は通る
- **WHEN** `tools/tmp-leak-check.sh` が走らせた試験の命令が、試験を 1 本以上走らせ、一時フォルダに何も残さずに rc=0 で終わる
- **THEN** 台本は rc=0 で、走った試験の本数を出す

#### Scenario: CI の試験の段は、残りを測る台本から走る
- **WHEN** `.github/workflows/ci.yml` の rust job と android job を読む
- **THEN** `cargo test --workspace` と `:app:testDebugUnitTest` は `tools/tmp-leak-check.sh` を通してだけ走る
