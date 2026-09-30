# ashiato2 — agent への指示（プロジェクト固有）

ハーネス共通の指示（Codex が executor のときの役割・長いコマンドの待ち方・`verify-run`・境界）は、harness2 が
起動のたびに渡している（`harness2/codex/executor.md`）。ここには **ashiato2 だけのこと**を書く。
Claude Code は `CLAUDE.md` の末尾の import でこのファイルを読む。プロジェクト全体の規約の正本は `CLAUDE.md`。

## Task agent の規約（ashiato2）

- **画面の Scenario は `web/e2e`（本物のブラウザ）で担保する。** jsdom へも「人間の確認待ち」へも逃がさない（置けるのは物理だけ）
- `cargo test <名前>` 型は `-- --list` で 1 本以上あることも確かめる（0 本でも rc=0）
- 移行の名前は `YYYYMMDDHHMM_<slug>.sql`、`crates/server/src/lib.rs` の `MIGRATIONS` の末尾へ
- 長いコマンドは repo の根から `scripts/quiet-run` で。gradle は `-p` でプロジェクトを指す:
  `scripts/quiet-run unit -- ./collector-android/gradlew -p collector-android :app:testDebugUnitTest --tests '*SourceIndependenceTest*'`

## Android の環境は設定済み（**探さない**）

ハーネスが起動時に `tools/agent-env.sh` の export を渡している: `JAVA_HOME` / `ANDROID_HOME` / `ANDROID_SDK_ROOT` / `PATH`
（`tools/android-env.sh` と同じ値）と `ASHIATO_ROBOLECTRIC_JARS`（Robolectric をオフラインで動かす jar の置き場。
`app/build.gradle.kts` が読む）。`env` / `rg JAVA_HOME` / `rg robolectric` で探したり、`tools/android-env.sh` を
source し直したりしない。gradle が「JDK が無い」「Robolectric の jar を落とせない」で落ちたときだけ、
`echo $JAVA_HOME $ASHIATO_ROBOLECTRIC_JARS` を 1 回確かめる。

`/dev/kvm` は sandbox の中では開けない。エミュレータが要る検証（`scripts/verify-run` が `BLOCKED_INFRA`:「/dev/kvm を開けない」を返したもの）
**だけ**を、`sandbox_permissions: require_escalated` で sandbox の外での実行を申請して走らせ直す。それ以外の操作は sandbox の中で行い、
外での実行を申請しない（2026-09-26 実測: `scripts/verify-run 7.1` は申請 1 回・承認 1 回で PASS）。

## 正典

関門の正典は `docs/flow-gates.md`、テスト規約は `docs/testing.md`。
