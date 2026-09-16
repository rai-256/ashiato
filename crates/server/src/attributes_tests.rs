// SPDX-License-Identifier: AGPL-3.0-only
//! 個人属性の主張を、**本物の PostgreSQL に対して**確かめる（ST19）。
//!
//! **利用者で隔離する**（`testdb::user()`）。種類の台帳は追記のみで消せないので、
//! テストは毎回新しい利用者を作る。既定の利用者（`00000000-…`）には触らない。
//!
//! 章の分かれ方は `tasks.md` と同じ:
//!   `kinds_*` = 種類の口（2.3）/ `read_*` = 読み出し（2.4）/
//!   `ingest_*` = 取り込み口の分岐（3.1）/ `store_*` = 格納（3.2）/ `erasure_*` = 乱数（3.3）
#![allow(clippy::unwrap_used)]

use super::*;
use crate::attributes::{self, Precision};
use crate::testdb;

const TOKEN: &str = "test-token-0123456789abcdef";
/// 22 文字以上の base64url（design D4）。**主張ごとに違えるのが本来**だが、
/// 内容の鍵を分けるのは原文の `claim` と出来事の時刻なので、検査では固定でよい。
const NONCE: &str = "Zm9vYmFyYmF6cXV4MTIzNDU2";

async fn app() -> App {
    App::for_test(testdb::pool().await, TOKEN)
}

fn auth() -> HeaderMap {
    let mut h = HeaderMap::new();
    h.insert(
        "authorization",
        format!("Bearer {TOKEN}").parse().expect("ヘッダ"),
    );
    h
}

fn today() -> chrono::NaiveDate {
    chrono::NaiveDate::from_ymd_opt(2026, 9, 15).unwrap()
}

/// 主張の原文を組む（design D1 の形）。画面の `attributes.ts` と同じ形にする。
fn claim_raw(
    id: uuid::Uuid,
    kind: uuid::Uuid,
    value: Option<&str>,
    precision: &str,
    date: Option<&str>,
    supersedes: Option<uuid::Uuid>,
    note: Option<&str>,
    nonce: &str,
) -> String {
    serde_json::json!({
        "claim": id,
        "nonce": nonce,
        "kind": kind,
        "value": value,
        "valid_from": { "precision": precision, "date": date },
        "supersedes": supersedes,
        "note": note,
    })
    .to_string()
}

/// 取り込み口へ送る 1 件（主張）。**由来は「本人が書いた」・端末識別子なし**。
fn claim_item(
    id: uuid::Uuid,
    user: uuid::Uuid,
    asserted_at: &str,
    raw: &str,
) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "user_id": user,
        "logical_source": attributes::SOURCE,
        "external_id": null,
        "device_id": null,
        "origin": "authored",
        "event_time": asserted_at,
        "tz_offset_min": 540,
        "tz_id": "Asia/Tokyo",
        "schema_version": 1,
        "raw": raw,
        "payload": {},
    })
}

async fn post_ingest(app: &App, body: serde_json::Value) -> (StatusCode, Vec<IngestResult>) {
    let (code, Json(res)) = ingest(State(app.clone()), auth(), Json(body))
        .await
        .expect("取り込み口");
    (code, res)
}

/// 主張を 1 件、取り込み口越しに入れて結果を返す。
async fn send_claim(app: &App, item: serde_json::Value) -> IngestResult {
    let (_, mut res) = post_ingest(app, serde_json::json!([item])).await;
    res.pop().expect("1 件ぶんの結果")
}

/// **取り込み口を通さずに**主張の行を置く（2.3 / 2.4 の検査用。取り込みの分岐は 3 章で見る）。
async fn put_claim(
    pool: &sqlx::PgPool,
    user: uuid::Uuid,
    kind: uuid::Uuid,
    value: Option<&str>,
    precision: &str,
    date: Option<&str>,
    asserted_at: &str,
) -> uuid::Uuid {
    put_claim_superseding(pool, user, kind, value, precision, date, asserted_at, None).await
}

