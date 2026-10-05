// SPDX-License-Identifier: AGPL-3.0-only
//! ST12 の「置いてから入るまで」を、置き場からの縦串で確かめる。
//!
//! `archive_tests` が部品ごとの振る舞いを見るのに対し、ここは **本人の操作の単位**
//! （書庫を置く・印を置く・置き直す・設定を変える）で spec の Scenario を担保する。
#![allow(clippy::unwrap_used)]

use crate::testdb;
use std::path::{Path, PathBuf};

/// 背景の取り込み器を待つ上限。待ちは 50 ms ごとの問い合わせなので、
/// 緑のときの所要時間は上限に依らない。
const WAIT: std::time::Duration = std::time::Duration::from_secs(60);

/// 合成の視聴履歴 1 件。**時刻と題名だけを変えられる**ようにして、
/// 「同じ視聴の内容が違う」を作れるようにする。
fn watch(time: &str, title: &str) -> String {
    format!(
        r#"[{{"time":"{time}","title":"{title}","titleUrl":"https://www.youtube.com/watch?v=abc"}}]"#
    )
}

/// 置き場・写し・DB をひとまとめにした足場。落ちても片付くよう `Drop` で消す。
struct Inbox {
    /// 起こした取り込み器。**落ちると止まる**ので、足場が生きている間だけ握る。
    workers: std::sync::Mutex<Vec<crate::archive::worker::WorkerHandle>>,
    root: PathBuf,
    inbox: PathBuf,
    downloads: PathBuf,
    copies: PathBuf,
    pool: sqlx::PgPool,
    user: uuid::Uuid,
}

impl Drop for Inbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

impl Inbox {
    async fn new(name: &str) -> Self {
        let pool = testdb::pool().await;
        let root = std::env::temp_dir().join(format!("ashiato-{name}-{}", uuid::Uuid::new_v4()));
        let inbox = root.join("inbox");
        let downloads = root.join("downloads");
        std::fs::create_dir_all(&inbox).unwrap();
        std::fs::create_dir_all(&downloads).unwrap();
        Self {
            workers: std::sync::Mutex::new(Vec::new()),
            copies: root.join("copies"),
            root,
            inbox,
            downloads,
            pool,
            user: testdb::user(),
        }
    }

    fn config(&self, keep_copies: bool) -> crate::archive::config::ArchiveConfig {
        crate::archive::config::ArchiveConfig {
            inbox_dir: self.inbox.clone(),
            downloads_dir: self.downloads.clone(),
            copy_dir: self.copies.clone(),
            keep_copies,
            user_id: Some(self.user),
            scan_sec: 1,
        }
    }

    fn spawn(&self, keep_copies: bool) {
        let handle = crate::archive::worker::spawn_inspecting(
            self.pool.clone(),
            self.config(keep_copies),
            self.user,
            crate::archive::worker::ReadingState::default(),
        );
        self.workers.lock().unwrap().push(handle);
    }

    /// その中身の形に印を置く（`tools/archive-shape.sh --confirm` と同じことを直に行う）。
    async fn confirm(&self, kind: crate::archive::classify::KnownKind, bytes: &[u8]) {
        let shape =
            crate::archive::worker::shape_for_file(kind, "Takeout/fixture.json", bytes).unwrap();
        sqlx::query(
            "INSERT INTO core.archive_shape_confirmation (user_id, shape_hash, shape)
             VALUES ($1, $2, $3)",
        )
        .bind(self.user)
        .bind(crate::archive::worker::hash_shape(&shape))
        .bind(shape)
        .execute(&self.pool)
        .await
        .unwrap();
    }

    /// 専用のフォルダへ書庫を置く。
    fn put(&self, name: &str, entries: &[(&str, &[u8])]) {
        write_zip(&self.inbox.join(name), entries);
    }

    /// ダウンロードのフォルダへ書庫を置く。
    fn put_downloads(&self, name: &str, entries: &[(&str, &[u8])]) {
        write_zip(&self.downloads.join(name), entries);
    }

