// SPDX-License-Identifier: AGPL-3.0-only
//! `tools/smoke.sh` 用の台本。Chromium の形の小さな履歴 DB を作り、本物の `Runtime` で取得して送る。
//!
//! - `make <db> [<url> <title>]` … 履歴 DB を作る
//! - `fetch <db> <state_dir> <base_url> <token>` … その DB を 1 回取得し、取り込み口へ送る（届かなければ未送信に残す）
//! - `send <state_dir> <base_url> <token>` … 置き場に残っている未送信を送る
use std::sync::Arc;

use ashiato_collector_windows::clock::HttpDateClock;
use ashiato_collector_windows::config::{Config, Zone};
use ashiato_collector_windows::engine::{Engine, IdleRead, Observation};
use ashiato_collector_windows::exclusion::Exclusions;
use ashiato_collector_windows::history::collect::{HistoryReader, ProfileRead, ReadOutcome};
use ashiato_collector_windows::history::locate::Browser;
use ashiato_collector_windows::history::read::read_visits;
use ashiato_collector_windows::runtime::{Runtime, Source};
use ashiato_collector_windows::sender::HttpTransport;
use rusqlite::Connection;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("make") => make(&args[1], args.get(2), args.get(3)),
        Some("fetch") => run(&args[2], &args[3], &args[4], Some(&args[1])),
        Some("send") => run(&args[1], &args[2], &args[3], None),
        // 引数 1 つは従来の呼び方（DB を作るだけ）
        _ => make(&args[0], None, None),
    }
}

fn make(path: &str, url: Option<&String>, title: Option<&String>) -> anyhow::Result<()> {
    let url = url.map_or("https://example.test/yesterday", String::as_str);
    let title = title.map_or("前日のページ", String::as_str);
    let db = Connection::open(path)?;
    db.execute_batch("CREATE TABLE urls(id INTEGER PRIMARY KEY, url TEXT, title TEXT); CREATE TABLE visits(id INTEGER PRIMARY KEY, url INTEGER, visit_time INTEGER, transition INTEGER, from_visit INTEGER, visit_duration INTEGER, originator_cache_guid TEXT, originator_visit_id INTEGER);")?;
    db.execute("INSERT INTO urls VALUES (1, ?1, ?2)", [url, title])?;
    db.execute_batch("INSERT INTO visits VALUES (1, 1, 13402627200000000, 1, 0, 0, NULL, NULL);")?;
    Ok(())
}

/// 画面がロックされている PC（前景は読まない）。履歴の取得だけを見るための偽の OS。
#[derive(Debug)]
struct LockedPc;

impl Source for LockedPc {
    fn observe(&mut self, at: chrono::DateTime<chrono::Utc>) -> Observation {
        Observation {
            at,
            foreground: None,
            idle: IdleRead::Unavailable,
            locked: true,
        }
    }
}

/// 指した 1 つの履歴 DB だけを読む。
struct OneDb(std::path::PathBuf);

impl HistoryReader for OneDb {
    fn read(&self) -> anyhow::Result<ReadOutcome> {
        Ok(ReadOutcome {
            profiles: vec![ProfileRead {
                browser: Browser::Chrome,
                directory: "Default".into(),
                visits: read_visits(Browser::Chrome, &self.0),
            }],
            names: Vec::new(),
        })
    }
}

fn run(state_dir: &str, base_url: &str, token: &str, db: Option<&String>) -> anyhow::Result<()> {
    let cfg = Config {
        base_url: base_url.into(),
        api_token: token.into(),
        user_id: uuid::Uuid::nil(),
        device_id: "history-smoke".into(),
        state_dir: state_dir.into(),
    };
    let zone = Zone {
        id: "Asia/Tokyo".into(),
        offset_min: 540,
    };
    let transport = HttpTransport::new(base_url, token);
    let reference = HttpDateClock::new(base_url);
    let mut rt = Runtime::new(
        &cfg,
        zone,
        Engine::new(Exclusions::default()),
        &transport,
        &reference,
        chrono::Utc::now(),
    )?;
    if let Some(db) = db {
        rt = rt.with_history(Arc::new(OneDb(db.into())));
    }
    let mut source = LockedPc;
    rt.start(&source);
    let done = std::path::Path::new(state_dir).join("browser-history/last_success.json");
    for _ in 0..200 {
        rt.tick(&mut source);
        if db.is_none() || done.is_file() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    anyhow::ensure!(db.is_none() || done.is_file(), "取得が終わらなかった");
    // 手元の未送信を送る（届かなければ置き場に残る）
    rt.stop();
    Ok(())
}