#[allow(clippy::too_many_arguments)]
async fn put_claim_superseding(
    pool: &sqlx::PgPool,
    user: uuid::Uuid,
    kind: uuid::Uuid,
    value: Option<&str>,
    precision: &str,
    date: Option<&str>,
    asserted_at: &str,
    supersedes: Option<uuid::Uuid>,
) -> uuid::Uuid {
    let id = uuid::Uuid::new_v4();
    let raw = claim_raw(id, kind, value, precision, date, supersedes, None, NONCE);
    let claim = attributes::parse_claim(&raw, id).expect("組んだ原文が形として通る");
    let at: chrono::DateTime<chrono::Utc> = asserted_at.parse().unwrap();
    sqlx::query(
        "INSERT INTO core.event
           (id, user_id, logical_source, origin, event_time, tz_offset_min, tz_id,
            schema_version, sensitivity, content_hash, raw, payload)
         VALUES ($1,$2,$3,'authored',$4,540,'Asia/Tokyo',1,2,$5,$6,$7)",
    )
    .bind(id)
    .bind(user)
    .bind(attributes::SOURCE)
    .bind(at)
    .bind(ingest::content_hash_of(attributes::SOURCE, at, &raw))
    .bind(&raw)
    .bind(&claim.payload)
    .execute(pool)
    .await
    .unwrap();
    id
}

/// その利用者の種類を、いまの名前つきで作った順に引く。
async fn read_view(app: &App, user: uuid::Uuid) -> attributes::AttributesView {
    attributes_store::attributes_view(&app.pool, user, today())
        .await
        .unwrap()
}

/// 名前でカードを引く。
fn kind_named<'a>(
    v: &'a attributes::AttributesView,
    name: &str,
) -> &'a attributes::KindView {
    v.kinds
        .iter()
        .find(|k| k.name == name)
        .unwrap_or_else(|| panic!("種類「{name}」が無い: {:?}", v.kinds.iter().map(|k| &k.name).collect::<Vec<_>>()))
}

// ================================================================ 1. 移行

/// 全版を**まっさらな DB に 2 回**当てて落ちない（tasks 1.1）。
///
/// 共有の開発 DB で当て直すと、並んで走る他のテストの挿入と `ALTER TABLE` の錠が
/// deadlock する（ST16 と同じ実測）。使い捨ての DB を作って当てる。
#[tokio::test]
async fn migration_applies_twice() {
    let (fresh, drop) = fresh_db().await;
    let applied = async {
        for round in 1..=2 {
            for (label, sql) in crate::MIGRATIONS {
                sqlx::raw_sql(sql)
                    .execute(&fresh)
                    .await
                    .map_err(|e| format!("{round} 回目の {label}: {e}"))?;
            }
        }
        let (n,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM core.source WHERE logical_source = 's01-attribute'",
        )
        .fetch_one(&fresh)
        .await
        .map_err(|e| e.to_string())?;
        Ok::<i64, String>(n)
    }
    .await;
    drop.await;

    assert_eq!(applied.unwrap(), 1, "2 回当てると s01-attribute が二重になる");
    assert!(
        crate::MIGRATIONS
            .last()
            .is_some_and(|(n, _)| n.ends_with("_personal_attributes")),
        "主張の移行が MIGRATIONS の末尾に無い"
    );
}

async fn fresh_db() -> (sqlx::PgPool, impl std::future::Future<Output = ()>) {
    let admin = testdb::pool().await;
    let name = format!("st19_tmp_{}", uuid::Uuid::new_v4().simple());
    sqlx::raw_sql(&format!("CREATE DATABASE {name}"))
        .execute(&admin)
        .await
        .unwrap();
    let base = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://ashiato:ashiato@127.0.0.1:55432/ashiato".into());
    let url = format!("{}/{name}", &base[..base.rfind('/').unwrap()]);
    let fresh = sqlx::PgPool::connect(&url).await.unwrap();
    let closer = fresh.clone();
    (fresh, async move {
        closer.close().await;
        sqlx::raw_sql(&format!("DROP DATABASE {name} WITH (FORCE)"))
            .execute(&admin)
            .await
            .unwrap();
    })
}

