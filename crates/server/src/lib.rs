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
/// 冪等の判定・更新と履歴・削除済みの保護（ST03）。
#[cfg(test)]
mod dedup_tests;
pub mod heartbeat;
pub mod ingest;
/// 登録簿の本物の行を、全移行を当てた後の状態で見る（ST07 / design D2）。
#[cfg(test)]
mod registry_tests;
/// 位置から滞在を切り出す判定（ST16 / FR-76）。DB に触らない。
pub mod stay;
/// 滞在の作り直しと 1 日の並び（ST16 / FR-31 / FR-50）。
pub mod stay_store;
/// 滞在の移行・作り直し・読み出し（ST16）。
#[cfg(test)]
mod stay_tests;
#[cfg(test)]
pub mod testdb;

use coverage::DAY_TZ;
use ingest::{content_hash, IngestRequest};

/// 当てる版と、その中身。**足したらここへ 1 行足す** ——
/// 当て忘れると、不変条件が本番だけ効いていない状態になる。
/// `run()` もテストも同じ並びを使う（テストだけ古い schema、が起きないようにする）。
pub const MIGRATIONS: [(&str, &str); 13] = [
    (
        "202609081618_envelope",
        include_str!("../../../migrations/202609081618_envelope.sql"),
    ),
    (
        "202609082001_immutable_collected",
        include_str!("../../../migrations/202609082001_immutable_collected.sql"),
    ),
    (
        "202609092315_raw_text",
        include_str!("../../../migrations/202609092315_raw_text.sql"),
    ),
    (
        "202609100000_immutable_origin",
        include_str!("../../../migrations/202609100000_immutable_origin.sql"),
    ),
    (
        "202609111111_coverage_rebuild",
        include_str!("../../../migrations/202609111111_coverage_rebuild.sql"),
    ),
    (
        "202609111112_immutable_heartbeat",
        include_str!("../../../migrations/202609111112_immutable_heartbeat.sql"),
    ),
    (
        "202609112113_source_lifecycle",
        include_str!("../../../migrations/202609112113_source_lifecycle.sql"),
    ),
    (
        "202609120940_source_columns",
        include_str!("../../../migrations/202609120940_source_columns.sql"),
    ),
    (
        "202609120941_event_columns",
        include_str!("../../../migrations/202609120941_event_columns.sql"),
    ),
    // **索引の作り替えより先に、この配列より前で `ON CONFLICT` が直っていること**
    // （design D2）。部分索引には述語を文に書かないと当たらず、順序が逆だと
    // その間の取り込みが全件 500 になる。
    (
        "202609120942_dedup_indexes",
        include_str!("../../../migrations/202609120942_dedup_indexes.sql"),
    ),
    (
        "202609120943_version_and_ledger",
        include_str!("../../../migrations/202609120943_version_and_ledger.sql"),
    ),
    (
        "202609120944_gates",
        include_str!("../../../migrations/202609120944_gates.sql"),
    ),
    // 滞在の基準と吸収の台帳（ST16 / design D10）。滞在そのものは `core.event` に入る
    (
        "202609142125_stays",
        include_str!("../../../migrations/202609142125_stays.sql"),
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
    /// 位置を受け入れた日の滞在を作り直す口（ST16 / design D5）。
    stays: StayRebuilder,
}

impl App {
    /// テスト用。作り直しの口は本物を使う。
    #[cfg(test)]
    pub(crate) fn for_test(pool: sqlx::PgPool, token: &str) -> Self {
        Self {
            pool,
            token: token.into(),
            stays: StayRebuilder::real(),
        }
    }
}

/// 作り直しの口が返す future。
pub type RebuildFuture =
    std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<()>> + Send>>;

/// 1 日ぶんの滞在を作り直す口（design D5）。
///
/// **差し替えられる形にしてある** —— 作り直しが失敗しても取り込みの応答が変わらないことを、
/// 表の権限を剥がさずに確かめるため（テストは開発 DB を共有するので、剥がすと他のテストの作り直しも落ちる。R21）。
#[derive(Clone)]
pub struct StayRebuilder(
    std::sync::Arc<
        dyn Fn(sqlx::PgPool, uuid::Uuid, chrono::NaiveDate) -> RebuildFuture + Send + Sync,
    >,
);

impl StayRebuilder {
    /// 本物（`stay_store::rebuild_day`）。
    pub fn real() -> Self {
        Self::from_fn(|pool, user, day| {
            Box::pin(async move { stay_store::rebuild_day(&pool, user, day).await.map(|_| ()) })
        })
    }

    /// 任意の関数から作る（テストで失敗を起こすため）。
    pub fn from_fn(
        f: impl Fn(sqlx::PgPool, uuid::Uuid, chrono::NaiveDate) -> RebuildFuture + Send + Sync + 'static,
    ) -> Self {
        Self(std::sync::Arc::new(f))
    }
}

impl std::fmt::Debug for StayRebuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("StayRebuilder")
    }
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
    /// 登録簿が「記録ごと」と宣言したソースなのに外部識別子が無い（ST03 / 深掘り Q4 / Q18）
    MissingExternalId,
    /// 外部識別子（または対象の識別子）が空文字（ST03 / R12）
    EmptyExternalId,
    /// 既に格納された記録と同じ収集側の識別子で、別の記録が届いた（ST03 / 深掘り Q5）。
    ///
    /// **正常系では一生出ない** —— Q14 で収集側の識別子は毎回新しく振ると決めたので、
    /// この応答は**収集側の採番が壊れていることの印**として働く。
    IdReused,
}

