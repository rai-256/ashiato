// SPDX-License-Identifier: AGPL-3.0-only
//! 途切れの判定の材料（`outage_store::inputs`）を、本物の PostgreSQL に記録と生存信号を置いて見る
//! （ST14 Task 5 / design D3・D4）。`inputs` → `judge` の結果で確かめる。
//!
//! 観測ではない種類の組は `(logical_source, payload->>'kind')` の完全一致なので、その組の試験は登録簿の
//! 本物の名前（`c02-window` / `c01-app-usage`）を使う。利用者は試験ごとに固有。共有の登録簿の行は書き換えず、
//! 収集開始日は試験の側で `SourceInput` に置く。
#![allow(clippy::unwrap_used)]

use crate::outage::{judge, Outage, SourceInput, NON_OBSERVATION_ALERT};
use crate::outage_store::inputs;
use crate::testdb;
use chrono::{DateTime, Duration, Utc};
use serde_json::json;

const WINDOW: &str = "c02-window";
const USAGE: &str = "c01-app-usage";
const SIX_HOURS: i32 = 21_600;

fn rfc(now: DateTime<Utc>, ago: Duration) -> String {
    (now - ago).to_rfc3339()
}

/// 記録を直に置く。`ingest` が `None` なら DB の既定（いま）。
async fn put(
    pool: &sqlx::PgPool,
    user: uuid::Uuid,
    source: &str,
    origin: &str,
    event_time: &str,
    ingest: Option<&str>,
    payload: serde_json::Value,
) {
    sqlx::query(
        "INSERT INTO core.event
           (id, user_id, logical_source, device_id, origin, event_time, ingest_time,
            tz_offset_min, tz_id, schema_version, content_hash, raw, payload)
         VALUES ($1,$2,$3,'test',$4,$5::timestamptz,coalesce($6::timestamptz, now()),
                 540,'Asia/Tokyo',1,$7,'{}',$8)",
    )
    .bind(uuid::Uuid::new_v4())
    .bind(user)
    .bind(source)
    .bind(origin)
    .bind(event_time)
    .bind(ingest)
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(payload)
    .execute(pool)
    .await
    .unwrap();
}

async fn put_collected(
    pool: &sqlx::PgPool,
    user: uuid::Uuid,
    source: &str,
    event_time: &str,
    payload: serde_json::Value,
) {
    put(pool, user, source, "collected", event_time, None, payload).await;
}

/// 試験ごとに固有の位置のソース（想定間隔 6 時間・収集開始済み）。
async fn location(pool: &sqlx::PgPool) -> String {
    let name = testdb::source(pool, "outage-loc", SIX_HOURS).await;
    testdb::set_started_on(pool, &name, "2026-01-01").await;
    name
}

/// `inputs` の中から 1 本取る。`started` を渡すと収集開始日をその日に置き換える（共有の登録簿の行を書き換えない）。
async fn input_of(
    pool: &sqlx::PgPool,
    user: uuid::Uuid,
    now: DateTime<Utc>,
    name: &str,
    started: Option<&str>,
) -> SourceInput {
    let mut s = inputs(pool, user, now)
        .await
        .unwrap()
        .into_iter()
        .find(|s| s.logical_source == name)
        .unwrap_or_else(|| panic!("{name} が inputs に無い"));
    if let Some(day) = started {
        s.collection_started_on = Some(testdb::date(day));
    }
    s
}

async fn outage_of(
    pool: &sqlx::PgPool,
    user: uuid::Uuid,
    now: DateTime<Utc>,
    name: &str,
    started: Option<&str>,
) -> Option<Outage> {
    let s = input_of(pool, user, now, name, started).await;
    judge(&[s], now).into_iter().next()
}

