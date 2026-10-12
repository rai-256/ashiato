# 関門 —— ashiato2 固有の部分

**関門・レビューの席・指摘の R 形式・処置・`[x]`・検証の証跡・関門の 3 段は harness2 の正典**
`.harness2/docs/flow-gates.md` にある（2026-10-01 にこのファイルから移した）。ここには ashiato2 だけの検査と規則を置く。
テストの書き方は `docs/testing.md`。

## ashiato2 の検査（harness2 の「検査の一覧」に足すもの）

| スクリプト | 見るもの | 走る場所 |
|---|---|---|
| `tools/verify-prep.sh`（ashiato2） | server の release / web の build / APK（実機があれば adb で入れる）/ `run.sh` / `manifest.md` を `dist/verify-<tag>/` に | `verify_batch` |
| `cargo test -p ashiato-collector-windows --test runtime_windows`（ashiato2、Windows の上で） | 前景・入力・アドレスバーを本物の OS から読ませて記録を数える実行時テスト。テストが自分で窓を作る | CI の `collector-windows-runtime`（windows-latest）、手元の Windows |
| `tools/android-emulator.sh`（ashiato2） | エミュレータを立てて `src/androidTest` の計測テスト（前景サービス・権限の入口・HTTP を本物の framework で）。実機を繋いでも同じ gradle タスク | CI の `android-instrumented`（ubuntu + KVM）、手元 |
| `check-migrations.sh`（ashiato2） | 前進側の破壊的変更・戻し手順の欠落・**名前が作成時刻 `YYYYMMDDHHMM_<slug>.sql` でない**もの | ローカル、CI |
| `tools/check-private.sh` | 手元の網の名前・私設 IP がリポジトリに入っていない | `.githooks/pre-commit`（`tools/pre-commit.sh`）、CI の `chain` |
| `tools/check-log-private.sh` | ログに私的データを出せる**書き方**（メッセージへの値の埋め込み・許可外のフィールド・Debug 出力・println、携帯の `Telemetry.line` を通さない文字列、画面の console）。例外は `// log-ok: <理由>`（製造準備 A-2。一度出たログは消せない） | `.githooks/pre-commit`（`tools/pre-commit.sh`）、CI の `chain` |

## ashiato2 の規則

- 実機の OS を触る部分も機械で確かめる: Windows は `windows-latest` の実行時テスト（`crates/collector-windows/tests/runtime_windows.rs`）、
  Android はエミュレータの計測テスト（`tools/android-emulator.sh`。実機を繋いでも同じものが走る）
- 移行の名前は作成時刻（`YYYYMMDDHHMM_<slug>.sql`）。連番にしない（並走する Story が番号を取り合う）。`tools/check-migrations.sh` が形を見る

## 現在地（2026-09-09）

- ST01: issue #2 を残 11 件の入口として再開。`check_scenarios` は 30 Scenario / 印 0 で FAIL（印を付けるのは ST01 の残作業。HTTP 層の結合テストを `crates/server/tests/` へ移すのと一緒にやる）
- ST02: `review_triage` が FR-33 / NFR-13 の戻し漏れで FAIL（上流の残作業。deep-questions JSON は `/tmp` に書き捨てたため残っていない。次の回から commit する）
