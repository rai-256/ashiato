// SPDX-License-Identifier: AGPL-3.0-only
//! S-01 バックエンド。書き込みは 1 本の取り込み口に集約する（製造準備 A-1）。
//!
//! 冪等の判定（FR-22）・原文の保存（FR-18）・稼働記録の更新（FR-33）は
//! この 1 か所を必ず通る。Kotlin と Rust の 2 実装が同じ口を叩くため、
//! 不変条件をサーバ側に置かないと守れない。
use anyhow::Context as _;
use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;

#[cfg(test)]
mod api_tests;
pub mod coverage;
pub mod heartbeat;
pub mod ingest;
#[cfg(test)]
pub mod testdb;

use coverage::DAY_TZ;
use ingest::{content_hash, IngestRequest};

/// 当てる版と、その中身。**足したらここへ 1 行足す** ——
/// 当て忘れると、不変条件が本番だけ効いていない状態になる。
/// `run()` もテストも同じ並びを使う（テストだけ古い schema、が起きないようにする）。
pub const MIGRATIONS: [(&str, &str); 6] = [
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
    (
        "0005_coverage_rebuild",
        include_str!("../../../migrations/0005_coverage_rebuild.sql"),
    ),
    (
        "0006_immutable_heartbeat",
        include_str!("../../../migrations/0006_immutable_heartbeat.sql"),
    ),
];

/// 版を順に当てる。**当て直しても壊れない**（`run()` は起動のたびに全部当てる）。
pub async fn migrate(pool: &sqlx::PgPool) -> anyhow::Result<()> {
    for (name, sql) in MIGRATIONS {
        sqlx::raw_sql(sql)
            .execute(pool)
            .await
            .with_context(|| format!("マイグレーション {name} の適用に失敗"))?;
    }
    Ok(())
}

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
        // **黙って断らない**（review/code.md の R27）。合言葉がずれた端末は 5 分ごとに
        // 401 を受け続け、画面には⑥「途絶」が並ぶ。それが「端末が死んだ」のか
        // 「合言葉がずれている」のかを分ける情報を、サーバは握っていながら捨てていた。
        // **出すのは「資格情報が有った／無かった」だけ** —— 値は載せない（製造準備 A-2）。
        tracing::warn!(
            kind = "unauthorized",
            credential_present = !given.is_empty(),
            "資格情報が一致しない"
        );
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
            .map_err(|e| internal_at("ingest.source_lookup", e))?;
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

    // **3 本を 1 トランザクションにまとめる**（review/code.md の R2）。
    // 別々の文にしていると、記録だけ入って稼働記録の加算が落ちた状態が作れる ——
    // そのあと収集側が再送しても記録は `duplicate` で弾かれ、加算は 0 のまま。
    // **その日の稼働記録は二度と戻らない**（引き直す経路が無い）。
    // 独立検証が実測で作って確かめている。
    let mut tx = app
        .pool
        .begin()
        .await
        .map_err(|e| internal_at("ingest.begin", e))?;

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
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| internal_at("ingest.event_insert", e))?;

    // 稼働記録は取り込みと同じ関門で更新する。別経路にすると
    // 「データが無いのは収集が止まっていたのか」が後から区別できなくなる（FR-33）。
    // **件数は新しく入った行だけ数える**（design D13）—— まとめ送りの部分失敗で
    // 成功分が再送されるので、重複まで数えると件数が実態から離れる。
    // 行そのものは重複でも立てる。「その日は収集が動いていた」は重複の到着でも真だから。
    //
    // **日は `Asia/Tokyo` で切る**（ST02 の深掘り Q2 / design D1）。記録に付いた
    // タイムゾーンでは切らない —— 東西の移動で 1 年が 364 日にも 366 日にもなり、
    // NFR-13 の分母がぶれる。当初の実装は UTC 固定だった。
    sqlx::query(&format!(
        "INSERT INTO core.coverage (user_id, logical_source, day, event_count)
         VALUES ($1, $2, ($3 AT TIME ZONE '{DAY_TZ}')::date, $4)
         ON CONFLICT (user_id, logical_source, day)
         DO UPDATE SET event_count = core.coverage.event_count + $4"
    ))
    .bind(req.user_id)
    .bind(&req.logical_source)
    .bind(req.event_time)
    .bind(i32::from(row.is_some()))
    .execute(&mut *tx)
    .await
    .map_err(|e| internal_at("ingest.coverage_upsert", e))?;

    // 収集開始日は**いちばん古い記録が作られた日**（FR-79 / 第 6 回 Q24 / 第 7 回 Q26）。
    // 重複でも当てる —— 同じ記録の再送でも「その日に取られた」ことは変わらない。
    coverage::touch_started_on(&mut *tx, &req.logical_source, req.event_time)
        .await
        .map_err(|e| internal_at("ingest.started_on", e))?;

    tx.commit()
        .await
        .map_err(|e| internal_at("ingest.commit", e))?;

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

