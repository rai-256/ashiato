// SPDX-License-Identifier: AGPL-3.0-only
//! 書き出しの更新の履歴・収集アプリの版の変化・生存信号の版（ST51 / design D11）。**書き出した zip を開き**、行と説明書を見る
//! （配置と読み方は `export_file_tests.rs` に合わせる）。試験ごとの database なので、読む行は自分が置いたものだけ。
#![allow(clippy::unwrap_used)]

use serde_json::Value;
use uuid::Uuid;

use crate::export::{self, Class, CATALOG, VIEW_FILES};
use crate::export_excluded_tests::World;
use crate::export_file_tests::{export_once, scan, text};

const COMMIT_A: &str = "0.1.0+aaaaaaa";
const COMMIT_B: &str = "0.1.0+bbbbbbb";

/// `c01-` で始まる名前の部品を登録簿へ置く（ビューは名前の前置きで C-01 を決める）。
async fn c01_source(pool: &sqlx::PgPool) -> String {
    let name = format!("c01-st51e-{}", Uuid::new_v4());
    sqlx::query(
        "INSERT INTO core.source
           (logical_source, display_name, expected_gap_sec, registered_at, external_id_kind)
         VALUES ($1, $1, 3600, '2000-01-01'::timestamptz, 'none')",
    )
    .bind(&name)
    .execute(pool)
    .await
    .unwrap();
    name
}

/// 生存信号 1 件を置く。`version` は列 `collector_version`。
async fn put_signal(w: &World, source: &str, device: &str, hhmm: &str, version: Option<&str>) {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO core.heartbeat
           (id, user_id, logical_source, device_id, emitted_at, received_at,
            capturable, blockers, attempts, successes, content_hash, raw, collector_version)
         VALUES ($1, $2, $3, $4, $5::timestamptz, $5::timestamptz, true, '{}', 1, 1, $6, '{}', $7)",
    )
    .bind(id)
    .bind(w.user)
    .bind(source)
    .bind(device)
    .bind(w.at(hhmm))
    .bind(id.to_string())
    .bind(version)
    .execute(&w.pool)
    .await
    .unwrap();
}

fn file_columns(file: &str) -> Vec<&'static str> {
    CATALOG
        .iter()
        .find_map(|(_, c)| match c {
            Class::Include {
                file: f, columns, ..
            } if *f == file => Some(*columns),
            _ => None,
        })
        .or_else(|| {
            VIEW_FILES
                .iter()
                .find(|v| v.file == file)
                .map(|v| v.columns)
        })
        .unwrap()
        .iter()
        .map(|(c, _)| *c)
        .collect()
}