    /// 条件が満たされるまで待つ。満たされなければ `why` を添えて落とす。
    async fn until<F, Fut>(&self, why: &str, mut check: F)
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = bool>,
    {
        let waited = tokio::time::timeout(WAIT, async {
            loop {
                if check().await {
                    return;
                }
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
        })
        .await;
        assert!(waited.is_ok(), "{why}");
    }

    async fn events(&self, logical_source: &str) -> i64 {
        sqlx::query_scalar(
            "SELECT count(*) FROM core.event
              WHERE user_id = $1 AND logical_source = $2 AND deleted_at IS NULL",
        )
        .bind(self.user)
        .bind(logical_source)
        .fetch_one(&self.pool)
        .await
        .unwrap()
    }

    async fn ledger_rows(&self, outcome: &str) -> i64 {
        sqlx::query_scalar(
            "SELECT count(*) FROM core.archive_ledger WHERE user_id = $1 AND outcome = $2",
        )
        .bind(self.user)
        .bind(outcome)
        .fetch_one(&self.pool)
        .await
        .unwrap()
    }

    async fn status(&self) -> crate::ArchivesStatus {
        crate::archives_status_for(&self.pool, self.user, None)
            .await
            .unwrap()
    }

    async fn last_event_on(&self, logical_source: &str) -> Option<String> {
        self.status()
            .await
            .sources
            .into_iter()
            .find(|source| source.logical_source == logical_source)
            .and_then(|source| source.last_event_on)
    }
}

fn write_zip(path: &Path, entries: &[(&str, &[u8])]) {
    use std::io::Write as _;
    let file = std::fs::File::create(path).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    for (name, contents) in entries {
        zip.start_file(*name, zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(contents).unwrap();
    }
    zip.finish().unwrap();
}

/// Scenario: ダウンロードのフォルダの Takeout の書庫が読まれる
#[tokio::test]
async fn archive_flow_reads_takeout_from_the_downloads_folder() {
    let inbox = Inbox::new("archive-downloads").await;
    let body = watch("2026-09-12T03:00:00Z", "ある動画");
    inbox
        .confirm(
            crate::archive::classify::KnownKind::YouTubeWatch,
            body.as_bytes(),
        )
        .await;
    inbox.put_downloads(
        "takeout-20260913T041200Z-001.zip",
        &[("Takeout/YouTube/watch-history.json", body.as_bytes())],
    );
    inbox.spawn(true);

    inbox
        .until("ダウンロードの Takeout が読まれない", || async {
            inbox.events("c03-youtube-watch").await == 1
        })
        .await;
}

/// Scenario: 既定の間隔でも置かれてから 10 分以内に読み始める
#[test]
fn archive_flow_default_interval_starts_reading_within_ten_minutes() {
    let home = std::collections::BTreeMap::from([("HOME".to_string(), "/home/me".to_string())]);
    let config = crate::archive::config::from_values(&home).unwrap();
    // D8 は「2 回続けて大きさと更新時刻が変わらない」ことを読み始めの条件にしている。
    // 置いた直後の走査は 1 回目なので、読み始めるのは **2 回目**の走査。
    const SCANS_BEFORE_READ: u64 = 2;
    assert!(
        config.scan_sec * SCANS_BEFORE_READ <= 600,
        "既定の走査の間隔 {} 秒では、置いてから読み始めるまでに 10 分を超える",
        config.scan_sec
    );
}

/// Scenario: 書庫の位置は携帯端末の位置に入らない
/// Scenario: 書庫の位置は携帯端末の位置の収集開始日を動かさない
/// Scenario: 書庫の位置と端末の位置が同じでも取りやめない
#[tokio::test]
async fn archive_flow_locations_stay_out_of_the_phone_source() {
    let inbox = Inbox::new("archive-legacy-location").await;
    // **同じ時刻・同じ座標の記録を、本物の格納関門を通して端末の位置へ入れる。**
    // 直に INSERT すると原文も内容の鍵も別物になり、spec の WHEN（同じ時刻・
    // 同じ座標でも取りやめない）を作れないまま緑になる（review R16）。
    let same_point =
        r#"{"timestampMs":"1622505600000","latitudeE7":356580000,"longitudeE7":1397450000}"#;
    crate::store_one(
        &inbox.pool,
        crate::IngestRequest {
            id: uuid::Uuid::new_v4(),
            user_id: inbox.user,
            logical_source: "c01-location".into(),
            external_id: None,
            device_id: Some("test".into()),
            origin: "collected".into(),
            event_time: chrono::DateTime::parse_from_rfc3339("2021-06-01T00:00:00Z")
                .unwrap()
                .to_utc(),
            tz_offset_min: 0,
            tz_id: "UTC".into(),
            schema_version: 1,
            unit_system: None,
            crs: None,
            source_updated_at: None,
            external_ref: None,
            raw: same_point.into(),
            payload: serde_json::json!({}),
        },
    )
    .await
    .unwrap();
    let before: Option<chrono::NaiveDate> = sqlx::query_scalar(
        "SELECT collection_started_on FROM core.source WHERE logical_source = 'c01-location'",
    )
    .fetch_one(&inbox.pool)
    .await
    .unwrap();
    let phone_events = inbox.events("c01-location").await;

    let records = format!(r#"{{"locations":[{same_point}]}}"#);
    inbox.put(
        "Records.json.zip",
        &[("Takeout/Records.json", records.as_bytes())],
    );
    inbox.spawn(true);

    inbox
        .until(
            "移行前のロケーション履歴が書庫のソースへ入らない",
            || async { inbox.events("c03-legacy-location").await >= 1 },
        )
        .await;
    assert_eq!(
        inbox.events("c01-location").await,
        phone_events,
        "書庫の位置が携帯端末の位置の論理ソースへ入っている"
    );
    let after: Option<chrono::NaiveDate> = sqlx::query_scalar(
        "SELECT collection_started_on FROM core.source WHERE logical_source = 'c01-location'",
    )
    .fetch_one(&inbox.pool)
    .await
    .unwrap();
    assert_eq!(
        after, before,
        "書庫の位置が携帯端末の位置の収集開始日を動かした"
    );
}

/// Scenario: 書庫の論理ソースは成功条件 1 の達成に数えられない
#[tokio::test]
async fn archive_flow_archive_sources_are_not_counted_in_achievement() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    testdb::put_event(
        &pool,
        user,
        "c03-youtube-watch",
        "2026-09-12T12:00:00+09:00",
    )
    .await;

    let got = crate::coverage::achievement(
        &pool,
        Some(user),
        testdb::date("2026-09-13"),
        &crate::coverage::must_sources(),
    )
    .await
    .unwrap();

    let named: Vec<&str> = got
        .sources
        .iter()
        .map(|s| s.named_source.as_str())
        .collect();
    assert_eq!(
        named,
        vec![
            "c01-location",
            "c01-app-usage",
            "c01-photo",
            "c02-window",
            "c02-browser-history"
        ],
        "達成の対象が Must の 5 ソースから変わっている"
    );
}

/// Scenario: 対象でないファイルは数だけ台帳に残る
#[tokio::test]
async fn archive_flow_skipped_files_are_counted_in_the_ledger() {
    let inbox = Inbox::new("archive-skipped").await;
    let body = watch("2026-09-12T03:00:00Z", "ある動画");
    inbox
        .confirm(
            crate::archive::classify::KnownKind::YouTubeWatch,
            body.as_bytes(),
        )
        .await;
    inbox.put(
        "takeout-20260912-000000.zip",
        &[
            ("Takeout/YouTube/watch-history.json", body.as_bytes()),
            (
                "Takeout/Google フォト/IMG_0001.jpg",
                b"\xff\xd8\xff\xe0jpeg",
            ),
            (
                "Takeout/Google フォト/IMG_0002.jpg",
                b"\xff\xd8\xff\xe0jpeg",
            ),
        ],
    );
    inbox.spawn(true);

    inbox
        .until("読めた書庫の台帳が残らない", || async {
            inbox.ledger_rows("read").await == 1
        })
        .await;
    let skipped: i32 = sqlx::query_scalar(
        "SELECT skipped_file_count FROM core.archive_ledger
          WHERE user_id = $1 AND outcome = 'read'",
    )
    .bind(inbox.user)
    .fetch_one(&inbox.pool)
    .await
    .unwrap();
    assert_eq!(
        skipped, 2,
        "写真 2 枚が読まなかったファイルとして数えられていない"
    );
}

/// 合成のマイアクティビティ 1 件。製品の名前だけを変えられるようにする。
///
/// **`titleUrl` を持たせる** —— 見分け（`classify`）は最初の項目の `titleUrl` で
/// 種類を決めるので、これが無いと読む対象にすらならない。
fn myactivity(product: &str) -> String {
    format!(
        r#"[{{"header":"{product}","title":"見た","titleUrl":"https://www.google.com/search?q=x","time":"2026-09-12T03:00:00Z","products":["{product}"]}}]"#
    )
}

/// Scenario: 知らない製品のマイアクティビティはまた確認待ちになる
/// Scenario: 知らない製品があっても同じ書庫の他のファイルは格納される
#[tokio::test]
async fn archive_flow_an_unknown_product_waits_while_the_rest_is_stored() {
    let inbox = Inbox::new("archive-unknown-product").await;
    let watch_body = watch("2026-09-12T03:00:00Z", "ある動画");
    let known = myactivity("検索");
    let unknown = myactivity("Discover");
    inbox
        .confirm(
            crate::archive::classify::KnownKind::YouTubeWatch,
            watch_body.as_bytes(),
        )
        .await;
    inbox
        .confirm(
            crate::archive::classify::KnownKind::MyActivity,
            known.as_bytes(),
        )
        .await;
    inbox.put(
        "takeout-20260912-000000.zip",
        &[
            ("Takeout/YouTube/watch-history.json", watch_body.as_bytes()),
            ("Takeout/My Activity/Discover/活動.json", unknown.as_bytes()),
        ],
    );
    inbox.spawn(true);

    // **確認待ちの側を待つ** —— 書庫の中のファイルは順に読まれるので、視聴履歴が
    // 入った時点ではまだ 2 つ目のファイルを読んでいない。
    inbox
        .until(
            "印に無い製品のファイルが確認待ちにならない",
            || async {
                let pending: i64 = sqlx::query_scalar(
                    "SELECT count(*) FROM core.archive_pending_shape WHERE user_id = $1",
                )
                .bind(inbox.user)
                .fetch_one(&inbox.pool)
                .await
                .unwrap();
                pending == 1
            },
        )
        .await;
    assert_eq!(
        inbox.events("c03-youtube-watch").await,
        1,
        "知らない製品があると同じ書庫の視聴履歴まで止まっている"
    );
    let waiting_path: String =
        sqlx::query_scalar("SELECT inner_path FROM core.archive_pending_shape WHERE user_id = $1")
            .bind(inbox.user)
            .fetch_one(&inbox.pool)
            .await
            .unwrap();
    assert!(
        waiting_path.contains("Discover"),
        "確認待ちになったのが知らない製品のファイルではない: {waiting_path}"
    );
}

/// Scenario: 形の確認の出力に記録の値が出ない
#[test]
fn archive_flow_shape_never_carries_a_search_query() {
    let body = r#"[{"time":"2026-09-12T03:00:00Z","title":"京都 旅館 を検索","titleUrl":"https://www.youtube.com/results?search_query=%E4%BA%AC%E9%83%BD+%E6%97%85%E9%A4%A8"}]"#;
    let shape = crate::archive::worker::shape_for_file(
        crate::archive::classify::KnownKind::YouTubeSearch,
        "Takeout/YouTube/search-history.json",
        body.as_bytes(),
    )
    .unwrap();

    let printed = serde_json::to_string(&shape).unwrap();
    for value in ["京都 旅館", "%E4%BA%AC%E9%83%BD", "youtube.com"] {
        assert!(
            !printed.contains(value),
            "形の出力に記録の値「{value}」が出ている: {printed}"
        );
    }
}

/// Scenario: 確認待ちの書庫は写しを残さない設定でも写される
#[tokio::test]
async fn archive_flow_pending_shape_is_copied_even_when_copies_are_off() {
    let inbox = Inbox::new("archive-pending-copy").await;
    let body = watch("2026-09-12T03:00:00Z", "ある動画");
    // 印は置かない —— 確認待ちのまま写しが要る（印を置いた後に写しから読み直すため）。
    inbox.put(
        "takeout-20260912-000000.zip",
        &[("Takeout/YouTube/watch-history.json", body.as_bytes())],
    );
    inbox.spawn(false);

    inbox
        .until("確認待ちの台帳が残らない", || async {
            inbox.ledger_rows("pending_shape").await == 1
        })
        .await;
    let copies: i64 =
        sqlx::query_scalar("SELECT count(*) FROM core.archive_file WHERE user_id = $1")
            .bind(inbox.user)
            .fetch_one(&inbox.pool)
            .await
            .unwrap();
    assert_eq!(
        copies, 1,
        "写しを残さない設定で確認待ちの写しが作られていない"
    );
}

/// Scenario: 読まなかった製品のファイルは写されない
#[tokio::test]
async fn archive_flow_unread_products_are_never_copied() {
    let inbox = Inbox::new("archive-copy-scope").await;
    let body = watch("2026-09-12T03:00:00Z", "ある動画");
    inbox
        .confirm(
            crate::archive::classify::KnownKind::YouTubeWatch,
            body.as_bytes(),
        )
        .await;
    inbox.put(
        "takeout-20260912-000000.zip",
        &[
            ("Takeout/YouTube/watch-history.json", body.as_bytes()),
            (
                "Takeout/Google フォト/IMG_0001.jpg",
                b"\xff\xd8\xff\xe0jpeg",
            ),
        ],
    );
    inbox.spawn(true);

    inbox
        .until("読めた書庫の台帳が残らない", || async {
            inbox.ledger_rows("read").await == 1
        })
        .await;
    let paths: Vec<String> =
        sqlx::query_scalar("SELECT inner_path FROM core.archive_file WHERE user_id = $1")
            .bind(inbox.user)
            .fetch_all(&inbox.pool)
            .await
            .unwrap();
    assert_eq!(
        paths.len(),
        1,
        "写しの目録が読んだ製品のファイルだけになっていない"
    );
    assert!(
        !paths[0].contains("IMG_0001"),
        "読まなかった写真の写しが残っている: {paths:?}"
    );
}

/// Scenario: 残さない設定でも記録は格納される
/// Scenario: 残さない設定に切り替えても既にある写しは残る
#[tokio::test]
async fn archive_flow_turning_copies_off_keeps_records_and_old_copies() {
    let inbox = Inbox::new("archive-copy-switch").await;
    let first = watch("2026-09-12T03:00:00Z", "ある動画");
    let second = watch("2026-09-13T03:00:00Z", "別の動画");
    inbox
        .confirm(
            crate::archive::classify::KnownKind::YouTubeWatch,
            first.as_bytes(),
        )
        .await;

    // 1 本目は写しを残す設定で読ませる。
    inbox.put(
        "takeout-20260912-000000.zip",
        &[("Takeout/YouTube/watch-history.json", first.as_bytes())],
    );
    inbox.spawn(true);
    inbox
        .until("1 本目が読まれない", || async {
            inbox.events("c03-youtube-watch").await == 1
        })
        .await;
    let kept: Vec<String> =
        sqlx::query_scalar("SELECT stored_path FROM core.archive_file WHERE user_id = $1")
            .bind(inbox.user)
            .fetch_all(&inbox.pool)
            .await
            .unwrap();
    assert_eq!(kept.len(), 1, "写しを残す設定で写しが作られていない");

    // 設定を切り替えて取り込み器を起こし直し、2 本目を読ませる。
    inbox.put(
        "takeout-20260913-000000.zip",
        &[("Takeout/YouTube/watch-history.json", second.as_bytes())],
    );
    inbox.spawn(false);
    inbox
        .until("残さない設定で記録が格納されない", || async {
            inbox.events("c03-youtube-watch").await == 2
        })
        .await;
    assert!(
        std::path::Path::new(&kept[0]).exists(),
        "切り替える前に作った写しが消えている: {}",
        kept[0]
    );
}

/// Scenario: 古い書庫を後から置いても新しい記録は書き換わらない
/// Scenario: 古い書庫を後から置いても最終日は戻らない
/// Scenario: 最終日と一緒に運んだ書庫の作られた時刻が残る
#[tokio::test]
async fn archive_flow_an_older_archive_never_rewinds_records_or_the_last_day() {
    let inbox = Inbox::new("archive-older").await;
    // 同じ視聴（同じ URL・同じ時刻）を、内容の違う 2 つの書庫が運ぶ。
    let newest = watch("2026-09-10T03:00:00Z", "新しい題名");
    let older = watch("2026-09-10T03:00:00Z", "古い題名");
    let ancient = watch("2024-06-30T03:00:00Z", "もっと古い視聴");
    for body in [&newest, &older, &ancient] {
        inbox
            .confirm(
                crate::archive::classify::KnownKind::YouTubeWatch,
                body.as_bytes(),
            )
            .await;
    }
    inbox.put(
        "takeout-20260913-041200.zip",
        &[("Takeout/YouTube/watch-history.json", newest.as_bytes())],
    );
    inbox.spawn(true);
    inbox
        .until("新しい書庫が読まれない", || async {
            inbox.last_event_on("c03-youtube-watch").await.is_some()
        })
        .await;
    assert_eq!(
        inbox.last_event_on("c03-youtube-watch").await.as_deref(),
        Some("2026-09-10"),
        "最終日がいちばん新しい出来事の日になっていない"
    );
    // 1 件も運ばれていないソースには、書庫の作られた時刻も付かない（D11）。
    let untouched = inbox
        .status()
        .await
        .sources
        .into_iter()
        .find(|s| s.logical_source == "c03-chrome-history")
        .expect("Chrome の履歴のソースが無い");
    assert_eq!(untouched.last_event_on, None);
    assert_eq!(
        untouched.last_archive_created_at, None,
        "記録を 1 件も運んでいないソースに、書庫の作られた時刻が付いている"
    );

    // 同じ視聴を違う内容で運ぶ、より古く作られた書庫を後から置く。
    inbox.put(
        "takeout-20260713-041200.zip",
        &[("Takeout/YouTube/watch-history.json", older.as_bytes())],
    );
    inbox
        .until("古い書庫が読まれない", || async {
            inbox.ledger_rows("read").await == 2
        })
        .await;

    let titles: Vec<String> = sqlx::query_scalar(
        "SELECT raw FROM core.event
          WHERE user_id = $1 AND logical_source = 'c03-youtube-watch' AND deleted_at IS NULL",
    )
    .bind(inbox.user)
    .fetch_all(&inbox.pool)
    .await
    .unwrap();
    assert!(
        titles.iter().any(|raw| raw.contains("新しい題名")),
        "先に入った記録の内容が古い書庫で書き換わっている: {titles:?}"
    );

    // 最終日と一緒に持つ「書庫の作られた時刻」は、いちばん新しく作られた側のまま。
    let created = inbox
        .status()
        .await
        .sources
        .into_iter()
        .find(|s| s.logical_source == "c03-youtube-watch")
        .and_then(|s| s.last_archive_created_at)
        .expect("最終日を運んだ書庫の作られた時刻が無い");
    assert_eq!(
        created.format("%F").to_string(),
        "2026-09-13",
        "最終日を運んだ書庫のうち、いちばん新しく作られた側が残っていない"
    );

    // もっと古い出来事しか持たない書庫を置いても最終日は動かない。
    inbox.put(
        "takeout-20240701-041200.zip",
        &[("Takeout/YouTube/watch-history.json", ancient.as_bytes())],
    );
    inbox
        .until("もっと古い書庫が読まれない", || async {
            inbox.ledger_rows("read").await == 3
        })
        .await;
    assert_eq!(
        inbox.last_event_on("c03-youtube-watch").await.as_deref(),
        Some("2026-09-10"),
        "古い書庫を後から置いたら最終日が戻った"
    );
}

/// Scenario: 最終日の記録を消しても最終日は戻らない
#[tokio::test]
async fn archive_flow_deleting_the_record_never_rewinds_the_last_day() {
    let inbox = Inbox::new("archive-last-day-delete").await;
    let body = watch("2026-09-10T03:00:00Z", "ある動画");
    inbox
        .confirm(
            crate::archive::classify::KnownKind::YouTubeWatch,
            body.as_bytes(),
        )
        .await;
    inbox.put(
        "takeout-20260913-041200.zip",
        &[("Takeout/YouTube/watch-history.json", body.as_bytes())],
    );
    inbox.spawn(true);
    inbox
        .until("書庫が読まれない", || async {
            inbox.last_event_on("c03-youtube-watch").await.is_some()
        })
        .await;

    sqlx::query(
        "UPDATE core.event SET deleted_at = now(), deleted_by = 'test'
          WHERE user_id = $1 AND logical_source = 'c03-youtube-watch'",
    )
    .bind(inbox.user)
    .execute(&inbox.pool)
    .await
    .unwrap();

    assert_eq!(
        inbox.events("c03-youtube-watch").await,
        0,
        "記録が消えていない"
    );
    assert_eq!(
        inbox.last_event_on("c03-youtube-watch").await.as_deref(),
        Some("2026-09-10"),
        "記録を消したら最終日が戻った（取り込んだ事実は削除で消えない）"
    );
}

/// Scenario: 消した記録は別の書庫からも戻らない
#[tokio::test]
async fn archive_flow_a_deleted_record_never_returns_from_another_archive() {
    let inbox = Inbox::new("archive-deleted-record").await;
    let body = watch("2026-09-10T03:00:00Z", "ある動画");
    let other = watch("2026-09-11T03:00:00Z", "別の動画");
    for content in [&body, &other] {
        inbox
            .confirm(
                crate::archive::classify::KnownKind::YouTubeWatch,
                content.as_bytes(),
            )
            .await;
    }
    inbox.put(
        "takeout-20260913-041200.zip",
        &[("Takeout/YouTube/watch-history.json", body.as_bytes())],
    );
    inbox.spawn(true);
    inbox
        .until("1 本目が読まれない", || async {
            inbox.events("c03-youtube-watch").await == 1
        })
        .await;
    sqlx::query(
        "UPDATE core.event SET deleted_at = now(), deleted_by = 'test'
          WHERE user_id = $1 AND logical_source = 'c03-youtube-watch'",
    )
    .bind(inbox.user)
    .execute(&inbox.pool)
    .await
    .unwrap();

    // 消した記録と同じ内容を含む、中身の違う別の書庫。
    let mixed = format!(
        "[{},{}]",
        body.trim_matches(['[', ']']),
        other.trim_matches(['[', ']'])
    );
    inbox
        .confirm(
            crate::archive::classify::KnownKind::YouTubeWatch,
            mixed.as_bytes(),
        )
        .await;
    inbox.put(
        "takeout-20260914-041200.zip",
        &[("Takeout/YouTube/watch-history.json", mixed.as_bytes())],
    );
    inbox
        .until("2 本目が読まれない", || async {
            inbox.ledger_rows("read").await == 2
        })
        .await;

    assert_eq!(
        inbox.events("c03-youtube-watch").await,
        1,
        "消した記録が別の書庫から戻っている（生きた記録は新しい 1 件だけのはず）"
    );
}

/// Scenario: 壊れた 1 件があっても残りは格納される
#[tokio::test]
async fn archive_flow_one_broken_item_does_not_stop_the_rest() {
    let inbox = Inbox::new("archive-broken-item").await;
    let mut items: Vec<String> = (0..9)
        .map(|n| {
            format!(
                r#"{{"time":"2026-09-1{n}T03:00:00Z","title":"動画{n}","titleUrl":"https://www.youtube.com/watch?v=v{n}"}}"#
            )
        })
        .collect();
    // 10 件目だけ時刻が壊れている。
    items.push(
        r#"{"time":"こわれた","title":"動画9","titleUrl":"https://www.youtube.com/watch?v=v9"}"#
            .to_owned(),
    );
    let body = format!("[{}]", items.join(","));
    inbox
        .confirm(
            crate::archive::classify::KnownKind::YouTubeWatch,
            body.as_bytes(),
        )
        .await;
    inbox.put(
        "takeout-20260913-041200.zip",
        &[("Takeout/YouTube/watch-history.json", body.as_bytes())],
    );
    inbox.spawn(true);

    inbox
        .until("読めた書庫の台帳が残らない", || async {
            inbox.ledger_rows("read").await == 1
        })
        .await;
    assert_eq!(
        inbox.events("c03-youtube-watch").await,
        9,
        "壊れた 1 件のせいで残りの 9 件が入っていない"
    );
}

/// Scenario: 地域は位置から推定されない
#[tokio::test]
async fn archive_flow_region_is_never_inferred_from_a_location() {
    let inbox = Inbox::new("archive-no-region-guess").await;
    // 同じ時刻にニューヨークにいたことを示すタイムラインを先に入れておく。
    let timeline = r#"{"semanticSegments":[{"startTime":"2026-09-12T03:00:00Z","endTime":"2026-09-12T04:00:00Z","visit":{"topCandidate":{"placeLocation":{"latLng":"40.7128°, -74.0060°"}}}}]}"#.as_bytes();
    inbox.put("Timeline.json.zip", &[("Timeline.json", timeline)]);
    inbox.spawn(true);
    inbox
        .until("タイムラインが読まれない", || async {
            inbox.events("c03-timeline-visit").await >= 1
        })
        .await;

    // 時刻が Z で終わる（＝地域を示さない）視聴履歴を置く。
    let body = watch("2026-09-12T03:00:00Z", "ある動画");
    inbox
        .confirm(
            crate::archive::classify::KnownKind::YouTubeWatch,
            body.as_bytes(),
        )
        .await;
    inbox.put(
        "takeout-20260913-041200.zip",
        &[("Takeout/YouTube/watch-history.json", body.as_bytes())],
    );
    inbox
        .until("視聴履歴が読まれない", || async {
            inbox.events("c03-youtube-watch").await == 1
        })
        .await;

    let tz_id: String = sqlx::query_scalar(
        "SELECT tz_id FROM core.event WHERE user_id = $1 AND logical_source = 'c03-youtube-watch'",
    )
    .bind(inbox.user)
    .fetch_one(&inbox.pool)
    .await
    .unwrap();
    assert_eq!(
        tz_id, "UTC",
        "地域を示さない時刻に、位置から推定した地域が入っている"
    );
}

/// Scenario: 同じ名前で置き直しても台帳に 1 行残り取り込み済みへ移る
#[tokio::test]
async fn archive_flow_replacing_the_same_name_adds_one_row_and_moves_the_file() {
    let inbox = Inbox::new("archive-replace-same-name").await;
    let body = watch("2026-09-12T03:00:00Z", "ある動画");
    inbox
        .confirm(
            crate::archive::classify::KnownKind::YouTubeWatch,
            body.as_bytes(),
        )
        .await;
    let entries: [(&str, &[u8]); 1] = [("Takeout/YouTube/watch-history.json", body.as_bytes())];
    inbox.put("takeout-20260912-000000.zip", &entries);
    inbox.spawn(true);

    let processed = inbox.inbox.join("取り込み済み");
    inbox
        .until("書庫が取り込み済みへ移らない", || async {
            processed.join("takeout-20260912-000000.zip").exists()
        })
        .await;

    // 同じ名前・同じ中身をもう一度置く。**zip を作り直さない** —— zip は項目の
    // 更新時刻を埋めるので、作り直すとバイト列が変わり「同じ中身」でなくなる。
    std::fs::copy(
        processed.join("takeout-20260912-000000.zip"),
        inbox.inbox.join("takeout-20260912-000000.zip"),
    )
    .unwrap();
    inbox
        .until("既に読んだ書庫の台帳が残らない", || async {
            inbox.ledger_rows("already_read").await == 1
        })
        .await;
    inbox
        .until(
            "置き直した書庫が取り込み済みへ移らない",
            || async {
                // 取り込み済みに同じ名前があるので 2 つ目は「(2)」が付く。
                processed.join("takeout-20260912-000000 (2).zip").exists()
            },
        )
        .await;
    assert_eq!(
        inbox.ledger_rows("read").await,
        1,
        "置き直しで読めた書庫の行まで増えている"
    );
}

/// 画面の「既に読んだ書庫を置き直すとそれが箱に出る」が読む材料を、サーバ側で固定する。
/// （画面の Scenario そのものは `web/src/__tests__` が担保する）
#[tokio::test]
async fn archive_flow_already_read_status_carries_the_previous_read_time() {
    let inbox = Inbox::new("archive-already-read-status").await;
    let sha = "e".repeat(64);
    let previous: i64 = sqlx::query_scalar(
        "INSERT INTO core.archive_ledger
           (user_id, sha256, parser_version, outcome, file_name, finished_at)
         VALUES ($1, $2, $3, 'read', 'takeout-20260912.zip', '2026-09-12T10:00:00Z')
         RETURNING id",
    )
    .bind(inbox.user)
    .bind(&sha)
    .bind(crate::archive::PARSER_VERSION)
    .fetch_one(&inbox.pool)
    .await
    .unwrap();

    crate::archive::worker::record_already_read(
        &inbox.pool,
        inbox.user,
        &sha,
        Some("takeout-20260912.zip".into()),
    )
    .await
    .unwrap();

    let latest = inbox
        .status()
        .await
        .latest_archive
        .expect("直近の書庫が無い");
    assert_eq!(latest.outcome, "already_read");
    assert_eq!(
        latest.previously_read_at.map(|at| at.to_rfc3339()),
        Some("2026-09-12T10:00:00+00:00".to_owned()),
        "前に読んだ時刻が返っていない（台帳の行 {previous} を指すはず）"
    );

    // 走査を重ねても「既に読んだ」の行は 1 つのまま。
    for _ in 0..3 {
        crate::archive::worker::record_already_read(&inbox.pool, inbox.user, &sha, None)
            .await
            .unwrap();
    }
    assert_eq!(
        inbox.ledger_rows("already_read").await,
        1,
        "走査のたびに「既に読んだ」の行が増えている"
    );
}

/// Scenario: 起動し直しても走査の回数は失われない
/// Scenario: 置き場が読めない日は取れない状態で残る
/// Scenario: 書庫のソースには生存信号が残らない
#[tokio::test]
async fn archive_flow_scan_counts_survive_a_restart() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let day = |day: u32, hour: u32| {
        chrono::DateTime::parse_from_rfc3339(&format!("2026-09-{day:02}T{hour:02}:00:00Z"))
            .unwrap()
            .to_utc()
    };

    // 1 日目: 最初の走査で信号が 1 件残る（この時点で回数は 0 に戻る）。
    crate::archive::worker::record_archive_heartbeat(&pool, user, day(12, 1), true, Vec::new())
        .await
        .unwrap();
    // 同じ日に 4 回走査する。取り込み器を起こし直しても回数は DB にある。
    for hour in 2..6 {
        crate::archive::worker::record_archive_heartbeat(
            &pool,
            user,
            day(12, hour),
            true,
            Vec::new(),
        )
        .await
        .unwrap();
    }
    // 次の日の最初の走査。専用のフォルダが読めない状態で来る。
    crate::archive::worker::record_archive_heartbeat(
        &pool,
        user,
        day(13, 1),
        false,
        vec!["dedicated_inbox_unreadable".into()],
    )
    .await
    .unwrap();

    let (attempts, capturable, blockers): (i32, bool, Vec<String>) = sqlx::query_as(
        "SELECT attempts, capturable, blockers FROM core.heartbeat
          WHERE user_id = $1 AND logical_source = 's01-archive-inbox'
          ORDER BY emitted_at DESC LIMIT 1",
    )
    .bind(user)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(attempts, 5, "起動し直す前の 4 回を含めた 5 になっていない");
    assert!(
        !capturable,
        "専用のフォルダが読めない日が取れる状態になっている"
    );
    assert!(
        blockers.contains(&"dedicated_inbox_unreadable".to_owned()),
        "満たされていないものに専用のフォルダが挙がっていない: {blockers:?}"
    );

    let archive_source_beats: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM core.heartbeat
          WHERE user_id = $1 AND logical_source LIKE 'c03-%'",
    )
    .bind(user)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        archive_source_beats, 0,
        "書庫の論理ソースに生存信号が残っている"
    );
}

