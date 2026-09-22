// SPDX-License-Identifier: AGPL-3.0-only
//! SQLite 履歴 DB の写しを読み取る。

/// Chromium の Windows epoch（1601-01-01）からのマイクロ秒を UTC へ換算する。
pub fn chromium_micros(value: i64) -> Option<chrono::DateTime<chrono::Utc>> {
    if value < 0 { return None; }
    let epoch = chrono::DateTime::parse_from_rfc3339("1601-01-01T00:00:00Z").ok()?.with_timezone(&chrono::Utc);
    epoch.checked_add_signed(chrono::Duration::microseconds(value))
}

/// Firefox の Unix epoch からのマイクロ秒を UTC へ換算する。
pub fn firefox_micros(value: i64) -> Option<chrono::DateTime<chrono::Utc>> {
    if value < 0 { return None; }
    chrono::DateTime::<chrono::Utc>::UNIX_EPOCH.checked_add_signed(chrono::Duration::microseconds(value))
}

/// SQLite から読んだ、まだ送信形式へ変換していない訪問。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadVisit { pub id: i64, pub url: String, pub title: Option<String>, pub at: chrono::DateTime<chrono::Utc>, pub transition: i64, pub from_visit: Option<i64>, pub duration_us: Option<i64>, pub originator_cache_guid: Option<String>, pub originator_visit_id: Option<i64> }

pub fn read_chromium(path: &std::path::Path) -> anyhow::Result<Vec<ReadVisit>> {
    let conn = rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let mut stmt = conn.prepare("SELECT v.id,u.url,u.title,v.visit_time,v.transition,v.from_visit,v.visit_duration,v.originator_cache_guid,v.originator_visit_id FROM visits v JOIN urls u ON u.id=v.url ORDER BY v.id")?;
    let visits = stmt.query_map([], |r| Ok(ReadVisit { id:r.get(0)?, url:r.get(1)?, title:r.get(2)?, at: chromium_micros(r.get(3)?).ok_or(rusqlite::Error::InvalidQuery)?, transition:r.get(4)?, from_visit: nonzero(r.get(5)?), duration_us: Some(r.get(6)?), originator_cache_guid: r.get(7)?, originator_visit_id: r.get(8)? }))?.collect::<Result<Vec<_>, _>>()?;
    Ok(visits)
}

pub fn read_firefox(path: &std::path::Path) -> anyhow::Result<Vec<ReadVisit>> {
    let conn = rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let mut stmt = conn.prepare("SELECT v.id,p.url,p.title,v.visit_date,v.visit_type,v.from_visit FROM moz_historyvisits v JOIN moz_places p ON p.id=v.place_id ORDER BY v.id")?;
    let visits = stmt.query_map([], |r| Ok(ReadVisit { id:r.get(0)?, url:r.get(1)?, title:r.get(2)?, at: firefox_micros(r.get(3)?).ok_or(rusqlite::Error::InvalidQuery)?, transition:r.get(4)?, from_visit: nonzero(r.get(5)?), duration_us: None, originator_cache_guid: None, originator_visit_id: None }))?.collect::<Result<Vec<_>, _>>()?;
    Ok(visits)
}

fn nonzero(value: i64) -> Option<i64> { (value != 0).then_some(value) }

/// 開いている DB を直接触らず、一時の写しへ読み取り操作を閉じ込める。
pub fn with_copy<T>(source: &std::path::Path, read: impl FnOnce(&std::path::Path) -> anyhow::Result<T>) -> anyhow::Result<T> {
    let copy = std::env::temp_dir().join(format!("ashiato-history-copy-{}.sqlite", uuid::Uuid::new_v4()));
    std::fs::copy(source, &copy)?;
    let result = read(&copy);
    std::fs::remove_file(copy).ok();
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_epoch_conversion() {
        assert_eq!(chromium_micros(0), Some(chrono::DateTime::parse_from_rfc3339("1601-01-01T00:00:00Z").expect("epoch").with_timezone(&chrono::Utc)));
        assert_eq!(firefox_micros(0), Some(chrono::DateTime::<chrono::Utc>::UNIX_EPOCH));
        assert_eq!(chromium_micros(-1), None);
        assert_eq!(firefox_micros(-1), None);
    }

    #[test]
    fn history_read_chromium() {
        // Scenario: 同じページを 2 回訪問すると 2 件になる
        // Scenario: ページの題名と滞在時間が載る
        // Scenario: 遷移の種類とどこから来たかが残る
        // Scenario: URL のクエリとフラグメントが残る
        // Scenario: 履歴 DB の URL の文字列を補正しない
        let db = temp_db(); let conn = rusqlite::Connection::open(&db).unwrap();
        conn.execute_batch("CREATE TABLE urls(id INTEGER PRIMARY KEY, url TEXT, title TEXT); CREATE TABLE visits(id INTEGER PRIMARY KEY, url INTEGER, visit_time INTEGER, transition INTEGER, from_visit INTEGER, visit_duration INTEGER, originator_cache_guid TEXT, originator_visit_id INTEGER);
          INSERT INTO urls VALUES(1, 'example.test/a?q=x#f', '題名'); INSERT INTO visits VALUES(1,1,1,3,0,7),(2,1,2,4,1,8);").unwrap(); drop(conn);
        let v = read_chromium(&db).unwrap(); assert_eq!(v.len(), 2); assert_eq!(v[0].url, "example.test/a?q=x#f"); assert_eq!(v[0].title.as_deref(), Some("題名")); assert_eq!(v[1].from_visit, Some(1)); std::fs::remove_file(db).ok();
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
        let db = temp_db(); let conn = rusqlite::Connection::open(&db).unwrap();
        conn.execute_batch("CREATE TABLE moz_places(id INTEGER PRIMARY KEY, url TEXT, title TEXT); CREATE TABLE moz_historyvisits(id INTEGER PRIMARY KEY, place_id INTEGER, visit_date INTEGER, visit_type INTEGER, from_visit INTEGER);
          INSERT INTO moz_places VALUES(1, 'https://example.test/a?q=x#f', '題名'); INSERT INTO moz_historyvisits VALUES(1,1,1,2,0);").unwrap(); drop(conn);
        let v = read_firefox(&db).unwrap(); assert_eq!(v.len(), 1); assert_eq!(v[0].url, "https://example.test/a?q=x#f"); std::fs::remove_file(db).ok();
    }
    #[test]
    fn history_copy_is_removed() {
        let db = temp_db(); rusqlite::Connection::open(&db).unwrap();
        let copy = with_copy(&db, |p| { assert!(p.exists()); Ok(p.to_owned()) }).unwrap();
        assert!(!copy.exists()); std::fs::remove_file(db).ok();
    }
    fn temp_db() -> std::path::PathBuf { std::env::temp_dir().join(format!("ashiato-read-{}.sqlite", uuid::Uuid::new_v4())) }
}
