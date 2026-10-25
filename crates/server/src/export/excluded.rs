// SPDX-License-Identifier: AGPL-3.0-only
//! 書き出しから除く範囲の SQL 断片（ST33 / design D5 / D5a / D5b）。
//!
//! **除く範囲の式はここの 1 か所だけ**。書き出しの本体・`GET /export/summary`・後に消した記録の件数（D17）は
//! 同じ断片を使う（件数を別の式で数えない）。断片は文字列で、`WITH` の後ろに置く CTE の並びと、
//! 記録・前の版の「残す」条件を返す。数値や時刻は束縛でなく、呼び出し側が渡す SQL の式で受ける。
//!
//! 緯度経度の判定は、記録の値（`payload` / `raw`）を DB の中で読むだけで、Rust の側へ持ち出さない。

/// 除く記録の id を集めた CTE の名前（`SELECT id FROM excluded_event`）。D17 が流す。
pub const EXCLUDED_EVENT: &str = "excluded_event";
/// 自分が条件を満たして除かれる前の版の id を集めた CTE の名前。**親が除かれたことによる除外は含まない**。
pub const EXCLUDED_VERSION: &str = "excluded_version";

/// 終わりの時刻を持つ内容の鍵（D5b (a)。順に読み、最初に読めたものを使う）。
/// `range_end` は `c02-window` の `idle` と `powered-off`（`collector-windows/src/contract.rs`）。
/// `end_time` の数字だけの値はミリ秒の UNIX 時刻（`stay_store::event_end_sql` と同じ）。
pub const END_KEYS: [&str; 6] = [
    "end",
    "end_time",
    "endTime",
    "endTimestamp",
    "end_at",
    "range_end",
];

/// 続いた時間を持つ内容の鍵と、その単位あたりのマイクロ秒（D5b (b)）。
/// **`elapsed_ms` は入れない** —— 携帯の時計の記録では起動からの経過で、続いた時間ではない。
pub const DURATION_KEYS: [(&str, i64); 3] = [
    // ST08 の Chromium の訪問
    ("visit_duration_us", 1),
    ("duration_ms", 1_000),
    ("duration_s", 1_000_000),
];

/// ソースごとの時刻の幅の読み方（D5b）。ここに無いソースは既定の (c)（次の記録まで）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanRule {
    /// 出来事 1 点（(d)。(a)(b) の鍵を持てばそちらが先に効く）。
    Point,
}

/// 「時刻 1 点」と決まったソース。**足すのは、そのソースの記録が出来事 1 点であることを収集側の型で確かめた Story だけ**
/// （理由を隣に書く）。足し忘れても (c) で次の記録までの幅になり、点として抜けることは無い（除き過ぎの側。C21）。
///
/// 書庫のソース（`c03-*`）は、ST12 の解析器の型で出来事 1 点と確かめたものが無いので足さない（(c) のまま）。
pub const SPAN_RULES: &[(&str, SpanRule)] = &[
    // 位置の 1 点（GPS の 1 回の測位）
    ("c01-location", SpanRule::Point),
    // 写真の位置・撮影の時刻（撮影は出来事 1 点）
    ("c01-photo", SpanRule::Point),
    // スクリーンショット・画像・動画も同じ（撮影・保存の 1 点。動画の長さは `duration_ms` の (b) が先に効く）
    ("c01-screenshot", SpanRule::Point),
    ("c01-image", SpanRule::Point),
    ("c01-video", SpanRule::Point),
    // 端末の時計の記録（読んだ時刻の 1 点）
    ("c01-clock", SpanRule::Point),
    // 健康データの権限の記録（観た時刻の 1 点。ST11 / design D12（仮））
    ("c01-health-permission", SpanRule::Point),
    // 個人属性（本人が書いた時点）
    ("s01-attribute", SpanRule::Point),
];

