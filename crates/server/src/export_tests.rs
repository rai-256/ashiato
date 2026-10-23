// SPDX-License-Identifier: AGPL-3.0-only
//! 書き出しの骨組み（置き場・分類・列の突き合わせ・権限。ST33 / design D3 / D8 / D9）。
#![allow(clippy::unwrap_used)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::export::{
    self, ExportConfig, ExportError, FailKind, Hooks, Inventory, Passphrase, Stage, Unclassified,
};
use crate::testdb;

/// 書き出しの台帳・状態・置き場は全利用者で 1 つなので、走らせるテストは直列にする（spec-review R16）。
pub(crate) static EXPORT_SERIAL: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// 試験の一時フォルダ。返した `TempDir` を試験の最後まで持つ（手放すと、その時点で中身ごと消える）。
/// panic で試験が落ちても、unwind で drop されて消える。
pub(crate) fn tmp_dir(name: &str) -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix(&format!("ashiato-export-{name}-"))
        .tempdir()
        .unwrap()
}

// Scenario: Rust の試験が panic しても、試験の一時フォルダは消える
#[test]
fn ch03_tmp_dir_is_removed_even_if_the_test_panics() {
    let seen: std::sync::Mutex<Option<PathBuf>> = std::sync::Mutex::new(None);
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let dir = tmp_dir("ch03");
        std::fs::write(dir.path().join("left.txt"), b"x").unwrap();
        *seen.lock().unwrap() = Some(dir.path().to_path_buf());
        panic!("ch03: 試験の途中で落ちる");
    }));
    assert!(r.is_err(), "panic が外へ出る");
    let path = seen
        .lock()
        .unwrap()
        .clone()
        .expect("path を写してから panic する");
    assert!(!path.exists(), "panic しても一時フォルダは消える: {path:?}");
}

pub(crate) fn pass() -> Passphrase {
    Passphrase::new("pw-for-test-0123456789".to_string())
}