// Scenario: 時計の測定だけが届き続けても途切れになる
#[tokio::test]
async fn st14_inputs_device_clock_only_still_outage() {
    let pool = testdb::pool().await;
    let (u, now) = (testdb::user(), Utc::now());
    put_collected(
        &pool,
        u,
        WINDOW,
        &rfc(now, Duration::hours(19)),
        json!({"kind": "foreground", "app_name": "x"}),
    )
    .await;
    testdb::put_heartbeat(&pool, u, WINDOW, &rfc(now, Duration::hours(19)), true).await;
    for h in (0..19).rev() {
        put_collected(
            &pool,
            u,
            WINDOW,
            &rfc(now, Duration::hours(h) + Duration::minutes(5)),
            json!({"kind": "clock-skew"}),
        )
        .await;
    }
    let o = outage_of(&pool, u, now, WINDOW, Some("2026-01-01")).await;
    let o = o.expect("時計の測定だけで途切れが隠れた");
    assert_eq!(
        o.last_alive_at.timestamp(),
        (now - Duration::hours(19)).timestamp()
    );
}

// Scenario: 取りこぼしの期間の主張は生きていた時刻に数えない
#[tokio::test]
async fn st14_inputs_device_gap_claim_is_not_alive() {
    let pool = testdb::pool().await;
    let (u, now) = (testdb::user(), Utc::now());
    put_collected(
        &pool,
        u,
        USAGE,
        &rfc(now, Duration::hours(19)),
        json!({"kind": "event", "app_name": "x"}),
    )
    .await;
    testdb::put_heartbeat(&pool, u, USAGE, &rfc(now, Duration::hours(19)), true).await;
    put_collected(
        &pool,
        u,
        USAGE,
        &rfc(now, Duration::hours(1)),
        json!({"kind": "gap", "begin": rfc(now, Duration::hours(10)), "end": rfc(now, Duration::hours(1)), "reason": "x"}),
    )
    .await;
    assert!(outage_of(&pool, u, now, USAGE, Some("2026-01-01"))
        .await
        .is_some());
}

// Scenario: 離席と停止と除外の印は生きていた時刻に数えない
#[tokio::test]
async fn st14_inputs_device_idle_powered_off_excluded_are_not_alive() {
    let pool = testdb::pool().await;
    let (u, now) = (testdb::user(), Utc::now());
    put_collected(
        &pool,
        u,
        WINDOW,
        &rfc(now, Duration::hours(19)),
        json!({"kind": "foreground", "app_name": "x"}),
    )
    .await;
    testdb::put_heartbeat(&pool, u, WINDOW, &rfc(now, Duration::hours(19)), true).await;
    for kind in ["idle", "powered-off", "excluded"] {
        put_collected(
            &pool,
            u,
            WINDOW,
            &rfc(now, Duration::hours(1)),
            json!({"kind": kind}),
        )
        .await;
    }
    assert!(outage_of(&pool, u, now, WINDOW, Some("2026-01-01"))
        .await
        .is_some());
}

// Scenario: 溜めてから送った記録は出来事の時刻で測る
#[tokio::test]
async fn st14_inputs_device_backlogged_record_is_measured_by_event_time() {
    let pool = testdb::pool().await;
    let (u, now) = (testdb::user(), Utc::now());
    let name = location(&pool).await;
    put(
        &pool,
        u,
        &name,
        "collected",
        &rfc(now, Duration::hours(20)),
        Some(&rfc(now, Duration::minutes(1))),
        json!({}),
    )
    .await;
    let o = outage_of(&pool, u, now, &name, None).await.expect("途切れ");
    assert_eq!(
        o.last_alive_at.timestamp(),
        (now - Duration::hours(20)).timestamp()
    );
}

// Scenario: 未来の時刻の記録は途切れを隠さない
#[tokio::test]
async fn st14_inputs_device_future_event_time_does_not_hide_outage() {
    let pool = testdb::pool().await;
    let (u, now) = (testdb::user(), Utc::now());
    let name = location(&pool).await;
    put_collected(&pool, u, &name, &rfc(now, Duration::hours(19)), json!({})).await;
    testdb::put_heartbeat(&pool, u, &name, &rfc(now, Duration::hours(19)), true).await;
    // 出来事の時刻が 1 年後で、19 時間前に届いた
    put(
        &pool,
        u,
        &name,
        "collected",
        &(now + Duration::days(365)).to_rfc3339(),
        Some(&rfc(now, Duration::hours(19))),
        json!({}),
    )
    .await;
    let o = outage_of(&pool, u, now, &name, None).await.expect("途切れ");
    assert_eq!(
        o.last_alive_at.timestamp(),
        (now - Duration::hours(19)).timestamp()
    );
    // 受け取った時刻が新しい未来の記録は、その受け取った時刻で生きていたと数える
    let name2 = location(&pool).await;
    put(
        &pool,
        u,
        &name2,
        "collected",
        &(now + Duration::days(365)).to_rfc3339(),
        Some(&rfc(now, Duration::hours(1))),
        json!({}),
    )
    .await;
    assert!(outage_of(&pool, u, now, &name2, None).await.is_none());
}

