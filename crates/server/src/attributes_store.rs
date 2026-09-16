// SPDX-License-Identifier: AGPL-3.0-only
//! 属性の種類の台帳と、個人属性の読み出し（ST19 / FR-44 / FR-29。design D7（仮）/ D8）。
//!
//! 解釈と導き方そのものは `attributes`（DB に触らない）。ここが持つのは、
//! **何を読んで `view` に渡し、種類をどう足すか** —— 錠の取り方（D7）と、
//! 削除の印・消去した本文の除き方（D6 の手順 1）。
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, Transaction};
use unicode_normalization::UnicodeNormalization as _;

use crate::attributes::{self, AttributesView, Claim, Kind, Precision, StoredClaim, ValidFrom};

/// 種類の錠の名前空間（`pg_advisory_xact_lock(key, hashtext(user))` の 1 つ目）。
/// 滞在の作り直し（4_816_016）とマイグレーション（4_820_251）とは別の空間。
pub(crate) const LOCK_KEY: i32 = 4_819_019;

/// 最初に置く種類（深掘り C7）。**識別子は利用者の UUID を名前空間にした v5**（design D7）——
/// 同じ利用者なら何度導いても同じ識別子になるので、初期化が競合しても増えない。
pub const INITIAL_KINDS: [(&str, &str); 2] = [("address", "住所"), ("job", "職業")];

/// 種類の操作を断る理由（design D7）。どれも 400。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum KindError {
    /// 名前が、前後の空白を除いて空
    EmptyName,
    /// その利用者のいまの名前と重なる
    DuplicateName,
    /// その利用者の種類ではない（無い種類と同じに扱う。design D7）
    UnknownKind,
}

/// 種類を足す・名前を変える要求の本文。
#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct KindRequest {
    /// 省けば nil UUID（ほかの読み出しと同じ。ST29 まで利用者は名乗り）
    #[serde(default)]
    pub user_id: Option<uuid::Uuid>,
    pub name: String,
}

/// 種類を足した結果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct KindCreated {
    pub id: uuid::Uuid,
}

/// その利用者の名前空間で、初期の種類の識別子を導く（design D7）。
///
/// **v5（名前から決まる）**なので、同時に走った 2 本が同じ識別子に落ちる ——
/// だから主キーの `ON CONFLICT DO NOTHING` で 1 つに畳める。
fn initial_kind_id(user_id: uuid::Uuid, slug: &str) -> uuid::Uuid {
    uuid::Uuid::new_v5(&user_id, slug.as_bytes())
}