impl From<ingest::Invalid> for IngestError {
    fn from(v: ingest::Invalid) -> Self {
        match v {
            ingest::Invalid::Origin => Self::UnknownOrigin,
            ingest::Invalid::Raw => Self::InvalidRaw,
            ingest::Invalid::DeviceId => Self::MissingDeviceId,
            ingest::Invalid::ExternalId => Self::EmptyExternalId,
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

/// 内容の鍵が同じ複数行を **1 件として読む形**（深掘り Q8 / 移行 `*_version_and_ledger`）。
///
/// 外部識別子を優先すると同じ内容の行が複数立ちうるので（Q6）、読む側が畳めないと
/// 画面と分析が二重になる。
///
/// **適用は後続 Story。** ST03 が作るのは置き場だけで、実際に使うのは
/// 閲覧・検索・AI・書き出しの各 Story（Q8 / レビュー R26）。期間指定の上書きは ST12。
/// ここで 4 か所へ当てにいくと、それぞれの Story が持つ判断を先取りすることになる。
pub const FOLDED_VIEW: &str = "core.event_folded";

/// 履歴に残した前の版を、**親と束ねた形でのみ**読む（深掘り Q21 / R49 / design D6）。
///
/// 履歴表を直に引くと、親の感度と削除が効かないまま前の版の本文が出る（実測）。
/// 本表が `core.event_live` で同じ危険を塞いでいるのと同じ手当て（製造準備 A-3）。
pub const VERSION_VIEW: &str = "core.event_version_live";

/// 既に格納されている 1 行のうち、取り込みの判断に要るもの。
///
/// **列名で受ける**（`FromRow`）。位置で受ける組にしていたときは 8 要素の型注釈になり、
/// 並びを 1 つ入れ替えても型が合ってしまう（`content_hash` と `raw` はどちらも `String`）。
#[derive(sqlx::FromRow)]
struct Stored {
    id: uuid::Uuid,
    content_hash: String,
    deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    source_updated_at: Option<chrono::DateTime<chrono::Utc>>,
    event_time: chrono::DateTime<chrono::Utc>,
    raw: String,
    payload: serde_json::Value,
    external_ref: Option<String>,
    /// 出来事の時刻を読むための欄（R111）。更新で `event_time` と一緒に動くので、
    /// **前の値を履歴へ残すためにここで受ける**
    tz_offset_min: i32,
    tz_id: String,
    schema_version: i32,
    unit_system: String,
    crs: String,
}

/// 外部識別子をどの列へ置くか（深掘り Q24 / design D13）。
///
/// **「記録ごと」と宣言したソースだけが `external_id` を使う。**
/// 「対象ごと」「無し」のソースから識別子が届いたときは `external_ref` へ回す ——
/// `external_id` に入れると `event_dedup_ext` に載り、**同じ対象の 2 件目が
/// 一意違反で落ちる**（実測 R42）。捨てずに回すのは、捨てたものは復元できないから。
fn place_identifiers(
    kind: ingest::ExternalIdKind,
    req: &IngestRequest,
) -> (Option<String>, Option<String>) {
    if kind.deduplicates_by_external_id() {
        (req.external_id.clone(), req.external_ref.clone())
    } else {
        (
            None,
            req.external_ref.clone().or_else(|| req.external_id.clone()),
        )
    }
}

/// 1 件を格納して結果を返す。**呼び出し側の誤りは Err ではなく `IngestResult` で返す** ——
/// まとめ送りの一部が不正でも、他の件は格納しなければならない（design D9）。
/// Err になるのはサーバ側の失敗（DB）だけ。
///
/// **1 件 1 トランザクション**（design D3）。Q10 / Q23 の門は制約トリガで
/// **COMMIT の瞬間に落ちる**ので、まとめ送りを 1 トランザクションにすると
/// 1 件の失敗が全件を巻き戻す —— ST01 の D9 / D19 に正面から反する。
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
    // **併せて外部識別子の粒度を読む**（深掘り Q13 / Q18）。
    let known: Option<(String, String)> = sqlx::query_as(
        "SELECT logical_source, external_id_kind FROM core.source WHERE logical_source = $1",
    )
    .bind(&req.logical_source)
    .fetch_optional(&app.pool)
    .await
    .map_err(|e| internal_at("ingest.source_lookup", e))?;
    let Some((_, kind_text)) = known else {
        return Ok(IngestResult::rejected(
            Some(req.id),
            IngestError::UnknownSource,
        ));
    };
    let kind = ingest::ExternalIdKind::from_registry(&kind_text);

    // **「記録ごと」と宣言したソースで識別子を欠けば受け付けない**（深掘り Q4 / Q18）。
    // 宣言を欠いたソースは `Record` に倒れるので、**書き忘れも同じくここで断られる**（Q16）——
    // 緩い側に倒すと、識別子なしで入った記録に後から識別子を足す手段が無い。
    if kind.deduplicates_by_external_id() && req.external_id.is_none() {
        return Ok(IngestResult::rejected(
            Some(req.id),
            IngestError::MissingExternalId,
        ));
    }

    let (external_id, external_ref) = place_identifiers(kind, &req);
    let hash = content_hash(&req);
    // **`payload` だけを NFC に揃える。`raw` は受け取ったまま送る**（design D2 / FR-18）。
    // 原文のバイト列は一度変換すると二度と戻らない。
    let payload = ingest::to_nfc(&req.payload);

    // **3 本を 1 トランザクションにまとめる**（review/code.md の R2）。
    // 別々の文にしていると、記録だけ入って稼働記録の加算が落ちた状態が作れる ——
    // そのあと収集側が再送しても記録は `duplicate` で弾かれ、加算は 0 のまま。
    // **その日の稼働記録は二度と戻らない**（引き直す経路が無い）。
    let mut tx = app
        .pool
        .begin()
        .await
        .map_err(|e| internal_at("ingest.begin", e))?;

    // **収集側の識別子の使い回しを格納の前に断る**（深掘り Q5）。
    // `id` は主キーなので、放っておくと重複違反で 500 になり**まとめ送り全体が落ちる**。
    let by_id: Option<(String, Option<String>, uuid::Uuid, String)> = sqlx::query_as(
        "SELECT content_hash, external_id, user_id, logical_source FROM core.event WHERE id = $1",
    )
    .bind(req.id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| internal_at("ingest.id_lookup", e))?;
    if let Some((stored_hash, stored_ext, stored_user, stored_source)) = by_id {
        // 同じ 1 件の再送だけを通す。**それ以外は「同じ識別子で別の記録」**
        //
        // **外部識別子で畳むソースでは、内容の一致を求めない**（R112）。
        // 求めていたときは、更新で `content_hash` が動いた行に**元の到着が再送される**と
        // `id_reused` が誤爆した（応答を取り落とした端末が再送する正常な経路）——
        // 運用者は存在しない「端末の採番破損」を追い、収集側はその 1 件を恒久的な拒否として捨てる。
        // Q5 の 400 が印すべきものは「同じ `id` で**別の記録**」なので、
        // 外部識別子が一致するなら同じ記録の再送として通すのが正しい。
        let same_key = stored_user == req.user_id
            && stored_source == req.logical_source
            && stored_ext.as_deref() == external_id.as_deref();
        let same_record = same_key && (external_id.is_some() || stored_hash == hash);
        if !same_record {
            tracing::warn!(
                kind = "id_reused",
                logical_source = %req.logical_source,
                "同じ収集側の識別子で別の記録が届いた"
            );
            return Ok(IngestResult::rejected(Some(req.id), IngestError::IdReused));
        }
    }

    // **削除済みの内容は、どの外部識別子で届いても入れない**（深掘り Q3 / Q11 / Q19）。
    // 撃つのは外部識別子で畳む記録のときだけ（design D9）—— 畳まない記録は
    // `event_dedup_hash` が削除済みの行も含めて弾くので、この問い合わせは要らない。
    //
    // **ここで早く返さない。** 取り込まなかった 1 件でも「その日は収集が動いていた」は
    // 真なので、稼働記録の行と収集開始日は下で当てる（FR-33 / 扉 #14）。
    let mut blocked_by_deleted: Option<uuid::Uuid> = None;
    if ingest::needs_deleted_check(kind, external_id.as_deref()) {
        let erased: Option<(uuid::Uuid,)> = sqlx::query_as(
            "SELECT id FROM core.event
              WHERE user_id = $1 AND logical_source = $2 AND content_hash = $3
                AND deleted_at IS NOT NULL
              LIMIT 1",
        )
        .bind(req.user_id)
        .bind(&req.logical_source)
        .bind(&hash)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| internal_at("ingest.deleted_lookup", e))?;
        blocked_by_deleted = erased.map(|(id,)| id);
        if blocked_by_deleted.is_some() {
            // **黙って落とさない**（R109 / HIGH-5）。取り込まなかったことが応答からは
            // 「重複」と区別できないので、サーバ側のログが唯一の観測点になる。
            // **値は載せない**（製造準備 A-2）—— 出すのはソース名と種別だけ。
            tracing::warn!(
                kind = "ingest_blocked_deleted",
                logical_source = %req.logical_source,
                has_external_id = external_id.is_some(),
                "削除済みの本文と一致したので取り込まなかった"
            );
        }
    }

    // **索引が 2 段なので、撃つ文も 2 通り**（design D1 / D2）。
    // `ON CONFLICT` は部分索引に対して**述語を文に書かないと当たらない** ——
    // 述語なしの `ON CONFLICT (logical_source, content_hash)` は
    // `there is no unique or exclusion constraint matching …` で文として落ちる（実測）。
    let insert_sql = if external_id.is_some() {
        "INSERT INTO core.event
           (id, user_id, logical_source, external_id, external_ref, device_id, origin,
            event_time, tz_offset_min, tz_id, schema_version, unit_system, crs,
            content_hash, raw, payload, source_updated_at)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17)
         ON CONFLICT (user_id, logical_source, external_id) WHERE external_id IS NOT NULL
           DO NOTHING
         RETURNING id"
    } else {
        "INSERT INTO core.event
           (id, user_id, logical_source, external_id, external_ref, device_id, origin,
            event_time, tz_offset_min, tz_id, schema_version, unit_system, crs,
            content_hash, raw, payload, source_updated_at)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17)
         ON CONFLICT (user_id, logical_source, content_hash) WHERE external_id IS NULL
           DO NOTHING
         RETURNING id"
    };
    let row: Option<(uuid::Uuid,)> = if blocked_by_deleted.is_some() {
        None
    } else {
        sqlx::query_as(insert_sql)
            .bind(req.id)
            .bind(req.user_id)
            .bind(&req.logical_source)
            .bind(&external_id)
            .bind(&external_ref)
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
            .bind(req.source_updated_at)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| internal_at("ingest.event_insert", e))?
    };

    // 畳まれたときは、**格納されている行**を引き直す（spec「結果に載せる識別子を、
    // 格納されている記録の識別子とする」/ R11）。送り主が名乗った識別子をそのまま返すと、
    // **DB に無い識別子が受理として返り**、後から突き合わせる手段が無い。
    // **止めたときも、その鍵で格納されている行を引き直す**（R106）。
    // 引かずに削除済みの行の識別子を返していたときは、**生きている行への更新を
    // 止めた場合に「利用者が消した別の行」の識別子が受理として返っていた** ——
    // spec の「結果に載せる識別子を、格納されている記録の識別子とする」に反し、
    // `core.event_live` からはその識別子が引けない。
    let folded = match row {
        Some(_) => None,
        None => load_stored(&mut tx, &req, &hash, external_id.as_deref()).await?,
    };

    // 外部サービス由来の更新かどうか（深掘り Q1）。**内容の鍵が違えば更新**
    let mut outcome = Update::NotApplicable;
    if let Some(stored) = &folded {
        if blocked_by_deleted.is_some() {
            outcome = Update::SkippedDeletedContent;
        } else if stored.content_hash != hash && external_id.is_some() {
            outcome = apply_external_update(&mut tx, stored, &req, &hash, &payload).await?;
        } else if external_id.is_some() {
            // **内容は同じでも更新時刻だけ新しい到着で、水位を進める**（R113）。
            // 進めないと、その後に届く**中間の時刻**の版が「新しい」と判定されて
            // 内容が過去へ動く（より新しい版を一度見ているのに戻る）。
            // 内容が変わらないので門は素通しし、履歴も要らない。
            outcome = advance_watermark(&mut tx, stored, &req).await?;
        }
    }
    if outcome.was_skipped() {
        // **捨てた側もログに残す**（R110）。当たった側だけを出していたときは、
        // 「本当の重複」「更新を当てた」「削除済みなので捨てた」「古いので捨てた」の
        // 4 つが応答から区別できないうえ、**捨てた側だけログにも残らなかった。**
        tracing::warn!(
            kind = "ingest_update_skipped",
            logical_source = %req.logical_source,
            reason = outcome.reason(),
            "外部サービス由来の更新を当てなかった"
        );
    }

    // 稼働記録は取り込みと同じ関門で更新する。別経路にすると
    // 「データが無いのは収集が止まっていたのか」が後から区別できなくなる（FR-33）。
    // **件数は新しく入った行だけ数える**（design D13 / 正典「新しく入った記録の数」）——
    // **更新は「新しく入った」ではない**ので 0 件。
    // 行そのものは重複でも立てる。「その日は収集が動いていた」は重複の到着でも真だから。
    //
    // **日は `Asia/Tokyo` で切る**（ST02 の深掘り Q2 / design D1）。
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
    coverage::touch_started_on(
        &mut *tx,
        &req.logical_source,
        req.event_time,
        coverage::Arrival::Record,
    )
    .await
    .map_err(|e| internal_at("ingest.started_on", e))?;

    // **門はここで落ちる**（design D4）。制約トリガは `DEFERRABLE INITIALLY DEFERRED` で、
    // 履歴を書かない書き換えは COMMIT の瞬間に拒まれる。
    tx.commit()
        .await
        .map_err(|e| internal_at("ingest.commit", e))?;

    if matches!(outcome, Update::Applied) {
        tracing::info!(
            kind = "ingest_update",
            logical_source = %req.logical_source,
            "外部サービス由来の更新で前の版を履歴へ移した"
        );
    }
    // **`accepted: true` に載せる識別子は、必ず DB から引いたもの**（R101 / spec の MODIFIED）。
    // 送り主が名乗った識別子を返す分岐を 1 つも残さない —— 残すと、
    // spec 自身が名指しした事故（「DB に無い識別子が受理として返っていた」）が
    // 到達可能になった瞬間に復活する。
    Ok(match (row, folded, blocked_by_deleted) {
        (Some((id,)), _, _) => IngestResult::stored(id, false),
        (None, Some(stored), _) => IngestResult::stored(stored.id, true),
        // その鍵の行は無いが、削除済みの本文と一致したので取り込まなかった。
        // **受理として返す**（Q3）—— 返さないと同じ 1 件が永久に送られ続ける
        (None, None, Some(id)) => IngestResult::stored(id, true),
        // 畳まれたのに引き直せず、削除済みでもない。**構造上ここには来ない**
        // （`load_stored` が `None` を返すのは行が無いときだけで、そのときは挿入が通る）。
        // 来たら 500 にする —— 黙って送り主の識別子を返すより、うるさく落ちるほうがよい
        (None, None, None) => {
            return Err(internal_at(
                "ingest.folded_row_vanished",
                sqlx::Error::RowNotFound,
            ))
        }
    })
}

