// SPDX-License-Identifier: AGPL-3.0-only
//! `GET /outages` と途切れの台帳を、router を通して本物の PostgreSQL に対して確かめる
//! （ST14 Task 7 / design D5・D6・D7）。利用者・ソースは試験ごとに固有。時刻と起動時刻は差し込む。
#![allow(clippy::unwrap_used)]

use axum::body::Body;
use axum::http::{header, Method, Request};
use chrono::{DateTime, Duration, Utc};
use tower::ServiceExt as _;

use super::*;
use crate::testdb;

const TOKEN: &str = "test-token-0123456789abcdef";
const SIX_HOURS: i32 = 21_600;

/// 起動して 31 分後（待ちが明けている）の App。`now` が判定の時刻。
async fn app_at(now: DateTime<Utc>) -> App {
    App::for_test(testdb::pool().await, TOKEN)
        .at(now)
        .started(now - Duration::minutes(31))
}

async fn get(app: &App, uri: &str) -> (StatusCode, String) {
    let req = Request::builder()
        .method(Method::GET)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {TOKEN}"))
        .body(Body::empty())
        .unwrap();
    send(app, req).await
}

async fn send(app: &App, req: Request<Body>) -> (StatusCode, String) {
    let res = router(app.clone()).oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    (status, String::from_utf8(bytes.to_vec()).unwrap())
}

fn json(body: &str) -> serde_json::Value {
    serde_json::from_str(body).unwrap()
}

/// 試験ごとに固有の、想定間隔 6 時間・収集開始済みのソース。表示名を付ける。
async fn source(pool: &sqlx::PgPool, display_name: &str) -> String {
    let name = testdb::source(pool, "outage-api", SIX_HOURS).await;
    testdb::set_started_on(pool, &name, "2026-01-01").await;
    sqlx::query("UPDATE core.source SET display_name = $2 WHERE logical_source = $1")
        .bind(&name)
        .bind(display_name)
        .execute(pool)
        .await
        .unwrap();
    name
}

/// 緯度・経度を持つ記録を直に置く（応答に中身が出ないことを見る）。
async fn put_located(pool: &sqlx::PgPool, user: uuid::Uuid, source: &str, at: DateTime<Utc>) {
    sqlx::query(
        "INSERT INTO core.event
           (id, user_id, logical_source, device_id, origin, event_time, ingest_time,
            tz_offset_min, tz_id, schema_version, content_hash, raw, payload)
         VALUES ($1,$2,$3,'test','collected',$4,$4,540,'Asia/Tokyo',1,$5,'{}',
                 '{\"lat\": 35.681234, \"lon\": 139.767125}')",
    )
    .bind(uuid::Uuid::new_v4())
    .bind(user)
    .bind(source)
    .bind(at)
    .bind(uuid::Uuid::new_v4().to_string())
    .execute(pool)
    .await
    .unwrap();
}

fn uri(user: uuid::Uuid, device: Option<&str>) -> String {
    match device {
        Some(d) => format!("/outages?user_id={user}&device_id={d}"),
        None => format!("/outages?user_id={user}"),
    }
}

fn items(body: &str) -> Vec<serde_json::Value> {
    json(body)["outages"].as_array().unwrap().clone()
}

fn item_of(body: &str, source: &str) -> Option<serde_json::Value> {
    items(body)
        .into_iter()
        .find(|o| o["logical_source"] == source)
}

async fn ledger_count(
    pool: &sqlx::PgPool,
    user: uuid::Uuid,
    source: &str,
    kind: &str,
    device: Option<&str>,
) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM core.outage_ledger
          WHERE user_id = $1 AND logical_source = $2 AND kind = $3
            AND ($4::text IS NULL OR device_id = $4)",
    )
    .bind(user)
    .bind(source)
    .bind(kind)
    .bind(device)
    .fetch_one(pool)
    .await
    .unwrap()
}

/// 位置が 2 日前から途切れている状態を作る。
async fn broken_location(pool: &sqlx::PgPool, now: DateTime<Utc>) -> (uuid::Uuid, String) {
    let (user, name) = (testdb::user(), source(pool, "位置").await);
    put_located(pool, user, &name, now - Duration::days(2)).await;
    (user, name)
}