// ------------------------------------------------------------------ 生存信号

/// 生存信号を断った理由。**受け取った値は載せない**（`IngestError` と同じ向き）。
#[derive(Debug, Clone, Copy, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum HeartbeatError {
    /// 項目が形として解釈できない（取得の試行回数・成功回数が無い場合を含む）
    Malformed,
    /// 登録簿に無い論理ソース
    UnknownSource,
    /// 原文が空、または DB に格納できないバイトを含む
    InvalidRaw,
    /// 取得できない状態を報告しながら、何が満たされていないかを持たない
    MissingBlockers,
    /// 取得の回数が負、または成功が試行を超える
    InvalidCounts,
}

impl From<heartbeat::Invalid> for HeartbeatError {
    fn from(v: heartbeat::Invalid) -> Self {
        match v {
            heartbeat::Invalid::Raw => Self::InvalidRaw,
            heartbeat::Invalid::Blockerless => Self::MissingBlockers,
            heartbeat::Invalid::Counts => Self::InvalidCounts,
        }
    }
}

/// 送った 1 件ごとの結果。**送った順に並ぶ**（`IngestResult` と同じ約束）。
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct HeartbeatResult {
    id: Option<uuid::Uuid>,
    /// 既に同じ 1 件があったか。再送しても行が増えないことの確認に使う
    duplicate: bool,
    /// 未送信から取り除いてよいか。収集側はこれだけを見る
    accepted: bool,
    error: Option<HeartbeatError>,
}

async fn heartbeat_one(
    app: &App,
    item: &serde_json::Value,
) -> Result<HeartbeatResult, (StatusCode, String)> {
    let sent_id = item
        .get("id")
        .and_then(|v| v.as_str())
        .and_then(|v| uuid::Uuid::parse_str(v).ok());

    let req: heartbeat::HeartbeatRequest = match serde_json::from_value(item.clone()) {
        Ok(r) => r,
        Err(_) => {
            // **元のエラーを載せない。** serde の文言は受け取った値を含むことがある
            tracing::warn!(kind = "hb_malformed", "解釈できない生存信号を断った");
            return Ok(HeartbeatResult {
                id: sent_id,
                duplicate: false,
                accepted: false,
                error: Some(HeartbeatError::Malformed),
            });
        }
    };
    if let Err(invalid) = req.validate() {
        return Ok(HeartbeatResult {
            id: Some(req.id),
            duplicate: false,
            accepted: false,
            error: Some(invalid.into()),
        });
    }

    let known: Option<(String,)> =
        sqlx::query_as("SELECT logical_source FROM core.source WHERE logical_source = $1")
            .bind(&req.logical_source)
            .fetch_optional(&app.pool)
            .await
            .map_err(|e| internal_at("heartbeat.source_lookup", e))?;
    if known.is_none() {
        return Ok(HeartbeatResult {
            id: Some(req.id),
            duplicate: false,
            accepted: false,
            error: Some(HeartbeatError::UnknownSource),
        });
    }

    let hash = heartbeat::content_hash(&req);
    // 記録側と同じく**1 トランザクション**（R2）。信号だけ入って収集開始日が動かないと、
    // その日が⑦「導入前」のまま残り、NFR-13 の分母からも落ちる。
    let mut tx = app
        .pool
        .begin()
        .await
        .map_err(|e| internal_at("heartbeat.begin", e))?;
    // **原文は素通し**（0003 と同じ理由。`jsonb` はキー順を変え、重複キーを落とす）。
    let row: Option<(uuid::Uuid,)> = sqlx::query_as(
        "INSERT INTO core.heartbeat
           (id, user_id, logical_source, device_id, emitted_at,
            capturable, blockers, attempts, successes, content_hash, raw)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
         ON CONFLICT (user_id, logical_source, content_hash) DO NOTHING
         RETURNING id",
    )
    .bind(req.id)
    .bind(req.user_id)
    .bind(&req.logical_source)
    .bind(&req.device_id)
    .bind(req.emitted_at)
    .bind(req.capturable)
    .bind(&req.blockers)
    .bind(req.attempts)
    .bind(req.successes)
    .bind(&hash)
    .bind(&req.raw)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| internal_at("heartbeat.insert", e))?;

    // 収集開始日は記録と同じ規則で動く（FR-79）—— **信号なら発信時刻の日**。
    coverage::touch_started_on(&mut *tx, &req.logical_source, req.emitted_at)
        .await
        .map_err(|e| internal_at("heartbeat.started_on", e))?;

    tx.commit()
        .await
        .map_err(|e| internal_at("heartbeat.commit", e))?;

    Ok(HeartbeatResult {
        id: Some(row.map_or(req.id, |(id,)| id)),
        duplicate: row.is_none(),
        accepted: true,
        error: None,
    })
}