/// Scenario: 記録の無い日も同じ判定で出る
#[tokio::test]
async fn archive_flow_a_day_without_records_uses_the_same_judgement() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    // **自分のソースを持つ。** `c03-youtube-watch` を直に使うと、並んで走る
    // 取り込みの試験が同じ行の `collection_started_on` を書き換え、その日が
    // 「導入前」に落ちる（実測: 新しい DB での最初の走りだけ落ちた）。
    // 書庫のソースが登録簿に 60 日で入ること自体は
    // `archive_migration_registers_sources_and_preserves_interval` が固定する。
    const ARCHIVE_GAP_SEC: i32 = 5_184_000;
    let source = testdb::source(&pool, "c03-archive", ARCHIVE_GAP_SEC).await;
    testdb::set_started_on(&pool, &source, "2026-01-01").await;
    // 前後の記録が 60 日以内にあり、その日には記録が無い。
    testdb::put_event(&pool, user, &source, "2026-08-20T12:00:00+09:00").await;
    testdb::put_event(&pool, user, &source, "2026-09-12T12:00:00+09:00").await;

    let days = crate::coverage::of_sources(
        &pool,
        Some(user),
        &[source],
        testdb::date("2026-09-01"),
        testdb::date("2026-09-01"),
    )
    .await
    .unwrap();

    let cell = days
        .first()
        .and_then(|source| source.days.first())
        .expect("その日のセルが無い");
    assert_eq!(
        cell.state,
        crate::coverage::DayState::AliveNoRecord,
        "書庫のソースの、記録が無い日が Must の 5 本と違う判定になっている"
    );
}

/// `tracing` の出力をそのまま集める試験用の層。**書き出しの文字列を丸ごと持つ**ので、
/// 欄の名前でも本文でも、検索語が 1 度でも出れば捕まる。
#[derive(Clone, Default)]
struct CapturedLog(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);

impl std::io::Write for CapturedLog {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if let Ok(mut sink) = self.0.lock() {
            sink.extend_from_slice(buf);
        }
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for CapturedLog {
    type Writer = Self;
    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

/// 試験バイナリ全体のログを集める口。
///
/// **スレッド局所の `set_default` では足りない**（実測: 並列で走らせると、背景の
/// 取り込み器が出すログを 1 行も拾えなかった）。取り込み器は別の task で動くので、
/// 「この試験のスレッドだけ」に仕掛けると、見ていないのに緑になる。
static CAPTURED: std::sync::OnceLock<CapturedLog> = std::sync::OnceLock::new();

fn captured_log() -> &'static CapturedLog {
    CAPTURED.get_or_init(|| {
        use tracing_subscriber::layer::SubscriberExt as _;
        let captured = CapturedLog::default();
        let subscriber = tracing_subscriber::registry().with(
            tracing_subscriber::fmt::layer()
                .with_writer(captured.clone())
                .with_ansi(false),
        );
        // 他の試験も同じ口へ流れる。**この試験にとっては厳しくなる側**なので通す
        // —— どの試験が出したログでも、記録の値が出ていれば落ちる。
        let _ = tracing::subscriber::set_global_default(subscriber);
        captured
    })
}

/// Scenario: 取り込みのログに検索語が出ない
#[tokio::test]
async fn archive_flow_the_log_never_carries_a_search_query() {
    let captured = captured_log().clone();

    let inbox = Inbox::new("archive-private-log").await;
    // 検索語・題名・URL・座標を全部含む書庫を、読める中身と読めない中身の両方で置く。
    let search = r#"[{"time":"2026-09-12T03:00:00Z","title":"祇園 旅館 を検索","titleUrl":"https://www.youtube.com/results?search_query=%E7%A5%87%E5%9C%92+%E6%97%85%E9%A4%A8"}]"#;
    let timeline = r#"{"semanticSegments":[{"startTime":"2026-09-12T03:00:00Z","endTime":"2026-09-12T04:00:00Z","visit":{"topCandidate":{"placeLocation":{"latLng":"35.0116°, 135.7681°"}}}}]}"#;
    inbox
        .confirm(
            crate::archive::classify::KnownKind::YouTubeSearch,
            search.as_bytes(),
        )
        .await;
    inbox.put(
        "takeout-20260913-041200.zip",
        &[
            ("Takeout/YouTube/search-history.json", search.as_bytes()),
            ("Takeout/Timeline.json", timeline.as_bytes()),
            // 読めない中身も混ぜる（失敗の経路のログも見る）。
            ("Takeout/broken.json", b"{ this is not json"),
        ],
    );
    inbox.spawn(true);
    inbox
        .until("書庫が読まれない", || async {
            inbox.ledger_rows("read").await == 1
        })
        .await;
    // **この書庫を読み終えたログが出るまで待つ** —— 台帳の行ができた時点では
    // まだ出ていない。集まる前に見ると、何も見ずに緑になる。
    inbox
        .until(
            "取り込みのログが 1 行も出ない（この試験が何も見ていない）",
            || async {
                let text =
                    String::from_utf8_lossy(&captured.0.lock().unwrap().clone()).into_owned();
                text.contains("archive_inspect")
            },
        )
        .await;

    let text = String::from_utf8_lossy(&captured.0.lock().unwrap().clone()).into_owned();
    for value in [
        "祇園 旅館",
        "%E7%A5%87%E5%9C%92",
        "祇園 旅館 を検索",
        "35.0116",
        "youtube.com",
    ] {
        assert!(
            !text.contains(value),
            "取り込みのログに記録の値「{value}」が出ている:\n{text}"
        );
    }
}

/// Scenario: 読めない形の書庫は台帳に残る
#[tokio::test]
async fn archive_flow_an_unreadable_archive_lands_in_the_ledger_and_the_box() {
    let inbox = Inbox::new("archive-unreadable").await;
    // `.tgz` は読める形ではない。**走査の対象にすらならないと、台帳にも画面にも出ない。**
    std::fs::write(
        inbox.inbox.join("takeout-20260913T041200Z-001.tgz"),
        b"\x1f\x8b\x08 not really a tarball",
    )
    .unwrap();
    inbox.spawn(true);

    inbox
        .until("読めない書庫が台帳に残らない", || async {
            inbox.ledger_rows("unreadable").await == 1
        })
        .await;
    let latest = inbox
        .status()
        .await
        .latest_archive
        .expect("直近の書庫が無い");
    assert_eq!(latest.outcome, "unreadable");
    assert_eq!(
        latest.unreadable_kind.as_deref(),
        Some("unsupported_format"),
        "読めなかった理由の種別が台帳に残っていない"
    );
    assert_eq!(
        latest.file_name.as_deref(),
        Some("takeout-20260913T041200Z-001.tgz")
    );
}

/// Scenario: 壊れた zip は台帳に残る
#[tokio::test]
async fn archive_flow_a_broken_zip_lands_in_the_ledger() {
    let inbox = Inbox::new("archive-broken-zip").await;
    std::fs::write(
        inbox.inbox.join("takeout-20260913T041200Z-002.zip"),
        b"PK\x03\x04 this is not a zip",
    )
    .unwrap();
    inbox.spawn(true);

    inbox
        .until("壊れた zip が台帳に残らない", || async {
            inbox.ledger_rows("unreadable").await == 1
        })
        .await;
    let latest = inbox
        .status()
        .await
        .latest_archive
        .expect("直近の書庫が無い");
    assert_eq!(latest.unreadable_kind.as_deref(), Some("broken_zip"));
}

/// Scenario: 端末から書き出したタイムラインを専用のフォルダに置くと読まれる
#[tokio::test]
async fn archive_flow_a_bare_timeline_json_is_read() {
    let inbox = Inbox::new("archive-bare-timeline").await;
    // **zip に包まない。** 端末はタイムラインを裸の JSON で書き出す（本人の決定 Q8）。
    // **本物の書き出しの形**（時刻と時差はセグメントの側、生の信号は 1 段入れ子。R48）。
    std::fs::write(
        inbox.inbox.join("Timeline.json"),
        crate::archive_tests::REAL_TIMELINE.as_bytes(),
    )
    .unwrap();
    inbox.spawn(true);

    inbox
        .until("裸の Timeline.json が読まれない", || async {
            inbox.events("c03-timeline-visit").await == 1
        })
        .await;
    // 訪問だけでなく、移動・経路の点・生の信号も入る（合成の形では 1 件も入らなかった）
    for (source, expected) in [
        ("c03-timeline-move", 1),
        ("c03-timeline-route", 2),
        ("c03-timeline-signal", 3),
    ] {
        inbox
            .until(&format!("{source} が {expected} 件入らない"), || async {
                inbox.events(source).await == expected
            })
            .await;
    }
    let offsets: Vec<i32> = sqlx::query_scalar(
        "SELECT DISTINCT tz_offset_min FROM core.event
          WHERE user_id = $1 AND logical_source LIKE 'c03-timeline-%'",
    )
    .bind(inbox.user)
    .fetch_all(&inbox.pool)
    .await
    .unwrap();
    assert_eq!(offsets, [540], "セグメントが示した +09:00 で残っていない");
    inbox
        .until("読めた書庫の台帳が残らない", || async {
            inbox.ledger_rows("read").await == 1
        })
        .await;
}

/// Scenario: 名前の時刻を持たない書庫は見つけた時刻を持つ
#[test]
fn archive_flow_created_at_reads_the_real_takeout_name() {
    let discovered = chrono::DateTime::parse_from_rfc3339("2026-09-19T00:00:00Z")
        .unwrap()
        .to_utc();
    // **本物の Takeout の名前**（区切りは `T`、末尾に `Z`）。
    let real = crate::archive::worker::archive_created_at(
        std::path::Path::new("takeout-20260913T041200Z-001.zip"),
        discovered,
    );
    assert_eq!(
        real.to_rfc3339(),
        "2026-09-13T04:12:00+00:00",
        "本物の Takeout の名前から書庫の作られた時刻を取れていない"
    );
    // 名前が時刻を持たないものは見つけた時刻へ落ちる。
    assert_eq!(
        crate::archive::worker::archive_created_at(
            std::path::Path::new("Timeline.json"),
            discovered
        ),
        discovered
    );
}

/// Scenario: 欄の名前が増えただけでは確認待ちにならない
#[test]
fn archive_flow_shape_ignores_item_count_and_order() {
    use crate::archive::classify::KnownKind::MyActivity;
    use crate::archive::worker::{hash_shape, shape_for_file};
    let item = |product: &str| {
        format!(
            r#"{{"header":"{product}","title":"見た","titleUrl":"https://www.google.com/search?q=x","time":"2026-09-12T03:00:00Z","products":["{product}"]}}"#
        )
    };
    let one = format!("[{}]", item("検索"));
    let many = format!("[{},{},{}]", item("検索"), item("検索"), item("検索"));
    let ab = format!("[{},{}]", item("検索"), item("マップ"));
    let ba = format!("[{},{}]", item("マップ"), item("検索"));

    let hash =
        |body: &str| hash_shape(&shape_for_file(MyActivity, "a/b.json", body.as_bytes()).unwrap());
    assert_eq!(
        hash(&one),
        hash(&many),
        "件数が変わっただけで確認待ちになる（2 か月ごとの書き出しのたびに手作業が要る）"
    );
    assert_eq!(hash(&ab), hash(&ba), "並びが変わっただけで確認待ちになる");
    assert_ne!(hash(&one), hash(&ab), "製品が増えても確認待ちにならない");
}

/// Scenario: ずれを持つ時刻はそのずれで残る
/// Scenario: UTC しか持たない時刻には取得元が地域を持たなかった印が付く
#[tokio::test]
async fn archive_flow_keeps_the_offset_the_source_declared() {
    let inbox = Inbox::new("archive-offset").await;
    // **取得元が `+09:00` で書き出した**視聴履歴。地域は位置から推定しないが、
    // 取得元が示したずれは残す（C2）。
    let with_offset = r#"[{"time":"2026-09-12T12:00:00+09:00","title":"ある動画","titleUrl":"https://www.youtube.com/watch?v=abc"}]"#;
    inbox
        .confirm(
            crate::archive::classify::KnownKind::YouTubeWatch,
            with_offset.as_bytes(),
        )
        .await;
    inbox.put(
        "takeout-20260913T041200Z-001.zip",
        &[("Takeout/YouTube/watch-history.json", with_offset.as_bytes())],
    );
    inbox.spawn(true);
    inbox
        .until("書庫が読まれない", || async {
            inbox.events("c03-youtube-watch").await == 1
        })
        .await;

    let (offset, tz_id): (i32, String) = sqlx::query_as(
        "SELECT tz_offset_min, tz_id FROM core.event
          WHERE user_id = $1 AND logical_source = 'c03-youtube-watch'",
    )
    .bind(inbox.user)
    .fetch_one(&inbox.pool)
    .await
    .unwrap();
    assert_eq!(offset, 540, "取得元が示した +09:00 が 0 に畳まれている");
    assert_ne!(tz_id, "UTC", "ずれを持つ時刻が UTC として残っている");
}

/// Scenario: 記録から運んだ書庫が分かる
#[tokio::test]
async fn archive_flow_legacy_records_keep_their_coordinates() {
    let inbox = Inbox::new("archive-legacy-raw").await;
    // 移行前のロケーション履歴の 1 件。**座標と精度を持っている。**
    let records = br#"{"locations":[{"timestampMs":"1622505600000","latitudeE7":356580000,"longitudeE7":1397450000,"accuracy":17}]}"#;
    inbox.put("Records.json.zip", &[("Takeout/Records.json", records)]);
    inbox.spawn(true);
    inbox
        .until(
            "移行前のロケーション履歴が読まれない",
            || async { inbox.events("c03-legacy-location").await == 1 },
        )
        .await;

    let raw: String = sqlx::query_scalar(
        "SELECT raw FROM core.event WHERE user_id = $1 AND logical_source = 'c03-legacy-location'",
    )
    .bind(inbox.user)
    .fetch_one(&inbox.pool)
    .await
    .unwrap();
    // **時刻だけを取り出して捨てない。** 書庫を消した後に座標は作り直せない。
    for kept in ["356580000", "1397450000", "17"] {
        assert!(
            raw.contains(kept),
            "移行前の記録の原文から「{kept}」が落ちている: {raw}"
        );
    }
}

/// Scenario: 置き場が読めない日は取れない状態で残る（どの置き場かまで残す）
#[tokio::test]
async fn archive_flow_names_only_the_unreadable_inbox() {
    let inbox = Inbox::new("archive-which-inbox").await;
    // **専用のフォルダだけ**を消す。ダウンロードのフォルダは読める。
    std::fs::remove_dir_all(&inbox.inbox).unwrap();
    inbox.spawn(true);

    inbox
        .until("取れない状態の生存信号が残らない", || async {
            let got: Option<(bool,)> = sqlx::query_as(
                "SELECT capturable FROM core.heartbeat
                  WHERE user_id = $1 AND logical_source = 's01-archive-inbox'
                  ORDER BY emitted_at DESC LIMIT 1",
            )
            .bind(inbox.user)
            .fetch_optional(&inbox.pool)
            .await
            .unwrap();
            got == Some((false,))
        })
        .await;

    let blockers: Vec<String> = sqlx::query_scalar(
        "SELECT blockers FROM core.heartbeat
          WHERE user_id = $1 AND logical_source = 's01-archive-inbox'
          ORDER BY emitted_at DESC LIMIT 1",
    )
    .bind(inbox.user)
    .fetch_one(&inbox.pool)
    .await
    .unwrap();
    assert_eq!(
        blockers,
        vec!["dedicated_inbox_unreadable".to_owned()],
        "読めない置き場だけを挙げていない（本人が直す場所を誤る）"
    );
}

/// Scenario: 原文は書庫のバイト列の一部と一致する
#[tokio::test]
async fn archive_flow_raw_is_a_slice_of_the_archive_bytes() {
    let inbox = Inbox::new("archive-raw-slice").await;
    // **わざと「書き戻すと変わる」形にする** —— 欄の並びが辞書順でなく、
    // 空白が入っていて、数値が指数表記。解釈して書き戻すとどれも変わる。
    let body = r#"[ {"titleUrl":"https://www.youtube.com/watch?v=abc",  "time":"2026-09-12T03:00:00Z", "zzz":1.0e2, "title":"ある動画"} ]"#;
    inbox
        .confirm(
            crate::archive::classify::KnownKind::YouTubeWatch,
            body.as_bytes(),
        )
        .await;
    inbox.put(
        "takeout-20260913T041200Z-001.zip",
        &[("Takeout/YouTube/watch-history.json", body.as_bytes())],
    );
    inbox.spawn(true);
    inbox
        .until("書庫が読まれない", || async {
            inbox.events("c03-youtube-watch").await == 1
        })
        .await;

    let raw: String = sqlx::query_scalar(
        "SELECT raw FROM core.event WHERE user_id = $1 AND logical_source = 'c03-youtube-watch'",
    )
    .bind(inbox.user)
    .fetch_one(&inbox.pool)
    .await
    .unwrap();
    assert!(
        body.contains(&raw),
        "原文が書庫のバイト列の連続した一部でない（解釈して書き戻している）:\n原文={raw}\n書庫={body}"
    );
    assert!(
        raw.contains("1.0e2"),
        "数値の表記が書き戻しで変わっている: {raw}"
    );
}

/// Scenario: 書庫のソースは 60 日で登録されている
/// Scenario: 取り込み器のソースは 1 日で登録されている
#[tokio::test]
async fn archive_flow_every_archive_source_is_registered_with_sixty_days() {
    let pool = testdb::pool().await;
    // **本人の決定 Q6（粒度は内容の鍵だけ = `none`）と Q7（60 日）の唯一の受け皿。**
    // 本数だけを見ていたときは、10 本すべてを 1 日に書き換えても落ちる試験が無かった。
    const SIXTY_DAYS_SEC: i32 = 5_184_000;
    let rows: Vec<(String, i32, String)> = sqlx::query_as(
        "SELECT logical_source, expected_gap_sec, external_id_kind FROM core.source
          WHERE logical_source LIKE 'c03-%' AND logical_source NOT LIKE 'c03-myactivity-%'
            AND logical_source NOT LIKE 't-%'
          ORDER BY logical_source",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(rows.len(), 10, "書庫の固定ソースは 10 本");
    for (name, gap, kind) in rows {
        assert_eq!(gap, SIXTY_DAYS_SEC, "{name} の想定間隔が 60 日でない");
        assert_eq!(
            kind, "none",
            "{name} の外部識別子の粒度が内容の鍵だけでない"
        );
    }
}

/// Scenario: 書き出しを忘れると書庫のソースは途絶になる
#[tokio::test]
async fn archive_flow_a_forgotten_export_turns_the_archive_source_into_an_outage() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    // **書庫の想定間隔（60 日）を持つソース**で見る。汎用の試験ソースでは、
    // 60 日という値そのものが固定されない。
    let source = testdb::source(&pool, "c03-outage", 5_184_000).await;
    testdb::set_started_on(&pool, &source, "2026-01-01").await;
    testdb::put_event(&pool, user, &source, "2026-07-01T12:00:00+09:00").await;

    let days = crate::coverage::of_sources(
        &pool,
        Some(user),
        &[source],
        testdb::date("2026-08-31"), // 最後の記録から 61 日後
        testdb::date("2026-08-31"),
    )
    .await
    .unwrap();
    let cell = days
        .first()
        .and_then(|source| source.days.first())
        .expect("その日のセルが無い");
    assert_eq!(
        cell.state,
        crate::coverage::DayState::Outage,
        "書き出しを 61 日忘れても途絶にならない（ST14 の通知が鳴らない）"
    );
}

/// Scenario: 台帳に記録の本文は載らない
#[tokio::test]
async fn archive_flow_no_ledger_column_carries_a_record_body() {
    let inbox = Inbox::new("archive-ledger-private").await;
    // **実際に読ませる。** 台帳へ空の行を入れて検索語を探しても、検索語がどこにも
    // 無いので落ちようがない（review R13）。読めない項目も混ぜて、場所の列も見る。
    let search = r#"[{"time":"2026-09-12T03:00:00Z","title":"白川郷 民宿 を検索","titleUrl":"https://www.youtube.com/results?search_query=%E7%99%BD%E5%B7%9D%E9%83%B7"},{"time":"こわれた","titleUrl":"https://www.youtube.com/results?search_query=x"}]"#;
    inbox
        .confirm(
            crate::archive::classify::KnownKind::YouTubeSearch,
            search.as_bytes(),
        )
        .await;
    inbox.put(
        "takeout-20260913T041200Z-001.zip",
        &[("Takeout/YouTube/search-history.json", search.as_bytes())],
    );
    inbox.spawn(true);
    inbox
        .until("書庫が読まれない", || async {
            inbox.ledger_rows("read").await == 1
        })
        .await;

    // Scenario: 壊れた 1 件の場所が台帳に残る
    let (count, at): (i32, Option<String>) = sqlx::query_as(
        "SELECT unreadable_count, unreadable_at FROM core.archive_ledger
          WHERE user_id = $1 AND outcome = 'read'",
    )
    .bind(inbox.user)
    .fetch_one(&inbox.pool)
    .await
    .unwrap();
    assert_eq!(count, 1, "読めなかった項目の件数が台帳に残っていない");
    let at = at.expect("読めなかった項目の場所が台帳に残っていない");
    assert!(
        at.contains("search-history.json") && at.contains('#'),
        "場所がファイル名とファイルの中の位置になっていない: {at}"
    );

    // **台帳の 3 表を丸ごと文字列にして見る**（列を足しても漏れない形）。
    // `archive_ledger_source` は利用者の列を持たないので、台帳の行から辿る。
    for (table, scope) in [
        ("core.archive_ledger", "t.user_id = $1"),
        (
            "core.archive_ledger_source",
            "EXISTS (SELECT 1 FROM core.archive_ledger l WHERE l.id = t.ledger_id AND l.user_id = $1)",
        ),
        ("core.archive_file", "t.user_id = $1"),
    ] {
        let dumped: Option<String> = sqlx::query_scalar(&format!(
            "SELECT string_agg(row_to_json(t)::text, ' ') FROM {table} t WHERE {scope}"
        ))
        .bind(inbox.user)
        .fetch_one(&inbox.pool)
        .await
        .unwrap();
        let dumped = dumped.unwrap_or_default();
        for value in ["白川郷", "民宿", "%E7%99%BD%E5%B7%9D%E9%83%B7"] {
            assert!(
                !dumped.contains(value),
                "{table} に記録の本文「{value}」が載っている: {dumped}"
            );
        }
    }
}

/// Scenario: 形の確認の出力に見分けた中身と製品の名前と件数が出る
#[test]
fn archive_flow_shape_shows_what_the_human_needs_to_judge() {
    use crate::archive::classify::KnownKind::MyActivity;
    use crate::archive::worker::shape_for_file;
    // **合成のファイルから作る。** 自分で書いた JSON を往復させても、
    // `shape_for_file` が何を出すかは 1 度も見ていない（review R15）。
    let item = |product: &str| {
        format!(
            r#"{{"header":"{product}","title":"京都 旅館 を検索","titleUrl":"https://www.google.com/search?q=kyoto","time":"2026-09-12T03:00:00Z","products":["{product}"]}}"#
        )
    };
    let body = format!("[{},{},{}]", item("検索"), item("マップ"), item("検索"));

    let shape = shape_for_file(
        MyActivity,
        "Takeout/My Activity/検索/活動.json",
        body.as_bytes(),
    )
    .unwrap();

    // 判断の材料が出る。
    assert_eq!(shape["kind"], "MyActivity");
    assert_eq!(shape["items"], 3, "件数が出ていない");
    assert_eq!(
        shape["path_shape"], "depth=3;ext=json",
        "書庫の中のパスの型が出ていない"
    );
    let products = shape["products"]
        .as_array()
        .expect("製品の名前が出ていない");
    assert_eq!(
        products.len(),
        2,
        "製品の名前が集合になっていない: {products:?}"
    );
    assert!(shape["field_names"]
        .as_array()
        .is_some_and(|names| names.iter().any(|name| name == "products")));

    // **値は出ない。**
    let printed = serde_json::to_string(&shape).unwrap();
    for value in ["京都 旅館", "google.com", "2026-09-12T03:00:00Z"] {
        assert!(
            !printed.contains(value),
            "形の出力に記録の値「{value}」が出ている: {printed}"
        );
    }
}

/// Scenario: 印を置くと確認待ちの書庫が格納される
/// Scenario: 印を置いた後の読み直しは台帳に 1 行足す
#[tokio::test]
async fn archive_flow_confirming_a_shape_ingests_a_takeout_archive() {
    let inbox = Inbox::new("archive-confirm-only-pending").await;
    // **確認待ちの中身だけ**の書庫。1 回目の読みは `read` の行を残さない。
    let activity = myactivity("Discover");
    inbox.put(
        "takeout-20260912T000000Z-001.zip",
        &[(
            "Takeout/My Activity/Discover/活動.json",
            activity.as_bytes(),
        )],
    );
    inbox.spawn(true);
    inbox
        .until("確認待ちの台帳が残らない", || async {
            inbox.ledger_rows("pending_shape").await == 1
        })
        .await;
    assert_eq!(inbox.ledger_rows("read").await, 0, "印の前に格納されている");

    // **ここで印を置く**（本人が `tools/archive-shape.sh --confirm` を叩いた状態）。
    inbox
        .confirm(
            crate::archive::classify::KnownKind::MyActivity,
            activity.as_bytes(),
        )
        .await;

    inbox
        .until(
            "印を置いても確認待ちの中身が格納されない",
            || async { inbox.events("c03-myactivity-discover").await == 1 },
        )
        .await;
    // 台帳の行は格納の後に入るので、イベントが見えた直後には無いことがある。待つ。
    inbox
        .until(
            "印を置いた後の読み直しが台帳に 1 行足していない",
            || async { inbox.ledger_rows("read").await == 1 },
        )
        .await;
    inbox
        .until(
            "格納した後も確認待ちの待ち行列に残っている",
            || async {
                let left: i64 = sqlx::query_scalar(
                    "SELECT count(*) FROM core.archive_pending_shape WHERE user_id = $1",
                )
                .bind(inbox.user)
                .fetch_one(&inbox.pool)
                .await
                .unwrap();
                left == 0
            },
        )
        .await;
}

/// Scenario: 知らない製品があっても同じ書庫の他のファイルは格納される
#[tokio::test]
async fn archive_flow_confirming_a_shape_ingests_a_mixed_archive() {
    let inbox = Inbox::new("archive-confirm-mixed").await;
    // **混在した書庫**（印の要らない Timeline + 印の要るマイアクティビティ）。
    // 本物の Takeout はこの形なので、ここが通らないと通常経路が入らない。
    let timeline = r#"{"semanticSegments":[{"startTime":"2026-09-12T03:00:00Z","endTime":"2026-09-12T04:00:00Z","visit":{"topCandidate":{"placeLocation":{"latLng":"35.0116°, 135.7681°"}}}}]}"#.as_bytes();
    let activity = myactivity("Discover");
    inbox.put(
        "takeout-20260912T000000Z-001.zip",
        &[
            ("Takeout/Timeline.json", timeline),
            (
                "Takeout/My Activity/Discover/活動.json",
                activity.as_bytes(),
            ),
        ],
    );
    inbox.spawn(true);
    inbox
        .until("混在した書庫が読まれない", || async {
            inbox.events("c03-timeline-visit").await == 1
        })
        .await;
    inbox
        .until("確認待ちが積まれない", || async {
            let n: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM core.archive_pending_shape WHERE user_id = $1",
            )
            .bind(inbox.user)
            .fetch_one(&inbox.pool)
            .await
            .unwrap();
            n == 1
        })
        .await;

