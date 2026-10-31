// SPDX-License-Identifier: AGPL-3.0-only
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