// 未来の発信時刻の生存信号は受け取った時刻で数える（下の Scenario の生存信号の側）
// Scenario: 未来の時刻の記録は途切れを隠さない
#[tokio::test]
async fn st14_inputs_device_future_heartbeat_uses_received_at() {
    let pool = testdb::pool().await;
    let (u, now) = (testdb::user(), Utc::now());
    let name = location(&pool).await;
    put_collected(&pool, u, &name, &rfc(now, Duration::hours(19)), json!({})).await;
    // 発信の時刻が未来（時計の狂った端末）。受け取りは 1 分前なら、その時刻で生きていた
    let s = input_after_future_heartbeat(&pool, u, now, &name, Duration::minutes(1)).await;
    assert_eq!(
        s.last_capturable_heartbeat_at.unwrap().timestamp(),
        (now - Duration::minutes(1)).timestamp()
    );
    assert!(judge(&[s], now).is_empty());
}

async fn input_after_future_heartbeat(
    pool: &sqlx::PgPool,
    user: uuid::Uuid,
    now: DateTime<Utc>,
    name: &str,
    received_ago: Duration,
) -> SourceInput {
    // 生存信号は書き換えられないので、受け取った時刻を指定して直に入れる
    sqlx::query(
        "INSERT INTO core.heartbeat
           (id, user_id, logical_source, device_id, emitted_at, received_at, capturable, blockers,
            attempts, successes, content_hash, raw)
         VALUES ($1,$2,$3,'test',$4::timestamptz,$5::timestamptz,true,'{}',1,1,$6,'{}')",
    )
    .bind(uuid::Uuid::new_v4())
    .bind(user)
    .bind(name)
    .bind((now + Duration::days(365)).to_rfc3339())
    .bind(rfc(now, received_ago))
    .bind(uuid::Uuid::new_v4().to_string())
    .execute(pool)
    .await
    .unwrap();
    input_of(pool, user, now, name, None).await
}

// Scenario: 本人が書いた記録のソースは印が無くても途切れにならない
#[tokio::test]
async fn st14_inputs_device_authored_source_is_not_outage() {
    let pool = testdb::pool().await;
    let (u, now) = (testdb::user(), Utc::now());
    let name = location(&pool).await;
    put(
        &pool,
        u,
        &name,
        "authored",
        &rfc(now, Duration::days(5)),
        None,
        json!({}),
    )
    .await;
    let s = input_of(&pool, u, now, &name, None).await;
    assert!(!s.has_collected);
    assert!(s.has_non_collected);
    assert!(judge(&[s], now).is_empty());
    // 収集した記録が 1 件混ざれば対象のまま
    let name2 = location(&pool).await;
    put(
        &pool,
        u,
        &name2,
        "authored",
        &rfc(now, Duration::days(5)),
        None,
        json!({}),
    )
    .await;
    put_collected(&pool, u, &name2, &rfc(now, Duration::days(5)), json!({})).await;
    let s = input_of(&pool, u, now, &name2, None).await;
    assert!(s.has_collected && s.has_non_collected);
    assert!(!judge(&[s], now).is_empty());
}

