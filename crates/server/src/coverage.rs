// SPDX-License-Identifier: AGPL-3.0-only
//! 稼働状況の導出（FR-54 / FR-79 / FR-80 / NFR-13）。
//!
//! **状態は行に焼かない。引くたびに導出する**（design D6）—— 途絶を書くバッチは
//! 検知する側が同じ理由（サーバの停止）で止まりうるので、止まっている間の途絶を
//! 止まっている当人が書けるはずがない。想定間隔を後から変えれば過去の判定も変わる。
use serde::Serialize;

/// 日の区切り。**本人が決めた**（深掘り Q2）。記録ごとのタイムゾーンで切らない ——
/// 東西の移動で 1 年が 364 日にも 366 日にもなり、NFR-13 の分母がぶれる。
///
/// **SQL のリテラルとして埋める**（design D1）。`AT TIME ZONE` は列名の位置なので
/// プレースホルダを取れない。値はここ 1 か所だけに置く。
pub const DAY_TZ: &str = "Asia/Tokyo";

/// NFR-13 の窓の長さ。**そのソースの収集開始日から 365 日**（第 6 回 Q23）。
pub const WINDOW_DAYS: i64 = 365;

/// 達成の線。**分母の割合**（第 5 回 Q18）。絶対値の 350 日ではない ——
/// 分母から日を除くほど達成が遠のき、導入 1 年未満では原理的に到達できなかった。
pub const ACHIEVE_RATIO: f64 = 0.95;

/// NFR-13 が名指しする Must の 5 ソースと、その達成日の数え方（design D11）。
///
/// **DB の列にしない。** 行を 1 つ更新するだけで判定式が動いてしまう。
/// 定数なら変更が diff に出て、CI とレビューを通る。
/// 論理ソース名は登録簿（`migrations/0005`）の名前と同じでなければならない。
pub const DEVICE_SUBJECT: [&str; 2] = ["c01-location", "c01-app-usage"];
/// 同上。**利用が主語**は「取得できる状態の生存信号があった日」で数える（第 4 回 Q7 / Q8）。
pub const USAGE_SUBJECT: [&str; 3] = ["c01-photo", "c02-window", "c02-browser-history"];

/// 達成日の数え方。ソースの**主語**で分かれる（第 4 回 Q8）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Subject {
    /// 端末が主語（位置・アプリ利用）。**記録が 1 件以上ある日**
    Device,
    /// 利用が主語（写真・ウィンドウ・ブラウザ履歴）。**取得できる状態の生存信号があった日**
    Usage,
}

/// NFR-13 の 5 ソースを主語つきで並べる。**この順が画面にも出る。**
pub fn must_sources() -> Vec<(String, Subject)> {
    DEVICE_SUBJECT
        .iter()
        .map(|s| ((*s).to_string(), Subject::Device))
        .chain(
            USAGE_SUBJECT
                .iter()
                .map(|s| ((*s).to_string(), Subject::Usage)),
        )
        .collect()
}

/// ソース × 日 の状態（FR-54）。**7 つのいずれか 1 つに決まる。**
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DayState {
    /// ① 記録あり
    Recorded,
    /// ② 動いていた・記録なし
    AliveNoRecord,
    /// ③ 動いていたが取れない状態だった
    AliveNotCapturable,
    /// ④ 意図的な停止
    Stopped,
    /// ⑤ 破棄された期間
    Dropped,
    /// ⑥ 途絶
    Outage,
    /// ⑦ 導入前
    BeforeStart,
}

impl DayState {
    /// 格子のセルが担う 3 段（design D10 / 第 5 回 Q21）。
    /// **7 段は成り立たない** —— 隣接 3:1 を 6 区間積むと 3^6 = 729:1 が要り、sRGB は 21:1 が上限。
    pub fn band(self) -> Band {
        match self {
            Self::Recorded => Band::Recorded,
            Self::AliveNoRecord => Band::AliveNoRecord,
            _ => Band::Other,
        }
    }
}

/// 格子の 3 段。**7 状態の区別は週を選んだときの文字が担う**（第 5 回 Q20 / Q21）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Band {
    Recorded,
    AliveNoRecord,
    Other,
}