/// その利用者が種類を 1 つも持たなければ「住所」「職業」を置く（design D7 / 深掘り C7）。
///
/// **`GET /attributes` と `POST /attributes/kinds` の両方の先頭で呼ぶ** ——
/// どちらが先に来ても、本人が最初に見る画面には住所と職業がある。
///
/// **名前の行は種類の `INSERT … RETURNING` が行を返したときだけ書く**（spec-review R15）。
/// 種類の主キーだけで衝突を止めると、同時に 0 件を見た 2 本が名前の行を 2 本ずつ積み、
/// **追記のみの台帳からは消せない。**
pub async fn ensure_initial_kinds(
    tx: &mut Transaction<'_, Postgres>,
    user_id: uuid::Uuid,
) -> Result<(), sqlx::Error> {
    // 利用者ごとの錠。**初期化と「住所」を足す要求が重なっても、いまの名前が「住所」の種類は 1 つ**
    lock_user(tx, user_id).await?;

    let existing: (i64,) =
        sqlx::query_as("SELECT count(*) FROM core.attribute_kind WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&mut **tx)
            .await?;
    if existing.0 > 0 {
        return Ok(());
    }

    for (slug, name) in INITIAL_KINDS {
        let id = initial_kind_id(user_id, slug);
        let inserted: Option<(uuid::Uuid,)> = sqlx::query_as(
            "INSERT INTO core.attribute_kind (id, user_id) VALUES ($1, $2)
             ON CONFLICT (id) DO NOTHING
             RETURNING id",
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(&mut **tx)
        .await?;
        // **行が返ったときだけ名前を書く。** 返らなかった＝別のまとまりが先に置いたので、
        // 名前の行もその側が書いている
        if inserted.is_some() {
            sqlx::query(
                "INSERT INTO core.attribute_kind_name (kind_id, user_id, name) VALUES ($1, $2, $3)",
            )
            .bind(id)
            .bind(user_id)
            .bind(name)
            .execute(&mut **tx)
            .await?;
        }
    }
    Ok(())
}

/// 利用者ごとの助言ロック。**トランザクションの終わりまで**（`_xact_`）。
async fn lock_user(
    tx: &mut Transaction<'_, Postgres>,
    user_id: uuid::Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT pg_advisory_xact_lock($1, hashtext($2::text))")
        .bind(LOCK_KEY)
        .bind(user_id)
        .execute(&mut **tx)
        .await
        .map(|_| ())
}

/// いまの名前が `name` と重なる種類があるか。**`except` の種類は除く**（自分の名前は重なりでない）。
///
/// **一意索引では表せない** —— 「いまの名前」は台帳の `id` が最大の行なので、
/// 行そのものには一意性が無い。錠の中で見る。
async fn name_is_taken(
    tx: &mut Transaction<'_, Postgres>,
    user_id: uuid::Uuid,
    name: &str,
    except: Option<uuid::Uuid>,
) -> Result<bool, sqlx::Error> {
    let taken: (bool,) = sqlx::query_as(
        "SELECT EXISTS (
           SELECT 1 FROM (
             SELECT DISTINCT ON (kind_id) kind_id, name
               FROM core.attribute_kind_name
              WHERE user_id = $1
              ORDER BY kind_id, id DESC
           ) cur
            WHERE cur.name = $2 AND ($3::uuid IS NULL OR cur.kind_id <> $3)
         )",
    )
    .bind(user_id)
    .bind(name)
    .bind(except)
    .fetch_one(&mut **tx)
    .await?;
    Ok(taken.0)
}

/// 種類を足す（design D7）。**名前は NFC**（FR-27）。
pub async fn add_kind(
    pool: &PgPool,
    user_id: uuid::Uuid,
    name: &str,
) -> Result<Result<KindCreated, KindError>, sqlx::Error> {
    let name: String = name.trim().nfc().collect();
    if name.is_empty() {
        return Ok(Err(KindError::EmptyName));
    }
    let mut tx = pool.begin().await?;
    // **足す前に住所と職業を置く**（spec「初めて種類を足す前に住所と職業が置かれる」）。
    // 錠はここで取られ、この下の重なりの確認まで同じまとまりで持ち続ける
    ensure_initial_kinds(&mut tx, user_id).await?;

    if name_is_taken(&mut tx, user_id, &name, None).await? {
        return Ok(Err(KindError::DuplicateName));
    }
    let id = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO core.attribute_kind (id, user_id) VALUES ($1, $2)")
        .bind(id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO core.attribute_kind_name (kind_id, user_id, name) VALUES ($1, $2, $3)",
    )
    .bind(id)
    .bind(user_id)
    .bind(&name)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Ok(KindCreated { id }))
}

/// 種類の名前を変える（design D7）。**前の名前を残したまま追記する**（台帳は追記のみ）。
///
/// **その利用者の種類でなければ、無い種類と同じ 400**（design D7）——
/// 別の利用者の種類を名指しできたことが応答から分かると、それ自体が漏れになる。
pub async fn rename_kind(
    pool: &PgPool,
    user_id: uuid::Uuid,
    kind_id: uuid::Uuid,
    name: &str,
) -> Result<Result<(), KindError>, sqlx::Error> {
    let name: String = name.trim().nfc().collect();
    if name.is_empty() {
        return Ok(Err(KindError::EmptyName));
    }
    let mut tx = pool.begin().await?;
    lock_user(&mut tx, user_id).await?;

    let mine: (bool,) = sqlx::query_as(
        "SELECT EXISTS (SELECT 1 FROM core.attribute_kind WHERE id = $1 AND user_id = $2)",
    )
    .bind(kind_id)
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await?;
    if !mine.0 {
        return Ok(Err(KindError::UnknownKind));
    }
    if name_is_taken(&mut tx, user_id, &name, Some(kind_id)).await? {
        return Ok(Err(KindError::DuplicateName));
    }
    sqlx::query(
        "INSERT INTO core.attribute_kind_name (kind_id, user_id, name) VALUES ($1, $2, $3)",
    )
    .bind(kind_id)
    .bind(user_id)
    .bind(&name)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Ok(()))
}

