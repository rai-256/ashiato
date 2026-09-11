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

/// ソース × 日 の状態（FR-54）。**8 つのいずれか 1 つに決まる。**
///
/// ⑧「退役」は ST03 の深掘りが差し戻したもの（R55 / R56）—— ソースを分けて
/// 古い名前を退役させる運用が通常になったため、**退役した後の空白日**を
/// ⑥「途絶」と区別できないと、退役したソースが毎日「壊れている」ように見える。
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
    /// ⑧ 退役（登録簿の退役した日より後。FR-61）
    Retired,
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
    /// その日の生存信号が報告した取得の試行回数の合計。**信号が無い日は返らない**
    /// —— 来ていない区間の取得率は埋まらない（specs）
    pub attempts: Option<i64>,
    pub successes: Option<i64>,
    /// その日に報告された、満たされていないもの（FR-78 / specs「何が満たされていないかが返る」）。
    /// **取れる状態しか無い日は空**
    pub blockers: Vec<String>,
    /// **区間ごと**の取得率（第 5 回 Q17）。日に畳んだ合計だけだと
    /// 「眠っていた区間」が薄まる —— 想定間隔 6 時間なら 1 日 4 区間になる（review/code.md の R9）
    pub intervals: Vec<Interval>,
}

/// 生存信号 1 件ぶんの区間。**これが「前回の信号から今回まで」にあたる。**
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct Interval {
    pub emitted_at: chrono::DateTime<chrono::Utc>,
    pub capturable: bool,
    pub attempts: i32,
    pub successes: i32,
}

/// ソース 1 本ぶんの稼働状況。
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct SourceCoverage {
    pub logical_source: String,
    pub display_name: String,
    pub expected_gap_sec: i32,
    /// **引き継ぎの鎖の根の日**（第 8 回 Q31）。自分の行の値ではない
    pub collection_started_on: Option<chrono::NaiveDate>,
    /// 退役した日（FR-61）。**この日より後が⑧**。まだ退役していなければ `None`
    pub retired_on: Option<chrono::NaiveDate>,
    pub days: Vec<DayCell>,
}

/// 登録簿の 1 行（導出に要る分だけ）。
#[derive(Debug, Clone)]
pub struct SourceRow {
    pub logical_source: String,
    pub display_name: String,
    pub expected_gap_sec: i32,
    /// **引き継ぎの鎖の根の日**（第 8 回 Q31）。`sources()` が鎖をたどって入れる
    pub collection_started_on: Option<chrono::NaiveDate>,
    pub retired_on: Option<chrono::NaiveDate>,
}

/// `facts` が SQL から受け取る 1 行。
/// （日, 件数, 取得可否, 試行, 成功, 丸ごと破棄, 丸ごと停止）
type FactRow = (
    chrono::NaiveDate,
    i32,
    Option<bool>,
    Option<i64>,
    Option<i64>,
    Vec<String>,
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
    /// その日の信号が報告した、満たされていないもの（`permission` / `sensor` / `network`）。
    /// **重複は畳んである。** 取れる状態しか無い日は空
    blockers: Vec<String>,
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
///
/// **登録簿に行ができた日より前の時刻は、この計算から外す**（第 8 回 Q29）。
/// 記録も信号も捨てない —— 外すのは収集開始日への寄与だけ。
/// 端末の時計が 27 年戻った生存信号が 1 件届くと、開始日が 1999 年に落ち、
/// **前にしか動かないので正しい日を送り直しても戻らない**（実測。成功条件 1 が
/// 「確定・未達」で固まる）。閾値は本人が選択肢から選んだもの。
pub async fn touch_started_on<'e, E>(
    executor: E,
    logical_source: &str,
    at: chrono::DateTime<chrono::Utc>,
) -> Result<(), sqlx::Error>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    let sql = format!(
        "UPDATE core.source
            SET collection_started_on = ($2 AT TIME ZONE '{tz}')::date
          WHERE logical_source = $1
            AND (collection_started_on IS NULL
                 OR collection_started_on > ($2 AT TIME ZONE '{tz}')::date)
            AND ($2 AT TIME ZONE '{tz}')::date >= (registered_at AT TIME ZONE '{tz}')::date",
        tz = DAY_TZ
    );
    sqlx::query(&sql)
        .bind(logical_source)
        .bind(at)
        .execute(executor)
        .await?;
    Ok(())
}

