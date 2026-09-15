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

/// 格子の 3 段。**8 状態の区別は週を選んだときの文字が担う**（第 5 回 Q20 / Q21）。
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
    /// その日（`Asia/Tokyo`）に属する破棄の件数（ST04 / design D9）。
    /// **時間ごとの件数から数える** —— 範囲を持たない破棄（読めなかった行など）は入らない
    pub dropped_count: i64,
    /// その日に重なる破棄の範囲を、**端が接するもの・重なるものでつないでから**その日で切った区間（design D8 / D9）
    pub dropped_ranges: Vec<DroppedRange>,
}

/// その日の中で切った破棄の区間（design D9）。画面の「うち N 件を破棄（from〜to）」の材料。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct DroppedRange {
    /// `Asia/Tokyo` の `HH:MM`（分に切り捨て）
    pub from: String,
    /// `Asia/Tokyo` の `HH:MM`（分に切り上げ）。日の終わりまで続くなら `24:00`
    pub to: String,
    /// この区間と重なる時間の件数の合計。時間が 2 つの区間にまたがるときは前の区間に数える
    pub count: i64,
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
    /// **鎖の先端**（第 8 回 Q31）
    pub logical_source: String,
    /// 定数が名指ししている名前。乗り換えが起きたことが画面から読めるように返す
    pub named_source: String,
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
    /// **鎖の先端**（いま生きている名前）。定数が指す名前が退役していれば後継が入る
    pub logical_source: String,
    /// 呼び出し側が名指しした名前（Must の定数）。鎖をたどっていなければ `logical_source` と同じ
    pub named_source: String,
    pub display_name: String,
    pub expected_gap_sec: i32,
    /// **鎖全体でいちばん古い収集開始日**（第 8 回 Q31）。自分の行の値ではない
    pub collection_started_on: Option<chrono::NaiveDate>,
    /// 先端が退役していればその日
    pub retired_on: Option<chrono::NaiveDate>,
    /// **数えるときに見る名前の全部**（根 → 先端）。先端だけで数えると、分母は鎖の根から
    /// 数えるのに分子は切り替え後しか拾わない（review/code-r2.md の R1）
    pub chain: Vec<String>,
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

/// 収集開始日を動かしにきたものが、記録か生存信号か（第 9 回 Q32）。
///
/// **閾値が掛かるのは生存信号だけ** —— 本人の答え。
/// 生存信号の `emitted_at` は**端末の時計そのもの**なので、狂えばそのまま入ってくる。
/// 記録の `event_time` は**出来事が起きた時刻**で、古いことに正当な理由がある
/// （端末にある写真は撮影時刻が何年も前、ブラウザ履歴は導入時点で過去ぶんが取れる、
/// Takeout 系は過去 1 年ぶんをまとめて流し込む）。**この 2 つを同じ閾値で測らない。**
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arrival {
    /// 記録（`core.event`）。**閾値を掛けない**
    Record,
    /// 生存信号（`core.heartbeat`）。**登録簿に行ができた日より前は外す**
    Heartbeat,
}

