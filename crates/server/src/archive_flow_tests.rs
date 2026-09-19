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
        crate::archive::worker::spawn_inspecting(
            self.pool.clone(),
            self.config(keep_copies),
            self.user,
            crate::archive::worker::ReadingState::default(),
        );
    }

    /// その中身の形に印を置く（`tools/archive-shape.sh --confirm` と同じことを直に行う）。
    async fn confirm(&self, kind: crate::archive::classify::KnownKind, bytes: &[u8]) {
        let shape = crate::archive::worker::shape_for_file(kind, bytes).unwrap();
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
        .confirm(crate::archive::classify::KnownKind::YouTubeWatch, body.as_bytes())
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
    // 端末の位置に、書庫と同じ時刻・同じ座標の記録を先に置く。
    testdb::put_event(
        &inbox.pool,
        inbox.user,
        "c01-location",
        "2021-06-01T12:00:00+09:00",
    )
    .await;
    let before: Option<chrono::NaiveDate> = sqlx::query_scalar(
        "SELECT collection_started_on FROM core.source WHERE logical_source = 'c01-location'",
    )
    .fetch_one(&inbox.pool)
    .await
    .unwrap();
    let phone_events = inbox.events("c01-location").await;

    let records = br#"{"locations":[{"timestampMs":"1622505600000","latitudeE7":356580000,"longitudeE7":1397450000}]}"#;
    inbox.put("Records.json.zip", &[("Takeout/Records.json", records)]);
    inbox.spawn(true);

    inbox
        .until("移行前のロケーション履歴が書庫のソースへ入らない", || async {
            inbox.events("c03-legacy-location").await >= 1
        })
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
    assert_eq!(after, before, "書庫の位置が携帯端末の位置の収集開始日を動かした");
}

/// Scenario: 書庫の論理ソースは成功条件 1 の達成に数えられない
#[tokio::test]
async fn archive_flow_archive_sources_are_not_counted_in_achievement() {
    let pool = testdb::pool().await;
    let user = testdb::user();
    testdb::put_event(&pool, user, "c03-youtube-watch", "2026-09-12T12:00:00+09:00").await;

    let got = crate::coverage::achievement(
        &pool,
        Some(user),
        testdb::date("2026-09-13"),
        &crate::coverage::must_sources(),
    )
    .await
    .unwrap();

    let named: Vec<&str> = got.sources.iter().map(|s| s.named_source.as_str()).collect();
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
        .confirm(crate::archive::classify::KnownKind::YouTubeWatch, body.as_bytes())
        .await;
    inbox.put(
        "takeout-20260912-000000.zip",
        &[
            ("Takeout/YouTube/watch-history.json", body.as_bytes()),
            ("Takeout/Google フォト/IMG_0001.jpg", b"\xff\xd8\xff\xe0jpeg"),
            ("Takeout/Google フォト/IMG_0002.jpg", b"\xff\xd8\xff\xe0jpeg"),
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
    assert_eq!(skipped, 2, "写真 2 枚が読まなかったファイルとして数えられていない");
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
        .until("印に無い製品のファイルが確認待ちにならない", || async {
            let pending: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM core.archive_pending_shape WHERE user_id = $1",
            )
            .bind(inbox.user)
            .fetch_one(&inbox.pool)
            .await
            .unwrap();
            pending == 1
        })
        .await;
    assert_eq!(
        inbox.events("c03-youtube-watch").await,
        1,
        "知らない製品があると同じ書庫の視聴履歴まで止まっている"
    );
    let waiting_path: String = sqlx::query_scalar(
        "SELECT inner_path FROM core.archive_pending_shape WHERE user_id = $1",
    )
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
    assert_eq!(copies, 1, "写しを残さない設定で確認待ちの写しが作られていない");
}

/// Scenario: 読まなかった製品のファイルは写されない
#[tokio::test]
async fn archive_flow_unread_products_are_never_copied() {
    let inbox = Inbox::new("archive-copy-scope").await;
    let body = watch("2026-09-12T03:00:00Z", "ある動画");
    inbox
        .confirm(crate::archive::classify::KnownKind::YouTubeWatch, body.as_bytes())
        .await;
    inbox.put(
        "takeout-20260912-000000.zip",
        &[
            ("Takeout/YouTube/watch-history.json", body.as_bytes()),
            ("Takeout/Google フォト/IMG_0001.jpg", b"\xff\xd8\xff\xe0jpeg"),
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
    assert_eq!(paths.len(), 1, "写しの目録が読んだ製品のファイルだけになっていない");
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
        .confirm(crate::archive::classify::KnownKind::YouTubeWatch, first.as_bytes())
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
            .confirm(crate::archive::classify::KnownKind::YouTubeWatch, body.as_bytes())
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
        .confirm(crate::archive::classify::KnownKind::YouTubeWatch, body.as_bytes())
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

    assert_eq!(inbox.events("c03-youtube-watch").await, 0, "記録が消えていない");
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
    let mixed = format!("[{},{}]", body.trim_matches(['[', ']']), other.trim_matches(['[', ']']));
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
        .confirm(crate::archive::classify::KnownKind::YouTubeWatch, body.as_bytes())
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
