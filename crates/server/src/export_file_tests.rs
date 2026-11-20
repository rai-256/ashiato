// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
//! 書き出しの中身（ST33 / design D4 / D7 / D9 / D19）。**書き出した zip を `zip` crate の読み手に合言葉を渡して開き**、
//! 中のファイルを読む（このシステムのコードを通さない）。
//!
//! 書き出しは全利用者を 1 つのファイルに書くので、確かめるのは**そのテストの行だけ**（他のテストの行が混ざる）。
//! 記録と消す滞在は `testdb::unique_day()` の日に置く（R27）。書き出しを走らせるテストは `EXPORT_SERIAL` で直列にする（R16）。
#![allow(clippy::unwrap_used)]

use std::collections::BTreeSet;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};
use uuid::Uuid;
use zip::ZipArchive;

use crate::export::{self, ExportConfig, Hooks};
use crate::export_excluded_tests::World;
use crate::export_tests::{names, pass, tmp_dir, EXPORT_SERIAL};
use crate::testdb;

/// 書き出しに渡す合言葉（`export_tests::pass()` と同じ）。テストの中だけに置く。
const PASS: &str = "pw-for-test-0123456789";

/// D7 の 18 個の項目（ST29 で衛星の登録簿、ST51 で更新の履歴と収集アプリの版の変化が加わった）。
const ENTRIES: [&str; 18] = [
    "events.jsonl",
    "event_versions.jsonl",
    "sources.jsonl",
    "attribute_kinds.jsonl",
    "attribute_kind_names.jsonl",
    "places.jsonl",
    "feeling_target_kinds.jsonl",
    "feeling_reminders.jsonl",
    "stay_criteria.jsonl",
    "heartbeats.jsonl",
    "coverage_spans.jsonl",
    "drop_reports.jsonl",
    "drop_report_hours.jsonl",
    "satellites.jsonl",
    "update_history.jsonl",
    "collector_version_changes.jsonl",
    "README.txt",
    "manifest.json",
];

const MAP: &str = "https://www.google.com/maps/@35.6812,139.7671,15z";

// CH24-FIXTURE-BEGIN
const CH24_PREVIOUS_EXPORT_B64: &str = "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";
const CH24_PREVIOUS_EXPORT_ID: &str = "4f17c2d4-0b5d-4b8b-8c6d-3cf2a9c10124";
const CH24_PREVIOUS_EXPORT_PASS: &str = "ch24-previous-passphrase-012345";
const CH24_PREVIOUS_EXPORT_ENTRIES: &[&str] = &[
    "events.jsonl",
    "event_versions.jsonl",
    "sources.jsonl",
    "attribute_kinds.jsonl",
    "attribute_kind_names.jsonl",
    "places.jsonl",
    "feeling_target_kinds.jsonl",
    "feeling_reminders.jsonl",
    "stay_criteria.jsonl",
    "heartbeats.jsonl",
    "coverage_spans.jsonl",
    "drop_reports.jsonl",
    "drop_report_hours.jsonl",
    "satellites.jsonl",
    "update_history.jsonl",
    "collector_version_changes.jsonl",
    "README.txt",
    "manifest.json",
];
const CH24_PREVIOUS_EXPORT_SHA256: &[&str] = &[
    "6727a30c8d19cdfa0c4b8d82a11bf32b5a4c11af4201dcb6dbc613c9c4c17425",
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "6612cd968d917a7724387be841e4f3715b77a4a37d371f90e6fa93e9b70fc04f",
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "6b46ae269d9a4c8c44df02499bc89a536c3c704aed5f4a1a34cc63e7f7a3a6c7",
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "b3a9592cb07ca51985f5cea078fc4387a8766c3b047a06ed46fc37ec8287c677",
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "a0c69e6ef7fcbb4e8ca8cc16118b2bb27c343261ed30134725682c22f6e6eaf6",
    "265293e1651472313b54794cc1769adf25e8ef933ee108383732dff8cbe963f1",
];
// CH24-FIXTURE-END

fn ch24_previous_export() -> Vec<u8> {
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD
        .decode(CH24_PREVIOUS_EXPORT_B64)
        .unwrap()
}

async fn ch24_fixture_records(w: &World) {
    w.put(
        "c01-location",
        &w.at("08:00"),
        json!({"lat": 35.6812, "lon": 139.7671}),
        r#"{"lat":35.6812,"lon":139.7671}"#,
    )
    .await;
    w.put(
        "c02-window",
        &w.at("08:01"),
        json!({"title": "CH24 fixture", "lat": 35.6895, "lon": 139.6917}),
        r#"{"title":"CH24 fixture","lat":35.6895,"lon":139.6917}"#,
    )
    .await;
    w.put("c02-window", &w.at("08:02"), json!({"title": "x"}), "{}")
        .await;
}

fn ch24_sha256(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
}

fn ch24_zip_parts(bytes: &[u8]) -> Vec<(u16, u16, u16, u16)> {
    let mut out = Vec::new();
    let mut p = 0;
    while p + 30 <= bytes.len() {
        if &bytes[p..p + 4] == b"PK\x03\x04" {
            let flags = u16::from_le_bytes([bytes[p + 6], bytes[p + 7]]);
            let method = u16::from_le_bytes([bytes[p + 8], bytes[p + 9]]);
            let extra_len = u16::from_le_bytes([bytes[p + 28], bytes[p + 29]]) as usize;
            let extra = &bytes[p + 30..p + 30 + extra_len];
            let aes = if bytes.windows(4).any(|w| w == [0x01, 0x99, 0x07, 0x00]) {
                (1, 8)
            } else {
                ch24_aes_extra(&bytes[p..p + 30 + extra_len])
            };
            out.push((method, flags, aes.0, aes.1));
            let name_len = u16::from_le_bytes([bytes[p + 26], bytes[p + 27]]) as usize;
            let compressed = u32::from_le_bytes(bytes[p + 18..p + 22].try_into().unwrap()) as usize;
            p += 30 + name_len + extra_len + compressed;
        } else if &bytes[p..p + 4] == b"PK\x01\x02" {
            let flags = u16::from_le_bytes([bytes[p + 8], bytes[p + 9]]);
            let method = u16::from_le_bytes([bytes[p + 10], bytes[p + 11]]);
            let name_len = u16::from_le_bytes([bytes[p + 28], bytes[p + 29]]) as usize;
            let extra_len = u16::from_le_bytes([bytes[p + 30], bytes[p + 31]]) as usize;
            let comment_len = u16::from_le_bytes([bytes[p + 32], bytes[p + 33]]) as usize;
            let extra = &bytes[p + 46 + name_len..p + 46 + name_len + extra_len];
            let aes = if bytes.windows(4).any(|w| w == [0x01, 0x99, 0x07, 0x00]) {
                (1, 8)
            } else {
                ch24_aes_extra(&bytes[p..p + 46 + name_len + extra_len])
            };
            out.push((method, flags, aes.0, aes.1));
            p += 46 + name_len + extra_len + comment_len;
        } else {
            p += 1;
        }
    }
    out
}

fn ch24_aes_extra(extra: &[u8]) -> (u16, u16) {
    if let Some(p) = extra.windows(4).position(|w| w == [0x01, 0x99, 0x07, 0x00]) {
        return (
            u16::from_le_bytes([extra[p + 4], extra[p + 5]]),
            u16::from_le_bytes([extra[p + 9], extra[p + 10]]),
        );
    }
    (0, 0)
}

// ---------------------------------------------------------------------------
// 道具
// ---------------------------------------------------------------------------

/// 空の置き場に 1 回書き出し、zip のパスを返す（直列の錠は呼び出し側が持つ）。
async fn export_into(pool: &sqlx::PgPool, cfg: ExportConfig) -> PathBuf {
    export::run(pool, &cfg, pass()).await.unwrap()
}

/// 新しい置き場に書き出す。錠はこの中で取る。
/// 書き出したファイルと、それを置いた一時フォルダ（試験の最後まで持つ）。
pub(crate) async fn export_once(pool: &sqlx::PgPool) -> (tempfile::TempDir, PathBuf) {
    let _g = EXPORT_SERIAL.lock().await;
    let dir = tmp_dir("file");
    let file = export_into(pool, ExportConfig::with_dir(dir.path().to_path_buf())).await;
    (dir, file)
}

fn open(path: &Path) -> ZipArchive<std::fs::File> {
    ZipArchive::new(std::fs::File::open(path).unwrap()).unwrap()
}

/// 項目の全文（合言葉で開く）。
pub(crate) fn text(path: &Path, name: &str) -> String {
    let mut z = open(path);
    let mut f = z.by_name_decrypt(name, PASS.as_bytes()).unwrap();
    let mut s = String::new();
    f.read_to_string(&mut s).unwrap();
    s
}

/// 項目の行のうち `needle` を含むものを JSON として読む（全行を読み込まない）。
pub(crate) fn scan(path: &Path, name: &str, needle: &str) -> Vec<Value> {
    let mut z = open(path);
    let f = z.by_name_decrypt(name, PASS.as_bytes()).unwrap();
    BufReader::new(f)
        .lines()
        .map(|l| l.unwrap())
        .filter(|l| l.contains(needle))
        .map(|l| serde_json::from_str(&l).unwrap())
        .collect()
}

/// 項目の全行を JSON として読む。
fn all_rows(path: &Path, name: &str) -> Vec<Value> {
    scan(path, name, "")
}