// ================================================================ 2.3 種類の口

#[tokio::test]
// Scenario: 種類を足せる
async fn kinds_can_be_added() {
    let app = app().await;
    let u = testdb::user();
    let created = attributes_store::add_kind(&app.pool, u, "副業")
        .await
        .unwrap()
        .expect("副業を足せる");
    let v = read_view(&app, u).await;
    let got = kind_named(&v, "副業");
    assert_eq!(got.id, created.id, "足した識別子で読み出せない");
}

#[tokio::test]
// Scenario: 名前を変えても識別子と主張が変わらない
// Scenario: 名前を変えても前の名前が台帳に残る
/// **名前で種類を指すと、名前を直した日に過去の主張が別の種類に割れる**（扉 #17 と同じ型）。
async fn kinds_rename_keeps_id_and_claims() {
    let app = app().await;
    let u = testdb::user();
    let before = read_view(&app, u).await;
    let job = kind_named(&before, "職業").id;
    // 主張は DB に直接入れる（取り込みの分岐は 3 章）
    put_claim(&app.pool, u, job, Some("会社員"), "year", Some("2020"), "2026-09-01T01:00:00Z").await;
    put_claim(&app.pool, u, job, Some("自営業"), "year", Some("2024"), "2026-09-02T01:00:00Z").await;

    attributes_store::rename_kind(&app.pool, u, job, "仕事")
        .await
        .unwrap()
        .expect("名前を変えられる");

    let after = read_view(&app, u).await;
    let renamed = kind_named(&after, "仕事");
    assert_eq!(renamed.id, job, "名前を変えたら識別子が変わった");
    assert_eq!(renamed.claims.len(), 2, "名前を変えたら主張が失われた");
    assert_eq!(renamed.current.as_ref().unwrap().value.as_deref(), Some("自営業"));
    assert!(
        after.kinds.iter().all(|k| k.name != "職業"),
        "前の名前のカードが残っている"
    );

    // **前の名前は台帳に残る**（追記のみ）
    let names: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM core.attribute_kind_name WHERE kind_id = $1 ORDER BY id",
    )
    .bind(job)
    .fetch_all(&app.pool)
    .await
    .unwrap();
    assert_eq!(names, ["職業", "仕事"], "前の名前が台帳に残っていない");
}

#[tokio::test]
// Scenario: 初めて種類を足す前に住所と職業が置かれる
async fn kinds_initial_are_placed_before_the_first_add() {
    let app = app().await;
    let u = testdb::user();
    // **読み出しより先に足す**（初期化が読み出しの側にしか無いと、ここで住所と職業が消える）
    attributes_store::add_kind(&app.pool, u, "副業")
        .await
        .unwrap()
        .expect("副業を足せる");
    let v = read_view(&app, u).await;
    let names: Vec<&str> = v.kinds.iter().map(|k| k.name.as_str()).collect();
    assert_eq!(names, ["住所", "職業", "副業"], "住所と職業が先に置かれていない");
}

#[tokio::test]
// Scenario: 空の名前の種類は足せない
async fn kinds_reject_empty_name() {
    let app = app().await;
    let u = testdb::user();
    for blank in ["", "   ", "\u{3000}"] {
        assert_eq!(
            attributes_store::add_kind(&app.pool, u, blank).await.unwrap(),
            Err(attributes_store::KindError::EmptyName),
            "空白だけの名前 {blank:?} が通っている"
        );
    }
    let v = read_view(&app, u).await;
    assert_eq!(v.kinds.len(), 2, "断ったのに種類が増えている");
}

