// SPDX-License-Identifier: AGPL-3.0-only
//! 幅のある出来事の終わりの時刻（ST41）を、本物の PostgreSQL に対して確かめる。
//!
//! Task 1 は門と錠と CHECK（DB の側にあるので、どの接続でも同じに拒む）。
//! `core.event` / `core.event_version` を直に書く。取り込み口を通る試験は後続の Task が足す。
#![allow(clippy::unwrap_used)]

use crate::testdb;
use sqlx::PgPool;
use uuid::Uuid;

const START: &str = "2026-09-08T02:00:00Z";
const END: &str = "2026-09-08T03:00:00Z";
const OTHER_END: &str = "2026-09-08T05:00:00Z";

/// 記録を終わりつきで置く。`end` は（時刻の RFC3339, ずれ(分), タイムゾーン識別子）。
/// 内容の鍵は試験ごとに固有。原文と解析済みは空でない（消去の形と区別するため）。
pub async fn put_with_end(
    pool: &PgPool,
    user: Uuid,
    source: &str,
    at: &str,
    end: Option<(&str, i32, &str)>,
    external_id: Option<&str>,
) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO core.event
           (id, user_id, logical_source, device_id, origin, event_time,
            tz_offset_min, tz_id, end_time, end_tz_offset_min, end_tz_id,
            external_id, schema_version, content_hash, raw, payload)
         VALUES ($1,$2,$3,'test','collected',$4::timestamptz,540,'Asia/Tokyo',
                 $5::timestamptz,$6,$7,$8,1,$9,'{\"v\":1}','{\"v\":1}')",
    )
    .bind(id)
    .bind(user)
    .bind(source)
    .bind(at)
    .bind(end.map(|e| e.0))
    .bind(end.map(|e| e.1))
    .bind(end.map(|e| e.2))
    .bind(external_id)
    .bind(Uuid::new_v4().to_string())
    .execute(pool)
    .await
    .unwrap();
    id
}

/// 文を 1 つのまとまり（トランザクション）で撃つ。拒まれたら（SQLSTATE, 本文）を返す。
/// 門は COMMIT の瞬間に見るので、`commit` の失敗も拾う。
async fn run(pool: &PgPool, sql: &str) -> Option<(String, String)> {
    let mut tx = pool.begin().await.unwrap();
    let result = match sqlx::raw_sql(sql).execute(&mut *tx).await {
        Ok(_) => tx.commit().await,
        Err(e) => Err(e),
    };
    result.err().map(|e| {
        let db = e.as_database_error().expect("DB の側の拒否");
        (
            db.code().map(|c| c.to_string()).unwrap_or_default(),
            db.message().to_string(),
        )
    })
}

/// 更新前の版の履歴を 1 行書く SQL。`end` は履歴に載せる終わり（3 列）。
fn version_sql(
    event: Uuid,
    user: Uuid,
    source: &str,
    version_no: i32,
    end: Option<(&str, i32, &str)>,
) -> String {
    let (t, off, tz) = match end {
        Some((t, off, tz)) => (format!("'{t}'"), off.to_string(), format!("'{tz}'")),
        None => ("NULL".into(), "NULL".into(), "NULL".into()),
    };
    format!(
        "INSERT INTO core.event_version
           (event_id, user_id, logical_source, version_no, event_time, content_hash, raw, payload,
            end_time, end_tz_offset_min, end_tz_id)
         SELECT id, '{user}', '{source}', {version_no}, event_time, content_hash, raw, payload,
                {t}, {off}, {tz}
           FROM core.event WHERE id = '{event}';"
    )
}