// Scenario: 書き出しに更新の履歴の全行と全列が入る
#[tokio::test]
async fn st51_export_update_history_rows() {
    let w = World::new().await;
    sqlx::query(
        "INSERT INTO core.update_history (component, via, from_version, to_version, outcome, applied_versions)
         VALUES ('S-01', 'migrate', NULL, '0.1.0+aaaaaaa', 'applied', ARRAY['202610250000_update_history'])",
    )
    .execute(&w.pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO core.update_history
           (component, via, from_version, to_version, outcome, failed_version, sqlstate)
         VALUES ('S-01', 'startup', '0.1.0+aaaaaaa', '0.1.0+bbbbbbb', 'failed', '202610260000_next', '42P01')",
    )
    .execute(&w.pool)
    .await
    .unwrap();
    let (_tmp, path) = export_once(&w.pool).await;
    let rows = scan(&path, "update_history.jsonl", "");
    assert_eq!(rows.len(), 2);

    let db: Vec<(
        i64,
        String,
        String,
        String,
        Option<String>,
        String,
        String,
        Vec<String>,
        Option<String>,
        Option<String>,
    )> = sqlx::query_as(
        "SELECT seq, to_char(at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"'),
                    component, via, from_version, to_version, outcome, applied_versions,
                    failed_version, sqlstate
               FROM core.update_history ORDER BY seq",
    )
    .fetch_all(&w.pool)
    .await
    .unwrap();
    let want = file_columns("update_history.jsonl");
    for (row, d) in rows.iter().zip(&db) {
        let keys: Vec<&str> = row
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(keys.len(), want.len(), "全列が出る: {keys:?}");
        for c in &want {
            assert!(row.get(*c).is_some(), "{c} の鍵が無い");
        }
        assert_eq!(row["seq"], d.0);
        assert_eq!(row["at"], d.1.as_str());
        assert_eq!(row["component"], d.2.as_str());
        assert_eq!(row["via"], d.3.as_str());
        assert_eq!(row["from_version"].as_str().map(String::from), d.4);
        assert_eq!(row["to_version"], d.5.as_str());
        assert_eq!(row["outcome"], d.6.as_str());
        let applied: Vec<String> = row["applied_versions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();
        assert_eq!(applied, d.7);
        assert_eq!(row["failed_version"].as_str().map(String::from), d.8);
        assert_eq!(row["sqlstate"].as_str().map(String::from), d.9);
    }
    assert_eq!(rows[0]["outcome"], "applied");
    assert_eq!(rows[1]["outcome"], "failed");
    assert_eq!(rows[1]["sqlstate"], "42P01");
}

// Scenario: 書き出しに収集アプリの版の変化が入る
#[tokio::test]
async fn st51_export_version_changes() {
    let w = World::new().await;
    let src = c01_source(&w.pool).await;
    put_signal(&w, &src, "d1", "10:00", Some(COMMIT_A)).await;
    put_signal(&w, &src, "d1", "11:00", Some(COMMIT_B)).await;
    let (_tmp, path) = export_once(&w.pool).await;
    let rows = scan(
        &path,
        "collector_version_changes.jsonl",
        &w.user.to_string(),
    );
    assert_eq!(rows.len(), 1);
    let r = &rows[0];
    assert_eq!(r["from_version"], COMMIT_A);
    assert_eq!(r["to_version"], COMMIT_B);
    assert_eq!(r["device_id"], "d1");
    assert_eq!(r["component"], "C-01");
    assert_eq!(r["logical_source"], src.as_str());
    assert_eq!(
        r["emitted_at"],
        w.at("11:00").replace(":00Z", ":00.000000Z")
    );
    let keys: Vec<&str> = r.as_object().unwrap().keys().map(String::as_str).collect();
    assert_eq!(
        keys.len(),
        file_columns("collector_version_changes.jsonl").len()
    );
}

// Scenario: 書き出しの生存信号のファイルに版の列が入る
#[tokio::test]
async fn st51_export_heartbeat_version_column() {
    let w = World::new().await;
    let src = c01_source(&w.pool).await;
    put_signal(&w, &src, "d1", "10:00", Some(COMMIT_A)).await;
    put_signal(&w, &src, "d1", "10:30", None).await;
    let (_tmp, path) = export_once(&w.pool).await;
    let rows = scan(&path, "heartbeats.jsonl", &w.user.to_string());
    assert_eq!(rows.len(), 2);
    let with = rows
        .iter()
        .find(|r| r["emitted_at"] == w.at("10:00").replace(":00Z", ":00.000000Z"))
        .unwrap();
    assert_eq!(with["collector_version"], COMMIT_A);
    let without = rows
        .iter()
        .find(|r| r["emitted_at"] == w.at("10:30").replace(":00Z", ":00.000000Z"))
        .unwrap();
    assert_eq!(without["collector_version"], Value::Null);
    assert!(with.get("raw").is_none());
}

// Scenario: 説明書が更新の履歴と版の変化と生存信号の版を説明する
#[tokio::test]
async fn st51_export_readme_explains() {
    let w = World::new().await;
    let (_tmp, path) = export_once(&w.pool).await;
    let readme = text(&path, "README.txt");
    let section = |name: &str| -> String {
        let start = readme
            .find(&format!("\n  {name}\n"))
            .unwrap_or_else(|| panic!("{name} の節が無い"));
        let rest = &readme[start + 1..];
        let end = rest[name.len()..]
            .find("\n\n  ")
            .map_or(rest.len(), |i| i + name.len());
        rest[..end].to_string()
    };
    let history = section("update_history.jsonl");
    assert!(history.contains("戻すと消えることがある"), "{history}");
    assert!(history.contains("前の版が空なら記録の始まり"));
    let changes = section("collector_version_changes.jsonl");
    assert!(changes.contains("S-01 は確かめていない"), "{changes}");
    let heartbeats = section("heartbeats.jsonl");
    let line = heartbeats
        .lines()
        .find(|l| l.contains("collector_version"))
        .unwrap();
    assert!(line.contains("空なら申告なし"), "{line}");
}

// ビューの列は分類を求めない（D11）。ビューの列は `information_schema.columns` に出るが、分類のある表の列だけが突き合わせの対象。
#[tokio::test]
async fn st51_export_view_needs_no_class() {
    let w = World::new().await;
    let inv = export::read_inventory(&w.pool).await.unwrap();
    assert!(
        inv.columns
            .iter()
            .any(|(t, _)| t == "collector_version_change"),
        "ビューの列が一覧に出ていない前提が崩れた"
    );
    assert!(!inv.tables.iter().any(|t| t == "collector_version_change"));
    let missing = export::unclassified(&inv);
    assert!(missing.is_empty(), "{missing:?}");
    assert!(export::missing_view_columns(&inv).is_empty());
}
