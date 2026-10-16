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
    /// URL の行が見つからない訪問は `None`（URL も題名も無い訪問として送る。deep.md 第 4 回 Q8 / design D2）
    pub url: Option<String>,
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

/// 1 つの履歴 DB から読んだもの。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReadHistory {
    pub visits: Vec<ReadVisit>,
    /// 読めなかった行の数（負の訪問時刻・型の合わない値）。**行 1 つでプロファイル全体を落とさない**（R59）
    pub skipped: usize,
    /// Chromium の `sqlite_sequence` の `visits` の値。表の作り直しの手がかり（design D10）。Firefox と、表が無いときは `None`
    pub sequence: Option<i64>,
}

impl From<Vec<ReadVisit>> for ReadHistory {
    fn from(visits: Vec<ReadVisit>) -> Self {
        Self {
            visits,
            ..Self::default()
        }
    }
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

/// 行ごとの結果を集める。**行の値が不正なだけなら、その行を飛ばして数える**（R59）——
/// 1 行のために全プロファイルの取得が失敗し続け、1 分ごとに全件を読み直すのを防ぐ。
/// SQLite そのものの失敗（壊れた DB など）は行の話ではないので、読みの失敗として返す。
fn collect_rows(
    rows: impl Iterator<Item = rusqlite::Result<ReadVisit>>,
) -> anyhow::Result<(Vec<ReadVisit>, usize)> {
    let mut visits = Vec::new();
    let mut skipped = 0;
    for row in rows {
        match row {
            Ok(visit) => visits.push(visit),
            Err(e @ rusqlite::Error::SqliteFailure(..)) => return Err(e.into()),
            Err(_) => skipped += 1,
        }
    }
    Ok((visits, skipped))
}

/// `sqlite_sequence` の `visits` の値。表が無ければ `None`（AUTOINCREMENT を持たない古い形）。
fn visits_sequence(conn: &rusqlite::Connection) -> anyhow::Result<Option<i64>> {
    use rusqlite::OptionalExtension as _;
    let has_table: i64 = conn.query_row(
        "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='sqlite_sequence'",
        [],
        |r| r.get(0),
    )?;
    if has_table == 0 {
        return Ok(None);
    }
    Ok(conn
        .query_row(
            "SELECT seq FROM sqlite_sequence WHERE name='visits'",
            [],
            |r| r.get(0),
        )
        .optional()?)
}

pub fn read_chromium(path: &std::path::Path) -> anyhow::Result<ReadHistory> {
    let conn =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let col = |c| column_or_null(&conn, "visits", c);
    // 訪問と URL を内部結合で読まない。URL の行を失った訪問も URL 無しで読む（deep.md 第 4 回 Q8）
    let sql = format!(
        "SELECT v.id,u.url,u.title,v.visit_time,v.transition,v.from_visit,{},{},{},{},{} \
         FROM visits v LEFT JOIN urls u ON u.id=v.url ORDER BY v.id",
        col("visit_duration")?,
        col("opener_visit")?,
        col("originator_cache_guid")?,
        col("originator_visit_id")?,
        col("is_known_to_sync")?,
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], |r| {
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
    })?;
    let (visits, skipped) = collect_rows(rows)?;
    Ok(ReadHistory {
        visits,
        skipped,
        sequence: visits_sequence(&conn)?,
    })
}

pub fn read_firefox(path: &std::path::Path) -> anyhow::Result<ReadHistory> {
    let conn =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    // 訪問と URL を内部結合で読まない（deep.md 第 4 回 Q8）
    let mut stmt = conn.prepare("SELECT v.id,p.url,p.title,v.visit_date,v.visit_type,v.from_visit FROM moz_historyvisits v LEFT JOIN moz_places p ON p.id=v.place_id ORDER BY v.id")?;
    let rows = stmt.query_map([], |r| {
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
    })?;
    let (visits, skipped) = collect_rows(rows)?;
    Ok(ReadHistory {
        visits,
        skipped,
        sequence: None,
    })
}

fn nonzero(value: i64) -> Option<i64> {
    (value != 0).then_some(value)
}