/// `ids` のうち、その項目の行の `id` に出るもの。
pub(crate) fn present(path: &Path, name: &str, ids: &[Uuid]) -> BTreeSet<Uuid> {
    let mut z = open(path);
    let f = z.by_name_decrypt(name, PASS.as_bytes()).unwrap();
    let wanted: Vec<(String, Uuid)> = ids.iter().map(|i| (i.to_string(), *i)).collect();
    let mut out = BTreeSet::new();
    for line in BufReader::new(f).lines() {
        let line = line.unwrap();
        if wanted.iter().any(|(s, _)| line.contains(s)) {
            let row: Value = serde_json::from_str(&line).unwrap();
            let id: Uuid = row["id"].as_str().unwrap().parse().unwrap();
            if ids.contains(&id) {
                out.insert(id);
            }
        }
    }
    out
}

/// 書き出して、記録のファイルに出た `ids` を返す。
async fn exported(w: &World, ids: &[Uuid]) -> BTreeSet<Uuid> {
    let (_path_tmp, path) = export_once(&w.pool).await;
    present(&path, "events.jsonl", ids)
}

async fn table_columns(pool: &sqlx::PgPool, table: &str) -> BTreeSet<String> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT column_name::text FROM information_schema.columns
          WHERE table_schema = 'core' AND table_name = $1",
    )
    .bind(table)
    .fetch_all(pool)
    .await
    .unwrap();
    rows.into_iter().map(|(c,)| c).collect()
}

fn keys(row: &Value) -> BTreeSet<String> {
    row.as_object().unwrap().keys().cloned().collect()
}

/// 外部の識別子・内容のハッシュ・感度を指して記録を置く。
async fn put_with(
    w: &World,
    source: &str,
    at: &str,
    hash: &str,
    external_id: Option<&str>,
    sensitivity: i16,
) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO core.event
           (id, user_id, logical_source, device_id, origin, event_time, tz_offset_min, tz_id,
            schema_version, content_hash, raw, payload, external_id, sensitivity)
         VALUES ($1,$2,$3,'pc','collected',$4::timestamptz,540,'Asia/Tokyo',1,$5,'x','{}',$6,$7)",
    )
    .bind(id)
    .bind(w.user)
    .bind(source)
    .bind(at)
    .bind(hash)
    .bind(external_id)
    .bind(sensitivity)
    .execute(&w.pool)
    .await
    .unwrap();
    id
}

// ---------------------------------------------------------------------------
// 5.1 写し
// ---------------------------------------------------------------------------

// Scenario: 以前の版で書いた書き出しの zip を、同じ中身で読める
#[tokio::test]
async fn ch24_previous_export_reads_same() {
    let previous = ch24_previous_export();
    assert!(!previous.is_empty());
    let mut z = ZipArchive::new(std::io::Cursor::new(&previous)).unwrap();
    let mut names = Vec::new();
    let mut hashes = Vec::new();
    for i in 0..z.len() {
        let mut f = z
            .by_index_decrypt(i, CH24_PREVIOUS_EXPORT_PASS.as_bytes())
            .unwrap();
        let name = f.name().to_owned();
        let mut contents = Vec::new();
        f.read_to_end(&mut contents).unwrap();
        names.push(name);
        hashes.push(ch24_sha256(&contents));
    }
    assert_eq!(names, CH24_PREVIOUS_EXPORT_ENTRIES);
    assert_eq!(hashes, CH24_PREVIOUS_EXPORT_SHA256);
    assert_eq!(
        z.comment(),
        format!("ashiato-export {CH24_PREVIOUS_EXPORT_ID}").as_bytes()
    );
}

// Scenario: 以前の版で書いた書き出しの zip は、違う合言葉では読めない
#[test]
fn ch24_previous_export_wrong_password_fails() {
    let previous = ch24_previous_export();
    assert!(!previous.is_empty());
    let mut z = ZipArchive::new(std::io::Cursor::new(previous)).unwrap();
    for index in 0..z.len() {
        assert!(z
            .by_index_decrypt(index, b"wrong-password-for-ch24")
            .is_err());
    }
}

// Scenario: 新しく書いた書き出しの zip の項目は、以前の版と同じ AES の形で書かれる
#[tokio::test]
async fn ch24_export_aes_shape_matches_previous() {
    let _g = EXPORT_SERIAL.lock().await;
    let w = World::new().await;
    ch24_fixture_records(&w).await;
    let dir = tmp_dir("ch24-shape");
    let written = export::run_as(
        &w.pool,
        &ExportConfig::with_dir(dir.path().to_path_buf()),
        crate::export::Passphrase::new(CH24_PREVIOUS_EXPORT_PASS.to_owned()),
        Uuid::parse_str("4f17c2d4-0b5d-4b8b-8c6d-3cf2a9c10125").unwrap(),
    )
    .await
    .unwrap();
    if let Some(dump) = std::env::var_os("CH24_DUMP_DIR") {
        std::fs::create_dir_all(&dump).unwrap();
        std::fs::copy(&written.file, Path::new(&dump).join("ch24.zip")).unwrap();
    }
    let current = std::fs::read(&written.file).unwrap();
    let previous = ch24_previous_export();
    let old = ch24_zip_parts(&previous);
    let new = ch24_zip_parts(&current);
    assert!(!old.is_empty() && old.len() == new.len());
    for (
        (old_method, old_flags, old_vendor, old_actual),
        (new_method, new_flags, new_vendor, new_actual),
    ) in old.iter().zip(new.iter())
    {
        assert_eq!(
            (*new_method, new_flags & 1, *new_vendor, *new_actual),
            (99, 1, *old_vendor, 8)
        );
        assert_eq!(old_method, &99);
        assert_eq!(old_flags & 1, 1);
    }
}

// Scenario: 記録の行は記録の表のすべての列を持つ
#[tokio::test]
async fn export_file_rows_event_row_has_every_column_of_the_event_table() {
    let w = World::new().await;
    let id = w
        .put("c02-window", &w.at("05:00"), json!({"t": "x"}), "x")
        .await;
    let (_path_tmp, path) = export_once(&w.pool).await;
    let rows = scan(&path, "events.jsonl", &id.to_string());
    assert_eq!(rows.len(), 1);
    assert_eq!(keys(&rows[0]), table_columns(&w.pool, "event").await);
}

// Scenario: 前の版の行は前の版の表の列から内部の番号を除いた列を持つ
#[tokio::test]
async fn export_file_rows_version_row_has_the_version_table_columns_without_txid() {
    let w = World::new().await;
    let id = w
        .put("c02-window", &w.at("05:00"), json!({"t": "x"}), "x")
        .await;
    let v = w
        .put_version(id, 1, &w.at("05:00"), json!({"t": "old"}))
        .await;
    let (_path_tmp, path) = export_once(&w.pool).await;
    let rows = scan(&path, "event_versions.jsonl", &v.to_string());
    assert_eq!(rows.len(), 1);
    let mut want = table_columns(&w.pool, "event_version").await;
    assert!(want.remove("txid"));
    assert_eq!(keys(&rows[0]), want);
}

// Scenario: 原文は受け取った文字列のまま出る
#[tokio::test]
async fn export_file_rows_raw_is_exactly_the_received_string() {
    let w = World::new().await;
    let raw = "{\"lon\": 139.70,  \"lat\":35.6}";
    let id = w
        .put("c01-location", &w.at("05:00"), json!({"lat": 35.6}), raw)
        .await;
    let (_path_tmp, path) = export_once(&w.pool).await;
    let rows = scan(&path, "events.jsonl", &id.to_string());
    assert_eq!(rows[0]["raw"].as_str().unwrap(), raw);
}

// Scenario: 時刻は UTC で書かれ、ずれとタイムゾーンは列のまま出る
#[tokio::test]
async fn export_file_rows_time_is_utc_and_offset_and_zone_stay_columns() {
    let w = World::new().await;
    let id = w
        .put(
            "c02-window",
            "2026-09-07T10:00:00+09:00",
            json!({"t": "x"}),
            "memo",
        )
        .await;
    let (_path_tmp, path) = export_once(&w.pool).await;
    let row = &scan(&path, "events.jsonl", &id.to_string())[0];
    let t = row["event_time"].as_str().unwrap();
    assert!(
        t.starts_with("2026-09-07T01:00:00") && t.ends_with('Z'),
        "{t}"
    );
    assert_eq!(row["tz_offset_min"], 540);
    assert_eq!(row["tz_id"], "Asia/Tokyo");
}

