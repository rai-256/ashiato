// SPDX-License-Identifier: AGPL-3.0-only
//! 収集の途切れの判定（ST14 / design D1・D3・D4・D8（仮））。**DB を見ない純粋な関数。**
//!
//! 材料は `outage_store::inputs` が SQL で集める。判定の結果を行に焼かない —— 想定間隔を変えたら
//! 判定もそのまま変わるように、問い合わせのたびに引く（ST02 の design D6 と同じ理由）。
use chrono::{DateTime, Duration, FixedOffset, NaiveDate, Utc};

/// 途切れの閾値の倍率。**最後に生きていた時刻から想定間隔の 3 倍を超えたら**途切れ（Q2）。
pub const ALERT_GAP_FACTOR: i64 = 3;

/// 最後に生きていた時刻に数えない記録。`(logical_source, payload->>'kind')` の組（design D4）。
///
/// 格子（`coverage::NON_OBSERVATION_GRID`）の 2 つ ＋ 離席・PC が止まっていた期間・除外の印。
/// どれも「収集が観測した」ではなく、動いていなくても残る記録（PC は時計の測定を 1 時間ごとに残す）。
/// 引き継ぎで名前が変わったら、同じ組を新しい名前にも足す（試験 `st14_inputs_nonobs_registry` が見る）。
pub const NON_OBSERVATION_ALERT: [(&str, &str); 5] = [
    ("c01-app-usage", "gap"),
    ("c02-window", "clock-skew"),
    ("c02-window", "idle"),
    ("c02-window", "powered-off"),
    ("c02-window", "excluded"),
];

/// 途切れの判定の材料 1 行（ソースの名前ごと。引き継ぎの鎖はたどらない。design D4）。
/// 真偽値は SQL が引く事実をそのまま持つ（D1）。状態機械にすると判定の分岐が材料の側へ漏れる。
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceInput {
    pub logical_source: String,
    pub display_name: String,
    pub expected_gap_sec: i32,
    /// 登録簿の「判定の対象か」の印（既定は対象）
    pub target: bool,
    /// 出どころが「収集した」の記録を 1 件でも持つか
    pub has_collected: bool,
    /// 出どころが「収集した」ではない（本人が書いた・導出した）記録を 1 件でも持つか
    pub has_non_collected: bool,
    pub retired_on: Option<NaiveDate>,
    /// 引き継ぎの鎖の根から継いだ収集開始日
    pub collection_started_on: Option<NaiveDate>,
    /// いまを覆う意図的な停止があるか
    pub stopped_now: bool,
    /// 最後に終わった意図的な停止の終わり
    pub last_stop_end: Option<DateTime<Utc>>,
    /// 観測の記録の最後の時刻（出来事の時刻。未来のものは受け取った時刻）
    pub last_record_at: Option<DateTime<Utc>>,
    /// 取得できる状態の生存信号の最後の時刻（未来のものは受け取った時刻）
    pub last_capturable_heartbeat_at: Option<DateTime<Utc>>,
    /// 引き継いだソースの、引き継いだ日（登録簿に行ができた時刻）。最後に生きていた時刻の下限
    pub alive_floor: Option<DateTime<Utc>>,
}

/// 途切れとして判定されたソース。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outage {
    pub logical_source: String,
    pub display_name: String,
    pub last_alive_at: DateTime<Utc>,
}

/// 日の区切りは `coverage::DAY_TZ`（Asia/Tokyo）。夏時間が無いので固定のずれで足りる。
fn day_of(now: DateTime<Utc>) -> NaiveDate {
    let tokyo = FixedOffset::east_opt(9 * 3600).expect("+09:00");
    now.with_timezone(&tokyo).date_naive()
}

/// 判定の対象か。**対象の判定（印・出どころ・退役・開始日・いまの停止）を 5 つ全部通ったもの。**
fn is_target(s: &SourceInput, now: DateTime<Utc>) -> bool {
    if !s.target {
        return false;
    }
    // 収集した記録が無く、本人が書いた・導出した記録があるときだけ外す（D3。1 件でも混ざれば外さない）
    if !s.has_collected && s.has_non_collected {
        return false;
    }
    // 退役した日そのものを含む
    if s.retired_on.is_some_and(|r| day_of(now) >= r) {
        return false;
    }
    if s.collection_started_on.is_none() {
        return false;
    }
    !s.stopped_now
}

/// 最後に生きていた時刻。観測の記録・取得できる状態の生存信号・最後に終わった停止の終わり・引き継いだ日のうち遅いもの。
/// 取得できない状態の生存信号は blockers を問わず入らない（D8（仮））。
fn last_alive(s: &SourceInput) -> Option<DateTime<Utc>> {
    [
        s.last_record_at,
        s.last_capturable_heartbeat_at,
        s.last_stop_end,
        s.alive_floor,
    ]
    .into_iter()
    .flatten()
    .max()
}