/// 収集開始日を**受け口の側で**埋める（design D5 / 第 6 回 Q24 / 第 7 回 Q26 / 第 9 回 Q32）。
///
/// **記録が作られた時刻の日**を当てる。受信時刻ではない —— 圏外で 3 日ぶん溜めて送ると、
/// 記録のある日が⑦「導入前」になり NFR-13 の分母からも落ちる。
///
/// **`least()` を取るので、古い記録が後から届くたびに前へ動く**（第 7 回 Q26。
/// 圏外の保持がこれを起こす）。遡ると NFR-13 の窓の起点も動くが、
/// **動くのは入力であって判定式ではない** —— 状態も達成も行に焼いていないので、
/// 同じ入力からは必ず同じ答えが出る。
///
/// **生存信号だけ、登録簿に行ができた日より前を計算から外す**（第 9 回 Q32。
/// 第 8 回 Q29 は記録にも掛けていたが、**過去ぶんを流し込む運用で全日が⑦になった**）。
/// 記録も信号も捨てない —— 外すのは収集開始日への寄与だけ。
/// 端末の時計が 27 年戻った生存信号が 1 件届くと、開始日が 1999 年に落ち、
/// **前にしか動かないので正しい日を送り直しても戻らない**（実測）。
pub async fn touch_started_on<'e, E>(
    executor: E,
    logical_source: &str,
    at: chrono::DateTime<chrono::Utc>,
    arrival: Arrival,
) -> Result<(), sqlx::Error>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    // **弾いたことを残す**（review/code-r2.md の H-1）。閾値に当たらなかったときの
    // 空振りには 2 種類あって、「すでに開始日がもっと前」（正常）と
    // 「登録より前なので外した」（異常）は呼び出し側から区別できない。
    // 黙ると、時計の狂った端末が 1 台あることに誰も気付けない ——
    // **1 往復のまま**判定できるので、更新と同じ文で聞く。
    let gate = match arrival {
        // 記録に閾値は掛からない（第 9 回 Q32）
        Arrival::Record => "true".to_string(),
        Arrival::Heartbeat => format!(
            "($2 AT TIME ZONE '{tz}')::date >= (registered_at AT TIME ZONE '{tz}')::date",
            tz = DAY_TZ
        ),
    };
    let sql = format!(
        "WITH u AS (
           UPDATE core.source
              SET collection_started_on = ($2 AT TIME ZONE '{tz}')::date
            WHERE logical_source = $1
              AND (collection_started_on IS NULL
                   OR collection_started_on > ($2 AT TIME ZONE '{tz}')::date)
              AND {gate}
            RETURNING 1
         )
         SELECT (SELECT count(*) FROM u) > 0 AS moved,
                NOT ({gate}) AS below
           FROM core.source WHERE logical_source = $1",
        tz = DAY_TZ,
        gate = gate
    );
    let got: Option<(bool, bool)> = sqlx::query_as(&sql)
        .bind(logical_source)
        .bind(at)
        .fetch_optional(executor)
        .await?;
    if let Some((moved, below)) = got {
        if below && !moved {
            tracing::warn!(
                kind = "started_on_before_registration",
                logical_source = %logical_source,
                at = %at,
                "登録簿に行ができた日より前に発信された生存信号。収集開始日の計算から外した（信号は残している）"
            );
        }
    }
    Ok(())
}

/// 登録簿の生の 1 行。**鎖をたどる前の姿**。
#[derive(Debug, Clone)]
struct RawSource {
    logical_source: String,
    display_name: String,
    expected_gap_sec: i32,
    /// **その行自身**の収集開始日。鎖をまたいだ値ではない
    own_started_on: Option<chrono::NaiveDate>,
    retired_on: Option<chrono::NaiveDate>,
    succeeds: Option<String>,
}

/// 引き継ぎの鎖をたどる深さの上限。**輪になったときに回り続けないため**。
///
/// 自分自身を指す 1 周は登録簿の CHECK が、枝分かれと合流は `succeeds` の一意索引が
/// 塞いでいるが、`A → B → A` の形は**どちらの行を入れた時点でも輪がまだ閉じていない**ので
/// 入口では塞げない。たどる側が**通った名前を覚えて**輪を検出し、警告を出して止める
/// （上限に当たることそのものが異常なので、黙って打ち切らない。review/code-r2.md の R12）。
const CHAIN_MAX_DEPTH: usize = 32;

/// 登録簿を丸ごと引く。**鎖をたどるので、要る行だけを引くことができない**
/// （引き継ぎ元も後継も別の行にある）。登録簿はソースの本数ぶんしかないので安い。
async fn registry(pool: &sqlx::PgPool) -> Result<Vec<RawSource>, sqlx::Error> {
    type Row = (
        String,
        String,
        i32,
        Option<chrono::NaiveDate>,
        Option<chrono::NaiveDate>,
        Option<String>,
    );
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT logical_source, display_name, expected_gap_sec,
                collection_started_on, retired_on, succeeds
           FROM core.source ORDER BY logical_source",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(
            |(
                logical_source,
                display_name,
                expected_gap_sec,
                own_started_on,
                retired_on,
                succeeds,
            )| {
                RawSource {
                    logical_source,
                    display_name,
                    expected_gap_sec,
                    own_started_on,
                    retired_on,
                    succeeds,
                }
            },
        )
        .collect())
}