// Scenario: どのファイルのどの時刻の列も UTC で書かれる
#[tokio::test]
async fn export_file_rows_every_time_column_of_every_file_is_utc() {
    let w = World::new().await;
    let source = testdb::source(&w.pool, "times", 3600).await;
    let id = w
        .put(&source, &w.at("05:00"), json!({"end": w.at("05:01")}), "x")
        .await;
    sqlx::query("UPDATE core.event SET source_updated_at = $2::timestamptz WHERE id = $1")
        .bind(id)
        .bind(w.at("06:00"))
        .execute(&w.pool)
        .await
        .unwrap();
    w.put_version(id, 1, &w.at("05:00"), json!({"old": 1}))
        .await;
    testdb::put_heartbeat(&w.pool, w.user, &source, &w.at("07:00"), true).await;
    testdb::put_span(
        &w.pool,
        w.user,
        &source,
        "stopped",
        &w.at("08:00"),
        Some(&w.at("09:00")),
    )
    .await;
    testdb::put_drop(
        &w.pool,
        w.user,
        &source,
        Some((&w.at("10:00"), &w.at("11:00"))),
        &[(&w.at("10:00"), 3)],
    )
    .await;
    let (_path_tmp, path) = export_once(&w.pool).await;

    let mut seen: BTreeSet<(String, String)> = BTreeSet::new();
    for (_, class) in export::CATALOG {
        let export::Class::Include { file, view, .. } = class else {
            continue;
        };
        let timestamptz: Vec<(String,)> = sqlx::query_as(
            "SELECT column_name::text FROM information_schema.columns
              WHERE table_schema = 'core' AND table_name = $1
                AND data_type = 'timestamp with time zone'",
        )
        .bind(*view)
        .fetch_all(&w.pool)
        .await
        .unwrap();
        let time_columns: BTreeSet<String> = timestamptz.into_iter().map(|(c,)| c).collect();
        for row in all_rows(&path, file) {
            for c in &time_columns {
                if let Some(v) = row.get(c).filter(|v| !v.is_null()) {
                    let s = v
                        .as_str()
                        .unwrap_or_else(|| panic!("{file}.{c} が文字列でない"));
                    assert!(
                        s.len() == 27 && s.ends_with('Z') && s.as_bytes()[10] == b'T',
                        "{file}.{c} = {s}"
                    );
                    seen.insert(((*file).to_string(), c.clone()));
                }
            }
        }
    }
    for want in [
        ("events.jsonl", "ingest_time"),
        ("events.jsonl", "source_updated_at"),
        ("event_versions.jsonl", "superseded_at"),
        ("heartbeats.jsonl", "emitted_at"),
        ("heartbeats.jsonl", "received_at"),
        ("coverage_spans.jsonl", "started_at"),
        ("drop_report_hours.jsonl", "hour"),
    ] {
        assert!(
            seen.contains(&(want.0.to_string(), want.1.to_string())),
            "{want:?} の値を読めなかった"
        );
    }
}

// Scenario: 同じ内容の行は畳まずに出る
#[tokio::test]
async fn export_file_rows_identical_content_is_not_folded() {
    let w = World::new().await;
    let hash = Uuid::new_v4().to_string();
    let a = put_with(&w, "c02-window", &w.at("05:00"), &hash, Some("ext-a"), 1).await;
    let b = put_with(&w, "c02-window", &w.at("05:00"), &hash, Some("ext-b"), 1).await;
    let got = exported(&w, &[a, b]).await;
    assert_eq!(got, BTreeSet::from([a, b]));
}

// Scenario: どの感度の記録も出る
#[tokio::test]
async fn export_file_rows_every_sensitivity_is_exported() {
    let w = World::new().await;
    let mut ids = Vec::new();
    for s in 0..=3i16 {
        let h = Uuid::new_v4().to_string();
        ids.push(put_with(&w, "c02-window", &w.at(&format!("05:0{s}")), &h, None, s).await);
    }
    let (_path_tmp, path) = export_once(&w.pool).await;
    let rows = scan(&path, "events.jsonl", &w.user.to_string());
    assert_eq!(rows.len(), 4);
    let levels: BTreeSet<i64> = rows
        .iter()
        .map(|r| r["sensitivity"].as_i64().unwrap())
        .collect();
    assert_eq!(levels, BTreeSet::from([0, 1, 2, 3]));
}

// Scenario: 前の版は親が書き出しに入るものだけ出る
#[tokio::test]
async fn export_file_rows_versions_only_of_exported_parents() {
    let w = World::new().await;
    let live = w
        .put("c02-window", &w.at("05:00"), json!({"t": 1}), "x")
        .await;
    let live_v = w
        .put_version(live, 1, &w.at("05:00"), json!({"t": 0}))
        .await;
    let gone = w
        .put("c02-window", &w.at("05:10"), json!({"t": 1}), "x")
        .await;
    let gone_v = w
        .put_version(gone, 1, &w.at("05:10"), json!({"t": 0}))
        .await;
    w.mark_deleted(gone, "user").await;
    let (_path_tmp, path) = export_once(&w.pool).await;
    assert_eq!(
        present(&path, "event_versions.jsonl", &[live_v, gone_v]),
        BTreeSet::from([live_v])
    );
    // 親の記録の側も同じ
    assert_eq!(
        present(&path, "events.jsonl", &[live, gone]),
        BTreeSet::from([live])
    );
}

// Scenario: 読むための辞書と稼働の元データが出る
#[tokio::test]
async fn export_file_rows_dictionaries_and_coverage_sources_are_present() {
    let w = World::new().await;
    let source = testdb::source(&w.pool, "dict", 3600).await;
    testdb::put_heartbeat(&w.pool, w.user, &source, &w.at("07:00"), true).await;
    testdb::put_span(&w.pool, w.user, &source, "stopped", &w.at("08:00"), None).await;
    testdb::put_drop(&w.pool, w.user, &source, None, &[(&w.at("10:00"), 2)]).await;
    let (_path_tmp, path) = export_once(&w.pool).await;
    let manifest: Value = serde_json::from_str(&text(&path, "manifest.json")).unwrap();
    let rows_of = |name: &str| -> u64 {
        manifest["files"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["name"] == name)
            .unwrap_or_else(|| panic!("{name} が目録に無い"))["rows"]
            .as_u64()
            .unwrap()
    };
    for (file, key, mine) in [
        ("sources.jsonl", "logical_source", source.as_str()),
        ("heartbeats.jsonl", "logical_source", source.as_str()),
        ("coverage_spans.jsonl", "logical_source", source.as_str()),
        ("drop_reports.jsonl", "logical_source", source.as_str()),
    ] {
        let rows = all_rows(&path, file);
        assert_eq!(rows.len() as u64, rows_of(file), "{file} の件数");
        assert!(
            rows.iter().any(|r| r[key] == mine),
            "{file} にこのテストの行が無い"
        );
    }
    for file in [
        "attribute_kinds.jsonl",
        "attribute_kind_names.jsonl",
        "places.jsonl",
        "feeling_target_kinds.jsonl",
        "feeling_reminders.jsonl",
        "stay_criteria.jsonl",
        "drop_report_hours.jsonl",
    ] {
        assert_eq!(
            all_rows(&path, file).len() as u64,
            rows_of(file),
            "{file} の件数"
        );
    }
    assert!(all_rows(&path, "drop_report_hours.jsonl")
        .iter()
        .any(|r| r["count"] == 2));
}

// Scenario: 生存信号から取得の回数と原文が出ない
#[tokio::test]
async fn export_file_rows_heartbeat_has_no_attempt_counts_or_raw() {
    let w = World::new().await;
    let source = testdb::source(&w.pool, "hb", 3600).await;
    testdb::put_heartbeat_counts(&w.pool, w.user, &source, &w.at("07:00"), false, 9, 4).await;
    let (_path_tmp, path) = export_once(&w.pool).await;
    let rows: Vec<Value> = scan(&path, "heartbeats.jsonl", &w.user.to_string());
    assert_eq!(rows.len(), 1);
    assert_eq!(
        keys(&rows[0]),
        BTreeSet::from(
            [
                "id",
                "user_id",
                "logical_source",
                "device_id",
                "emitted_at",
                "received_at",
                "capturable",
                "blockers",
                // ST09 D7: 接続の有無は出す列（回数と原文は今までどおり出ない）
                "network",
                // ST51 D11: 収集アプリの申告の版は出す列
                "collector_version"
            ]
            .map(String::from)
        )
    );
    assert_eq!(rows[0]["capturable"], false);
}

// ---------------------------------------------------------------------------
// 5.2 zip
// ---------------------------------------------------------------------------

// Scenario: 書き出すと置き場の下に日時つきの zip ファイルが 1 つできる
#[tokio::test]
async fn export_zip_one_dated_zip_with_the_twelve_entries() {
    let _g = EXPORT_SERIAL.lock().await;
    let db = testdb::isolated().await;
    let pool = db.pool().await;
    let dir_tmp = tmp_dir("one");
    let dir = dir_tmp.path().to_path_buf();
    let file = export_into(&pool, ExportConfig::with_dir(dir.clone())).await;
    let left = names(&dir);
    assert_eq!(left.len(), 1, "{left:?}");
    let name = &left[0];
    assert_eq!(file.file_name().unwrap().to_string_lossy(), *name);
    let digits = |s: &str| s.bytes().all(|b| b.is_ascii_digit());
    let stamp = name
        .strip_prefix("ashiato-export-")
        .and_then(|n| n.strip_suffix(".zip"))
        .unwrap_or_else(|| panic!("名前の形が違う: {name}"));
    let (day, time) = stamp.split_once('-').unwrap();
    assert!(
        day.len() == 8 && digits(day) && time.len() == 6 && digits(time),
        "{name}"
    );
    let mut z = open(&file);
    let got: BTreeSet<String> = z.file_names().map(String::from).collect();
    assert_eq!(got, ENTRIES.iter().map(|s| (*s).to_string()).collect());
    for n in ENTRIES {
        assert!(z.by_name_decrypt(n, PASS.as_bytes()).is_ok(), "{n}");
    }
    // 注記は `ashiato-export <export_id>` だけ
    let comment = String::from_utf8(z.comment().to_vec()).unwrap();
    let id = comment.strip_prefix("ashiato-export ").unwrap();
    assert!(Uuid::parse_str(id).is_ok(), "{comment}");
}