    inbox
        .confirm(
            crate::archive::classify::KnownKind::MyActivity,
            activity.as_bytes(),
        )
        .await;

    inbox
        .until(
            "印を置いても確認待ちの中身が格納されない",
            || async { inbox.events("c03-myactivity-discover").await == 1 },
        )
        .await;
    // **台帳の行は増えない**（design D18）—— 1 回目の読みで既に `read` の行がある。
    // 増やすと「同じ書庫をもう一度置いても行が増えない」と衝突する。
    let ledger_sources = || async {
        let got: Vec<String> = sqlx::query_scalar(
            "SELECT ls.logical_source FROM core.archive_ledger_source ls
               JOIN core.archive_ledger l ON l.id = ls.ledger_id
              WHERE l.user_id = $1 ORDER BY ls.logical_source",
        )
        .bind(inbox.user)
        .fetch_all(&inbox.pool)
        .await
        .unwrap();
        got
    };
    // 記録が入ってからソース別の台帳が書かれるまでに間がある。
    inbox
        .until(
            "読み直したぶんがソース別の台帳に残らない",
            || async {
                ledger_sources()
                    .await
                    .iter()
                    .any(|s| s.starts_with("c03-myactivity-"))
            },
        )
        .await;
    assert_eq!(inbox.ledger_rows("read").await, 1);
}

/// Scenario: マイアクティビティの製品ごとに論理ソースが分かれる
#[tokio::test]
async fn archive_flow_myactivity_display_name_uses_the_product() {
    let inbox = Inbox::new("archive-myactivity-name").await;
    // **非 ASCII の製品名。** 論理ソース名は安定したハッシュになるので、
    // 表示名にそれを入れると見出しが `マイアクティビティ: c03-myactivity-u…` になる。
    //
    // **製品名は走りごとに変える。** `core.source` は利用者で分かれていない
    // （review I10）ので、固定名だと他の走りが先に登録した行に当たって、
    // 直っていなくても緑になる。
    let product = format!("マップ{}", uuid::Uuid::new_v4().simple());
    let activity = myactivity(&product);
    inbox
        .confirm(
            crate::archive::classify::KnownKind::MyActivity,
            activity.as_bytes(),
        )
        .await;
    inbox.put(
        "takeout-20260912T000000Z-001.zip",
        &[("Takeout/My Activity/マップ/活動.json", activity.as_bytes())],
    );
    inbox.spawn(true);

    let expected = crate::archive::myactivity::source_name(&product);
    inbox
        .until("マイアクティビティが格納されない", || async {
            inbox.events(&expected).await == 1
        })
        .await;
    let display: String =
        sqlx::query_scalar("SELECT display_name FROM core.source WHERE logical_source = $1")
            .bind(&expected)
            .fetch_one(&inbox.pool)
            .await
            .unwrap();
    assert_eq!(
        display,
        format!("マイアクティビティ: {product}"),
        "画面の見出しに論理ソース名が出ている（本人には読めない）"
    );
}

/// Scenario: ずれを持つ時刻はそのずれで残る（30 分刻みの地域）
#[test]
fn archive_flow_half_hour_offsets_keep_their_minutes() {
    let half = crate::archive::timezone::from_rfc3339("2026-09-12T12:00:00+05:30").unwrap();
    assert_eq!(half.offset_min, 330);
    assert!(
        !half.id.contains("GMT-5"),
        "30 分を切り捨てた地域が入っている: {}",
        half.id
    );
    // 正時のずれはこれまでどおり。
    let whole = crate::archive::timezone::from_rfc3339("2026-09-12T12:00:00+09:00").unwrap();
    assert_eq!((whole.offset_min, whole.id.as_str()), (540, "Etc/GMT-9"));
}

/// Scenario: 直近に置いた書庫の結果が箱に出る
///
/// （画面は `user_id` を付けずに呼ぶ。）
/// 画面（`web/src/App.tsx`）は他の読み出しと同じく `user_id` を付けない。ここだけ必須に
/// していたときは axum の `Query` が 400 を返し、箱はいつも「読み出せませんでした」だった
/// （final review R47）。ハンドラを直に呼ぶ試験はこの経路を通らないので、口から叩く。
#[tokio::test]
async fn archive_flow_status_answers_without_a_user_id() {
    use tower::ServiceExt as _;
    let pool = testdb::pool().await;
    let token = "test-token-0123456789abcdef";
    let app = crate::App::for_test(pool, token);
    let response = crate::router(app)
        .oneshot(
            axum::http::Request::builder()
                .uri("/archives/status")
                .header("authorization", format!("Bearer {token}"))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.status(),
        axum::http::StatusCode::OK,
        "`user_id` の無い問い合わせを断っている（画面の箱が必ず「読み出せませんでした」になる）"
    );
}

/// Scenario: UTC しか持たない時刻には取得元が地域を持たなかった印が付く
/// Scenario: UTC しか持たない時刻は UTC で残る
///
/// **格納された payload を読む**（final review R54）。印の名前が試験の名前にだけあり、
/// 中身を見ていなかった。
#[tokio::test]
async fn archive_flow_stored_payload_marks_a_utc_only_time() {
    let inbox = Inbox::new("archive-payload-utc").await;
    let body = watch("2026-09-12T03:00:00Z", "ある動画");
    inbox
        .confirm(
            crate::archive::classify::KnownKind::YouTubeWatch,
            body.as_bytes(),
        )
        .await;
    inbox.put(
        "takeout-20260913T041200Z-001.zip",
        &[("Takeout/YouTube/watch-history.json", body.as_bytes())],
    );
    inbox.spawn(true);
    inbox
        .until("書庫が読まれない", || async {
            inbox.events("c03-youtube-watch").await == 1
        })
        .await;
    let (offset, tz_id, payload): (i32, String, serde_json::Value) = sqlx::query_as(
        "SELECT tz_offset_min, tz_id, payload FROM core.event
          WHERE user_id = $1 AND logical_source = 'c03-youtube-watch'",
    )
    .bind(inbox.user)
    .fetch_one(&inbox.pool)
    .await
    .unwrap();
    assert_eq!((offset, tz_id.as_str()), (0, "UTC"));
    assert_eq!(
        payload["tz_from_source"],
        serde_json::json!(false),
        "取得元が地域を持たなかった印が payload に無い: {payload}"
    );
    assert_eq!(
        payload["parser_version"],
        serde_json::json!(crate::archive::PARSER_VERSION)
    );
    assert_eq!(payload["url"], "https://www.youtube.com/watch?v=abc");
}

