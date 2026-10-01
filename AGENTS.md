# ashiato2 — agent への指示（プロジェクト固有）

ここには **ashiato2 だけのこと**を書く。プロジェクト全体の規約の正本は `CLAUDE.md`（構成・テスト・git 運用）。
Claude Code は `CLAUDE.md` の末尾の import でこのファイルを読み、Codex は自分で読む。

ハーネス共通の規律の届き方は executor で違う:
- **Codex**: harness2 がグラフから `codex exec` を起こすたびに `developer_instructions` で渡す（`.harness2/codex/executor.md`。
  役割・長いコマンドの待ち方・`verify-run`・push / PR をしない境界）
- **Claude**: implement / fix は prompt に埋め込まれた `.harness2/skills/story/task.md`、それ以外は呼ばれた skill が持つ
- **正式な実行経路はグラフ（`scripts/hx`）だけ。** 手で `codex` / `claude` を起動したセッションには上のハーネスの規律は届かない
  （このファイルと `CLAUDE.md` だけ）。そのセッションで Story を進めない。main への commit とレビュー前の `feat/st*` の push は、
  どの経路でも `.githooks/` が止める

## 実装と計画の規約（上流の計画 agent・Task agent・reviewer に共通）

上流が `tasks.md` に書く検証・置き場も、下流が実装する形も、これに合わせる。コードの置き場は `CLAUDE.md` の「構成」。

### テスト
<!-- harness2:tests -->
- 置き場と走らせ方の正本は `docs/testing.md`（層ごとの表と CI の job）。単体の置き場は `CLAUDE.md` の「構成」の表
- **画面の Scenario は `web/e2e`（本物のブラウザ）で担保する。** jsdom へも「人間の確認待ち」へも逃がさない（置けるのは物理だけ）。
  走らせ方は `docs/testing.md` §4.5（`cd web && npm run test:e2e`）
- `// Scenario: <名前>` の印は、Rust / Kotlin / TypeScript のテストのコメント、bash は echo か `#`
- `cargo test <名前>` 型は `-- --list` で 1 本以上あることも確かめる（0 本でも rc=0）

### 移行
<!-- harness2:migrations -->
- 移行の名前は `YYYYMMDDHHMM_<slug>.sql`（と `.down.sql`）、`migrations/` に置き、`crates/server/src/lib.rs` の `MIGRATIONS` の末尾へ。
  連番にしない（並走する Story が番号を取り合う）。`tools/check-migrations.sh` が形を見る

### 長いコマンド
<!-- harness2:long-commands -->
- **数十秒以上かかるもの**は repo の根から `scripts/quiet-run` で走らせ、1 回で待つ。ashiato2 で該当するのは
  **gradle（`collector-android`）/ `cargo test` / `tools/smoke.sh` / 計測テスト（`tools/android-emulator.sh`）**。
  gradle は `-p` でプロジェクトを指す:
  `scripts/quiet-run unit -- ./collector-android/gradlew -p collector-android :app:testDebugUnitTest --tests '*SourceIndependenceTest*'`

## Android の環境は設定済み（**探さない**。executor に依らない）
<!-- harness2:env -->

ハーネスが起動時に `tools/agent-env.sh` の export を渡している: `JAVA_HOME` / `ANDROID_HOME` / `ANDROID_SDK_ROOT` / `PATH`
（`tools/android-env.sh` と同じ値）と `ASHIATO_ROBOLECTRIC_JARS`（Robolectric をオフラインで動かす jar の置き場。
`app/build.gradle.kts` が読む）。Codex にはさらに、sandbox で書ける道具のキャッシュ（`HARNESS_CODEX_WRITABLE_DIRS` =
`~/.cargo` `~/.gradle` `~/.android`）と、gradle の daemon を使わない `GRADLE_OPTS` を渡す（daemon は sandbox の制限を
持ったまま生き残り、外のビルドを壊す）。`env` / `rg JAVA_HOME` / `rg robolectric` で探したり、`tools/android-env.sh` を
source し直したりしない。gradle が「JDK が無い」「Robolectric の jar を落とせない」で落ちたときだけ、
`echo $JAVA_HOME $ASHIATO_ROBOLECTRIC_JARS` を 1 回確かめる。

### `/dev/kvm`（**Codex のとき**。Codex の sandbox の中でだけ起きる）

`/dev/kvm` は Codex の sandbox の中では開けない。エミュレータが要る検証（`scripts/verify-run` が `BLOCKED_INFRA`:「/dev/kvm を開けない」を返したもの）
**だけ**を、`sandbox_permissions: require_escalated` で sandbox の外での実行を申請して走らせ直す。それ以外の操作は sandbox の中で行い、
外での実行を申請しない（2026-09-26 実測: `scripts/verify-run 7.1` は申請 1 回・承認 1 回で PASS）。
Claude（`claude -p`）は sandbox を設定していない（`~/.claude/settings.json` にも project にも無い）ので、この申請は無い。

## 正典

関門・レビューの席・指摘の R 形式は harness2 の `.harness2/docs/flow-gates.md`。ashiato2 固有の検査は `docs/flow-gates.md`、
テスト規約は `docs/testing.md`。