/// その鍵で既に格納されている行を引き直す。**無ければ `None`。**
///
/// 外部識別子で畳んだときはその識別子で、内容の鍵で畳んだときは鍵で引く ——
/// **畳んだ索引と同じ条件で引かないと、別の行を指しうる**。
///
/// 挿入が畳まれたときは必ず在る。**削除済みの本文で止めたときは無いことがある**
/// （その外部識別子の行がまだ無い場合）—— そこで `fetch_one` にしていると 500 になる。
async fn load_stored(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    req: &IngestRequest,
    hash: &str,
    external_id: Option<&str>,
) -> Result<Option<Stored>, (StatusCode, String)> {
    let sql = if external_id.is_some() {
        "SELECT id, content_hash, deleted_at, source_updated_at, event_time, raw, payload,
                external_ref, tz_offset_min, tz_id, schema_version, unit_system, crs
           FROM core.event
          WHERE user_id = $1 AND logical_source = $2 AND external_id = $3"
    } else {
        "SELECT id, content_hash, deleted_at, source_updated_at, event_time, raw, payload,
                external_ref, tz_offset_min, tz_id, schema_version, unit_system, crs
           FROM core.event
          WHERE user_id = $1 AND logical_source = $2 AND content_hash = $3
            AND external_id IS NULL"
    };
    let key = external_id.map_or(hash, |e| e);
    sqlx::query_as::<_, Stored>(sql)
        .bind(req.user_id)
        .bind(&req.logical_source)
        .bind(key)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|e| internal_at("ingest.stored_lookup", e))
}