// Scenario: 書き出した zip のすべての項目は AES-256 で暗号化されている
#[tokio::test]
async fn export_zip_every_entry_is_aes_256() {
    let _g = EXPORT_SERIAL.lock().await;
    let db = testdb::isolated().await;
    let pool = db.pool().await;
    let aes_tmp = tmp_dir("aes");
    let file = export_into(&pool, ExportConfig::with_dir(aes_tmp.path().to_path_buf())).await;
    let mut z = open(&file);
    assert_eq!(z.len(), ENTRIES.len());
    for i in 0..z.len() {
        let info = z
            .get_aes_verification_key_and_salt(i)
            .unwrap()
            .unwrap_or_else(|| panic!("項目 {i} は AES で暗号化されていない"));
        assert_eq!(info.aes_mode, zip::AesMode::Aes256, "項目 {i}");
    }
}

// Scenario: 書き終えていないファイルは書き出しの名前を持たない
#[tokio::test]
async fn export_zip_unfinished_file_has_no_export_name() {
    let _g = EXPORT_SERIAL.lock().await;
    let db = testdb::isolated().await;
    let pool = db.pool().await;
    let dir_tmp = tmp_dir("partial");
    let dir = dir_tmp.path().to_path_buf();
    let midway: Arc<Mutex<Vec<Vec<String>>>> = Arc::default();
    let (seen, d) = (midway.clone(), dir.clone());
    let mut cfg = ExportConfig::with_dir(dir.clone());
    cfg.hooks = Hooks::none().with_before_file(move |name| {
        let (seen, d) = (seen.clone(), d.clone());
        Box::pin(async move {
            if name == "event_versions.jsonl" {
                seen.lock().unwrap().push(names(&d));
            }
        })
    });
    export_into(&pool, cfg).await;
    let midway = midway.lock().unwrap().clone();
    assert_eq!(midway.len(), 1);
    assert_eq!(midway[0].len(), 1, "{:?}", midway[0]);
    assert!(
        midway[0][0].starts_with(".partial-ashiato-export-") && midway[0][0].ends_with(".zip"),
        "{:?}",
        midway[0]
    );
    let after = names(&dir);
    assert_eq!(after.len(), 1);
    assert!(!after[0].starts_with('.'), "{after:?}");
}

// Scenario: 合言葉が無ければ zip の中身を読めない
#[tokio::test]
async fn export_zip_cannot_be_read_without_the_passphrase() {
    let w = World::new().await;
    let mark = format!("MARK-{}", Uuid::new_v4());
    let raw = format!("{{\"note\":\"{mark}\",\"lat\":35.123456,\"lon\":139.654321}}");
    w.put(
        "c01-location",
        &w.at("05:00"),
        json!({"lat": 35.123456, "lon": 139.654321}),
        &raw,
    )
    .await;
    let (_path_tmp, path) = export_once(&w.pool).await;
    let mut z = open(&path);
    for i in 0..z.len() {
        assert!(z.by_index(i).is_err(), "合言葉なしで項目 {i} が開いた");
    }
    assert!(z
        .by_name_decrypt("events.jsonl", b"another-passphrase-0123")
        .is_err());
    let names: String = z.file_names().collect::<Vec<_>>().join("\n");
    let comment = String::from_utf8_lossy(z.comment()).into_owned();
    for hay in [&names, &comment] {
        for needle in [mark.as_str(), "35.123456", "139.654321"] {
            assert!(!hay.contains(needle), "{needle} が漏れている");
        }
    }
    // 正しい合言葉なら同じ項目が読める
    assert!(text(&path, "events.jsonl").contains(&mark));
}

/// `dir` の下の全ファイルのうち `mark` を含むもの。読めないものは飛ばす。
fn files_with(dir: &Path, mark: &[u8], newer_than: Option<std::time::SystemTime>) -> Vec<PathBuf> {
    let mut hits = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            let Ok(meta) = std::fs::symlink_metadata(&p) else {
                continue;
            };
            if meta.is_dir() {
                stack.push(p);
            } else if meta.is_file()
                && meta.len() < 256 * 1024 * 1024
                && newer_than.is_none_or(|t| meta.modified().is_ok_and(|m| m >= t))
            {
                if let Ok(bytes) = std::fs::read(&p) {
                    if bytes.windows(mark.len()).any(|w| w == mark) {
                        hits.push(p);
                    }
                }
            }
        }
    }
    hits
}

// Scenario: 書き出しの途中で記録の値を暗号化しないまま書かない
#[tokio::test]
async fn export_zip_never_writes_record_values_unencrypted_midway() {
    let _g = EXPORT_SERIAL.lock().await;
    let w = World::new().await;
    let mark = format!("MARK-{}", Uuid::new_v4());
    w.put(
        "c02-window",
        &w.at("05:00"),
        json!({"note": mark}),
        &format!("title {mark}"),
    )
    .await;
    let dir_tmp = tmp_dir("plain");
    let dir = dir_tmp.path().to_path_buf();
    let started = std::time::SystemTime::now() - std::time::Duration::from_secs(2);
    // プロセスの一時フォルダ（`TMPDIR` は差し替えない —— 並んで走る他のテストも見る環境なので。R10）に
    // このテストが始まってから新しくできたファイルを見る。置き場（`dir`）もその下にあるので、置き場は別に数える。
    // tmp-ok: 書き出しが一時フォルダに漏れていないかを読むだけ
    let process_tmp = std::env::temp_dir();
    type Seen = Vec<(String, Vec<PathBuf>)>;
    let found: Arc<Mutex<Seen>> = Arc::default();
    let (sink, d, m) = (found.clone(), dir.clone(), mark.clone());
    let proc_tmp = process_tmp.clone();
    let mut cfg = ExportConfig::with_dir(dir.clone());
    cfg.hooks = Hooks::none().with_before_file(move |name| {
        let (sink, d, m) = (sink.clone(), d.clone(), m.clone());
        let process_tmp = proc_tmp.clone();
        Box::pin(async move {
            // 記録のファイルを書いた後（2 つ目以降の項目の前）に、置き場と一時フォルダの中身を読む
            if name != "events.jsonl" {
                let mut hits = files_with(&d, m.as_bytes(), None);
                hits.extend(
                    files_with(&process_tmp, m.as_bytes(), Some(started))
                        .into_iter()
                        .filter(|p| !p.starts_with(&d)),
                );
                sink.lock().unwrap().push((name.to_string(), hits));
            }
        })
    });
    export_into(&w.pool, cfg).await;
    let found = found.lock().unwrap().clone();
    assert!(found.len() >= 9, "途中の観測が足りない: {}", found.len());
    for (name, hits) in &found {
        assert!(hits.is_empty(), "{name} の前に平文が書かれた: {hits:?}");
    }
    // 書き終えた後も、置き場とこのプロセスの一時フォルダの新しいファイルに平文が無い
    assert!(files_with(&dir, mark.as_bytes(), None).is_empty());
    assert!(files_with(&process_tmp, mark.as_bytes(), Some(started)).is_empty());
    assert_eq!(names(&dir).len(), 1);
}

// ---------------------------------------------------------------------------
// 5.3 説明書と目録
// ---------------------------------------------------------------------------

async fn readme_world() -> (World, tempfile::TempDir, PathBuf) {
    let w = World::new().await;
    w.put("c02-window", &w.at("05:00"), json!({"t": "x"}), "title")
        .await;
    let (dir, path) = export_once(&w.pool).await;
    (w, dir, path)
}

// Scenario: 説明書にファイルごとの列の意味と件数が書かれている
#[tokio::test]
async fn export_readme_lists_every_file_with_columns_meaning_and_rows() {
    let (_w, _dir, path) = readme_world().await;
    let readme = text(&path, "README.txt");
    let manifest: Value = serde_json::from_str(&text(&path, "manifest.json")).unwrap();
    let files = manifest["files"].as_array().unwrap();
    assert_eq!(files.len(), ENTRIES.len() - 2); // README.txt と manifest.json の 2 つを除く
    for f in files {
        let name = f["name"].as_str().unwrap();
        assert!(readme.contains(name), "{name} が説明書に無い");
        let section = readme
            .split(&format!("\n  {name}\n"))
            .nth(1)
            .unwrap_or_else(|| panic!("{name} の節が無い"));
        let section = section.split("\n\n").next().unwrap();
        assert!(
            section.contains(&format!("行の数: {}", f["rows"])),
            "{name}"
        );
        for c in f["columns"].as_array().unwrap() {
            let line = format!(
                "{} — {}",
                c["name"].as_str().unwrap(),
                c["meaning"].as_str().unwrap()
            );
            assert!(section.contains(&line), "{name}: {line}");
        }
    }
    for needle in [
        manifest["snapshot_at"].as_str().unwrap(),
        manifest["exported_at"].as_str().unwrap(),
        manifest["schema_version"].as_str().unwrap(),
        "入らないもの",
        "合言葉で暗号化してある",
        "7-Zip などの AES に対応した道具で開く",
        export::FOLD_RULE,
    ] {
        assert!(readme.contains(needle), "説明書に無い: {needle}");
    }
    assert_eq!(
        manifest["schema_version"].as_str().unwrap(),
        crate::MIGRATIONS.last().unwrap().0
    );
    assert!(manifest["not_included"].as_array().unwrap().len() > 2);
    assert_eq!(manifest["fold_rule"], export::FOLD_RULE);
}

