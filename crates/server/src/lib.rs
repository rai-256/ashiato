// SPDX-License-Identifier: AGPL-3.0-only
//! S-01 バックエンド。書き込みは 1 本の取り込み口に集約する（製造準備 A-1）。
//!
//! 冪等の判定（FR-22）・原文の保存（FR-18）・稼働記録の更新（FR-33）は
//! この 1 か所を必ず通る。Kotlin と Rust の 2 実装が同じ口を叩くため、
//! 不変条件をサーバ側に置かないと守れない。
use anyhow::Context as _;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;

pub mod ingest;
use ingest::{content_hash, IngestRequest};

#[derive(Clone, Debug)]
pub struct App {
    pool: sqlx::PgPool,
    /// 共有の合言葉。**loopback に閉じているだけでは足りない** ——
    /// 同じ PC の別プロセス（＝第三者製プラグイン。PERM-8 は既定を最も厳しい側に置いている）が
    /// 素通しで読み書きできてしまう。
    token: String,
}

/// 合言葉を確かめる。無ければ 401。
fn authorize(app: &App, headers: &HeaderMap) -> Result<(), (StatusCode, String)> {
    let given = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or_default();
    // 比較にかかる時間で中身が漏れないよう、長さを確かめてから全バイトを畳み込む
    let ok = given.len() == app.token.len()
        && given
            .bytes()
            .zip(app.token.bytes())
            .fold(0u8, |acc, (a, b)| acc | (a ^ b))
            == 0;
    if ok {
        Ok(())
    } else {
        Err((StatusCode::UNAUTHORIZED, "unauthorized".into()))
    }
}

/// 取り込みを断った理由。**受け取った値は載せない**（design D5）——
/// 値をそのまま返すと、呼び出し元へ内容が反射する。
#[derive(Debug, Clone, Copy, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum IngestError {
    /// 項目が形として解釈できない（必須の欄が欠けている場合を含む）
    Malformed,
    /// 由来の分類が列挙のどれでもない
    UnknownOrigin,
    /// 登録簿に無い論理ソース
    UnknownSource,
}

/// 送った 1 件ごとの結果。**送った順に並ぶ**ので、収集側は位置で対応づける
/// （design D12）。断られた項目には `id` が無い場合がある。
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct IngestResult {
    /// 格納された記録の識別子。断られたときは null
    id: Option<uuid::Uuid>,
    /// 既に同じ 1 件があったか。再送しても行が増えないことの確認に使う（FR-22）
    duplicate: bool,
    /// 未送信から取り除いてよいか。収集側はこれだけを見る（design D9）
    accepted: bool,
    /// 断った理由の種別。受け付けたときは null
    error: Option<IngestError>,
}

impl IngestResult {
    fn stored(id: uuid::Uuid, duplicate: bool) -> Self {
        Self {
            id: Some(id),
            duplicate,
            accepted: true,
            error: None,
        }
    }

