// SPDX-License-Identifier: AGPL-3.0-only
//! C-02 Windows 収集（ST07）。**PC の上で起きていることを集める。**
//!
//! 送信の形・冪等キー・エンベロープ・生存信号の受け口は ST01 / ST02 / ST03 が
//! 決めており、**ここは新しい契約を作らない**（`docs/collector-contract.md` と
//! `crates/server/src/ingest.rs` が単一の情報源）。
//!
//! # 何をどこに置いたか
//!
//! **OS を触る部分（`platform`）と、記録を組み立てる部分（`engine`）を分ける。**
//! 前者は Windows の実機でしか動かないが、後者は「変化を 1 件にする」規則そのもので、
//! 題名の間引き（design D8）・離席の出入り（FR-81）・除外（FR-83）・
//! PC が止まっていた期間（FR-82）はすべて後者にある。分けないと、
//! **実機を出すまで 1 行も確かめられない**。
//!
//! # 取っていないものは後から作れない
//!
//! この Story の判断はほとんどが不可逆（`deep.md` の A が 7 問）。
//! 迷ったら**取る側・残す側**に倒す。
pub mod autostart;
pub mod browsers;
pub mod clock;
pub mod config;
pub mod contract;
pub mod engine;
pub mod exclusion;
pub mod heartbeat;
pub mod marker;
pub mod outbox;
#[cfg(windows)]
pub mod platform;
pub mod runtime;
pub mod sender;
pub mod telemetry;

/// 登録簿（`core.source`）にある論理ソース名。**ここにしか書かない。**
pub const LOGICAL_SOURCE: &str = "c02-window";

/// 生存信号の間隔。**登録簿の `c02-window.expected_gap_sec` と同じ値**
/// （`migrations/202609111111_coverage_rebuild.sql` の 21600 秒 = 6 時間）。
///
/// ずらすと、受け手が「想定間隔を超えて何も来ない」と判定する窓とずれ、
/// 正常な運用が⑥「途絶」に見える（FR-80）。**深掘り Q8 で本人が
/// 「6 時間のまま」を明示的に選んでいる**ので、ここを黙って変えない。
pub const EXPECTED_GAP_SEC: i64 = 21_600;