/// 置き場の下のファイルの名前（無ければ空）。
pub(crate) fn names(dir: &Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .map(|rd| {
            rd.map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default()
}

/// 書き終えた書き出しの名前を持つファイルも、`.partial-…` も無いこと。
fn assert_nothing_written(dir: &Path) {
    let left = names(dir);
    assert!(left.is_empty(), "書き出しのファイルが残っている: {left:?}");
}

fn env_of<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
    move |k| {
        pairs
            .iter()
            .find(|(n, _)| *n == k)
            .map(|(_, v)| (*v).to_string())
    }
}

// Scenario: 置き場の指定が無ければローカルのアプリのデータのフォルダの下に書く
#[test]
fn export_dir_defaults_under_local_app_data() {
    let local = r"C:\Users\me\AppData\Local";
    let got =
        export::export_dir(env_of(&[("LOCALAPPDATA", local), ("HOME", r"C:\Users\me")])).unwrap();
    assert_eq!(got, Path::new(local).join("ashiato").join("export"));
    let s = got.to_string_lossy().to_lowercase();
    for bad in ["documents", "desktop", "onedrive"] {
        assert!(!s.contains(bad), "{bad} の下にしてはいけない: {s}");
    }
}

#[test]
fn export_dir_order_is_env_then_localappdata_then_xdg_then_home() {
    let env = [
        ("ASHIATO_EXPORT_DIR", "/x/chosen"),
        ("LOCALAPPDATA", "/l"),
        ("XDG_DATA_HOME", "/xdg"),
        ("HOME", "/home/u"),
    ];
    let at = |n: usize| export::export_dir(env_of(&env[n..])).unwrap();
    assert_eq!(at(0), Path::new("/x/chosen"));
    assert_eq!(at(1), Path::new("/l/ashiato/export"));
    assert_eq!(at(2), Path::new("/xdg/ashiato/export"));
    assert_eq!(at(3), Path::new("/home/u/.local/share/ashiato/export"));
    // 空の値は無いものとして次へ進む。どれも無ければ決まらない。
    assert_eq!(
        export::export_dir(env_of(&[("ASHIATO_EXPORT_DIR", ""), ("HOME", "/h")])).unwrap(),
        Path::new("/h/.local/share/ashiato/export")
    );
    assert!(export::export_dir(env_of(&[])).is_none());
}

// Scenario: 置き場を環境で指定すればそこに書く
#[tokio::test]
async fn export_dir_writes_under_the_chosen_dir() {
    let _g = EXPORT_SERIAL.lock().await;
    let db = testdb::isolated().await;
    let pool = db.pool().await;
    let dir_tmp = tmp_dir("chosen");
    let dir = dir_tmp.path().to_path_buf();
    let cfg = ExportConfig::with_dir(dir.clone());
    let file = export::run(&pool, &cfg, pass()).await.unwrap();
    assert_eq!(file.parent().unwrap(), dir);
    let left = names(&dir);
    assert_eq!(left.len(), 1, "{left:?}");
    assert!(left[0].starts_with("ashiato-export-") && left[0].ends_with(".zip"));
    assert!(!left[0].starts_with('.'));
    // 閉じた zip として読める
    let z = zip::ZipArchive::new(std::fs::File::open(&file).unwrap()).unwrap();
    let tables = export::CATALOG
        .iter()
        .filter(|(_, c)| matches!(c, export::Class::Include { .. }))
        .count();
    assert_eq!(z.len(), tables + 2); // 表のファイル（CATALOG の Include）と説明書と目録（D7）
}

#[test]
fn final_name_never_replaces_an_existing_file() {
    let dir_tmp = tmp_dir("names");
    let dir = dir_tmp.path().to_path_buf();
    let stamp = "20261013-101500";
    let first = export::free_name(&dir, stamp);
    assert_eq!(first, dir.join("ashiato-export-20261013-101500.zip"));
    std::fs::write(&first, b"x").unwrap();
    let second = export::free_name(&dir, stamp);
    assert_eq!(second, dir.join("ashiato-export-20261013-101500-2.zip"));
    std::fs::write(&second, b"x").unwrap();
    assert_eq!(
        export::free_name(&dir, stamp),
        dir.join("ashiato-export-20261013-101500-3.zip")
    );
}

// Scenario: Unix では置き場と書き出しのファイルは持ち主だけが読み書きできる
#[cfg(unix)]
#[tokio::test]
async fn export_dir_and_file_are_owner_only_on_unix() {
    use std::os::unix::fs::PermissionsExt;
    let _g = EXPORT_SERIAL.lock().await;
    let db = testdb::isolated().await;
    let pool = db.pool().await;
    let base_tmp = tmp_dir("perm");
    let base = base_tmp.path().to_path_buf();
    let dir = base.join("not-yet");
    let file = export::run(&pool, &ExportConfig::with_dir(dir.clone()), pass())
        .await
        .unwrap();
    let mode = |p: &Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode(&dir), 0o700);
    assert_eq!(mode(&file), 0o600);
}

#[cfg(unix)]
#[tokio::test]
async fn export_dir_keeps_existing_permissions() {
    use std::os::unix::fs::PermissionsExt;
    let _g = EXPORT_SERIAL.lock().await;
    let db = testdb::isolated().await;
    let pool = db.pool().await;
    let dir_tmp = tmp_dir("keep");
    let dir = dir_tmp.path().to_path_buf();
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
    export::run(&pool, &ExportConfig::with_dir(dir.clone()), pass())
        .await
        .unwrap();
    assert_eq!(
        std::fs::metadata(&dir).unwrap().permissions().mode() & 0o777,
        0o755
    );
}

#[test]
fn passphrase_is_hidden_in_debug() {
    let p = Passphrase::new("secret-secret-secret".to_string());
    assert!(!format!("{p:?}").contains("secret"));
}

#[tokio::test]
async fn no_dir_fails_with_io() {
    let _g = EXPORT_SERIAL.lock().await;
    let db = testdb::isolated().await;
    let pool = db.pool().await;
    let e = export::run(&pool, &ExportConfig::without_dir(), pass())
        .await
        .unwrap_err();
    assert_eq!(e.kind, FailKind::Io);
}