/// 生存信号をまとめて受け取る（FR-78）。
///
/// **`/ingest` と統合しない**（design D9）—— `/ingest` は記録のエンベロープ
/// （`event_time` / `tz_id` / `schema_version` / `crs` …）を必須にしており、
/// 生存信号はそのどれも持たない。混ぜると片方のために必須の欄が緩む。
///
/// 400 は「1 件も受け付けなかった」ことを意味する（`/ingest` と同じ約束）。
#[utoipa::path(post, path = "/heartbeat", request_body = Vec<heartbeat::HeartbeatRequest>,
    responses((status = 200, body = Vec<HeartbeatResult>), (status = 400, body = Vec<HeartbeatResult>),
              (status = 401)))]
pub async fn heartbeat_post(
    State(app): State<App>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<(StatusCode, Json<Vec<HeartbeatResult>>), (StatusCode, String)> {
    authorize(&app, &headers)?;
    let items: Vec<serde_json::Value> = match body {
        serde_json::Value::Array(a) => a,
        obj @ serde_json::Value::Object(_) => vec![obj],
        _ => return Ok((StatusCode::BAD_REQUEST, Json(Vec::new()))),
    };
    if items.is_empty() {
        return Ok((StatusCode::BAD_REQUEST, Json(Vec::new())));
    }
    let mut results = Vec::with_capacity(items.len());
    for item in &items {
        results.push(heartbeat_one(&app, item).await?);
    }
    let code = if results.iter().any(|r| r.accepted) {
        StatusCode::OK
    } else {
        StatusCode::BAD_REQUEST
    };
    tracing::info!(
        kind = "heartbeat",
        sent = items.len(),
        accepted = results.iter().filter(|r| r.accepted).count(),
        "生存信号"
    );
    Ok((code, Json(results)))
}

// ------------------------------------------------------------------ 稼働状況

/// `GET /coverage` の絞り込み。
#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct CoverageQuery {
    from: chrono::NaiveDate,
    to: chrono::NaiveDate,
    /// 利用者。**省略すると絞らない**（単一利用者でも列は day one から持つ。FR-29 / 扉 #9）
    user_id: Option<uuid::Uuid>,
}

/// ソース × 日 の 7 状態を返す（FR-54）。**状態は行に焼かず導出する**（design D6）。
#[utoipa::path(get, path = "/coverage", params(CoverageQuery),
    responses((status = 200, body = Vec<coverage::SourceCoverage>), (status = 401)))]
