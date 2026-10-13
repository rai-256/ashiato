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
    // 端末の時計の記録（読んだ時刻の 1 点）
    ("c01-clock", SpanRule::Point),
    // 個人属性（本人が書いた時点）
    ("s01-attribute", SpanRule::Point),
];

/// 緯度経度を持たないと決まったソース（D5a (1)）。**ここに無いソースは緯度経度を持つとみなす**（既定は厳しい側）。
/// 足すのは、そのソースの payload に座標の項目が無いことを収集側の型で確かめた Story だけ（理由を隣に書く）。
/// `c01-location` / `s01-stay` / `c01-photo` / `c03-*` / `s01-place` は入れない。
pub const NO_COORD_SOURCES: [&str; 6] = [
    // 前景のアプリの名前と時間だけ
    "c01-app-usage",
    "c01-app-usage-rollup",
    // 端末の時計の記録
    "c01-clock",
    // 前景のウィンドウの題名・アドレスバー（座標は文字列の中にだけ持ちうる。D5a (2)(3) が拾う）
    "c02-window",
    // ブラウザの履歴（URL。同上）
    "c02-browser-history",
    // 個人属性（本人が書いた時点と値）
    "s01-attribute",
];

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
/// - `snapshot_at`: 次の記録が無いときに幅の終わりにする「写しの時点」の SQL の式（例 `now()`・`$1::timestamptz`）。
/// - `erased_as_of`: 消した滞在を集める時点の SQL の式。`Some` なら `deleted_at` がその時点以前のものだけ
///   （D17 が写しの時点 `T` で使う）。`None` なら、いま消してあるもの全部。
pub fn excluded_sql(snapshot_at: &str, erased_as_of: Option<&str>) -> ExcludedSql {
    let as_of = erased_as_of
        .map(|t| format!("AND s.deleted_at <= {t}"))
        .unwrap_or_default();
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
            WHERE {user_deleted} {as_of}
           UNION ALL
           SELECT v.event_time, coalesce(core.try_timestamptz(v.payload->>'end'), v.event_time)
             FROM core.stay_erased s
             JOIN core.event_version v ON v.event_id = s.id
            WHERE {user_deleted} {as_of}
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