// Scenario: いまの記録の schema の表と列はすべて分類されている
#[tokio::test]
async fn export_catalog_every_table_and_column_is_classified() {
    let db = testdb::isolated().await;
    let pool = db.pool().await;
    let inv = export::read_inventory(&pool).await.unwrap();
    assert!(!inv.tables.is_empty());
    let bad = export::unclassified(&inv);
    assert!(bad.is_empty(), "分類の無いもの: {bad:?}");
}

#[test]
fn export_catalog_excludes_archive_tables_and_ledgers() {
    for t in [
        "coverage",
        "erasure_ledger",
        "deletion_ledger",
        "export_ledger",
        "access_log",
        "web_session",
        "stay_absorbed",
        "archive_ledger",
        "archive_ledger_source",
        "archive_file",
        "archive_shape_confirmation",
        "archive_sighting",
        "archive_scan_counter",
        "archive_pending_shape",
        "archive_reread_failure",
    ] {
        assert!(
            matches!(export::class_of(t), Some(export::Class::Exclude { .. })),
            "{t} は Exclude"
        );
    }
    let names: Vec<_> = export::CATALOG.iter().map(|(n, _)| *n).collect();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), names.len(), "表の分類が重なっている");
}

/// 分類の段に渡す一覧を差し替えて本体を走らせ、失敗で終わること・何も残らないことを返す。
async fn run_with_inventory(
    tamper: impl Fn(&mut Inventory) + Send + Sync + 'static,
) -> (ExportError, tempfile::TempDir) {
    let _g = EXPORT_SERIAL.lock().await;
    let db = testdb::isolated().await;
    let pool = db.pool().await;
    let dir_tmp = tmp_dir("tamper");
    let mut cfg = ExportConfig::with_dir(dir_tmp.path().to_path_buf());
    cfg.hooks = Hooks::none().with_inventory(tamper);
    let e = export::run(&pool, &cfg, pass()).await.unwrap_err();
    (e, dir_tmp)
}

// Scenario: 分類の無い表があると書き出しは始まらない
#[tokio::test]
async fn export_catalog_unclassified_table_stops_the_export() {
    let (e, dir_tmp) = run_with_inventory(|inv| inv.tables.push("x".into())).await;
    assert_eq!(e.kind, FailKind::UnclassifiedTable);
    assert!(e.reason.contains('x'), "{}", e.reason);
    assert_nothing_written(dir_tmp.path());
}

// Scenario: 分類の無い列があると書き出しは始まらない
#[tokio::test]
async fn export_catalog_unclassified_column_stops_the_export() {
    let (e, dir_tmp) = run_with_inventory(|inv| inv.columns.push(("event".into(), "y".into()))).await;
    assert_eq!(e.kind, FailKind::UnclassifiedTable);
    assert!(
        e.reason.contains("event") && e.reason.contains('y'),
        "{}",
        e.reason
    );
    assert_nothing_written(dir_tmp.path());
}

#[test]
fn export_catalog_unclassified_names_table_and_column() {
    let mut inv = Inventory::default();
    inv.tables.push("x".into());
    inv.columns.push(("source".into(), "zzz".into()));
    let got = export::unclassified(&inv);
    assert!(got.contains(&Unclassified::Table("x".into())));
    assert!(got.contains(&Unclassified::Column {
        table: "source".into(),
        column: "zzz".into()
    }));
}

// Scenario: いまの記録の表の列はすべて読み出しにある
#[tokio::test]
async fn export_view_columns_every_exported_column_is_in_the_live_view() {
    let db = testdb::isolated().await;
    let pool = db.pool().await;
    let inv = export::read_inventory(&pool).await.unwrap();
    let missing = export::missing_view_columns(&inv);
    assert!(missing.is_empty(), "読み出しに無い列: {missing:?}");
    // 出さない列（txid）は突き合わせに入らない
    assert!(!export::CATALOG.iter().any(|(t, c)| *t == "event_version"
        && matches!(c, export::Class::Include { columns, .. }
            if columns.iter().any(|(n, _)| *n == "txid"))));
}

