// SPDX-License-Identifier: AGPL-3.0-only
//! 写真と画像と動画の書き出しの分類（ST09 tasks 9.1 / design D16 / D18）と、滞在を消したときの写真（9.2 / D23）を、
//! 本物の PostgreSQL と本物の置き場（一時ディレクトリ）で確かめる。
//! 書き出しを走らせるテストは `EXPORT_SERIAL` を取り、記録と消す滞在は `unique_day()` の日に置き、その利用者の行だけを見る。
#![allow(clippy::unwrap_used)]

use std::collections::BTreeSet;

use serde_json::{json, Value};
use uuid::Uuid;

use crate::export;
use crate::export_excluded_tests::World;
use crate::export_file_tests::{export_once, present, scan, text};
use crate::testdb;

fn latlon() -> Value {
    json!({"lat": 35.0, "lon": 139.0})
}

/// 01:00〜02:00 の滞在を消した世界に、`source` の記録を `at` に置いて書き出し、記録のファイルに出たかを返す。
async fn exported_after_erasing(source: &str, at: &str, payload: Value) -> bool {
    let w = World::new().await;
    w.erased_stay(&w.at("01:00"), &w.at("02:00"), &[]).await;
    let id = w.put(source, &w.at(at), payload, "{}").await;
    let path = export_once(&w.pool).await;
    present(&path, "events.jsonl", &[id]).contains(&id)
}

// Scenario: 消した滞在の時間に撮った撮影位置つきのスクリーンショットと画像は書き出しに出ない
#[tokio::test]
async fn st09_exp_screenshot_and_image_with_position_inside_the_erased_time_are_not_exported() {
    assert!(!exported_after_erasing("c01-screenshot", "01:30", latlon()).await);
    assert!(!exported_after_erasing("c01-image", "01:30", latlon()).await);
}

// Scenario: 消した時間の前に撮り始め、長さで重なる動画は書き出しに出ない
#[tokio::test]
async fn st09_exp_video_started_before_and_overlapping_by_duration_is_not_exported() {
    let payload = json!({"lat": 35.0, "lon": 139.0, "duration_ms": 600_000});
    assert!(!exported_after_erasing("c01-video", "00:55", payload).await);
}

// Scenario: 消した時間の後に撮った写真は書き出しに出る
#[tokio::test]
async fn st09_exp_photo_after_the_erased_time_is_exported() {
    assert!(exported_after_erasing("c01-photo", "02:01", latlon()).await);
}

// Scenario: 生存信号の接続の有無は書き出しに出る
#[tokio::test]
async fn st09_exp_heartbeat_network_column_is_exported() {
    let w = World::new().await;
    let s = testdb::source_media(&w.pool, "exp-net").await;
    sqlx::query(
        "INSERT INTO core.heartbeat
           (id, user_id, logical_source, device_id, emitted_at, capturable, blockers,
            attempts, successes, content_hash, raw, network)
         VALUES ($1,$2,$3,'phone',$4::timestamptz,true,'{}',1,1,$5,'{}',false)",
    )
    .bind(Uuid::new_v4())
    .bind(w.user)
    .bind(&s)
    .bind(w.at("03:00"))
    .bind(Uuid::new_v4().to_string())
    .execute(&w.pool)
    .await
    .unwrap();
    let path = export_once(&w.pool).await;
    let rows = scan(&path, "heartbeats.jsonl", &w.user.to_string());
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["network"], json!(false));
}

/// 原本の受け取りと原本の欠けが 1 件ずつある世界を書き出す。
async fn export_with_original_and_missing() -> std::path::PathBuf {
    let w = World::new().await;
    sqlx::query(
        "INSERT INTO core.media_original (user_id, sha256, logical_source, size_bytes)
         VALUES ($1, $2, 'c01-photo', 3)",
    )
    .bind(w.user)
    .bind(Uuid::new_v4().simple().to_string())
    .execute(&w.pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO core.media_missing (id, user_id, logical_source, sha256, reason, noticed_at)
         VALUES ($1, $2, 'c01-photo', $3, 'vanished', now())",
    )
    .bind(Uuid::new_v4())
    .bind(w.user)
    .bind(Uuid::new_v4().simple().to_string())
    .execute(&w.pool)
    .await
    .unwrap();
    export_once(&w.pool).await
}