impl Inbox {
    async fn pending_files(&self) -> i64 {
        sqlx::query_scalar("SELECT count(*) FROM core.archive_pending_shape WHERE user_id = $1")
            .bind(self.user)
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }

    /// 写しの置き場にある実体の数（目録の行ではなく、ファイルそのもの）。
    fn copy_files(&self) -> usize {
        fn walk(dir: &Path) -> usize {
            std::fs::read_dir(dir)
                .map(|entries| {
                    entries
                        .filter_map(Result::ok)
                        .map(|entry| {
                            let path = entry.path();
                            if path.is_dir() {
                                walk(&path)
                            } else {
                                1
                            }
                        })
                        .sum()
                })
                .unwrap_or(0)
        }
        walk(&self.copies)
    }
}

/// マイアクティビティの 1 ファイルに複数の製品を持たせる（製品の集合が形になる）。
fn myactivity_products(products: &[&str]) -> String {
    let rows: Vec<String> = products
        .iter()
        .enumerate()
        .map(|(i, product)| {
            format!(
                r#"{{"header":"{product}","title":"見た {i}","titleUrl":"https://www.google.com/search?q={i}","time":"2026-09-1{i}T03:00:00Z","products":["{product}"]}}"#
            )
        })
        .collect();
    format!("[{}]", rows.join(","))
}

/// Scenario: 印を置くと確認待ちの書庫が格納される
///
/// 同じ書庫の中で、印のあるファイルと確認待ちのファイルが同じ論理ソースを作るとき、
/// 読み直しがソース別の台帳の鍵 `(ledger_id, logical_source)` に当たって関数ごと抜け、
/// **毎走査同じ行から始まって永久に止まっていた**（final review R50）。
#[tokio::test]
async fn archive_flow_confirming_never_sticks_on_a_source_the_first_read_wrote() {
    let inbox = Inbox::new("archive-confirm-overlap").await;
    let known = myactivity("Search");
    let mixed = myactivity_products(&["Search", "Discover"]);
    inbox
        .confirm(
            crate::archive::classify::KnownKind::MyActivity,
            known.as_bytes(),
        )
        .await;
    inbox.put(
        "takeout-20260912T000000Z-001.zip",
        &[
            (
                "Takeout/My Activity/Search/MyActivity.json",
                known.as_bytes(),
            ),
            (
                "Takeout/My Activity/Discover/MyActivity.json",
                mixed.as_bytes(),
            ),
        ],
    );
    inbox.spawn(true);
    inbox
        .until("確認待ちが積まれない", || async {
            inbox.pending_files().await == 1
        })
        .await;
    inbox
        .confirm(
            crate::archive::classify::KnownKind::MyActivity,
            mixed.as_bytes(),
        )
        .await;
    inbox
        .until(
            "印を置いても確認待ちの中身が格納されない",
            || async { inbox.events("c03-myactivity-discover").await == 1 },
        )
        .await;
    inbox
        .until(
            "読み直しが止まり、確認待ちが永久に残っている",
            || async { inbox.pending_files().await == 0 },
        )
        .await;
}

/// Scenario: 印を置くと確認待ちの書庫が格納される
///
/// 中身が同じファイルを持つ 2 冊目の書庫は、写しの目録の鍵 `(user_id, sha256)` で
/// 目録が落ち、読み直しの JOIN に当たらず**永久に確認待ちに残っていた**（final review R51）。
#[tokio::test]
async fn archive_flow_a_second_archive_with_the_same_file_is_read_after_confirming() {
    let inbox = Inbox::new("archive-confirm-same-file").await;
    let activity = myactivity("Discover");
    let first_watch = watch("2026-09-10T03:00:00Z", "一冊目");
    let second_watch = watch("2026-09-11T03:00:00Z", "二冊目");
    inbox
        .confirm(
            crate::archive::classify::KnownKind::YouTubeWatch,
            first_watch.as_bytes(),
        )
        .await;
    inbox.put(
        "takeout-20260912T000000Z-001.zip",
        &[
            ("Takeout/YouTube/watch-history.json", first_watch.as_bytes()),
            (
                "Takeout/My Activity/Discover/活動.json",
                activity.as_bytes(),
            ),
        ],
    );
    inbox.spawn(true);
    inbox
        .until("1 冊目が確認待ちにならない", || async {
            inbox.pending_files().await == 1
        })
        .await;
    inbox.put(
        "takeout-20260913T000000Z-001.zip",
        &[
            (
                "Takeout/YouTube/watch-history.json",
                second_watch.as_bytes(),
            ),
            (
                "Takeout/My Activity/Discover/活動.json",
                activity.as_bytes(),
            ),
        ],
    );
    inbox
        .until("2 冊目が確認待ちにならない", || async {
            inbox.pending_files().await == 2
        })
        .await;
    inbox
        .confirm(
            crate::archive::classify::KnownKind::MyActivity,
            activity.as_bytes(),
        )
        .await;
    inbox
        .until("2 冊目の確認待ちが永久に残る", || async {
            inbox.pending_files().await == 0
        })
        .await;
}

/// Scenario: 確認待ちの書庫は走査を重ねても台帳の行が増えない
///
/// 確認待ちだけの書庫は `read` の行を持たないので、走査が毎回「読む」へ回し、
/// 120 秒ごとに書庫全体を展開し直していた（final review R52）。
#[tokio::test]
async fn archive_flow_a_pending_only_archive_is_not_read_again_on_every_scan() {
    let inbox = Inbox::new("archive-pending-rescan").await;
    let activity = myactivity("Discover");
    inbox.put_downloads(
        "takeout-20260912T000000Z-001.zip",
        &[(
            "Takeout/My Activity/Discover/活動.json",
            activity.as_bytes(),
        )],
    );
    inbox.spawn(true);
    inbox
        .until("確認待ちの台帳が残らない", || async {
            inbox.ledger_rows("pending_shape").await == 1
        })
        .await;
    let config = inbox.config(true);
    for _ in 0..2 {
        let candidates = crate::archive::scan::scan_once(&inbox.pool, &config, inbox.user)
            .await
            .unwrap();
        assert!(
            candidates
                .iter()
                .all(|c| c.disposition != crate::archive::scan::ScanDisposition::Read),
            "確認待ちだけの書庫が走査のたびに読み直しへ回っている"
        );
    }
}

/// Scenario: 確認待ちのために作った写しは読み直した後に消える
///
/// 本人の決定（第 2 回 Q11 の補足の読み / spec）。消す側が無かった（final review R56）。
#[tokio::test]
async fn archive_flow_pending_copies_are_removed_after_rereading_when_copies_are_off() {
    let inbox = Inbox::new("archive-pending-copy-removed").await;
    let activity = myactivity("Discover");
    inbox.put(
        "takeout-20260912T000000Z-001.zip",
        &[(
            "Takeout/My Activity/Discover/活動.json",
            activity.as_bytes(),
        )],
    );
    inbox.spawn(false);
    inbox
        .until("確認待ちの台帳が残らない", || async {
            inbox.ledger_rows("pending_shape").await == 1
        })
        .await;
    assert_eq!(inbox.copy_files(), 1, "確認待ちの写しが作られていない");
    inbox
        .confirm(
            crate::archive::classify::KnownKind::MyActivity,
            activity.as_bytes(),
        )
        .await;
    inbox
        .until("印を置いても格納されない", || async {
            inbox.events("c03-myactivity-discover").await == 1
        })
        .await;
    inbox
        .until(
            "残さない設定で、確認待ちのために作った写しが残っている",
            || async { inbox.copy_files() == 0 },
        )
        .await;
}

/// Scenario: 印を置いた後の読み直しは台帳に 1 行足す
///
/// 読み直しが書く `read` の行に、置き場の名前・作られた時刻・置き場の種類を引き継ぐ
/// （NULL と既定の `inbox` だった。final review R58）。
#[tokio::test]
async fn archive_flow_the_reread_ledger_row_keeps_the_archive_name_and_place() {
    let inbox = Inbox::new("archive-reread-row").await;
    let activity = myactivity("Discover");
    inbox.put_downloads(
        "takeout-20260912T010203Z-001.zip",
        &[(
            "Takeout/My Activity/Discover/活動.json",
            activity.as_bytes(),
        )],
    );
    inbox.spawn(true);
    inbox
        .until("確認待ちの台帳が残らない", || async {
            inbox.ledger_rows("pending_shape").await == 1
        })
        .await;
    inbox
        .confirm(
            crate::archive::classify::KnownKind::MyActivity,
            activity.as_bytes(),
        )
        .await;
    inbox
        .until("読み直しの行が無い", || async {
            inbox.ledger_rows("read").await == 1
        })
        .await;
    let (file_name, created_at, inbox_kind): (
        Option<String>,
        Option<chrono::DateTime<chrono::Utc>>,
        String,
    ) = sqlx::query_as(
        "SELECT file_name, created_at, inbox_kind FROM core.archive_ledger
          WHERE user_id = $1 AND outcome = 'read'",
    )
    .bind(inbox.user)
    .fetch_one(&inbox.pool)
    .await
    .unwrap();
    assert_eq!(
        file_name.as_deref(),
        Some("takeout-20260912T010203Z-001.zip")
    );
    assert_eq!(
        created_at.map(|t| t.to_rfc3339()).as_deref(),
        Some("2026-09-12T01:02:03+00:00")
    );
    assert_eq!(inbox_kind, "downloads");
}

/// Scenario: 解析器の版が上がると読み直される
///
/// 本番の走査から呼ばれていなかった（final review R53）。専用のフォルダの書庫は
/// 「取り込み済み」へ移って走査されないので、**写しから**読み直すしか道が無い（D8）。
#[tokio::test]
async fn archive_flow_a_parser_version_bump_rereads_from_the_copies() {
    let inbox = Inbox::new("archive-version-bump").await;
    let body = watch("2026-09-12T03:00:00Z", "ある動画");
    inbox
        .confirm(
            crate::archive::classify::KnownKind::YouTubeWatch,
            body.as_bytes(),
        )
        .await;
    // **古い版で読み終えた状態**を作る: 台帳に古い版の `read` と、写しと目録。
    let archive_sha = format!("{:064x}", uuid::Uuid::new_v4().as_u128());
    std::fs::create_dir_all(&inbox.copies).unwrap();
    let stored = crate::archive::worker::copy_known_file(&inbox.copies, body.as_bytes()).unwrap();
    crate::archive::worker::record_copy(
        &inbox.pool,
        inbox.user,
        stored.file_name().unwrap().to_str().unwrap().to_owned(),
        "Takeout/YouTube/watch-history.json",
        &stored,
        &archive_sha,
    )
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO core.archive_ledger (user_id, sha256, parser_version, outcome, file_name)
         VALUES ($1, $2, 'older', 'read', 'takeout-old.zip')",
    )
    .bind(inbox.user)
    .bind(&archive_sha)
    .execute(&inbox.pool)
    .await
    .unwrap();

    inbox.spawn(true);
    inbox
        .until(
            "版を上げても写しから読み直されない",
            || async {
                let rows: i64 = sqlx::query_scalar(
                    "SELECT count(*) FROM core.archive_ledger
                  WHERE user_id = $1 AND sha256 = $2 AND parser_version = $3 AND outcome = 'read'",
                )
                .bind(inbox.user)
                .bind(&archive_sha)
                .bind(crate::archive::PARSER_VERSION)
                .fetch_one(&inbox.pool)
                .await
                .unwrap();
                rows == 1
            },
        )
        .await;
    assert_eq!(inbox.events("c03-youtube-watch").await, 1);
}

/// Scenario: 置き場が読めないことが画面に出る
///
/// 生存信号は日に 1 回なので、昼に置き場が読めなくなっても、箱が信号だけを見ていると
/// 翌日まで出なかった（final review R59 / design D19（仮））。箱は**直近の走査**を見る。
#[tokio::test]
async fn archive_flow_the_box_shows_an_unreadable_inbox_on_the_same_day() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    let at = |hour: u32| {
        chrono::DateTime::parse_from_rfc3339(&format!("2026-09-12T{hour:02}:00:00Z"))
            .unwrap()
            .to_utc()
    };
    // 朝の最初の走査は読めた（その日の信号はこれ 1 件）
    crate::archive::worker::record_archive_heartbeat(&pool, user, at(0), true, Vec::new())
        .await
        .unwrap();
    // 昼に専用のフォルダが読めなくなった
    crate::archive::worker::record_archive_heartbeat(
        &pool,
        user,
        at(3),
        false,
        vec!["dedicated_inbox_unreadable".into()],
    )
    .await
    .unwrap();
    let inbox = crate::archives_status_for(&pool, user, None)
        .await
        .unwrap()
        .inbox
        .expect("取り込み器の状態が無い");
    assert!(!inbox.capturable, "昼に読めなくなった置き場が箱に出ない");
    assert_eq!(inbox.blockers, ["dedicated_inbox_unreadable"]);
    assert_eq!(
        inbox.emitted_at,
        at(3),
        "最後の確認が直近の走査になっていない"
    );
}

/// 端末で書き出した合成の `Timeline.json`。位置は消した滞在（2026-09-12 12:00〜13:00 JST = 03:00Z〜04:00Z）に対して
/// 端が触れる訪問・触れる移動・内側の信号・内側の経路の点・外の訪問・外の信号・外の経路の点を持つ
/// （位置は 7 行。印が付くのは 4 行。経路の点は code-verify 第 4 回 R79 で足した）。
const ERASE_TIMELINE: &str = r#"{"semanticSegments":[
  {"startTime":"2026-09-12T02:00:00Z","endTime":"2026-09-12T03:00:00Z","visit":{"topCandidate":{"placeLocation":{"latLng":"35.658°, 139.745°"}}}},
  {"startTime":"2026-09-12T04:00:00Z","endTime":"2026-09-12T05:00:00Z","activity":{"start":{"latLng":"35.658°, 139.745°"},"end":{"latLng":"35.660°, 139.750°"},"topCandidate":{"type":"WALKING"}}},
  {"startTime":"2026-09-12T06:00:00Z","endTime":"2026-09-12T07:00:00Z","visit":{"topCandidate":{"placeLocation":{"latLng":"35.670°, 139.760°"}}}},
  {"startTime":"2026-09-12T03:00:00Z","endTime":"2026-09-12T09:00:00Z","timelinePath":[
    {"point":"35.659°, 139.746°","time":"2026-09-12T03:40:00Z"},
    {"point":"35.671°, 139.761°","time":"2026-09-12T08:30:00Z"}
  ]}
 ],
 "rawSignals":[
  {"position":{"LatLng":"35.658°, 139.745°","timestamp":"2026-09-12T03:30:00Z"}},
  {"position":{"LatLng":"35.658°, 139.745°","timestamp":"2026-09-12T08:00:00Z"}}
 ]}"#;

/// 消した滞在の外だけを含む `Timeline.json`。
const OUTSIDE_TIMELINE: &str = r#"{"semanticSegments":[
  {"startTime":"2026-09-12T06:00:00Z","endTime":"2026-09-12T07:00:00Z","visit":{"topCandidate":{"placeLocation":{"latLng":"35.670°, 139.760°"}}}}
 ],
 "rawSignals":[
  {"position":{"LatLng":"35.658°, 139.745°","timestamp":"2026-09-12T08:00:00Z"}}
 ]}"#;

impl Inbox {
    /// 2026-09-12 12:00〜13:00 JST の滞在を 1 件置く（`deletion::erase` が消せる、本物の導出行）。
    async fn put_stay(&self) -> uuid::Uuid {
        self.put_stay_at("2026-09-12T03:00:00+00:00", "2026-09-12T04:00:00+00:00")
            .await
    }

    /// 始まりと終わり（オフセット付きの時刻）を指定して滞在を 1 件置く。
    async fn put_stay_at(&self, start: &str, end: &str) -> uuid::Uuid {
        let id = uuid::Uuid::new_v4();
        let payload = serde_json::json!({
            "start": start,
            "end": end,
            "lat": 35.658,
            "lon": 139.745,
        })
        .to_string();
        sqlx::query(
            "INSERT INTO core.event
               (id, user_id, logical_source, external_id, origin, event_time,
                tz_offset_min, tz_id, schema_version, content_hash, raw, payload)
             VALUES ($1,$2,'s01-stay',$3,'derived',$5::timestamptz,540,'Asia/Tokyo',1,$3,$4,$4::jsonb)",
        )
        .bind(id)
        .bind(self.user)
        .bind(id.to_string())
        .bind(payload)
        .bind(start)
        .execute(&self.pool)
        .await
        .unwrap();
        id
    }

    async fn stays(&self) -> i64 {
        sqlx::query_scalar(
            "SELECT count(*) FROM core.event WHERE user_id = $1 AND logical_source = 's01-stay'",
        )
        .bind(self.user)
        .fetch_one(&self.pool)
        .await
        .unwrap()
    }