/// 写しの後始末に失敗した回数。読みは別スレッドなので、ここに数えて見回りの側がログに出す（種別と件数だけ）。
static CLEANUP_FAILURES: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// 前回から写しの後始末に失敗した回数を返して 0 に戻す。
pub fn take_cleanup_failures() -> usize {
    CLEANUP_FAILURES.swap(0, std::sync::atomic::Ordering::Relaxed)
}

/// 写しを置いたディレクトリ。**読み手が panic しても、落ちる途中で消す**（design D2。R60）。
struct CopyDir(std::path::PathBuf);

impl Drop for CopyDir {
    fn drop(&mut self) {
        match std::fs::remove_dir_all(&self.0) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            // 文言にはパス（利用者名を含む）が入るので、数えるだけ。残骸は次の起動の掃除で消す
            Err(_) => {
                CLEANUP_FAILURES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
        }
    }
}

/// 起動時に、前のプロセスが残した写しを消す（落ちた・電源が切れた。design D2。R60）。
/// 無ければ何もしない。
pub fn sweep_copies(tmp: &std::path::Path) -> std::io::Result<()> {
    match std::fs::remove_dir_all(tmp) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}

/// 開いている DB を直接触らず、一時の写しへ読み取り操作を閉じ込める（design D2）。
///
/// `<名前>` と、あれば `<名前>-journal` / `<名前>-wal` を置き場の `tmp`（`state_dir/browser-history/tmp`）の下へ写し、
/// 読み終えたらディレクトリごと消す（私的な内容を置き場に残さない）。
pub fn with_copy<T>(
    source: &std::path::Path,
    tmp: &std::path::Path,
    read: impl FnOnce(&std::path::Path) -> anyhow::Result<T>,
) -> anyhow::Result<T> {
    let dir = CopyDir(tmp.join(uuid::Uuid::new_v4().to_string()));
    std::fs::create_dir_all(&dir.0)?;
    copy_and_read(source, &dir.0, read)
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

/// ブラウザの系統に合う読み手で、履歴 DB の写しから訪問を読む。写しは `tmp` の下に作る。
pub fn read_visits(
    browser: crate::history::locate::Browser,
    source: &std::path::Path,
    tmp: &std::path::Path,
) -> anyhow::Result<ReadHistory> {
    with_copy(source, tmp, |copy| match browser {
        crate::history::locate::Browser::Firefox => read_firefox(copy),
        _ => read_chromium(copy),
    })
}

/// 履歴を読む区間が無いときでも、写しを開いて最小の問い合わせまで通す。
///
/// 生存信号を「読める」と報告する前の確認専用で、訪問行は読み取らない。
pub fn probe_readable(source: &std::path::Path, tmp: &std::path::Path) -> anyhow::Result<()> {
    with_copy(source, tmp, |copy| {
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
        let v = read_chromium(&db).unwrap().visits;
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].url.as_deref(), Some("example.test/a?q=x#f"));
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
        let visits = read_chromium(&db).unwrap().visits;
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
        let visits = read_chromium(&db).unwrap().visits;
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
        let v = read_firefox(&db).unwrap().visits;
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].url.as_deref(), Some("https://example.test/a?q=x#f"));
        std::fs::remove_file(db).ok();
    }
    #[test]
    fn history_copy_is_removed() {
        // Scenario の印は Windows の本物のブラウザで確かめる `tests/runtime_windows.rs` が持つ（R62）
        let db = temp_db();
        let _open_browser_db = rusqlite::Connection::open(&db).unwrap();
        let copy = with_copy(&db, &tmp_root(), |p| {
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
        assert!(probe_readable(&db, &tmp_root()).is_ok());
        std::fs::remove_file(db).ok();
    }

    #[test]
    fn history_heartbeat_rejects_an_unreadable_database() {
        // Scenario: 履歴 DB が開けなければ取得できない状態として扱える
        let db = temp_db();
        std::fs::write(&db, "not a sqlite database").unwrap();
        assert!(probe_readable(&db, &tmp_root()).is_err());
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
        let v = read_chromium(&db).unwrap().visits;
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
        let v = read_chromium(&db).unwrap().visits;
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
        let v = read_firefox(&db).unwrap().visits;
        assert_eq!(v.len(), 2, "同じページを 2 回訪問すると 2 件");
        assert_eq!(v[0].url.as_deref(), Some("HTTPS://Example.TEST/Path"));
        assert_eq!(v[0].title, None);
        assert_eq!(v[1].from_visit, Some(1));
        assert_eq!(v[0].originator_cache_guid, None);
        std::fs::remove_file(db).ok();
    }

    #[test]
    fn history_copy_is_removed_with_wal_and_journal_and_reads_wal_rows() {
        let dir = std::env::temp_dir().join(format!("ashiato-read-dir-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let db = dir.join("History");
        let open = rusqlite::Connection::open(&db).unwrap();
        open.execute_batch("PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0; CREATE TABLE t(x INTEGER); INSERT INTO t VALUES(7);").unwrap();
        // 開いたまま（ブラウザが動いている）。行は -wal にだけある
        assert!(dir.join("History-wal").exists());
        std::fs::write(dir.join("History-journal"), []).unwrap();
        let mut seen = Vec::new();
        let n: i64 = with_copy(&db, &tmp_root(), |copy| {
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
        let v = read_visits(crate::history::locate::Browser::Firefox, &db, &tmp_root())
            .unwrap()
            .visits;
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
        let v = read_visits(b, &db, &tmp_root()).unwrap().visits;
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

    /// URL の行が無い訪問も読む。URL と題名は `None`、時刻・遷移・滞在時間は残る（deep.md 第 4 回 Q8。R57）。
    #[test]
    fn history_read_keeps_visits_whose_url_row_is_missing() {
        let db = temp_db();
        let conn = rusqlite::Connection::open(&db).unwrap();
        conn.execute_batch(CHROMIUM_FULL).unwrap();
        conn.execute_batch("INSERT INTO urls VALUES(1,'https://example.test/','t'); INSERT INTO visits VALUES(1,1,10,0,1,5,0,'',0,0),(2,99,20,0,8,7,0,'',0,0);").unwrap();
        drop(conn);
        let v = read_chromium(&db).unwrap().visits;
        assert_eq!(v.len(), 2, "URL の行が無い訪問を落とした");
        assert_eq!((v[1].url.as_ref(), v[1].title.as_ref()), (None, None));
        assert_eq!((v[1].transition, v[1].duration_us), (8, Some(7)));
        std::fs::remove_file(db).ok();

        let db = temp_db();
        let conn = rusqlite::Connection::open(&db).unwrap();
        conn.execute_batch("CREATE TABLE moz_places(id INTEGER PRIMARY KEY, url TEXT, title TEXT); CREATE TABLE moz_historyvisits(id INTEGER PRIMARY KEY, place_id INTEGER, visit_date INTEGER, visit_type INTEGER, from_visit INTEGER); INSERT INTO moz_places VALUES(1,'https://a.test/','t'); INSERT INTO moz_historyvisits VALUES(1,1,5,1,0),(2,42,6,2,0);").unwrap();
        drop(conn);
        let v = read_firefox(&db).unwrap().visits;
        assert_eq!(v.len(), 2);
        assert_eq!((v[1].url.as_ref(), v[1].transition), (None, 2));
        std::fs::remove_file(db).ok();
    }

    /// 負の訪問時刻・型の合わない値の行は、飛ばして数える。プロファイル全体を読めないにしない（R59）。
    #[test]
    fn history_read_skips_and_counts_bad_rows() {
        let db = temp_db();
        let conn = rusqlite::Connection::open(&db).unwrap();
        conn.execute_batch(CHROMIUM_FULL).unwrap();
        conn.execute_batch("INSERT INTO urls VALUES(1,'https://example.test/','t'); INSERT INTO visits VALUES(1,1,10,0,1,5,0,'',0,0),(2,1,-5,0,1,5,0,'',0,0),(3,1,'not-a-time',0,1,5,0,'',0,0),(4,1,30,0,1,5,0,'',0,0);").unwrap();
        drop(conn);
        let read = read_chromium(&db).unwrap();
        assert_eq!(
            read.visits.iter().map(|v| v.id).collect::<Vec<_>>(),
            vec![1, 4]
        );
        assert_eq!(read.skipped, 2);
        std::fs::remove_file(db).ok();

        let db = temp_db();
        let conn = rusqlite::Connection::open(&db).unwrap();
        conn.execute_batch("CREATE TABLE moz_places(id INTEGER PRIMARY KEY, url TEXT, title TEXT); CREATE TABLE moz_historyvisits(id INTEGER PRIMARY KEY, place_id INTEGER, visit_date INTEGER, visit_type INTEGER, from_visit INTEGER); INSERT INTO moz_places VALUES(1,'https://a.test/','t'); INSERT INTO moz_historyvisits VALUES(1,1,-1,1,0),(2,1,6,2,0);").unwrap();
        drop(conn);
        let read = read_firefox(&db).unwrap();
        assert_eq!((read.visits.len(), read.skipped), (1, 1));
        std::fs::remove_file(db).ok();
    }

    /// Chromium の `sqlite_sequence` の値を読む。表が無い古い形は `None`（design D10。R64）。
    #[test]
    fn history_read_chromium_visits_sequence() {
        let db = temp_db();
        let conn = rusqlite::Connection::open(&db).unwrap();
        conn.execute_batch("CREATE TABLE urls(id INTEGER PRIMARY KEY, url TEXT, title TEXT); CREATE TABLE visits(id INTEGER PRIMARY KEY AUTOINCREMENT, url INTEGER, visit_time INTEGER, from_visit INTEGER, transition INTEGER); INSERT INTO urls VALUES(1,'https://example.test/','t'); INSERT INTO visits(url,visit_time,from_visit,transition) VALUES(1,10,0,1),(1,20,0,1); DELETE FROM visits WHERE id=2;").unwrap();
        drop(conn);
        let read = read_chromium(&db).unwrap();
        assert_eq!((read.visits.len(), read.sequence), (1, Some(2)));
        std::fs::remove_file(db).ok();

        let db = temp_db();
        let conn = rusqlite::Connection::open(&db).unwrap();
        conn.execute_batch("CREATE TABLE urls(id INTEGER PRIMARY KEY, url TEXT, title TEXT); CREATE TABLE visits(id INTEGER PRIMARY KEY, url INTEGER, visit_time INTEGER, from_visit INTEGER, transition INTEGER);").unwrap();
        drop(conn);
        assert_eq!(read_chromium(&db).unwrap().sequence, None);
        std::fs::remove_file(db).ok();
    }

    /// 写しは置き場の `tmp` の下に作り、読み手が panic しても消える（design D2。R60）。
    #[test]
    fn history_copy_lives_under_tmp_and_is_removed_on_panic() {
        let db = temp_db();
        rusqlite::Connection::open(&db).unwrap();
        let tmp = tmp_root();
        let mut seen = None;
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            with_copy(&db, &tmp, |copy| -> anyhow::Result<()> {
                seen = Some(copy.to_owned());
                panic!("読み手が落ちた");
            })
        }));
        assert!(result.is_err());
        let copy = seen.unwrap();
        assert!(
            copy.starts_with(&tmp),
            "写しが置き場の外: {}",
            copy.display()
        );
        assert!(!copy.exists(), "panic で写しが残った");
        assert_eq!(std::fs::read_dir(&tmp).unwrap().count(), 0);
        std::fs::remove_dir_all(tmp).ok();
        std::fs::remove_file(db).ok();
    }

    /// 起動時に前のプロセスの残骸を消す。無いときは何もしない（R60）。
    #[test]
    fn history_copy_leftovers_are_swept() {
        let tmp = tmp_root();
        std::fs::create_dir_all(tmp.join("left-over")).unwrap();
        std::fs::write(tmp.join("left-over/History"), b"private").unwrap();
        sweep_copies(&tmp).unwrap();
        assert!(!tmp.exists());
        sweep_copies(&tmp).unwrap();
    }

    /// 写しの置き場（本物は `state_dir/browser-history/tmp`）。
    fn tmp_root() -> std::path::PathBuf {
        std::env::temp_dir().join(format!("ashiato-read-tmp-{}", uuid::Uuid::new_v4()))
    }

    fn temp_db() -> std::path::PathBuf {
        std::env::temp_dir().join(format!("ashiato-read-{}.sqlite", uuid::Uuid::new_v4()))
    }
}
