//! サーバの系統ごとの契約・既定値・画面の印（ST54 Task 2）。
#![allow(clippy::unwrap_used)]

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use http_body_util::BodyExt as _;
use tower::ServiceExt as _;

use super::*;
use crate::migration_ledger;
use crate::testdb;
use ashiato_build_stamp::Channel;

async fn migrate_channel(db: &testdb::TestDb, channel: Channel) {
    let pool = db.pool().await;
    let conn = pool.acquire().await.unwrap();
    migration_ledger::apply_migrations_with(conn, MIGRATIONS, || Ok(()), GRANTS, channel)
        .await
        .unwrap();
    if channel == Channel::Dev {
        sqlx::query("UPDATE migration_ledger.applied SET build_version = '0.1.0-dev.20261102'")
            .execute(&pool)
            .await
            .unwrap();
    }
    pool.close().await;
}

fn tee_app(channel: Channel, pool: sqlx::PgPool) -> App {
    App::for_test(pool, TOKEN).with_channel(channel)
}

async fn healthz(app: App) -> StatusCode {
    router(app)
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/healthz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
        .status()
}

const LINEAGE_OLD: &str = "202611030001_old_name";
const LINEAGE_NEW: &str = "202611030002_new_name";
const LINEAGE_REMOVED: &str = "202611030003_removed_name";
const LINEAGE_SQL: &str = "CREATE TABLE IF NOT EXISTS st54_lineage_marker (id integer);";

async fn apply_lineage_fixture(db: &testdb::TestDb, name: &str) {
    let pool = db.pool().await;
    migration_ledger::apply_migrations_with(
        pool.acquire().await.unwrap(),
        &[(name, LINEAGE_SQL)],
        || Ok(()),
        "",
        Channel::Official,
    )
    .await
    .unwrap();
}

async fn migrate_with_lineage(
    db: &testdb::TestDb,
    versions: &[(&str, &str)],
    lineage: &[(migration_ledger::LineageKind, &str, &str)],
) -> Result<(), migration_ledger::MigrateError> {
    migration_ledger::apply_migrations_with_lineage(
        db.pool().await.acquire().await.unwrap(),
        versions,
        || Ok(()),
        "",
        lineage,
        Channel::Official,
    )
    .await
}

// Scenario: 名前を変えた移行は、旧名の対応があれば、旧名で当てた DB でも起動し、当て直さない
#[tokio::test]
async fn st54_lineage_renamed_migration_starts_without_reapplying() {
    let db = testdb::scratch().await;
    apply_lineage_fixture(&db, LINEAGE_OLD).await;
    let pool = db.pool().await;
    let versions = &[(LINEAGE_NEW, LINEAGE_SQL)];
    let lineage = &[(
        migration_ledger::LineageKind::Renamed,
        LINEAGE_OLD,
        LINEAGE_NEW,
    )];
    migrate_with_lineage(&db, versions, lineage).await.unwrap();
    assert_eq!(
        migration_ledger::startup_refusal_with_lineage(
            &pool,
            versions,
            lineage,
            Channel::Official,
        )
        .await
        .unwrap(),
        None
    );
    assert_eq!(healthz(App::for_test(pool, TOKEN)).await, StatusCode::OK);
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM migration_ledger.applied WHERE name = $1")
            .bind(LINEAGE_OLD)
            .fetch_one(&db.pool().await)
            .await
            .unwrap();
    assert_eq!(count, 1);
    let new_count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM migration_ledger.applied WHERE name = $1")
            .bind(LINEAGE_NEW)
            .fetch_one(&db.pool().await)
            .await
            .unwrap();
    assert_eq!(new_count, 0);
}

// Scenario: 消した移行は、消したことの対応があれば、それを当てた DB でも起動する
#[tokio::test]
async fn st54_lineage_removed_migration_starts() {
    let db = testdb::scratch().await;
    apply_lineage_fixture(&db, LINEAGE_REMOVED).await;
    let pool = db.pool().await;
    let lineage = &[(migration_ledger::LineageKind::Removed, LINEAGE_REMOVED, "")];
    assert_eq!(
        migration_ledger::startup_refusal_with_lineage(&pool, &[], lineage, Channel::Official)
            .await
            .unwrap(),
        None
    );
    assert_eq!(healthz(App::for_test(pool, TOKEN)).await, StatusCode::OK);
}

// Scenario: 旧名の対応が無ければ、今どおり知らない版で止まり、その名前と当てた組み立てを示す
#[tokio::test]
async fn st54_lineage_without_mapping_reports_unknown_version() {
    let db = testdb::scratch().await;
    apply_lineage_fixture(&db, LINEAGE_OLD).await;
    let pool = db.pool().await;
    let refusal = migration_ledger::startup_refusal(&pool, &[], Channel::Official)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(refusal.0, "db_ahead");
    assert!(refusal.1.contains(LINEAGE_OLD));
    assert!(refusal.1.contains("0.1.0"));
}

