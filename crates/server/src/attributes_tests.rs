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
#[allow(clippy::too_many_arguments)]
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
fn claim_item(id: uuid::Uuid, user: uuid::Uuid, asserted_at: &str, raw: &str) -> serde_json::Value {
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
fn kind_named<'a>(v: &'a attributes::AttributesView, name: &str) -> &'a attributes::KindView {
    v.kinds.iter().find(|k| k.name == name).unwrap_or_else(|| {
        panic!(
            "種類「{name}」が無い: {:?}",
            v.kinds.iter().map(|k| &k.name).collect::<Vec<_>>()
        )
    })
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

    assert_eq!(
        applied.unwrap(),
        1,
        "2 回当てると s01-attribute が二重になる"
    );
    // **「末尾にある」とは書かない**（ST16 がそう書いて、この change が末尾を取った瞬間に落ちた）。
    // 見たいのは登録し忘れていないことと、**依存する版より後にあること** ——
    // 主張の錠は `core.event` と `core.erasure_ledger` を前提にする。
    let names: Vec<&str> = crate::MIGRATIONS.iter().map(|(n, _)| *n).collect();
    let mine = names
        .iter()
        .position(|n| n.ends_with("_personal_attributes"))
        .expect("主張の移行が MIGRATIONS に無い（当て忘れると錠が本番だけ効かない）");
    let gates = names
        .iter()
        .position(|n| n.ends_with("_gates"))
        .expect("ST03 の門の版が MIGRATIONS に無い");
    assert!(
        mine > gates,
        "主張の移行が、前提にしている門の版より前にある"
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
    put_claim(
        &app.pool,
        u,
        job,
        Some("会社員"),
        "year",
        Some("2020"),
        "2026-09-01T01:00:00Z",
    )
    .await;
    put_claim(
        &app.pool,
        u,
        job,
        Some("自営業"),
        "year",
        Some("2024"),
        "2026-09-02T01:00:00Z",
    )
    .await;

    attributes_store::rename_kind(&app.pool, u, job, "仕事")
        .await
        .unwrap()
        .expect("名前を変えられる");

    let after = read_view(&app, u).await;
    let renamed = kind_named(&after, "仕事");
    assert_eq!(renamed.id, job, "名前を変えたら識別子が変わった");
    assert_eq!(renamed.claims.len(), 2, "名前を変えたら主張が失われた");
    assert_eq!(
        renamed.current.as_ref().unwrap().value.as_deref(),
        Some("自営業")
    );
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
    assert_eq!(
        names,
        ["住所", "職業", "副業"],
        "住所と職業が先に置かれていない"
    );
}