/// 外部サービス由来の更新を当てたか、当てなかったならなぜか（R110）。
///
/// **当てなかったことが応答から見えない**ので、理由を持ち帰ってログに出す ——
/// 「本当の重複」「更新を当てた」「削除済みなので捨てた」「古いので捨てた」が
/// どれも `accepted: true` / `duplicate: true` で返る。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Update {
    /// 更新の経路ではない（新規の挿入、または内容が同じ再送）
    NotApplicable,
    /// 当てた。前の版は履歴にある
    Applied,
    /// 内容は同じで、外部サービス側の更新時刻だけを進めた
    WatermarkAdvanced,
    /// 削除済みの行だった（Q11。削除が勝つ）
    SkippedDeleted,
    /// 新しい内容が、別の削除済みの本文と一致した（Q19）
    SkippedDeletedContent,
    /// 届いた更新時刻が保存済みより古い（Q20）
    SkippedStale,
}

impl Update {
    fn was_skipped(self) -> bool {
        matches!(
            self,
            Self::SkippedDeleted | Self::SkippedDeletedContent | Self::SkippedStale
        )
    }

    fn reason(self) -> &'static str {
        match self {
            Self::NotApplicable => "not_applicable",
            Self::Applied => "applied",
            Self::WatermarkAdvanced => "watermark_advanced",
            Self::SkippedDeleted => "deleted_row",
            Self::SkippedDeletedContent => "deleted_content",
            Self::SkippedStale => "stale",
        }
    }
}