/// 1 本の論理ソースを**引き継ぎの鎖ごと**解決する（第 8 回 Q31）。
///
/// 鎖は 3 つの部分でできている:
///
/// 1. **引き継ぎ元**（`succeeds` を遡る）—— 収集開始日も達成日もここから続いている。
///    「引き継ぎ元のあるソースは、引き継ぎ元の収集開始日を自分の収集開始日とする」（FR-61）
/// 2. **自分**
/// 3. **後継**（`succeeds` で指されている側へ進む）。ただし
///    **いま見ているソースが退役しているあいだだけ**進む ——
///    まだ動いているソースから勝手に乗り換えない（review/code-r2.md の R2）。
///    spec の逐語も「Must の 5 ソースの**うち退役したものについて**」
///
/// **鎖の全部の名前で数える**（同 R1）。先端の名前だけで数えると、
/// 分母は鎖の根から数えるのに分子は切り替え後の記録しか拾わず、
/// **名前を分けた翌日に成功条件 1 が 0 % に落ちる** —— spec が防ぐと書いている当のもの。
fn resolve_chain(reg: &std::collections::HashMap<String, RawSource>, base: &str) -> Chain {
    // 後継を引くための逆引き。登録簿の一意索引が枝分かれを塞いでいるが、
    // **その索引は作れないことがある**（既に枝分かれのある DB では作成に失敗し、
    // 起動を止めないよう WARNING にしてある）。たどる側でも受ける ——
    // **名前順で決定的に 1 本選び、分岐は警告に残す**（review/code-r2.md の H-4）。
    let mut successor: std::collections::HashMap<&str, Vec<&str>> =
        std::collections::HashMap::new();
    for r in reg.values() {
        if let Some(prev) = r.succeeds.as_deref() {
            successor
                .entry(prev)
                .or_default()
                .push(r.logical_source.as_str());
        }
    }
    for v in successor.values_mut() {
        v.sort_unstable();
    }

    let mut names: Vec<String> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    // (1) 引き継ぎ元を遡って、根から順に積む
    let mut back: Vec<String> = Vec::new();
    let mut cur = base.to_string();
    while let Some(row) = reg.get(&cur) {
        if !seen.insert(cur.clone()) {
            tracing::warn!(
                kind = "source_chain_cycle",
                logical_source = %base, at = %cur,
                "引き継ぎの鎖が輪になっている。そこで止める"
            );
            break;
        }
        back.push(cur.clone());
        match &row.succeeds {
            Some(p) if back.len() < CHAIN_MAX_DEPTH => cur = p.clone(),
            Some(_) => {
                tracing::warn!(
                    kind = "source_chain_too_deep",
                    logical_source = %base, depth = CHAIN_MAX_DEPTH,
                    "引き継ぎ元をたどる深さが上限に達した。収集開始日が本来より後ろになる"
                );
                break;
            }
            None => break,
        }
    }
    back.reverse();
    names.extend(back);

    // (3) 退役しているあいだだけ後継へ進む
    let mut tip = base.to_string();
    while let Some(row) = reg.get(&tip) {
        if row.retired_on.is_none() {
            break;
        }
        let Some(next) = successor.get(tip.as_str()).and_then(|v| {
            if v.len() > 1 {
                tracing::warn!(
                    kind = "source_chain_forked",
                    logical_source = %tip, successors = ?v,
                    "引き継ぎ元が枝分かれしている。名前順で先頭を採る（登録簿を直すこと）"
                );
            }
            v.first().copied()
        }) else {
            // 退役しているのに後継が無い。**5 本から黙って消さない**（合否が緩む）ので
            // そのまま数え続けるが、人間が知るべき状態なので残す
            tracing::warn!(
                kind = "source_retired_without_successor",
                logical_source = %tip,
                "退役しているのに引き継ぎ先が無い。このソースは達成日が伸びない"
            );
            break;
        };
        let next = next.to_string();
        if !seen.insert(next.clone()) {
            tracing::warn!(
                kind = "source_chain_cycle",
                logical_source = %base, at = %next,
                "引き継ぎの鎖が輪になっている。そこで止める"
            );
            break;
        }
        if names.len() >= CHAIN_MAX_DEPTH {
            tracing::warn!(
                kind = "source_chain_too_deep",
                logical_source = %base, depth = CHAIN_MAX_DEPTH,
                "後継をたどる深さが上限に達した。先端が本当の先端でない"
            );
            break;
        }
        names.push(next.clone());
        tip = next;
    }

    let started_on = names
        .iter()
        .filter_map(|n| reg.get(n).and_then(|r| r.own_started_on))
        .min();
    let head = reg.get(&tip);
    Chain {
        names,
        started_on,
        retired_on: head.and_then(|r| r.retired_on),
        display_name: head.map_or_else(|| tip.clone(), |r| r.display_name.clone()),
        expected_gap_sec: head.map_or(0, |r| r.expected_gap_sec),
        tip,
    }
}