    /// 書庫の位置の論理ソース 7 本のうち、印の付いた行と生きた行の件数。
    async fn archive_locations(&self) -> (i64, i64) {
        let hidden: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM core.event
              WHERE user_id = $1 AND logical_source = ANY($2) AND deleted_at IS NOT NULL",
        )
        .bind(self.user)
        .bind(crate::archive::LOCATION_SOURCES.to_vec())
        .fetch_one(&self.pool)
        .await
        .unwrap();
        let live: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM core.event_live
              WHERE user_id = $1 AND logical_source = ANY($2)",
        )
        .bind(self.user)
        .bind(crate::archive::LOCATION_SOURCES.to_vec())
        .fetch_one(&self.pool)
        .await
        .unwrap();
        (hidden, live)
    }

    async fn marks(&self) -> Vec<(String, Option<String>)> {
        sqlx::query_as(
            "SELECT logical_source, deleted_by FROM core.event
              WHERE user_id = $1 AND logical_source = ANY($2) ORDER BY logical_source, event_time",
        )
        .bind(self.user)
        .bind(crate::archive::LOCATION_SOURCES.to_vec())
        .fetch_all(&self.pool)
        .await
        .unwrap()
    }

    async fn erase_ledger_rows(&self, cause: uuid::Uuid, mark: &str) -> i64 {
        sqlx::query_scalar(
            "SELECT count(*) FROM core.deletion_ledger
              WHERE user_id = $1 AND cause_event_id = $2 AND action = 'erase' AND mark = $3
                AND logical_source = ANY($4)",
        )
        .bind(self.user)
        .bind(cause)
        .bind(mark)
        .bind(crate::archive::LOCATION_SOURCES.to_vec())
        .fetch_one(&self.pool)
        .await
        .unwrap()
    }

    /// 書庫が置き場から「取り込み済み」へ移るまで待つ（格納・印付け・台帳の後）。位置の行数も確かめる。
    async fn until_archive_locations(&self, total: i64) {
        let processed = self.inbox.join("取り込み済み").join("Timeline.json.zip");
        self.until("Timeline.json の位置が格納されない", || async {
            let (hidden, live) = self.archive_locations().await;
            processed.exists() && hidden + live >= total
        })
        .await;
    }
}

// Scenario: 消した滞在の時間帯に書庫から入る位置は削除済みになる
// Scenario: 書庫の位置の印は滞在の作り直しを待たずに付く
#[tokio::test]
async fn archive_erased_window_marks_the_overlapping_archive_locations() {
    let inbox = Inbox::new("archive-erased-window").await;
    let stay = inbox.put_stay().await;
    crate::deletion::erase(&inbox.pool, stay, None)
        .await
        .unwrap();
    inbox.put(
        "Timeline.json.zip",
        &[("Timeline.json", ERASE_TIMELINE.as_bytes())],
    );
    inbox.spawn(true);
    inbox.until_archive_locations(7).await;

    // 端が触れるだけの訪問（〜03:00Z）と移動（04:00Z〜）、内側の信号に印が付く。外の 2 行は生きる。
    let marks = inbox.marks().await;
    let late = |source: &str| {
        marks
            .iter()
            .filter(|(s, by)| s == source && by.as_deref() == Some("user:late"))
            .count()
    };
    assert_eq!(
        late("c03-timeline-visit"),
        1,
        "終わりが触れる訪問に印が無い: {marks:?}"
    );
    assert_eq!(
        late("c03-timeline-move"),
        1,
        "始まりが触れる移動に印が無い: {marks:?}"
    );
    assert_eq!(
        late("c03-timeline-signal"),
        1,
        "内側の信号に印が無い: {marks:?}"
    );
    assert_eq!(
        late("c03-timeline-route"),
        1,
        "内側の経路の点に印が無い: {marks:?}"
    );
    assert_eq!(
        inbox.archive_locations().await,
        (4, 3),
        "生きた行の件数が違う"
    );
    assert_eq!(
        inbox.erase_ledger_rows(stay, "user:late").await,
        4,
        "台帳の erase 行が違う"
    );
    // 作り直しを走らせていない（final review 第 2 回 R77）。作り直しは最初に基準の版を書き
    // （`ensure_criteria`）、置き換えた滞在に `rebuild:` の印を付ける。どちらも無く、消した滞在は本人の印のまま。
    assert_eq!(
        inbox.rebuild_traces().await,
        (0, 0),
        "滞在の作り直しが走っている（基準の版の行 / rebuild: の印）"
    );
    assert_eq!(inbox.stays().await, 1, "滞在が増えている");
    let stay_mark: Option<String> =
        sqlx::query_scalar("SELECT deleted_by FROM core.event WHERE id = $1")
            .bind(stay)
            .fetch_one(&inbox.pool)
            .await
            .unwrap();
    assert_eq!(stay_mark.as_deref(), Some("user"));
}

// Scenario: 消した滞在の時間帯の外の書庫の位置は生きた記録として入る
#[tokio::test]
async fn archive_erased_window_leaves_locations_outside_alive() {
    let inbox = Inbox::new("archive-erased-outside").await;
    let stay = inbox.put_stay().await;
    crate::deletion::erase(&inbox.pool, stay, None)
        .await
        .unwrap();
    inbox.put(
        "Timeline.json.zip",
        &[("Timeline.json", OUTSIDE_TIMELINE.as_bytes())],
    );
    inbox.spawn(true);
    inbox.until_archive_locations(2).await;

    assert_eq!(inbox.archive_locations().await, (0, 2));
    assert_eq!(inbox.erase_ledger_rows(stay, "user:late").await, 0);
}

// Scenario: 滞在を消すとその時間帯の書庫の位置も削除済みになる
#[tokio::test]
async fn archive_erased_cascade_marks_locations_stored_before_the_erase() {
    let inbox = Inbox::new("archive-erased-cascade").await;
    inbox.put(
        "Timeline.json.zip",
        &[("Timeline.json", ERASE_TIMELINE.as_bytes())],
    );
    inbox.spawn(true);
    inbox.until_archive_locations(7).await;
    assert_eq!(
        inbox.archive_locations().await,
        (0, 7),
        "格納の時点で印が付いている"
    );

    let stay = inbox.put_stay().await;
    crate::deletion::erase(&inbox.pool, stay, None)
        .await
        .unwrap();
    let marks = inbox.marks().await;
    assert_eq!(
        marks
            .iter()
            .filter(|(_, by)| by.as_deref() == Some("user:cascade"))
            .count(),
        4,
        "{marks:?}"
    );
    assert_eq!(inbox.archive_locations().await, (4, 3));
    assert_eq!(inbox.erase_ledger_rows(stay, "user:cascade").await, 4);

    // 同じ内容の書庫を置き直しても行は増えない（内容の鍵。final review 第 2 回 R75）。書庫の
    // ハッシュが同じだと走査が既読として畳み中身を読まないので、読まない別のファイルを足して別の書庫にする。
    inbox.put(
        "Timeline-again.json.zip",
        &[
            ("Timeline.json", ERASE_TIMELINE.as_bytes()),
            ("readme.txt", b"not an export"),
        ],
    );
    let again = inbox
        .inbox
        .join("取り込み済み")
        .join("Timeline-again.json.zip");
    inbox
        .until("置き直した書庫が読まれない", || async {
            again.exists() && inbox.read_rows_now(None).await == 2
        })
        .await;
    assert_eq!(
        inbox.archive_locations().await,
        (4, 3),
        "置き直した書庫で位置の行が増えた / 印が外れた"
    );
    assert_eq!(
        inbox.erase_ledger_rows(stay, "user:cascade").await,
        4,
        "置き直しで連鎖の台帳の行が増えた"
    );

    // 滞在の判定の入力は基準のソースのままで、書庫の位置は足されていない。
    assert_eq!(
        crate::stay::Criteria::default_values().sources,
        vec!["c01-location".to_owned()]
    );
}

// Scenario: 滞在の削除を戻すと書庫の位置も戻る
#[tokio::test]
async fn archive_erased_cascade_restore_brings_the_locations_back() {
    let inbox = Inbox::new("archive-erased-restore").await;
    inbox.put(
        "Timeline.json.zip",
        &[("Timeline.json", ERASE_TIMELINE.as_bytes())],
    );
    inbox.spawn(true);
    inbox.until_archive_locations(7).await;
    let stay = inbox.put_stay().await;
    crate::deletion::erase(&inbox.pool, stay, None)
        .await
        .unwrap();
    assert_eq!(inbox.archive_locations().await, (4, 3));

    let outcome = crate::deletion::restore(&inbox.pool, &[stay], None)
        .await
        .unwrap();
    assert_eq!(outcome.locations, 4);
    assert_eq!(
        inbox.archive_locations().await,
        (0, 7),
        "戻した後に印が残っている"
    );
}

/// 移行前の `Records.json`。消した滞在（03:00Z〜04:00Z）の中の点と外の点を 1 つずつ持つ（code-verify 第 4 回 R79）。
const LEGACY_RECORDS: &str = r#"{"locations":[
  {"latitudeE7":356580000,"longitudeE7":1397450000,"accuracy":10,"timestamp":"2026-09-12T03:20:00Z"},
  {"latitudeE7":356700000,"longitudeE7":1397600000,"accuracy":10,"timestamp":"2026-09-12T08:20:00Z"}
 ]}"#;

/// spec が名指しする書庫の位置の論理ソース 7 本の**文字列**。`LOCATION_SOURCES` を通さずに数えるので、
/// 並びからソースが抜けると件数が合わなくなる（R79）。
const SPEC_LOCATION_SOURCES: [&str; 7] = [
    "c03-timeline-visit",
    "c03-timeline-move",
    "c03-timeline-route",
    "c03-timeline-signal",
    "c03-legacy-location",
    "c03-legacy-visit",
    "c03-legacy-activity",
];

impl Inbox {
    /// 移行前の `Records.json` を、取り込み器を介さずに格納する（退役の日付を動かさない）。
    async fn store_records(&self, body: &str) -> Vec<crate::IngestRequest> {
        let requests = crate::archive::worker::requests_for_file(
            crate::archive::classify::KnownKind::Records,
            "Takeout/Location History/Records.json",
            body.as_bytes(),
            self.user,
            format!("{:064x}", uuid::Uuid::new_v4().as_u128()),
        )
        .unwrap();
        let sink = crate::PgSink::new(self.pool.clone());
        crate::archive::worker::store_requests(&sink, requests.clone())
            .await
            .unwrap();
        requests
    }

    /// 名指しした論理ソースの行の (ソース, 印)。並びは論理ソース・時刻の順。
    async fn marks_of(&self, sources: &[&str]) -> Vec<(String, Option<String>)> {
        sqlx::query_as(
            "SELECT logical_source, deleted_by FROM core.event
              WHERE user_id = $1 AND logical_source = ANY($2) ORDER BY logical_source, event_time",
        )
        .bind(self.user)
        .bind(sources.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>())
        .fetch_all(&self.pool)
        .await
        .unwrap()
    }
}

fn mark(source: &str, by: Option<&str>) -> (String, Option<String>) {
    (source.to_owned(), by.map(str::to_owned))
}

// Scenario: 消した滞在の時間帯に書庫から入る位置は削除済みになる
// Scenario: 滞在の削除を戻すと書庫の位置も戻る
//
// 件数の大半を占める経路の点（`c03-timeline-route`）と移行前の点（`c03-legacy-location`）にも、消した後に
// 置いた書庫で後着の印（`user:late`）が付き、`deletion::restore` で戻る（code-verify 第 4 回 R79。
// それまでの試験は先に格納して後から消した `user:cascade` だけを戻していた）。
#[tokio::test]
async fn archive_erased_late_marks_on_route_and_records_points_are_restored() {
    let inbox = Inbox::new("archive-erased-late-restore").await;
    let stay = inbox.put_stay().await;
    crate::deletion::erase(&inbox.pool, stay, None)
        .await
        .unwrap();
    inbox.put(
        "Timeline.json.zip",
        &[("Timeline.json", ERASE_TIMELINE.as_bytes())],
    );
    inbox.spawn(true);
    inbox.until_archive_locations(7).await;
    inbox
        .until("書庫の位置に後着の印が付かない", || async {
            Inbox::late_marks(&inbox.marks().await) == 4
        })
        .await;
    let records = inbox.store_records(LEGACY_RECORDS).await;
    assert_eq!(records.len(), 2);
    crate::stay_store::mark_archive_arrivals(&inbox.pool, inbox.user, &records)
        .await
        .unwrap();

    assert_eq!(
        inbox
            .marks_of(&["c03-timeline-route", "c03-legacy-location"])
            .await,
        vec![
            mark("c03-legacy-location", Some("user:late")),
            mark("c03-legacy-location", None),
            mark("c03-timeline-route", Some("user:late")),
            mark("c03-timeline-route", None),
        ],
        "経路の点と移行前の点の中だけに後着の印が付いていない"
    );
    assert_eq!(inbox.erase_ledger_rows(stay, "user:late").await, 5);

    let outcome = crate::deletion::restore(&inbox.pool, &[stay], None)
        .await
        .unwrap();
    assert_eq!(outcome.locations, 5, "後着の印を付けた位置が戻らない");
    let after = inbox.marks_of(&SPEC_LOCATION_SOURCES).await;
    assert_eq!(after.len(), 9, "{after:?}");
    assert!(
        after.iter().all(|(_, by)| by.is_none()),
        "戻した後に印が残っている: {after:?}"
    );
}

// Scenario: 滞在を消すとその時間帯の書庫の位置も削除済みになる
// Scenario: 滞在の削除を戻すと書庫の位置も戻る
//
// 先に格納した移行前の点（`c03-legacy-location`）にも、滞在を消すときの連鎖の印が付き、戻すと外れる（R79）。
#[tokio::test]
async fn archive_erased_cascade_marks_and_restores_records_points() {
    let inbox = Inbox::new("archive-erased-records-cascade").await;
    inbox.store_records(LEGACY_RECORDS).await;
    let stay = inbox.put_stay().await;
    crate::deletion::erase(&inbox.pool, stay, None)
        .await
        .unwrap();
    assert_eq!(
        inbox.marks_of(&["c03-legacy-location"]).await,
        vec![
            mark("c03-legacy-location", Some("user:cascade")),
            mark("c03-legacy-location", None),
        ]
    );
    assert_eq!(inbox.erase_ledger_rows(stay, "user:cascade").await, 1);

    let outcome = crate::deletion::restore(&inbox.pool, &[stay], None)
        .await
        .unwrap();
    assert_eq!(outcome.locations, 1);
    assert_eq!(
        inbox.marks_of(&["c03-legacy-location"]).await,
        vec![
            mark("c03-legacy-location", None),
            mark("c03-legacy-location", None),
        ]
    );
}

/// 印付け（`mark_archive_arrivals`）か格納を落とす継ぎ目。この利用者の `user:late` の削除の台帳の行
/// （格納なら `core.event` の行）を拒む trigger を置き、拒んだ回数を sequence に数える
/// （sequence は transaction が巻き戻っても戻らない）。
/// **条件にこの利用者を入れる**ので、DB を共有する他の試験には掛からない。
struct LateMarkFault {
    pool: sqlx::PgPool,
    name: String,
    table: &'static str,
}

impl LateMarkFault {
    async fn install(inbox: &Inbox) -> Self {
        Self::install_on(inbox, "core.deletion_ledger", "AND NEW.mark = 'user:late'").await
    }

    /// 格納（`core.event` への書き込み）を落とす。`PgSink` は最初の失敗で止まるので、1 回の格納で 1 回だけ数える。
    async fn install_store(inbox: &Inbox) -> Self {
        Self::install_on(inbox, "core.event", "").await
    }

    async fn install_on(inbox: &Inbox, table: &'static str, condition: &str) -> Self {
        let name = format!("st12_fault_{}", inbox.user.simple());
        sqlx::raw_sql(&format!(
            "CREATE SEQUENCE core.{name}_seq;
             CREATE FUNCTION core.{name}_fn() RETURNS trigger AS $fn$
             BEGIN
               PERFORM nextval('core.{name}_seq');
               RAISE EXCEPTION '試験の差し込み: 書き込みを落とす';
             END;
             $fn$ LANGUAGE plpgsql;
             CREATE TRIGGER {name}_tg BEFORE INSERT ON {table}
               FOR EACH ROW WHEN (NEW.user_id = '{user}'::uuid {condition})
               EXECUTE FUNCTION core.{name}_fn();",
            user = inbox.user
        ))
        .execute(&inbox.pool)
        .await
        .unwrap();
        Self {
            pool: inbox.pool.clone(),
            name,
            table,
        }
    }

    /// 印付けを拒んだ回数。
    async fn fired(&self) -> i64 {
        sqlx::query_scalar(&format!(
            "SELECT CASE WHEN is_called THEN last_value ELSE 0 END FROM core.{}_seq",
            self.name
        ))
        .fetch_one(&self.pool)
        .await
        .unwrap()
    }

    async fn remove(self) {
        sqlx::raw_sql(&format!(
            "DROP TRIGGER {name}_tg ON {table};
             DROP FUNCTION core.{name}_fn();
             DROP SEQUENCE core.{name}_seq;",
            name = self.name,
            table = self.table
        ))
        .execute(&self.pool)
        .await
        .unwrap();
    }
}