// Scenario: 原本の台帳と原本の欠けと文字抽出の表は書き出しに入らない
#[tokio::test]
async fn st09_exp_original_ledger_missing_and_text_extraction_are_not_exported() {
    let path = export_with_original_and_missing().await;
    let z = zip::ZipArchive::new(std::fs::File::open(&path).unwrap()).unwrap();
    let names: BTreeSet<String> = z.file_names().map(String::from).collect();
    for n in &names {
        for banned in ["original", "missing", "extraction"] {
            assert!(!n.contains(banned), "入らないはずのファイル: {n}");
        }
    }
}

// Scenario: 説明書の入らないものに原本の台帳と欠けと文字抽出が挙がる
#[tokio::test]
async fn st09_exp_readme_names_original_ledger_missing_and_text_extraction() {
    let path = export_with_original_and_missing().await;
    let readme = text(&path, "README.txt");
    for table in ["media_original", "media_missing", "text_extraction"] {
        assert!(
            readme.contains(&format!("表 {table}")),
            "説明書の入らないものに無い: {table}"
        );
    }
    assert!(readme.contains("D-02 の原本"));
}

// Scenario: ST09 が足したソースと表と列はすべて分類されている
#[tokio::test]
async fn st09_exp_sources_tables_and_columns_are_all_classified() {
    let (fresh, drop) = crate::stay_tests::fresh_db().await;
    let got = async {
        for (label, sql) in crate::MIGRATIONS {
            sqlx::raw_sql(sql)
                .execute(&fresh)
                .await
                .map_err(|e| format!("{label}: {e}"))?;
        }
        let sources: Vec<String> =
            sqlx::query_scalar("SELECT logical_source FROM core.source ORDER BY 1")
                .fetch_all(&fresh)
                .await
                .map_err(|e| e.to_string())?;
        let inv = export::read_inventory(&fresh)
            .await
            .map_err(|e| e.to_string())?;
        Ok::<_, String>((sources, inv))
    }
    .await;
    drop.await;
    let (sources, inv) = got.unwrap();

    let points: Vec<&str> = export::SPAN_RULES.iter().map(|(s, _)| *s).collect();
    for s in ["c01-screenshot", "c01-image", "c01-video"] {
        assert!(sources.iter().any(|x| x == s), "登録簿に無い: {s}");
        assert!(export::listed(export::COORD_SOURCES, s), "{s} の座標の分類");
        assert!(!export::listed(&export::NO_COORD_SOURCES, s));
        assert!(export::listed(&points, s), "{s} の幅の分類");
    }
    for t in ["media_original", "media_missing", "text_extraction"] {
        assert!(inv.tables.iter().any(|x| x == t), "表が無い: {t}");
        assert!(
            matches!(export::class_of(t), Some(export::Class::Exclude { .. })),
            "{t} は入れない分類"
        );
    }
    assert!(inv
        .columns
        .iter()
        .any(|(t, c)| t == "heartbeat" && c == "network"));
    assert!(
        matches!(
            export::class_of("heartbeat"),
            Some(export::Class::Include { columns, excluded_columns, .. })
                if columns.iter().any(|(c, _)| *c == "network") && !excluded_columns.contains(&"network")
        ),
        "network は出す列"
    );
    let bad = export::unclassified(&inv);
    assert!(bad.is_empty(), "分類の無いもの: {bad:?}");
}

// ---------------------------------------------------------------------------
// 9.2 滞在を消したときの写真（D23。`deletion.rs` は変えない）
// ---------------------------------------------------------------------------

mod deleting_a_stay {
    use super::*;
    use crate::deletion_tests::{app, erase, put_stay, t};
    use crate::media_tests::{app_with, put, record, sha_of, store, unique_bytes, Tmp};