const TOKEN: &str = "st54-test-token-0123456789";

async fn app(channel: Channel) -> App {
    App::for_test(testdb::pool().await, TOKEN).with_channel(channel)
}

async fn contract(app: App) -> axum::response::Response<Body> {
    let request = Request::builder()
        .method(Method::GET)
        .uri("/contract")
        .header(header::AUTHORIZATION, format!("Bearer {TOKEN}"))
        .body(Body::empty())
        .unwrap();
    router(app).oneshot(request).await.unwrap()
}

async fn contract_body(response: axum::response::Response<Body>) -> Vec<u8> {
    response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes()
        .to_vec()
}

// Scenario: 開発版のサーバは、契約の問い合わせに系統 dev を答える
#[tokio::test]
async fn st54_contract_answers_dev_channel() {
    let response = contract(app(Channel::Dev).await).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["Ashiato-Channel"], "dev");
    assert_eq!(contract_body(response).await, br#"{"contract_version":1}"#);
}

// Scenario: 正式版のサーバは、契約の問い合わせに系統 official を答える
#[tokio::test]
async fn st54_contract_answers_official_channel() {
    let response = contract(app(Channel::Official).await).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["Ashiato-Channel"], "official");
    assert_eq!(contract_body(response).await, br#"{"contract_version":1}"#);
}

// Scenario: 開発版のサーバの既定は、正式版と別の待ち受け・クッキー・退避の置き場
#[test]
fn st54_dev_defaults_are_separate() {
    assert_eq!(net_guard::default_bind(Channel::Dev), "127.0.0.1:18788");
    assert_eq!(
        web_session::cookie_name(Channel::Dev),
        "ashiato_session_dev"
    );
    assert_eq!(
        migrate_backup::default_dir(Channel::Dev),
        "backups/pre-migrate-dev"
    );
}

// Scenario: 正式版のサーバの既定は、今のまま
#[test]
fn st54_official_defaults_are_unchanged() {
    assert_eq!(
        net_guard::default_bind(Channel::Official),
        "127.0.0.1:18787"
    );
    assert_eq!(
        web_session::cookie_name(Channel::Official),
        "ashiato_session"
    );
    assert_eq!(
        migrate_backup::default_dir(Channel::Official),
        "backups/pre-migrate"
    );
}