    fn rejected(id: Option<uuid::Uuid>, error: IngestError) -> Self {
        Self {
            id,
            duplicate: false,
            accepted: false,
            error: Some(error),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct EventRow {
    id: uuid::Uuid,
    logical_source: String,
    event_time: chrono::DateTime<chrono::Utc>,
    tz_id: String,
    origin: String,
    raw: serde_json::Value,
}

/// 1 件を格納して結果を返す。**呼び出し側の誤りは Err ではなく `IngestResult` で返す** ——
/// まとめ送りの一部が不正でも、他の件は格納しなければならない（design D9）。
/// Err になるのはサーバ側の失敗（DB）だけ。
async fn ingest_one(
    app: &App,
    item: &serde_json::Value,
) -> Result<IngestResult, (StatusCode, String)> {
    // 収集側が採番した id だけは、形が壊れていても拾えるなら結果に載せる（対応づけの助けになる）
    let sent_id = item
        .get("id")
        .and_then(|v| v.as_str())
        .and_then(|v| uuid::Uuid::parse_str(v).ok());

    let req: IngestRequest = match serde_json::from_value(item.clone()) {
        Ok(r) => r,
        Err(_) => {
            // **元のエラーを載せない。** serde の文言は受け取った値を含むことがある（design D5）
            tracing::warn!(kind = "malformed", "解釈できない項目を断った");
            return Ok(IngestResult::rejected(sent_id, IngestError::Malformed));
        }
    };

    // 由来の分類はアプリ層で閉じる（design D5）。DB の CHECK に任せると 500 になり、
    // 呼び出し側から「自分の要求が悪い」と分からない。
    if !req.origin_is_known() {
        return Ok(IngestResult::rejected(
            Some(req.id),
            IngestError::UnknownOrigin,
        ));
    }

    // 登録簿に無いソースは受け付けない。API を変えずにソースを増やすので（FR-61）、
    // 増やす操作は「登録簿へ 1 行 INSERT」だけになる。
    let known: Option<(String,)> =
        sqlx::query_as("SELECT logical_source FROM core.source WHERE logical_source = $1")
            .bind(&req.logical_source)
            .fetch_optional(&app.pool)
            .await
            .map_err(internal)?;
    if known.is_none() {
        return Ok(IngestResult::rejected(
            Some(req.id),
            IngestError::UnknownSource,
        ));
    }

    let hash = content_hash(&req);
    // **`payload` だけを NFC に揃える。`raw` は受け取ったまま送る**（design D2 / FR-18）。
    // 原文のバイト列は一度変換すると二度と戻らない。
    let payload = ingest::to_nfc(&req.payload);

    let row: Option<(uuid::Uuid,)> = sqlx::query_as(
        "INSERT INTO core.event
           (id, user_id, logical_source, external_id, device_id, origin,
            event_time, tz_offset_min, tz_id, schema_version, unit_system, crs,
            content_hash, raw, payload)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)
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
    .bind(req.unit_system_or_default())
    .bind(req.crs_or_default())
    .bind(&hash)
    .bind(&req.raw)
    .bind(&payload)
    .fetch_optional(&app.pool)
    .await
    .map_err(internal)?;

    // 稼働記録は取り込みと同じ関門で更新する。別経路にすると
    // 「データが無いのは収集が止まっていたのか」が後から区別できなくなる（FR-33）。
    // **件数は新しく入った行だけ数える**（design D13）—— まとめ送りの部分失敗で
    // 成功分が再送されるので、重複まで数えると件数が実態から離れる。
    // 行そのものは重複でも立てる。「その日は収集が動いていた」は重複の到着でも真だから。
    sqlx::query(
        "INSERT INTO core.coverage (logical_source, day, state, event_count)
         VALUES ($1, ($2 AT TIME ZONE 'UTC')::date, 'alive', $3)
         ON CONFLICT (logical_source, day, state)
         DO UPDATE SET event_count = core.coverage.event_count + $3",
    )
    .bind(&req.logical_source)
    .bind(req.event_time)
    .bind(i32::from(row.is_some()))
    .execute(&app.pool)
    .await
    .map_err(internal)?;

    Ok(match row {
        Some((id,)) => IngestResult::stored(id, false),
        None => IngestResult::stored(req.id, true),
    })
}

/// 記録をまとめて受け取る。同じ内容を再送しても行は増えない（FR-22）。
///
/// 要求は**配列**（まとめ送り。design D9）。1 件だけの裸のオブジェクトも受け取る ——
/// 既存の収集側と `tools/smoke.sh` を壊さないため（design D12）。
/// 応答は**送った順に並ぶ 1 件ごとの結果**で、収集側は `accepted` が真の分だけを
/// 未送信から取り除く。
///
/// **400 は「1 件も受け付けなかった」ことを意味する。** 一部だけが不正なときは 200 を返し、
/// 正しい分は格納される —— 1 件の恒久的な失敗が後続を永久に止めないため。
#[utoipa::path(post, path = "/ingest", request_body = Vec<IngestRequest>,
    responses((status = 200, body = Vec<IngestResult>), (status = 400, body = Vec<IngestResult>),
              (status = 401)))]
pub async fn ingest(
    State(app): State<App>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<(StatusCode, Json<Vec<IngestResult>>), (StatusCode, String)> {
    authorize(&app, &headers)?;

    let items: Vec<serde_json::Value> = match body {
        serde_json::Value::Array(a) => a,
        obj @ serde_json::Value::Object(_) => vec![obj],
        _ => return Err((StatusCode::BAD_REQUEST, "配列かオブジェクトを送る".into())),
    };
    if items.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "空の配列".into()));
    }

    let mut results = Vec::with_capacity(items.len());
    for item in &items {
        results.push(ingest_one(&app, item).await?);
    }

    // 1 件も受け付けなかったときだけ 400。**本文は同じ形のまま返す** ——
    // 収集側は状態符号ではなく 1 件ごとの結果を見て未送信を減らす。
    let code = if results.iter().any(|r| r.accepted) {
        StatusCode::OK
    } else {
        StatusCode::BAD_REQUEST
    };
    tracing::info!(
        kind = "ingest",
        sent = items.len(),
        accepted = results.iter().filter(|r| r.accepted).count(),
        "取り込み"
    );
    Ok((code, Json(results)))
}