/// 解決済みの鎖。
struct Chain {
    /// 鎖の全部の名前（根 → 先端）。**数えるときはこの全部を見る**
    names: Vec<String>,
    /// 鎖の先端（いま生きている名前）
    tip: String,
    display_name: String,
    expected_gap_sec: i32,
    /// 鎖全体でいちばん古い収集開始日
    started_on: Option<chrono::NaiveDate>,
    /// 先端が退役していればその日
    retired_on: Option<chrono::NaiveDate>,
}

/// 登録簿を引き、名前ごとに鎖を解決して返す。
///
/// **登録簿に無い名前も落とさない**（review/code.md の R20）—— 画面に 4 本しか並ばず
/// 5 本目が「無い」ことすら出ないのを防ぐ。
pub async fn sources(pool: &sqlx::PgPool, only: &[String]) -> Result<Vec<SourceRow>, sqlx::Error> {
    let reg: std::collections::HashMap<String, RawSource> = registry(pool)
        .await?
        .into_iter()
        .map(|r| (r.logical_source.clone(), r))
        .collect();
    let wanted: Vec<String> = if only.is_empty() {
        let mut all: Vec<String> = reg.keys().cloned().collect();
        all.sort();
        all
    } else {
        only.to_vec()
    };
    Ok(wanted
        .iter()
        .map(|name| {
            if !reg.contains_key(name) {
                tracing::warn!(
                    kind = "source_missing",
                    logical_source = %name,
                    "登録簿に無いソース。まだ開始していないものとして返す"
                );
            }
            let c = resolve_chain(&reg, name);
            SourceRow {
                logical_source: c.tip.clone(),
                named_source: name.clone(),
                display_name: c.display_name,
                expected_gap_sec: c.expected_gap_sec,
                collection_started_on: c.started_on,
                retired_on: c.retired_on,
                chain: if c.names.is_empty() {
                    vec![name.clone()]
                } else {
                    c.names
                },
            }
        })
        .collect())
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
    let mut out = Vec::with_capacity(rows.len());
    for src in &rows {
        out.push(of_source(pool, user, src, from, to).await?);
    }
    Ok(out)
}

