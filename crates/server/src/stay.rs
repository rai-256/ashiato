// SPDX-License-Identifier: AGPL-3.0-only
//! 位置の列から滞在を切り出す（ST16 / FR-76。design D7）。**DB に触らない。**
//!
//! 作り直し（`stay_store`）も 1 日の並び（`GET /stays`）も、判定はここだけを通す ——
//! 判定の規則が 2 か所に割れると、片方だけ直したときに一覧と滞在の行が食い違う。
use chrono::{DateTime, Duration, SecondsFormat, Utc};

/// 滞在を置く論理ソース（design D1 / D2）。
pub const SOURCE: &str = "s01-stay";

/// 判定の基準の 1 版（design D10）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Criteria {
    /// 基準の台帳の行の識別子。**既定をまだ書いていない利用者では 0**
    pub id: i64,
    pub radius_m: i32,
    pub min_minutes: i32,
    pub gap_minutes: i32,
    pub sources: Vec<String>,
}

impl Criteria {
    /// 既定の基準（FR-76 の 100 m / 10 分、design D6 の 10 分、入力は端末の位置）。
    /// **既定の値はここ 1 か所に置く**（design D10）。
    pub fn default_values() -> Self {
        Self {
            id: 0,
            radius_m: 100,
            min_minutes: 10,
            gap_minutes: 10,
            sources: vec!["c01-location".into()],
        }
    }
}

/// 判定の入力にする位置の記録 1 件。
#[derive(Debug, Clone, PartialEq)]
pub struct Point {
    pub at: DateTime<Utc>,
    /// 緯度経度。**本文を消去した記録では無い**（FR-51 で `payload = '{}'`）
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    /// 水平精度（m）。欄が無い記録は精度が良いものとして扱う（spec / Q8）
    pub acc_m: Option<f64>,
    pub tz_offset_min: i32,
    pub tz_id: String,
}

/// 判定の途中でできる「集まり」。**最短のとどまりに満たないものも含む**
/// （作り直しの範囲を広げるときに、端をまたぐとどまりを見るため。design D5）。
#[derive(Debug, Clone, PartialEq)]
pub struct Cluster {
    /// 集まりに入った最初の位置の時刻
    pub start: DateTime<Utc>,
    /// 集まりに入った最後の位置の時刻
    pub end: DateTime<Utc>,
    /// 重心（丸めない）
    pub lat: f64,
    pub lon: f64,
    /// 判定に使った位置の件数
    pub points_used: i64,
    /// 最初の位置の時差と地域
    pub tz_offset_min: i32,
    pub tz_id: String,
}

/// 滞在 1 件。`Cluster` のうち最短のとどまりを満たしたもの。
pub type Stay = Cluster;

/// 2 点の距離（m）。正距円筒近似（design D7）—— 数百 m の判定には十分。
pub fn distance_m(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    const M_PER_DEG: f64 = 111_320.0;
    let mean_lat = ((lat1 + lat2) / 2.0).to_radians();
    let dy = (lat2 - lat1) * M_PER_DEG;
    let dx = (lon2 - lon1) * M_PER_DEG * mean_lat.cos();
    (dx * dx + dy * dy).sqrt()
}

/// 位置の列を集まりに分ける（design D7 の 1〜4）。**最短のとどまりでは絞らない。**
///
/// 1. 緯度経度を持たない記録（本文を消去した記録）は読み飛ばす —— 「記録が無い」の判定にも数えない
/// 2. 前の記録（精度を問わない）との間隔が `gap_minutes` **以上**なら、いまの集まりを閉じる（Q7）
/// 3. 精度が半径より悪い記録は、集まりに入れず、集まりも閉じない（Q8）。精度の欄が無い記録は使う
/// 4. 重心から半径以内なら加えて重心を動かす。外なら閉じて、この記録から新しい集まりを始める
pub fn clusters(points: &[Point], radius_m: i32, gap_minutes: i32) -> Vec<Cluster> {
    let mut sorted: Vec<&Point> = points
        .iter()
        .filter(|p| p.lat.is_some() && p.lon.is_some())
        .collect();
    // 呼び出し側は時刻順に渡すが、**順が崩れると滞在が黙って割れる**ので、ここでも揃える（安定な並べ替え）
    sorted.sort_by_key(|p| p.at);

    let radius = f64::from(radius_m);
    let gap = Duration::minutes(i64::from(gap_minutes));
    let mut out = Vec::new();
    let mut current: Option<Building> = None;
    let mut prev_at: Option<DateTime<Utc>> = None;

    for p in sorted {
        let (Some(lat), Some(lon)) = (p.lat, p.lon) else {
            continue;
        };
        if prev_at.is_some_and(|prev| p.at - prev >= gap) {
            out.extend(current.take().map(Building::close));
        }
        prev_at = Some(p.at);
        if p.acc_m.is_some_and(|acc| acc > radius) {
            continue;
        }
        match current.as_mut() {
            Some(c) if distance_m(c.lat(), c.lon(), lat, lon) <= radius => c.add(p.at, lat, lon),
            _ => {
                out.extend(current.take().map(Building::close));
                current = Some(Building::start(p, lat, lon));
            }
        }
    }
    out.extend(current.map(Building::close));
    out
}

