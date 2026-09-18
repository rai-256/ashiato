// SPDX-License-Identifier: AGPL-3.0-only
//! 本人が置いた書庫を読む背景の仕事（ST12）。

pub mod classify;
pub mod config;
pub mod open;
pub mod scan;
pub mod slice;
pub mod timezone;
pub mod worker;

/// 読み方を変えたとき、同じ書庫を再び読むための版。
pub const PARSER_VERSION: &str = "1";