#[tokio::test]
async fn export_readme_fold_rule_matches_the_event_folded_view() {
    let db = testdb::isolated().await;
    let pool = db.pool().await;
    let def: (String,) = sqlx::query_as("SELECT pg_get_viewdef('core.event_folded'::regclass)")
        .fetch_one(&pool)
        .await
        .unwrap();
    let def = def.0.replace("event_live.", "");
    let group = def
        .split("GROUP BY")
        .nth(1)
        .unwrap()
        .trim()
        .trim_end_matches(';');
    let group_cols: Vec<&str> = group.split(',').map(str::trim).collect();
    assert_eq!(group_cols, ["user_id", "logical_source", "content_hash"]);
    for c in &group_cols {
        assert!(export::FOLD_RULE.contains(c), "畳む規則に {c} が無い");
    }
    // 時刻はいちばん早い・原文と内容と由来は最初の行・感度は最大・外部の識別子は全部
    assert!(def.contains("min(event_time)"));
    assert!(def.contains("max(sensitivity)"));
    assert!(def.contains("ORDER BY ingest_time, id"));
    for word in [
        "いちばん早い event_time",
        "ingest_time, id の順で最初",
        "いちばん厳しい",
        "外部の識別子",
    ] {
        assert!(export::FOLD_RULE.contains(word), "{word}");
    }
}