/// 位置の列から滞在を切り出す（design D7）。集まりの長さ（最初の位置から最後の位置まで）が
/// 最短のとどまり**以上**のものを滞在にする。**長さに上限は置かない**（Q9）。
pub fn detect(points: &[Point], c: &Criteria) -> Vec<Stay> {
    let min = Duration::minutes(i64::from(c.min_minutes));
    clusters(points, c.radius_m, c.gap_minutes)
        .into_iter()
        .filter(|k| k.end - k.start >= min)
        .collect()
}

/// 作りかけの集まり。重心は和で持ち、閉じるときに割る。
struct Building {
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    sum_lat: f64,
    sum_lon: f64,
    n: i64,
    tz_offset_min: i32,
    tz_id: String,
}

impl Building {
    fn start(p: &Point, lat: f64, lon: f64) -> Self {
        Self {
            start: p.at,
            end: p.at,
            sum_lat: lat,
            sum_lon: lon,
            n: 1,
            tz_offset_min: p.tz_offset_min,
            tz_id: p.tz_id.clone(),
        }
    }

    fn lat(&self) -> f64 {
        self.sum_lat / self.n as f64
    }

    fn lon(&self) -> f64 {
        self.sum_lon / self.n as f64
    }

    fn add(&mut self, at: DateTime<Utc>, lat: f64, lon: f64) {
        self.end = at;
        self.sum_lat += lat;
        self.sum_lon += lon;
        self.n += 1;
    }

    fn close(self) -> Cluster {
        Cluster {
            start: self.start,
            end: self.end,
            lat: self.lat(),
            lon: self.lon(),
            points_used: self.n,
            tz_offset_min: self.tz_offset_min,
            tz_id: self.tz_id,
        }
    }
}

/// 滞在の原文（design D1）。**キーの順と数値の表記を固定する** —— 同じ滞在は毎回同じ文字列になり、
/// 作り直しの「内容が同じなら更新しない」（D3）が効く。
///
/// 緯度経度は小数 6 桁（約 0.1 m）に丸める。丸めないと浮動小数の誤差で同じ滞在が別の文字列になり、
/// 作り直しのたびに前の版が積まれる。時刻は秒の端数があるときだけ端数を書く（`AutoSi`）。
pub fn raw(stay: &Stay, c: &Criteria) -> String {
    let ts = |t: DateTime<Utc>| {
        serde_json::Value::String(t.to_rfc3339_opts(SecondsFormat::AutoSi, true)).to_string()
    };
    let sources = serde_json::Value::from(c.sources.clone()).to_string();
    format!(
        r#"{{"start":{},"end":{},"lat":{},"lon":{},"points_used":{},"criteria":{{"id":{},"radius_m":{},"min_minutes":{},"gap_minutes":{},"sources":{}}}}}"#,
        ts(stay.start),
        ts(stay.end),
        coord(stay.lat),
        coord(stay.lon),
        stay.points_used,
        c.id,
        c.radius_m,
        c.min_minutes,
        c.gap_minutes,
        sources,
    )
}

/// 緯度経度を 6 桁に丸めて JSON の数値として書く。**有限でない値は `null`**（位置の記録からは来ない）。
fn coord(x: f64) -> String {
    serde_json::Number::from_f64((x * 1e6).round() / 1e6)
        .map_or_else(|| "null".into(), |n| n.to_string())
}