pub async fn coverage_get(
    State(app): State<App>,
    headers: HeaderMap,
    Query(q): Query<CoverageQuery>,
) -> Result<Json<Vec<coverage::SourceCoverage>>, (StatusCode, String)> {
    authorize(&app, &headers)?;
    // **NFR-13 の 5 ソースの順で返す**（登録簿の並び順ではない）。画面の縦の並びがこれになる。
    let names: Vec<String> = coverage::must_sources()
        .into_iter()
        .map(|(n, _)| n)
        .collect();
    let out = coverage::of_sources(&app.pool, q.user_id, &names, q.from, q.to)
        .await
        .map_err(|e| internal_at("coverage.of_sources", e))?;
    Ok(Json(out))
}

/// `GET /coverage/achievement` の絞り込み。**期間は取らない**（第 6 回 Q23）——
/// 窓が仕様で決まったので、呼び出し側に委ねると合否が動く。
#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct AchievementQuery {
    user_id: Option<uuid::Uuid>,
}

/// 5 ソースの達成日数と分母、合否、確定か暫定か（NFR-13）。
#[utoipa::path(get, path = "/coverage/achievement", params(AchievementQuery),
    responses((status = 200, body = coverage::Achievement), (status = 401)))]
pub async fn achievement_get(
    State(app): State<App>,
    headers: HeaderMap,
    Query(q): Query<AchievementQuery>,
) -> Result<Json<coverage::Achievement>, (StatusCode, String)> {
    authorize(&app, &headers)?;
    let today = today_jst();
    let got = coverage::achievement(&app.pool, q.user_id, today, &coverage::must_sources())
        .await
        .map_err(|e| internal_at("coverage.achievement", e))?;
    Ok(Json(got))
}

/// いまの `Asia/Tokyo` の日付。**日境界の決定はここにも効く**（深掘り Q2）。
fn today_jst() -> chrono::NaiveDate {
    // JST は 1951 年以降 夏時間を持たない固定の +09:00 なので、ずらしてから日を取れば
    // PostgreSQL の `AT TIME ZONE 'Asia/Tokyo'` と同じ日になる。
    // **失敗しうる経路を作らない** —— 落ちる代わりに UTC の日を返す実装にすると、
    // 日境界が黙って 9 時間ずれる（NFR-13 の分母がぶれる）。
    (chrono::Utc::now() + chrono::Duration::hours(9)).date_naive()
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
    .map_err(|e| internal_at("events.select", e))?;
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

/// DB の失敗を畳む。**ログに出すのは SQLSTATE と操作名だけ**（A-2 / design D20）。
///
/// `sqlx::Error` の Display は `error returned from database: <PostgreSQL の本文>` で、
/// PostgreSQL は `invalid input syntax for type ...: "<値>"` のように**入力値を本文に含める**。
/// 種別＝ SQLSTATE なら値を含まない。
///
/// **どの操作で落ちたかを添える**（review/code.md の R26）。
/// `ingest_one` だけで 3 本、`heartbeat_one` で 3 本、読み出しでさらに数本の SQL が
/// 同じ 1 行に畳まれていた。SQLSTATE `08006` が出たとき、それが
/// 「記録は入ったが稼働記録が落ちた」（R2）なのか登録簿の照会が落ちただけなのかを
/// ログから区別できない。**操作名は値ではない**ので、A-2 は出さない理由にならない。
fn internal_at(op: &'static str, e: sqlx::Error) -> (StatusCode, String) {
    let code = e
        .as_database_error()
        .and_then(|d| d.code())
        .map(|c| c.into_owned())
        .unwrap_or_else(|| "unknown".into());
    tracing::error!(kind = "db", op = op, sqlstate = %code, "データベース操作に失敗");
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
    migrate(&pool).await?;

    let mut app = Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .route("/ingest", post(ingest))
        .route("/heartbeat", post(heartbeat_post))
        .route("/events", get(events))
        .route("/coverage", get(coverage_get))
        .route("/coverage/achievement", get(achievement_get))
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
    paths(ingest, heartbeat_post, events, coverage_get, achievement_get),
    components(schemas(
        IngestResult,
        IngestError,
        EventRow,
        ingest::IngestRequest,
        HeartbeatResult,
        HeartbeatError,
        heartbeat::HeartbeatRequest,
        coverage::SourceCoverage,
        coverage::DayCell,
        coverage::Interval,
        coverage::DayState,
        coverage::Band,
        coverage::Subject,
        coverage::Achievement,
        coverage::SourceAchievement,
    )),
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