/// 1 日ぶんの状態と、その日の材料。
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct DayCell {
    pub day: chrono::NaiveDate,
    pub state: DayState,
    pub event_count: i32,
    /// その日の生存信号が報告した取得の試行回数。**信号が無い日は返らない**
    /// —— 来ていない区間の取得率は埋まらない（specs）
    pub attempts: Option<i64>,
    pub successes: Option<i64>,
}

/// ソース 1 本ぶんの稼働状況。
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct SourceCoverage {
    pub logical_source: String,
    pub display_name: String,
    pub expected_gap_sec: i32,
    pub collection_started_on: Option<chrono::NaiveDate>,
    pub days: Vec<DayCell>,
}

/// 登録簿の 1 行（導出に要る分だけ）。
#[derive(Debug, Clone)]
pub struct SourceRow {
    pub logical_source: String,
    pub display_name: String,
    pub expected_gap_sec: i32,
    pub collection_started_on: Option<chrono::NaiveDate>,
}

/// `facts` が SQL から受け取る 1 行。
/// （日, 件数, 取得可否, 試行, 成功, 丸ごと破棄, 丸ごと停止）
type FactRow = (
    chrono::NaiveDate,
    i32,
    Option<bool>,
    Option<i64>,
    Option<i64>,
    bool,
    bool,
);

/// その日の生の材料。状態を決める前の事実だけを持つ。
#[derive(Debug, Clone)]
struct DayFacts {
    day: chrono::NaiveDate,
    event_count: i32,
    /// その日に生存信号があったか / そのうち 1 件でも取得できる状態だったか
    capturable: Option<bool>,
    attempts: Option<i64>,
    successes: Option<i64>,
    /// その日を**丸ごと覆う**破棄 / 停止があるか。
    /// **丸ごと覆わない範囲は状態を決めない**（design D7 / 深掘り Q3 と同じ粒度）
    dropped_full: bool,
    stopped_full: bool,
}

/// 収集開始日を**受け口の側で**埋める（design D5 / 第 6 回 Q24 / 第 7 回 Q26）。
///
/// **記録が作られた時刻の日**を当てる。受信時刻ではない —— 圏外で 3 日ぶん溜めて送ると、
/// 記録のある日が⑦「導入前」になり NFR-13 の分母からも落ちる。
///
/// **`least()` を取るので、古い記録が後から届くたびに前へ動く**（第 7 回 Q26。
/// 圏外の保持がこれを起こす）。遡ると NFR-13 の窓の起点も動くが、
/// **動くのは入力であって判定式ではない** —— 状態も達成も行に焼いていないので、
/// 同じ入力からは必ず同じ答えが出る。
pub async fn touch_started_on(
    pool: &sqlx::PgPool,
    logical_source: &str,
    at: chrono::DateTime<chrono::Utc>,
) -> Result<(), sqlx::Error> {
    let sql = format!(
        "UPDATE core.source
            SET collection_started_on = ($2 AT TIME ZONE '{tz}')::date
          WHERE logical_source = $1
            AND (collection_started_on IS NULL
                 OR collection_started_on > ($2 AT TIME ZONE '{tz}')::date)",
        tz = DAY_TZ
    );
    sqlx::query(&sql)
        .bind(logical_source)
        .bind(at)
        .execute(pool)
        .await?;
    Ok(())
}

/// 登録簿を引く。`only` が空でなければその論理ソースだけ。
pub async fn sources(pool: &sqlx::PgPool, only: &[String]) -> Result<Vec<SourceRow>, sqlx::Error> {
    let rows: Vec<(String, String, i32, Option<chrono::NaiveDate>)> = sqlx::query_as(
        "SELECT logical_source, display_name, expected_gap_sec, collection_started_on
           FROM core.source
          WHERE cardinality($1::text[]) = 0 OR logical_source = ANY($1)
          ORDER BY logical_source",
    )
    .bind(only)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(
            |(logical_source, display_name, expected_gap_sec, collection_started_on)| SourceRow {
                logical_source,
                display_name,
                expected_gap_sec,
                collection_started_on,
            },
        )
        .collect())
}