/// 内容は同じで、外部サービス側の更新時刻だけが新しい到着で**水位を進める**（R113）。
///
/// 進めないと、その後に届く**中間の時刻**の版が「新しい」と判定されて内容が過去へ動く。
/// 内容が変わらないので門（design D4）は素通しし、履歴も要らない。
async fn advance_watermark(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    stored: &Stored,
    req: &IngestRequest,
) -> Result<Update, (StatusCode, String)> {
    let Some(incoming) = req.source_updated_at else {
        return Ok(Update::NotApplicable);
    };
    if stored.deleted_at.is_some() || stored.source_updated_at.is_some_and(|k| incoming <= k) {
        return Ok(Update::NotApplicable);
    }
    sqlx::query("UPDATE core.event SET source_updated_at = $2 WHERE id = $1")
        .bind(stored.id)
        .bind(incoming)
        .execute(&mut **tx)
        .await
        .map_err(|e| internal_at("ingest.watermark_update", e))?;
    Ok(Update::WatermarkAdvanced)
}

/// 外部サービス由来の更新を当てる（深掘り Q1 / Q11 / Q20）。
///
/// **前の版を履歴へ書いてから本表を書き換える。** 門（design D4）は
/// 同じトランザクションに**更新前の版**の履歴行があることを COMMIT の瞬間に見るので、
/// 履歴を書き忘れた書き換えは DB の側で落ちる。
async fn apply_external_update(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    stored: &Stored,
    req: &IngestRequest,
    hash: &str,
    payload: &serde_json::Value,
) -> Result<Update, (StatusCode, String)> {
    // **削除が勝つ**（Q11）。削除済みの行は外部からの更新でも書き換えない ——
    // 更新は削除の印を見ないので、放っておくと 1 行足すだけで復活する（実測）。
    if stored.deleted_at.is_some() {
        return Ok(Update::SkippedDeleted);
    }
    // **古い到着では書き換えない**（Q20）。往復のたびに履歴が無限に積むのを止める。
    // **同じ更新時刻で内容だけ違う到着は「新しい」として扱う**（`>=`。design D8）——
    // `>` にすると `accepted` を返しながら内容が変わらず、応答から見えない。
    if let (Some(incoming), Some(known)) = (req.source_updated_at, stored.source_updated_at) {
        if incoming < known {
            return Ok(Update::SkippedStale);
        }
    }

    // 前の版を履歴へ。**原文はそのまま**（FR-18。`raw` は `text` なのでバイト単位で残る）
    sqlx::query(
        "INSERT INTO core.event_version
           (event_id, user_id, logical_source, version_no, event_time, content_hash,
            raw, payload, source_updated_at, external_ref,
            tz_offset_min, tz_id, schema_version, unit_system, crs)
         SELECT $1, $2, $3,
                coalesce(max(v.version_no), 0) + 1,
                $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14
           FROM core.event_version v WHERE v.event_id = $1",
    )
    .bind(stored.id)
    .bind(req.user_id)
    .bind(&req.logical_source)
    .bind(stored.event_time)
    .bind(&stored.content_hash)
    .bind(&stored.raw)
    .bind(&stored.payload)
    .bind(stored.source_updated_at)
    .bind(&stored.external_ref)
    .bind(stored.tz_offset_min)
    .bind(&stored.tz_id)
    .bind(stored.schema_version)
    .bind(&stored.unit_system)
    .bind(&stored.crs)
    .execute(&mut **tx)
    .await
    .map_err(|e| internal_at("ingest.version_insert", e))?;

    // **更新時刻を持たない到着は、保存済みの値を消さない**（Q20 / R45）。
    // 消すと以後、古い版が来ても止められない。
    //
    // **`external_id` と `external_ref` は触らない**（design D10）。どちらも来歴として
    // 凍結してあり、動かすと同じ本文が 2 行に増える（実測）。届いた値が保存済みと違えば
    // 門の手前の錠が落とす —— それが正しい（識別子が変わったなら別の記録）。
    //
    // **出来事の時刻と一緒に、その時刻を読むための欄も動かす**（R111）。
    // `event_time` だけを更新して `tz_offset_min` / `tz_id` を据え置いていたときは、
    // **更新で日をまたいだ記録の現地時刻が狂った**（出来事の時刻と地域がずれた組になる）。
    // `schema_version` / `unit_system` / `crs` も新しい内容を読むための欄なので同じ ——
    // 据え置くと、古い版の宣言で新しい `payload` を読むことになる。
    // **前の値は履歴に残る**ので、失われるものは無い。
    sqlx::query(
        "UPDATE core.event
            SET raw = $2, payload = $3, content_hash = $4, event_time = $5,
                source_updated_at = coalesce($6, source_updated_at),
                tz_offset_min = $7, tz_id = $8, schema_version = $9,
                unit_system = $10, crs = $11
          WHERE id = $1",
    )
    .bind(stored.id)
    .bind(&req.raw)
    .bind(payload)
    .bind(hash)
    .bind(req.event_time)
    .bind(req.source_updated_at)
    .bind(req.tz_offset_min)
    .bind(&req.tz_id)
    .bind(req.schema_version)
    .bind(req.unit_system_or_default())
    .bind(req.crs_or_default())
    .execute(&mut **tx)
    .await
    .map_err(|e| internal_at("ingest.event_update", e))?;
    Ok(Update::Applied)
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

    // **位置を受け入れた日の滞在を作り直す**（ST16 / Q6 / design D5）。まとめ送り 1 回につき、日ごと・利用者ごとに 1 回。
    // 位置の記録は 1 件 1 トランザクションで既に確定しているので、**ここが失敗しても応答は変えない**。
    rebuild_stays_after_ingest(&app, &items, &results).await;

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

/// 受け入れた記録のうち、利用者の基準の `sources` に入るものの日を作り直す（design D5）。
///
/// **失敗を応答に混ぜない。** ログに残すのは種別・日付・SQLSTATE・かかった時間だけで、
/// **エラーの本文は出さない** —— PostgreSQL の本文は入力値（緯度経度・時刻）を含むことがある（製造準備 A-2）。
async fn rebuild_stays_after_ingest(
    app: &App,
    items: &[serde_json::Value],
    results: &[IngestResult],
) {
    let mut sources_of: std::collections::HashMap<uuid::Uuid, Vec<String>> =
        std::collections::HashMap::new();
    let mut days = std::collections::BTreeSet::new();
    for (item, result) in items.iter().zip(results) {
        if !result.accepted {
            continue;
        }
        let Ok(req) = serde_json::from_value::<IngestRequest>(item.clone()) else {
            continue;
        };
        let sources = match sources_of.entry(req.user_id) {
            std::collections::hash_map::Entry::Occupied(o) => o.into_mut(),
            std::collections::hash_map::Entry::Vacant(v) => {
                match stay_store::criteria_or_default(&app.pool, req.user_id).await {
                    Ok(c) => v.insert(c.sources),
                    Err(e) => {
                        tracing::error!(
                            kind = "stay.rebuild",
                            op = "criteria_lookup",
                            user = %req.user_id,
                            sqlstate = %sqlstate_of(&e),
                            "滞在の基準を読めなかったので作り直さなかった（記録は受け入れ済み）"
                        );
                        v.insert(Vec::new())
                    }
                }
            }
        };
        if sources.contains(&req.logical_source) {
            days.insert((req.user_id, stay_store::jst_date(req.event_time)));
        }
    }
    for (user, day) in days {
        let started = std::time::Instant::now();
        let took_ms = || started.elapsed().as_millis() as u64;
        match (app.stays.0)(app.pool.clone(), user, day).await {
            Ok(()) if took_ms() >= SLOW_REBUILD_MS => tracing::warn!(
                kind = "stay.rebuild_slow",
                %user,
                %day,
                took_ms = took_ms(),
                "滞在の作り直しが遅い（取り込みの応答を待たせている。design D5 の反転条件）"
            ),
            Ok(()) => {
                tracing::info!(kind = "stay.rebuild", %day, took_ms = took_ms(), "滞在を作り直した")
            }
            Err(e) => {
                // **失敗した利用者と日と種別を残す**（R41）。失敗した日を覚えておく場所は無いので、
                // 運用者が `POST /stays/rebuild` を叩く手がかりはこの 1 行だけ。種別も利用者も値（座標・時刻）ではない
                tracing::error!(
                    kind = "stay.rebuild",
                    %user,
                    %day,
                    failure = %stay_store::failure_kind(&e),
                    took_ms = took_ms(),
                    "滞在の作り直しに失敗（位置の記録は受け入れ済み。その日の位置が次に届くか、手の作り直しで戻る）"
                );
            }
        }
    }
}

/// 取り込みの応答を待たせていると見なす作り直しの長さ（`collector-android` の読み取り上限 15 秒の 1/3）。
const SLOW_REBUILD_MS: u64 = 5_000;

/// SQLSTATE だけを取り出す（値を含まない）。
fn sqlstate_of(e: &sqlx::Error) -> String {
    e.as_database_error()
        .and_then(|d| d.code())
        .map(|c| c.into_owned())
        .unwrap_or_else(|| "unknown".into())
}

// ------------------------------------------------------------------ 滞在（ST16）

/// `POST /stays/rebuild` の本文。**すべて省ける**（省いた基準はいまの値のまま）。
#[derive(Debug, Default, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RebuildRequest {
    /// 利用者。省くと既定の利用者（FR-29。単一利用者でも列は day one から持つ）
    user_id: Option<uuid::Uuid>,
    /// 判定の半径（m）。1〜10,000
    radius_m: Option<i32>,
    /// 最短のとどまり（分）。1〜1,440
    min_minutes: Option<i32>,
    /// 記録が無いとみなす間隔（分）。1〜1,440
    gap_minutes: Option<i32>,
}