    const STAY_START: &str = "2026-09-28T10:00:00+09:00";
    const STAY_END: &str = "2026-09-28T11:00:00+09:00";
    const SHOT_AT: &str = "2026-09-28T10:30:00+09:00";

    /// 記録の読み出し（削除の印の無い記録だけの見え方）に出るか、と削除の印。
    async fn readable(pool: &sqlx::PgPool, user: Uuid, sha: &str) -> bool {
        let n: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM core.event_live
              WHERE user_id = $1 AND external_id = $2 AND deleted_at IS NULL AND deleted_by IS NULL",
        )
        .bind(user)
        .bind(sha)
        .fetch_one(pool)
        .await
        .unwrap();
        n == 1
    }

    // Scenario: 滞在を消しても撮影位置つきの写真は読み出しに出る
    #[tokio::test]
    async fn st09_del_photo_with_position_stays_readable() {
        let tmp = Tmp::new();
        let app = app_with(tmp.made()).await;
        let user = testdb::user();
        let stay = put_stay(&app.pool, user, STAY_START, STAY_END).await;
        let sha = sha_of(&unique_bytes("del-photo"));
        store(
            &app,
            vec![record(
                user,
                "c01-photo",
                &sha,
                "a.jpg",
                &t(SHOT_AT).to_rfc3339(),
                json!({"lat": 35.68, "lon": 139.76}),
            )],
        )
        .await;
        erase(&app, stay, Some(user)).await.unwrap();
        assert!(readable(&app.pool, user, &sha).await);
    }

    // Scenario: 滞在を消しても動画とスクリーンショットの記録は読み出しに出る
    #[tokio::test]
    async fn st09_del_video_and_screenshot_stay_readable() {
        let app = app().await;
        let user = testdb::user();
        let stay = put_stay(&app.pool, user, STAY_START, STAY_END).await;
        let video = sha_of(&unique_bytes("del-video"));
        let shot = sha_of(&unique_bytes("del-shot"));
        let at = t(SHOT_AT).to_rfc3339();
        store(
            &app,
            vec![
                record(
                    user,
                    "c01-video",
                    &video,
                    "a.mp4",
                    &at,
                    json!({"lat": 35.68, "lon": 139.76, "duration_ms": 1000}),
                ),
                record(
                    user,
                    "c01-screenshot",
                    &shot,
                    "s.png",
                    &at,
                    json!({"lat": 35.68, "lon": 139.76}),
                ),
            ],
        )
        .await;
        erase(&app, stay, Some(user)).await.unwrap();
        assert!(readable(&app.pool, user, &video).await);
        assert!(readable(&app.pool, user, &shot).await);
    }

    fn files_under(dir: &std::path::Path) -> Vec<(std::path::PathBuf, Vec<u8>)> {
        let mut out = Vec::new();
        for e in std::fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                out.extend(files_under(&p));
            } else {
                let bytes = std::fs::read(&p).unwrap();
                out.push((p, bytes));
            }
        }
        out
    }

    // Scenario: 滞在を消しても D-02 の原本は残る
    #[tokio::test]
    async fn st09_del_d02_original_stays_with_the_same_bytes() {
        let tmp = Tmp::new();
        let app = app_with(tmp.made()).await;
        let user = testdb::user();
        let stay = put_stay(&app.pool, user, STAY_START, STAY_END).await;
        let bytes = unique_bytes("del-original");
        let sha =
            crate::media_tests::photo(&app, user, &bytes, "o.jpg", &t(SHOT_AT).to_rfc3339()).await;
        let (status, body) = put(&app, &sha, user, &bytes).await;
        assert!(status.is_success(), "{status} {body}");
        let before = files_under(&tmp.root());
        assert_eq!(before.len(), 1);

        erase(&app, stay, Some(user)).await.unwrap();

        let after = files_under(&tmp.root());
        assert_eq!(after, before);
        assert_eq!(after[0].1, bytes);
    }
}