/// `sources` が SQL から受け取る 1 行。
/// （論理ソース名, 表示名, 想定間隔, 鎖の根の収集開始日, 退役した日）
type SourceRowSql = (
    String,
    String,
    i32,
    Option<chrono::NaiveDate>,
    Option<chrono::NaiveDate>,
);

/// 引き継ぎの鎖をたどる深さの上限。**循環したときに回り続けないため**。
/// 自分自身を指す 1 周の循環は登録簿の CHECK 制約が塞いでいるが、2 本以上で
/// 輪になる形は塞げない（どちらの行を入れた時点でも輪はまだ閉じていない）。
const CHAIN_MAX_DEPTH: i32 = 32;

/// 登録簿を引く。`only` が空でなければその論理ソースだけ。
///
/// **収集開始日は引き継ぎの鎖の根から引く**（第 8 回 Q31）—— ソースを分けて
/// 古い名前を退役させたとき、新しい名前の収集開始日は引き継ぎ元の収集開始日にする。
/// これが無いと、名前を分けた日に**成功条件 1 の窓が振り出しに戻る**（1 年の連続性が切れる）。
/// `min()` は NULL を飛ばすので、まだ 1 件も届いていない新しい名前も根の日を継ぐ。
pub async fn sources(pool: &sqlx::PgPool, only: &[String]) -> Result<Vec<SourceRow>, sqlx::Error> {
    let rows: Vec<SourceRowSql> = sqlx::query_as(
        "WITH RECURSIVE chain AS (
           SELECT s.logical_source AS head, s.succeeds AS next,
                  s.collection_started_on AS started, 0 AS depth
             FROM core.source s
           UNION ALL
           SELECT c.head, p.succeeds, p.collection_started_on, c.depth + 1
             FROM chain c JOIN core.source p ON p.logical_source = c.next
            WHERE c.depth < $2
         ),
         root AS (SELECT head, min(started) AS started FROM chain GROUP BY head)
         SELECT s.logical_source, s.display_name, s.expected_gap_sec,
                r.started, s.retired_on
           FROM core.source s JOIN root r ON r.head = s.logical_source
          WHERE cardinality($1::text[]) = 0 OR s.logical_source = ANY($1)
          ORDER BY s.logical_source",
    )
    .bind(only)
    .bind(CHAIN_MAX_DEPTH)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(
            |(
                logical_source,
                display_name,
                expected_gap_sec,
                collection_started_on,
                retired_on,
            )| SourceRow {
                logical_source,
                display_name,
                expected_gap_sec,
                collection_started_on,
                retired_on,
            },
        )
        .collect())
}

/// Must の 5 本を**引き継ぎの鎖の先端**に解決する（第 8 回 Q31
/// 「古い名前を分母から外し、新しい名前が窓を引き継ぐ」）。
///
/// 定数が指す名前が退役していれば、その名前を引き継いだ**退役していないソース**を返す。
/// 引き継ぎ先が無い（まだ作られていない・そちらも退役した）ときは**定数の名前のまま返す**
/// —— 黙って 5 本から消すと、達成が 4 本の合否になって成功条件 1 が緩む。
async fn resolve_tips(
    pool: &sqlx::PgPool,
    names: &[String],
) -> Result<std::collections::HashMap<String, String>, sqlx::Error> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "WITH RECURSIVE tip AS (
           SELECT s.logical_source AS base, s.logical_source AS node,
                  s.retired_on, 0 AS depth
             FROM core.source s WHERE s.logical_source = ANY($1)
           UNION ALL
           SELECT t.base, s.logical_source, s.retired_on, t.depth + 1
             FROM tip t JOIN core.source s ON s.succeeds = t.node
            WHERE t.depth < $2
         )
         SELECT DISTINCT ON (base) base, node
           FROM tip
          WHERE retired_on IS NULL
          ORDER BY base, depth DESC, node",
    )
    .bind(names)
    .bind(CHAIN_MAX_DEPTH)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().collect())
}

