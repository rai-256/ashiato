// SPDX-License-Identifier: AGPL-3.0-only
//! `tools/smoke.sh` 用の最小 Chromium 履歴 DB を作る台本。
use rusqlite::Connection;

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("出力先の History パス");
    let db = Connection::open(path)?;
    db.execute_batch("CREATE TABLE urls(id INTEGER PRIMARY KEY, url TEXT, title TEXT); CREATE TABLE visits(id INTEGER PRIMARY KEY, url INTEGER, visit_time INTEGER, transition INTEGER, from_visit INTEGER, visit_duration INTEGER, originator_cache_guid TEXT, originator_visit_id INTEGER); INSERT INTO urls VALUES (1, 'https://example.test/yesterday', '前日のページ'); INSERT INTO visits VALUES (1, 1, 13402627200000000, 1, 0, 0, NULL, NULL);")?;
    Ok(())
}