/// 同じソースの破棄の範囲を**つないだ島**を作る CTE（design D8（仮） / spec「端が接する 2 本の破棄は合わせて丸ごと判定される」）。
///
/// 端末は 1 度送ろうとした報告を書き換えずに次の報告を作る（C3 / R2）ので、2 本に割れた破棄は
/// 1 本ずつ見るとどちらもその日を丸ごと覆わない。**`次の始まり <= これまでの終わり` ならつなぐ**（gaps-and-islands）。
/// 隙間の許容は置かない —— 近さで合わせると、間に届いた記録がある日まで⑤に塗る。
///
/// 材料は `core.drop_report`（範囲を持つ行）と、ST02 の `core.coverage_span` の `dropped`（終わりの無い範囲は無限）。
/// 引数は `$1` 利用者 / `$2` ソースの配列 / `$3` 最初の日 / `$4` 最後の日。窓の端に接する範囲まで拾う。
fn drop_islands_cte(tz: &str) -> String {
    format!(
        "dr AS (SELECT range_start AS s, range_end AS e
                  FROM core.drop_report
                 WHERE ($1::uuid IS NULL OR user_id = $1) AND logical_source = ANY($2)
                   AND range_start IS NOT NULL
                   AND range_start <= (($4::date + 1)::timestamp AT TIME ZONE '{tz}')
                   AND range_end   >= ($3::date::timestamp AT TIME ZONE '{tz}')
                UNION ALL
                SELECT started_at, coalesce(ended_at, 'infinity'::timestamptz)
                  FROM core.coverage_span
                 WHERE ($1::uuid IS NULL OR user_id = $1) AND logical_source = ANY($2)
                   AND kind = 'dropped'
                   AND started_at <= (($4::date + 1)::timestamp AT TIME ZONE '{tz}')
                   AND (ended_at IS NULL OR ended_at >= ($3::date::timestamp AT TIME ZONE '{tz}'))),
              dr_prev AS (SELECT s, e,
                                 max(e) OVER (ORDER BY s, e ROWS BETWEEN UNBOUNDED PRECEDING AND 1 PRECEDING)
                                   AS prev_end
                            FROM dr),
              dr_grp AS (SELECT s, e,
                                sum(CASE WHEN prev_end IS NULL OR s > prev_end THEN 1 ELSE 0 END)
                                  OVER (ORDER BY s, e ROWS UNBOUNDED PRECEDING) AS grp
                           FROM dr_prev),
              islands AS (SELECT min(s) AS s, max(e) AS e FROM dr_grp GROUP BY grp)"
    )
}

/// 日ごとの材料を SQL で 1 度に集める。
///
/// **日の区切りは PostgreSQL の `AT TIME ZONE` に任せる**（design D1）——
/// 日を引く場所がアプリと SQL に割れると、片方だけがずれても誰も気付かない。
async fn facts(
    pool: &sqlx::PgPool,
    user: Option<uuid::Uuid>,
    sources: &[String],
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
                     WHERE ($1::uuid IS NULL OR user_id = $1) AND logical_source = ANY($2)
                       AND event_time >= ($3::date::timestamp AT TIME ZONE '{tz}')
                       AND event_time <  (($4::date + 1)::timestamp AT TIME ZONE '{tz}')
                     GROUP BY 1),
              -- **`bool_or`**: その日に取得できる状態の信号が 1 件でもあれば②（design D7 の (5)）。
              -- 全部が取れない状態のときだけ③（同 (6)）。混在する日は現実にいちばん起きる形
              -- （日の途中で権限が剥がれる）なので、`bool_and` との違いが観測できる検査を置いてある。
              h AS (SELECT (emitted_at AT TIME ZONE '{tz}')::date AS day,
                           bool_or(capturable) AS capturable,
                           sum(attempts)::bigint  AS attempts,
                           sum(successes)::bigint AS successes
                      FROM core.heartbeat
                     WHERE ($1::uuid IS NULL OR user_id = $1) AND logical_source = ANY($2)
                       AND emitted_at >= ($3::date::timestamp AT TIME ZONE '{tz}')
                       AND emitted_at <  (($4::date + 1)::timestamp AT TIME ZONE '{tz}')
                     GROUP BY 1),
              -- **満たされていないものを日ごとに畳む**（review/code.md の R39）。
              -- spec の Scenario「取得できない状態が理由とともに残る …
              -- AND 何が満たされていないか（権限）が返る」の後半が未実装だった。
              -- 配列を展開してから集約する（`array_agg` を入れ子にすると型が合わない）。
              b AS (SELECT (emitted_at AT TIME ZONE '{tz}')::date AS day,
                           array_agg(DISTINCT x ORDER BY x) AS blockers
                      FROM core.heartbeat, unnest(blockers) AS x
                     WHERE ($1::uuid IS NULL OR user_id = $1) AND logical_source = ANY($2)
                       AND emitted_at >= ($3::date::timestamp AT TIME ZONE '{tz}')
                       AND emitted_at <  (($4::date + 1)::timestamp AT TIME ZONE '{tz}')
                     GROUP BY 1),
              {islands}
         SELECT d.day,
                coalesce(c.event_count, 0) AS event_count,
                h.capturable, h.attempts, h.successes,
                coalesce(b.blockers, '{{}}') AS blockers,
                -- **つないだ島で**丸ごと覆うかを見る（design D8）。1 本ずつ見ると、割れた報告で⑤が立たない
                EXISTS (SELECT 1 FROM islands i
                         WHERE i.s <= (d.day::timestamp AT TIME ZONE '{tz}')
                           AND i.e >= ((d.day + 1)::timestamp AT TIME ZONE '{tz}'))
                  AS dropped_full,
                EXISTS (SELECT 1 FROM core.coverage_span s
                         WHERE ($1::uuid IS NULL OR s.user_id = $1) AND s.logical_source = ANY($2) AND s.kind = 'stopped'
                           AND s.started_at <= (d.day::timestamp AT TIME ZONE '{tz}')
                           AND (s.ended_at IS NULL
                                OR s.ended_at >= ((d.day + 1)::timestamp AT TIME ZONE '{tz}')))
                  AS stopped_full
           FROM d
           LEFT JOIN c ON c.day = d.day
           LEFT JOIN h ON h.day = d.day
           LEFT JOIN b ON b.day = d.day
          ORDER BY d.day",
        tz = DAY_TZ,
        islands = drop_islands_cte(DAY_TZ)
    );
    let rows: Vec<FactRow> = sqlx::query_as(&sql)
        .bind(user)
        .bind(sources)
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