/// 日ごとの材料を SQL で 1 度に集める。
///
/// **日の区切りは PostgreSQL の `AT TIME ZONE` に任せる**（design D1）——
/// 日を引く場所がアプリと SQL に割れると、片方だけがずれても誰も気付かない。
async fn facts(
    pool: &sqlx::PgPool,
    user: Option<uuid::Uuid>,
    source: &str,
    from: chrono::NaiveDate,
    to: chrono::NaiveDate,
) -> Result<Vec<DayFacts>, sqlx::Error> {
    let sql = format!(
        "WITH d AS (SELECT generate_series($3::date, $4::date, '1 day')::date AS day),
              c AS (SELECT day, event_count FROM core.coverage
                     WHERE ($1::uuid IS NULL OR user_id = $1) AND logical_source = $2),
              h AS (SELECT (emitted_at AT TIME ZONE '{tz}')::date AS day,
                           bool_or(capturable) AS capturable,
                           sum(attempts)::bigint  AS attempts,
                           sum(successes)::bigint AS successes
                      FROM core.heartbeat
                     WHERE ($1::uuid IS NULL OR user_id = $1) AND logical_source = $2
                     GROUP BY 1)
         SELECT d.day,
                coalesce(c.event_count, 0) AS event_count,
                h.capturable, h.attempts, h.successes,
                EXISTS (SELECT 1 FROM core.coverage_span s
                         WHERE ($1::uuid IS NULL OR s.user_id = $1) AND s.logical_source = $2 AND s.kind = 'dropped'
                           AND s.started_at <= (d.day::timestamp AT TIME ZONE '{tz}')
                           AND (s.ended_at IS NULL
                                OR s.ended_at >= ((d.day + 1)::timestamp AT TIME ZONE '{tz}')))
                  AS dropped_full,
                EXISTS (SELECT 1 FROM core.coverage_span s
                         WHERE ($1::uuid IS NULL OR s.user_id = $1) AND s.logical_source = $2 AND s.kind = 'stopped'
                           AND s.started_at <= (d.day::timestamp AT TIME ZONE '{tz}')
                           AND (s.ended_at IS NULL
                                OR s.ended_at >= ((d.day + 1)::timestamp AT TIME ZONE '{tz}')))
                  AS stopped_full
           FROM d
           LEFT JOIN c ON c.day = d.day
           LEFT JOIN h ON h.day = d.day
          ORDER BY d.day",
        tz = DAY_TZ
    );
    let rows: Vec<FactRow> = sqlx::query_as(&sql)
        .bind(user)
        .bind(source)
        .bind(from)
        .bind(to)
        .fetch_all(pool)
        .await?;
    Ok(rows
        .into_iter()
        .map(
            |(day, event_count, capturable, attempts, successes, dropped_full, stopped_full)| {
                DayFacts {
                    day,
                    event_count,
                    capturable,
                    attempts,
                    successes,
                    dropped_full,
                    stopped_full,
                }
            },
        )
        .collect())
}

/// 記録か生存信号があった日を古い順に。**途絶の判定に要る**（前後の活動を測る）。
async fn active_days(
    pool: &sqlx::PgPool,
    user: Option<uuid::Uuid>,
    source: &str,
) -> Result<Vec<chrono::NaiveDate>, sqlx::Error> {
    let sql = format!(
        "SELECT day FROM (
           SELECT day FROM core.coverage
            WHERE ($1::uuid IS NULL OR user_id = $1) AND logical_source = $2 AND event_count > 0
           UNION
           SELECT (emitted_at AT TIME ZONE '{tz}')::date FROM core.heartbeat
            WHERE ($1::uuid IS NULL OR user_id = $1) AND logical_source = $2
         ) t ORDER BY day",
        tz = DAY_TZ
    );
    let rows: Vec<(chrono::NaiveDate,)> = sqlx::query_as(&sql)
        .bind(user)
        .bind(source)
        .fetch_all(pool)
        .await?;
    Ok(rows.into_iter().map(|(d,)| d).collect())
}

