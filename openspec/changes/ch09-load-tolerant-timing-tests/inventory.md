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