/// 名前の並びどおりに稼働状況を集める。**登録簿に無い名前も落とさない**
/// （review/code.md の R20 / H-3）。
///
/// `continue` で飛ばすと、画面に 4 本の格子が並び**5 本目が「無い」ことすら表示されない**。
/// 達成の側は同じ状況を `not_started` として返しているので、黙るとその 2 つが食い違う。
/// 定数と登録簿のずれ自体は `expected_gap_seeded` が CI で止めるが、
/// 実行時にずれたとき（登録簿から行を消した運用）はここが唯一の出口になる。
pub async fn of_sources(
    pool: &sqlx::PgPool,
    user: Option<uuid::Uuid>,
    names: &[String],
    from: chrono::NaiveDate,
    to: chrono::NaiveDate,
) -> Result<Vec<SourceCoverage>, sqlx::Error> {
    let rows = sources(pool, names).await?;
    let mut out = Vec::with_capacity(names.len());
    for name in names {
        let src = match rows.iter().find(|r| &r.logical_source == name) {
            Some(src) => src.clone(),
            None => {
                tracing::warn!(
                    kind = "source_missing",
                    logical_source = %name,
                    "登録簿に無いソース。まだ開始していないものとして返す"
                );
                SourceRow {
                    logical_source: name.clone(),
                    display_name: name.clone(),
                    expected_gap_sec: 0,
                    collection_started_on: None,
                    retired_on: None,
                }
            }
        };
        out.push(of_source(pool, user, &src, from, to).await?);
    }
    Ok(out)
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
              -- **集約する**（review/code.md の R21）。主キーは (user_id, logical_source, day) なので、
              -- 利用者が 2 人いれば同じ日が 2 行返り、LEFT JOIN で 1 日が 2 行に膨らむ。
              -- 達成の分母は「日数」ではなく「行数」になって二重に数えられる。
              -- 生存信号の側（h）は最初から GROUP BY があり、**片方だけ集約が無かった**。
              -- **記録そのものから引く**（tasks 15.5 / ST03 の R57）。
              -- `core.coverage.event_count` で決めていたときは、ST03 が外部サービスからの
              -- 更新経路を開けて出来事の時刻が別の日へ動いた瞬間、
              -- **記録の無い日が「記録あり」・記録のある日が「途絶」**になる（ST03 の実測）。
              -- 稼働記録の表は「新しく入った記録の数」を数え続ける（正典の Requirement）が、
              -- **状態の出どころではなくなる**。
              --
              -- **`core.event_live` ではなく `core.event`。** 稼働記録が答えるのは
              -- 「その日に収集が動いていたか」で、後からの論理削除（FR-50）は
              -- それを書き換えない。丸ごと消した期間は⑤「破棄された期間」が担う。
              c AS (SELECT (event_time AT TIME ZONE '{tz}')::date AS day,
                           count(*)::int AS event_count
                      FROM core.event
                     WHERE ($1::uuid IS NULL OR user_id = $1) AND logical_source = $2
                     GROUP BY 1),
              -- **`bool_or`**: その日に取得できる状態の信号が 1 件でもあれば②（design D7 の (5)）。
              -- 全部が取れない状態のときだけ③（同 (6)）。混在する日は現実にいちばん起きる形
              -- （日の途中で権限が剥がれる）なので、`bool_and` との違いが観測できる検査を置いてある。
              h AS (SELECT (emitted_at AT TIME ZONE '{tz}')::date AS day,
                           bool_or(capturable) AS capturable,
                           sum(attempts)::bigint  AS attempts,
                           sum(successes)::bigint AS successes
                      FROM core.heartbeat
                     WHERE ($1::uuid IS NULL OR user_id = $1) AND logical_source = $2
                     GROUP BY 1),
              -- **満たされていないものを日ごとに畳む**（review/code.md の R39）。
              -- spec の Scenario「取得できない状態が理由とともに残る …
              -- AND 何が満たされていないか（権限）が返る」の後半が未実装だった。
              -- 配列を展開してから集約する（`array_agg` を入れ子にすると型が合わない）。
              b AS (SELECT (emitted_at AT TIME ZONE '{tz}')::date AS day,
                           array_agg(DISTINCT x ORDER BY x) AS blockers
                      FROM core.heartbeat, unnest(blockers) AS x
                     WHERE ($1::uuid IS NULL OR user_id = $1) AND logical_source = $2
                     GROUP BY 1)
         SELECT d.day,
                coalesce(c.event_count, 0) AS event_count,
                h.capturable, h.attempts, h.successes,
                coalesce(b.blockers, '{{}}') AS blockers,
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
           LEFT JOIN b ON b.day = d.day
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
            |(
                day,
                event_count,
                capturable,
                attempts,
                successes,
                blockers,
                dropped_full,
                stopped_full,
            )| {
                DayFacts {
                    day,
                    event_count,
                    capturable,
                    attempts,
                    successes,
                    blockers,
                    dropped_full,
                    stopped_full,
                }
            },
        )
        .collect())
}