/// テストの位置を組む足場。`stay_tests` も使う。
#[cfg(test)]
pub(crate) mod fixture {
    #![allow(clippy::unwrap_used)]
    use super::*;

    /// 基準点（東京駅のあたり）。
    pub const LAT: f64 = 35.681_2;
    pub const LON: f64 = 139.767_1;

    /// 基準点から北へ `north_m`、東へ `east_m` ずらした位置。
    pub fn offset(north_m: f64, east_m: f64) -> (f64, f64) {
        let lat = LAT + north_m / 111_320.0;
        let lon = LON + east_m / (111_320.0 * LAT.to_radians().cos());
        (lat, lon)
    }

    /// `at`（RFC3339）から 60 秒ごとに `minutes + 1` 件、同じ場所の位置。
    pub fn dwell(at: &str, minutes: i64, north_m: f64, east_m: f64) -> Vec<Point> {
        let t0: DateTime<Utc> = at.parse().unwrap();
        let (lat, lon) = offset(north_m, east_m);
        (0..=minutes)
            .map(|i| Point {
                at: t0 + Duration::minutes(i),
                lat: Some(lat),
                lon: Some(lon),
                acc_m: Some(10.0),
                tz_offset_min: 540,
                tz_id: "Asia/Tokyo".into(),
            })
            .collect()
    }

    pub fn t(s: &str) -> DateTime<Utc> {
        s.parse().unwrap()
    }
}

/// 半径・最短のとどまり・重心・消去された記録（tasks 2.1）。
#[cfg(test)]
mod detect {
    #![allow(clippy::unwrap_used)]
    use super::fixture::*;
    use super::*;

    // Scenario: 半径の中に最短のとどまり以上いると滞在が 1 件できる
    #[test]
    fn one_stay_within_radius() {
        // 互いに 30 m 以内の 21 件（20 分ぶん）。揺らしても 1 件のまま
        let mut pts = dwell("2026-09-01T00:00:00Z", 20, 0.0, 0.0);
        for (i, p) in pts.iter_mut().enumerate() {
            let (lat, lon) = offset((i % 3) as f64 * 10.0, (i % 2) as f64 * 10.0);
            p.lat = Some(lat);
            p.lon = Some(lon);
        }
        assert_eq!(pts.len(), 21);
        let got = super::detect(&pts, &Criteria::default_values());
        assert_eq!(got.len(), 1);
        assert_eq!(
            got[0].start,
            t("2026-09-01T00:00:00Z"),
            "始まりは最初の位置"
        );
        assert_eq!(got[0].end, t("2026-09-01T00:20:00Z"), "終わりは最後の位置");
        assert_eq!(got[0].points_used, 21);
    }

    // Scenario: 最短のとどまりより短い立ち寄りは滞在にならない
    #[test]
    fn short_visit_is_not_a_stay() {
        let mut pts = dwell("2026-09-01T00:00:00Z", 6, 0.0, 0.0);
        // 1 km 離れた地点へ移る（そこにも 6 分だけ）
        pts.extend(dwell("2026-09-01T00:07:00Z", 6, 1_000.0, 0.0));
        assert!(super::detect(&pts, &Criteria::default_values()).is_empty());
    }

    // Scenario: 半径を出ると滞在が閉じる
    #[test]
    fn leaving_the_radius_closes_the_stay() {
        let mut pts = dwell("2026-09-01T00:00:00Z", 30, 0.0, 0.0);
        pts.extend(dwell("2026-09-01T00:31:00Z", 30, 500.0, 0.0));
        let got = super::detect(&pts, &Criteria::default_values());
        assert_eq!(got.len(), 2);
        assert_eq!(
            got[0].end,
            t("2026-09-01T00:30:00Z"),
            "地点 A を出る前の最後の位置で終わる"
        );
        assert_eq!(got[1].start, t("2026-09-01T00:31:00Z"));
    }

