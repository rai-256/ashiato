// SPDX-License-Identifier: AGPL-3.0-only
//! SQLite 履歴 DB の写しを読み取る。

/// Chromium の Windows epoch（1601-01-01）からのマイクロ秒を UTC へ換算する。
pub fn chromium_micros(value: i64) -> Option<chrono::DateTime<chrono::Utc>> {
    if value < 0 {
        return None;
    }
    let epoch = chrono::DateTime::parse_from_rfc3339("1601-01-01T00:00:00Z")
        .ok()?
        .with_timezone(&chrono::Utc);
    epoch.checked_add_signed(chrono::Duration::microseconds(value))
}

/// Firefox の Unix epoch からのマイクロ秒を UTC へ換算する。
pub fn firefox_micros(value: i64) -> Option<chrono::DateTime<chrono::Utc>> {
    if value < 0 {
        return None;
    }
    chrono::DateTime::<chrono::Utc>::UNIX_EPOCH
        .checked_add_signed(chrono::Duration::microseconds(value))
}

/// SQLite から読んだ、まだ送信形式へ変換していない訪問。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadVisit {
    pub id: i64,
    /// DB の訪問時刻そのもの（Chromium は 1601 年起点、Firefox は 1970 年起点のマイクロ秒）
    pub visit_time_raw: i64,
    pub url: String,
    pub title: Option<String>,
    pub at: chrono::DateTime<chrono::Utc>,
    pub transition: i64,
    pub from_visit: Option<i64>,
    pub opener_visit: Option<i64>,
    pub duration_us: Option<i64>,
    pub originator_cache_guid: Option<String>,
    pub originator_visit_id: Option<i64>,
    pub is_known_to_sync: Option<bool>,
}

/// 表にその列があれば `v.<列>`、無ければ `NULL`（古い版のブラウザの履歴 DB は列が少ない）。
fn column_or_null(
    conn: &rusqlite::Connection,
    table: &str,
    column: &str,
) -> anyhow::Result<String> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let has = stmt
        .query_map([], |r| r.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?
        .iter()
        .any(|name| name == column);
    Ok(if has {
        format!("v.{column}")
    } else {
        "NULL".to_owned()
    })
}