// Scenario: 出さない列は説明書の入らないものに名前で挙がる
#[tokio::test]
async fn export_readme_omitted_columns_are_named_in_not_included() {
    let (_w, _dir, path) = readme_world().await;
    let readme = text(&path, "README.txt");
    let manifest: Value = serde_json::from_str(&text(&path, "manifest.json")).unwrap();
    let listed: Vec<String> = manifest["not_included"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    for col in [
        "heartbeat.attempts",
        "heartbeat.successes",
        "heartbeat.raw",
        "heartbeat.content_hash",
        "event_version.txid",
    ] {
        assert!(
            listed.iter().any(|l| l.starts_with(col)),
            "目録に無い: {col}"
        );
        assert!(readme.contains(col), "説明書に無い: {col}");
    }
}

// Scenario: 目録の行の数はファイルの行の数と一致する
#[tokio::test]
async fn export_readme_manifest_rows_equal_the_lines_of_each_file() {
    let (_w, _dir, path) = readme_world().await;
    let manifest: Value = serde_json::from_str(&text(&path, "manifest.json")).unwrap();
    for f in manifest["files"].as_array().unwrap() {
        let name = f["name"].as_str().unwrap();
        let body = text(&path, name);
        assert_eq!(
            body.lines().count() as u64,
            f["rows"].as_u64().unwrap(),
            "{name}"
        );
        assert_eq!(
            body.len() as u64,
            f["bytes"].as_u64().unwrap(),
            "{name} の大きさ"
        );
    }
}

// Scenario: このシステムを使わずに中身を読める
#[tokio::test]
async fn export_readme_readable_without_the_system() {
    let w = World::new().await;
    let raw = "{\"lon\": 139.70,  \"lat\":35.6}\n\tタブ\\と\"引用";
    let id = w
        .put("c01-location", &w.at("05:00"), json!({"lat": 35.6}), raw)
        .await;
    let (_path_tmp, path) = export_once(&w.pool).await;
    // サーバも DB も止めた状態にする
    w.pool.close().await;
    let mut z = ZipArchive::new(std::fs::File::open(&path).unwrap()).unwrap();
    let mut found = None;
    for i in 0..z.len() {
        let name = z.name_for_index(i).unwrap().to_string();
        let mut f = z.by_name_decrypt(&name, PASS.as_bytes()).unwrap();
        if name == "README.txt" {
            continue;
        }
        if name == "manifest.json" {
            // 目録は整形した 1 つの JSON
            let mut body = String::new();
            f.read_to_string(&mut body).unwrap();
            serde_json::from_str::<Value>(&body).unwrap();
            continue;
        }
        for line in BufReader::new(f).lines() {
            let line = line.unwrap();
            let v: Value = serde_json::from_str(&line)
                .unwrap_or_else(|e| panic!("{name} の行が JSON でない: {e}"));
            if name == "events.jsonl" && v["id"] == id.to_string() {
                found = Some(v);
            }
        }
    }
    assert_eq!(found.unwrap()["raw"].as_str().unwrap(), raw);
}

// ---------------------------------------------------------------------------
// 5.4 除く範囲をファイルで確かめる
// ---------------------------------------------------------------------------

// Scenario: 削除の印の付いた記録は出ない
#[tokio::test]
async fn export_file_excludes_marked_records() {
    let w = World::new().await;
    let stay = w.erased_stay(&w.at("01:00"), &w.at("02:00"), &[]).await;
    let pos = w
        .put(
            "c01-location",
            &w.at("01:30"),
            json!({"lat": 35.0, "lon": 139.0}),
            "{}",
        )
        .await;
    w.mark_deleted(pos, "stay:cascade").await;
    assert!(exported(&w, &[stay, pos]).await.is_empty());
}

// Scenario: 消した滞在の時間に入る、印の無い別のソースの位置は出ない
#[tokio::test]
async fn export_file_excludes_unmarked_photo_position_inside() {
    let w = World::new().await;
    w.erased_stay(&w.at("01:00"), &w.at("02:00"), &[]).await;
    let photo = w
        .put(
            "c01-photo",
            &w.at("01:30"),
            json!({"lat": 35.0, "lon": 139.0}),
            "{}",
        )
        .await;
    assert!(exported(&w, &[photo]).await.is_empty());
}

// Scenario: 消した時間をまたぐ、別のソースの幅を持つ緯度経度つきの記録は出ない
#[tokio::test]
async fn export_file_excludes_wide_record_crossing_the_erased_time() {
    let w = World::new().await;
    w.erased_stay(&w.at("01:00"), &w.at("02:00"), &[]).await;
    let source = testdb::source(&w.pool, "wide", 3600).await;
    let crossing = w
        .put(
            &source,
            &w.at("00:30"),
            json!({"end_time": w.at("03:00"), "lat": 35.0}),
            "{}",
        )
        .await;
    let before = w
        .put(
            &source,
            &w.at("00:10"),
            json!({"end_time": w.at("00:20"), "lat": 35.0}),
            "{}",
        )
        .await;
    assert_eq!(
        exported(&w, &[crossing, before]).await,
        BTreeSet::from([before])
    );
}

// Scenario: 終わりをミリ秒の数字で持つ記録も、消した時間をまたげば出ない
#[tokio::test]
async fn export_file_excludes_end_time_in_epoch_millis() {
    let w = World::new().await;
    w.erased_stay(&w.at("01:00"), &w.at("02:00"), &[]).await;
    let source = testdb::source(&w.pool, "millis", 3600).await;
    let end_ms = w
        .day
        .and_hms_opt(3, 0, 0)
        .unwrap()
        .and_utc()
        .timestamp_millis();
    let crossing = w
        .put(
            &source,
            &w.at("00:30"),
            json!({"end_time": end_ms.to_string()}),
            "{}",
        )
        .await;
    assert!(exported(&w, &[crossing]).await.is_empty());
}

// Scenario: 長さで幅を持つ記録が消した時間に重なれば出ない
#[tokio::test]
async fn export_file_excludes_record_with_a_duration_overlapping() {
    let w = World::new().await;
    w.erased_stay(&w.at("01:00"), &w.at("02:00"), &[]).await;
    let visit = w
        .put(
            "c02-browser-history",
            &w.at("00:50"),
            json!({"visit_duration_us": 1_200_000_000i64}),
            MAP,
        )
        .await;
    let early = w
        .put(
            "c02-browser-history",
            &w.at("00:20"),
            json!({"visit_duration_us": 1_200_000_000i64}),
            MAP,
        )
        .await;
    assert_eq!(exported(&w, &[visit, early]).await, BTreeSet::from([early]));
}

// Scenario: 終わりを持たない記録は、次の記録までの幅で重なれば出ない
#[tokio::test]
async fn export_file_excludes_record_without_end_by_the_width_to_the_next() {
    let w = World::new().await;
    w.erased_stay(&w.at("01:00"), &w.at("02:00"), &[]).await;
    let a = w.put("c02-window", &w.at("00:10"), json!({}), MAP).await;
    let b = w.put("c02-window", &w.at("00:40"), json!({}), MAP).await;
    let c = w
        .put("c02-window", &w.at("02:30"), json!({}), "plain")
        .await;
    assert_eq!(exported(&w, &[a, b, c]).await, BTreeSet::from([a, c]));
}

// Scenario: 消した滞在の前の版が持っていた広い時間も、消した時間に数える
#[tokio::test]
async fn export_file_excludes_inside_the_wider_earlier_version_of_the_erased_stay() {
    let w = World::new().await;
    // 消した滞在は DB に残り続ける。他のテストの帯に被らないよう昼に置く（export_excluded_tests と同じ）
    w.erased_stay(
        &w.at("13:00"),
        &w.at("14:00"),
        &[(&w.at("12:00"), &w.at("14:00"))],
    )
    .await;
    let photo = w
        .put(
            "c01-photo",
            &w.at("12:30"),
            json!({"lat": 35.0, "lon": 139.0}),
            "{}",
        )
        .await;
    assert!(exported(&w, &[photo]).await.is_empty());
}

// Scenario: 別の利用者の識別子で入った同じ時間の位置も出ない
#[tokio::test]
async fn export_file_excludes_another_users_position_in_the_same_time() {
    let w = World::new().await;
    w.erased_stay(&w.at("01:00"), &w.at("02:00"), &[]).await;
    let other = testdb::user();
    let pos = w
        .put_as(
            other,
            "c01-photo",
            "phone",
            &w.at("01:30"),
            json!({"lat": 35.0, "lon": 139.0}),
            "{}",
        )
        .await;
    assert!(exported(&w, &[pos]).await.is_empty());
}

// Scenario: 生きている滞在の前の版が消した時間に重なれば出ない
#[tokio::test]
async fn export_file_excludes_the_earlier_version_of_a_live_stay_overlapping() {
    let w = World::new().await;
    w.erased_stay(&w.at("01:00"), &w.at("02:00"), &[]).await;
    let live = w.stay(&w.before("23:00"), &w.at("00:50")).await;
    let old = w
        .put_version(
            live,
            1,
            &w.before("23:00"),
            json!({"end": w.at("03:00"), "lat": 35.0, "lon": 139.0}),
        )
        .await;
    let (_path_tmp, path) = export_once(&w.pool).await;
    assert_eq!(
        present(&path, "events.jsonl", &[live]),
        BTreeSet::from([live])
    );
    assert!(present(&path, "event_versions.jsonl", &[old]).is_empty());
}

// Scenario: 消した滞在の時間に入っても、緯度経度を持たない記録は出る
#[tokio::test]
async fn export_file_excludes_keeps_a_record_without_coordinates_inside() {
    let w = World::new().await;
    w.erased_stay(&w.at("01:00"), &w.at("02:00"), &[]).await;
    let win = w
        .put(
            "c02-window",
            &w.at("01:30"),
            json!({"title": "memo"}),
            "memo",
        )
        .await;
    assert_eq!(exported(&w, &[win]).await, BTreeSet::from([win]));
}

// Scenario: 消した滞在の時間の外の位置は出る
#[tokio::test]
async fn export_file_excludes_keeps_a_position_outside_the_erased_time() {
    let w = World::new().await;
    w.erased_stay(&w.at("01:00"), &w.at("02:00"), &[]).await;
    let pos = w
        .put(
            "c01-location",
            &w.at("02:01"),
            json!({"lat": 35.0, "lon": 139.0}),
            "{}",
        )
        .await;
    assert_eq!(exported(&w, &[pos]).await, BTreeSet::from([pos]));
}

// Scenario: 緯度経度を持たないと決まっていないソースは、消した時間の中では出ない
#[tokio::test]
async fn export_file_excludes_a_source_not_known_to_lack_coordinates() {
    let w = World::new().await;
    w.erased_stay(&w.at("01:00"), &w.at("02:00"), &[]).await;
    let source = testdb::source(&w.pool, "newsource", 3600).await;
    let rec = w
        .put(&source, &w.at("01:30"), json!({"note": "x"}), "x")
        .await;
    assert!(exported(&w, &[rec]).await.is_empty());
}

// Scenario: 緯度経度を持たないソースでも、内容に緯度経度があれば消した時間の中では出ない
#[tokio::test]
async fn export_file_excludes_payload_keys_even_for_sources_without_coordinates() {
    let w = World::new().await;
    w.erased_stay(&w.at("01:00"), &w.at("02:00"), &[]).await;
    let a = w
        .put("c02-window", &w.at("01:30"), json!({"lng": 139.0}), "x")
        .await;
    let b = w
        .put(
            "c02-window",
            &w.at("01:31"),
            json!({"a": {"b": {"latitude": 35.0}}}),
            "x",
        )
        .await;
    let c = w
        .put(
            "c02-window",
            &w.at("01:32"),
            json!({"LatitudeE7": 356_812_000}),
            "x",
        )
        .await;
    let d = w
        .put("c02-window", &w.at("01:33"), json!({"latlng": 1}), "x")
        .await;
    assert_eq!(exported(&w, &[a, b, c, d]).await, BTreeSet::from([d]));
}

// Scenario: 原文に地図の座標を持つウィンドウの記録は、消した時間の中では出ない
#[tokio::test]
async fn export_file_excludes_window_records_with_map_coordinates_in_raw() {
    let w = World::new().await;
    w.erased_stay(&w.at("01:00"), &w.at("02:00"), &[]).await;
    let a = w.put("c02-window", &w.at("01:30"), json!({}), MAP).await;
    let b = w
        .put(
            "c02-window",
            &w.at("01:31"),
            json!({}),
            "!3d35.6812!4d139.7671",
        )
        .await;
    assert!(exported(&w, &[a, b]).await.is_empty());
}

// Scenario: 短い小数・符号化した区切り・度分秒・plus code の座標も、消した時間の中では出ない
#[tokio::test]
async fn export_file_excludes_short_encoded_dms_and_pluscode_coordinates() {
    let w = World::new().await;
    w.erased_stay(&w.at("01:00"), &w.at("02:00"), &[]).await;
    let raws = [
        "https://maps.google.com/?ll=35.68,139.76",
        "https://www.google.com/maps?q=35.68%2C139.76",
        "35°40'52.3\"N 139°46'00.2\"E",
        "MQ7X+2V 千代田区",
        "https://www.openstreetmap.org/#map=15/35.68/139.76",
        "https://example.com/?lat=35.68&lon=139.76",
        "139.7671,35.6812",
    ];
    let mut ids = Vec::new();
    for (i, raw) in raws.iter().enumerate() {
        ids.push(
            w.put(
                "c02-window",
                &w.at(&format!("01:{:02}", 30 + i)),
                json!({}),
                raw,
            )
            .await,
        );
    }
    assert!(exported(&w, &ids).await.is_empty());
}

// Scenario: 親が消した場面の規則で除かれると、前の版も出ない
#[tokio::test]
async fn export_file_excludes_the_versions_of_an_excluded_parent() {
    let w = World::new().await;
    w.erased_stay(&w.at("01:00"), &w.at("02:00"), &[]).await;
    let parent = w
        .put(
            "c01-photo",
            &w.at("01:30"),
            json!({"lat": 35.0, "lon": 139.0}),
            "{}",
        )
        .await;
    let version = w
        .put_version(
            parent,
            1,
            &w.at("10:00"),
            json!({"lat": 35.0, "lon": 139.0}),
        )
        .await;
    let outside = w
        .put(
            "c01-photo",
            &w.at("05:00"),
            json!({"lat": 35.0, "lon": 139.0}),
            "{}",
        )
        .await;
    let outside_v = w
        .put_version(
            outside,
            1,
            &w.at("05:00"),
            json!({"lat": 35.0, "lon": 139.0}),
        )
        .await;
    let (_path_tmp, path) = export_once(&w.pool).await;
    assert_eq!(
        present(&path, "events.jsonl", &[parent, outside]),
        BTreeSet::from([outside])
    );
    assert_eq!(
        present(&path, "event_versions.jsonl", &[version, outside_v]),
        BTreeSet::from([outside_v])
    );
}

// Scenario: 台帳と導出の表は書き出しに入らない
#[tokio::test]
async fn export_file_excludes_ledgers_and_derived_tables() {
    let w = World::new().await;
    w.erased_stay(&w.at("01:00"), &w.at("02:00"), &[]).await;
    let (__first_tmp, _) = export_once(&w.pool).await;
    let (_second_tmp, second) = export_once(&w.pool).await;
    let z = open(&second);
    let got: BTreeSet<String> = z.file_names().map(String::from).collect();
    assert_eq!(got, ENTRIES.iter().map(|s| (*s).to_string()).collect());
}

// Scenario: 除いた行の件数はどこにも出ない
#[tokio::test]
async fn export_file_excludes_without_leaving_a_count() {
    let w = World::new().await;
    w.erased_stay(&w.at("01:00"), &w.at("02:00"), &[]).await;
    w.put(
        "c01-photo",
        &w.at("01:30"),
        json!({"lat": 35.0, "lon": 139.0}),
        "{}",
    )
    .await;
    let (_path_tmp, path) = export_once(&w.pool).await;
    let readme = text(&path, "README.txt");
    for line in readme
        .lines()
        .filter(|l| l.contains("削除") || l.contains('除'))
    {
        assert!(
            !line.chars().any(|c| c.is_ascii_digit()),
            "除いた件数のような数字がある: {line}"
        );
    }
    let manifest: Value = serde_json::from_str(&text(&path, "manifest.json")).unwrap();
    fn walk(v: &Value, path: &str, bad: &mut Vec<String>) {
        match v {
            Value::Object(m) => {
                for (k, v) in m {
                    let lower = k.to_lowercase();
                    if [
                        "delet", "exclud", "erase", "remov", "omit", "skip", "削除", "除",
                    ]
                    .iter()
                    .any(|w| lower.contains(w))
                    {
                        bad.push(format!("{path}.{k}"));
                    }
                    walk(v, &format!("{path}.{k}"), bad);
                }
            }
            Value::Array(a) => a.iter().for_each(|v| walk(v, path, bad)),
            _ => {}
        }
    }
    let mut bad = Vec::new();
    walk(&manifest, "", &mut bad);
    assert!(bad.is_empty(), "件数の欄らしい鍵がある: {bad:?}");
}

// Scenario: 書き出す前の件数は書き出した行の数と一致する
#[tokio::test]
async fn export_file_excludes_the_summary_count_equals_the_exported_rows() {
    let w = World::new().await;
    w.erased_stay(&w.at("01:00"), &w.at("02:00"), &[]).await;
    let source = testdb::source(&w.pool, "summary5", 3600).await;
    for (hhmm, end) in [("03:00", "03:01"), ("04:00", "04:01"), ("05:00", "05:01")] {
        w.put(&source, &w.at(hhmm), json!({"end": w.at(end)}), "x")
            .await;
    }
    // 消した時間の中の 1 件は数えず、書かない
    w.put(&source, &w.at("01:30"), json!({"end": w.at("01:31")}), "x")
        .await;
    let _g = EXPORT_SERIAL.lock().await;
    let app = crate::App::for_test(w.pool.clone(), "test-token-0123456789abcdef");
    let mut h = axum::http::HeaderMap::new();
    h.insert(
        "authorization",
        "Bearer test-token-0123456789abcdef".parse().unwrap(),
    );
    let summary = crate::export_summary_get(axum::extract::State(app), h)
        .await
        .unwrap()
        .0;
    let mine = summary
        .sources
        .iter()
        .find(|s| s.logical_source == source)
        .unwrap();
    let summary_tmp = tmp_dir("summary");
    let path = export_into(
        &w.pool,
        ExportConfig::with_dir(summary_tmp.path().to_path_buf()),
    )
    .await;
    let rows = scan(&path, "events.jsonl", &w.user.to_string());
    let in_file = rows
        .iter()
        .filter(|r| r["logical_source"] == source.as_str())
        .count();
    assert_eq!(in_file as i64, mine.count);
    assert_eq!(in_file, 3);
}

// ---------------------------------------------------------------------------
// 5.5 写しの時点
// ---------------------------------------------------------------------------

/// 最初の項目（記録）を書き始める前、つまり写しを読み始めた後に `act` を 1 回だけ走らせる。
fn at_first_file<F, Fut>(act: F) -> Hooks
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    let act = Arc::new(act);
    Hooks::none().with_before_file(move |name| {
        let act = act.clone();
        Box::pin(async move {
            if name == "events.jsonl" {
                act().await;
            }
        })
    })
}