// Scenario: 取れない状態の生存信号だけが届き続けると途切れになる
#[tokio::test]
async fn st14_inputs_device_uncapturable_heartbeats_do_not_count() {
    let pool = testdb::pool().await;
    let (u, now) = (testdb::user(), Utc::now());
    let name = location(&pool).await;
    put_collected(&pool, u, &name, &rfc(now, Duration::hours(19)), json!({})).await;
    testdb::put_heartbeat(&pool, u, &name, &rfc(now, Duration::hours(19)), true).await;
    for h in [13, 7, 1] {
        testdb::put_heartbeat(&pool, u, &name, &rfc(now, Duration::hours(h)), false).await;
    }
    let o = outage_of(&pool, u, now, &name, None).await.expect("途切れ");
    assert_eq!(
        o.last_alive_at.timestamp(),
        (now - Duration::hours(19)).timestamp()
    );
}

// Scenario: 削除済みの記録も生きていた時刻に数える
#[tokio::test]
async fn st14_inputs_device_deleted_record_still_counts() {
    let pool = testdb::pool().await;
    let (u, now) = (testdb::user(), Utc::now());
    let name = location(&pool).await;
    testdb::put_deleted_event(&pool, u, &name, &rfc(now, Duration::hours(1))).await;
    assert!(outage_of(&pool, u, now, &name, None).await.is_none());
}

// Scenario: 破棄の報告は生きていた時刻に数えない
#[tokio::test]
async fn st14_inputs_device_drop_report_is_not_alive() {
    let pool = testdb::pool().await;
    let (u, now) = (testdb::user(), Utc::now());
    let name = location(&pool).await;
    put_collected(&pool, u, &name, &rfc(now, Duration::hours(19)), json!({})).await;
    testdb::put_heartbeat(&pool, u, &name, &rfc(now, Duration::hours(19)), true).await;
    testdb::put_drop(
        &pool,
        u,
        &name,
        Some((
            &rfc(now, Duration::hours(30)),
            &rfc(now, Duration::hours(25)),
        )),
        &[],
    )
    .await;
    assert!(outage_of(&pool, u, now, &name, None).await.is_some());
}

// Scenario: 破棄の報告だけでは途切れにならない
#[tokio::test]
async fn st14_inputs_device_drop_report_alone_is_not_outage() {
    let pool = testdb::pool().await;
    let (u, now) = (testdb::user(), Utc::now());
    let name = location(&pool).await;
    put_collected(&pool, u, &name, &rfc(now, Duration::hours(1)), json!({})).await;
    testdb::put_drop(
        &pool,
        u,
        &name,
        Some((
            &rfc(now, Duration::hours(30)),
            &rfc(now, Duration::hours(25)),
        )),
        &[],
    )
    .await;
    assert!(outage_of(&pool, u, now, &name, None).await.is_none());
}

// Scenario: 停止している間は途切れにならない
#[tokio::test]
async fn st14_inputs_device_stop_covering_now_and_last_stop_end() {
    let pool = testdb::pool().await;
    let (u, now) = (testdb::user(), Utc::now());
    let name = location(&pool).await;
    put_collected(&pool, u, &name, &rfc(now, Duration::days(3)), json!({})).await;
    testdb::put_span(
        &pool,
        u,
        &name,
        "stopped",
        &rfc(now, Duration::days(2)),
        None,
    )
    .await;
    let s = input_of(&pool, u, now, &name, None).await;
    assert!(s.stopped_now);
    assert!(judge(&[s], now).is_empty());
    // 停止が 2 時間前に終わった
    // Scenario: 停止が終わった時刻から数え直す
    let name2 = location(&pool).await;
    put_collected(&pool, u, &name2, &rfc(now, Duration::days(3)), json!({})).await;
    testdb::put_span(
        &pool,
        u,
        &name2,
        "stopped",
        &rfc(now, Duration::days(2)),
        Some(&rfc(now, Duration::hours(2))),
    )
    .await;
    let s = input_of(&pool, u, now, &name2, None).await;
    assert!(!s.stopped_now);
    assert_eq!(
        s.last_stop_end.unwrap().timestamp(),
        (now - Duration::hours(2)).timestamp()
    );
    assert!(judge(&[s], now).is_empty());
}