pub fn read_chromium(path: &std::path::Path) -> anyhow::Result<Vec<ReadVisit>> {
    let conn =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let col = |c| column_or_null(&conn, "visits", c);
    let sql = format!(
        "SELECT v.id,u.url,u.title,v.visit_time,v.transition,v.from_visit,{},{},{},{},{} \
         FROM visits v JOIN urls u ON u.id=v.url ORDER BY v.id",
        col("visit_duration")?,
        col("opener_visit")?,
        col("originator_cache_guid")?,
        col("originator_visit_id")?,
        col("is_known_to_sync")?,
    );
    let mut stmt = conn.prepare(&sql)?;
    let visits = stmt
        .query_map([], |r| {
            let raw: i64 = r.get(3)?;
            // 空の発生元は「この PC 自身の訪問」（他端末の訪問だけが印を持つ。design D9）
            let guid = r.get::<_, Option<String>>(8)?.filter(|g| !g.is_empty());
            let originator_visit_id = guid.as_ref().and(r.get::<_, Option<i64>>(9)?);
            Ok(ReadVisit {
                id: r.get(0)?,
                visit_time_raw: raw,
                url: r.get(1)?,
                title: r.get(2)?,
                at: chromium_micros(raw).ok_or(rusqlite::Error::InvalidQuery)?,
                transition: r.get(4)?,
                from_visit: nonzero(r.get::<_, Option<i64>>(5)?.unwrap_or(0)),
                opener_visit: nonzero(r.get::<_, Option<i64>>(7)?.unwrap_or(0)),
                duration_us: r.get(6)?,
                originator_cache_guid: guid,
                originator_visit_id,
                is_known_to_sync: r.get::<_, Option<i64>>(10)?.map(|f| f != 0),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(visits)
}

pub fn read_firefox(path: &std::path::Path) -> anyhow::Result<Vec<ReadVisit>> {
    let conn =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let mut stmt = conn.prepare("SELECT v.id,p.url,p.title,v.visit_date,v.visit_type,v.from_visit FROM moz_historyvisits v JOIN moz_places p ON p.id=v.place_id ORDER BY v.id")?;
    let visits = stmt
        .query_map([], |r| {
            let raw: i64 = r.get(3)?;
            Ok(ReadVisit {
                id: r.get(0)?,
                visit_time_raw: raw,
                url: r.get(1)?,
                title: r.get(2)?,
                at: firefox_micros(raw).ok_or(rusqlite::Error::InvalidQuery)?,
                transition: r.get(4)?,
                from_visit: nonzero(r.get::<_, Option<i64>>(5)?.unwrap_or(0)),
                opener_visit: None,
                duration_us: None,
                originator_cache_guid: None,
                originator_visit_id: None,
                is_known_to_sync: None,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(visits)
}

fn nonzero(value: i64) -> Option<i64> {
    (value != 0).then_some(value)
}

/// 開いている DB を直接触らず、一時の写しへ読み取り操作を閉じ込める（design D2）。
///
/// `<名前>` と、あれば `<名前>-journal` / `<名前>-wal` を一時ディレクトリへ写し、
/// 読み終えたらディレクトリごと消す（私的な内容を置き場に残さない）。
pub fn with_copy<T>(
    source: &std::path::Path,
    read: impl FnOnce(&std::path::Path) -> anyhow::Result<T>,
) -> anyhow::Result<T> {
    let dir = std::env::temp_dir().join(format!("ashiato-history-copy-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir)?;
    let result = copy_and_read(source, &dir, read);
    std::fs::remove_dir_all(&dir).ok();
    result
}

fn copy_and_read<T>(
    source: &std::path::Path,
    dir: &std::path::Path,
    read: impl FnOnce(&std::path::Path) -> anyhow::Result<T>,
) -> anyhow::Result<T> {
    let name = source
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("履歴 DB のファイル名が無い"))?;
    let copy = dir.join(name);
    std::fs::copy(source, &copy)?;
    for suffix in ["-journal", "-wal"] {
        let mut from = source.as_os_str().to_owned();
        from.push(suffix);
        let mut to = copy.as_os_str().to_owned();
        to.push(suffix);
        // 無いのが普通。あるのに写せなければ、写しが古い DB になるので失敗として返す
        if std::path::Path::new(&from).is_file() {
            std::fs::copy(&from, &to)?;
        }
    }
    read(&copy)
}

/// ブラウザの系統に合う読み手で、履歴 DB の写しから訪問を読む。
pub fn read_visits(
    browser: crate::history::locate::Browser,
    source: &std::path::Path,
) -> anyhow::Result<Vec<ReadVisit>> {
    with_copy(source, |copy| match browser {
        crate::history::locate::Browser::Firefox => read_firefox(copy),
        _ => read_chromium(copy),
    })
}

/// 履歴を読む区間が無いときでも、写しを開いて最小の問い合わせまで通す。
///
/// 生存信号を「読める」と報告する前の確認専用で、訪問行は読み取らない。
pub fn probe_readable(source: &std::path::Path) -> anyhow::Result<()> {
    with_copy(source, |copy| {
        let conn = rusqlite::Connection::open_with_flags(
            copy,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )?;
        conn.query_row("PRAGMA schema_version", [], |_| Ok(()))?;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn history_epoch_conversion() {
        assert_eq!(
            chromium_micros(0),
            Some(
                chrono::DateTime::parse_from_rfc3339("1601-01-01T00:00:00Z")
                    .expect("epoch")
                    .with_timezone(&chrono::Utc)
            )
        );
        assert_eq!(
            firefox_micros(0),
            Some(chrono::DateTime::<chrono::Utc>::UNIX_EPOCH)
        );
        assert_eq!(chromium_micros(-1), None);
        assert_eq!(firefox_micros(-1), None);
        // 起点の差（1601 → 1970）は 11,644,473,600 秒。マイクロ秒の端数は落とさない
        assert_eq!(
            chromium_micros(11_644_473_600_000_001),
            Some(chrono::DateTime::<chrono::Utc>::UNIX_EPOCH + chrono::Duration::microseconds(1))
        );
        assert_eq!(
            firefox_micros(1_758_153_600_000_001).map(|t| t.timestamp_subsec_micros()),
            Some(1)
        );
        // 範囲を超える値は None（panic しない）
        assert_eq!(chromium_micros(i64::MAX), None);
        assert_eq!(firefox_micros(i64::MAX), None);
    }

    #[test]
    fn history_read_chromium() {
        // Scenario: 同じページを 2 回訪問すると 2 件になる
        // Scenario: ページの題名と滞在時間が載る
        // Scenario: 遷移の種類とどこから来たかが残る
        // Scenario: URL のクエリとフラグメントが残る
        // Scenario: 履歴 DB の URL の文字列を補正しない
        let db = temp_db();
        let conn = rusqlite::Connection::open(&db).unwrap();
        conn.execute_batch("CREATE TABLE urls(id INTEGER PRIMARY KEY, url TEXT, title TEXT); CREATE TABLE visits(id INTEGER PRIMARY KEY, url INTEGER, visit_time INTEGER, transition INTEGER, from_visit INTEGER, visit_duration INTEGER, originator_cache_guid TEXT, originator_visit_id INTEGER);
          INSERT INTO urls VALUES(1, 'example.test/a?q=x#f', '題名'); INSERT INTO visits VALUES(1,1,1,3,0,7,NULL,NULL),(2,1,2,4,1,8,NULL,NULL);").unwrap();
        drop(conn);
        let v = read_chromium(&db).unwrap();
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].url, "example.test/a?q=x#f");
        assert_eq!(v[0].title.as_deref(), Some("題名"));
        assert_eq!(v[0].duration_us, Some(7));
        assert_eq!(v[0].transition, 3);
        assert_eq!(v[1].from_visit, Some(1));
        std::fs::remove_file(db).ok();
    }

    #[test]
    fn history_read_chromium_allows_null_duration() {
        // Scenario: 滞在時間が未記録の訪問も取り込める
        let db = temp_db();
        let conn = rusqlite::Connection::open(&db).unwrap();
        conn.execute_batch("CREATE TABLE urls(id INTEGER PRIMARY KEY, url TEXT, title TEXT); CREATE TABLE visits(id INTEGER PRIMARY KEY, url INTEGER, visit_time INTEGER, transition INTEGER, from_visit INTEGER, visit_duration INTEGER, originator_cache_guid TEXT, originator_visit_id INTEGER); INSERT INTO urls VALUES(1, 'https://example.test/a', NULL); INSERT INTO visits VALUES(1,1,1,3,0,NULL,NULL,NULL);").unwrap();
        drop(conn);
        let visits = read_chromium(&db).unwrap();
        assert_eq!(visits[0].duration_us, None);
        std::fs::remove_file(db).ok();
    }

    /// Scenario: 他の端末の訪問は発生元の印を持つ
    /// Scenario: PC 自身の訪問は発生元の印を持たない
    #[test]
    fn history_foreign_visits_are_read_from_chromium() {
        let db = temp_db();
        let conn = rusqlite::Connection::open(&db).unwrap();
        conn.execute_batch("CREATE TABLE urls(id INTEGER PRIMARY KEY, url TEXT, title TEXT); CREATE TABLE visits(id INTEGER PRIMARY KEY, url INTEGER, visit_time INTEGER, transition INTEGER, from_visit INTEGER, visit_duration INTEGER, originator_cache_guid TEXT, originator_visit_id INTEGER); INSERT INTO urls VALUES(1, 'https://example.test/a', '題名'); INSERT INTO visits VALUES(1,1,1,3,0,7,'other-pc',99),(2,1,2,3,0,7,NULL,NULL);").unwrap();
        drop(conn);
        let visits = read_chromium(&db).unwrap();
        assert_eq!(visits[0].originator_cache_guid.as_deref(), Some("other-pc"));
        assert_eq!(visits[0].originator_visit_id, Some(99));
        assert_eq!(visits[1].originator_cache_guid, None);
        assert_eq!(visits[1].originator_visit_id, None);
        std::fs::remove_file(db).ok();
    }

    #[test]
    fn history_read_firefox() {
        let db = temp_db();
        let conn = rusqlite::Connection::open(&db).unwrap();
        conn.execute_batch("CREATE TABLE moz_places(id INTEGER PRIMARY KEY, url TEXT, title TEXT); CREATE TABLE moz_historyvisits(id INTEGER PRIMARY KEY, place_id INTEGER, visit_date INTEGER, visit_type INTEGER, from_visit INTEGER);
          INSERT INTO moz_places VALUES(1, 'https://example.test/a?q=x#f', '題名'); INSERT INTO moz_historyvisits VALUES(1,1,1,2,0);").unwrap();
        drop(conn);
        let v = read_firefox(&db).unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].url, "https://example.test/a?q=x#f");
        std::fs::remove_file(db).ok();
    }
    #[test]
    fn history_copy_is_removed() {
        // Scenario: ブラウザが動いている間も取得できる
        let db = temp_db();
        let _open_browser_db = rusqlite::Connection::open(&db).unwrap();
        let copy = with_copy(&db, |p| {
            assert!(p.exists());
            Ok(p.to_owned())
        })
        .unwrap();
        assert!(!copy.exists());
        std::fs::remove_file(db).ok();
    }
    #[test]
    fn history_heartbeat_probes_when_not_read() {
        // Scenario: 区間に読みが無くても、開けるかを確かめてから報告する
        let db = temp_db();
        rusqlite::Connection::open(&db).unwrap();
        assert!(probe_readable(&db).is_ok());
        std::fs::remove_file(db).ok();
    }

    #[test]
    fn history_heartbeat_rejects_an_unreadable_database() {
        // Scenario: 履歴 DB が開けなければ取得できない状態として扱える
        let db = temp_db();
        std::fs::write(&db, "not a sqlite database").unwrap();
        assert!(probe_readable(&db).is_err());
        std::fs::remove_file(db).ok();
    }
    const CHROMIUM_FULL: &str = "CREATE TABLE urls(id INTEGER PRIMARY KEY, url TEXT, title TEXT); CREATE TABLE visits(id INTEGER PRIMARY KEY, url INTEGER, visit_time INTEGER, from_visit INTEGER, transition INTEGER, visit_duration INTEGER, opener_visit INTEGER, originator_cache_guid TEXT, originator_visit_id INTEGER, is_known_to_sync INTEGER);";

    #[test]
    fn history_read_chromium_opener_and_sync_flag() {
        // Scenario: 遷移の種類とどこから来たかが残る
        let db = temp_db();
        let conn = rusqlite::Connection::open(&db).unwrap();
        conn.execute_batch(CHROMIUM_FULL).unwrap();
        conn.execute_batch("INSERT INTO urls VALUES(1,'https://example.test/',NULL); INSERT INTO visits VALUES(1,1,10,0,1,5,0,'',0,0),(2,1,20,1,1,5,1,'',0,1);").unwrap();
        drop(conn);
        let v = read_chromium(&db).unwrap();
        assert_eq!(v[0].opener_visit, None);
        assert_eq!(v[0].is_known_to_sync, Some(false));
        assert_eq!(v[1].opener_visit, Some(1));
        assert_eq!(v[1].is_known_to_sync, Some(true));
        // 空の発生元は「他端末の訪問」ではない
        assert_eq!(v[0].originator_cache_guid, None);
        assert_eq!(v[0].originator_visit_id, None);
        std::fs::remove_file(db).ok();
    }

    #[test]
    fn history_read_chromium_old_schema_without_optional_columns() {
        // Scenario: 同じページを 2 回訪問すると 2 件になる
        let db = temp_db();
        let conn = rusqlite::Connection::open(&db).unwrap();
        conn.execute_batch("CREATE TABLE urls(id INTEGER PRIMARY KEY, url TEXT, title TEXT); CREATE TABLE visits(id INTEGER PRIMARY KEY, url INTEGER, visit_time INTEGER, from_visit INTEGER, transition INTEGER); INSERT INTO urls VALUES(1,'https://example.test/','t'); INSERT INTO visits VALUES(1,1,10,0,1);").unwrap();
        drop(conn);
        let v = read_chromium(&db).unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].duration_us, None);
        assert_eq!(v[0].originator_cache_guid, None);
        assert_eq!(v[0].is_known_to_sync, None);
        std::fs::remove_file(db).ok();
    }

    #[test]
    fn history_read_firefox_keeps_url_text_untouched() {
        // Scenario: 履歴 DB の URL の文字列を補正しない
        let db = temp_db();
        let conn = rusqlite::Connection::open(&db).unwrap();
        conn.execute_batch("CREATE TABLE moz_places(id INTEGER PRIMARY KEY, url TEXT, title TEXT); CREATE TABLE moz_historyvisits(id INTEGER PRIMARY KEY, place_id INTEGER, visit_date INTEGER, visit_type INTEGER, from_visit INTEGER); INSERT INTO moz_places VALUES(1,'HTTPS://Example.TEST/Path',NULL); INSERT INTO moz_historyvisits VALUES(1,1,5,1,0),(2,1,6,1,1);").unwrap();
        drop(conn);
        let v = read_firefox(&db).unwrap();
        assert_eq!(v.len(), 2, "同じページを 2 回訪問すると 2 件");
        assert_eq!(v[0].url, "HTTPS://Example.TEST/Path");
        assert_eq!(v[0].title, None);
        assert_eq!(v[1].from_visit, Some(1));
        assert_eq!(v[0].originator_cache_guid, None);
        std::fs::remove_file(db).ok();
    }

    #[test]
    fn history_copy_is_removed_with_wal_and_journal_and_reads_wal_rows() {
        // Scenario: ブラウザが動いている間も取得できる
        let dir = std::env::temp_dir().join(format!("ashiato-read-dir-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = dir.join("History");
        let open = rusqlite::Connection::open(&db).unwrap();
        open.execute_batch("PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0; CREATE TABLE t(x INTEGER); INSERT INTO t VALUES(7);").unwrap();
        // 開いたまま（ブラウザが動いている）。行は -wal にだけある
        assert!(dir.join("History-wal").exists());
        std::fs::write(dir.join("History-journal"), []).unwrap();
        let mut seen = Vec::new();
        let n: i64 = with_copy(&db, |copy| {
            let parent = copy.parent().unwrap();
            seen = std::fs::read_dir(parent)
                .unwrap()
                .map(|e| e.unwrap().path())
                .collect();
            let c = rusqlite::Connection::open_with_flags(
                copy,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
            )?;
            Ok(c.query_row("SELECT x FROM t", [], |r| r.get(0))?)
        })
        .unwrap();
        assert_eq!(n, 7);
        assert!(seen.len() >= 2, "journal / wal も写す: {seen:?}");
        for p in &seen {
            assert!(!p.exists(), "写しは消える: {}", p.display());
        }
        drop(open);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn history_read_visits_picks_reader_by_family() {
        // Scenario: 複数のブラウザと複数のプロファイルの履歴が全部入る
        let db = temp_db();
        let conn = rusqlite::Connection::open(&db).unwrap();
        conn.execute_batch("CREATE TABLE moz_places(id INTEGER PRIMARY KEY, url TEXT, title TEXT); CREATE TABLE moz_historyvisits(id INTEGER PRIMARY KEY, place_id INTEGER, visit_date INTEGER, visit_type INTEGER, from_visit INTEGER); INSERT INTO moz_places VALUES(1,'https://a.test/',NULL); INSERT INTO moz_historyvisits VALUES(1,1,5,1,0);").unwrap();
        drop(conn);
        let v = read_visits(crate::history::locate::Browser::Firefox, &db).unwrap();
        assert_eq!(v.len(), 1);
        std::fs::remove_file(db).ok();
    }

    #[test]
    fn history_foreign_visits_keep_pc_side_identifier_and_pc_mark() {
        // Scenario: 他の端末の訪問は発生元の印を持つ
        // Scenario: PC 自身の訪問は発生元の印を持たない
        let db = temp_db();
        let conn = rusqlite::Connection::open(&db).unwrap();
        conn.execute_batch(CHROMIUM_FULL).unwrap();
        conn.execute_batch("INSERT INTO urls VALUES(1,'https://example.test/','t'); INSERT INTO visits VALUES(1,1,10,0,1,5,0,'other-pc',99,1),(2,1,10,0,1,5,0,'',0,0);").unwrap();
        drop(conn);
        let b = crate::history::locate::Browser::Chrome;
        let v = read_visits(b, &db).unwrap();
        let foreign = crate::history::contract::Visit::from_read(b, "Default", &v[0]).unwrap();
        let local = crate::history::contract::Visit::from_read(b, "Default", &v[1]).unwrap();
        assert_eq!(
            foreign.payload.originator_cache_guid.as_deref(),
            Some("other-pc")
        );
        assert_eq!(foreign.payload.originator_visit_id, Some(99));
        assert_eq!(
            foreign.payload.visit_id,
            Some(1),
            "識別子の元は PC 側の番号"
        );
        assert_eq!(local.payload.originator_cache_guid, None);
        assert!(!local.raw.contains("originator"));
        std::fs::remove_file(db).ok();
    }

    fn temp_db() -> std::path::PathBuf {
        std::env::temp_dir().join(format!("ashiato-read-{}.sqlite", uuid::Uuid::new_v4()))
    }
}