impl Inbox {
    /// この書庫の、いまの解析器の版の `read` の台帳の行の数。
    async fn read_rows_now(&self, archive_sha256: Option<&str>) -> i64 {
        sqlx::query_scalar(
            "SELECT count(*) FROM core.archive_ledger
              WHERE user_id = $1 AND parser_version = $2 AND outcome = 'read'
                AND ($3::text IS NULL OR sha256 = $3)",
        )
        .bind(self.user)
        .bind(crate::archive::PARSER_VERSION)
        .bind(archive_sha256)
        .fetch_one(&self.pool)
        .await
        .unwrap()
    }

    /// 滞在の作り直しの跡: この利用者の基準の版の行の数と、`rebuild:` の印の付いた行の数。
    async fn rebuild_traces(&self) -> (i64, i64) {
        let criteria: i64 =
            sqlx::query_scalar("SELECT count(*) FROM core.stay_criteria WHERE user_id = $1")
                .bind(self.user)
                .fetch_one(&self.pool)
                .await
                .unwrap();
        let rebuilt: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM core.event WHERE user_id = $1 AND deleted_by LIKE $2",
        )
        .bind(self.user)
        .bind(format!("{}%", crate::stay_store::REBUILD_PREFIX))
        .fetch_one(&self.pool)
        .await
        .unwrap();
        (criteria, rebuilt)
    }

    fn late_marks(marks: &[(String, Option<String>)]) -> usize {
        marks
            .iter()
            .filter(|(_, by)| by.as_deref() == Some("user:late"))
            .count()
    }
}

// Scenario: 消した滞在の時間帯に書庫から入る位置は削除済みになる
//
// 印付けが一時的に落ちた書庫は `read` の台帳の行を持たずに置き場に残り、次の走査で読み直されて印が付く
// （final review 第 2 回 R72。`read` の行を先に書いていたときは、次の走査が既読として畳み、
// 消した時間帯の位置が生きた記録のまま残った）。
#[tokio::test]
async fn archive_erased_window_marks_after_a_failed_marking_on_the_next_scan() {
    let inbox = Inbox::new("archive-erased-mark-retry").await;
    let stay = inbox.put_stay().await;
    crate::deletion::erase(&inbox.pool, stay, None)
        .await
        .unwrap();
    let fault = LateMarkFault::install(&inbox).await;
    inbox.put(
        "Timeline.json.zip",
        &[("Timeline.json", ERASE_TIMELINE.as_bytes())],
    );
    inbox.spawn(true);
    inbox
        .until("印付けが一度も試されない", || async {
            fault.fired().await >= 1
        })
        .await;
    // 落ちた書庫は既読になっていない（次の走査で読み直される）。位置は格納済みで印は無い。
    assert_eq!(
        inbox.read_rows_now(None).await,
        0,
        "印付けに落ちた書庫に read の台帳の行がある（次の走査が読み直さない）"
    );
    assert!(
        inbox.inbox.join("Timeline.json.zip").exists(),
        "印付けに落ちた書庫が置き場に残っていない"
    );
    assert_eq!(inbox.archive_locations().await, (0, 7));
    fault.remove().await;

    inbox.until_archive_locations(7).await;
    inbox
        .until("次の走査で印が付かない", || async {
            Inbox::late_marks(&inbox.marks().await) == 4
        })
        .await;
    assert_eq!(inbox.archive_locations().await, (4, 3));
    assert_eq!(inbox.erase_ledger_rows(stay, "user:late").await, 4);
    assert_eq!(inbox.read_rows_now(None).await, 1);
}

// Scenario: 消した滞在の時間帯に書庫から入る位置は削除済みになる
//
// 解析器の版が上がって写しから読み直す経路（`reparse_older_versions` → `reread_archive`）も、
// 印付けに落ちればいまの版の `read` の行を残さず、次の周で読み直して印を付ける（R72）。
#[tokio::test]
async fn archive_erased_window_reparse_marks_after_a_failed_marking() {
    let inbox = Inbox::new("archive-erased-reparse-retry").await;
    let archive_sha = inbox.seed_older_version().await;
    let stay = inbox.put_stay().await;
    crate::deletion::erase(&inbox.pool, stay, None)
        .await
        .unwrap();
    let fault = LateMarkFault::install(&inbox).await;
    inbox.spawn(true);
    inbox
        .until(
            "読み直しの印付けが一度も試されない",
            || async { fault.fired().await >= 1 },
        )
        .await;
    assert_eq!(
        inbox.read_rows_now(Some(&archive_sha)).await,
        0,
        "印付けに落ちた読み直しに いまの版の read の行がある（次の周の対象から外れる）"
    );
    fault.remove().await;

    inbox
        .until("次の周の読み直しで印が付かない", || async {
            Inbox::late_marks(&inbox.marks().await) == 4
                && inbox.read_rows_now(Some(&archive_sha)).await == 1
        })
        .await;
    assert_eq!(inbox.archive_locations().await, (4, 3));
    assert_eq!(inbox.erase_ledger_rows(stay, "user:late").await, 4);
}

impl Inbox {
    /// 前の版で読んだことにした `Timeline.json` の写しを 1 冊置き、その書庫の `sha256` を返す
    /// （`reparse_older_versions` が写しから読み直す対象になる）。
    async fn seed_older_version(&self) -> String {
        let archive_sha = format!("{:064x}", uuid::Uuid::new_v4().as_u128());
        std::fs::create_dir_all(&self.copies).unwrap();
        let stored =
            crate::archive::worker::copy_known_file(&self.copies, ERASE_TIMELINE.as_bytes())
                .unwrap();
        crate::archive::worker::record_copy(
            &self.pool,
            self.user,
            stored.file_name().unwrap().to_str().unwrap().to_owned(),
            "Timeline.json",
            &stored,
            &archive_sha,
        )
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO core.archive_ledger (user_id, sha256, parser_version, outcome, file_name)
             VALUES ($1, $2, 'older', 'read', 'Timeline.json.zip')",
        )
        .bind(self.user)
        .bind(&archive_sha)
        .execute(&self.pool)
        .await
        .unwrap();
        archive_sha
    }

    /// 読み直しの失敗の数と、読み直しを再開するまでの分（待ちが無ければ `None`）。行が無ければ `None`。
    async fn reread_failure(&self) -> Option<(i32, Option<f64>)> {
        sqlx::query_as(
            "SELECT consecutive_failures,
                    (extract(epoch FROM retry_after - now()) / 60)::float8
               FROM core.archive_reread_failure WHERE user_id = $1",
        )
        .bind(self.user)
        .fetch_optional(&self.pool)
        .await
        .unwrap()
    }
}

// Scenario: 格納に続けて失敗した書庫は台帳と画面に出る
//
// 解析器の版の読み直し（`reparse_older_versions` → `reread_archive`）で**格納**が落ち続けたときも書庫ごとに数え、
// ちょうど 3 回目で `store_failed` を 1 行書き、1 時間はその書庫を読み直さない（code-verify 第 5 回 R90。design D22-a）。
// 取り込み器を起こさず関数を直に呼ぶので、回数と待ちが走査の間隔に依らず決まる。
#[tokio::test]
async fn archive_reparse_persistent_store_failure_is_ledgered_and_throttled() {
    let inbox = Inbox::new("archive-reparse-store-persistent").await;
    let archive_sha = inbox.seed_older_version().await;
    let fault = LateMarkFault::install_store(&inbox).await;
    for attempt in 1..=3 {
        assert_eq!(
            crate::archive::worker::reparse_older_versions(&inbox.pool, inbox.user)
                .await
                .unwrap(),
            0
        );
        assert_eq!(
            fault.fired().await,
            attempt,
            "1 回の読み直しで格納が 1 回でない"
        );
        let expected_failed = i64::from(attempt == 3);
        assert_eq!(
            inbox.ledger_rows("store_failed").await,
            expected_failed,
            "{attempt} 回目の格納の失敗の後の store_failed の行の数が違う"
        );
    }
    let (failures, retry_in) = inbox
        .reread_failure()
        .await
        .expect("失敗が数えられていない");
    assert_eq!(failures, 3);
    let retry_in = retry_in.expect("3 回落ちても読み直しの待ちが無い");
    assert!(
        (59.0..=61.0).contains(&retry_in),
        "読み直しの待ちが 1 時間でない: {retry_in} 分"
    );
    // 待ちの間は版の読み直しもその書庫を飛ばす。
    crate::archive::worker::reparse_older_versions(&inbox.pool, inbox.user)
        .await
        .unwrap();
    let fired_later = fault.fired().await;
    fault.remove().await;
    assert_eq!(fired_later, 3, "待ちの間に版の読み直しが写しを読んだ");
    assert_eq!(inbox.ledger_rows("store_failed").await, 1);
    assert_eq!(inbox.read_rows_now(Some(&archive_sha)).await, 0);
}

// Scenario: 格納に続けて失敗した書庫は台帳と画面に出る
//
// 読み直しに成功したら数を消す（code-verify 第 5 回 R90。design D22-a）。消さなければ、一時的に 2 回落ちた書庫は
// 成功した後でも、次の 1 回の失敗で `store_failed` と 1 時間の待ちになる。
#[tokio::test]
async fn archive_reparse_success_clears_reread_failures() {
    let inbox = Inbox::new("archive-reparse-store-recovers").await;
    let archive_sha = inbox.seed_older_version().await;
    let fault = LateMarkFault::install_store(&inbox).await;
    for _ in 0..2 {
        crate::archive::worker::reparse_older_versions(&inbox.pool, inbox.user)
            .await
            .unwrap();
    }
    assert_eq!(fault.fired().await, 2);
    assert_eq!(inbox.reread_failure().await, Some((2, None)));
    fault.remove().await;

    assert_eq!(
        crate::archive::worker::reparse_older_versions(&inbox.pool, inbox.user)
            .await
            .unwrap(),
        1
    );
    assert_eq!(inbox.read_rows_now(Some(&archive_sha)).await, 1);
    assert_eq!(
        inbox.reread_failure().await,
        None,
        "読み直しに成功しても失敗の数が残っている"
    );
    assert_eq!(inbox.ledger_rows("store_failed").await, 0);
}

// Scenario: 格納に続けて失敗した書庫は台帳と画面に出る
//
// 印付けが落ち続ける書庫も、格納の失敗と同じく数える（code-verify 第 4 回 R80。design D22）。数えなかったときは、
// 走査のたびに書庫を丸ごと読み直し続け、台帳は 0 行・`latest_archive` は null（箱は「置かれていない」）だった。
// 3 回続いたら `store_failed` を 1 行書き、以後 1 時間はその書庫を読みへ回さない。
#[tokio::test]
async fn archive_erased_window_persistent_marking_failure_is_ledgered_and_throttled() {
    let inbox = Inbox::new("archive-erased-mark-persistent").await;
    let stay = inbox.put_stay().await;
    crate::deletion::erase(&inbox.pool, stay, None)
        .await
        .unwrap();
    let fault = LateMarkFault::install(&inbox).await;
    inbox.put(
        "Timeline.json.zip",
        &[("Timeline.json", ERASE_TIMELINE.as_bytes())],
    );
    inbox.spawn(true);
    inbox
        .until(
            "印付けが 3 回落ちても store_failed の行が無い",
            || async { inbox.ledger_rows("store_failed").await == 1 },
        )
        .await;
    let fired = fault.fired().await;
    // 1 時間に 1 回へ落ちている: 走査（1 秒ごと）を 4 回以上待っても読み直さない。
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    let fired_later = fault.fired().await;
    let status = inbox.status().await;
    let ledger_failed = inbox.ledger_rows("store_failed").await;
    let read_rows = inbox.read_rows_now(None).await;
    let still_in_inbox = inbox.inbox.join("Timeline.json.zip").exists();
    fault.remove().await;

    assert_eq!(fired, 3, "store_failed の行が 3 回目の失敗で書かれていない");
    assert_eq!(
        fired_later, fired,
        "store_failed の後も走査のたびに読み直している"
    );
    assert_eq!(ledger_failed, 1, "store_failed の行が 1 行でない");
    assert_eq!(read_rows, 0, "印の付かない書庫に read の行がある");
    assert!(still_in_inbox, "印の付かない書庫が置き場から動いた");
    let latest = status.latest_archive.expect("直近の書庫が箱に出ない");
    assert_eq!(latest.outcome, "store_failed");
    assert_eq!(latest.file_name.as_deref(), Some("Timeline.json.zip"));
}

/// 移行前の `Semantic Location History`。消した滞在（03:00Z〜04:00Z）に対して、**始まりは前で終わりだけが
/// 触れる・入る**訪問と移動（ミリ秒の `endTimestampMs`）と、外の訪問を持つ。終わりをミリ秒として読まないと
/// 終わりが始まりに倒れ、重なる 2 件にも印が付かない（final review 第 2 回 R76）。
const LEGACY_SEMANTIC_BEFORE: &str = r#"{"timelineObjects":[
  {"placeVisit":{"location":{"placeId":"before-touch"},"duration":{"startTimestampMs":"1789178400000","endTimestampMs":"1789182000000"}}},
  {"activitySegment":{"activityType":"WALKING","duration":{"startTimestampMs":"1789180200000","endTimestampMs":"1789183800000"}}},
  {"placeVisit":{"location":{"placeId":"outside"},"duration":{"startTimestampMs":"1789192800000","endTimestampMs":"1789196400000"}}}
 ]}"#;

/// 消した後に届く移行前の訪問。始まりは消した時間帯の前、終わりは中（ミリ秒）。
const LEGACY_SEMANTIC_AFTER: &str = r#"{"timelineObjects":[
  {"placeVisit":{"location":{"placeId":"late-arrival"},"duration":{"startTimestampMs":"1789180200000","endTimestampMs":"1789185540000"}}}
 ]}"#;

impl Inbox {
    /// 移行前の `Semantic Location History` を、取り込み器を介さずに格納する（退役の日付を動かさない）。
    async fn store_semantic(&self, body: &str) -> Vec<crate::IngestRequest> {
        let requests = crate::archive::worker::requests_for_file(
            crate::archive::classify::KnownKind::SemanticHistory,
            "Semantic Location History/2026/2026_SEPTEMBER.json",
            body.as_bytes(),
            self.user,
            format!("{:064x}", uuid::Uuid::new_v4().as_u128()),
        )
        .unwrap();
        let sink = crate::PgSink::new(self.pool.clone());
        crate::archive::worker::store_requests(&sink, requests.clone())
            .await
            .unwrap();
        requests
    }

    async fn legacy_marks(&self) -> Vec<(String, Option<String>)> {
        sqlx::query_as(
            "SELECT logical_source, deleted_by FROM core.event
              WHERE user_id = $1 AND logical_source IN ('c03-legacy-visit', 'c03-legacy-activity')
              ORDER BY event_time, logical_source",
        )
        .bind(self.user)
        .fetch_all(&self.pool)
        .await
        .unwrap()
    }
}

// Scenario: 滞在を消すとその時間帯の書庫の位置も削除済みになる
// Scenario: 消した滞在の時間帯に書庫から入る位置は削除済みになる
//
// 移行前の区間（`c03-legacy-visit` / `-activity`）の終わりはミリ秒（`endTimestampMs`）。消すときの連鎖と
// 格納の直後の印付けの両方が、ミリ秒の終わりで重なりを見る（final review 第 2 回 R76）。
#[tokio::test]
async fn archive_erased_legacy_millisecond_spans_follow_the_erased_stay() {
    let inbox = Inbox::new("archive-erased-legacy").await;
    let before = inbox.store_semantic(LEGACY_SEMANTIC_BEFORE).await;
    assert_eq!(before.len(), 3);
    assert!(
        before.iter().all(|r| r.payload["end_time"]
            .as_str()
            .is_some_and(|t| t.len() == 13)),
        "終わりがミリ秒の文字列で格納されていない: {:?}",
        before
            .iter()
            .map(|r| &r.payload["end_time"])
            .collect::<Vec<_>>()
    );

    // (2) 消すとき: 終わりが触れる訪問と、中で終わる移動に連鎖の印。外の訪問は生きる。
    let stay = inbox.put_stay().await;
    crate::deletion::erase(&inbox.pool, stay, None)
        .await
        .unwrap();
    assert_eq!(
        inbox.legacy_marks().await,
        vec![
            (
                "c03-legacy-visit".to_owned(),
                Some("user:cascade".to_owned())
            ),
            (
                "c03-legacy-activity".to_owned(),
                Some("user:cascade".to_owned())
            ),
            ("c03-legacy-visit".to_owned(), None),
        ]
    );
    assert_eq!(inbox.erase_ledger_rows(stay, "user:cascade").await, 2);

    // (1) 格納の直後: 消した後に届いた、始まりが前で終わりが中の訪問に後着の印。
    let after = inbox.store_semantic(LEGACY_SEMANTIC_AFTER).await;
    crate::stay_store::mark_archive_arrivals(&inbox.pool, inbox.user, &after)
        .await
        .unwrap();
    let late: Vec<_> = inbox
        .legacy_marks()
        .await
        .into_iter()
        .filter(|(_, by)| by.as_deref() == Some("user:late"))
        .collect();
    assert_eq!(
        late,
        vec![("c03-legacy-visit".to_owned(), Some("user:late".to_owned()))]
    );
    assert_eq!(inbox.erase_ledger_rows(stay, "user:late").await, 1);
}

