//! S-01 バックエンド。書き込みは 1 本の取り込み口に集約する（製造準備 A-1）。
//!
//! 冪等の判定（FR-22）・原文の保存（FR-18）・稼働記録の更新（FR-33）は
//! この 1 か所を必ず通る。Kotlin と Rust の 2 実装が同じ口を叩くため、
//! 不変条件をサーバ側に置かないと守れない。
use anyhow::Context as _;
use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;

mod ingest;
use ingest::{content_hash, IngestRequest};

#[derive(Clone)]
struct App {
    pool: sqlx::PgPool,
}

#[derive(Serialize)]
struct IngestReply {
    id: uuid::Uuid,
    /// 既に同じ 1 件があったか。再送しても行が増えないことの確認に使う（FR-22）
    duplicate: bool,
}

#[derive(Serialize, Deserialize)]
struct EventRow {
    id: uuid::Uuid,
    logical_source: String,
    event_time: chrono::DateTime<chrono::Utc>,
    tz_id: String,
    origin: String,
    raw: serde_json::Value,
}

async fn ingest(
    State(app): State<App>,
    Json(req): Json<IngestRequest>,
) -> Result<Json<IngestReply>, (StatusCode, String)> {
    let hash = content_hash(&req);
    // 登録簿に無いソースは受け付けない。API を変えずにソースを増やすので（FR-61）、
    // 増やす操作は「登録簿へ 1 行 INSERT」だけになる。
    let known: Option<(String,)> =
        sqlx::query_as("SELECT logical_source FROM core.source WHERE logical_source = $1")
            .bind(&req.logical_source)
            .fetch_optional(&app.pool)
            .await
            .map_err(internal)?;
    if known.is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("未登録のソース: {}", req.logical_source),
        ));
    }

    let row: Option<(uuid::Uuid,)> = sqlx::query_as(
        "INSERT INTO core.event
           (id, user_id, logical_source, external_id, device_id, origin,
            event_time, tz_offset_min, tz_id, schema_version, content_hash, raw, payload)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)
         ON CONFLICT (logical_source, content_hash) DO NOTHING
         RETURNING id",
    )
    .bind(req.id)
    .bind(req.user_id)
    .bind(&req.logical_source)
    .bind(&req.external_id)
    .bind(&req.device_id)
    .bind(&req.origin)
    .bind(req.event_time)
    .bind(req.tz_offset_min)
    .bind(&req.tz_id)
    .bind(req.schema_version)
    .bind(&hash)
    .bind(&req.raw)
    .bind(&req.payload)
    .fetch_optional(&app.pool)
    .await
    .map_err(internal)?;

    // 稼働記録は取り込みと同じ関門で更新する。別経路にすると
    // 「データが無いのは収集が止まっていたのか」が後から区別できなくなる（FR-33）。
    sqlx::query(
        "INSERT INTO core.coverage (logical_source, day, state, event_count)
         VALUES ($1, ($2 AT TIME ZONE 'UTC')::date, 'alive', 1)
         ON CONFLICT (logical_source, day, state)
         DO UPDATE SET event_count = core.coverage.event_count + 1",
    )
    .bind(&req.logical_source)
    .bind(req.event_time)
    .execute(&app.pool)
    .await
    .map_err(internal)?;

    match row {
        Some((id,)) => Ok(Json(IngestReply {
            id,
            duplicate: false,
        })),
        None => Ok(Json(IngestReply {
            id: req.id,
            duplicate: true,
        })),
    }
}

async fn events(State(app): State<App>) -> Result<Json<Vec<EventRow>>, (StatusCode, String)> {
    // 素のテーブルではなくビューを引く。論理削除を全クエリに効かせるため（A-3）。
    let rows = sqlx::query_as::<
        _,
        (
            uuid::Uuid,
            String,
            chrono::DateTime<chrono::Utc>,
            String,
            String,
            serde_json::Value,
        ),
    >(
        "SELECT id, logical_source, event_time, tz_id, origin, raw
           FROM core.event_live ORDER BY event_time",
    )
    .fetch_all(&app.pool)
    .await
    .map_err(internal)?;
    Ok(Json(
        rows.into_iter()
            .map(
                |(id, logical_source, event_time, tz_id, origin, raw)| EventRow {
                    id,
                    logical_source,
                    event_time,
                    tz_id,
                    origin,
                    raw,
                },
            )
            .collect(),
    ))
}

fn internal<E: std::fmt::Display>(e: E) -> (StatusCode, String) {
    // 私的データはログに出さない（A-2）。出すのはエラーの種別だけ。
    tracing::error!(kind = "db", "データベース操作に失敗");
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();
    std::panic::set_hook(Box::new(|info| {
        // 未捕捉の異常が黙って消えないようにする（製造準備 C）
        tracing::error!(kind = "panic", location = ?info.location(), "未捕捉の異常");
    }));

    let url = std::env::var("DATABASE_URL").context("DATABASE_URL が未設定")?;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await?;
    sqlx::raw_sql(include_str!("../../../migrations/0001_envelope.sql"))
        .execute(&pool)
        .await
        .context("マイグレーションの適用に失敗")?;

    let app = Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .route("/ingest", post(ingest))
        .route("/events", get(events))
        .with_state(App { pool });

    let addr = std::env::var("BIND").unwrap_or_else(|_| "127.0.0.1:8787".into());
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!(addr = %addr, "起動");
    axum::serve(listener, app).await?;
    Ok(())
}