/// 幅を「次の記録まで」（(c)。(a)(b) の鍵を持てばそちらが先に効く）と**決めた**ソース。
/// **SQL はこれを読まない** —— `SPAN_RULES` に無いソースは (c) にする既定（C21。厳しい側）のまま。
/// 登録簿の全ソースが `SPAN_RULES` かこれのちょうど一方に置かれていることを試験が DB と突き合わせ（R7 / handoff C1）、
/// 一覧に無いソースが足されたら名前つきで落ちる。末尾が `-` のものは前置き（`ARCHIVE_PRODUCT_PREFIX`）。
pub const SPAN_NEXT_SOURCES: &[&str] = &[
    // 前景のアプリの切り替わり（次の変化まで続く。spec-review R34）と、その日ごとの集計（`end` で (a)）
    "c01-app-usage",
    "c01-app-usage-rollup",
    // 前景のウィンドウ（`range_end` を持つものは (a)）とブラウザの履歴（`visit_duration_us` を持つものは (b)）
    "c02-window",
    "c02-browser-history",
    // 滞在（`end` で (a)）
    "s01-stay",
    // 場所の記録（ST21）。本人が書いた時点の 1 点だと収集側の型で確かめていないので (c) のまま
    "s01-place",
    // 主観の記録（ST17）。紐づけ先（日・滞在）で幅を読む規則は持たず (c) のまま（handoff C1。緯度経度を持たない側なので、
    // 幅が効くのはひとことに座標の並びを書いた行だけ。除き過ぎの側）
    "s01-feeling",
    // 書庫の取り込み口の印（記録を持たない）
    "s01-archive-inbox",
    // 書庫の区間（`end_time` で (a)。`archive::INTERVAL_SOURCES`）
    "c03-timeline-visit",
    "c03-timeline-move",
    "c03-legacy-visit",
    "c03-legacy-activity",
    // 書庫の位置の点と項目。ST12 の解析器の型で出来事 1 点と確かめていないので (c) のまま（D5b）
    "c03-timeline-route",
    "c03-timeline-signal",
    "c03-legacy-location",
    "c03-youtube-watch",
    "c03-youtube-search",
    "c03-chrome-history",
    ARCHIVE_PRODUCT_PREFIX,
];

/// 書庫がマイアクティビティの製品ごとに登録簿へ足すソースの前置き（`archive/myactivity.rs`）。
/// 名前が書庫ごとに増えるので、一覧には前置きで置く。
pub const ARCHIVE_PRODUCT_PREFIX: &str = "c03-myactivity-";

/// 緯度経度を持たないと決まったソース（D5a (1)）。**ここに無いソースは緯度経度を持つとみなす**（既定は厳しい側）。
/// 足すのは、そのソースの payload に座標の項目が無いことを収集側の型で確かめた Story だけ（理由を隣に書く）。
/// `c01-location` / `s01-stay` / `c01-photo` / `c03-*` / `s01-place` は入れない。
pub const NO_COORD_SOURCES: [&str; 8] = [
    // 前景のアプリの名前と時間だけ
    "c01-app-usage",
    "c01-app-usage-rollup",
    // 端末の時計の記録
    "c01-clock",
    // 健康データの権限の記録（許可の有無と対応の有無だけ。ST11 / design D2）
    "c01-health-permission",
    // 前景のウィンドウの題名・アドレスバー（座標は文字列の中にだけ持ちうる。D5a (2)(3) が拾う）
    "c02-window",
    // ブラウザの履歴（URL。同上）
    "c02-browser-history",
    // 個人属性（本人が書いた時点と値）
    "s01-attribute",
    // 主観の記録（ST17。`feelings::parse_feeling` の内容は尺度・紐づけ先・ひとことだけ。ひとことの中の座標は D5a (3) が拾う）
    "s01-feeling",
];

/// 緯度経度を持つと**決めた**ソース（D5a (1) の反対側）。**SQL はこれを読まない** —— `NO_COORD_SOURCES` に無い
/// ソースは持つとみなす既定（厳しい側）のまま。登録簿の全ソースが `NO_COORD_SOURCES` かこれのちょうど一方に
/// 置かれていることを試験が DB と突き合わせ（R7 / handoff C1）、一覧に無いソースが足されたら名前つきで落ちる。
pub const COORD_SOURCES: &[&str] = &[
    // 位置の 1 点・写真の位置・滞在（payload に緯度経度）
    "c01-location",
    "c01-photo",
    "c01-screenshot",
    "c01-image",
    "c01-video",
    "s01-stay",
    // 場所の記録（ST21。payload に場所の中心の緯度経度）
    "s01-place",
    // 書庫の取り込み口の印（記録を持たない。確かめた Story が無いので持つ側）
    "s01-archive-inbox",
    // 書庫の位置と区間（`archive::LOCATION_SOURCES`）
    "c03-timeline-visit",
    "c03-timeline-move",
    "c03-timeline-route",
    "c03-timeline-signal",
    "c03-legacy-location",
    "c03-legacy-visit",
    "c03-legacy-activity",
    // 書庫の項目（`locationInfos` と URL の中の座標。FR-50 ★ 2026-10-13）
    "c03-youtube-watch",
    "c03-youtube-search",
    "c03-chrome-history",
    ARCHIVE_PRODUCT_PREFIX,
];