/// 7 状態を決める（design D7）。**上から評価して最初に当たったものを返す。**
///
/// 順序は specs の Requirement 本文に列挙してある —— **本人が決めた**（第 5 回 Q19）。
/// 「なぜこの日はデータが少ないのか」に画面が先に答えるので、
/// 1 日を丸ごと止めた日に記録が一部残っていても、その日は④として見える。
fn decide(
    facts: &DayFacts,
    started_on: Option<chrono::NaiveDate>,
    gap_days: f64,
    active: &[chrono::NaiveDate],
) -> DayState {
    match started_on {
        None => return DayState::BeforeStart,
        Some(s) if facts.day < s => return DayState::BeforeStart,
        _ => {}
    }
    if facts.dropped_full {
        return DayState::Dropped;
    }
    if facts.stopped_full {
        return DayState::Stopped;
    }
    if facts.event_count > 0 {
        return DayState::Recorded;
    }
    match facts.capturable {
        Some(true) => return DayState::AliveNoRecord,
        Some(false) => return DayState::AliveNotCapturable,
        None => {}
    }
    // 記録も生存信号も無い日。**想定間隔以内に前か後の活動があれば②、無ければ⑥**
    // （design D17）。想定間隔を条件に持たせないと、ブラウザ履歴（24 時間）や
    // Takeout 系（60 日）の**正常な空白日がすべて途絶になる**（review/spec.md の R4）。
    let near = |other: &chrono::NaiveDate| {
        let diff = (*other - facts.day).num_days().abs() as f64;
        diff <= gap_days
    };
    if active.iter().any(near) {
        DayState::AliveNoRecord
    } else {
        DayState::Outage
    }
}

/// ソース 1 本の稼働状況を引く。
pub async fn of_source(
    pool: &sqlx::PgPool,
    user: Option<uuid::Uuid>,
    src: &SourceRow,
    from: chrono::NaiveDate,
    to: chrono::NaiveDate,
) -> Result<SourceCoverage, sqlx::Error> {
    let facts = facts(pool, user, &src.logical_source, from, to).await?;
    let active = active_days(pool, user, &src.logical_source).await?;
    let gap_days = f64::from(src.expected_gap_sec) / 86_400.0;
    let days = facts
        .iter()
        .map(|f| DayCell {
            day: f.day,
            state: decide(f, src.collection_started_on, gap_days, &active),
            event_count: f.event_count,
            attempts: f.attempts,
            successes: f.successes,
        })
        .collect();
    Ok(SourceCoverage {
        logical_source: src.logical_source.clone(),
        display_name: src.display_name.clone(),
        expected_gap_sec: src.expected_gap_sec,
        collection_started_on: src.collection_started_on,
        days,
    })
}

// ---------------------------------------------------------------- 達成日数と合否

/// ソース 1 本ぶんの達成（NFR-13）。
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct SourceAchievement {
    pub logical_source: String,
    pub display_name: String,
    pub subject: Subject,
    pub collection_started_on: Option<chrono::NaiveDate>,
    /// 達成日数（分子）
    pub achieved_days: i64,
    /// 分母。**1 日を丸ごと覆う停止は抜く**（深掘り Q3）
    pub denominator: i64,
    /// 線 = 分母 × 95 %
    pub threshold: f64,
    /// このソースが線に届いているか。**まだ開始していないソースは届いていない扱い**
    pub met: bool,
    /// 窓（収集開始日から 365 日）が閉じたか
    pub window_closed: bool,
    /// 窓が閉じる日。開始していなければ返らない
    pub window_closes_on: Option<chrono::NaiveDate>,
}

/// 5 ソースの達成と、成功条件 1 の合否（NFR-13）。
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct Achievement {
    pub sources: Vec<SourceAchievement>,
    /// **5 本すべてが分母の 95 % 以上**か（第 4 回 Q9 / 第 5 回 Q18）
    pub verdict: bool,
    /// 落ちたソース。どれが落ちたかがそのまま分かる
    pub failing: Vec<String>,
    /// **5 本すべての窓が閉じた日にのみ確定**（第 7 回 Q27）
    pub confirmed: bool,
    /// 確定する日。**1 本でも開始していなければ返らない**
    pub confirms_on: Option<chrono::NaiveDate>,
    /// 確定までの残り日数。同上
    pub days_until_confirmed: Option<i64>,
    /// まだ収集を開始していないソース。確定日が定まらない理由
    pub not_started: Vec<String>,
}