// Scenario: 途切れたソースの表示名と最後に生きていた時刻が返る
#[tokio::test]
async fn st14_api_outages_names_last_alive() {
    let (now, pool) = (Utc::now(), testdb::pool().await);
    let (user, name) = broken_location(&pool, now).await;
    let (status, body) = get(&app_at(now).await, &uri(user, Some("dev-a"))).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let v = json(&body);
    assert_eq!(v["status"], "ready");
    let o = item_of(&body, &name).expect("位置が返らない");
    assert_eq!(o["display_name"], "位置");
    assert_eq!(o["logical_source"], name.as_str());
    let alive: DateTime<Utc> = o["last_alive_at"].as_str().unwrap().parse().unwrap();
    assert_eq!(alive.timestamp(), (now - Duration::days(2)).timestamp());
    assert!(o["episode_id"].as_i64().unwrap() > 0, "識別子が無い");
    // AND 判定した時刻が返る
    let judged: DateTime<Utc> = v["judged_at"].as_str().unwrap().parse().unwrap();
    assert_eq!(judged.timestamp(), now.timestamp());
}

// Scenario: 途切れの応答に記録の中身が含まれない
#[tokio::test]
async fn st14_api_outages_no_record_content() {
    let (now, pool) = (Utc::now(), testdb::pool().await);
    let (user, name) = broken_location(&pool, now).await;
    let (_, body) = get(&app_at(now).await, &uri(user, Some("dev-a"))).await;
    let item = item_of(&body, &name).expect("途切れが返っていない");
    // 共有の登録簿の他の試験のソース（名前に raw などを含む）も途切れとして並ぶので、文字列ではなく欄の形で見る
    let keys = |v: &serde_json::Value| {
        let mut k: Vec<String> = v.as_object().unwrap().keys().cloned().collect();
        k.sort();
        k
    };
    assert_eq!(
        keys(&json(&body)),
        ["judged_at", "outages", "status"],
        "{body}"
    );
    for o in items(&body) {
        assert_eq!(
            keys(&o),
            [
                "display_name",
                "episode_id",
                "last_alive_at",
                "logical_source"
            ],
            "{o}"
        );
    }
    let mine = item.to_string();
    for needle in ["35.681234", "139.767125", "lat", "lon", "payload", "raw"] {
        assert!(!mine.contains(needle), "応答に {needle} が含まれる: {mine}");
    }
    assert!(
        !body.contains("35.681234") && !body.contains("139.767125"),
        "{body}"
    );
}

// Scenario: 収集アプリの資格情報で途切れの判定が読める
// ST44 が収集アプリの資格情報を締めたら、この試験が落ちて気づける。
#[tokio::test]
async fn st14_api_outages_bearer() {
    let (status, body) = get(&app_at(Utc::now()).await, &uri(testdb::user(), Some("d"))).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(json(&body)["status"], "ready");
}

// Scenario: 画面の資格情報で途切れの判定が読める
#[tokio::test]
async fn st14_api_outages_cookie() {
    let app = app_at(Utc::now()).await;
    let password = "test-web-password-0123456789";
    let login = Request::builder()
        .method(Method::POST)
        .uri("/session")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::json!({ "password": password }).to_string(),
        ))
        .unwrap();
    let res = router(app.clone()).oneshot(login).await.unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);
    let set_cookie = res.headers()[header::SET_COOKIE]
        .to_str()
        .unwrap()
        .to_owned();
    let cookie = set_cookie.split(';').next().unwrap().to_owned();
    let req = Request::builder()
        .uri(uri(testdb::user(), None))
        .header(header::COOKIE, cookie)
        .body(Body::empty())
        .unwrap();
    let (status, body) = send(&app, req).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(json(&body)["status"], "ready");
}

