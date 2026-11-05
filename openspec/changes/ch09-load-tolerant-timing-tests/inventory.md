# Task 5 の Rust 候補一覧

| 場所（試験・補助） | 型 | 方針 | 理由 |
|---|---|---|---|
| `crates/server/src/scheduled_tests.rs` の `wait_for_calls`、4 件のログ待ち | 条件の待ち | 実時間 30 秒期限へ変更 | 停止した Tokio 時計の刻みは変えず、回数上限だけを実時間期限に置き換えた。 |
| `crates/server/src/outage_api_tests.rs:419` の台帳ロック待ち | 条件の待ち | 実時間 30 秒期限へ変更 | DB の待機数を観測できるため、最後の待機数を含む期限診断にした。 |
| `crates/collector-windows/src/runtime.rs` の `HistoryRuntime::drop` | 条件の待ち | 実時間 30 秒期限へ変更 | 履歴 worker の終了を観測でき、期限切れ時に最後の終了状態を出せる。 |
| `crates/collector-windows/src/runtime.rs` の `tick_until_history_idle` | 条件の待ち | 実時間 30 秒期限へ変更 | `is_reading` を観測できるため、最後に見た値を含む期限診断にした。 |
| `crates/collector-windows/src/runtime.rs` の `tick_until_history_beats` | 条件の待ち | 実時間 30 秒期限へ変更 | 生存信号の件数を観測できるため、最後の件数を含む期限診断にした。 |
| `crates/collector-windows/src/history/collect.rs:868`・`:969` の履歴取得待ち | 条件の待ち | 実時間 30 秒期限へ変更 | 取得結果と積み込み状態を観測できるため、最後に見た値を含む期限診断にした。 |
| `crates/collector-windows/src/runtime.rs` の `tick_until_read`・`tick_until_read_again` | 条件の待ち（時刻を兼ねる） | 直さない | 1 回ごとに試験時刻を進めるループであり、期限化すると時刻が進みすぎる（D4）。 |
| `crates/server/src/stay_tests.rs:2389` | 計算の形 | 直さない | 計算の形を実時間で見ており、製品コードを変えずに観測できる条件がない（D6）。 |
| `crates/server/src/testdb_tests.rs:371` `ch04_statement_timeout_cancels_a_long_query` | 打ち切り | `pg_sleep(60)`・30 秒未満へ変更 | 打ち切りの相手と上限を一緒に延ばした（D6）。 |
| `crates/collector-windows/src/time_sync.rs:650` `time_sync_timeout_kills_the_child` | 打ち切り | `sleep 60`・30 秒未満へ変更 | 打ち切りの相手と上限を一緒に延ばした（D6）。 |

## 探した範囲・探していない範囲

Task 5 が名指しした `crates/server` と `crates/collector-windows` の Rust 試験を走査した。画面・Android・Windows 実機試験・縦串台本は別 Task の担当であり、この一覧では扱っていない。

## 製品の不具合の疑い

無し。変更は `#[cfg(test)]` の試験コード内に限る。

## Task 8: Windows 実行時テスト

| 場所（試験・補助） | 型 | 方針 | 理由・確認状況 |
|---|---|---|---|
| `crates/collector-windows/tests/runtime_windows.rs:141` `HelperWindow::drop`、`:217` `focus_window_of`、`:412` 題名変化、`:957` `Edge::drop`、`:1223` `trigger`、`:1433` `HistoryBrowser::drop` | 固定の眠り | `// timing-ok:` を付けて維持 | 相手役の終了・窓の出現・題名変化・プロファイル解放・問い合わせ完了を観測する口がないため、D5 に従い余裕として残した。Windows runtime 15 本の実行で確認済み。 |
| `crates/collector-windows/tests/runtime_windows.rs:241` `run_for` | 決まった時間の窓 | `// timing-ok:` を付けて維持 | 指定秒数の観測窓そのものなので D3/D4 に従い延ばさない。Windows runtime 15 本の実行で確認済み。 |
| `crates/collector-windows/tests/runtime_windows.rs:225` `observe_until`、`:253` `run_until`、`:195` `focus_window_of` の期限、`:865` W32Time、`:1203` 通知履歴 | 条件の待ち | 実時間 30 秒期限へ変更 | 条件が来た時点で返る待ちを D4 に統一し、失敗時に待った時間を出す。Windows runtime 15 本の実行で確認済み。 |
| `crates/collector-windows/tests/runtime_windows.rs:1441` `fetch_history_until` | 条件の待ち | 実時間 90 秒期限を維持 | ブラウザ履歴の遅延と 25 時間刻みを含む実行時テスト固有の上限。Windows runtime 15 本の実行で確認済み。 |

Windows 側の検証は `cargo test -p ashiato-collector-windows --test runtime_windows -- --test-threads=1` を実行し、`test result: ok. 15 passed; 0 failed`（2026-11-05、evidence fingerprint `ae55e14e`）を得た。前回の 9/15 は対話デスクトップの前景窓取得に失敗した環境結果であり、今回の再実行で解消を確認した。