    // Scenario: 長い滞在は区切られない
    #[test]
    fn long_stay_is_not_split() {
        let pts = dwell("2026-09-01T00:00:00Z", 15 * 60, 0.0, 0.0);
        let got = super::detect(&pts, &Criteria::default_values());
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].end - got[0].start, Duration::hours(15));
    }

    // Scenario: 日付をまたぐ滞在は 1 件のまま
    #[test]
    fn stay_across_midnight_is_one() {
        // Asia/Tokyo の 23:00 から翌日の 02:00
        let pts = dwell("2026-09-01T14:00:00Z", 3 * 60, 0.0, 0.0);
        let got = super::detect(&pts, &Criteria::default_values());
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].start, t("2026-09-01T23:00:00+09:00"));
        assert_eq!(got[0].end, t("2026-09-02T02:00:00+09:00"));
    }

    // Scenario: 本文を消去した位置の記録は判定に数えない
    #[test]
    fn erased_points_are_not_counted() {
        let mut pts = dwell("2026-09-01T00:00:00Z", 30, 0.0, 0.0);
        pts[15].lat = None;
        pts[15].lon = None;
        pts[15].acc_m = None;
        let got = super::detect(&pts, &Criteria::default_values());
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].points_used, 30, "消去された 1 件を数えている");
    }

    /// 10 分ちょうどのとどまり（11 件）は滞在になる（「以上」。R62）。
    #[test]
    fn exactly_min_minutes_is_a_stay() {
        let pts = dwell("2026-09-01T00:00:00Z", 10, 0.0, 0.0);
        assert_eq!(super::detect(&pts, &Criteria::default_values()).len(), 1);
        let pts = dwell("2026-09-01T00:00:00Z", 9, 0.0, 0.0);
        assert!(super::detect(&pts, &Criteria::default_values()).is_empty());
    }

    /// 経度の方向の距離も緯度を見て測る（R59）。東へ 90 m は半径 100 m の中。
    #[test]
    fn east_west_distance_uses_latitude() {
        let mut pts = dwell("2026-09-01T00:00:00Z", 30, 0.0, 0.0);
        pts.extend(dwell("2026-09-01T00:31:00Z", 30, 0.0, 90.0));
        assert_eq!(
            super::detect(&pts, &Criteria::default_values()).len(),
            1,
            "東西の距離を長く測っている"
        );
        let d = distance_m(LAT, LON, LAT, offset(0.0, 90.0).1);
        assert!((d - 90.0).abs() < 0.5, "{d}");
    }

    /// 中心は重心で、集まりとともに動く（最初の点を中心にすると 130 m 先で割れる。R67）。
    #[test]
    fn centroid_moves_with_the_cluster() {
        let mut pts = dwell("2026-09-01T00:00:00Z", 29, 0.0, 0.0);
        pts.extend(dwell("2026-09-01T00:30:00Z", 29, 90.0, 0.0));
        pts.extend(dwell("2026-09-01T01:00:00Z", 29, 130.0, 0.0));
        assert_eq!(super::detect(&pts, &Criteria::default_values()).len(), 1);
    }

    /// 重心が動くことの確認（半径 50 m では 70 m 先が別の滞在になる。spec の入力）。
    #[test]
    fn radius_changes_the_split() {
        let mut pts = dwell("2026-09-01T00:00:00Z", 30, 0.0, 0.0);
        pts.extend(dwell("2026-09-01T00:31:00Z", 29, 70.0, 0.0));
        let mut c = Criteria::default_values();
        assert_eq!(super::detect(&pts, &c).len(), 1);
        c.radius_m = 50;
        assert_eq!(super::detect(&pts, &c).len(), 2);
    }
}

/// 記録が無い区間で切る（tasks 2.2 / Q7）。
#[cfg(test)]
mod gap {
    #![allow(clippy::unwrap_used)]
    use super::fixture::*;
    use super::*;

    // Scenario: 記録が欠けた区間の前後は別々の滞在になる
    #[test]
    fn gap_splits_the_stay() {
        let mut pts = dwell("2026-09-01T08:00:00Z", 30, 0.0, 0.0);
        pts.extend(dwell("2026-09-01T16:30:00Z", 30, 0.0, 0.0));
        let got = super::detect(&pts, &Criteria::default_values());
        assert_eq!(got.len(), 2, "同じ場所でも欠けをまたいで 1 件にしている");
        assert!(got[0].end <= t("2026-09-01T08:30:00Z"));
        assert_eq!(got[1].start, t("2026-09-01T16:30:00Z"));
    }

