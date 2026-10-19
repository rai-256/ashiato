# ashiato2

行動履歴（足跡）を記録・可視化するライフログ系プロジェクト。harness2 で上流から作り直す試走。

## 応答言語

ユーザーへの応答は **常に日本語**。例外はコード・識別子・パス・コマンド・ツールの生出力のみ。

## 構成

### コード（探索の入口。deep-review / iwakan / Task agent はここから当たりを付ける）
<!-- harness2:layout -->

| パス | 何が置かれているか | テストの置き場 |
|---|---|---|
| `crates/server` | S-01 バックエンド（Rust）。API・取り込み（`ingest.rs`）・滞在（`stay*.rs`）・稼働状況（`coverage*`）・属性・削除（`deletion.rs`）・破棄の報告（`drops.rs`。ST04）・画面のログイン（`web_session.rs`）・読み出しの記録（`access_log.rs`）・待ち受けと DB の接続先の検査（`net_guard.rs`）。`src/lib.rs` が router を持ち、`MIGRATIONS`（当てる版の配列）は `build.rs` が `migrations/ORDER` から組む。`src/bin/openapi.rs` が API 契約を生成する | 同じ `src/` の `*_tests.rs`（`testdb.rs` がテスト用 DB） |
| `crates/collector-windows` | C-02 PC の収集アプリ（Rust）。前景の窓・入力・ブラウザのアドレスバー（`platform.rs` `browsers.rs`）→ `engine.rs` → 送信（`outbox.rs` `sender.rs`）。`runtime.rs` が組み立て | 各モジュールの `#[cfg(test)]`、本物の Windows で走る `tests/runtime_windows.rs` |
| `collector-android` | C-01 携帯の収集アプリ（Kotlin / Android）。`app/src/main/kotlin/dev/ashiato/collector/` | `app/src/test`（JUnit4 + Robolectric）、`app/src/androidTest`（エミュレータ・実機の計測テスト） |
| `migrations` | D-01 PostgreSQL の移行。`YYYYMMDDHHMM_<slug>.sql` と `.down.sql` の対。足したら **`migrations/ORDER`（適用順の正本）の末尾へ 1 行** | `tools/check-migrations.sh`・`tools/check-immutable.sh` |
| `web/src` | V-01 画面（TypeScript + React + vite）。`App.tsx` が並び、画面の部品は `*View.tsx` / `*Panel.tsx` / `CoverageGrid.tsx`、API の型と文字列は `stays.ts` `coverage.ts` `attributes.ts` | `web/src/__tests__`（vitest + jsdom。指定と勘定まで） |
| `web/e2e` | 画面の e2e（playwright + 本物の Chromium）。`*.spec.ts` に `// Scenario: <名前>` の印。**画面の Scenario はここが担保する** | 走らせ方は `docs/testing.md` §4.5 |
| `tools` | プロジェクトの道具。縦串（`stack.sh` `smoke.sh` `dev.sh` `seed.sh`）・検査（`check-*.sh`）・Android（`android-env.sh` `android-emulator.sh`）・ハーネスの差し込み口（`agent-env.sh` `verify-prep.sh` `pre-commit.sh`） | — |

テストの規約と CI の job は `docs/testing.md`。

### 文書とハーネス

| パス | 何か | 書き込み |
|---|---|---|
| `docs/` | フローが作る成果物（`docs/changes/CH<NN>.md` は change の依頼と完了の判定。`hx change` が置く） | する |
| `openspec/` | Story ごとの設計と正典（`openspec` CLI が管理） | する |
| `.harness2` | harness2 を指す唯一のリンク（gitignore。`init.py` / `prepare.sh` が張る） | 作らない |
| `scripts` `.claude/skills` `.claude/agents` `.agents/skills` `openspec/schemas/story` `.githooks/*` | `.harness2/…` への symlink | harness2 側で直す |
| `AGENTS.md` | agent へのプロジェクト固有の指示（末尾で import） | する |

ハーネスの設計原則・責務分担・フロー・関門の正本は harness2（`.harness2/README.md`、`.harness2/docs/flow-gates.md`）。
ここでは複製しない。旧 ashiato の資料は `~/dev/ashiato/` にだけある。

## 進め方の要点

- **正式な実行経路は LangGraph のグラフ（`scripts/hx`）だけ。** Story を手動のセッションで進めない。
  操作: `scripts/hx dev` / `start ST<NN>` / `status` / `answer ST<NN> <file>` / `poke` / `retry ST<NN>` / `verify`
- **要件から来ない作業（作り直し・lint・道具・画面の表面・見つかった不具合）も change としてグラフに流す。**
  `scripts/hx change "<題>" --text "<依頼>"`（操作盤の「change を始める」）。`CH<NN>` が付き、Story と同じ流れを通る
- **`deep`（深掘り）で人間に問うのは、失う側がある問い（A）だけ。** 失わない側が高くつかず・範囲を越えず・要件を変えないなら、
  AI が仮で進む（P）。仮は台帳（`provisional.py`）に載り、別の席（deep-review）が確かめ、確認バッチで人間が見る。
  規則の正本は `.harness2/docs/flow-gates.md`。問い方の規範は `grilling` skill
- **機械が判定できることは機械に決めさせる。** 規約に書く・人間に聞くで代替しない
- 人間が止める場所は 3 つだけ: 深掘りの答え、merge、確認バッチ。人間は draft でない PR だけを merge する
- superpowers の規範より本プロジェクトの取り決めが優先する（採否と理由は `.harness2/CATALOG.md`）。
  要件は `grilling`、計画は OpenSpec、下流は `subagent-driven-development` の prompt をグラフが直接使う。並列はしない

## git

`main` / `develop` で commit しない（`.githooks/pre-commit` が止める。clone 後に `git config core.hooksPath .githooks`）。
Story は `feat/st<NN>-<slug>`、change は `feat/ch<NN>-<slug>` で PR にする（どちらもグラフが作る）。コミットは `feat:` / `fix:` / `docs:` / `chore:` + 日本語 1 行、本文に「なぜ」。

## agent への指示（プロジェクト固有）

@AGENTS.md