// Scenario: 読み出しに欠けた列があると書き出しは失敗する
#[tokio::test]
async fn export_view_columns_missing_column_stops_the_export() {
    for (view, col) in [("event_live", "tz_id"), ("event_version_live", "raw")] {
        let (e, dir_tmp) = run_with_inventory(move |inv| {
            inv.columns.retain(|(t, c)| !(t == view && c == col));
        })
        .await;
        assert_eq!(e.kind, FailKind::MissingColumn, "{view}.{col}");
        assert!(
            e.reason.contains(col) && e.reason.contains(view),
            "{}",
            e.reason
        );
        assert_nothing_written(dir_tmp.path());
    }
}

#[tokio::test]
async fn injected_failure_leaves_no_partial_file() {
    let _g = EXPORT_SERIAL.lock().await;
    let db = testdb::isolated().await;
    let pool = db.pool().await;
    let dir_tmp = tmp_dir("inject");
    let dir = dir_tmp.path().to_path_buf();
    let mut cfg = ExportConfig::with_dir(dir.clone());
    cfg.hooks = Hooks::none().with_fail(|s| {
        (s == Stage::Write).then(|| ExportError::new(FailKind::Io, "差し込んだ失敗"))
    });
    let e = export::run(&pool, &cfg, pass()).await.unwrap_err();
    assert_eq!(e.kind, FailKind::Io);
    assert_nothing_written(&dir);
}

/// 写しを流している途中（`.partial-…` に前の項目を書いた後）で落ちても、`.partial-…` を残さない（R6）。
#[tokio::test]
async fn failure_in_the_middle_of_the_stream_leaves_no_partial_file() {
    let _g = EXPORT_SERIAL.lock().await;
    let db = testdb::isolated().await;
    let pool = db.pool().await;
    let dir_tmp = tmp_dir("inject-mid");
    let dir = dir_tmp.path().to_path_buf();
    let items = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let partial_seen = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let mut cfg = ExportConfig::with_dir(dir.clone());
    let (n, seen, d) = (items.clone(), partial_seen.clone(), dir.clone());
    cfg.hooks = Hooks::none().with_fail(move |s| {
        if s != Stage::Item {
            return None;
        }
        // 3 つ目の項目の手前 = 記録と前の版を `.partial-…` に書いた後
        if n.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 2 {
            let partial = names(&d).into_iter().any(|f| f.starts_with(".partial-"));
            seen.store(partial, std::sync::atomic::Ordering::SeqCst);
            return Some(ExportError::new(FailKind::Io, "差し込んだ途中の失敗"));
        }
        None
    });
    let e = export::run(&pool, &cfg, pass()).await.unwrap_err();
    assert_eq!(e.kind, FailKind::Io);
    assert!(
        partial_seen.load(std::sync::atomic::Ordering::SeqCst),
        "前提: 落とした時点で .partial- がある"
    );
    assert_nothing_written(&dir);
}

#[tokio::test]
async fn hooks_run_around_each_stage_in_order() {
    let _g = EXPORT_SERIAL.lock().await;
    let db = testdb::isolated().await;
    let pool = db.pool().await;
    let dir_tmp = tmp_dir("around");
    let dir = dir_tmp.path().to_path_buf();
    let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
    let log = seen.clone();
    let mut cfg = ExportConfig::with_dir(dir);
    cfg.hooks = Hooks::none().with_around(move |s, p| {
        log.lock().unwrap().push((s, p));
        Box::pin(async {})
    });
    export::run(&pool, &cfg, pass()).await.unwrap();
    use export::Phase::{After, Before};
    assert_eq!(
        *seen.lock().unwrap(),
        vec![
            (Stage::Catalog, Before),
            (Stage::Catalog, After),
            (Stage::Columns, Before),
            (Stage::Columns, After),
            (Stage::Write, Before),
            (Stage::Write, After),
        ]
    );
}