/// 達成日数と合否を数える。
///
/// **`today` を引数に取る**のは、判定が「いつ引いたか」に依存するため（窓の開閉・暫定/確定）。
/// 呼ぶ側が `Asia/Tokyo` の今日を渡す。テストは固定の日を渡す。
///
/// **期間は取らない**（第 6 回 Q23）—— 窓が仕様で決まったので、呼び出し側に委ねると
/// 窓の取り方で合否が動く。
pub async fn achievement(
    pool: &sqlx::PgPool,
    user: Option<uuid::Uuid>,
    today: chrono::NaiveDate,
    targets: &[(String, Subject)],
) -> Result<Achievement, sqlx::Error> {
    let names: Vec<String> = targets.iter().map(|(n, _)| n.clone()).collect();
    let rows = sources(pool, &names).await?;

    let mut out = Vec::with_capacity(targets.len());
    for (name, subject) in targets {
        let src = rows.iter().find(|r| &r.logical_source == name);
        let (display_name, started_on) = match src {
            Some(r) => (r.display_name.clone(), r.collection_started_on),
            None => (name.clone(), None),
        };
        let window_closes_on = started_on.map(|s| s + chrono::Duration::days(WINDOW_DAYS));
        let window_closed = window_closes_on.is_some_and(|c| today >= c);

        // **数えるのは終わった日だけ**（design D15）。今日はまだ終わっていないので分母に入れない。
        // 収集開始日から 200 日なら分母は 200 日になる（specs の固定値がこの数え方を指している）。
        let (achieved_days, denominator) = match started_on {
            None => (0, 0),
            Some(start) => {
                let last_of_window = start + chrono::Duration::days(WINDOW_DAYS - 1);
                let last_counted = (today - chrono::Duration::days(1)).min(last_of_window);
                if last_counted < start {
                    (0, 0)
                } else {
                    let f = facts(pool, user, name, start, last_counted).await?;
                    // **分母から抜けた日は達成日にも数えない**（2 巡目 R4）——
                    // 達成日数が分母を超えるのを防ぐ。
                    let live = f.iter().filter(|d| !d.stopped_full);
                    let denom = live.clone().count() as i64;
                    let hit = live
                        .filter(|d| match subject {
                            Subject::Device => d.event_count > 0,
                            Subject::Usage => d.capturable == Some(true),
                        })
                        .count() as i64;
                    (hit, denom)
                }
            }
        };
        let threshold = denominator as f64 * ACHIEVE_RATIO;
        let met = started_on.is_some() && denominator > 0 && achieved_days as f64 >= threshold;
        out.push(SourceAchievement {
            logical_source: name.clone(),
            display_name,
            subject: *subject,
            collection_started_on: started_on,
            achieved_days,
            denominator,
            threshold,
            met,
            window_closed,
            window_closes_on,
        });
    }

    let failing: Vec<String> = out
        .iter()
        .filter(|s| !s.met)
        .map(|s| s.logical_source.clone())
        .collect();
    let not_started: Vec<String> = out
        .iter()
        .filter(|s| s.collection_started_on.is_none())
        .map(|s| s.logical_source.clone())
        .collect();
    // **確定は 5 本すべての窓が閉じた日**（第 7 回 Q27）。これが無いと、
    // 収集開始 10 日目のソースが 10 日とも達成していれば 100 % で「達成」に見える。
    let confirmed = !out.is_empty() && out.iter().all(|s| s.window_closed);
    let (confirms_on, days_until_confirmed) = if not_started.is_empty() {
        let last = out.iter().filter_map(|s| s.window_closes_on).max();
        (last, last.map(|d| (d - today).num_days().max(0)))
    } else {
        (None, None)
    };

    Ok(Achievement {
        verdict: failing.is_empty(),
        failing,
        confirmed,
        confirms_on,
        days_until_confirmed,
        not_started,
        sources: out,
    })
}

#[cfg(test)]
mod tests;