#[tokio::test]
// Scenario: いまある名前と同じ種類は足せない
/// **NFC にしてから比べる**（FR-27）—— NFD の「ジ」で同じ名前の種類を 2 つ作れると、
/// 画面に同じ名前のカードが 2 枚並ぶ。
async fn kinds_reject_duplicate_name() {
    let app = app().await;
    let u = testdb::user();
    assert_eq!(
        attributes_store::add_kind(&app.pool, u, "住所").await.unwrap(),
        Err(attributes_store::KindError::DuplicateName),
    );
    // 前後の空白を除いてから比べる
    assert_eq!(
        attributes_store::add_kind(&app.pool, u, " 住所 ").await.unwrap(),
        Err(attributes_store::KindError::DuplicateName),
    );
    // NFD（「シ」+ 濁点）で書いた「住所」…ではなく、濁点を持つ名前で確かめる
    attributes_store::add_kind(&app.pool, u, "\u{304C}\u{3093}\u{3053}\u{3046}")
        .await
        .unwrap()
        .expect("「がんこう」を足せる");
    assert_eq!(
        attributes_store::add_kind(&app.pool, u, "\u{304B}\u{3099}\u{3093}\u{3053}\u{3046}")
            .await
            .unwrap(),
        Err(attributes_store::KindError::DuplicateName),
        "NFD で書いた同じ名前が通っている"
    );
    let v = read_view(&app, u).await;
    assert_eq!(v.kinds.len(), 3, "断ったのに種類が増えている");
}

#[tokio::test]
// Scenario: いまある名前へは変えられない
async fn kinds_rename_rejects_taken_name() {
    let app = app().await;
    let u = testdb::user();
    let side = attributes_store::add_kind(&app.pool, u, "副業")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        attributes_store::rename_kind(&app.pool, u, side.id, "住所")
            .await
            .unwrap(),
        Err(attributes_store::KindError::DuplicateName),
    );
    let v = read_view(&app, u).await;
    assert_eq!(kind_named(&v, "副業").id, side.id, "断ったのに名前が変わっている");

    // **自分のいまの名前へは変えられる**（重なりでない。断ると名前を戻せなくなる）
    assert!(
        attributes_store::rename_kind(&app.pool, u, side.id, "副業")
            .await
            .unwrap()
            .is_ok(),
        "自分のいまの名前を重なりとして断っている"
    );
}

#[tokio::test]
// Scenario: 別の利用者の種類の名前は変えられない
/// **無い種類と同じ 400 にする**（design D7）—— 別の利用者の種類を名指しできたことが
/// 応答から分かると、それ自体が漏れになる。
async fn kinds_rename_rejects_another_users_kind() {
    let app = app().await;
    let a = testdb::user();
    let b = testdb::user();
    let bs = read_view(&app, b).await;
    let bs_address = kind_named(&bs, "住所").id;

    assert_eq!(
        attributes_store::rename_kind(&app.pool, a, bs_address, "自宅")
            .await
            .unwrap(),
        Err(attributes_store::KindError::UnknownKind),
    );
    let after = read_view(&app, b).await;
    assert_eq!(
        kind_named(&after, "住所").id,
        bs_address,
        "別の利用者から名前を変えられた"
    );
    // 無い種類も同じ理由で断る（区別が応答から見えない）
    assert_eq!(
        attributes_store::rename_kind(&app.pool, a, uuid::Uuid::new_v4(), "自宅")
            .await
            .unwrap(),
        Err(attributes_store::KindError::UnknownKind),
    );
}

// ================================================================ 2.4 読み出し

#[tokio::test]
// Scenario: 最初に住所と職業がある
async fn read_starts_with_address_and_job() {
    let app = app().await;
    let u = testdb::user();
    let v = read_view(&app, u).await;
    let names: Vec<&str> = v.kinds.iter().map(|k| k.name.as_str()).collect();
    assert_eq!(names, ["住所", "職業"]);
    // **まだ書いていない**（画面がそう出す。いまの値が無いことと読み出しの失敗は別）
    assert!(v.kinds.iter().all(|k| k.current.is_none() && k.claims.is_empty()));
}