/// 生存信号を**1 件ずつ**日に割り当てて返す（review/code.md の R9）。
///
/// spec の Scenario は「**前回の信号からの間に**取得を 360 回試みて 230 回成功したことを示す
/// 生存信号が届く → **その区間の**試行回数と成功回数が返る」。日に畳んだ合計だけを返すと、
/// 想定間隔 6 時間のソースでは 1 日 4 区間が 1 つに混ざり、
/// **眠っていた区間が薄まって見えなくなる** —— 第 5 回 Q17 が Q17 を入れた理由そのものが消える。
async fn intervals(
    pool: &sqlx::PgPool,
    user: Option<uuid::Uuid>,
    source: &str,
    from: chrono::NaiveDate,
    to: chrono::NaiveDate,
) -> Result<std::collections::HashMap<chrono::NaiveDate, Vec<Interval>>, sqlx::Error> {
    let sql = format!(
        "SELECT (emitted_at AT TIME ZONE '{tz}')::date AS day,
                emitted_at, capturable, attempts, successes
           FROM core.heartbeat
          WHERE ($1::uuid IS NULL OR user_id = $1) AND logical_source = $2
            AND emitted_at >= ($3::date::timestamp AT TIME ZONE '{tz}')
            AND emitted_at <  (($4::date + 1)::timestamp AT TIME ZONE '{tz}')
          ORDER BY emitted_at",
        tz = DAY_TZ
    );
    let rows: Vec<(
        chrono::NaiveDate,
        chrono::DateTime<chrono::Utc>,
        bool,
        i32,
        i32,
    )> = sqlx::query_as(&sql)
        .bind(user)
        .bind(source)
        .bind(from)
        .bind(to)
        .fetch_all(pool)
        .await?;
    let mut out: std::collections::HashMap<chrono::NaiveDate, Vec<Interval>> =
        std::collections::HashMap::new();
    for (day, emitted_at, capturable, attempts, successes) in rows {
        out.entry(day).or_default().push(Interval {
            emitted_at,
            capturable,
            attempts,
            successes,
        });
    }
    Ok(out)
}