/// `list` の名前か前置き（末尾が `-`）に `source` が当たるか。
pub fn listed(list: &[&str], source: &str) -> bool {
    list.iter()
        .any(|l| *l == source || (l.ends_with('-') && source.starts_with(l)))
}

/// 除く範囲の断片。
#[derive(Debug, Clone)]
pub struct ExcludedSql {
    ctes: String,
}

impl ExcludedSql {
    /// `WITH` の後ろに置く CTE の並び（先頭に `WITH` も、末尾にカンマも付けない）。
    /// 呼び出し側の CTE は、これの後ろに `,` でつなぐ。
    pub fn ctes(&self) -> &str {
        &self.ctes
    }

    /// `core.event_live` の行 `alias` を**残す**条件。
    pub fn event_kept(&self, alias: &str) -> String {
        format!("NOT EXISTS (SELECT 1 FROM {EXCLUDED_EVENT} x WHERE x.id = {alias}.id)")
    }

    /// `core.event_version_live` の行 `alias` を**残す**条件。親が除かれるか、自分が条件を満たせば除く。
    pub fn version_kept(&self, alias: &str) -> String {
        format!(
            "NOT EXISTS (SELECT 1 FROM {EXCLUDED_EVENT} x WHERE x.id = {alias}.event_id)
             AND NOT EXISTS (SELECT 1 FROM {EXCLUDED_VERSION} x WHERE x.id = {alias}.id)"
        )
    }
}

/// 除く範囲の断片を組む。
///
/// `snapshot_at` は、次の記録が無いときに幅の終わりにする「写しの時点」の SQL の式（例 `now()`・`transaction_timestamp()`）。
/// 消した滞在は、その文が見る時点で本人が消してあるもの全部を集める。
///
/// 次の記録は、行ごとの相関副問い合わせでなく、鍵の列だけを並べた 1 つの窓（`span_next`）で一度に読む（CH02 D1〜D4）。
/// ST33 D5 は「次の記録は行ごとに索引で引く」と決めたが、120 万行で 14 分かかったので覆した（CH02）。
/// 次の記録は「同じ組（利用者・ソース・端末）で、時刻が厳密に後の最初の記録」で、同じ時刻の記録は飛ばす（CH02 Q1）。
pub fn excluded_sql(snapshot_at: &str) -> ExcludedSql {
    let user_deleted = format!(
        "(s.deleted_by IS NULL OR s.deleted_by NOT LIKE '{}%')",
        crate::stay_store::REBUILD_PREFIX
    );
    let events = "core.event_live";
    let versions = "(SELECT v.id, e.logical_source, v.event_time, v.payload, v.raw
                       FROM core.event_version_live v
                       JOIN core.event_live e ON e.id = v.event_id)";
    let overlap_event = overlap_sql(events, true, snapshot_at);
    let overlap_version = overlap_sql(versions, false, snapshot_at);
    // 窓に載せるのは鍵の列だけ（`payload` / `raw` は載せず、窓の後に `id` で戻す。D2）。
    // 消した滞在が 0 件なら、各枝の `WHERE EXISTS` が One-Time Filter になって表を読まない（D3）。
    // 窓には緯度経度で絞る前の全部を、時刻が `max(end_at)` 以下の行と、組ごとにそれより後の最初の記録 1 行（候補としてだけ。`id` は無い）で入れる。
    let ctes = format!(
        "erased_spans AS MATERIALIZED (
           SELECT s.start_at, s.end_at
             FROM core.stay_erased s
            WHERE {user_deleted}
           UNION ALL
           SELECT v.event_time, coalesce(core.try_timestamptz(v.payload->>'end'), v.event_time)
             FROM core.stay_erased s
             JOIN core.event_version v ON v.event_id = s.id
            WHERE {user_deleted}
         ),
         erased_range AS MATERIALIZED (
           SELECT range_agg(tstzrange(s.start_at, s.end_at, '[]')) AS spans
             FROM erased_spans s
            WHERE s.start_at <= s.end_at
         ),
         erased_odd AS MATERIALIZED (
           SELECT s.start_at, s.end_at FROM erased_spans s WHERE s.start_at > s.end_at
         ),
         span_next AS MATERIALIZED (
           SELECT w.id, w.is_event, w.next_time
             FROM (
               SELECT q.id, q.is_event, q.event_time,
                      min(q.event_time) FILTER (WHERE q.is_event) OVER (
                        PARTITION BY q.user_id, q.logical_source, q.device_id
                        ORDER BY q.event_time DESC
                        RANGE BETWEEN UNBOUNDED PRECEDING AND '1 microsecond' PRECEDING
                      ) AS next_time
                 FROM (
                   SELECT e.id, e.user_id, e.logical_source, e.device_id, e.event_time, true AS is_event
                     FROM core.event_live e
                    WHERE EXISTS (SELECT 1 FROM erased_spans)
                      AND e.event_time <= (SELECT max(end_at) FROM erased_spans)
                   UNION ALL
                   SELECT NULL::uuid, e.user_id, e.logical_source, e.device_id, min(e.event_time), true
                     FROM core.event_live e
                    WHERE EXISTS (SELECT 1 FROM erased_spans)
                      AND e.event_time > (SELECT max(end_at) FROM erased_spans)
                    GROUP BY e.user_id, e.logical_source, e.device_id
                   UNION ALL
                   SELECT v.id, e.user_id, e.logical_source, e.device_id, v.event_time, false
                     FROM core.event_version_live v
                     JOIN core.event_live e ON e.id = v.event_id
                    WHERE EXISTS (SELECT 1 FROM erased_spans)
                      AND v.event_time <= (SELECT max(end_at) FROM erased_spans)
                 ) q
             ) w
            WHERE w.id IS NOT NULL
         ),
         overlap_event AS MATERIALIZED ({overlap_event}),
         {EXCLUDED_EVENT} AS MATERIALIZED (
           SELECT o.id FROM overlap_event o
         ),
         overlap_version AS MATERIALIZED ({overlap_version}),
         {EXCLUDED_VERSION} AS MATERIALIZED (
           SELECT o.id FROM overlap_version o
         )"
    );
    ExcludedSql { ctes }
}