// Scenario: 資格情報の無い途切れの問い合わせは断られる
#[tokio::test]
async fn st14_api_outages_unauthorized() {
    let app = app_at(Utc::now()).await;
    let req = Request::builder()
        .uri(uri(testdb::user(), Some("d")))
        .body(Body::empty())
        .unwrap();
    let (status, body) = send(&app, req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(!body.contains("outages"), "判定が返った: {body}");
}

// Scenario: 起動して 30 分の間は判定を待っていると返る
#[tokio::test]
async fn st14_api_outages_warming_up() {
    let (now, pool) = (Utc::now(), testdb::pool().await);
    let (user, _) = broken_location(&pool, now).await;
    let app = app_at(now).await.started(now - Duration::minutes(10));
    let (status, body) = get(&app, &uri(user, Some("dev-a"))).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let v = json(&body);
    assert_eq!(v["status"], "warming_up");
    assert!(v.get("outages").is_none(), "一覧が返った: {body}");
}

// 試験だけが待ちを縮める口（縦串 `tools/smoke.sh`。既定は `OUTAGE_WARMUP` のまま）
#[tokio::test]
async fn st14_api_outages_warmup_override_is_zero() {
    let now = Utc::now();
    let app = app_at(now)
        .await
        .started(now)
        .with_warmup(std::time::Duration::ZERO);
    let (_, body) = get(&app, &uri(testdb::user(), Some("dev-a"))).await;
    assert_eq!(json(&body)["status"], "ready", "{body}");
}

// Scenario: 起動して 30 分が経つと判定が返る
#[tokio::test]
async fn st14_api_outages_after_warmup() {
    let (now, pool) = (Utc::now(), testdb::pool().await);
    let (user, name) = broken_location(&pool, now).await;
    let (_, body) = get(&app_at(now).await, &uri(user, Some("dev-a"))).await;
    assert_eq!(json(&body)["status"], "ready");
    assert!(
        item_of(&body, &name).is_some(),
        "位置が途切れとして返らない"
    );
}

// Scenario: 途切れが無ければ空の一覧が返る
#[tokio::test]
async fn st14_api_outages_empty() {
    // 登録簿は他の試験と共有で、どのソースも収集開始日の 0 時（D26（仮））か引き継いだ時刻から途切れとして数えられる。
    // 共有の行のどれよりも前の時刻で問い合わせて、共有の行を判定から外す（その時刻より後に生きていたことになる）
    let pool = testdb::pool().await;
    let earliest: Option<DateTime<Utc>> = sqlx::query_scalar(
        "SELECT least(min(collection_started_on::timestamp AT TIME ZONE 'Asia/Tokyo'),
                      min(registered_at) FILTER (WHERE succeeds IS NOT NULL))
           FROM core.source",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let cap = "2000-01-01T00:00:00Z".parse::<DateTime<Utc>>().unwrap();
    let now = earliest.map_or(cap, |e| (e - Duration::days(1)).min(cap));
    let (user, name) = (testdb::user(), source(&pool, "位置").await);
    put_located(&pool, user, &name, now - Duration::hours(1)).await;
    let (_, body) = get(&app_at(now).await, &uri(user, Some("dev-a"))).await;
    assert_eq!(json(&body)["status"], "ready");
    assert_eq!(items(&body).len(), 0, "{body}");
}

// Scenario: 端末識別子の無い問い合わせにも判定が返る
#[tokio::test]
async fn st14_api_outages_without_device_id() {
    let (now, pool) = (Utc::now(), testdb::pool().await);
    let (user, name) = broken_location(&pool, now).await;
    let (status, body) = get(&app_at(now).await, &uri(user, None)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(
        item_of(&body, &name).is_some(),
        "位置が途切れとして返らない"
    );
}

// Scenario: 途切れの一覧は表示名の順に返る
#[tokio::test]
async fn st14_api_outages_sorted_by_display_name() {
    let (now, pool) = (Utc::now(), testdb::pool().await);
    let user = testdb::user();
    let photo = source(&pool, "写真").await;
    let location = source(&pool, "位置").await;
    for s in [&photo, &location] {
        put_located(&pool, user, s, now - Duration::days(2)).await;
    }
    let (_, body) = get(&app_at(now).await, &uri(user, None)).await;
    let all = items(&body);
    let names: Vec<&str> = all
        .iter()
        .map(|o| o["display_name"].as_str().unwrap())
        .collect();
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted, "表示名の順でない");
    let pos = |s: &str| all.iter().position(|o| o["logical_source"] == s).unwrap();
    assert!(pos(&location) < pos(&photo), "位置と写真の順が昇順でない");
}

// Scenario: 途切れが続く間は同じ識別子が返る
#[tokio::test]
async fn st14_ledger_same_episode_id_while_out() {
    let (now, pool) = (Utc::now(), testdb::pool().await);
    let (user, name) = broken_location(&pool, now).await;
    let app = app_at(now).await;
    let (_, first) = get(&app, &uri(user, Some("dev-a"))).await;
    let (_, second) = get(&app, &uri(user, Some("dev-a"))).await;
    let id = |b: &str| item_of(b, &name).unwrap()["episode_id"].as_i64().unwrap();
    assert_eq!(id(&first), id(&second));
}

// Scenario: 新しく途切れると台帳に始まりが 1 行残る
#[tokio::test]
async fn st14_ledger_opened_once() {
    let (now, pool) = (Utc::now(), testdb::pool().await);
    let (user, name) = broken_location(&pool, now).await;
    let app = app_at(now).await;
    get(&app, &uri(user, Some("dev-a"))).await;
    get(&app, &uri(user, Some("dev-a"))).await;
    assert_eq!(ledger_count(&pool, user, &name, "opened", None).await, 1);
    // AND 最後に生きていた時刻と判定した時刻を持つ
    let (alive, judged): (Option<DateTime<Utc>>, DateTime<Utc>) = sqlx::query_as(
        "SELECT last_alive_at, judged_at FROM core.outage_ledger
          WHERE user_id = $1 AND logical_source = $2 AND kind = 'opened'",
    )
    .bind(user)
    .bind(&name)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        alive.unwrap().timestamp(),
        (now - Duration::days(2)).timestamp()
    );
    assert_eq!(judged.timestamp(), now.timestamp());
}