/// 削除されていない記録を時刻順に返す（FR-50 のビュー越し）。
#[utoipa::path(get, path = "/events",
    responses((status = 200, body = Vec<EventRow>), (status = 401)))]
pub async fn events(
    State(app): State<App>,
    headers: HeaderMap,
) -> Result<Json<Vec<EventRow>>, (StatusCode, String)> {
    authorize(&app, &headers)?;
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

/// 未捕捉の異常がログに出ることを確かめるためだけの経路。既定では生えない。
async fn selftest_panic() -> &'static str {
    panic!("selftest: 意図的な異常")
}

fn internal<E: std::fmt::Display>(e: E) -> (StatusCode, String) {
    // 私的データはログに出さない（A-2）。出すのはエラーの種別だけ。
    // **応答に元のエラーを載せない** —— スキーマ名・接続先・値が呼び出し側へ漏れる。
    tracing::error!(kind = "db", detail = %e, "データベース操作に失敗");
    (StatusCode::INTERNAL_SERVER_ERROR, "internal error".into())
}

pub async fn run() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();
    std::panic::set_hook(Box::new(|info| {
        // 未捕捉の異常が黙って消えないようにする（製造準備 C）
        tracing::error!(kind = "panic", location = ?info.location(), "未捕捉の異常");
    }));

    let url = std::env::var("DATABASE_URL").context("DATABASE_URL が未設定")?;
    let token = std::env::var("API_TOKEN").context("API_TOKEN が未設定")?;
    if token.len() < 16 {
        anyhow::bail!("API_TOKEN が短すぎる（16 文字以上にする）");
    }
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await?;
    // 版の順に当てる。**足したら必ずここへ 1 行足す** ——
    // 当て忘れると、不変条件が本番だけ効いていない状態になる。
    for (name, sql) in [
        (
            "0001_envelope",
            include_str!("../../../migrations/0001_envelope.sql"),
        ),
        (
            "0002_immutable_collected",
            include_str!("../../../migrations/0002_immutable_collected.sql"),
        ),
    ] {
        sqlx::raw_sql(sql)
            .execute(&pool)
            .await
            .with_context(|| format!("マイグレーション {name} の適用に失敗"))?;
    }

    let mut app = Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .route("/ingest", post(ingest))
        .route("/events", get(events))
        .with_state(App { pool, token });

    // 未捕捉の異常がログに出ることを確かめるための経路。
    // **既定では生えない** —— 環境変数で明示的に開けたときだけ。
    if std::env::var("ASHIATO_SELFTEST_PANIC").as_deref() == Ok("1") {
        app = app.route("/selftest/panic", get(selftest_panic));
    }

    let addr = std::env::var("BIND").unwrap_or_else(|_| "127.0.0.1:8787".into());
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!(addr = %addr, "起動");
    axum::serve(listener, app).await?;
    Ok(())
}

/// この API の契約。**コードから生成する**（製造準備 A-1: 手書きしない）。
#[derive(Debug, utoipa::OpenApi)]
#[openapi(
    paths(ingest, events),
    components(schemas(IngestResult, IngestError, EventRow, ingest::IngestRequest)),
    info(
        title = "ashiato S-01",
        version = "0.1.0",
        description = "書き込みは 1 本の取り込み口に集約する。読みは PostgREST が別に自動生成する。"
    )
)]
pub struct ApiDoc;