/// 日ごとの破棄の件数と区間（design D9 / spec「稼働状況の応答は日ごとの破棄の件数と時刻の範囲を持つ」）。
#[derive(Debug, Clone, Default)]
struct DayDrops {
    count: i64,
    ranges: Vec<DroppedRange>,
}

/// 時刻の区間 `[始まり, 終わり)`。
type Span = (chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>);

/// `dropped_by_day` が SQL から受け取る区間の 1 行。（日, 切った始まり, 切った終わり, `HH:MM`, `HH:MM`）
type SectionRow = (
    chrono::NaiveDate,
    chrono::DateTime<chrono::Utc>,
    chrono::DateTime<chrono::Utc>,
    String,
    String,
);

/// 破棄の件数と区間を日ごとに引く。
///
/// **日の区切りと時と分の表記は SQL の `AT TIME ZONE` に任せる**（design D1。日を引く場所を割らない）。
/// 件数は `core.drop_report_hour` から数える —— 範囲と総件数からは日ごとの件数を割り戻せない（C12）。
async fn dropped_by_day(
    pool: &sqlx::PgPool,
    user: Option<uuid::Uuid>,
    sources: &[String],
    from: chrono::NaiveDate,
    to: chrono::NaiveDate,
) -> Result<std::collections::HashMap<chrono::NaiveDate, DayDrops>, sqlx::Error> {
    // 区間: 島をその日で切る。`to` は分に切り上げ、日の終わりに届けば 24:00
    let sections_sql = format!(
        "WITH d AS (SELECT day,
                           (day::timestamp AT TIME ZONE '{tz}') AS ds,
                           ((day + 1)::timestamp AT TIME ZONE '{tz}') AS de
                      FROM (SELECT generate_series($3::date, $4::date, '1 day')::date AS day) g),
              {islands},
              cut AS (SELECT d.day, d.de, greatest(i.s, d.ds) AS cs, least(i.e, d.de) AS ce
                        FROM d JOIN islands i ON i.s < d.de AND i.e > d.ds),
              -- 終わりを分に切り上げてから日の終わりと比べる（review R11）。切り上げる前に比べると、
              -- 23:59:00.001〜23:59:59.999 に終わる区間が翌日の 0 時に丸まって `00:00` と出る
              up AS (SELECT day, de, cs, ce,
                            date_trunc('minute', ce)
                              + CASE WHEN ce > date_trunc('minute', ce)
                                     THEN interval '1 minute' ELSE interval '0' END AS cr
                       FROM cut)
         SELECT day, cs, ce,
                to_char(cs AT TIME ZONE '{tz}', 'HH24:MI') AS from_hm,
                CASE WHEN cr >= de THEN '24:00'
                     ELSE to_char(cr AT TIME ZONE '{tz}', 'HH24:MI')
                END AS to_hm
           FROM up
          ORDER BY day, cs",
        tz = DAY_TZ,
        islands = drop_islands_cte(DAY_TZ)
    );
    let sections: Vec<SectionRow> = sqlx::query_as(&sections_sql)
        .bind(user)
        .bind(sources)
        .bind(from)
        .bind(to)
        .fetch_all(pool)
        .await?;

    // 時間ごとの件数: 時間の始まりが属する日に入れる（`Asia/Tokyo` は整数時間のずれなので時間は日をまたがない）
    let hours_sql = format!(
        "SELECT (h.hour AT TIME ZONE '{tz}')::date AS day, h.hour, sum(h.count)::bigint
           FROM core.drop_report_hour h
           JOIN core.drop_report r ON r.id = h.report_id
          WHERE ($1::uuid IS NULL OR r.user_id = $1) AND r.logical_source = ANY($2)
            AND h.hour >= ($3::date::timestamp AT TIME ZONE '{tz}')
            AND h.hour <  (($4::date + 1)::timestamp AT TIME ZONE '{tz}')
          GROUP BY 1, 2
          ORDER BY 2",
        tz = DAY_TZ
    );
    let hours: Vec<(chrono::NaiveDate, chrono::DateTime<chrono::Utc>, i64)> =
        sqlx::query_as(&hours_sql)
            .bind(user)
            .bind(sources)
            .bind(from)
            .bind(to)
            .fetch_all(pool)
            .await?;

    let mut out: std::collections::HashMap<chrono::NaiveDate, DayDrops> =
        std::collections::HashMap::new();
    let mut bounds: std::collections::HashMap<chrono::NaiveDate, Vec<Span>> =
        std::collections::HashMap::new();
    for (day, cs, ce, from_hm, to_hm) in sections {
        let e = out.entry(day).or_default();
        e.ranges.push(DroppedRange {
            from: from_hm,
            to: to_hm,
            count: 0,
        });
        bounds.entry(day).or_default().push((cs, ce));
    }
    for (day, hour, count) in hours {
        let e = out.entry(day).or_default();
        e.count += count;
        let hour_end = hour + chrono::Duration::hours(1);
        // 前の区間から当てる（1 つの時間を 2 つの区間に二重に数えない）
        if let Some(k) = bounds
            .get(&day)
            .and_then(|b| b.iter().position(|(cs, ce)| hour < *ce && hour_end > *cs))
        {
            e.ranges[k].count += count;
        }
    }
    Ok(out)
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
    sources: &[String],
    from: chrono::NaiveDate,
    to: chrono::NaiveDate,
) -> Result<std::collections::HashMap<chrono::NaiveDate, Vec<Interval>>, sqlx::Error> {
    let sql = format!(
        "SELECT (emitted_at AT TIME ZONE '{tz}')::date AS day,
                emitted_at, capturable, attempts, successes
           FROM core.heartbeat
          WHERE ($1::uuid IS NULL OR user_id = $1) AND logical_source = ANY($2)
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
        .bind(sources)
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
///
/// **期間で絞る**（review/code-r2.md の R9 / I4）。絞りが無かったときは
/// `core.event` を毎リクエスト全走査していた —— 位置は 60 秒間隔で年 50 万行を超え、
/// `achievement` はこれをソースごとに呼ぶ。`core.coverage`（1 日 1 行）から
/// `core.event`（記録ごとに 1 行）へ出どころを移したときに、絞りを足していなかった。
///
/// 窓の外も要るので、**想定間隔ぶん前後に広げる** —— 途絶は「その日をまたぐ前後の
/// 記録・生存信号が想定間隔以内にあるか」で決まるので、窓の端の日は外側を見る。
async fn active_days(
    pool: &sqlx::PgPool,
    user: Option<uuid::Uuid>,
    sources: &[String],
    from: chrono::NaiveDate,
    to: chrono::NaiveDate,
    gap_days: f64,
) -> Result<Vec<chrono::NaiveDate>, sqlx::Error> {
    // 想定間隔を日に切り上げて広げる（60 日のソースもあるので固定値にしない）
    let pad = chrono::Duration::days(gap_days.ceil().max(0.0) as i64 + 1);
    let lo = from - pad;
    let hi = to + pad;
    let sql = format!(
        "SELECT day FROM (
           SELECT (event_time AT TIME ZONE '{tz}')::date AS day FROM core.event
            WHERE ($1::uuid IS NULL OR user_id = $1) AND logical_source = ANY($2)
              AND event_time >= ($3::date::timestamp AT TIME ZONE '{tz}')
              AND event_time <  (($4::date + 1)::timestamp AT TIME ZONE '{tz}')
           UNION
           SELECT (emitted_at AT TIME ZONE '{tz}')::date FROM core.heartbeat
            WHERE ($1::uuid IS NULL OR user_id = $1) AND logical_source = ANY($2)
              AND emitted_at >= ($3::date::timestamp AT TIME ZONE '{tz}')
              AND emitted_at <  (($4::date + 1)::timestamp AT TIME ZONE '{tz}')
         ) t ORDER BY day",
        tz = DAY_TZ
    );
    let rows: Vec<(chrono::NaiveDate,)> = sqlx::query_as(&sql)
        .bind(user)
        .bind(sources)
        .bind(lo)
        .bind(hi)
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
/// 「その日はこのソースの収集期間の外」という同じ型で、`retired_on` は
/// `collection_started_on` の対。
///
/// 判定は **`day >= retired_on`（退役した日を含む）**。当初は「退役した日そのものは
/// まだ収集していた」を理由に `>` にしていたが、**正典の逐語は「以降」**
/// （`docs/requirements.md` の FR-54「登録簿の退役した日以降」/ FR-80
/// 「退役した日以降は途絶の判定の対象外」）。実装が要件の逐語を独断で変えていたので戻した
/// （review/code-r2.md の I1）。
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
    if retired_on.is_some_and(|r| facts.day >= r) {
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
    let gap_days = f64::from(src.expected_gap_sec) / 86_400.0;
    // **鎖の全部の名前で引く**（第 8 回 Q31 / review/code-r2.md の R1）——
    // 引き継ぎ元の時代の記録もこのソースの稼働状況の一部。
    let facts = facts(pool, user, &src.chain, from, to).await?;
    let active = active_days(pool, user, &src.chain, from, to, gap_days).await?;
    let mut by_day = intervals(pool, user, &src.chain, from, to).await?;
    let mut drops = dropped_by_day(pool, user, &src.chain, from, to).await?;
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
            dropped_count: drops.get(&f.day).map_or(0, |d| d.count),
            dropped_ranges: drops.remove(&f.day).map(|d| d.ranges).unwrap_or_default(),
        })
        .collect();
    Ok(SourceCoverage {
        logical_source: src.logical_source.clone(),
        named_source: src.named_source.clone(),
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
    // **鎖ごと解決する**（第 8 回 Q31）。古い名前は分母から外れ、後継が窓を引き継ぐ ——
    // ただし数える材料は鎖の全部（review/code-r2.md の R1）。
    let rows = sources(pool, &named).await?;

    let mut out = Vec::with_capacity(targets.len());
    for ((named_source, subject), src) in targets.iter().zip(rows.iter()) {
        let name = &src.logical_source;
        let display_name = src.display_name.clone();
        let started_on = src.collection_started_on;
        let retired_on = src.retired_on;
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
                    let f = facts(pool, user, &src.chain, start, last_counted).await?;
                    // **分母から抜けた日は達成日にも数えない**（2 巡目 R4）——
                    // 達成日数が分母を超えるのを防ぐ。
                    //
                    // **退役した日より後も分母に入れない**（ST03 の R64 / 第 8 回 Q31）。
                    // 入れたままだと、退役してから窓が閉じるまでの日が毎日「未達」として
                    // 積まれ、**名前を分けただけで成功条件 1 が落ちる**。
                    let live = f
                        .iter()
                        .filter(|d| !d.stopped_full)
                        .filter(|d| !retired_on.is_some_and(|r| d.day >= r));
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
