// SPDX-License-Identifier: AGPL-3.0-only
//! 本人が置いた書庫を読む背景の仕事（ST12）。

pub mod chrome;
pub mod classify;
pub mod config;
pub mod legacy;
pub mod myactivity;
pub mod open;
pub mod scan;
pub mod slice;
pub mod timeline;
pub mod timezone;
pub mod worker;
pub mod youtube;

/// 書庫が位置を入れる論理ソース 7 本の並び（design D22）。**滞在の判定には使わない**。
/// 滞在の削除（`deletion::erase` の連鎖と `stay_store::mark_late_arrivals`）が基準のソースと合わせて読む。
pub const LOCATION_SOURCES: [&str; 7] = [
    "c03-timeline-visit",
    "c03-timeline-move",
    "c03-timeline-route",
    "c03-timeline-signal",
    "c03-legacy-location",
    "c03-legacy-visit",
    "c03-legacy-activity",
];

/// 読み方を変えたとき、同じ書庫を再び読むための版。
pub const PARSER_VERSION: &str = "1";