/// 幅が消した時間のどれかと重なる、の条件（`overlap_sql` の最後の `WHERE`。`w` は幅を求めた行。CH04 / design D9）。
///
/// 候補の行 × 消した時間の入れ子を取るため、3 つの和にする:
/// 1. 整った幅（`event_time <= width_end`）の行は、整った消した時間を 1 つの `tstzmultirange` にした `erased_range` と `&&` で 1 回で引く。
///    **幅の範囲は `CASE` の中でだけ作る**（`tstzrange` は始まりが終わりより後だとエラーで、AND は評価の順を約束しない）。
/// 2. 整っていない消した時間（`erased_odd`。`start_at > end_at`）は、いまの `EXISTS` の式のまま全部の行に当てる（件数は小さい）。
/// 3. 整っていない幅の行は、いまの `EXISTS` の式のまま全部の消した時間に当てる。
///
/// 端は閉じた区間（`'[]'`）。`start_at` か `end_at` が NULL の消した時間は、どれにも重ならない（CH02 の式と同じ）。
/// 試験（`export_overlap_tests`）がこの文字列を CH02 の `EXISTS` に置き換えて新旧を突き合わせるので、`overlap_sql` の中で 1 度だけ使う。
pub(crate) const OVERLAP_CONDITION: &str = "(
             CASE WHEN w.event_time <= w.width_end
                  THEN (SELECT m.spans FROM erased_range m) && tstzrange(w.event_time, w.width_end, '[]')
             END
             OR EXISTS (SELECT 1 FROM erased_odd s WHERE s.start_at <= w.width_end AND s.end_at >= w.event_time)
             OR (w.width_end < w.event_time
                 AND EXISTS (SELECT 1 FROM erased_spans s WHERE s.start_at <= w.width_end AND s.end_at >= w.event_time))
           )";

