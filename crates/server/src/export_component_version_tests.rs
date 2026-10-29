// SPDX-License-Identifier: AGPL-3.0-only
//! 書き出しの版の列と説明（ST49 / design D17・D4）。**書き出した zip を開き**、行の鍵と値を見る
//! （配置と読み方は `export_file_tests.rs` に合わせる）。確かめるのは**そのテストの行だけ**。
#![allow(clippy::unwrap_used)]

use std::path::{Path, PathBuf};

use serde_json::Value;
use uuid::Uuid;

use crate::export::{Class, CATALOG};
use crate::export_excluded_tests::World;
use crate::export_file_tests::{export_once, scan, text};

const VERSIONS: [&str; 3] = ["collector_version", "web_version", "server_version"];

/// 書き出した zip。落とすときに置き場（`export_tests::tmp_dir("file")` の `ashiato-export-file-*`）ごと消す
/// （一時フォルダに残さない。R7）。
struct Exported(PathBuf);

impl std::ops::Deref for Exported {
    type Target = Path;
    fn deref(&self) -> &Path {
        &self.0
    }
}

impl Drop for Exported {
    fn drop(&mut self) {
        let Some(dir) = self.0.parent() else { return };
        // 置き場の名前を確かめてから消す（別の場所を消さない）
        let ours = dir
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with("ashiato-export-file-"));
        // tmp-ok: 作らない。消す前に置き場が一時フォルダの下かを確かめるために読むだけ
        if ours && dir.starts_with(std::env::temp_dir()) {
            let _ = std::fs::remove_dir_all(dir);
        }
    }
}

/// 新しい置き場に 1 回書き出す（`export_file_tests::export_once`）。置き場は返した値を落とすときに消える。
async fn export_here(pool: &sqlx::PgPool) -> Exported {
    Exported(export_once(pool).await)
}

fn columns_of(file: &str) -> &'static [(&'static str, &'static str)] {
    CATALOG
        .iter()
        .find_map(|(_, c)| match c {
            Class::Include {
                file: f, columns, ..
            } if *f == file => Some(*columns),
            _ => None,
        })
        .unwrap()
}

fn meaning(file: &str, col: &str) -> &'static str {
    columns_of(file)
        .iter()
        .find(|(c, _)| *c == col)
        .unwrap_or_else(|| panic!("{file} に {col} の列が無い"))
        .1
}

async fn put_event(w: &World, hhmm: &str, collector: Option<&str>, server: Option<&str>) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO core.event
           (id, user_id, logical_source, device_id, origin, event_time, tz_offset_min, tz_id,
            schema_version, content_hash, raw, payload, collector_version, server_version)
         VALUES ($1,$2,'c02-window','pc','collected',$3::timestamptz,540,'Asia/Tokyo',1,$4,'x','{}',$5,$6)",
    )
    .bind(id)
    .bind(w.user)
    .bind(w.at(hhmm))
    .bind(Uuid::new_v4().to_string())
    .bind(collector)
    .bind(server)
    .execute(&w.pool)
    .await
    .unwrap();
    id
}

fn str_of(row: &Value, key: &str) -> Option<String> {
    row.get(key)
        .unwrap_or_else(|| panic!("行に {key} の鍵が無い"))
        .as_str()
        .map(str::to_string)
}

// Scenario: 書き出しの記録の行は、3 つの版を持つ
#[tokio::test]
async fn st49_export_event_rows_have_versions() {
    let w = World::new().await;
    let with = put_event(&w, "05:00", Some("0.1.0+aaaa"), Some("0.2.0+bbbb")).await;
    let without = put_event(&w, "05:01", None, None).await;
    let path = export_here(&w.pool).await;
    let a = &scan(&path, "events.jsonl", &with.to_string())[0];
    assert_eq!(
        str_of(a, "collector_version").as_deref(),
        Some("0.1.0+aaaa")
    );
    assert_eq!(str_of(a, "server_version").as_deref(), Some("0.2.0+bbbb"));
    assert_eq!(str_of(a, "web_version"), None);
    let b = &scan(&path, "events.jsonl", &without.to_string())[0];
    for k in VERSIONS {
        assert_eq!(str_of(b, k), None, "{k}");
    }
}