// Scenario: 開発版のサーバの画面のログインは、開発版の名前のクッキーだけを使う
#[tokio::test]
async fn st54_dev_web_login_uses_dev_cookie() {
    let password = "st54-web-password-0123456789";
    let app = app(Channel::Dev).await.with_web_login(password, 0);
    let request = Request::builder()
        .method(Method::POST)
        .uri("/session")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(format!(r#"{{"password":"{password}"}}"#)))
        .unwrap();
    let app_router = router(app);
    let response = app_router.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let set_cookie = response.headers()[header::SET_COOKIE].to_str().unwrap();
    assert!(set_cookie.starts_with("ashiato_session_dev="));
    let dev_cookie = set_cookie.split(';').next().unwrap();

    let response = app_router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/session")
                .header(header::COOKIE, dev_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let response = app_router
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/session")
                .header(header::COOKIE, "ashiato_session=wrong-channel")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

// Scenario: 待ち受けと退避の置き場を明示で渡せば、開発版でもその値を使う
#[test]
fn st54_explicit_bind_and_backup_dir_win_for_dev() {
    assert_eq!(
        net_guard::bind_for(Channel::Dev, Some("127.0.0.1:19999")),
        "127.0.0.1:19999"
    );
    assert_eq!(
        migrate_backup::dir_for(Channel::Dev, Some("custom/backups")),
        "custom/backups"
    );
}

// Scenario: 開発版の migrate は、正式版の DB に 1 つも当てずに止まる
#[tokio::test]
async fn st54_migration_channel_refuses_official_without_writing() {
    let db = testdb::scratch().await;
    migrate_channel(&db, Channel::Official).await;
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM migration_ledger.applied")
        .fetch_one(&db.pool().await)
        .await
        .unwrap();
    let pool = db.pool().await;
    let result = migration_ledger::apply_migrations_with(
        pool.acquire().await.unwrap(),
        MIGRATIONS,
        || Ok(()),
        GRANTS,
        Channel::Dev,
    )
    .await;
    assert!(matches!(
        result,
        Err(migration_ledger::MigrateError::ChannelMismatch { .. })
    ));
    let after: i64 = sqlx::query_scalar("SELECT count(*) FROM migration_ledger.applied")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(after, before);
}

// Scenario: 正式版の migrate とサーバは、開発版の DB では止まる
#[tokio::test]
async fn st54_official_migration_and_startup_refuse_dev() {
    let db = testdb::scratch().await;
    migrate_channel(&db, Channel::Dev).await;
    let pool = db.pool().await;
    let result = migration_ledger::apply_migrations_with(
        pool.acquire().await.unwrap(),
        MIGRATIONS,
        || Ok(()),
        GRANTS,
        Channel::Official,
    )
    .await;
    assert!(matches!(
        result,
        Err(migration_ledger::MigrateError::ChannelMismatch { .. })
    ));
    let refusal = migration_ledger::startup_refusal(&pool, MIGRATIONS, Channel::Official)
        .await
        .unwrap();
    assert_eq!(refusal.unwrap().0, "channel_mismatch");
}

// Scenario: 空の DB に開発版の migrate を当てると、開発版の DB になる
#[tokio::test]
async fn st54_empty_database_becomes_dev() {
    let db = testdb::scratch().await;
    migrate_channel(&db, Channel::Dev).await;
    let pool = db.pool().await;
    let version: String = sqlx::query_scalar(
        "SELECT build_version FROM migration_ledger.applied ORDER BY applied_at, name LIMIT 1",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(version.contains("-dev."), "{version}");
    // 行の版番号は組み立てのときの定数なので、この試験の組み立てでは `migrate_channel` が書き換える。
    // 「開発版の migrate が全行に開発版の版番号を書き、続く起動が 200」は実バイナリの
    // `st54_bin_dev_migrate_bootstraps_dev_db` が確かめる。ここでは、空の DB に開発版として当てた結果を
    // 本物の起動の判定が開発版の DB と読むことを見る（final review R4）
    let applied: i64 = sqlx::query_scalar("SELECT count(*) FROM migration_ledger.applied")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(usize::try_from(applied).unwrap(), MIGRATIONS.len());
    let dev = migration_ledger::startup_refusal(&pool, MIGRATIONS, Channel::Dev)
        .await
        .unwrap();
    assert!(
        dev.is_none(),
        "開発版のサーバが開発版の DB で止まる: {dev:?}"
    );
    let official = migration_ledger::startup_refusal(&pool, MIGRATIONS, Channel::Official)
        .await
        .unwrap();
    assert_eq!(official.unwrap().0, "channel_mismatch");
    assert_eq!(
        healthz(tee_app(Channel::Dev, pool.clone())).await,
        StatusCode::OK
    );
}

// Scenario: 版の記録の無い古い DB には、開発版の migrate は当てずに止まる
#[tokio::test]
async fn st54_before_ledger_is_official() {
    let db = testdb::scratch().await;
    let pool = db.pool().await;
    sqlx::query("CREATE SCHEMA core")
        .execute(&pool)
        .await
        .unwrap();
    let result = migration_ledger::apply_migrations_with(
        pool.acquire().await.unwrap(),
        MIGRATIONS,
        || Ok(()),
        GRANTS,
        Channel::Dev,
    )
    .await;
    assert!(matches!(
        result,
        Err(migration_ledger::MigrateError::ChannelMismatch {
            db: Channel::Official,
            me: Channel::Dev
        })
    ));
}

// Scenario: 版の記録の行が 1 つも無い DB には、どちらの系統の migrate も当てる
#[tokio::test]
async fn st54_empty_ledger_allows_both_channels() {
    let db = testdb::scratch().await;
    migrate_channel(&db, Channel::Dev).await;
    let pool = db.pool().await;
    sqlx::query("DELETE FROM migration_ledger.applied")
        .execute(&pool)
        .await
        .unwrap();
    migration_ledger::apply_migrations_with(
        pool.acquire().await.unwrap(),
        MIGRATIONS,
        || Ok(()),
        GRANTS,
        Channel::Official,
    )
    .await
    .unwrap();
}

// Scenario: 系統が同じなら、今どおり当てて起動する
#[tokio::test]
async fn st54_same_channel_starts_and_healthz_is_ok() {
    let db = testdb::scratch().await;
    migrate_channel(&db, Channel::Dev).await;
    let pool = db.pool().await;
    assert_eq!(
        migration_ledger::startup_refusal(&pool, MIGRATIONS, Channel::Dev)
            .await
            .unwrap(),
        None
    );
    assert_eq!(healthz(tee_app(Channel::Dev, pool)).await, StatusCode::OK);
}

/// 退避の後のログは、その系統の既定の置き場を示す（開発版で正式版の置き場を示さない。final review R5）。
#[test]
fn st54_backup_log_names_channel_default_dir() {
    assert_eq!(
        backup_done_message(Channel::Dev),
        "移行の前に退避した（MIGRATE_BACKUP_DIR の下。置き場は既定 backups/pre-migrate-dev）"
    );
    assert_eq!(
        backup_done_message(Channel::Official),
        "移行の前に退避した（MIGRATE_BACKUP_DIR の下。置き場は既定 backups/pre-migrate）"
    );
}