/// その利用者の種類を、**作った順**に、いまの名前とともに返す（design D8 / D14）。
///
/// **並びは `seq`（単調増加）**。`created_at` はトランザクションの開始時刻なので、
/// 同じまとまりで置いた住所と職業が同時刻になり、並びが UUID 任せになる（実測で落ちた）。
async fn kinds_of(
    tx: &mut Transaction<'_, Postgres>,
    user_id: uuid::Uuid,
) -> Result<Vec<Kind>, sqlx::Error> {
    let rows: Vec<(uuid::Uuid, String)> = sqlx::query_as(
        "SELECT k.id, n.name
           FROM core.attribute_kind k
           JOIN LATERAL (
             SELECT name FROM core.attribute_kind_name
              WHERE kind_id = k.id ORDER BY id DESC LIMIT 1
           ) n ON true
          WHERE k.user_id = $1
          ORDER BY k.seq",
    )
    .bind(user_id)
    .fetch_all(&mut **tx)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(id, name)| Kind { id, name })
        .collect())
}

/// DB から読んだ主張の 1 行。
#[derive(sqlx::FromRow)]
struct ClaimRow {
    id: uuid::Uuid,
    event_time: DateTime<Utc>,
    tz_offset_min: i32,
    ingest_time: DateTime<Utc>,
    raw: String,
    payload: serde_json::Value,
}

/// 格納されている主張を読む。**`core.event_live` から引く**（削除の印を全クエリに効かせる。製造準備 A-3）。
///
/// **本文を消去した行（`raw = ''`）も読んで `erased` を立てる** —— 落とすのは `view` の側に寄せ、
/// 「どこにも出さない」を 1 か所で決める（design D6 の手順 1 / spec-review R18）。
async fn claims_of(
    tx: &mut Transaction<'_, Postgres>,
    user_id: uuid::Uuid,
) -> Result<Vec<StoredClaim>, sqlx::Error> {
    let rows: Vec<ClaimRow> = sqlx::query_as(
        "SELECT id, event_time, tz_offset_min, ingest_time, raw, payload
           FROM core.event_live
          WHERE user_id = $1 AND logical_source = $2",
    )
    .bind(user_id)
    .bind(attributes::SOURCE)
    .fetch_all(&mut **tx)
    .await?;

    Ok(rows.into_iter().filter_map(stored_claim_of).collect())
}

/// 1 行を主張にする。**読めない行は落とす**（消去した行はここで `erased` になる）。
fn stored_claim_of(row: ClaimRow) -> Option<StoredClaim> {
    let offset = chrono::FixedOffset::east_opt(row.tz_offset_min * 60)?;
    let asserted_at = row.event_time.with_timezone(&offset);
    if row.raw.is_empty() {
        // **消去された主張**。種類も値も読めないので、器だけ作って `view` に落とさせる
        return Some(StoredClaim {
            claim: Claim {
                id: row.id,
                kind: uuid::Uuid::nil(),
                value: None,
                valid_from: ValidFrom {
                    precision: Precision::Unknown,
                    date: None,
                },
                supersedes: None,
                note: None,
                payload: serde_json::json!({}),
            },
            asserted_at,
            ingested_at: row.ingest_time,
            erased: true,
        });
    }
    // **解析済みから読む**（原文ではなく）—— 格納のときに原文から組み直してあり、
    // 原文をもう一度解釈すると解釈の規則が 2 か所に割れる。
    let claim = claim_from_payload(&row.payload, row.id)?;
    Some(StoredClaim {
        claim,
        asserted_at,
        ingested_at: row.ingest_time,
        erased: false,
    })
}