/// 行 `r`（`id` `logical_source` `event_time` `payload` `raw` を持つ）のうち、
/// 時刻の幅が `erased_spans` のどれかと重なるものの `id` `logical_source` `payload` `raw`（D5b）。
///
/// 次の記録の時刻は `span_next` から `id` で引く（`is_event` は記録の側が true、前の版の側が false）。
/// 組の鍵（利用者・端末）は `span_next` が持ち、ここでは読まない。前の版の `rows` が親の記録を JOIN するのは、
/// 親が生きている前の版に絞るのと、`logical_source` を親から取るため。
/// 消した時間より後に始まる行は先に落とす。
/// `span_next` も消した滞在が 0 件の門と `max(end_at)` 以下の絞りを持つので、ここの 2 つの条件は結果を変えない。
/// 門を重ねて置く（どちらか片方を外しても、消した滞在 0 件で記録の表を読まない。CH02 の code_verify の M1 / M1b）。
fn overlap_sql(rows: &str, is_event: bool, snapshot_at: &str) -> String {
    let end_keys: Vec<String> = END_KEYS.iter().map(|k| end_key_sql(k)).collect();
    let dur_keys: Vec<String> = DURATION_KEYS
        .iter()
        .map(|(k, us)| duration_key_sql(k, *us))
        .collect();
    let points = SPAN_RULES
        .iter()
        .filter(|(_, rule)| *rule == SpanRule::Point)
        .map(|(s, _)| format!("'{s}'"))
        .collect::<Vec<_>>()
        .join(", ");
    let coord = has_coordinates_sql("r.logical_source", "r.payload", "r.raw");
    // 幅の計算は行ごとに 1 回だけ。内側の副問い合わせは `OFFSET 0` で畳み込みを止める
    // （畳むと重なりの判定のたびに、消した時間の数だけ幅を計算し直す）。緯度経度を持たない行は幅を求めずに落とす。
    format!(
        "SELECT w.id, w.logical_source, w.payload, w.raw
           FROM (
             SELECT r.id, r.logical_source, r.payload, r.raw, r.event_time,
                    coalesce(a.t, b.t, c.t) AS width_end
               FROM span_next k
               JOIN {rows} r ON r.id = k.id
              CROSS JOIN LATERAL (SELECT coalesce({ends}) AS t) ka
              CROSS JOIN LATERAL (SELECT CASE WHEN ka.t >= r.event_time THEN ka.t END AS t) a
              CROSS JOIN LATERAL (SELECT coalesce({durs}) AS t) b
              CROSS JOIN LATERAL (
                SELECT CASE
                         WHEN a.t IS NOT NULL OR b.t IS NOT NULL THEN NULL
                         WHEN r.logical_source IN ({points}) THEN r.event_time
                         ELSE coalesce(k.next_time, {snapshot_at})
                       END AS t
              ) c
              WHERE k.is_event = {is_event}
                AND EXISTS (SELECT 1 FROM erased_spans)
                AND r.event_time <= (SELECT max(end_at) FROM erased_spans)
                AND {coord}
             OFFSET 0
           ) w
          WHERE {OVERLAP_CONDITION}",
        ends = end_keys.join(", "),
        durs = dur_keys.join(", "),
    )
}

/// 終わりの鍵 1 つの読み方。`end_time` だけは数字だけの値をミリ秒の UNIX 時刻として読む。
fn end_key_sql(key: &str) -> String {
    let text = format!("r.payload->>'{key}'");
    let plain = format!("core.try_timestamptz({text})");
    if key == "end_time" {
        format!(
            "coalesce({plain}, CASE WHEN {text} ~ '^[0-9]{{1,15}}$'
                                   THEN to_timestamp(({text})::numeric / 1000.0) END)"
        )
    } else {
        plain
    }
}

/// 長さの鍵 1 つを `event_time` に足した時刻。数でない・負・大きすぎる値は読めないもの（NULL）にする。
fn duration_key_sql(key: &str, micros_per_unit: i64) -> String {
    let text = format!("r.payload->>'{key}'");
    format!(
        "CASE WHEN {text} ~ '^[0-9]{{1,12}}(\\.[0-9]+)?$'
              THEN r.event_time + (({text})::float8 * {micros_per_unit}) * interval '1 microsecond' END"
    )
}

/// 「緯度経度を持つ」の判定（D5a (1)〜(3)）。引数は `logical_source` `payload` `raw` を指す SQL の式。
pub fn has_coordinates_sql(source: &str, payload: &str, raw: &str) -> String {
    let sources = NO_COORD_SOURCES
        .iter()
        .map(|s| format!("'{s}'"))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "({source} NOT IN ({sources}) OR {} OR {})",
        coord_in_payload_sql(payload),
        coord_in_raw_sql(raw)
    )
}

