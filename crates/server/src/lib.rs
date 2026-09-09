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

/// 合言葉を突き合わせる。**一致した長さから内容が推測されない**
/// （spec「資格情報の比較を、一致した長さから内容が推測されない方法で行う」）。
///
/// 長さが同じなら、**最初の 1 バイトが違っても全部違っても同じ回数の比較を行う** ——
/// 早期 return を書くと、掛かった時間から「どこまで合っていたか」が漏れ、
/// 合言葉を 1 バイトずつ削り出せる。
///
/// **`subtle` に委ねてある**（review R7）。畳み込みを手で書いていたときは
/// `given == expected` に戻しても `cargo test` も `tools/smoke.sh` も緑のままで、
/// **この性質は単体テストでは捕まえられない**（時間を測らない限り観測できない）。
/// 早期打ち切りが書けない型に置き換えて、性質を構造で保証する。
///
/// 長さの一致は先に見る。**全体の長さは漏れるが、それは合言葉の中身ではない** ——
/// spec が禁じているのは「一致した長さ（＝どこまで合っていたか）」からの推測。
pub fn token_matches(given: &str, expected: &str) -> bool {
    use subtle::ConstantTimeEq as _;
    given.len() == expected.len() && given.as_bytes().ct_eq(expected.as_bytes()).into()
}

/// 合言葉を確かめる。無ければ 401。
fn authorize(app: &App, headers: &HeaderMap) -> Result<(), (StatusCode, String)> {
    let given = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or_default();
    let ok = token_matches(given, &app.token);
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
    /// 原文が空、または DB に格納できないバイトを含む（`ingest::Invalid::Raw`）
    InvalidRaw,
    /// 「収集した」記録なのに端末識別子が無い（`ingest::Invalid::DeviceId`）
    MissingDeviceId,
}

impl From<ingest::Invalid> for IngestError {
    fn from(v: ingest::Invalid) -> Self {
        match v {
            ingest::Invalid::Origin => Self::UnknownOrigin,
            ingest::Invalid::Raw => Self::InvalidRaw,
            ingest::Invalid::DeviceId => Self::MissingDeviceId,
        }
    }
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
    /// 原文は**文字列**（design D16）。JSON 型で持つと DB が並びと表記を正規化する
    raw: String,
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

    // 受け取り時の検査はアプリ層で閉じる（design D5）。DB の制約に任せると 500 になり、
    // 呼び出し側から「自分の要求が悪い」と分からない。
    // **500 はまとめ送り全体を落とす** —— 1 件の恒久的な失敗が後続を永久に止める（design D20）。
    if let Err(invalid) = req.validate() {
        return Ok(IngestResult::rejected(Some(req.id), invalid.into()));
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
        // **本文の形は常に「1 件ごとの結果の配列」**（docs/collector-contract.md §状態符号）。
        // 平文を返すと収集側がパースに失敗し、`unreadable_response` として
        // 状態符号の意味を失う（結果として何も取り除けない）。
        _ => return Ok((StatusCode::BAD_REQUEST, Json(Vec::new()))),
    };
    if items.is_empty() {
        return Ok((StatusCode::BAD_REQUEST, Json(Vec::new())));
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
            String,
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

/// DB の失敗を畳む。**ログに出すのは SQLSTATE だけ**（A-2 / design D20）。
///
/// `sqlx::Error` の Display は `error returned from database: <PostgreSQL の本文>` で、
/// PostgreSQL は `invalid input syntax for type ...: "<値>"` のように**入力値を本文に含める**。
/// 原文が `text` になって格納の失敗経路が増えた（design D16）ぶん、ここから私的データが
/// 漏れる筋が太くなっている。種別＝ SQLSTATE なら値を含まない。
fn internal(e: sqlx::Error) -> (StatusCode, String) {
    let code = e
        .as_database_error()
        .and_then(|d| d.code())
        .map(|c| c.into_owned())
        .unwrap_or_else(|| "unknown".into());
    tracing::error!(kind = "db", sqlstate = %code, "データベース操作に失敗");
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
        (
            "0003_raw_text",
            include_str!("../../../migrations/0003_raw_text.sql"),
        ),
        (
            "0004_immutable_origin",
            include_str!("../../../migrations/0004_immutable_origin.sql"),
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

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    /// 合言葉の突き合わせが**正しい**ことを固定する（spec / review R7）。
    ///
    /// **定数時間であることはここでは確かめられない** —— 時間を測らない限り
    /// `==` と区別が付かない。その性質は `subtle::ConstantTimeEq` が構造で持っている
    /// （`token_matches` の実装を見よ）。ここが見るのは可否の正しさだけ。
    #[test]
    fn token_comparison_is_correct() {
        let token = "smoke-token-0123456789abcdef";
        assert!(token_matches(token, token));

        // 同じ長さで、違う位置が 1 バイトだけ
        let head = format!("X{}", &token[1..]);
        let tail = format!("{}X", &token[..token.len() - 1]);
        let all = "X".repeat(token.len());
        for wrong in [head.as_str(), tail.as_str(), all.as_str()] {
            assert_eq!(wrong.len(), token.len());
            assert!(!token_matches(wrong, token), "{wrong} が通っている");
        }

        // 長さ違いは通らない（前方一致で通す実装への回帰を止める）
        assert!(!token_matches(&token[..token.len() - 1], token));
        assert!(!token_matches(&format!("{token}X"), token));
        assert!(!token_matches("", token));
    }

    /// 空の合言葉を設定した運用でも、空のヘッダが通ってはいけない…
    /// わけではない（`run()` が 16 文字未満を拒む）。ここは**長さ 0 同士が一致する**ことだけ確かめ、
    /// 短い合言葉を止めるのは起動時の検査だと明示する。
    #[test]
    fn empty_token_is_rejected_at_startup_not_here() {
        assert!(token_matches("", ""));
    }
}