// Scenario: 引き継いだソースは届く前から判定の対象になる
#[tokio::test]
async fn st14_inputs_device_successor_inherits_start_and_floor() {
    let pool = testdb::pool().await;
    let (u, now) = (testdb::user(), Utc::now());
    let old = location(&pool).await;
    let new = testdb::source(&pool, "outage-next", SIX_HOURS).await;
    testdb::set_succeeds(&pool, &new, &old).await;
    // 新しい名前は自分の開始日を持たないが、鎖の根から継ぐ。引き継いだ日（行ができた時刻）が下限
    sqlx::query("UPDATE core.source SET registered_at = $2::timestamptz WHERE logical_source = $1")
        .bind(&new)
        .bind(rfc(now, Duration::hours(19)))
        .execute(&pool)
        .await
        .unwrap();
    let s = input_of(&pool, u, now, &new, None).await;
    assert_eq!(s.collection_started_on, Some(testdb::date("2026-01-01")));
    let o = judge(&[s], now).into_iter().next().expect("途切れ");
    assert_eq!(
        o.last_alive_at.timestamp(),
        (now - Duration::hours(19)).timestamp()
    );
}

/// D4: 登録簿の `c02-%` と `c01-app-usage%` の名前が、観測ではない種類の表で扱われているか、
/// 扱わないと明示した表にあることを見る。引き継ぎで名前が変わったら、表に足し忘れると落ちる。
#[tokio::test]
async fn st14_inputs_nonobs_registry() {
    /// 観測ではない記録を持たない（全部が観測）か、途切れの判定から外れているので、表に載せない名前
    const NOT_HANDLED: [&str; 2] = ["c02-browser-history", "c01-app-usage-rollup"];
    let pool = testdb::pool().await;
    let names: Vec<String> = sqlx::query_scalar(
        "SELECT logical_source FROM core.source
          WHERE logical_source LIKE 'c02-%' OR logical_source LIKE 'c01-app-usage%'",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert!(!names.is_empty(), "登録簿に c02-% も c01-app-usage% も無い");
    for name in &names {
        let handled = NON_OBSERVATION_ALERT.iter().any(|(s, _)| s == name);
        let exempt = NOT_HANDLED.contains(&name.as_str());
        assert!(
            handled || exempt,
            "{name} が NON_OBSERVATION_ALERT にも NOT_HANDLED にも無い（引き継ぎで名前が変わったか）"
        );
        assert!(!(handled && exempt), "{name} が両方にある");
    }
}

// ---- 書庫（Task 6）。台帳は追記だけなので、試験ごとに固有の利用者・固有のソースに置く ----

const ARCHIVE_DAYS: i32 = 75;
const SIXTY_DAYS: i32 = 5_184_000;

/// 書庫のソース（日数 75・収集開始済み）。
async fn archive_source(pool: &sqlx::PgPool) -> String {
    let name = testdb::source(pool, "outage-arc", SIXTY_DAYS).await;
    sqlx::query("UPDATE core.source SET alert_after_days = $2 WHERE logical_source = $1")
        .bind(&name)
        .bind(ARCHIVE_DAYS)
        .execute(pool)
        .await
        .unwrap();
    testdb::set_started_on(pool, &name, "2025-01-01").await;
    name
}

/// 台帳に書庫を 1 件置く。`created` / `discovered` は何日前か。`sources` の各ソースに `(入った件数, 既にあった件数)` を立てる
/// （項目が 0 件の書庫は `archive_ledger_source` の行が立たない）。
async fn ledger(
    pool: &sqlx::PgPool,
    user: uuid::Uuid,
    outcome: &str,
    created: Option<Duration>,
    discovered: Duration,
    sources: &[(&str, i32, i32, Option<Duration>)],
) {
    let now = Utc::now();
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO core.archive_ledger (user_id, sha256, parser_version, outcome, created_at, discovered_at)
         VALUES ($1, $2, 'v1', $3, $4::timestamptz, $5::timestamptz) RETURNING id",
    )
    .bind(user)
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(outcome)
    .bind(created.map(|d| rfc(now, d)))
    .bind(rfc(now, discovered))
    .fetch_one(pool)
    .await
    .unwrap();
    for (src, inserted, dup, max_event) in sources {
        sqlx::query(
            "INSERT INTO core.archive_ledger_source
               (ledger_id, logical_source, inserted_count, duplicate_count, max_event_at)
             VALUES ($1, $2, $3, $4, $5::timestamptz)",
        )
        .bind(id)
        .bind(src)
        .bind(inserted)
        .bind(dup)
        .bind(max_event.map(|d| rfc(now, d)))
        .execute(pool)
        .await
        .unwrap();
    }
}