/// 途切れているソースを返す。**「超える」は厳密な `>`**（ちょうど 3 倍は途切れではない）。
pub fn judge(inputs: &[SourceInput], now: DateTime<Utc>) -> Vec<Outage> {
    inputs
        .iter()
        .filter(|s| is_target(s, now))
        .filter_map(|s| {
            let alive = last_alive(s)?;
            let limit = Duration::seconds(i64::from(s.expected_gap_sec) * ALERT_GAP_FACTOR);
            (now - alive > limit).then(|| Outage {
                logical_source: s.logical_source.clone(),
                display_name: s.display_name.clone(),
                last_alive_at: alive,
            })
        })
        .collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    const SIX_HOURS: i32 = 21_600;

    fn now() -> DateTime<Utc> {
        "2026-06-10T12:00:00Z".parse().unwrap()
    }

    fn ago(d: Duration) -> Option<DateTime<Utc>> {
        Some(now() - d)
    }

    /// 対象で、収集を始めていて、何も届いていない位置。
    fn base() -> SourceInput {
        SourceInput {
            logical_source: "c01-location".into(),
            display_name: "位置".into(),
            expected_gap_sec: SIX_HOURS,
            target: true,
            has_collected: true,
            has_non_collected: false,
            retired_on: None,
            collection_started_on: NaiveDate::from_ymd_opt(2026, 5, 1),
            stopped_now: false,
            last_stop_end: None,
            last_record_at: None,
            last_capturable_heartbeat_at: None,
            alive_floor: None,
        }
    }

    fn stale(extra: Duration) -> SourceInput {
        SourceInput {
            last_record_at: ago(Duration::hours(18) + extra),
            ..base()
        }
    }

    fn outage_of(s: SourceInput) -> bool {
        !judge(&[s], now()).is_empty()
    }

    #[test]
    fn st14_judge_constants_are_pinned() {
        assert_eq!(ALERT_GAP_FACTOR, 3);
        assert_eq!(NON_OBSERVATION_ALERT.len(), 5);
        for pair in [
            ("c01-app-usage", "gap"),
            ("c02-window", "clock-skew"),
            ("c02-window", "idle"),
            ("c02-window", "powered-off"),
            ("c02-window", "excluded"),
        ] {
            assert!(NON_OBSERVATION_ALERT.contains(&pair), "{pair:?}");
        }
    }

    // Scenario: 新しく登録したソースは判定の対象になる
    #[test]
    fn st14_judge_new_source_is_target() {
        // 印を指定しない登録の既定は「対象」。記録が届いた後に 3 倍を超えれば途切れ
        assert!(outage_of(stale(Duration::minutes(1))));
    }

    // Scenario: 対象外の印のソースは途切れにならない
    #[test]
    fn st14_judge_untargeted_mark_is_never_outage() {
        let s = SourceInput {
            target: false,
            ..stale(Duration::days(30))
        };
        assert!(!outage_of(s));
    }

    // Scenario: 本人が書いた記録のソースは印が無くても途切れにならない
    #[test]
    fn st14_judge_authored_only_source_is_never_outage() {
        let s = SourceInput {
            has_collected: false,
            has_non_collected: true,
            ..stale(Duration::days(30))
        };
        assert!(!outage_of(s));
    }

    // Scenario: 収集した記録と本人が書いた記録が混ざったソースは判定の対象のまま
    #[test]
    fn st14_judge_mixed_origin_source_stays_target() {
        let s = SourceInput {
            has_collected: true,
            has_non_collected: true,
            ..stale(Duration::minutes(1))
        };
        assert!(outage_of(s));
    }

    // Scenario: 導出した記録のソースは途切れにならない
    #[test]
    fn st14_judge_derived_only_source_is_never_outage() {
        // 導出も「収集した」ではない記録。出どころの分類は authored と同じ側
        let s = SourceInput {
            has_collected: false,
            has_non_collected: true,
            ..stale(Duration::days(2))
        };
        assert!(!outage_of(s));
    }

    // Scenario: 引き継いだソースは届く前から判定の対象になる
    #[test]
    fn st14_judge_successor_is_target_before_anything_arrives() {
        // 記録も生存信号も無く、引き継いだ日（下限）から 3 倍を超えた
        let late = SourceInput {
            has_collected: false,
            alive_floor: ago(Duration::hours(18) + Duration::minutes(1)),
            ..base()
        };
        let out = judge(std::slice::from_ref(&late), now());
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].last_alive_at, late.alive_floor.unwrap());
        // 名前を分けた直後は鳴らない
        let fresh = SourceInput {
            alive_floor: ago(Duration::hours(1)),
            ..late
        };
        assert!(!outage_of(fresh));
    }

    // Scenario: 退役した日からは途切れにならない
    #[test]
    fn st14_judge_retired_day_is_excluded() {
        let day = NaiveDate::from_ymd_opt(2026, 6, 10);
        // 退役した日そのもの（now は 2026-06-10 21:00 JST）
        assert!(!outage_of(SourceInput {
            retired_on: day,
            ..stale(Duration::days(1))
        }));
        // 前日に退役していても外れ、翌日に退役するならまだ判定される
        assert!(!outage_of(SourceInput {
            retired_on: NaiveDate::from_ymd_opt(2026, 6, 9),
            ..stale(Duration::days(1))
        }));
        assert!(outage_of(SourceInput {
            retired_on: NaiveDate::from_ymd_opt(2026, 6, 11),
            ..stale(Duration::days(1))
        }));
    }

    // Scenario: 収集を始めていないソースは途切れにならない
    #[test]
    fn st14_judge_not_started_source_is_never_outage() {
        assert!(!outage_of(SourceInput {
            collection_started_on: None,
            ..stale(Duration::days(30))
        }));
        // 開始日があっても、生きていた時刻の材料が何も無ければ判定できない
        assert!(!outage_of(base()));
    }

    // Scenario: 停止している間は途切れにならない
    #[test]
    fn st14_judge_stopped_now_is_never_outage() {
        assert!(!outage_of(SourceInput {
            stopped_now: true,
            ..stale(Duration::days(30))
        }));
    }

    // Scenario: 位置の記録が 18 時間を超えて来ないと途切れになる
    #[test]
    fn st14_judge_location_over_18h_is_outage() {
        let out = judge(&[stale(Duration::minutes(1))], now());
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].logical_source, "c01-location");
        assert_eq!(
            out[0].last_alive_at,
            now() - Duration::hours(18) - Duration::minutes(1)
        );
    }

    // Scenario: 18 時間ちょうどでは途切れにならない
    #[test]
    fn st14_judge_exactly_18h_is_not_outage() {
        assert!(!outage_of(stale(Duration::zero())));
        // 1 秒でも超えれば途切れ（厳密な >）
        assert!(outage_of(stale(Duration::seconds(1))));
    }

    // Scenario: 取得できる状態の生存信号があれば記録が無くても途切れにならない
    #[test]
    fn st14_judge_capturable_heartbeat_keeps_source_alive() {
        let s = SourceInput {
            logical_source: "c01-photo".into(),
            last_record_at: ago(Duration::days(10)),
            last_capturable_heartbeat_at: ago(Duration::hours(1)),
            ..base()
        };
        assert!(!outage_of(s));
    }

    // Scenario: 取れない状態の生存信号だけが届き続けると途切れになる
    #[test]
    fn st14_judge_uncapturable_heartbeats_do_not_keep_alive() {
        // 取れない状態の生存信号は材料に入らない（SourceInput は取れる側の最後だけを持つ）。
        // 最後の記録と取れる側の生存信号が 19 時間前なら、blockers が何であっても途切れ
        let s = SourceInput {
            last_record_at: ago(Duration::hours(19)),
            last_capturable_heartbeat_at: ago(Duration::hours(19)),
            ..base()
        };
        let out = judge(&[s], now());
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].last_alive_at, now() - Duration::hours(19));
    }

    // Scenario: 停止が終わった時刻から数え直す
    #[test]
    fn st14_judge_counts_again_from_stop_end() {
        let s = SourceInput {
            last_record_at: ago(Duration::days(3)),
            last_stop_end: ago(Duration::hours(2)),
            ..base()
        };
        assert!(!outage_of(s));
        // 停止が終わってから 3 倍を超えれば、また途切れ
        let later = SourceInput {
            last_record_at: ago(Duration::days(3)),
            last_stop_end: ago(Duration::hours(19)),
            ..base()
        };
        assert!(outage_of(later));
    }

    // Scenario: PC を閉じていた時間も経過に数える
    #[test]
    fn st14_judge_powered_off_time_counts_as_elapsed() {
        // 閉じていた期間の記録（powered-off）は材料に入らない。最後の記録と生存信号が 19 時間前なら途切れ
        let s = SourceInput {
            logical_source: "c02-window".into(),
            last_record_at: ago(Duration::hours(19)),
            last_capturable_heartbeat_at: ago(Duration::hours(19)),
            ..base()
        };
        assert!(outage_of(s));
    }

    // Scenario: 想定間隔を変えると判定も変わる
    #[test]
    fn st14_judge_changing_expected_gap_changes_verdict() {
        let two_days = SourceInput {
            expected_gap_sec: 86_400,
            last_record_at: ago(Duration::days(2)),
            ..base()
        };
        assert!(!outage_of(two_days.clone()));
        assert!(outage_of(SourceInput {
            expected_gap_sec: SIX_HOURS,
            ..two_days
        }));
    }

    #[test]
    fn st14_judge_picks_the_latest_alive_time() {
        let s = SourceInput {
            last_record_at: ago(Duration::hours(30)),
            last_capturable_heartbeat_at: ago(Duration::hours(25)),
            last_stop_end: ago(Duration::hours(40)),
            ..base()
        };
        assert_eq!(last_alive(&s), ago(Duration::hours(25)));
    }
}