#[tokio::test]
// Scenario: 空の名前の種類は足せない
async fn kinds_reject_empty_name() {
    let app = app().await;
    let u = testdb::user();
    for blank in ["", "   ", "\u{3000}"] {
        assert_eq!(
            attributes_store::add_kind(&app.pool, u, blank)
                .await
                .unwrap(),
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
        attributes_store::add_kind(&app.pool, u, "住所")
            .await
            .unwrap(),
        Err(attributes_store::KindError::DuplicateName),
    );
    // 前後の空白を除いてから比べる
    assert_eq!(
        attributes_store::add_kind(&app.pool, u, " 住所 ")
            .await
            .unwrap(),
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
    assert_eq!(
        kind_named(&v, "副業").id,
        side.id,
        "断ったのに名前が変わっている"
    );

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
    assert!(v
        .kinds
        .iter()
        .all(|k| k.current.is_none() && k.claims.is_empty()));
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
    assert_eq!(
        names, 2,
        "同時の初期化で名前の台帳が {names} 行になった（消せない）"
    );
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
    put_claim(
        &app.pool,
        u,
        address,
        Some("新居"),
        "day",
        Some("2026-10-01"),
        "2026-09-20T01:00:00Z",
    )
    .await;

    let at: chrono::DateTime<chrono::Utc> = "2026-09-30T16:00:00Z".parse().unwrap();
    let today = stay_store::jst_date(at);
    assert_eq!(today, chrono::NaiveDate::from_ymd_opt(2026, 10, 1).unwrap());

    let v = attributes_store::attributes_view(&app.pool, u, today)
        .await
        .unwrap();
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
    put_claim(
        &app.pool,
        u,
        address,
        Some("旧居"),
        "month",
        Some("2019-10"),
        "2026-09-01T01:00:00Z",
    )
    .await;
    let newest = put_claim(
        &app.pool,
        u,
        address,
        Some("新居"),
        "month",
        Some("2023-03"),
        "2026-09-02T01:00:00Z",
    )
    .await;

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
    let a = put_claim(
        &app.pool,
        u,
        address,
        Some("A"),
        "month",
        Some("2019-04"),
        "2026-09-01T01:00:00Z",
    )
    .await;
    let b = put_claim_superseding(
        &app.pool,
        u,
        address,
        Some("A"),
        "month",
        Some("2019-10"),
        "2026-09-02T01:00:00Z",
        Some(a),
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
    assert_eq!(
        got.claims.iter().map(|c| c.id).collect::<Vec<_>>(),
        vec![a],
        "A が積んだ主張に戻っていない"
    );
    assert!(
        got.superseded.is_empty(),
        "消した主張の取り消しがまだ効いている"
    );
}

#[tokio::test]
// Scenario: 本文を消去した主張は出ず、その取り消しも効かない
/// **消去した主張は種類も値も読めない**ので、並べる置き場を持たない（spec-review R18）。
async fn read_erased_claims_vanish_and_their_supersession_lifts() {
    let app = app().await;
    let u = testdb::user();
    let v0 = read_view(&app, u).await;
    let address = kind_named(&v0, "住所").id;
    let a = put_claim(
        &app.pool,
        u,
        address,
        Some("A"),
        "month",
        Some("2019-04"),
        "2026-09-01T01:00:00Z",
    )
    .await;
    let b = put_claim_superseding(
        &app.pool,
        u,
        address,
        Some("A"),
        "month",
        Some("2019-10"),
        "2026-09-02T01:00:00Z",
        Some(a),
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
    let seen: Vec<uuid::Uuid> = got
        .claims
        .iter()
        .chain(&got.superseded)
        .map(|c| c.id)
        .collect();
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
    put_claim(
        &app.pool,
        u,
        address,
        Some("A"),
        "month",
        Some("2019-10"),
        "2026-09-15T02:30:00Z",
    )
    .await;

    let v = read_view(&app, u).await;
    let c = kind_named(&v, "住所").claims.first().unwrap();
    assert!(
        c.asserted_at.starts_with("2026-09-15"),
        "主張した日時が入っていない: {}",
        c.asserted_at
    );
    assert!(
        c.asserted_at.ends_with("+09:00"),
        "主張した日時に地域のずれが無い: {}",
        c.asserted_at
    );
    assert!(!c.ingested_at.is_empty(), "D-01 に入った時刻が無い");
    assert_ne!(
        c.asserted_at, c.ingested_at,
        "2 つの時刻が同じ欄から来ている"
    );
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
    let id = put_claim(
        &app.pool,
        u,
        address,
        Some("秘密の住所"),
        "year",
        Some("2019"),
        "2026-09-01T01:00:00Z",
    )
    .await;
    // 「AI に出さない」（PERM-2 の 4 段階のいちばん厳しい側）へ締める
    sqlx::query("UPDATE core.event SET sensitivity = 3 WHERE id = $1")
        .bind(id)
        .execute(&app.pool)
        .await
        .unwrap();

    let v = read_view(&app, u).await;
    let got = kind_named(&v, "住所");
    assert_eq!(
        got.claims.iter().map(|c| c.id).collect::<Vec<_>>(),
        vec![id]
    );
    assert_eq!(
        got.current.as_ref().unwrap().value.as_deref(),
        Some("秘密の住所")
    );
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
    let a_claim = put_claim(
        &app.pool,
        a,
        a_address,
        Some("A の住所"),
        "year",
        Some("2019"),
        "2026-09-01T01:00:00Z",
    )
    .await;
    let b_claim = put_claim(
        &app.pool,
        b,
        b_address,
        Some("B の住所"),
        "year",
        Some("2019"),
        "2026-09-01T01:00:00Z",
    )
    .await;

    let v = read_view(&app, a).await;
    let ids: Vec<uuid::Uuid> = v
        .kinds
        .iter()
        .flat_map(|k| k.claims.iter().map(|c| c.id))
        .collect();
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

// ================================================================ 3.1 取り込み口の分岐

/// その利用者の「住所」の識別子を引く（初期化も済ませる）。
async fn address_kind(app: &App, user: uuid::Uuid) -> uuid::Uuid {
    kind_named(&read_view(app, user).await, "住所").id
}

/// 通る形の主張を 1 件送って、格納された識別子を返す。
async fn store_claim(
    app: &App,
    user: uuid::Uuid,
    kind: uuid::Uuid,
    value: Option<&str>,
    precision: &str,
    date: Option<&str>,
    asserted_at: &str,
) -> uuid::Uuid {
    let id = uuid::Uuid::new_v4();
    let raw = claim_raw(id, kind, value, precision, date, None, None, NONCE);
    let res = send_claim(app, claim_item(id, user, asserted_at, &raw)).await;
    assert!(res.accepted, "主張が受け付けられなかった: {:?}", res.error);
    res.id.expect("格納された識別子")
}

#[tokio::test]
// Scenario: 無い種類の主張は受け付けない
async fn ingest_rejects_unknown_kind() {
    let app = app().await;
    let u = testdb::user();
    address_kind(&app, u).await; // 初期化だけ済ませる
    let id = uuid::Uuid::new_v4();
    let raw = claim_raw(
        id,
        uuid::Uuid::new_v4(),
        Some("A"),
        "year",
        Some("2019"),
        None,
        None,
        NONCE,
    );
    let res = send_claim(&app, claim_item(id, u, "2026-09-15T02:00:00Z", &raw)).await;
    assert!(!res.accepted);
    assert_eq!(
        serde_json::to_value(res.error).unwrap(),
        serde_json::json!("unknown_attribute_kind")
    );
}

#[tokio::test]
// Scenario: 別の種類の主張は取り消せない
/// **種類まで見ないと、別の種類の主張を取り消して「いまの値」が黙って変わる。**
async fn ingest_rejects_supersedes_from_another_kind() {
    let app = app().await;
    let u = testdb::user();
    let v = read_view(&app, u).await;
    let address = kind_named(&v, "住所").id;
    let job = kind_named(&v, "職業").id;
    let job_claim = store_claim(
        &app,
        u,
        job,
        Some("会社員"),
        "year",
        Some("2020"),
        "2026-09-01T01:00:00Z",
    )
    .await;

    // 「職業」の主張を、「住所」の主張として取り消す
    let id = uuid::Uuid::new_v4();
    let raw = claim_raw(
        id,
        address,
        Some("東京"),
        "year",
        Some("2021"),
        Some(job_claim),
        None,
        NONCE,
    );
    let res = send_claim(&app, claim_item(id, u, "2026-09-02T01:00:00Z", &raw)).await;
    assert!(!res.accepted);
    assert_eq!(
        serde_json::to_value(res.error).unwrap(),
        serde_json::json!("invalid_supersedes")
    );
}

#[tokio::test]
// Scenario: 無い主張は取り消せない
// Scenario: 自分自身は取り消せない
// Scenario: 主張でない記録は取り消せない
async fn ingest_rejects_bad_supersedes_targets() {
    let app = app().await;
    let u = testdb::user();
    let address = address_kind(&app, u).await;

    // 位置の記録（主張でない記録）を 1 件置く
    let other = uuid::Uuid::new_v4();
    let source = testdb::source(&app.pool, "st19-other", 21_600).await;
    sqlx::query(
        "INSERT INTO core.event
           (id, user_id, logical_source, device_id, origin, event_time, tz_offset_min, tz_id,
            schema_version, content_hash, raw, payload)
         VALUES ($1,$2,$3,'dev','collected','2026-09-01T01:00:00Z',540,'Asia/Tokyo',1,$4,'{}','{}')",
    )
    .bind(other)
    .bind(u)
    .bind(&source)
    .bind(other.to_string())
    .execute(&app.pool)
    .await
    .unwrap();

    let self_id = uuid::Uuid::new_v4();
    for (why, target, id) in [
        ("無い主張", uuid::Uuid::new_v4(), uuid::Uuid::new_v4()),
        ("自分自身", self_id, self_id),
        ("主張でない記録", other, uuid::Uuid::new_v4()),
    ] {
        let raw = claim_raw(
            id,
            address,
            Some("A"),
            "year",
            Some("2019"),
            Some(target),
            None,
            NONCE,
        );
        let res = send_claim(&app, claim_item(id, u, "2026-09-02T01:00:00Z", &raw)).await;
        assert!(!res.accepted, "{why} を取り消せた");
        assert_eq!(
            serde_json::to_value(res.error).unwrap(),
            serde_json::json!("invalid_supersedes"),
            "{why} の種別が違う"
        );
    }
}

#[tokio::test]
// Scenario: 別の利用者の主張は取り消せない
async fn ingest_rejects_supersedes_from_another_user() {
    let app = app().await;
    let a = testdb::user();
    let b = testdb::user();
    let a_address = address_kind(&app, a).await;
    let b_address = address_kind(&app, b).await;
    let b_claim = store_claim(
        &app,
        b,
        b_address,
        Some("B の住所"),
        "year",
        Some("2019"),
        "2026-09-01T01:00:00Z",
    )
    .await;

    let id = uuid::Uuid::new_v4();
    let raw = claim_raw(
        id,
        a_address,
        Some("A の住所"),
        "year",
        Some("2020"),
        Some(b_claim),
        None,
        NONCE,
    );
    let res = send_claim(&app, claim_item(id, a, "2026-09-02T01:00:00Z", &raw)).await;
    assert!(!res.accepted);
    assert_eq!(
        serde_json::to_value(res.error).unwrap(),
        serde_json::json!("invalid_supersedes")
    );
}

#[tokio::test]
// Scenario: 消した主張を取り消し先に指せる
/// **読み出しがその取り消しを効かせる相手はいない**ので害が無い（design D5）。
/// 断ると、消した主張を訂正しようとした本人が行き止まりになる。
async fn ingest_allows_superseding_a_deleted_claim() {
    let app = app().await;
    let u = testdb::user();
    let address = address_kind(&app, u).await;
    let gone = store_claim(
        &app,
        u,
        address,
        Some("旧居"),
        "year",
        Some("2019"),
        "2026-09-01T01:00:00Z",
    )
    .await;
    sqlx::query("UPDATE core.event SET deleted_at = now(), deleted_by = 'test' WHERE id = $1")
        .bind(gone)
        .execute(&app.pool)
        .await
        .unwrap();

    let id = uuid::Uuid::new_v4();
    let raw = claim_raw(
        id,
        address,
        Some("新居"),
        "year",
        Some("2020"),
        Some(gone),
        None,
        NONCE,
    );
    let res = send_claim(&app, claim_item(id, u, "2026-09-02T01:00:00Z", &raw)).await;
    assert!(
        res.accepted,
        "消した主張を取り消し先に指せない: {:?}",
        res.error
    );
}

#[tokio::test]
// Scenario: 1 つの主張を 2 つの主張が取り消せる
async fn ingest_allows_two_claims_to_supersede_one() {
    let app = app().await;
    let u = testdb::user();
    let address = address_kind(&app, u).await;
    let target = store_claim(
        &app,
        u,
        address,
        Some("A"),
        "year",
        Some("2019"),
        "2026-09-01T01:00:00Z",
    )
    .await;

    for (n, value) in [(1, "B"), (2, "C")] {
        let id = uuid::Uuid::new_v4();
        let raw = claim_raw(
            id,
            address,
            Some(value),
            "year",
            Some("2020"),
            Some(target),
            None,
            NONCE,
        );
        let res = send_claim(
            &app,
            claim_item(id, u, &format!("2026-09-0{}T01:00:00Z", n + 1), &raw),
        )
        .await;
        assert!(res.accepted, "{n} 件目が受け付けられない: {:?}", res.error);
    }
}

#[tokio::test]
// Scenario: 本人が書いたでない主張は受け付けない
// Scenario: 端末識別子を持つ主張は受け付けない
async fn ingest_rejects_claims_that_are_not_authored() {
    let app = app().await;
    let u = testdb::user();
    let address = address_kind(&app, u).await;

    for (why, field, value) in [
        ("由来が派生", "origin", "derived"),
        ("由来が収集", "origin", "collected"),
        ("端末識別子を持つ", "device_id", "dev-1"),
    ] {
        let id = uuid::Uuid::new_v4();
        let raw = claim_raw(
            id,
            address,
            Some("A"),
            "year",
            Some("2019"),
            None,
            None,
            NONCE,
        );
        let mut item = claim_item(id, u, "2026-09-15T02:00:00Z", &raw);
        item[field] = serde_json::json!(value);
        let res = send_claim(&app, item).await;
        assert!(!res.accepted, "{why} の主張が通っている");
        assert_eq!(
            serde_json::to_value(res.error).unwrap(),
            serde_json::json!("claim_not_authored"),
            "{why} の種別が違う"
        );
    }
}

#[tokio::test]
// Scenario: 外部識別子を持つ主張は受け付けない
/// 主張は外部サービスから来ない。**識別子を持てると重複の判定の経路が変わる。**
async fn ingest_rejects_claims_with_external_ids() {
    let app = app().await;
    let u = testdb::user();
    let address = address_kind(&app, u).await;

    for field in ["external_id", "external_ref"] {
        let id = uuid::Uuid::new_v4();
        let raw = claim_raw(
            id,
            address,
            Some("A"),
            "year",
            Some("2019"),
            None,
            None,
            NONCE,
        );
        let mut item = claim_item(id, u, "2026-09-15T02:00:00Z", &raw);
        item[field] = serde_json::json!("ext-1");
        let res = send_claim(&app, item).await;
        assert!(!res.accepted, "{field} を持つ主張が通っている");
        assert_eq!(
            serde_json::to_value(res.error).unwrap(),
            serde_json::json!("claim_has_external_id"),
            "{field} の種別が違う"
        );
    }
}

#[tokio::test]
// Scenario: 乱数が短い主張は受け付けない
async fn ingest_rejects_short_nonce() {
    let app = app().await;
    let u = testdb::user();
    let address = address_kind(&app, u).await;
    let id = uuid::Uuid::new_v4();
    // 64 bit を base64url で書くと 11 文字
    let raw = claim_raw(
        id,
        address,
        Some("A"),
        "year",
        Some("2019"),
        None,
        None,
        "MTIzNDU2Nzg",
    );
    let res = send_claim(&app, claim_item(id, u, "2026-09-15T02:00:00Z", &raw)).await;
    assert!(!res.accepted);
    assert_eq!(
        serde_json::to_value(res.error).unwrap(),
        serde_json::json!("malformed_claim")
    );
}

#[tokio::test]
/// 値と「いつから」の種別も、応答の値まで見て固定する（spec の表）。
async fn ingest_maps_value_and_valid_from_errors() {
    let app = app().await;
    let u = testdb::user();
    let address = address_kind(&app, u).await;

    for (why, value, precision, date, want) in [
        (
            "空の値",
            Some("   "),
            "year",
            Some("2019"),
            "invalid_claim_value",
        ),
        (
            "精度と日付が合わない",
            Some("A"),
            "year",
            Some("2019-10-01"),
            "invalid_valid_from",
        ),
        (
            "暦に無い日付",
            Some("A"),
            "day",
            Some("2019-02-30"),
            "invalid_valid_from",
        ),
    ] {
        let id = uuid::Uuid::new_v4();
        let raw = claim_raw(id, address, value, precision, date, None, None, NONCE);
        let res = send_claim(&app, claim_item(id, u, "2026-09-15T02:00:00Z", &raw)).await;
        assert!(!res.accepted, "{why} が通っている");
        assert_eq!(
            serde_json::to_value(res.error).unwrap(),
            serde_json::json!(want),
            "{why} の種別が違う"
        );
    }
}

#[tokio::test]
// Scenario: 主張の拒否の応答に値が含まれない
/// **値をそのまま返すと、呼び出し元へ内容が反射する**（design D5 / 製造準備 A-2）。
async fn ingest_rejection_does_not_echo_the_value() {
    let app = app().await;
    let u = testdb::user();
    let address = address_kind(&app, u).await;
    let secret = "東京都 目黒区 青葉台 9-9-9";
    let secret_note = "誰にも見せたくない補足";

    let id = uuid::Uuid::new_v4();
    // 暦に無い日付で断らせる（値と補足は形として通る）
    let raw = claim_raw(
        id,
        address,
        Some(secret),
        "day",
        Some("2019-02-30"),
        None,
        Some(secret_note),
        NONCE,
    );
    let (_, res) = post_ingest(
        &app,
        serde_json::json!([claim_item(id, u, "2026-09-15T02:00:00Z", &raw)]),
    )
    .await;
    let body = serde_json::to_string(&res).unwrap();
    assert!(!res[0].accepted);
    assert!(!body.contains(secret), "応答に値が含まれている: {body}");
    assert!(
        !body.contains(secret_note),
        "応答に補足が含まれている: {body}"
    );
}

#[tokio::test]
// Scenario: 主張はローカル AI までで格納される
// Scenario: 主張以外の既定は変わらない
/// PERM-4（主観・感情と同じ「ローカル AI まで」。深掘り Q3）と PERM-3（収集は「外部 AI に出してよい」）。
async fn ingest_default_sensitivity() {
    let app = app().await;
    let u = testdb::user();
    let address = address_kind(&app, u).await;
    let claim = store_claim(
        &app,
        u,
        address,
        Some("A"),
        "year",
        Some("2019"),
        "2026-09-15T02:00:00Z",
    )
    .await;

    let (s,): (i16,) = sqlx::query_as("SELECT sensitivity FROM core.event WHERE id = $1")
        .bind(claim)
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(
        i32::from(s),
        attributes::DEFAULT_SENSITIVITY,
        "主張の感度が「ローカル AI まで」でない"
    );

    // 端末からの位置の記録は既定のまま
    let source = testdb::source(&app.pool, "st19-sens", 21_600).await;
    let other = uuid::Uuid::new_v4();
    let (_, res) = post_ingest(
        &app,
        serde_json::json!([{
            "id": other, "user_id": u, "logical_source": source,
            "external_id": null, "device_id": "dev-1", "origin": "collected",
            "event_time": "2026-09-15T02:00:00Z", "tz_offset_min": 540, "tz_id": "Asia/Tokyo",
            "schema_version": 1, "raw": r#"{"lat":1}"#, "payload": {},
        }]),
    )
    .await;
    assert!(
        res[0].accepted,
        "位置の記録が受け付けられない: {:?}",
        res[0].error
    );
    let (s,): (i16,) = sqlx::query_as("SELECT sensitivity FROM core.event WHERE id = $1")
        .bind(other)
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(
        i32::from(s),
        DEFAULT_SENSITIVITY,
        "主張以外の既定の感度が動いている"
    );
}

#[tokio::test]
/// **コードが持つ既定と、DB の列の既定が同じ値であること。**
/// 片方だけ動くと、既定が黙って緩む側にも締まる側にも転びうる。
async fn default_sensitivity_matches_the_column() {
    let pool = testdb::pool().await;
    let (default,): (Option<String>,) = sqlx::query_as(
        "SELECT column_default FROM information_schema.columns
          WHERE table_schema = 'core' AND table_name = 'event' AND column_name = 'sensitivity'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let got = default.expect("感度の列に既定が無い");
    assert!(
        got.starts_with(&DEFAULT_SENSITIVITY.to_string()),
        "DB の既定（{got}）がコードの既定（{DEFAULT_SENSITIVITY}）と違う"
    );
}

// ================================================================ 3.2 格納

#[tokio::test]
// Scenario: 住所を 2 回変えると 3 つの主張が残る
/// **完了の判定 1**（`docs/stories/ST19.md`）。A → B → A と書いて 3 件 ——
/// **同じ値へ戻しても畳まれない**（内容の鍵に原文の識別子と乱数が入る。深掘り C2）。
async fn store_three_claims_after_two_moves() {
    let app = app().await;
    let u = testdb::user();
    let address = address_kind(&app, u).await;
    for (n, value) in [
        (1, "東京都 目黒区"),
        (2, "東京都 世田谷区"),
        (3, "東京都 目黒区"),
    ] {
        store_claim(
            &app,
            u,
            address,
            Some(value),
            "year",
            Some("2019"),
            &format!("2026-09-0{n}T01:00:00Z"),
        )
        .await;
    }
    let v = read_view(&app, u).await;
    let got = kind_named(&v, "住所");
    assert_eq!(
        got.claims.len(),
        3,
        "住所を 2 回変えたのに主張が 3 件残っていない"
    );
    assert_eq!(
        got.current.as_ref().unwrap().value.as_deref(),
        Some("東京都 目黒区"),
        "最後に書いた値がいまの値でない"
    );
}

#[tokio::test]
// Scenario: 主張した日時といつからが別々に入る
// Scenario: D-01 に入った時刻も別に残る
/// **完了の判定 2**（`docs/stories/ST19.md`）。FR-45 の 2 軸を DB の列で確かめる。
async fn store_keeps_both_times_in_separate_columns() {
    let app = app().await;
    let u = testdb::user();
    let address = address_kind(&app, u).await;
    // 2026-09-15 に「いつから」を 2019 年 10 月とする主張を書く
    let id = store_claim(
        &app,
        u,
        address,
        Some("東京都"),
        "month",
        Some("2019-10"),
        "2026-09-15T02:00:00Z",
    )
    .await;

    let (event_time, ingest_time, payload): (
        chrono::DateTime<chrono::Utc>,
        chrono::DateTime<chrono::Utc>,
        serde_json::Value,
    ) = sqlx::query_as("SELECT event_time, ingest_time, payload FROM core.event WHERE id = $1")
        .bind(id)
        .fetch_one(&app.pool)
        .await
        .unwrap();

    // 主張した日時は**出来事の時刻**に入る（深掘り C4 / C13）
    assert_eq!(event_time.to_rfc3339(), "2026-09-15T02:00:00+00:00");
    // 「いつから」は**精度つきの別の欄**（出来事の時刻に入れると「分からない」を置く値が要る）
    assert_eq!(
        payload["valid_from"]["precision"],
        serde_json::json!("month")
    );
    assert_eq!(payload["valid_from"]["date"], serde_json::json!("2019-10"));
    // D-01 に入った時刻は**さらに別**（FR-19。サーバの時計）
    assert_ne!(
        ingest_time, event_time,
        "D-01 に入った時刻が出来事の時刻と同じ欄から来ている"
    );

    let v = read_view(&app, u).await;
    let c = kind_named(&v, "住所").claims.first().unwrap();
    assert!(c.asserted_at.starts_with("2026-09-15"));
    assert_eq!(c.valid_from.date.as_deref(), Some("2019-10"));
    assert!(!c.ingested_at.is_empty());
}

#[tokio::test]
// Scenario: 年だけ分かるいつからは年のまま残る
// Scenario: いつからが分からない主張を受け付ける
// Scenario: 未来のいつからを受け付ける
// Scenario: なしの主張を受け付ける
/// **精度を丸めない**（深掘り C3）。2019 年とだけ分かっている値を 2019-01-01 にしない。
async fn store_keeps_precision_and_accepts_the_edges() {
    let app = app().await;
    let u = testdb::user();
    let v0 = read_view(&app, u).await;
    let address = kind_named(&v0, "住所").id;
    let job = kind_named(&v0, "職業").id;

    // 年だけ
    let year = store_claim(
        &app,
        u,
        address,
        Some("東京都"),
        "year",
        Some("2019"),
        "2026-09-01T01:00:00Z",
    )
    .await;
    // 分からない
    let unknown = store_claim(
        &app,
        u,
        address,
        Some("実家"),
        "unknown",
        None,
        "2026-09-02T01:00:00Z",
    )
    .await;
    // 未来（来月から新しい住所。深掘り C6）
    let future = store_claim(
        &app,
        u,
        address,
        Some("新居"),
        "day",
        Some("2026-12-01"),
        "2026-09-03T01:00:00Z",
    )
    .await;
    // 「なし」（副業をやめた。深掘り C10）
    let none = store_claim(
        &app,
        u,
        job,
        None,
        "year",
        Some("2025"),
        "2026-09-04T01:00:00Z",
    )
    .await;

    let v = read_view(&app, u).await;
    let addr = kind_named(&v, "住所");
    let find = |id: uuid::Uuid| {
        addr.claims
            .iter()
            .chain(&addr.upcoming)
            .find(|c| c.id == id)
            .unwrap_or_else(|| panic!("{id} が読み出せない"))
    };
    let y = find(year);
    assert_eq!(y.valid_from.precision, Precision::Year);
    assert_eq!(
        y.valid_from.date.as_deref(),
        Some("2019"),
        "年が月日まで丸められている"
    );
    let un = find(unknown);
    assert_eq!(un.valid_from.precision, Precision::Unknown);
    assert_eq!(un.valid_from.date, None, "「分からない」に日付が入っている");
    // 未来は予定に出て、いまの値にならない
    assert_eq!(
        addr.upcoming.iter().map(|c| c.id).collect::<Vec<_>>(),
        vec![future]
    );
    assert_ne!(
        addr.current.as_ref().unwrap().id,
        future,
        "未来の主張がいまの値になっている"
    );

    let j = kind_named(&v, "職業");
    assert_eq!(j.current.as_ref().unwrap().id, none);
    assert_eq!(
        j.current.as_ref().unwrap().value,
        None,
        "「なし」が値として読めない"
    );
}

#[tokio::test]
// Scenario: 同じ値といつからを書き直しても 1 件増える
// Scenario: 同じ主張の再送は増えない
/// **「その日にまだそうだと確かめた」が残る**（深掘り C2）。畳まれるのは原文が同じ再送だけ。
async fn store_same_value_stacks_but_resend_does_not() {
    let app = app().await;
    let u = testdb::user();
    let address = address_kind(&app, u).await;

    // 同じ値・同じ「いつから」を 2 回**書く**（識別子と主張した日時が違う）
    store_claim(
        &app,
        u,
        address,
        Some("東京都"),
        "year",
        Some("2019"),
        "2026-09-01T01:00:00Z",
    )
    .await;
    store_claim(
        &app,
        u,
        address,
        Some("東京都"),
        "year",
        Some("2019"),
        "2026-09-02T01:00:00Z",
    )
    .await;
    assert_eq!(
        kind_named(&read_view(&app, u).await, "住所").claims.len(),
        2,
        "同じ値をもう一度書いたら畳まれた（確かめた事実が消える）"
    );

    // **同じ原文を再送**（通信が切れて画面が送り直す）。1 件のまま
    let id = uuid::Uuid::new_v4();
    let raw = claim_raw(
        id,
        address,
        Some("大阪府"),
        "year",
        Some("2020"),
        None,
        None,
        NONCE,
    );
    let item = claim_item(id, u, "2026-09-03T01:00:00Z", &raw);
    let first = send_claim(&app, item.clone()).await;
    let second = send_claim(&app, item).await;
    assert!(first.accepted && second.accepted, "再送が受理されない");
    assert!(second.duplicate, "再送が重複として返っていない");
    assert_eq!(first.id, second.id, "再送で別の識別子が返った");
    assert_eq!(
        kind_named(&read_view(&app, u).await, "住所").claims.len(),
        3,
        "同じ原文の再送で主張が増えた"
    );
}

#[tokio::test]
// Scenario: 補足が残る
async fn store_keeps_the_note() {
    let app = app().await;
    let u = testdb::user();
    let address = address_kind(&app, u).await;
    let id = uuid::Uuid::new_v4();
    let note = "転職に合わせて引っ越した";
    let raw = claim_raw(
        id,
        address,
        Some("東京都"),
        "year",
        Some("2019"),
        None,
        Some(note),
        NONCE,
    );
    assert!(
        send_claim(&app, claim_item(id, u, "2026-09-01T01:00:00Z", &raw))
            .await
            .accepted
    );

    let v = read_view(&app, u).await;
    assert_eq!(
        kind_named(&v, "住所").claims[0].note.as_deref(),
        Some(note),
        "補足が失われている"
    );
}

#[tokio::test]
// Scenario: 主張の原文が 1 バイトも変わらずに残る
/// **原文のバイト列は一度変換すると二度と戻らない**（FR-18 / design D2）。
async fn store_keeps_the_raw_byte_for_byte() {
    let app = app().await;
    let u = testdb::user();
    let address = address_kind(&app, u).await;
    let id = uuid::Uuid::new_v4();
    // **表記を揺らした原文**（空白つき・欄の並びが design の順でない）を送る
    let raw = format!(
        r#"{{ "nonce" : "{NONCE}", "claim":"{id}", "kind":"{address}", "note":null, "value":"東京都 目黒区", "supersedes":null, "valid_from":{{"date":"2019-10","precision":"month"}} }}"#
    );
    assert!(
        send_claim(&app, claim_item(id, u, "2026-09-01T01:00:00Z", &raw))
            .await
            .accepted
    );

    let (stored,): (String,) = sqlx::query_as("SELECT raw FROM core.event WHERE id = $1")
        .bind(id)
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(
        stored.as_bytes(),
        raw.as_bytes(),
        "原文がバイト単位で一致しない"
    );
}

#[tokio::test]
// Scenario: 合成済みでない値は合成済みで読み出される
// Scenario: 原文と食い違う解析済みを送っても原文の値で格納される
/// **解析済みは原文から組み直す**（design D1）—— 送り主の `payload` は使わない。
/// 使うと、原文と解析済みがずれた行を送り主が自由に作れる。
async fn store_builds_payload_from_the_raw() {
    let app = app().await;
    let u = testdb::user();
    let address = address_kind(&app, u).await;

    // (a) NFD（「か」+ 濁点）で書いた値は NFC で読み出される
    let id = uuid::Uuid::new_v4();
    let nfd = "\u{304B}\u{3099}";
    let raw = claim_raw(
        id,
        address,
        Some(nfd),
        "year",
        Some("2019"),
        None,
        None,
        NONCE,
    );
    assert!(
        send_claim(&app, claim_item(id, u, "2026-09-01T01:00:00Z", &raw))
            .await
            .accepted
    );

    // (b) 原文が「東京都」で、送り主の解析済みが「大阪府」の主張
    let forged = uuid::Uuid::new_v4();
    let raw2 = claim_raw(
        forged,
        address,
        Some("東京都"),
        "year",
        Some("2020"),
        None,
        None,
        NONCE,
    );
    let mut item = claim_item(forged, u, "2026-09-02T01:00:00Z", &raw2);
    item["payload"] = serde_json::json!({ "value": "大阪府", "kind": address });
    assert!(send_claim(&app, item).await.accepted);

    let v = read_view(&app, u).await;
    let got = kind_named(&v, "住所");
    let by = |id: uuid::Uuid| got.claims.iter().find(|c| c.id == id).unwrap();
    assert_eq!(
        by(id).value.as_deref(),
        Some("\u{304C}"),
        "値が NFC で読み出されていない"
    );
    assert_eq!(
        by(forged).value.as_deref(),
        Some("東京都"),
        "送り主の解析済みの値が格納されている（原文とずれた行が作れる）"
    );
    // 原文は受け取ったまま（NFD のまま残る）
    let (stored,): (String,) = sqlx::query_as("SELECT raw FROM core.event WHERE id = $1")
        .bind(id)
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert!(
        stored.contains(nfd),
        "原文まで NFC にされている（受け取ったままでない）"
    );
}

// ================================================================ 3.3 乱数（design D4 / 深掘り C12）

#[tokio::test]
// Scenario: 乱数は解析済みに写らない
/// **`id` だけでは守れない**（`id` の列は消去の後も残る）。原文にだけ入る乱数が要る。
async fn erasure_nonce_is_not_copied_to_the_payload() {
    let app = app().await;
    let u = testdb::user();
    let address = address_kind(&app, u).await;
    let id = store_claim(
        &app,
        u,
        address,
        Some("東京都 目黒区"),
        "month",
        Some("2019-10"),
        "2026-09-01T01:00:00Z",
    )
    .await;

    let (payload,): (serde_json::Value,) =
        sqlx::query_as("SELECT payload FROM core.event WHERE id = $1")
            .bind(id)
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert!(
        !payload.to_string().contains(NONCE),
        "解析済みに乱数が写っている: {payload}"
    );

    let v = read_view(&app, u).await;
    assert!(
        !serde_json::to_string(&v).unwrap().contains(NONCE),
        "読み出しの応答に乱数が出ている"
    );
}

#[tokio::test]
// Scenario: 消去後に残る列と正しい値から鍵を作り直せない
/// **いまの鍵は塩の無い SHA-256(ソース, 出来事の時刻, 原文)**（深掘り C12 / spec-review R3）。
/// 住所は候補が少ないので、乱数が無ければ残った列から総当たりで確かめられる。
async fn erasure_hash_cannot_be_rebuilt_from_what_remains() {
    let app = app().await;
    let u = testdb::user();
    let address = address_kind(&app, u).await;
    let value = "東京都 目黒区";
    let id = store_claim(
        &app,
        u,
        address,
        Some(value),
        "month",
        Some("2019-10"),
        "2026-09-01T01:00:00Z",
    )
    .await;

    // 台帳つきで消去する（門が通す唯一の形）
    let mut tx = app.pool.begin().await.unwrap();
    sqlx::query(
        "INSERT INTO core.erasure_ledger (event_id, user_id, logical_source, scope, erased_by)
         VALUES ($1, $2, $3, 'event', 'test')",
    )
    .bind(id)
    .bind(u)
    .bind(attributes::SOURCE)
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::query("UPDATE core.event SET raw = '', payload = '{}'::jsonb WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.expect("台帳つきの消去は通る");

    // **消去の後に残る列**を読む（原文と解析済みは空）
    let (event_time, raw, content_hash): (chrono::DateTime<chrono::Utc>, String, String) =
        sqlx::query_as("SELECT event_time, raw, content_hash FROM core.event WHERE id = $1")
            .bind(id)
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert!(raw.is_empty(), "消去したのに原文が残っている");
    assert!(
        !content_hash.is_empty(),
        "消去で鍵まで消えている（前提が変わっている）"
    );

    // **正しい種類・値・「いつから」を知っていても**、乱数を知らなければ鍵は作れない
    let guess = serde_json::json!({
        "claim": id, "kind": address, "value": value,
        "valid_from": { "precision": "month", "date": "2019-10" },
        "supersedes": null, "note": null,
    })
    .to_string();
    assert_ne!(
        ingest::content_hash_of(attributes::SOURCE, event_time, &guess),
        content_hash,
        "乱数を知らないまま鍵を作り直せた（消した値が総当たりで確かめられる）"
    );
}

// ================================================================ 独立レビューで足したもの（review/code.md）

#[tokio::test]
// Scenario: 主張はローカル AI までで格納される
/// **R2**: 既定の感度を、実装の定数ではなく**リテラル**と突き合わせる。
/// 定数自身と比べていたときは、2 を 1 に変えても 280 件全部緑だった（本人の決定が回帰から守られていない）。
async fn ingest_default_sensitivity_is_pinned_to_two() {
    // PERM-2 の 4 段階: 0 公開可 / 1 外部 AI に出してよい / **2 ローカル AI まで** / 3 AI に出さない
    const PERM4_LOCAL_AI: i16 = 2;
    assert_eq!(
        attributes::DEFAULT_SENSITIVITY,
        i32::from(PERM4_LOCAL_AI),
        "本人が Q3 で選んだ既定（ローカル AI まで）が動いている"
    );

    let app = app().await;
    let u = testdb::user();
    let address = address_kind(&app, u).await;
    let id = store_claim(
        &app,
        u,
        address,
        Some("A"),
        "year",
        Some("2019"),
        "2026-09-15T02:00:00Z",
    )
    .await;
    let (s,): (i16,) = sqlx::query_as("SELECT sensitivity FROM core.event WHERE id = $1")
        .bind(id)
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(s, PERM4_LOCAL_AI, "主張の感度が「ローカル AI まで」でない");
}

#[tokio::test]
// Scenario: 今日は Asia/Tokyo の日付で決まる
/// **R4**: **ハンドラの高さで**日境界を確かめる。store を直に呼んで `today` を自分で渡す形だと、
/// ハンドラが UTC で日を切っていても誰も気付かない（実測: `today_jst()` を UTC にしても全件緑）。
async fn read_handler_cuts_the_day_in_asia_tokyo() {
    let app = app().await;
    let u = testdb::user();
    let address = address_kind(&app, u).await;
    put_claim(
        &app.pool,
        u,
        address,
        Some("新居"),
        "day",
        Some("2026-10-01"),
        "2026-09-20T01:00:00Z",
    )
    .await;

    // UTC の 2026-09-30T16:00 は `Asia/Tokyo` では 2026-10-01
    let at: chrono::DateTime<chrono::Utc> = "2026-09-30T16:00:00Z".parse().unwrap();
    let Json(view) = attributes_get(
        State(app.at(at)),
        auth(),
        Query(AttributesQuery { user_id: Some(u) }),
    )
    .await
    .expect("読み出し");

    assert_eq!(view.today, "2026-10-01", "ハンドラが UTC で日を切っている");
    let got = kind_named(&view, "住所");
    assert_eq!(
        got.current.as_ref().map(|c| c.value.as_deref()),
        Some(Some("新居")),
        "JST の 10/1 に 10/1 からの主張がまだ予定のまま"
    );
    assert!(got.upcoming.is_empty());
}

#[tokio::test]
// Scenario: 1 つの主張を 2 つの主張が取り消せる
/// **R5**: 取り消しが 2 件あるときの**読み出しが決定的**であること。
/// `HashMap` の後勝ちに任せていたときは、`superseded_by` に入るのが行の順で決まり、
/// **読み出しのたびに答えが変わった**（`ORDER BY` も無かった）。
async fn read_two_supersessions_are_deterministic() {
    let app = app().await;
    let u = testdb::user();
    let address = address_kind(&app, u).await;
    let target = store_claim(
        &app,
        u,
        address,
        Some("A"),
        "year",
        Some("2019"),
        "2026-09-01T01:00:00Z",
    )
    .await;

    let mut ids = Vec::new();
    for (n, value) in [(2, "B"), (3, "C")] {
        let id = uuid::Uuid::new_v4();
        let raw = claim_raw(
            id,
            address,
            Some(value),
            "year",
            Some("2020"),
            Some(target),
            None,
            NONCE,
        );
        let res = send_claim(
            &app,
            claim_item(id, u, &format!("2026-09-0{n}T01:00:00Z"), &raw),
        )
        .await;
        assert!(res.accepted, "{value} が受け付けられない: {:?}", res.error);
        ids.push(res.id.unwrap());
    }

    // **10 回読んで毎回同じ答え**（行の順に依らない）
    let mut seen = std::collections::HashSet::new();
    for _ in 0..10 {
        let v = read_view(&app, u).await;
        let got = kind_named(&v, "住所");
        assert_eq!(got.superseded.len(), 1, "取り消された主張が 1 件でない");
        assert_eq!(got.superseded[0].id, target);
        seen.insert(got.superseded[0].superseded_by);
    }
    assert_eq!(
        seen.len(),
        1,
        "「どの主張で取り消されたか」が読み出しのたびに変わる: {seen:?}"
    );
    // 先に書いた側（B）が勝つ。**どちらでもよいが、決まっていること**が要る
    assert_eq!(seen.into_iter().next().unwrap(), Some(ids[0]));
}

#[tokio::test]
/// **R3**: 地域のずれが範囲外の主張を**格納の前に断る**。
///
/// 断らないと `accepted: true` で入り、`GET /attributes` から**無言で消える**。
/// 主張の行は DB が削除を拒むので、一度入ると取り除くことも直すこともできない。
async fn ingest_rejects_out_of_range_tz_offset() {
    let app = app().await;
    let u = testdb::user();
    let address = address_kind(&app, u).await;

    // `FixedOffset` の上限は ±86,399 秒 = ±1,439 分。`i32` をあふれさせる値も含める
    for bad in [1440, -1440, 100_000, 40_000_000, i32::MIN] {
        let id = uuid::Uuid::new_v4();
        let raw = claim_raw(
            id,
            address,
            Some("A"),
            "year",
            Some("2019"),
            None,
            None,
            NONCE,
        );
        let mut item = claim_item(id, u, "2026-09-15T02:00:00Z", &raw);
        item["tz_offset_min"] = serde_json::json!(bad);
        let res = send_claim(&app, item).await;
        assert!(!res.accepted, "tz_offset_min = {bad} が通っている");
        assert_eq!(
            serde_json::to_value(res.error).unwrap(),
            serde_json::json!("malformed_claim"),
            "tz_offset_min = {bad} の種別が違う"
        );
    }
    // 端は通る（範囲を狭めすぎていないこと）
    for ok in [1439, -1439, 0, 540] {
        let id = uuid::Uuid::new_v4();
        let raw = claim_raw(
            id,
            address,
            Some("A"),
            "year",
            Some("2019"),
            None,
            None,
            NONCE,
        );
        let mut item = claim_item(id, u, "2026-09-15T02:00:00Z", &raw);
        item["tz_offset_min"] = serde_json::json!(ok);
        assert!(
            send_claim(&app, item).await.accepted,
            "tz_offset_min = {ok} が断られている"
        );
    }
    // **どれも読み出しから消えていない**（受理した数と読める数が一致する）
    let v = read_view(&app, u).await;
    assert_eq!(kind_named(&v, "住所").claims.len(), 4);
}

#[tokio::test]
/// **R14**: 値や補足に制御文字を持つ主張を断る。
///
/// 原文は JSON の**テキスト**なので、エスケープされた NUL は原文のバイト列に現れず、
/// `IngestRequest::validate` の原文の検査をすり抜ける。解釈すると Rust の `String` に
/// 本物の制御文字が入り、`payload`（`jsonb`）への INSERT が 22P05 で落ちて
/// **まとめ送り全体が 500 になる**（画面からは「届かなかった」に見え、本人は押し直し続ける）。
async fn ingest_rejects_control_characters() {
    let app = app().await;
    let u = testdb::user();
    let address = address_kind(&app, u).await;
    // JSON の原文に書くエスケープ（この文字列自体には制御文字を含めない）
    let nul = "\\u0000";

    for (why, value, note) in [
        ("値に NUL", format!("\"a{nul}b\""), "null".to_string()),
        ("補足に NUL", "\"ok\"".to_string(), format!("\"a{nul}b\"")),
    ] {
        let id = uuid::Uuid::new_v4();
        let raw = format!(
            r#"{{"claim":"{id}","nonce":"{NONCE}","kind":"{address}","value":{value},"valid_from":{{"precision":"year","date":"2019"}},"supersedes":null,"note":{note}}}"#
        );
        let res = send_claim(&app, claim_item(id, u, "2026-09-15T02:00:00Z", &raw)).await;
        assert!(!res.accepted, "{why} の主張が通っている");
        assert_eq!(
            serde_json::to_value(res.error).unwrap(),
            serde_json::json!("invalid_claim_value"),
            "{why} の種別が違う"
        );
    }

    // **まとめ送りの後続が止まらない**（1 件の恒久的な失敗が後続を永久に止めない）
    let bad = uuid::Uuid::new_v4();
    let good = uuid::Uuid::new_v4();
    let bad_raw = format!(
        r#"{{"claim":"{bad}","nonce":"{NONCE}","kind":"{address}","value":"a{nul}b","valid_from":{{"precision":"year","date":"2019"}},"supersedes":null,"note":null}}"#
    );
    let good_raw = claim_raw(
        good,
        address,
        Some("通る値"),
        "year",
        Some("2020"),
        None,
        None,
        NONCE,
    );
    let (code, res) = post_ingest(
        &app,
        serde_json::json!([
            claim_item(bad, u, "2026-09-15T02:00:00Z", &bad_raw),
            claim_item(good, u, "2026-09-15T03:00:00Z", &good_raw),
        ]),
    )
    .await;
    assert_eq!(code, StatusCode::OK, "1 件の不正でまとめ送り全体が落ちた");
    assert!(!res[0].accepted);
    assert!(
        res[1].accepted,
        "後続が巻き添えで落ちた: {:?}",
        res[1].error
    );
}

#[tokio::test]
/// **R17**: 本文を消去した主張は取り消し先に指せない（`payload` が空で種類が引けない）。
/// **削除の印の付いた主張は指せる**（spec が認めている）—— 2 つを分けて固定する。
async fn ingest_supersedes_rejects_an_erased_claim() {
    let app = app().await;
    let u = testdb::user();
    let address = address_kind(&app, u).await;
    let erased = store_claim(
        &app,
        u,
        address,
        Some("消される"),
        "year",
        Some("2019"),
        "2026-09-01T01:00:00Z",
    )
    .await;

    let mut tx = app.pool.begin().await.unwrap();
    sqlx::query(
        "INSERT INTO core.erasure_ledger (event_id, user_id, logical_source, scope, erased_by)
         VALUES ($1, $2, $3, 'event', 'test')",
    )
    .bind(erased)
    .bind(u)
    .bind(attributes::SOURCE)
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::query("UPDATE core.event SET raw = '', payload = '{}'::jsonb WHERE id = $1")
        .bind(erased)
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let id = uuid::Uuid::new_v4();
    let raw = claim_raw(
        id,
        address,
        Some("A"),
        "year",
        Some("2020"),
        Some(erased),
        None,
        NONCE,
    );
    let res = send_claim(&app, claim_item(id, u, "2026-09-02T01:00:00Z", &raw)).await;
    assert!(!res.accepted, "消去した主張を取り消し先に指せた");
    assert_eq!(
        serde_json::to_value(res.error).unwrap(),
        serde_json::json!("invalid_supersedes")
    );
}

#[tokio::test]
/// **R12**: 名前の台帳に行の無い種類を、黙って落とさない（その種類の主張ごと消える）。
/// 取り込み口を通れば起きないが、`core.attribute_kind` へ直に INSERT する経路は実在する。
async fn read_kind_without_a_name_is_not_dropped() {
    let app = app().await;
    let u = testdb::user();
    let address = address_kind(&app, u).await;

    // 名前の行を書かずに種類だけ置く（錠の外の経路）
    let orphan = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO core.attribute_kind (id, user_id) VALUES ($1, $2)")
        .bind(orphan)
        .bind(u)
        .execute(&app.pool)
        .await
        .unwrap();

    let v = read_view(&app, u).await;
    let ids: Vec<uuid::Uuid> = v.kinds.iter().map(|k| k.id).collect();
    assert!(ids.contains(&orphan), "名前の無い種類が読み出しから消えた");
    assert!(ids.contains(&address));
    let got = v.kinds.iter().find(|k| k.id == orphan).unwrap();
    assert_eq!(got.name, attributes_store::NAMELESS_KIND);
}

// ---------------------------------------------------------------- 種類の口（ハンドラの高さ）

/// **R21**: 種類の 2 つの書き込み口には、**ハンドラのテストが 1 本も無かった** ——
/// `authorize` を外しても 280 件全部緑になる（＝合言葉なしで誰でも種類を足せる状態が緑）。
/// 経路（合言葉・400 の本文の形・成功したものが読み出しに出る）をここで固定する。
#[tokio::test]
async fn kinds_post_requires_the_token() {
    let app = app().await;
    let (code, _) = attributes_kind_post(
        State(app.clone()),
        HeaderMap::new(),
        Json(attributes_store::KindRequest {
            user_id: Some(testdb::user()),
            name: "副業".into(),
        }),
    )
    .await
    .expect_err("合言葉なしで種類が足せた");
    assert_eq!(code, StatusCode::UNAUTHORIZED);

    let (code, _) = attributes_kind_name_post(
        State(app),
        HeaderMap::new(),
        Path(uuid::Uuid::new_v4()),
        Json(attributes_store::KindRequest {
            user_id: Some(testdb::user()),
            name: "仕事".into(),
        }),
    )
    .await
    .expect_err("合言葉なしで名前が変えられた");
    assert_eq!(code, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
// Scenario: 種類を足せる
// Scenario: 名前を変えても識別子と主張が変わらない
/// **R21**: 口を通して足し、口を通して名前を変え、読み出しに出ることまで見る。
/// 併せて `POST /attributes/kinds/{id}/names` の**経路の形**（axum 0.8 の `{id}`）を固定する。
async fn kinds_post_round_trip_through_the_handlers() {
    let app = app().await;
    let u = testdb::user();

    let Json(created) = attributes_kind_post(
        State(app.clone()),
        auth(),
        Json(attributes_store::KindRequest {
            user_id: Some(u),
            name: "副業".into(),
        }),
    )
    .await
    .expect("種類を足せる");

    let v = read_view(&app, u).await;
    assert_eq!(kind_named(&v, "副業").id, created.id);

    let code = attributes_kind_name_post(
        State(app.clone()),
        auth(),
        Path(created.id),
        Json(attributes_store::KindRequest {
            user_id: Some(u),
            name: "副収入".into(),
        }),
    )
    .await
    .expect("名前を変えられる");
    assert_eq!(code, StatusCode::NO_CONTENT);

    let after = read_view(&app, u).await;
    assert_eq!(
        kind_named(&after, "副収入").id,
        created.id,
        "名前を変えたら識別子が変わった"
    );
    assert!(after.kinds.iter().all(|k| k.name != "副業"));
}

#[tokio::test]
/// **R8 / R21**: 断った 400 の**本文の形**を固定する（`{"error":"duplicate_name"}`）。
/// 画面はこの形を読んで文を選ぶので、裸の文字列に変わると「届かなかった」に化ける。
async fn kinds_post_rejection_body_shape() {
    let app = app().await;
    let u = testdb::user();
    read_view(&app, u).await; // 住所と職業を置く

    let (code, Json(body)) = attributes_kind_post(
        State(app.clone()),
        auth(),
        Json(attributes_store::KindRequest {
            user_id: Some(u),
            name: "住所".into(),
        }),
    )
    .await
    .expect_err("重なる名前が通った");
    assert_eq!(code, StatusCode::BAD_REQUEST);
    assert_eq!(
        serde_json::to_value(body).unwrap(),
        serde_json::json!({ "error": "duplicate_name" })
    );

    // その利用者の種類でない種類は `unknown_kind`（別の利用者を名指しできたと分からせない）
    let (code, Json(body)) = attributes_kind_name_post(
        State(app),
        auth(),
        Path(uuid::Uuid::new_v4()),
        Json(attributes_store::KindRequest {
            user_id: Some(u),
            name: "仕事".into(),
        }),
    )
    .await
    .expect_err("無い種類の名前が変えられた");
    assert_eq!(code, StatusCode::BAD_REQUEST);
    assert_eq!(
        serde_json::to_value(body).unwrap(),
        serde_json::json!({ "error": "unknown_kind" })
    );
}