    /// 間隔がちょうど `gap_minutes` なら切る（「以上」。design D6）。
    #[test]
    fn gap_is_inclusive() {
        let mut pts = dwell("2026-09-01T08:00:00Z", 15, 0.0, 0.0);
        pts.extend(dwell("2026-09-01T08:25:00Z", 15, 0.0, 0.0));
        assert_eq!(super::detect(&pts, &Criteria::default_values()).len(), 2);
        let mut pts = dwell("2026-09-01T08:00:00Z", 15, 0.0, 0.0);
        pts.extend(dwell("2026-09-01T08:24:59Z", 15, 0.0, 0.0));
        assert_eq!(super::detect(&pts, &Criteria::default_values()).len(), 1);
    }

    // Scenario: 精度の悪い点しか無い区間は記録なしにならない
    #[test]
    fn inaccurate_points_keep_the_gap_closed() {
        let mut pts = dwell("2026-09-01T08:00:00Z", 30, 0.0, 0.0);
        for p in pts.iter_mut().skip(8).take(15) {
            p.acc_m = Some(150.0);
        }
        let got = super::detect(&pts, &Criteria::default_values());
        assert_eq!(got.len(), 1, "精度の悪い 15 分で滞在が切られている");
        assert_eq!(got[0].end, t("2026-09-01T08:30:00Z"));
    }
}

/// 精度が半径より悪い記録を判定に使わない（tasks 2.3 / Q8）。
#[cfg(test)]
mod accuracy {
    #![allow(clippy::unwrap_used)]
    use super::fixture::*;
    use super::*;

    // Scenario: 精度の悪い 1 点が混ざっても滞在は割れない
    #[test]
    fn one_inaccurate_point_does_not_split() {
        let mut pts = dwell("2026-09-01T08:00:00Z", 40, 0.0, 0.0);
        let (lat, lon) = offset(400.0, 0.0);
        pts[20].lat = Some(lat);
        pts[20].lon = Some(lon);
        pts[20].acc_m = Some(150.0);
        let got = super::detect(&pts, &Criteria::default_values());
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].points_used, 40, "精度の悪い点を判定に使っている");
    }

    // Scenario: 精度を持たない位置の記録は判定に使われる
    #[test]
    fn points_without_accuracy_are_used() {
        let mut pts = dwell("2026-09-01T08:00:00Z", 20, 0.0, 0.0);
        for p in &mut pts {
            p.acc_m = None;
        }
        let got = super::detect(&pts, &Criteria::default_values());
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].points_used, 21);
    }

    /// 精度がちょうど半径なら使う（「半径より悪い」だけを外す）。
    #[test]
    fn accuracy_equal_to_radius_is_used() {
        let mut pts = dwell("2026-09-01T08:00:00Z", 20, 0.0, 0.0);
        for p in &mut pts {
            p.acc_m = Some(100.0);
        }
        assert_eq!(
            super::detect(&pts, &Criteria::default_values())[0].points_used,
            21
        );
    }
}

/// `raw` の直列化を固定する（tasks 2.4 / design D1）。
#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::fixture::*;
    use super::*;

    #[test]
    fn stay_raw_is_pinned() {
        let stay = Stay {
            start: t("2026-09-07T00:00:00Z"),
            end: t("2026-09-07T01:30:00.5Z"),
            lat: 35.681_234_49,
            lon: 139.767_125_51,
            points_used: 38,
            tz_offset_min: 540,
            tz_id: "Asia/Tokyo".into(),
        };
        let c = Criteria {
            id: 7,
            ..Criteria::default_values()
        };
        let a = raw(&stay, &c);
        let b = raw(&stay.clone(), &c.clone());
        assert_eq!(a.as_bytes(), b.as_bytes(), "同じ入力から違う原文ができる");
        // **期待する文字列そのもの**。キーの順・緯度経度 6 桁・時刻の表記のどれが動いても落ちる ——
        // 動くと保存済みの滞在が作り直しのたびに「内容が変わった」と判定され、前の版が積まれる
        assert_eq!(
            a,
            r#"{"start":"2026-09-07T00:00:00Z","end":"2026-09-07T01:30:00.500Z","lat":35.681234,"lon":139.767126,"points_used":38,"criteria":{"id":7,"radius_m":100,"min_minutes":10,"gap_minutes":10,"sources":["c01-location"]}}"#
        );
        // 原文は JSON として読める（`payload` はこれを jsonb にしたもの）
        let v: serde_json::Value = serde_json::from_str(&a).unwrap();
        assert_eq!(v["criteria"]["radius_m"], 100);
    }
}
