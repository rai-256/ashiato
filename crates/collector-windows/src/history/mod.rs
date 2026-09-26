// SPDX-License-Identifier: AGPL-3.0-only
//! C-02 のブラウザ履歴収集。
//!
//! 置き場の発見、SQLite の写しの読取り、送る契約、帳面、取得の順に分ける。
//! 各モジュールは OS を触る境界を小さく保ち、履歴の規則を Linux の単体テストで
//! 固定できるようにする（ST08 design D1 / D2）。
pub mod contract;
pub mod fetch;
pub mod ledger;
pub mod locate;
pub mod read;