/// 位置を 1 件も入れなかった書庫（YouTube だけ）では印付けを飛ばす（final review 第 2 回 R78）。
/// 飛ばさなければ利用者の錠を取りに行くので、錠を握ったままでも待たずに返ることで見る。
#[tokio::test]
async fn archive_marking_skips_an_archive_without_locations() {
    let inbox = Inbox::new("archive-mark-skip").await;
    let requests = crate::archive::worker::requests_for_file(
        crate::archive::classify::KnownKind::YouTubeWatch,
        "Takeout/YouTube/watch-history.json",
        watch("2026-09-12T03:00:00Z", "ある動画").as_bytes(),
        inbox.user,
        "e".repeat(64),
    )
    .unwrap();
    assert!(!requests.is_empty());
    let mut held = inbox.pool.begin().await.unwrap();
    sqlx::query("SELECT pg_advisory_xact_lock($1, hashtext($2::text))")
        .bind(crate::stay_store::LOCK_KEY)
        .bind(inbox.user)
        .execute(&mut *held)
        .await
        .unwrap();
    let other = testdb::pool().await;
    let skipped = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        crate::stay_store::mark_archive_arrivals(&other, inbox.user, &requests),
    )
    .await;
    assert!(
        matches!(skipped, Ok(Ok(()))),
        "位置の無い書庫で印付けが利用者の錠を待った"
    );
    // 位置を入れた書庫なら錠を待つ（上の観測が、飛ばしたことを見ている裏付け）。
    let located = inbox.store_semantic(LEGACY_SEMANTIC_AFTER).await;
    let waited = tokio::time::timeout(
        std::time::Duration::from_millis(500),
        crate::stay_store::mark_archive_arrivals(&other, inbox.user, &located),
    )
    .await;
    assert!(waited.is_err(), "位置を入れた書庫の印付けが錠を待たない");
    held.rollback().await.unwrap();
}

/// 位置（`locationInfos`）を持つ項目と持たない項目を 1 件ずつ含む合成のマイアクティビティ。
/// どちらも消した滞在（03:00Z〜04:00Z）の中の時刻（第 5 回 Q14）。
const LOCATED_MYACTIVITY: &str = r#"[
  {"header":"検索","title":"位置あり","titleUrl":"https://www.google.com/search?q=a","time":"2026-09-12T03:30:00Z","products":["検索"],
   "locationInfos":[{"name":"この付近","url":"https://www.google.com/maps/@?api=1&map_action=map&center=35.658,139.745&zoom=12","source":"もとの場所"}]},
  {"header":"検索","title":"位置なし","titleUrl":"https://www.google.com/search?q=b","time":"2026-09-12T03:40:00Z","products":["検索"]}
 ]"#;

impl Inbox {
    /// 合成のマイアクティビティ（`LOCATED_MYACTIVITY`）の 2 行の (題名, 印)。題名の順。
    async fn myactivity_marks(&self) -> Vec<(String, Option<String>)> {
        sqlx::query_as(
            "SELECT payload->>'title', deleted_by FROM core.event
              WHERE user_id = $1 AND logical_source LIKE 'c03-myactivity-%'
              ORDER BY payload->>'title'",
        )
        .bind(self.user)
        .fetch_all(&self.pool)
        .await
        .unwrap()
    }

    async fn myactivity_ledger_rows(&self, cause: uuid::Uuid, mark: &str) -> i64 {
        sqlx::query_scalar(
            "SELECT count(*) FROM core.deletion_ledger
              WHERE user_id = $1 AND cause_event_id = $2 AND action = 'erase' AND mark = $3
                AND logical_source LIKE 'c03-myactivity-%'",
        )
        .bind(self.user)
        .bind(cause)
        .bind(mark)
        .fetch_one(&self.pool)
        .await
        .unwrap()
    }

    /// `LOCATED_MYACTIVITY` を取り込み器を介さずに格納する（形の確認の印は不要）。
    async fn store_myactivity(&self) -> Vec<crate::IngestRequest> {
        let requests = crate::archive::worker::requests_for_file(
            crate::archive::classify::KnownKind::MyActivity,
            "Takeout/My Activity/検索/活動.json",
            LOCATED_MYACTIVITY.as_bytes(),
            self.user,
            format!("{:064x}", uuid::Uuid::new_v4().as_u128()),
        )
        .unwrap();
        for request in &requests {
            crate::archive::worker::ensure_myactivity_source(
                &self.pool,
                &request.logical_source,
                "検索",
            )
            .await
            .unwrap();
        }
        let sink = crate::PgSink::new(self.pool.clone());
        crate::archive::worker::store_requests(&sink, requests.clone())
            .await
            .unwrap();
        requests
    }
}

// Scenario: 消した滞在の時間帯の位置を持つマイアクティビティの項目は削除済みになる
// Scenario: 位置を持たないマイアクティビティの項目は消した時間帯でも生きた記録として入る
#[tokio::test]
async fn archive_erased_myactivity_window() {
    let inbox = Inbox::new("archive-erased-myactivity-window").await;
    let stay = inbox.put_stay().await;
    crate::deletion::erase(&inbox.pool, stay, None)
        .await
        .unwrap();
    // 形の確認の印を置いて、書庫を取り込み器へ置く
    inbox
        .confirm(
            crate::archive::classify::KnownKind::MyActivity,
            LOCATED_MYACTIVITY.as_bytes(),
        )
        .await;
    inbox.put(
        "takeout-20260912T000000Z-001.zip",
        &[(
            "Takeout/My Activity/検索/活動.json",
            LOCATED_MYACTIVITY.as_bytes(),
        )],
    );
    inbox.spawn(true);
    inbox
        .until(
            "位置を持つ項目に後着の印が付かない",
            || async {
                inbox.myactivity_marks().await
                    == vec![
                        ("位置あり".to_owned(), Some("user:late".to_owned())),
                        ("位置なし".to_owned(), None),
                    ]
            },
        )
        .await;
    assert_eq!(inbox.myactivity_ledger_rows(stay, "user:late").await, 1);
    let raw: String = sqlx::query_scalar(
        "SELECT raw FROM core.event
          WHERE user_id = $1 AND logical_source LIKE 'c03-myactivity-%' AND payload->>'title' = '位置あり'",
    )
    .bind(inbox.user)
    .fetch_one(&inbox.pool)
    .await
    .unwrap();
    assert!(raw.contains("locationInfos"), "原文から欄を外している");
}

// Scenario: 滞在を消すとその時間帯の位置を持つマイアクティビティの項目も削除済みになる
// Scenario: 滞在の削除を戻すと位置を持つマイアクティビティの項目も戻る
#[tokio::test]
async fn archive_erased_myactivity_cascade() {
    let inbox = Inbox::new("archive-erased-myactivity-cascade").await;
    inbox.store_myactivity().await;
    assert_eq!(
        inbox.myactivity_marks().await,
        vec![("位置あり".to_owned(), None), ("位置なし".to_owned(), None)]
    );
    let stay = inbox.put_stay().await;
    crate::deletion::erase(&inbox.pool, stay, None)
        .await
        .unwrap();
    assert_eq!(
        inbox.myactivity_marks().await,
        vec![
            ("位置あり".to_owned(), Some("user:cascade".to_owned())),
            ("位置なし".to_owned(), None)
        ]
    );
    assert_eq!(inbox.myactivity_ledger_rows(stay, "user:cascade").await, 1);

    let outcome = crate::deletion::restore(&inbox.pool, &[stay], None)
        .await
        .unwrap();
    assert_eq!(outcome.locations, 1);
    assert_eq!(
        inbox.myactivity_marks().await,
        vec![("位置あり".to_owned(), None), ("位置なし".to_owned(), None)]
    );
}

// Scenario: 消した滞在の時間帯の位置を持つマイアクティビティの項目は削除済みになる
//
// 最初の Takeout のマイアクティビティは、形の印が無いので必ず「置く → 確認待ち → 印 → 写しから読み直す」
// （`ingest_confirmed_pending` → `reread_archive`）の順で入る（D16）。この順でも消した時間帯の位置を持つ項目に
// 印が付く（final review 第 3 回 R84）。
#[tokio::test]
async fn archive_erased_myactivity_window_after_confirming_a_pending_archive() {
    let inbox = Inbox::new("archive-erased-myactivity-pending").await;
    let stay = inbox.put_stay().await;
    crate::deletion::erase(&inbox.pool, stay, None)
        .await
        .unwrap();
    inbox.put(
        "takeout-20260912T000000Z-001.zip",
        &[(
            "Takeout/My Activity/検索/活動.json",
            LOCATED_MYACTIVITY.as_bytes(),
        )],
    );
    inbox.spawn(true);
    inbox
        .until(
            "印の無い形の書庫が確認待ちにならない",
            || async { inbox.ledger_rows("pending_shape").await == 1 },
        )
        .await;
    assert!(
        inbox.myactivity_marks().await.is_empty(),
        "確認待ちの書庫の中身が格納された"
    );
    inbox
        .confirm(
            crate::archive::classify::KnownKind::MyActivity,
            LOCATED_MYACTIVITY.as_bytes(),
        )
        .await;
    inbox
        .until(
            "確認待ちを経た書庫の位置を持つ項目に後着の印が付かない",
            || async {
                inbox.myactivity_marks().await
                    == vec![
                        ("位置あり".to_owned(), Some("user:late".to_owned())),
                        ("位置なし".to_owned(), None),
                    ]
            },
        )
        .await;
    assert_eq!(inbox.myactivity_ledger_rows(stay, "user:late").await, 1);
    inbox
        .until(
            "確認待ちを経た書庫に read の行が無い",
            || async { inbox.read_rows_now(None).await == 1 },
        )
        .await;
}

// Scenario: 格納に続けて失敗した書庫は台帳と画面に出る
//
// 確認待ちを写しから読み直す経路（`ingest_confirmed_pending` → `reread_archive`）でも、印付けが落ち続ければ
// 書庫ごとに数え、3 回続いたら `store_failed` を 1 行書き、以後 1 時間に 1 回へ落とす（final review 第 3 回 R82。
// design D22-a）。数えなかったときは、走査のたびに写しを全件読み直し、台帳にも画面にも何も出なかった。
#[tokio::test]
async fn archive_erased_pending_reread_persistent_marking_failure_is_ledgered_and_throttled() {
    let inbox = Inbox::new("archive-erased-pending-mark-persistent").await;
    let stay = inbox.put_stay().await;
    crate::deletion::erase(&inbox.pool, stay, None)
        .await
        .unwrap();
    let fault = LateMarkFault::install(&inbox).await;
    inbox.put(
        "takeout-20260912T000000Z-001.zip",
        &[(
            "Takeout/My Activity/検索/活動.json",
            LOCATED_MYACTIVITY.as_bytes(),
        )],
    );
    inbox.spawn(true);
    inbox
        .until(
            "印の無い形の書庫が確認待ちにならない",
            || async { inbox.ledger_rows("pending_shape").await == 1 },
        )
        .await;
    inbox
        .confirm(
            crate::archive::classify::KnownKind::MyActivity,
            LOCATED_MYACTIVITY.as_bytes(),
        )
        .await;
    inbox
        .until(
            "読み直しの印付けが 3 回落ちても store_failed の行が無い",
            || async { inbox.ledger_rows("store_failed").await == 1 },
        )
        .await;
    let fired = fault.fired().await;
    // 1 時間に 1 回へ落ちている: 走査（1 秒ごと）を 4 回以上待っても読み直さない。
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    let fired_later = fault.fired().await;
    let status = inbox.status().await;
    let ledger_failed = inbox.ledger_rows("store_failed").await;
    let read_rows = inbox.read_rows_now(None).await;
    let marks = inbox.myactivity_marks().await;
    let reread_failure = inbox.reread_failure().await;
    fault.remove().await;

    assert_eq!(fired, 3, "store_failed の行が 3 回目の失敗で書かれていない");
    assert_eq!(
        fired_later, fired,
        "store_failed の後も走査のたびに写しを読み直している"
    );
    assert_eq!(ledger_failed, 1, "store_failed の行が 1 行でない");
    assert_eq!(read_rows, 0, "印の付かない読み直しに read の行がある");
    let (failures, retry_in) = reread_failure.expect("印付けの失敗が数えられていない");
    assert_eq!(failures, 3);
    assert!(
        retry_in.is_some_and(|minutes| (59.0..=61.0).contains(&minutes)),
        "読み直しの待ちが 1 時間でない: {retry_in:?} 分"
    );
    assert_eq!(
        marks,
        vec![("位置あり".to_owned(), None), ("位置なし".to_owned(), None)],
        "落ちた印付けの後の行が想定と違う"
    );
    let latest = status.latest_archive.expect("直近の書庫が箱に出ない");
    assert_eq!(latest.outcome, "store_failed");
    assert_eq!(
        latest.file_name.as_deref(),
        Some("takeout-20260912T000000Z-001.zip")
    );
}

// Scenario: 滞在の削除を戻すと位置を持つマイアクティビティの項目も戻る
//
// 重なる 2 つの滞在を消すと、位置を持つ項目は 2 つ目の消去の原因も台帳に持つ（`already_deleted` + `located_e`）。
// 片方だけを戻しても、もう一方の消去がまだ効いているので項目は隠れたまま（final review 第 3 回 R87）。
#[tokio::test]
async fn archive_erased_myactivity_overlapping_erasures_restore_one_keeps_it_hidden() {
    let inbox = Inbox::new("archive-erased-myactivity-overlap").await;
    inbox.store_myactivity().await;
    let first = inbox.put_stay().await;
    let second = inbox
        .put_stay_at("2026-09-12T03:15:00+00:00", "2026-09-12T04:15:00+00:00")
        .await;
    crate::deletion::erase(&inbox.pool, first, None)
        .await
        .unwrap();
    crate::deletion::erase(&inbox.pool, second, None)
        .await
        .unwrap();
    let hidden = vec![
        ("位置あり".to_owned(), Some("user:cascade".to_owned())),
        ("位置なし".to_owned(), None),
    ];
    assert_eq!(inbox.myactivity_marks().await, hidden);
    // 2 つ目の消去も、既に隠れていた項目に自分の原因を追記している。
    assert_eq!(inbox.myactivity_ledger_rows(first, "user:cascade").await, 1);
    assert_eq!(
        inbox.myactivity_ledger_rows(second, "user:cascade").await,
        1,
        "重なる 2 つ目の消去が、位置を持つ項目に原因を追記していない"
    );

    crate::deletion::restore(&inbox.pool, &[first], None)
        .await
        .unwrap();
    assert_eq!(
        inbox.myactivity_marks().await,
        hidden,
        "片方の消去を戻しただけで、もう一方の時間帯の位置を持つ項目が戻った"
    );

    crate::deletion::restore(&inbox.pool, &[second], None)
        .await
        .unwrap();
    assert_eq!(
        inbox.myactivity_marks().await,
        vec![("位置あり".to_owned(), None), ("位置なし".to_owned(), None)],
        "両方の消去を戻しても、位置を持つ項目が戻らない"
    );
}

// Scenario: 位置を持たないマイアクティビティの項目は消した時間帯でも生きた記録として入る
//
// 印付けの範囲（Rust の `carries_location`）と、印を付ける条件（SQL の `myactivity_located_sql`）は
// 同じ判定でなければならない。片方だけを直すと、範囲と印付けが黙ってずれる（final review 第 3 回 R86）。
#[tokio::test]
async fn archive_myactivity_location_rust_and_sql_agree() {
    let pool = testdb::pool().await;
    let cases: &[(&str, bool)] = &[
        (
            r#"{"title":"a","locationInfos":[{"name":"この付近"}]}"#,
            true,
        ),
        (r#"{"title":"a", "locationInfos" : [ 1 ] }"#, true),
        (r#"{"title":"a","locationInfos":[]}"#, false),
        (r#"{"title":"a","locationInfos":null}"#, false),
        // 中身の形は見ない（C: 厳しい側。code-verify 第 5 回 R91）。配列でなくても空でなければ位置を持つ。
        (r#"{"title":"a","locationInfos":{"name":"この付近"}}"#, true),
        (r#"{"title":"a","locationInfos":"この付近"}"#, true),
        (r#"{"title":"a","locationInfos":1}"#, true),
        (r#"{"title":"a","locationInfos":{}}"#, false),
        (r#"{"title":"a","locationInfos":""}"#, false),
        (r#"[{"locationInfos":[{"name":"この付近"}]}]"#, false),
        (r#"{"title":"a"}"#, false),
        (r#"{"title":"\"locationInfos\""}"#, false),
        // 欄の名前をエスケープした原文は、両方とも前置き（R85）で落とす（Takeout はそう書かない）。
        (r#"{"location\u0049nfos":[{"name":"x"}]}"#, false),
        (r#"{"title":"a","locationInfos":[{"name":"x"}]"#, false),
        ("", false),
    ];
    let sql = format!(
        "SELECT {} FROM (SELECT $1::text AS raw, $2::text AS logical_source) t",
        crate::stay_store::myactivity_located_sql("t")
    );
    for (raw, expected) in cases {
        let rust = crate::stay_store::carries_location(raw);
        let in_db: bool = sqlx::query_scalar(&sql)
            .bind(raw)
            .bind("c03-myactivity-検索")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(rust, *expected, "Rust の判定が違う: {raw}");
        assert_eq!(in_db, *expected, "SQL の判定が違う: {raw}");
        // マイアクティビティでない行は常に通す（SQL の側だけの約束）。
        let other: bool = sqlx::query_scalar(&sql)
            .bind(raw)
            .bind("c03-timeline-visit")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert!(other, "マイアクティビティでない行を落とした: {raw}");
    }
}