async fn end_of(pool: &PgPool, id: Uuid) -> (Option<String>, Option<i32>, Option<String>) {
    sqlx::query_as(
        "SELECT to_char(end_time AT TIME ZONE 'UTC','YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),
                end_tz_offset_min, end_tz_id FROM core.event WHERE id = $1",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .unwrap()
}

fn tokyo(t: &str) -> (&str, i32, &str) {
    (t, 540, "Asia/Tokyo")
}

#[tokio::test]
async fn st41_gate_end_only_rewrite_is_rejected() {
    // Scenario: 収集した記録の終わりの時刻だけを履歴なしで変えると拒まれる
    let pool = testdb::pool().await;
    let user = testdb::user();
    let source = testdb::source_of_kind(&pool, "st41-gate-end", "none").await;
    let id = put_with_end(&pool, user, &source, START, Some(tokyo(END)), None).await;

    // 別の時刻にする
    let (code, msg) = run(
        &pool,
        &format!("UPDATE core.event SET end_time = '{OTHER_END}' WHERE id = '{id}';"),
    )
    .await
    .expect("履歴なしで終わりを変える書き換えが通った");
    assert_eq!(code, "P0001");
    assert!(msg.contains("更新前の版"), "門の本文: {msg}");

    // 3 列とも NULL にする（CHECK ではなく門で落ちる）
    let (code, msg) = run(
        &pool,
        &format!(
            "UPDATE core.event SET end_time = NULL, end_tz_offset_min = NULL, end_tz_id = NULL
              WHERE id = '{id}';"
        ),
    )
    .await
    .expect("履歴なしで終わりを消す書き換えが通った");
    assert_eq!(code, "P0001", "CHECK でなく門で落ちる: {msg}");
    assert!(msg.contains("更新前の版"), "門の本文: {msg}");

    let (t, off, tz) = end_of(&pool, id).await;
    assert_eq!(t.as_deref(), Some(END));
    assert_eq!((off, tz.as_deref()), (Some(540), Some("Asia/Tokyo")));
}

#[tokio::test]
async fn st41_gate_version_with_only_end_different_is_rejected() {
    // Scenario: 終わりだけが違う履歴を書いても、終わりの書き換えは通らない
    let pool = testdb::pool().await;
    let user = testdb::user();
    let source = testdb::source_of_kind(&pool, "st41-gate-ver", "none").await;
    let id = put_with_end(&pool, user, &source, START, Some(tokyo(END)), None).await;

    // 履歴は原文・内容・始まり・鍵が更新前と同じで、終わりだけが更新前と違う
    let version = version_sql(id, user, &source, 1, Some(tokyo(OTHER_END)));
    let (code, msg) = run(
        &pool,
        &format!(
            "{version}
             UPDATE core.event SET end_time = '{OTHER_END}' WHERE id = '{id}';"
        ),
    )
    .await
    .expect("終わりだけ違う履歴で終わりの書き換えが通った");
    assert_eq!(code, "P0001");
    assert!(msg.contains("更新前の版"), "門の本文: {msg}");

    assert_eq!(end_of(&pool, id).await.0.as_deref(), Some(END));
}

#[tokio::test]
async fn st41_gate_erasure_cannot_move_end() {
    // Scenario: 本文の消去で終わりの時刻を変えると拒まれる
    let pool = testdb::pool().await;
    let user = testdb::user();
    let source = testdb::source_of_kind(&pool, "st41-gate-erase", "none").await;
    let id = put_with_end(&pool, user, &source, START, Some(tokyo(END)), None).await;

    let ledger = format!(
        "INSERT INTO core.erasure_ledger (event_id, user_id, logical_source, scope, erased_by)
         VALUES ('{id}','{user}','{source}','event','test');"
    );
    let (code, _) = run(
        &pool,
        &format!(
            "{ledger}
             UPDATE core.event SET raw = '', payload = '{{}}', end_time = '{OTHER_END}'
              WHERE id = '{id}';"
        ),
    )
    .await
    .expect("消去の顔で終わりを動かす書き換えが通った");
    assert_eq!(code, "P0001");

    let (raw, payload): (String, serde_json::Value) =
        sqlx::query_as("SELECT raw, payload FROM core.event WHERE id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(raw, "{\"v\":1}", "本文が残っている");
    assert_eq!(payload, serde_json::json!({"v": 1}));
    assert_eq!(end_of(&pool, id).await.0.as_deref(), Some(END));
    let ledgered: i64 =
        sqlx::query_scalar("SELECT count(*) FROM core.erasure_ledger WHERE event_id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(ledgered, 0, "拒まれたので台帳の行も残らない");
}

#[tokio::test]
async fn st41_gate_end_tz_can_be_corrected() {
    // Scenario: 収集した記録の終わりのずれとタイムゾーンは直せる
    let pool = testdb::pool().await;
    let user = testdb::user();
    let source = testdb::source_of_kind(&pool, "st41-gate-tz", "none").await;
    let id = put_with_end(&pool, user, &source, START, Some(tokyo(END)), None).await;

    let rejected = run(
        &pool,
        &format!(
            "UPDATE core.event SET end_tz_offset_min = 0, end_tz_id = 'UTC' WHERE id = '{id}';"
        ),
    )
    .await;
    assert!(
        rejected.is_none(),
        "履歴なしのずれ・TZ の訂正が拒まれた: {rejected:?}"
    );

    let (t, off, tz) = end_of(&pool, id).await;
    assert_eq!(t.as_deref(), Some(END), "終わりの時刻そのものは動かない");
    assert_eq!((off, tz.as_deref()), (Some(0), Some("UTC")));
}

#[tokio::test]
async fn st41_gate_version_end_is_frozen() {
    // Scenario: 履歴の前の版の終わりは書き換えられない
    let pool = testdb::pool().await;
    let user = testdb::user();
    let source = testdb::source_of_kind(&pool, "st41-gate-vfrozen", "none").await;
    let id = put_with_end(&pool, user, &source, START, Some(tokyo(END)), None).await;
    let wrote = run(&pool, &version_sql(id, user, &source, 1, Some(tokyo(END)))).await;
    assert!(wrote.is_none(), "終わりつきの履歴が書けない: {wrote:?}");

    for set in [
        format!("end_time = '{OTHER_END}'"),
        "end_tz_offset_min = 0".to_string(),
        "end_tz_id = 'UTC'".to_string(),
    ] {
        let (code, msg) = run(
            &pool,
            &format!("UPDATE core.event_version SET {set} WHERE event_id = '{id}';"),
        )
        .await
        .unwrap_or_else(|| panic!("前の版の終わりが書き換えられた: {set}"));
        assert_eq!(code, "P0001", "{set}: {msg}");
        assert!(msg.contains("履歴は追記のみ"), "{set}: {msg}");
    }

    let kept: (Option<String>, Option<i32>, Option<String>) = sqlx::query_as(
        "SELECT to_char(end_time AT TIME ZONE 'UTC','YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),
                end_tz_offset_min, end_tz_id FROM core.event_version WHERE event_id = $1",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        kept,
        (Some(END.into()), Some(540), Some("Asia/Tokyo".into()))
    );
}

#[tokio::test]
async fn st41_gate_check_rejects_partial_end_in_db() {
    // Scenario: 取り込み口を通さない書き込みでも、終わりの一部だけ・範囲外のずれ・空の識別子は入らない
    let pool = testdb::pool().await;
    let user = testdb::user();
    let source = testdb::source_of_kind(&pool, "st41-gate-check", "none").await;
    let parent = put_with_end(&pool, user, &source, START, None, None).await;

    // (end_time, end_tz_offset_min, end_tz_id)。どれも CHECK で落ちる
    let bad = [
        ("'2026-09-08T03:00:00Z'", "NULL", "NULL"),
        ("NULL", "540", "NULL"),
        ("NULL", "NULL", "'Asia/Tokyo'"),
        ("'2026-09-08T03:00:00Z'", "540", "NULL"),
        ("'2026-09-08T03:00:00Z'", "1440", "'Asia/Tokyo'"),
        ("'2026-09-08T03:00:00Z'", "-1440", "'Asia/Tokyo'"),
        ("'2026-09-08T03:00:00Z'", "540", "''"),
    ];
    for (t, off, tz) in bad {
        let event = format!(
            "INSERT INTO core.event
               (id, user_id, logical_source, device_id, origin, event_time, tz_offset_min, tz_id,
                end_time, end_tz_offset_min, end_tz_id,
                schema_version, content_hash, raw, payload)
             VALUES ('{}','{user}','{source}','test','collected','{START}',540,'Asia/Tokyo',
                     {t},{off},{tz},1,'{}','{{}}','{{}}');",
            Uuid::new_v4(),
            Uuid::new_v4()
        );
        let version = format!(
            "INSERT INTO core.event_version
               (event_id, user_id, logical_source, version_no, event_time, content_hash, raw, payload,
                end_time, end_tz_offset_min, end_tz_id)
             VALUES ('{parent}','{user}','{source}',1,'{START}','h','{{}}','{{}}',{t},{off},{tz});"
        );
        for (table, sql) in [("core.event", event), ("core.event_version", version)] {
            let (code, msg) = run(&pool, &sql)
                .await
                .unwrap_or_else(|| panic!("{table} に入った: {t} / {off} / {tz}"));
            assert_eq!(code, "23514", "{table}: {t} / {off} / {tz}: {msg}");
        }
    }

    // 端の値（±1439）は入る
    let ok = put_with_end(
        &pool,
        user,
        &source,
        START,
        Some(("2026-09-08T03:00:00Z", 1439, "X")),
        None,
    )
    .await;
    assert_eq!(end_of(&pool, ok).await.1, Some(1439));

    // 移行を 2 回当てても落ちない（CHECK の重複も作らない）
    let (name, sql) = crate::MIGRATIONS
        .iter()
        .find(|(n, _)| n.contains("event_end_time"))
        .expect("終わりの時刻の移行が MIGRATIONS にある");
    for round in 1..=2 {
        let again = sqlx::raw_sql(sql).execute(&pool).await;
        assert!(
            again.is_ok(),
            "{name} の {round} 回目の当てが落ちた: {again:?}"
        );
    }
    let checks: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pg_constraint
          WHERE conrelid IN ('core.event'::regclass, 'core.event_version'::regclass)
            AND conname LIKE '%\\_end\\_%'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(checks, 4, "CHECK は表ごとに 2 つ");
}
