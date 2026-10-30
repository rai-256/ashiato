// SPDX-License-Identifier: AGPL-3.0-only
//! 書き出しが送信契約の版の列を持ち、説明書がそれを説明すること（ST50 / design D20）。
//!
//! 配置と読み方は `export_excluded_tests` の `World` と `export_file_tests` の `export_once` / `scan` / `text` に合わせる。
#![allow(clippy::unwrap_used)]

use serde_json::json;

use crate::export_excluded_tests::World;
use crate::export_file_tests::{export_once, scan, text};

const MEANING: &str =
    "収集アプリが記録を積んだ時の送信契約の版（スキーマ版とは別の番号。収集アプリ以外が書いた記録は空）";

// Scenario: 書き出しの記録と前の版の行は契約の版を持つ
#[tokio::test]
async fn st50_export_contract_version() {
    let w = World::new().await;
    let id = uuid::Uuid::new_v4();
    // 契約の版 1 で入り、版 2 の外部の更新で動いた形（本表は 2、前の版は 1）
    sqlx::query(
        "INSERT INTO core.event
           (id, user_id, logical_source, device_id, origin, event_time, tz_offset_min, tz_id,
            schema_version, content_hash, raw, payload, contract_version)
         VALUES ($1,$2,'c02-window','pc','collected',$3::timestamptz,540,'Asia/Tokyo',1,$4,'x','{}',2)",
    )
    .bind(id)
    .bind(w.user)
    .bind(w.at("05:00"))
    .bind(uuid::Uuid::new_v4().to_string())
    .execute(&w.pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO core.event_version
           (event_id, user_id, logical_source, version_no, event_time, content_hash, raw, payload, contract_version)
         SELECT id, user_id, logical_source, 1, event_time, $2, 'old', '{}', 1 FROM core.event WHERE id = $1",
    )
    .bind(id)
    .bind(uuid::Uuid::new_v4().to_string())
    .execute(&w.pool)
    .await
    .unwrap();
    let plain = w
        .put("c01-location", &w.at("06:00"), json!({"t": "y"}), "y")
        .await;
    let (_tmp, path) = export_once(&w.pool).await;
    let event = &scan(&path, "events.jsonl", &id.to_string())[0];
    assert_eq!(event["contract_version"], 2);
    let versions = scan(&path, "event_versions.jsonl", &id.to_string());
    assert!(versions.iter().any(|r| r["contract_version"] == 1));
    assert!(versions
        .iter()
        .all(|r| r.as_object().unwrap().contains_key("contract_version")));
    let blank = &scan(&path, "events.jsonl", &plain.to_string())[0];
    assert!(blank.as_object().unwrap().contains_key("contract_version"));
    assert!(blank["contract_version"].is_null());
}

// Scenario: 説明書は契約の版の列を説明する
#[tokio::test]
async fn st50_export_manual_describes() {
    let w = World::new().await;
    let id = w
        .put("c02-window", &w.at("05:00"), json!({"t": "x"}), "x")
        .await;
    w.put_version(id, 1, &w.at("05:00"), json!({"t": "old"}))
        .await;
    let (_tmp, path) = export_once(&w.pool).await;
    let readme = text(&path, "README.txt");
    let line = format!("contract_version — {MEANING}");
    for name in ["events.jsonl", "event_versions.jsonl"] {
        let section = readme
            .split(&format!("\n  {name}\n"))
            .nth(1)
            .unwrap_or_else(|| panic!("{name} の節が無い"));
        let section = section.split("\n\n").next().unwrap();
        assert!(section.contains(&line), "{name}");
        assert!(section.contains("送信契約の版") && section.contains("スキーマ版とは別"));
    }
}