/// 解析済み（`payload`）から主張を組む。`nonce` は入っていない（design D4）。
fn claim_from_payload(payload: &serde_json::Value, id: uuid::Uuid) -> Option<Claim> {
    let kind = payload
        .get("kind")
        .and_then(|v| v.as_str())
        .and_then(|s| uuid::Uuid::parse_str(s).ok())?;
    let vf = payload.get("valid_from")?;
    let precision = match vf.get("precision").and_then(|v| v.as_str())? {
        "year" => Precision::Year,
        "month" => Precision::Month,
        "day" => Precision::Day,
        "unknown" => Precision::Unknown,
        _ => return None,
    };
    Some(Claim {
        id,
        kind,
        value: payload
            .get("value")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        valid_from: ValidFrom {
            precision,
            date: vf.get("date").and_then(|v| v.as_str()).map(str::to_string),
        },
        supersedes: payload
            .get("supersedes")
            .and_then(|v| v.as_str())
            .and_then(|s| uuid::Uuid::parse_str(s).ok()),
        note: payload
            .get("note")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        payload: payload.clone(),
    })
}

/// `GET /attributes` の中身（design D8）。
///
/// **先頭で住所と職業を置く**（design D7 の反転条件つき —— 読み出しに書き込みがあることが
/// 問題になったら明示の初期化の口へ移す。識別子は v5 なので変わらない）。
///
/// `today` は呼び出し側が `Asia/Tokyo` で決めて渡す（**試験が時刻を差し込めるように**）。
pub async fn attributes_view(
    pool: &PgPool,
    user_id: uuid::Uuid,
    today: NaiveDate,
) -> Result<AttributesView, sqlx::Error> {
    let mut tx = pool.begin().await?;
    ensure_initial_kinds(&mut tx, user_id).await?;
    let kinds = kinds_of(&mut tx, user_id).await?;
    let claims = claims_of(&mut tx, user_id).await?;
    tx.commit().await?;
    Ok(attributes::view(&kinds, &claims, today))
}

/// その種類がその利用者のものか（取り込み口の検査。design D5）。
pub async fn kind_exists(
    tx: &mut Transaction<'_, Postgres>,
    user_id: uuid::Uuid,
    kind_id: uuid::Uuid,
) -> Result<bool, sqlx::Error> {
    let found: (bool,) = sqlx::query_as(
        "SELECT EXISTS (SELECT 1 FROM core.attribute_kind WHERE id = $1 AND user_id = $2)",
    )
    .bind(kind_id)
    .bind(user_id)
    .fetch_one(&mut **tx)
    .await?;
    Ok(found.0)
}

/// 取り消し先として使える主張か（design D5 / spec「形の合わない主張は受け付けない」）。
///
/// **同じ利用者・同じ種類の、主張のソースの記録**でなければならない。
/// **削除の印や消去は見ない** —— 消した主張を取り消し先に指せることは spec が認めている
/// （読み出しがその取り消しを効かせる相手を持たないだけで害が無い）。
///
/// **行錠を取らずに読む**（spec-review R17）。読んだ直後に別のまとまりが取り消し先を
/// 消しても、読み出しは消した主張を出さないので、その取り消しは効く相手がいないだけ。
pub async fn supersedes_is_valid(
    tx: &mut Transaction<'_, Postgres>,
    user_id: uuid::Uuid,
    kind_id: uuid::Uuid,
    target: uuid::Uuid,
) -> Result<bool, sqlx::Error> {
    let row: Option<(serde_json::Value,)> = sqlx::query_as(
        "SELECT payload FROM core.event
          WHERE id = $1 AND user_id = $2 AND logical_source = $3",
    )
    .bind(target)
    .bind(user_id)
    .bind(attributes::SOURCE)
    .fetch_optional(&mut **tx)
    .await?;
    // **種類まで一致することを見る**（別の種類の主張を取り消すと「いまの値」が黙って変わる）
    Ok(row.is_some_and(|(payload,)| {
        payload.get("kind").and_then(|v| v.as_str()) == Some(kind_id.to_string().as_str())
    }))
}