/// 記録か生存信号があった日を古い順に。**途絶の判定に要る**（前後の活動を測る）。
async fn active_days(
    pool: &sqlx::PgPool,
    user: Option<uuid::Uuid>,
    source: &str,
) -> Result<Vec<chrono::NaiveDate>, sqlx::Error> {
    let sql = format!(
        "SELECT day FROM (
           -- **記録そのものから引く**（tasks 15.5 / ST03 の R57）。`facts` と同じ出どころ ——
           -- 片方だけ `core.coverage` に残すと、更新で時刻が動いた日が
           -- 「記録あり」にはならないのに「前後の活動」には数えられる。
           SELECT (event_time AT TIME ZONE '{tz}')::date AS day FROM core.event
            WHERE ($1::uuid IS NULL OR user_id = $1) AND logical_source = $2
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

/// 8 状態を決める（design D7 / D33）。**上から評価して最初に当たったものを返す。**
///
/// 順序は specs の Requirement 本文に列挙してある —— **本人が決めた**（第 5 回 Q19）。
/// 「なぜこの日はデータが少ないのか」に画面が先に答えるので、
/// 1 日を丸ごと止めた日に記録が一部残っていても、その日は④として見える。
///
/// ⑧「退役」は⑦「導入前」の**直後**に見る（design D33）—— どちらも
/// 「その日はこのソースの収集期間の外side」という同じ型で、`retired_on` は
/// `collection_started_on` の対。判定は **`day > retired_on`**（退役した日そのものは
/// まだ収集していたので、その日は本来の状態のまま出る）。
fn decide(
    facts: &DayFacts,
    started_on: Option<chrono::NaiveDate>,
    retired_on: Option<chrono::NaiveDate>,
    gap_days: f64,
    active: &[chrono::NaiveDate],
) -> DayState {
    match started_on {
        None => return DayState::BeforeStart,
        Some(s) if facts.day < s => return DayState::BeforeStart,
        _ => {}
    }
    if retired_on.is_some_and(|r| facts.day > r) {
        return DayState::Retired;
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
    let mut by_day = intervals(pool, user, &src.logical_source, from, to).await?;
    let gap_days = f64::from(src.expected_gap_sec) / 86_400.0;
    let days = facts
        .iter()
        .map(|f| DayCell {
            day: f.day,
            state: decide(
                f,
                src.collection_started_on,
                src.retired_on,
                gap_days,
                &active,
            ),
            event_count: f.event_count,
            attempts: f.attempts,
            successes: f.successes,
            blockers: f.blockers.clone(),
            intervals: by_day.remove(&f.day).unwrap_or_default(),
        })
        .collect();
    Ok(SourceCoverage {
        logical_source: src.logical_source.clone(),
        display_name: src.display_name.clone(),
        expected_gap_sec: src.expected_gap_sec,
        collection_started_on: src.collection_started_on,
        retired_on: src.retired_on,
        days,
    })
}

// ---------------------------------------------------------------- 達成日数と合否

/// ソース 1 本ぶんの達成（NFR-13）。
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct SourceAchievement {
    /// **実際に数えた名前**（引き継ぎの鎖の先端）。定数の名前が退役していれば後継が入る
    pub logical_source: String,
    /// 定数が名指ししている名前（`must_sources()` の側）。鎖をたどっていなければ同じ
    pub named_source: String,
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
    let named: Vec<String> = targets.iter().map(|(n, _)| n.clone()).collect();
    // **鎖の先端を数える**（第 8 回 Q31）。古い名前は分母から外れ、後継が窓を引き継ぐ。
    let tips = resolve_tips(pool, &named).await?;
    let names: Vec<String> = named
        .iter()
        .map(|n| tips.get(n).cloned().unwrap_or_else(|| n.clone()))
        .collect();
    let rows = sources(pool, &names).await?;

    let mut out = Vec::with_capacity(targets.len());
    for ((named_source, subject), name) in targets.iter().zip(names.iter()) {
        let src = rows.iter().find(|r| &r.logical_source == name);
        let (display_name, started_on, retired_on) = match src {
            Some(r) => (
                r.display_name.clone(),
                r.collection_started_on,
                r.retired_on,
            ),
            None => (name.clone(), None, None),
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
                    //
                    // **退役した日より後も分母に入れない**（ST03 の R64 / 第 8 回 Q31）。
                    // 入れたままだと、退役してから窓が閉じるまでの日が毎日「未達」として
                    // 積まれ、**名前を分けただけで成功条件 1 が落ちる**。
                    let live = f
                        .iter()
                        .filter(|d| !d.stopped_full)
                        .filter(|d| !retired_on.is_some_and(|r| d.day > r));
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
            named_source: named_source.clone(),
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
