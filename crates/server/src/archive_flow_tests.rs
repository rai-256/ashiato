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
    let config = crate::archive::config::from_values(&std::collections::BTreeMap::new()).unwrap();
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
    let timeline = br#"{"semanticSegments":[{"visit":{"startTime":"2026-09-12T03:00:00Z","topCandidate":{"placeLocation":{"latLng":"40.7128, -74.0060"}}}}]}"#;
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
    let timeline = r#"{"semanticSegments":[{"visit":{"startTime":"2026-09-12T03:00:00Z","topCandidate":{"placeLocation":{"latLng":"35.0116, 135.7681"}}}}]}"#;
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
    std::fs::write(
        inbox.inbox.join("Timeline.json"),
        br#"{"semanticSegments":[{"visit":{"startTime":"2026-09-12T03:00:00Z","topCandidate":{"placeLocation":{"latLng":"35.0116, 135.7681"}}}}]}"#,
    )
    .unwrap();
    inbox.spawn(true);

    inbox
        .until("裸の Timeline.json が読まれない", || async {
            inbox.events("c03-timeline-visit").await == 1
        })
        .await;
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
    assert_eq!(
        inbox.ledger_rows("read").await,
        1,
        "印を置いた後の読み直しが台帳に 1 行足していない"
    );
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
    let timeline = br#"{"semanticSegments":[{"visit":{"startTime":"2026-09-12T03:00:00Z","topCandidate":{"placeLocation":{"latLng":"35.0116, 135.7681"}}}}]}"#;
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