/// `POST /stays/rebuild` の応答。**値（座標・時刻）を含まない。**
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct RebuildResponse {
    /// 作り直しに使った基準の版
    criteria_id: i64,
    /// 作り直した日数
    days: i64,
    /// 作り直す前に読み出しに出ていた滞在の件数
    stays_before: i64,
    /// 作り直した後に読み出しに出ている滞在の件数
    stays_after: i64,
    took_ms: i64,
}

/// 範囲の外にある基準の欄の名前（spec「範囲外の基準は断られる」）。**欄の名前だけを返し、値は返さない。**
fn criteria_out_of_range(r: &RebuildRequest) -> Option<&'static str> {
    let within = |v: Option<i32>, hi: i32| v.is_none_or(|x| (1..=hi).contains(&x));
    [
        ("radius_m", within(r.radius_m, 10_000)),
        ("min_minutes", within(r.min_minutes, 1_440)),
        ("gap_minutes", within(r.gap_minutes, 1_440)),
    ]
    .into_iter()
    .find_map(|(name, ok)| (!ok).then_some(name))
}

/// 全期間の滞在を作り直す（FR-31 / Q6 / C10）。基準が添えられ、いまと違えば新しい版を足してから作り直す。
///
/// **範囲外の基準は 400 で、基準も滞在も変えない**（DB の `CHECK` に当てると 500 になる）。
#[utoipa::path(post, path = "/stays/rebuild", request_body = RebuildRequest,
    responses((status = 200, body = RebuildResponse), (status = 400), (status = 401)))]