/// (2) 内容のどの深さにでも、緯度か経度を表す名前の項目がある。
/// オブジェクトに絞ってから `keyvalue()` を当てる（スカラーや配列に当てると DB がエラーにする）。
/// **`silent` で黙らせない**（黙らせるとエラーの行が「持たない」になり、漏れる側に倒れる）。
pub fn coord_in_payload_sql(payload: &str) -> String {
    format!(
        r#"jsonb_path_exists({payload}, 'strict $.** ? (@.type() == "object").keyvalue() ? (@.key like_regex "^(lat|lon|lng|latitude|longitude)(e7)?$" flag "i")')"#
    )
}

/// (3) 原文の中に、緯度と経度の組に読める並びがある。`l` は小文字にした原文、`n` は `l` の URL の符号化を戻したもの。
/// 形を足すのは下流で決めてよく、**狭める（いま座標と読むものを読まなくする）のは本人に返す**（C20）。
pub fn coord_in_raw_sql(raw: &str) -> String {
    let l = format!("lower({raw})");
    let n = format!(
        "replace(replace(replace(replace({l}, '%2c', ','), '%20', ' '), '+', ' '), '%c2%b0', '°')"
    );
    format!(
        r#"({n} ~ '(^|[^0-9.])-?\d{{1,3}}\.\d+\s*°?\s*[,/~ ]\s*-?\d{{1,3}}\.\d+'
          OR {n} ~ '!3d-?\d+(\.\d+)?!4d-?\d+(\.\d+)?'
          OR {n} ~ '!2d-?\d+(\.\d+)?!3d-?\d+(\.\d+)?'
          OR {n} ~ '\d{{1,3}}°\s*\d{{1,2}}[''′]'
          OR {n} ~ '(^|[^a-z])(lat|latitude|lon|lng|longitude)"?\s*[=:]\s*"?-?\d{{1,3}}(\.\d+)?'
          OR {l} ~ '(^|[^0-9a-z])[23456789cfghjmpqrvwx]{{4,8}}\+[23456789cfghjmpqrvwx]{{2,3}}($|[^0-9a-z])')"#
    )
}

/// CH02 の前の `excluded_sql` / `overlap_sql` の写し（試験専用）。新旧の除く集合を突き合わせるために残す（CH02 Q2）。
///
/// 写した日: 2026-10-21。元: `crates/server/src/export/excluded.rs` を最後に変えた commit
/// `ab0806401572f3a46a8c7f73e0982f32ae4d33dd`（HEAD 43c330b1 の時点）。関数の本体は逐語。
/// `excluded_sql_strict` だけ、`excluded_sql` の断片の記録の側の条件を置き換えて作る。
#[cfg(test)]
pub(crate) mod legacy {
    use super::*;