// 2 本の問い合わせが同時に走っても `opened` は 1 行（助言ロック）
#[tokio::test]
async fn st14_ledger_concurrent_opens_once() {
    let (now, pool) = (Utc::now(), testdb::pool().await);
    let (user, name) = broken_location(&pool, now).await;
    let app = app_at(now).await;
    let (a, b) = (uri(user, Some("dev-a")), uri(user, Some("dev-b")));
    let ((sa, ba), (sb, bb)) = tokio::join!(get(&app, &a), get(&app, &b));
    assert_eq!((sa, sb), (StatusCode::OK, StatusCode::OK), "{ba} {bb}");
    assert_eq!(ledger_count(&pool, user, &name, "opened", None).await, 1);
    let id = |b: &str| item_of(b, &name).unwrap()["episode_id"].as_i64().unwrap();
    assert_eq!(id(&ba), id(&bb));
}

// 判定の材料は台帳の助言ロックを取った後に読む（review/code.md R2）。
// 別の問い合わせ（B）がロックを持つ間に A が待ち、その間に B が「戻った」を書いて抜ける。
// A がロックの前に材料を読んでいたら、古い「途切れ」で新しい `opened` を作り、識別子が変わって端末で再び鳴る
#[tokio::test]
async fn st14_ledger_judges_inside_the_lock() {
    let (now, pool) = (Utc::now(), testdb::pool().await);
    let (user, name) = broken_location(&pool, now).await;
    let app = app_at(now).await;
    let (_, first) = get(&app, &uri(user, Some("dev-a"))).await;
    let episode = item_of(&first, &name).unwrap()["episode_id"]
        .as_i64()
        .unwrap();
    // B: ロックを取ったまま
    let mut held = pool.begin().await.unwrap();
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('outage_ledger:' || $1::text, 0))")
        .bind(user)
        .execute(&mut *held)
        .await
        .unwrap();
    // A: ロックで待つまで進める
    let a = tokio::spawn({
        let (app, uri) = (app.clone(), uri(user, Some("dev-a")));
        async move { get(&app, &uri).await }
    });
    let mut waited = false;
    for _ in 0..200 {
        let n: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM pg_stat_activity
              WHERE wait_event_type = 'Lock' AND wait_event = 'advisory'
                AND query LIKE '%outage_ledger:%' AND pid <> pg_backend_pid()",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        if n > 0 {
            waited = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    assert!(waited, "A が台帳のロックで待たなかった");
    // B: 位置が戻った（1 分前の記録）と判定して `closed` を書き、抜ける
    sqlx::query(
        "INSERT INTO core.event
           (id, user_id, logical_source, device_id, origin, event_time, ingest_time,
            tz_offset_min, tz_id, schema_version, content_hash, raw, payload)
         VALUES ($1,$2,$3,'test','collected',$4,$4,540,'Asia/Tokyo',1,$5,'{}','{}')",
    )
    .bind(uuid::Uuid::new_v4())
    .bind(user)
    .bind(&name)
    .bind(now - Duration::minutes(1))
    .bind(uuid::Uuid::new_v4().to_string())
    .execute(&mut *held)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO core.outage_ledger (user_id, logical_source, kind, episode_id, judged_at)
         VALUES ($1, $2, 'closed', $3, $4)",
    )
    .bind(user)
    .bind(&name)
    .bind(episode)
    .bind(now)
    .execute(&mut *held)
    .await
    .unwrap();
    held.commit().await.unwrap();
    let (status, body) = a.await.unwrap();
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(
        item_of(&body, &name).is_none(),
        "戻った後に古い判定で途切れを返した: {body}"
    );
    assert_eq!(
        ledger_count(&pool, user, &name, "opened", None).await,
        1,
        "古い判定で途切れが開き直された"
    );
}