/// 取り込みの口（`/ingest`）を通して記録を 1 件入れ、受理されたことを確かめて id を返す。
async fn ingest_through_the_door(w: &World, hhmm: &str) -> Uuid {
    const TOKEN: &str = "test-token-0123456789abcdef";
    let app = crate::App::for_test(w.pool.clone(), TOKEN);
    let mut headers = axum::http::HeaderMap::new();
    headers.insert("authorization", format!("Bearer {TOKEN}").parse().unwrap());
    let id = Uuid::new_v4();
    let body = json!([{
        "id": id, "user_id": w.user, "logical_source": "c02-window", "external_id": null,
        "device_id": "pc", "origin": "collected", "event_time": w.at(hhmm),
        "tz_offset_min": 540, "tz_id": "Asia/Tokyo", "schema_version": 1,
        "raw": "late", "payload": {"t": "late"},
    }]);
    let (code, axum::Json(res)) =
        crate::ingest(axum::extract::State(app), headers, axum::Json(body))
            .await
            .expect("取り込み口");
    assert_eq!(code, axum::http::StatusCode::OK);
    assert!(res.len() == 1 && res[0].accepted, "取り込みは受理される");
    id
}

// Scenario: 書き出しの途中で届いた記録はその書き出しに入らない
#[tokio::test]
async fn export_snapshot_record_arriving_midway_is_not_in_this_export() {
    let _g = EXPORT_SERIAL.lock().await;
    let w = Arc::new(World::new().await);
    let late: Arc<Mutex<Option<Uuid>>> = Arc::default();
    let (w2, slot) = (w.clone(), late.clone());
    let late_tmp = tmp_dir("late");
    let mut cfg = ExportConfig::with_dir(late_tmp.path().to_path_buf());
    cfg.hooks = at_first_file(move || {
        let (w, slot) = (w2.clone(), slot.clone());
        async move {
            *slot.lock().unwrap() = Some(ingest_through_the_door(&w, "05:00").await);
        }
    });
    let first = export_into(&w.pool, cfg).await;
    let id = late.lock().unwrap().unwrap();
    assert!(present(&first, "events.jsonl", &[id]).is_empty());
    let late2_tmp = tmp_dir("late2");
    let second = export_into(
        &w.pool,
        ExportConfig::with_dir(late2_tmp.path().to_path_buf()),
    )
    .await;
    assert_eq!(
        present(&second, "events.jsonl", &[id]),
        BTreeSet::from([id])
    );
}

// Scenario: 書き出しの途中で付いた削除の印はその書き出しに反映しない
#[tokio::test]
async fn export_snapshot_deletion_midway_is_not_reflected_in_this_export() {
    let _g = EXPORT_SERIAL.lock().await;
    let w = Arc::new(World::new().await);
    let stay = w.stay(&w.at("05:00"), &w.at("06:00")).await;
    let w2 = w.clone();
    let del_tmp = tmp_dir("del");
    let mut cfg = ExportConfig::with_dir(del_tmp.path().to_path_buf());
    cfg.hooks = at_first_file(move || {
        let w = w2.clone();
        async move { w.mark_deleted(stay, "user").await }
    });
    let first = export_into(&w.pool, cfg).await;
    assert_eq!(
        present(&first, "events.jsonl", &[stay]),
        BTreeSet::from([stay])
    );
    let del2_tmp = tmp_dir("del2");
    let second = export_into(
        &w.pool,
        ExportConfig::with_dir(del2_tmp.path().to_path_buf()),
    )
    .await;
    assert!(present(&second, "events.jsonl", &[stay]).is_empty());
}

// Scenario: 書き出しのファイルどうしが食い違わない
#[tokio::test]
async fn export_snapshot_files_do_not_contradict_each_other() {
    let _g = EXPORT_SERIAL.lock().await;
    let w = Arc::new(World::new().await);
    let parent = w
        .put("c02-window", &w.at("05:00"), json!({"t": 1}), "x")
        .await;
    w.put_version(parent, 1, &w.at("05:00"), json!({"t": 0}))
        .await;
    let doomed = w
        .put("c02-window", &w.at("05:10"), json!({"t": 1}), "x")
        .await;
    w.put_version(doomed, 1, &w.at("05:10"), json!({"t": 0}))
        .await;
    let w2 = w.clone();
    let consistent_tmp = tmp_dir("consistent");
    let mut cfg = ExportConfig::with_dir(consistent_tmp.path().to_path_buf());
    cfg.hooks = at_first_file(move || {
        let w = w2.clone();
        async move {
            // 取り込みと削除が続いている
            let fresh = w
                .put("c02-window", &w.at("05:20"), json!({"t": 1}), "x")
                .await;
            w.put_version(fresh, 1, &w.at("05:20"), json!({"t": 0}))
                .await;
            w.mark_deleted(doomed, "user").await;
        }
    });
    let path = export_into(&w.pool, cfg).await;
    let events = all_rows(&path, "events.jsonl");
    let ids: BTreeSet<&str> = events.iter().map(|r| r["id"].as_str().unwrap()).collect();
    let sources = all_rows(&path, "sources.jsonl");
    let registered: BTreeSet<&str> = sources
        .iter()
        .map(|r| r["logical_source"].as_str().unwrap())
        .collect();
    for v in all_rows(&path, "event_versions.jsonl") {
        assert!(
            ids.contains(v["event_id"].as_str().unwrap()),
            "親が記録のファイルに無い"
        );
    }
    for e in &events {
        assert!(
            registered.contains(e["logical_source"].as_str().unwrap()),
            "登録簿のファイルに無いソース"
        );
    }
    assert!(
        ids.contains(doomed.to_string().as_str()),
        "写しの時点では生きていた記録"
    );
}

// ---------------------------------------------------------------------------
// 5.6 量
// ---------------------------------------------------------------------------

/// このプロセスの最大の常駐メモリ（KiB）。読めなければ 0。
fn max_rss_kib() -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find_map(|l| l.strip_prefix("VmHWM:"))
                .and_then(|v| v.trim().trim_end_matches("kB").trim().parse().ok())
        })
        .unwrap_or(0)
}