#[tokio::test]
// Scenario: 同時に初めて読み出しても住所と職業は 1 つずつ
/// **名前の行は種類の `INSERT … RETURNING` が返したときだけ書く**（spec-review R15）——
/// 種類の主キーだけで衝突を止めると、同時に 0 件を見た 2 本が名前の行を 2 本ずつ積み、
/// **追記のみの台帳からは消せない。**
async fn read_concurrent_first_read_places_one_each() {
    let app = app().await;
    let u = testdb::user();
    let (a, b) = tokio::join!(
        attributes_store::attributes_view(&app.pool, u, today()),
        attributes_store::attributes_view(&app.pool, u, today()),
    );
    a.unwrap();
    b.unwrap();
    // 続けてもう 1 回（初期化が毎回走って増えないこと）
    read_view(&app, u).await;

    let (kinds,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM core.attribute_kind WHERE user_id = $1")
            .bind(u)
            .fetch_one(&app.pool)
            .await
            .unwrap();
    let (names,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM core.attribute_kind_name WHERE user_id = $1")
            .bind(u)
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert_eq!(kinds, 2, "同時の初期化で種類が {kinds} 行になった");
    assert_eq!(names, 2, "同時の初期化で名前の台帳が {names} 行になった（消せない）");
}

#[tokio::test]
// Scenario: 今日は Asia/Tokyo の日付で決まる
/// UTC の 2026-09-30T16:00 は `Asia/Tokyo` では 2026-10-01。
/// **UTC で日を切っていると、9 時間だけ「予定」のままになる**（ST02 の R10 と同じ型）。
async fn read_today_is_asia_tokyo() {
    let app = app().await;
    let u = testdb::user();
    let v0 = read_view(&app, u).await;
    let address = kind_named(&v0, "住所").id;
    put_claim(&app.pool, u, address, Some("新居"), "day", Some("2026-10-01"), "2026-09-20T01:00:00Z").await;

    let at: chrono::DateTime<chrono::Utc> = "2026-09-30T16:00:00Z".parse().unwrap();
    let today = stay_store::jst_date(at);
    assert_eq!(today, chrono::NaiveDate::from_ymd_opt(2026, 10, 1).unwrap());

    let v = attributes_store::attributes_view(&app.pool, u, today).await.unwrap();
    let got = kind_named(&v, "住所");
    assert_eq!(
        got.current.as_ref().map(|c| c.value.as_deref()),
        Some(Some("新居")),
        "JST の 10/1 に 10/1 からの主張がまだ予定のまま"
    );
    assert!(got.upcoming.is_empty());
}

#[tokio::test]
// Scenario: 消したことにした主張は出ない
// Scenario: 消したことにした主張の次がいまの値になる
async fn read_soft_deleted_claims_are_hidden() {
    let app = app().await;
    let u = testdb::user();
    let v0 = read_view(&app, u).await;
    let address = kind_named(&v0, "住所").id;
    put_claim(&app.pool, u, address, Some("旧居"), "month", Some("2019-10"), "2026-09-01T01:00:00Z").await;
    let newest = put_claim(&app.pool, u, address, Some("新居"), "month", Some("2023-03"), "2026-09-02T01:00:00Z").await;

    sqlx::query("UPDATE core.event SET deleted_at = now(), deleted_by = 'test' WHERE id = $1")
        .bind(newest)
        .execute(&app.pool)
        .await
        .unwrap();

    let v = read_view(&app, u).await;
    let got = kind_named(&v, "住所");
    let seen: Vec<uuid::Uuid> = got
        .claims
        .iter()
        .chain(&got.superseded)
        .chain(&got.upcoming)
        .map(|c| c.id)
        .collect();
    assert!(!seen.contains(&newest), "消したことにした主張が出ている");
    assert_eq!(
        got.current.as_ref().unwrap().value.as_deref(),
        Some("旧居"),
        "消したことにした主張の次がいまの値になっていない"
    );
}

#[tokio::test]
// Scenario: 消した主張が取り消していた主張は戻る
async fn read_deleted_supersession_restores_the_target() {
    let app = app().await;
    let u = testdb::user();
    let v0 = read_view(&app, u).await;
    let address = kind_named(&v0, "住所").id;
    let a = put_claim(&app.pool, u, address, Some("A"), "month", Some("2019-04"), "2026-09-01T01:00:00Z").await;
    let b = put_claim_superseding(
        &app.pool, u, address, Some("A"), "month", Some("2019-10"), "2026-09-02T01:00:00Z", Some(a),
    )
    .await;
    // B が A を取り消している状態を確かめてから、B を消す
    let before = read_view(&app, u).await;
    assert_eq!(kind_named(&before, "住所").superseded.len(), 1);

    sqlx::query("UPDATE core.event SET deleted_at = now(), deleted_by = 'test' WHERE id = $1")
        .bind(b)
        .execute(&app.pool)
        .await
        .unwrap();

    let after = read_view(&app, u).await;
    let got = kind_named(&after, "住所");
    assert_eq!(got.claims.iter().map(|c| c.id).collect::<Vec<_>>(), vec![a], "A が積んだ主張に戻っていない");
    assert!(got.superseded.is_empty(), "消した主張の取り消しがまだ効いている");
}

#[tokio::test]
// Scenario: 本文を消去した主張は出ず、その取り消しも効かない
/// **消去した主張は種類も値も読めない**ので、並べる置き場を持たない（spec-review R18）。
async fn read_erased_claims_vanish_and_their_supersession_lifts() {
    let app = app().await;
    let u = testdb::user();
    let v0 = read_view(&app, u).await;
    let address = kind_named(&v0, "住所").id;
    let a = put_claim(&app.pool, u, address, Some("A"), "month", Some("2019-04"), "2026-09-01T01:00:00Z").await;
    let b = put_claim_superseding(
        &app.pool, u, address, Some("A"), "month", Some("2019-10"), "2026-09-02T01:00:00Z", Some(a),
    )
    .await;

    // **台帳つきで消去する**（門が通す唯一の形。同じまとまりでその主張の行を書く）
    let mut tx = app.pool.begin().await.unwrap();
    sqlx::query(
        "INSERT INTO core.erasure_ledger (event_id, user_id, logical_source, scope, erased_by)
         VALUES ($1, $2, $3, 'event', 'test')",
    )
    .bind(b)
    .bind(u)
    .bind(attributes::SOURCE)
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::query("UPDATE core.event SET raw = '', payload = '{}'::jsonb WHERE id = $1")
        .bind(b)
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.expect("台帳つきの消去は通る");

    let v = read_view(&app, u).await;
    let got = kind_named(&v, "住所");
    let seen: Vec<uuid::Uuid> = got.claims.iter().chain(&got.superseded).map(|c| c.id).collect();
    assert!(!seen.contains(&b), "消去した主張が出ている");
    assert_eq!(seen, vec![a], "消去した主張の取り消しがまだ効いている");
}

#[tokio::test]
// Scenario: 読み出しは 2 つの時刻を別々に返す
/// **FR-45 の 2 軸は保存も読み出しも別々に持つ** —— 画面が隠すのは表示だけ（Q2）。
async fn read_returns_both_times_separately() {
    let app = app().await;
    let u = testdb::user();
    let v0 = read_view(&app, u).await;
    let address = kind_named(&v0, "住所").id;
    put_claim(&app.pool, u, address, Some("A"), "month", Some("2019-10"), "2026-09-15T02:30:00Z").await;

    let v = read_view(&app, u).await;
    let c = kind_named(&v, "住所").claims.first().unwrap();
    assert!(c.asserted_at.starts_with("2026-09-15"), "主張した日時が入っていない: {}", c.asserted_at);
    assert!(
        c.asserted_at.ends_with("+09:00"),
        "主張した日時に地域のずれが無い: {}",
        c.asserted_at
    );
    assert!(!c.ingested_at.is_empty(), "D-01 に入った時刻が無い");
    assert_ne!(c.asserted_at, c.ingested_at, "2 つの時刻が同じ欄から来ている");
    assert_eq!(c.valid_from.precision, Precision::Month);
    assert_eq!(c.valid_from.date.as_deref(), Some("2019-10"));
}

#[tokio::test]
// Scenario: 種類は作った順に返る
async fn read_kinds_come_in_creation_order() {
    let app = app().await;
    let u = testdb::user();
    for name in ["副業", "同居"] {
        attributes_store::add_kind(&app.pool, u, name)
            .await
            .unwrap()
            .unwrap();
    }
    let v = read_view(&app, u).await;
    let names: Vec<&str> = v.kinds.iter().map(|k| k.name.as_str()).collect();
    assert_eq!(names, ["住所", "職業", "副業", "同居"]);
}

#[tokio::test]
// Scenario: 感度で主張を絞らない
/// **感度で絞るのは AI への経路（ST27）で、本人の画面ではない。**
async fn read_does_not_filter_by_sensitivity() {
    let app = app().await;
    let u = testdb::user();
    let v0 = read_view(&app, u).await;
    let address = kind_named(&v0, "住所").id;
    let id = put_claim(&app.pool, u, address, Some("秘密の住所"), "year", Some("2019"), "2026-09-01T01:00:00Z").await;
    // 「AI に出さない」（PERM-2 の 4 段階のいちばん厳しい側）へ締める
    sqlx::query("UPDATE core.event SET sensitivity = 3 WHERE id = $1")
        .bind(id)
        .execute(&app.pool)
        .await
        .unwrap();

    let v = read_view(&app, u).await;
    let got = kind_named(&v, "住所");
    assert_eq!(got.claims.iter().map(|c| c.id).collect::<Vec<_>>(), vec![id]);
    assert_eq!(got.current.as_ref().unwrap().value.as_deref(), Some("秘密の住所"));
}

#[tokio::test]
// Scenario: 別の利用者の主張は読み出せない
async fn read_does_not_cross_users() {
    let app = app().await;
    let a = testdb::user();
    let b = testdb::user();
    let va = read_view(&app, a).await;
    let vb = read_view(&app, b).await;
    let a_address = kind_named(&va, "住所").id;
    let b_address = kind_named(&vb, "住所").id;
    let a_claim = put_claim(&app.pool, a, a_address, Some("A の住所"), "year", Some("2019"), "2026-09-01T01:00:00Z").await;
    let b_claim = put_claim(&app.pool, b, b_address, Some("B の住所"), "year", Some("2019"), "2026-09-01T01:00:00Z").await;

    let v = read_view(&app, a).await;
    let ids: Vec<uuid::Uuid> = v.kinds.iter().flat_map(|k| k.claims.iter().map(|c| c.id)).collect();
    assert!(ids.contains(&a_claim), "自分の主張が読めない");
    assert!(!ids.contains(&b_claim), "別の利用者の主張が読めた");
    let kind_ids: Vec<uuid::Uuid> = v.kinds.iter().map(|k| k.id).collect();
    assert!(!kind_ids.contains(&b_address), "別の利用者の種類が読めた");
}

#[tokio::test]
/// 読み出しの口が合言葉を要る（ほかの読み出しと同じ）。
async fn read_requires_the_token() {
    let app = app().await;
    let res = attributes_get(
        State(app),
        HeaderMap::new(),
        Query(AttributesQuery { user_id: None }),
    )
    .await;
    assert_eq!(res.unwrap_err().0, StatusCode::UNAUTHORIZED);
}