// Scenario: 戻ると台帳に戻りが 1 行残る
// Scenario: 戻ったソースは応答の一覧から消える
#[tokio::test]
async fn st14_ledger_closed_when_back() {
    let (now, pool) = (Utc::now(), testdb::pool().await);
    let (user, name) = broken_location(&pool, now).await;
    let app = app_at(now).await;
    get(&app, &uri(user, Some("dev-a"))).await;
    put_located(&pool, user, &name, now - Duration::minutes(1)).await;
    let (_, body) = get(&app, &uri(user, Some("dev-a"))).await;
    assert!(item_of(&body, &name).is_none(), "戻ったのに一覧に残る");
    get(&app, &uri(user, Some("dev-a"))).await;
    assert_eq!(ledger_count(&pool, user, &name, "closed", None).await, 1);
}

// Scenario: 戻った後にまた途切れると別の識別子になる
#[tokio::test]
async fn st14_ledger_new_episode_after_return() {
    let (now, pool) = (Utc::now(), testdb::pool().await);
    let (user, name) = broken_location(&pool, now).await;
    let id = |b: &str| item_of(b, &name).unwrap()["episode_id"].as_i64().unwrap();
    let (_, first) = get(&app_at(now).await, &uri(user, Some("dev-a"))).await;
    put_located(&pool, user, &name, now + Duration::minutes(1)).await;
    let back = now + Duration::minutes(2);
    get(&app_at(back).await, &uri(user, Some("dev-a"))).await;
    let again = now + Duration::days(3);
    let (_, third) = get(&app_at(again).await, &uri(user, Some("dev-a"))).await;
    assert_ne!(id(&first), id(&third));
    assert_eq!(ledger_count(&pool, user, &name, "opened", None).await, 2);
}

// Scenario: 端末に初めて返した途切れが台帳に残る
// Scenario: 同じ途切れでも端末ごとに 1 回鳴る
#[tokio::test]
async fn st14_ledger_delivered_per_device() {
    let (now, pool) = (Utc::now(), testdb::pool().await);
    let (user, name) = broken_location(&pool, now).await;
    let app = app_at(now).await;
    get(&app, &uri(user, Some("dev-a"))).await;
    get(&app, &uri(user, Some("dev-a"))).await;
    get(&app, &uri(user, Some("dev-b"))).await;
    let n = |d| ledger_count(&pool, user, &name, "delivered", Some(d));
    assert_eq!(n("dev-a").await, 1);
    assert_eq!(n("dev-b").await, 1);
}

// Scenario: 端末識別子の無い問い合わせは届けたことを残さない
#[tokio::test]
async fn st14_ledger_no_delivered_without_device() {
    let (now, pool) = (Utc::now(), testdb::pool().await);
    let (user, name) = broken_location(&pool, now).await;
    get(&app_at(now).await, &uri(user, None)).await;
    assert_eq!(ledger_count(&pool, user, &name, "delivered", None).await, 0);
}

// Scenario: 判定を待つ間は台帳に書かない
#[tokio::test]
async fn st14_ledger_nothing_written_while_warming_up() {
    let (now, pool) = (Utc::now(), testdb::pool().await);
    let (user, _) = broken_location(&pool, now).await;
    let app = app_at(now).await.started(now - Duration::minutes(10));
    get(&app, &uri(user, Some("dev-a"))).await;
    let rows: i64 =
        sqlx::query_scalar("SELECT count(*) FROM core.outage_ledger WHERE user_id = $1")
            .bind(user)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(rows, 0);
}