pub async fn stays_rebuild(
    State(app): State<App>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Result<Json<RebuildResponse>, (StatusCode, String)> {
    authorize(&app, &headers)?;
    let req: RebuildRequest = if body.iter().all(u8::is_ascii_whitespace) {
        RebuildRequest::default()
    } else {
        serde_json::from_slice(&body)
            .map_err(|_| (StatusCode::BAD_REQUEST, "malformed".to_string()))?
    };
    if let Some(field) = criteria_out_of_range(&req) {
        tracing::warn!(
            kind = "stays.rebuild_rejected",
            field,
            "範囲外の基準を断った"
        );
        return Err((
            StatusCode::BAD_REQUEST,
            format!("criteria_out_of_range:{field}"),
        ));
    }
    let user = req.user_id.unwrap_or_default();
    let started = std::time::Instant::now();
    if req.radius_m.is_some() || req.min_minutes.is_some() || req.gap_minutes.is_some() {
        stay_store::set_criteria(
            &app.pool,
            user,
            req.radius_m,
            req.min_minutes,
            req.gap_minutes,
        )
        .await
        .map_err(|e| internal_at("stays.set_criteria", e))?;
    }
    let out = stay_store::rebuild_all(&app.pool, user)
        .await
        .map_err(|e| match e.downcast::<sqlx::Error>() {
            Ok(db) => internal_at("stays.rebuild_all", db),
            // 落ちた日と種別は `rebuild_all` がログに残している（R42）
            Err(e) => {
                tracing::error!(
                    kind = "stays.rebuild",
                    failure = %stay_store::failure_kind(&e),
                    "全期間の作り直しに失敗（基準を添えていれば、基準の版は既に足してある）"
                );
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "rebuild_incomplete".into(),
                )
            }
        })?;
    let took_ms = i64::try_from(started.elapsed().as_millis()).unwrap_or(i64::MAX);
    tracing::info!(
        kind = "stays.rebuild",
        days = out.days,
        stays_before = out.stays_before,
        stays_after = out.stays_after,
        took_ms,
        "全期間の滞在を作り直した"
    );
    Ok(Json(RebuildResponse {
        criteria_id: out.criteria_id,
        days: out.days,
        stays_before: out.stays_before,
        stays_after: out.stays_after,
        took_ms,
    }))
}