    /// 除く範囲の断片を組む。
    ///
    /// `snapshot_at` は、次の記録が無いときに幅の終わりにする「写しの時点」の SQL の式（例 `now()`・`transaction_timestamp()`）。
    /// 消した滞在は、その文が見る時点で本人が消してあるもの全部を集める。
    pub fn excluded_sql(snapshot_at: &str) -> ExcludedSql {
        let user_deleted = format!(
            "(s.deleted_by IS NULL OR s.deleted_by NOT LIKE '{}%')",
            crate::stay_store::REBUILD_PREFIX
        );
        let events = "core.event_live";
        let versions = "(SELECT v.id, v.event_id, e.user_id, e.logical_source, e.device_id,
                                v.event_time, v.payload, v.raw
                           FROM core.event_version_live v
                           JOIN core.event_live e ON e.id = v.event_id)";
        let overlap_event = overlap_sql(
            events,
            "(n.event_time, n.id) > (r.event_time, r.id)",
            snapshot_at,
        );
        let overlap_version = overlap_sql(versions, "n.event_time > r.event_time", snapshot_at);
        let ctes = format!(
            "erased_spans AS MATERIALIZED (
               SELECT s.start_at, s.end_at
                 FROM core.stay_erased s
                WHERE {user_deleted}
               UNION ALL
               SELECT v.event_time, coalesce(core.try_timestamptz(v.payload->>'end'), v.event_time)
                 FROM core.stay_erased s
                 JOIN core.event_version v ON v.event_id = s.id
                WHERE {user_deleted}
             ),
             overlap_event AS MATERIALIZED ({overlap_event}),
             {EXCLUDED_EVENT} AS MATERIALIZED (
               SELECT o.id FROM overlap_event o
             ),
             overlap_version AS MATERIALIZED ({overlap_version}),
             {EXCLUDED_VERSION} AS MATERIALIZED (
               SELECT o.id FROM overlap_version o
             )"
        );
        ExcludedSql { ctes }
    }

    /// `excluded_sql` の記録の側の条件だけを `n.event_time > r.event_time` にしたもの（同じ時刻の記録を飛ばす形。CH02 Q1）。
    ///
    /// 本体は `excluded_sql` の断片の、記録の側の条件（版の側とは文字列が違う）を 1 か所だけ置き換えて作る。
    pub fn excluded_sql_strict(snapshot_at: &str) -> ExcludedSql {
        const EVENT_NEXT_AFTER: &str = "(n.event_time, n.id) > (r.event_time, r.id)";
        let ctes = excluded_sql(snapshot_at).ctes;
        assert_eq!(ctes.matches(EVENT_NEXT_AFTER).count(), 1);
        ExcludedSql {
            ctes: ctes.replace(EVENT_NEXT_AFTER, "n.event_time > r.event_time"),
        }
    }

    /// 行 `r`（`id` `user_id` `logical_source` `device_id` `event_time` `payload` `raw` を持つ）のうち、
    /// 時刻の幅が `erased_spans` のどれかと重なるものの `id` `logical_source` `payload` `raw`（D5b）。
    ///
    /// 消した時間より後に始まる行は先に落とす（写しは約 120 万行で、次の記録の引きは重なりうる行にだけ要る）。
    /// `next_after` は次の記録の条件（`n` が次の記録の候補、`r` が自分）。
    fn overlap_sql(rows: &str, next_after: &str, snapshot_at: &str) -> String {
        let end_keys: Vec<String> = END_KEYS.iter().map(|k| end_key_sql(k)).collect();
        let dur_keys: Vec<String> = DURATION_KEYS
            .iter()
            .map(|(k, us)| duration_key_sql(k, *us))
            .collect();
        let points = SPAN_RULES
            .iter()
            .filter(|(_, rule)| *rule == SpanRule::Point)
            .map(|(s, _)| format!("'{s}'"))
            .collect::<Vec<_>>()
            .join(", ");
        let coord = has_coordinates_sql("r.logical_source", "r.payload", "r.raw");
        // 幅の計算（次の記録の引き）は行ごとに 1 回だけ。内側の副問い合わせは `OFFSET 0` で畳み込みを止める
        // （畳むと重なりの判定のたびに、消した時間の数だけ次の記録を引き直す）。緯度経度を持たない行は幅を求めずに落とす。
        format!(
            "SELECT w.id, w.logical_source, w.payload, w.raw
               FROM (
                 SELECT r.id, r.logical_source, r.payload, r.raw, r.event_time,
                        coalesce(a.t, b.t, c.t) AS width_end
                   FROM {rows} r
                  CROSS JOIN LATERAL (SELECT coalesce({ends}) AS t) ka
                  CROSS JOIN LATERAL (SELECT CASE WHEN ka.t >= r.event_time THEN ka.t END AS t) a
                  CROSS JOIN LATERAL (SELECT coalesce({durs}) AS t) b
                  CROSS JOIN LATERAL (
                    SELECT CASE
                             WHEN a.t IS NOT NULL OR b.t IS NOT NULL THEN NULL
                             WHEN r.logical_source IN ({points}) THEN r.event_time
                             ELSE coalesce(
                               (SELECT n.event_time FROM core.event_live n
                                 WHERE n.user_id = r.user_id AND n.logical_source = r.logical_source
                                   AND n.device_id IS NOT DISTINCT FROM r.device_id
                                   AND {next_after}
                                 ORDER BY n.event_time, n.id LIMIT 1),
                               {snapshot_at})
                           END AS t
                  ) c
                  WHERE r.event_time <= (SELECT max(end_at) FROM erased_spans)
                    AND {coord}
                 OFFSET 0
               ) w
              WHERE EXISTS (SELECT 1 FROM erased_spans s
                             WHERE s.start_at <= w.width_end
                               AND s.end_at >= w.event_time)",
            ends = end_keys.join(", "),
            durs = dur_keys.join(", "),
        )
    }
}