// Scenario: 書き出しの前の版の行は、その版の版を持つ
#[tokio::test]
async fn st49_export_version_rows_have_versions() {
    let w = World::new().await;
    let id = w
        .put("c02-window", &w.at("05:00"), serde_json::json!({}), "x")
        .await;
    let v = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO core.event_version
           (id, event_id, user_id, logical_source, version_no, event_time, content_hash, raw, payload,
            collector_version, server_version)
         SELECT $1, e.id, e.user_id, e.logical_source, 1, e.event_time, $2, '{}', '{}',
                '0.1.0+old', '0.2.0+olds'
           FROM core.event e WHERE e.id = $3",
    )
    .bind(v)
    .bind(Uuid::new_v4().to_string())
    .bind(id)
    .execute(&w.pool)
    .await
    .unwrap();
    let path = export_here(&w.pool).await;
    let r = &scan(&path, "event_versions.jsonl", &v.to_string())[0];
    assert_eq!(str_of(r, "collector_version").as_deref(), Some("0.1.0+old"));
    assert_eq!(str_of(r, "server_version").as_deref(), Some("0.2.0+olds"));
    assert_eq!(str_of(r, "web_version"), None);
}

// Scenario: 書き出しの破棄の報告と画面の操作の台帳は、版の列を持つ
#[tokio::test]
async fn st49_export_ledger_rows_have_versions() {
    let w = World::new().await;
    let report = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO core.drop_report
           (id, user_id, logical_source, device_id, reason, count, created_at, content_hash, raw,
            collector_version, server_version)
         VALUES ($1, $2, 'c01-clock', 'dev', 'age', 1, now(), $3, '{}', '0.1.0+drop', '0.2.0+drops')",
    )
    .bind(report)
    .bind(w.user)
    .bind(Uuid::new_v4().to_string())
    .execute(&w.pool)
    .await
    .unwrap();
    let reminder: i64 = sqlx::query_scalar(
        "INSERT INTO core.feeling_reminder (user_id, at, server_version) VALUES ($1, '21:00', '0.2.0+rem') RETURNING id",
    )
    .bind(w.user)
    .fetch_one(&w.pool)
    .await
    .unwrap();
    let criteria: i64 = sqlx::query_scalar(
        "INSERT INTO core.stay_criteria (user_id, radius_m, min_minutes, gap_minutes, sources, server_version)
         VALUES ($1, 100, 10, 10, ARRAY['c01-location'], '0.2.0+crit') RETURNING id",
    )
    .bind(w.user)
    .fetch_one(&w.pool)
    .await
    .unwrap();
    let kind = Uuid::new_v4();
    sqlx::query("INSERT INTO core.attribute_kind (id, user_id) VALUES ($1, $2)")
        .bind(kind)
        .bind(w.user)
        .execute(&w.pool)
        .await
        .unwrap();
    let name: i64 = sqlx::query_scalar(
        "INSERT INTO core.attribute_kind_name (kind_id, user_id, name, server_version)
         VALUES ($1, $2, '住所', '0.2.0+name') RETURNING id",
    )
    .bind(kind)
    .bind(w.user)
    .fetch_one(&w.pool)
    .await
    .unwrap();

    let path = export_here(&w.pool).await;
    // 整数の鍵は文字列の中の数字とも当たるので、全行を読んで `id` と利用者で絞る。
    let pick = |file: &str, id: String| -> Value {
        scan(&path, file, "")
            .into_iter()
            .find(|r| {
                r["user_id"] == w.user.to_string() && (r["id"].to_string().trim_matches('"') == id)
            })
            .unwrap_or_else(|| panic!("{file} に行が出ない"))
    };
    let d = pick("drop_reports.jsonl", report.to_string());
    assert_eq!(
        str_of(&d, "collector_version").as_deref(),
        Some("0.1.0+drop")
    );
    assert_eq!(str_of(&d, "server_version").as_deref(), Some("0.2.0+drops"));
    for (file, id, want) in [
        ("feeling_reminders.jsonl", reminder, "0.2.0+rem"),
        ("stay_criteria.jsonl", criteria, "0.2.0+crit"),
        ("attribute_kind_names.jsonl", name, "0.2.0+name"),
    ] {
        let r = pick(file, id.to_string());
        assert_eq!(
            str_of(&r, "server_version").as_deref(),
            Some(want),
            "{file}"
        );
        assert!(r.get("collector_version").is_none(), "{file}");
    }
}