async fn archive_outage(pool: &sqlx::PgPool, user: uuid::Uuid, name: &str) -> Option<Outage> {
    outage_of(pool, user, Utc::now(), name, None).await
}

// Scenario: 最後の書庫から 75 日を過ぎると書庫のソースが途切れになる
#[tokio::test]
async fn st14_inputs_archive_over_75_days_is_outage() {
    let pool = testdb::pool().await;
    let (u, name) = (testdb::user(), archive_source(&pool).await);
    let age = Duration::days(75) + Duration::hours(1);
    ledger(&pool, u, "read", Some(age), age, &[(&name, 10, 0, None)]).await;
    let o = archive_outage(&pool, u, &name).await.expect("途切れ");
    assert_eq!(o.logical_source, name);
}

// Scenario: 2 か月ごとの書き出しでは書庫のソースは途切れにならない
#[tokio::test]
async fn st14_inputs_archive_every_two_months_is_not_outage() {
    let pool = testdb::pool().await;
    let (u, name) = (testdb::user(), archive_source(&pool).await);
    for d in [180, 120, 60] {
        let a = Duration::days(d);
        ledger(&pool, u, "read", Some(a), a, &[(&name, 10, 0, None)]).await;
    }
    assert!(archive_outage(&pool, u, &name).await.is_none());
}

// Scenario: 新しい行の無い書庫でも最後の書庫は進む
#[tokio::test]
async fn st14_inputs_archive_ledger_without_new_rows_still_advances() {
    let pool = testdb::pool().await;
    let (u, name) = (testdb::user(), archive_source(&pool).await);
    let (old, new) = (Duration::days(80), Duration::days(10));
    ledger(&pool, u, "read", Some(old), old, &[(&name, 10, 0, None)]).await;
    // 入った記録は 0 件、既にあった記録だけ
    ledger(&pool, u, "read", Some(new), new, &[(&name, 0, 25, None)]).await;
    assert!(archive_outage(&pool, u, &name).await.is_none());
}

// Scenario: 後の書庫が古い出来事を運ばなくても最後の書庫で測る
#[tokio::test]
async fn st14_inputs_archive_later_archive_with_older_events_still_counts() {
    let pool = testdb::pool().await;
    let (u, name) = (testdb::user(), archive_source(&pool).await);
    let (old, new) = (Duration::days(80), Duration::days(10));
    ledger(
        &pool,
        u,
        "read",
        Some(old),
        old,
        &[(&name, 10, 0, Some(old))],
    )
    .await;
    // 後の書庫が運んだ出来事は、もっと古い期間だけ
    ledger(
        &pool,
        u,
        "read",
        Some(new),
        new,
        &[(&name, 5, 0, Some(Duration::days(400)))],
    )
    .await;
    assert!(archive_outage(&pool, u, &name).await.is_none());
}

// Scenario: 形の確認を待っている書庫は最後の書庫に数えない
#[tokio::test]
async fn st14_inputs_archive_pending_shape_is_not_counted() {
    let pool = testdb::pool().await;
    let (u, name) = (testdb::user(), archive_source(&pool).await);
    let old = Duration::days(76);
    ledger(&pool, u, "read", Some(old), old, &[(&name, 10, 0, None)]).await;
    let new = Duration::days(5);
    ledger(
        &pool,
        u,
        "pending_shape",
        Some(new),
        new,
        &[(&name, 3, 0, None)],
    )
    .await;
    assert!(archive_outage(&pool, u, &name).await.is_some());
}