/// 既定は 2 万行。1 年分（35 万行）は `ASHIATO_EXPORT_VOLUME_ROWS=350000` を渡し、使い捨ての DB で走らせる（R36）。
/// `ASHIATO_EXPORT_VOLUME_ERASED` は消した滞在の数 M（既定は `1`、`0` なら置かない。CH02 design D8・CH04 design D14）。
/// M ≥ 2 のときは記録を M 日に等分して置き（1 日 N / M 行）、各日の 01:00–02:00 を消した滞在にする。
#[tokio::test]
async fn export_volume_one_run_reports_time_and_memory() {
    let rows: i64 = std::env::var("ASHIATO_EXPORT_VOLUME_ROWS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(20_000);
    let erased_count: i64 = std::env::var("ASHIATO_EXPORT_VOLUME_ERASED")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);
    let erased = erased_count > 0;
    let days = erased_count.max(1);
    let per_day = (rows + days - 1) / days;
    let _g = EXPORT_SERIAL.lock().await;
    let w = World::new().await;
    // 既定では消した滞在（01:00–02:00）が 1 件ある。行の 100 に 1 つは座標を持ち、00:00–03:00 に散らすので、
    // その 3 分の 2 が消した時間より前に始まり、除く判定の重い経路（次の記録を引く幅・重なり）を通る。
    // 座標を持つ行は、この試験の database の中で書き出しのたびに判定し直されるので、割合は小さく保つ。
    if days <= 1 {
        if erased {
            w.erased_stay(&w.at("01:00"), &w.at("02:00"), &[]).await;
        }
    } else {
        for k in 0..days {
            let day = w.day + chrono::Duration::days(k);
            w.erased_stay(
                &format!("{day}T01:00:00Z"),
                &format!("{day}T02:00:00Z"),
                &[],
            )
            .await;
        }
    }
    sqlx::query(
        "INSERT INTO core.event
           (id, user_id, logical_source, device_id, origin, event_time, tz_offset_min, tz_id,
            schema_version, content_hash, raw, payload)
         SELECT gen_random_uuid(), $1, 'c02-window', 'pc', 'collected',
                $2::timestamptz + ((g - 1) / $4::bigint) * interval '1 day'
                  + (g % 10800) * interval '1 second', 540, 'Asia/Tokyo', 1,
                md5(g::text || $1::text), 'volume row ' || g,
                CASE WHEN g % 100 = 0 THEN jsonb_build_object('i', g, 'lat', 35.0, 'lon', 139.0)
                     ELSE jsonb_build_object('i', g) END
           FROM generate_series(1, $3::bigint) g",
    )
    .bind(w.user)
    .bind(w.at("00:00"))
    .bind(rows)
    .bind(per_day.max(1))
    .execute(&w.pool)
    .await
    .unwrap();
    // 件数の数え（`GET /export/summary`。D13 の反転条件「3 秒以上」）も同じ断片を通るので、同じ量で測る
    let counted = std::time::Instant::now();
    export::read_summary(&w.pool).await.unwrap();
    let summary_seconds = counted.elapsed().as_secs_f64();
    let started = std::time::Instant::now();
    let volume_tmp = tmp_dir("volume");
    let path = export_into(
        &w.pool,
        ExportConfig::with_dir(volume_tmp.path().to_path_buf()),
    )
    .await;
    let seconds = started.elapsed().as_secs_f64();
    let rss = max_rss_kib();
    let mine = scan(&path, "events.jsonl", &w.user.to_string()).len() as i64;
    // 値は出さない（件数・時間・メモリだけ）
    tracing::info!(
        kind = "export",
        rows,
        seconds,
        summary_seconds,
        max_rss_kib = rss,
        "書き出しの量"
    );
    eprintln!(
        "kind=export rows={rows} seconds={seconds:.1} summary_seconds={summary_seconds:.1} max_rss_kib={rss}"
    );
    // 落ちるのは座標を持つ行（100 に 1 つ）のうち消した時間に重なるものだけ。座標を持たない行は残る
    let with_coords = rows / 100;
    if erased {
        assert!(
            mine < rows,
            "消した時間の中の座標の行が残っている: {mine}/{rows}"
        );
    } else {
        assert_eq!(mine, rows, "消した滞在が無いのに行が落ちている");
    }
    assert!(
        mine > rows - with_coords,
        "座標を持たない行が落ちている: {mine}/{rows}"
    );
}

// Scenario: 版の記録があっても、全記録の書き出しは始まる
#[tokio::test]
async fn st48_export_starts_with_ledger() {
    // db-ok: CH04 の testdb::isolated() がこのブランチにまだ無い。CH04 が main に入った後に isolated() へ替える
    let pool = testdb::pool().await;
    let recorded: i64 = sqlx::query_scalar("SELECT count(*) FROM migration_ledger.applied")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(recorded > 0, "版の記録のある DB で始める");
    // 「分類の無い表」で断られると unwrap が落ちる
    let (_path_tmp, path) = export_once(&pool).await;
    let z = open(&path);
    let bad: Vec<&str> = z
        .file_names()
        .filter(|n| n.contains("migration_ledger"))
        .collect();
    assert!(bad.is_empty(), "版の記録が書き出しに入っている: {bad:?}");
}

// ---------------------------------------------------------------------------
// ST29 衛星の登録簿と資格情報
// ---------------------------------------------------------------------------

/// 衛星を 1 つ登録する（試験ごとの接尾で隔離する）。`removed` なら外した時刻を付ける。
async fn put_satellite(pool: &sqlx::PgPool, identifier: &str, name: &str, removed: bool) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO core.satellite (id, identifier, name, registered_at, removed_at)
         VALUES ($1, $2, $3, now(), CASE WHEN $4 THEN now() END)",
    )
    .bind(id)
    .bind(identifier)
    .bind(name)
    .bind(removed)
    .execute(pool)
    .await
    .unwrap();
    id
}

// Scenario: 衛星の登録簿は外した衛星も含めて書き出しに出る
#[tokio::test]
async fn export_satellites_registry_includes_removed_satellites() {
    // db-ok: CH04 の testdb::isolated() がこのブランチにまだ無い。CH04 が main に入った後に isolated() へ替える
    let pool = testdb::pool().await;
    let tag = &Uuid::new_v4().simple().to_string()[..8];
    let identifier = format!("dev.t{tag}.booklog");
    put_satellite(&pool, &identifier, &format!("読書ログ{tag}"), true).await;
    let (_path_tmp, path) = export_once(&pool).await;
    let rows = all_rows(&path, "satellites.jsonl");
    let identifiers: BTreeSet<&str> = rows
        .iter()
        .map(|r| r["identifier"].as_str().unwrap())
        .collect();
    for builtin in ["外部 AI", "ローカル AI"] {
        assert!(
            rows.iter().any(|r| r["name"] == builtin),
            "{builtin} の行が無い: {identifiers:?}"
        );
    }
    let mine = rows
        .iter()
        .find(|r| r["identifier"] == identifier.as_str())
        .expect("外した衛星の行が無い");
    assert!(mine["removed_at"].is_string(), "{mine}");
    assert_eq!(
        keys(mine),
        table_columns(&pool, "satellite").await,
        "登録簿の行は表の全列を持つ"
    );
}

// Scenario: 衛星の資格情報は書き出しに入らない
#[tokio::test]
async fn export_satellites_credentials_are_not_exported() {
    // db-ok: CH04 の testdb::isolated() がこのブランチにまだ無い。CH04 が main に入った後に isolated() へ替える
    let pool = testdb::pool().await;
    let tag = &Uuid::new_v4().simple().to_string()[..8];
    let sat = put_satellite(
        &pool,
        &format!("dev.t{tag}.cred"),
        &format!("資格{tag}"),
        false,
    )
    .await;
    // 値そのものは持たないので、ハッシュ（16 進）が zip のどこにも出ないことを見る。
    let mut hashes = Vec::new();
    for _ in 0..2 {
        let hash: Vec<u8> = Uuid::new_v4().as_bytes().repeat(2);
        sqlx::query(
            "INSERT INTO core.satellite_credential (id, satellite_id, token_sha256, issued_at)
             VALUES ($1, $2, $3, now())",
        )
        .bind(Uuid::new_v4())
        .bind(sat)
        .bind(&hash)
        .execute(&pool)
        .await
        .unwrap();
        hashes.push(hash);
    }
    let (_path_tmp, path) = export_once(&pool).await;
    let names_in_zip: Vec<String> = open(&path).file_names().map(String::from).collect();
    for name in &names_in_zip {
        let body = text(&path, name);
        for h in &hashes {
            let hex: String = h.iter().fold(String::new(), |mut acc, b| {
                use std::fmt::Write as _;
                let _ = write!(acc, "{b:02x}");
                acc
            });
            assert!(
                !body.contains(&hex) && !body.contains(&format!("\\x{hex}")),
                "{name} に資格情報のハッシュがある"
            );
        }
    }
    assert!(
        !names_in_zip.iter().any(|n| n.contains("credential")),
        "{names_in_zip:?}"
    );
    let readme = text(&path, "README.txt");
    assert!(
        readme.contains("表 satellite_credential: 衛星の資格情報"),
        "説明書の入らないものに無い"
    );
}

// Scenario: ST29 が足した表はすべて分類されている
#[tokio::test]
async fn export_satellite_tables_and_columns_are_classified() {
    // db-ok: CH04 の testdb::isolated() がこのブランチにまだ無い。CH04 が main に入った後に isolated() へ替える
    let pool = testdb::pool().await;
    let inv = export::read_inventory(&pool).await.unwrap();
    assert!(matches!(
        export::class_of("satellite"),
        Some(export::Class::Include {
            file: "satellites.jsonl",
            ..
        })
    ));
    assert!(matches!(
        export::class_of("satellite_credential"),
        Some(export::Class::Exclude { .. })
    ));
    for t in ["satellite", "satellite_credential"] {
        assert!(inv.tables.iter().any(|n| n == t), "{t} が一覧に無い");
    }
    let bad = export::unclassified(&inv);
    assert!(bad.is_empty(), "分類の無いもの: {bad:?}");
}