// Scenario: 書き出しの説明書は、申告の版と S-01 が入れた版を書き分ける
#[tokio::test]
async fn st49_export_doc_distinguishes_declared_and_server() {
    let w = World::new().await;
    let path = export_here(&w.pool).await;
    let readme = text(&path, "README.txt");
    for file in ["events.jsonl", "event_versions.jsonl"] {
        assert!(meaning(file, "collector_version").contains("S-01 は確かめていない"));
        assert!(meaning(file, "web_version").contains("S-01 は確かめていない"));
        assert!(meaning(file, "server_version").contains("S-01 が自分で入れた"));
    }
    assert!(meaning("drop_reports.jsonl", "collector_version").contains("S-01 は確かめていない"));
    for file in [
        "drop_reports.jsonl",
        "feeling_reminders.jsonl",
        "stay_criteria.jsonl",
        "attribute_kind_names.jsonl",
    ] {
        assert!(
            meaning(file, "server_version").contains("S-01 が自分で入れた"),
            "{file}"
        );
    }
    // 説明書そのものにも同じ文が出る
    assert!(readme.contains(&format!(
        "collector_version — {}",
        meaning("events.jsonl", "collector_version")
    )));
    assert!(readme.contains(&format!(
        "server_version — {}",
        meaning("events.jsonl", "server_version")
    )));
}

// Scenario: 書き出しの説明書は、版を今の中身を作った部品の版と説明する
#[tokio::test]
async fn st49_export_doc_says_current_content() {
    let w = World::new().await;
    let path = export_here(&w.pool).await;
    let readme = text(&path, "README.txt");
    for col in VERSIONS {
        assert!(
            meaning("events.jsonl", col).contains("今の中身"),
            "events {col}"
        );
        assert!(
            meaning("event_versions.jsonl", col).contains("その版の中身"),
            "versions {col}"
        );
    }
    let mut six: Vec<&str> = VERSIONS
        .iter()
        .map(|c| meaning("events.jsonl", c))
        .collect();
    six.extend(VERSIONS.iter().map(|c| meaning("event_versions.jsonl", c)));
    for m in six {
        assert!(
            !m.contains("記録を作った") && !m.contains("記録を組んだ"),
            "{m}"
        );
        assert!(readme.contains(m), "説明書に無い: {m}");
    }
}

// Scenario: 版の列を足しても、書き出しの今の列は残る
#[tokio::test]
async fn st49_export_existing_columns_are_kept() {
    // 足す前の一覧（逐語）。
    const BEFORE: [(&str, &[&str]); 6] = [
        (
            "events.jsonl",
            &[
                "id",
                "user_id",
                "logical_source",
                "external_id",
                "device_id",
                "origin",
                "event_time",
                "ingest_time",
                "tz_offset_min",
                "tz_id",
                "end_time",
                "end_tz_offset_min",
                "end_tz_id",
                "schema_version",
                "unit_system",
                "crs",
                "sensitivity",
                "content_hash",
                "raw",
                "payload",
                "deleted_at",
                "deleted_by",
                "source_updated_at",
                "external_ref",
            ],
        ),
        (
            "event_versions.jsonl",
            &[
                "id",
                "event_id",
                "user_id",
                "logical_source",
                "version_no",
                "event_time",
                "content_hash",
                "raw",
                "payload",
                "source_updated_at",
                "external_ref",
                "superseded_at",
                "tz_offset_min",
                "tz_id",
                "end_time",
                "end_tz_offset_min",
                "end_tz_id",
                "schema_version",
                "unit_system",
                "crs",
            ],
        ),
        (
            "drop_reports.jsonl",
            &[
                "id",
                "user_id",
                "logical_source",
                "device_id",
                "reason",
                "range_start",
                "range_end",
                "count",
                "created_at",
                "received_at",
                "content_hash",
                "raw",
            ],
        ),
        (
            "feeling_reminders.jsonl",
            &["id", "user_id", "at", "created_at"],
        ),
        (
            "stay_criteria.jsonl",
            &[
                "id",
                "user_id",
                "radius_m",
                "min_minutes",
                "gap_minutes",
                "sources",
                "created_at",
            ],
        ),
        (
            "attribute_kind_names.jsonl",
            &["id", "kind_id", "user_id", "name", "created_at"],
        ),
    ];
    for (file, before) in BEFORE {
        let after: Vec<&str> = columns_of(file).iter().map(|(c, _)| *c).collect();
        for c in before {
            assert!(after.contains(c), "{file}: {c} が消えた");
        }
        let added: Vec<&str> = after
            .iter()
            .copied()
            .filter(|c| !before.contains(c))
            .collect();
        assert!(
            !added.is_empty() && added.iter().all(|c| VERSIONS.contains(c)),
            "{file}: 増えたのが版の列だけでない: {added:?}"
        );
    }
}