// Scenario: 作られた時刻の無い書庫は見つけた時刻で測る
#[tokio::test]
async fn st14_inputs_archive_without_created_at_uses_discovered_at() {
    let pool = testdb::pool().await;
    let (u, name) = (testdb::user(), archive_source(&pool).await);
    ledger(
        &pool,
        u,
        "read",
        None,
        Duration::days(76),
        &[(&name, 10, 0, None)],
    )
    .await;
    assert!(archive_outage(&pool, u, &name).await.is_some());
}

// Scenario: 項目が 0 件の書庫では最後の書庫は進まない
#[tokio::test]
async fn st14_inputs_archive_zero_item_archive_does_not_advance() {
    let pool = testdb::pool().await;
    let (u, name) = (testdb::user(), archive_source(&pool).await);
    let old = Duration::days(76);
    ledger(&pool, u, "read", Some(old), old, &[(&name, 10, 0, None)]).await;
    // ファイルは読めたが、このソースの項目が 0 件なので archive_ledger_source の行が立たない
    let new = Duration::days(5);
    ledger(&pool, u, "read", Some(new), new, &[]).await;
    assert!(archive_outage(&pool, u, &name).await.is_some());
}

// Scenario: 運んだ書庫が台帳に無いソースは届いた時刻で測る
#[tokio::test]
async fn st14_inputs_archive_without_ledger_uses_ingest_time() {
    let pool = testdb::pool().await;
    let (u, name) = (testdb::user(), archive_source(&pool).await);
    let now = Utc::now();
    put(
        &pool,
        u,
        &name,
        "collected",
        &rfc(now, Duration::days(300)),
        Some(&rfc(now, Duration::days(10))),
        json!({}),
    )
    .await;
    assert!(archive_outage(&pool, u, &name).await.is_none());
    // 80 日前に届いたきりなら途切れ
    let (u2, name2) = (testdb::user(), archive_source(&pool).await);
    put(
        &pool,
        u2,
        &name2,
        "collected",
        &rfc(now, Duration::days(300)),
        Some(&rfc(now, Duration::days(80))),
        json!({}),
    )
    .await;
    assert!(archive_outage(&pool, u2, &name2).await.is_some());
}

// Scenario: 使わなかった製品は途切れにならない
#[tokio::test]
async fn st14_inputs_archive_unused_product_is_not_outage() {
    let pool = testdb::pool().await;
    let (u, name) = (testdb::user(), archive_source(&pool).await);
    let now = Utc::now();
    put_collected(&pool, u, &name, &rfc(now, Duration::days(70)), json!({})).await;
    let new = Duration::days(5);
    ledger(&pool, u, "read", Some(new), new, &[(&name, 0, 0, None)]).await;
    assert!(archive_outage(&pool, u, &name).await.is_none());
}

// Scenario: 使わなかった製品は格子では途絶に見える
#[tokio::test]
async fn st14_inputs_archive_grid_shows_unused_product_as_outage() {
    use crate::coverage::{of_source, DayState, SourceRow};
    let pool = testdb::pool().await;
    let (u, name) = (testdb::user(), archive_source(&pool).await);
    let now = Utc::now();
    let last = now - Duration::days(70);
    put_collected(&pool, u, &name, &last.to_rfc3339(), json!({})).await;
    let new = Duration::days(5);
    ledger(&pool, u, "read", Some(new), new, &[(&name, 0, 0, None)]).await;
    // 判定の側（通知）は途切れにしない
    assert!(archive_outage(&pool, u, &name).await.is_none());
    // 格子は最後の記録から 61 日目の日を「途絶」と見る（想定間隔 60 日）
    let tokyo = chrono::FixedOffset::east_opt(9 * 3600).unwrap();
    let day = last.with_timezone(&tokyo).date_naive() + Duration::days(61);
    let src = SourceRow {
        logical_source: name.clone(),
        named_source: name.clone(),
        display_name: name.clone(),
        expected_gap_sec: SIXTY_DAYS,
        collection_started_on: Some(testdb::date("2025-01-01")),
        retired_on: None,
        chain: vec![name.clone()],
    };
    let cells = of_source(&pool, Some(u), &src, day, day)
        .await
        .unwrap()
        .days;
    assert_eq!(cells[0].state, DayState::Outage);
}