/// `GET /stays/criteria` の絞り込み。
#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct StaysCriteriaQuery {
    user_id: Option<uuid::Uuid>,
}

/// 利用者の判定の基準の版を古い順に返す（spec「判定の基準は利用者ごとに版として残る」）。
#[utoipa::path(get, path = "/stays/criteria", params(StaysCriteriaQuery),
    responses((status = 200, body = Vec<stay_store::CriteriaVersion>), (status = 401)))]
pub async fn stays_criteria_get(
    State(app): State<App>,
    headers: HeaderMap,
    Query(q): Query<StaysCriteriaQuery>,
) -> Result<Json<Vec<stay_store::CriteriaVersion>>, (StatusCode, String)> {
    authorize(&app, &headers)?;
    stay_store::criteria_versions(&app.pool, q.user_id.unwrap_or_default())
        .await
        .map(Json)
        .map_err(|e| internal_at("stays.criteria", e))
}

/// `GET /stays` の絞り込み。**日付は文字列で受けて自分で読む** —— 読めない日付を 400 で断る経路をここに持つ。
#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct StaysQuery {
    /// `YYYY-MM-DD`（Asia/Tokyo の日）
    date: String,
    user_id: Option<uuid::Uuid>,
}

/// 1 日の並び（滞在・移動・記録なし）を時刻順に返す（design D8）。
#[utoipa::path(get, path = "/stays", params(StaysQuery),
    responses((status = 200, body = stay_store::DayView), (status = 400), (status = 401)))]
pub async fn stays_get(
    State(app): State<App>,
    headers: HeaderMap,
    Query(q): Query<StaysQuery>,
) -> Result<Json<stay_store::DayView>, (StatusCode, String)> {
    authorize(&app, &headers)?;
    let date = chrono::NaiveDate::parse_from_str(&q.date, "%Y-%m-%d")
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid_date".to_string()))?;
    stay_store::day_view(
        &app.pool,
        q.user_id.unwrap_or_default(),
        date,
        chrono::Utc::now(),
    )
    .await
    .map(Json)
    .map_err(|e| internal_at("stays.day", e))
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
    // ただし**閾値が掛かるのは信号だけ**（第 9 回 Q32）—— 端末の時計がそのまま入ってくる側。
    coverage::touch_started_on(
        &mut *tx,
        &req.logical_source,
        req.emitted_at,
        coverage::Arrival::Heartbeat,
    )
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

/// ソース × 日 の 8 状態を返す（FR-54）。**状態は行に焼かず導出する**（design D6）。
#[utoipa::path(get, path = "/coverage", params(CoverageQuery),
    responses((status = 200, body = Vec<coverage::SourceCoverage>), (status = 401)))]
pub async fn coverage_get(
    State(app): State<App>,
    headers: HeaderMap,
    Query(q): Query<CoverageQuery>,
) -> Result<Json<Vec<coverage::SourceCoverage>>, (StatusCode, String)> {
    authorize(&app, &headers)?;
    // **NFR-13 の 5 ソースの順で返す**（登録簿の並び順ではない）。画面の縦の並びがこれになる。
    //
    // **`of_sources` が名前ごとに引き継ぎの鎖を解決する**（第 8 回 Q31 /
    // review/code-r2.md の R3）。ここで定数名のまま引いていたときは、達成の側だけが
    // 鎖の先端を数え、**格子と達成パネルが別の 5 本を見ていた** ——
    // 後継のソースの格子が画面のどこにも出ず、いま実際に収集しているソースの途絶が
    // 稼働状況の画面から消えていた。
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
        .route("/stays", get(stays_get))
        .route("/stays/rebuild", post(stays_rebuild))
        .route("/stays/criteria", get(stays_criteria_get))
        .with_state(App {
            pool,
            token,
            stays: StayRebuilder::real(),
        });

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
    paths(
        ingest,
        heartbeat_post,
        events,
        coverage_get,
        achievement_get,
        stays_get,
        stays_rebuild,
        stays_criteria_get
    ),
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
        RebuildRequest,
        RebuildResponse,
        stay_store::CriteriaVersion,
        stay_store::DayView,
        stay_store::DayEntry,
        stay_store::EntryKind,
        stay_store::CriteriaTag,
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
