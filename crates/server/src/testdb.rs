// SPDX-License-Identifier: AGPL-3.0-only
//! テストが本物の PostgreSQL を叩くための足場。**偽物に差し替えない。**
//!
//! 稼働記録の振る舞いは、書き換え禁止のトリガ・一意索引・`AT TIME ZONE` の
//! いずれも DB の側にある。模擬した DB で確かめると、**0004 が実測で見つけた
//! 「3 手の迂回」の型の穴がそのまま残る**。
//!
//! **接続できなければテストは落ちる。飛ばさない。** 飛ばすと、DB が無い環境で
//! 全部が緑になり、稼働記録の検査が 1 本も走っていないことに気付けない。
#![allow(clippy::unwrap_used)]

use sqlx::postgres::PgPoolOptions;

/// 開発用 DB（`docker compose up -d db`）の既定。CI は `DATABASE_URL` で差し替える。
const DEFAULT_URL: &str = "postgres://ashiato:ashiato@127.0.0.1:55432/ashiato";

/// マイグレーションは 1 プロセスに 1 回だけ当てる。
/// **同時に当てると `CREATE TABLE IF NOT EXISTS` 同士が競合する**ので、
/// 助言ロックで直列化する（別プロセスのテストと並んでも安全になる）。
static MIGRATED: tokio::sync::OnceCell<()> = tokio::sync::OnceCell::const_new();

/// 接続済みのプールを返す。初回だけマイグレーションを当てる。
pub async fn pool() -> sqlx::PgPool {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_URL.into());
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .unwrap_or_else(|e| {
            panic!("テスト用 DB へ接続できない（{url}）: {e}\n  docker compose up -d --wait db を先に実行する")
        });
    MIGRATED
        .get_or_init(|| async {
            let mut tx = pool.begin().await.unwrap();
            sqlx::query("SELECT pg_advisory_xact_lock(4820251)")
                .execute(&mut *tx)
                .await
                .unwrap();
            for sql in crate::MIGRATIONS {
                sqlx::raw_sql(sql.1).execute(&mut *tx).await.unwrap();
            }
            tx.commit().await.unwrap();
        })
        .await;
    pool
}

/// テストごとに固有の論理ソースを登録簿へ置く。
///
/// **テストどうしの隔離をこれで取る**（design D14）—— 1 つの DB を共有したまま、
/// 稼働記録・生存信号・停止/破棄はすべて `logical_source` で分かれる。
/// 表を作り直したり schema を掘ったりしない（本物と同じ表を叩くことに意味がある）。
pub async fn source(pool: &sqlx::PgPool, prefix: &str, expected_gap_sec: i32) -> String {
    let name = format!("t-{prefix}-{}", uuid::Uuid::new_v4());
    sqlx::query(
        "INSERT INTO core.source (logical_source, display_name, expected_gap_sec)
         VALUES ($1, $1, $2)",
    )
    .bind(&name)
    .bind(expected_gap_sec)
    .execute(pool)
    .await
    .unwrap();
    name
}

/// テストごとに固有の利用者。稼働記録・生存信号は利用者でも分かれる（FR-29）。
pub fn user() -> uuid::Uuid {
    uuid::Uuid::new_v4()
}

/// 稼働記録を直に置く（取り込み口を通さずに状態の導出だけを見たいとき）。
pub async fn put_coverage(
    pool: &sqlx::PgPool,
    user: uuid::Uuid,
    source: &str,
    day: &str,
    count: i32,
) {
    sqlx::query(
        "INSERT INTO core.coverage (user_id, logical_source, day, event_count)
         VALUES ($1,$2,$3::date,$4)
         ON CONFLICT (user_id, logical_source, day)
         DO UPDATE SET event_count = core.coverage.event_count + $4",
    )
    .bind(user)
    .bind(source)
    .bind(day)
    .bind(count)
    .execute(pool)
    .await
    .unwrap();
}

/// 生存信号を直に置く。`emitted` は RFC3339。
pub async fn put_heartbeat(
    pool: &sqlx::PgPool,
    user: uuid::Uuid,
    source: &str,
    emitted: &str,
    capturable: bool,
) {
    put_heartbeat_counts(pool, user, source, emitted, capturable, 1, 1).await;
}

/// 取得の試行回数と成功回数まで指定して生存信号を置く。
pub async fn put_heartbeat_counts(
    pool: &sqlx::PgPool,
    user: uuid::Uuid,
    source: &str,
    emitted: &str,
    capturable: bool,
    attempts: i32,
    successes: i32,
) {
    let blockers: Vec<String> = if capturable {
        vec![]
    } else {
        vec!["permission".into()]
    };
    sqlx::query(
        "INSERT INTO core.heartbeat
           (id, user_id, logical_source, device_id, emitted_at, capturable, blockers,
            attempts, successes, content_hash, raw)
         VALUES ($1,$2,$3,'test',$4::timestamptz,$5,$6,$7,$8,$9,'{}')",
    )
    .bind(uuid::Uuid::new_v4())
    .bind(user)
    .bind(source)
    .bind(emitted)
    .bind(capturable)
    .bind(&blockers)
    .bind(attempts)
    .bind(successes)
    .bind(uuid::Uuid::new_v4().to_string())
    .execute(pool)
    .await
    .unwrap();
}

/// 停止・破棄の範囲を置く。`kind` は `stopped` か `dropped`。
pub async fn put_span(
    pool: &sqlx::PgPool,
    user: uuid::Uuid,
    source: &str,
    kind: &str,
    started: &str,
    ended: Option<&str>,
) {
    sqlx::query(
        "INSERT INTO core.coverage_span
           (id, user_id, logical_source, kind, started_at, ended_at)
         VALUES ($1,$2,$3,$4,$5::timestamptz,$6::timestamptz)",
    )
    .bind(uuid::Uuid::new_v4())
    .bind(user)
    .bind(source)
    .bind(kind)
    .bind(started)
    .bind(ended)
    .execute(pool)
    .await
    .unwrap();
}

/// 収集開始日を直に置く。
pub async fn set_started_on(pool: &sqlx::PgPool, source: &str, day: &str) {
    sqlx::query(
        "UPDATE core.source SET collection_started_on = $2::date WHERE logical_source = $1",
    )
    .bind(source)
    .bind(day)
    .execute(pool)
    .await
    .unwrap();
}

/// 日付リテラル。テストの読みやすさのためだけに置く。
pub fn date(s: &str) -> chrono::NaiveDate {
    s.parse().unwrap()
}
