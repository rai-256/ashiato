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

/// `LOCATION_SOURCES` のうち、終わり（`payload.end_time`）を持つ区間のソース 4 本（design D22）。
/// 削除の重なりの判定で、これだけを終わりで見る。他のソース（基準のソースと点のソース）は始まりの時刻
/// だけで判定でき、索引 `event_by_source_time` の上下限が効く（final review 第 2 回 R73）。
pub const INTERVAL_SOURCES: [&str; 4] = [
    "c03-timeline-visit",
    "c03-timeline-move",
    "c03-legacy-visit",
    "c03-legacy-activity",
];

/// 読み方を変えたとき、同じ書庫を再び読むための版。
pub const PARSER_VERSION: &str = "1";

#[cfg(test)]
mod tests {
    // Scenario: 消した滞在の時間帯に書庫から入る位置は削除済みになる
    //
    // 並びは spec `external-ingestion` の Requirement「本人が滞在を消した時間帯の書庫の位置は、
    // 削除済みの印を付けて入る」が名指しする 7 本と同じ文字列で固定する。試験の数え方も並びそのものを
    // 使うので、並びから外したソースは数からも消えて試験が通っていた（code-verify 第 4 回 R79）。
    #[test]
    fn location_sources_are_the_seven_named_by_the_spec() {
        assert_eq!(
            super::LOCATION_SOURCES.as_slice(),
            [
                "c03-timeline-visit",
                "c03-timeline-move",
                "c03-timeline-route",
                "c03-timeline-signal",
                "c03-legacy-location",
                "c03-legacy-visit",
                "c03-legacy-activity",
            ]
            .as_slice()
        );
        for source in super::INTERVAL_SOURCES {
            assert!(
                super::LOCATION_SOURCES.contains(&source),
                "区間のソース {source} が位置の並びに無い"
            );
        }
    }
}
