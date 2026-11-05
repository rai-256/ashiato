// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
//! `migrations/ORDER` から `MIGRATIONS`（当てる版の名前と中身の配列）を組む。
//!
//! 並びを Rust の配列に手で書いていたときは、並走する Story がみな配列の末尾と要素数の行を書き換え、
//! どの 2 本を合わせても衝突した（2026-10-15 実測: ST08・ST14・ST17・ST47 の 6 組すべて）。
//! ORDER は 1 行 1 版で、`.gitattributes` の `merge=union` で追記どうしが衝突しない。
//!
//! 落とすもの（ビルドを止める）: ORDER に無い `.sql`（当て忘れ）・ORDER にあるがファイルが無い版・同じ版が 2 回。
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::{env, fs};

/// S-01 のソースの道（design D5）。`.dirty` の判定と、cargo への作り直しの印の両方に使う。
const STAMP_SOURCES: &[&str] = &[
    "crates/server",
    "crates/build-stamp",
    "migrations",
    "Cargo.toml",
    "Cargo.lock",
];

fn main() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../migrations");
    let dir = dir
        .canonicalize()
        .unwrap_or_else(|e| panic!("{} が無い: {e}", dir.display()));
    let order = dir.join("ORDER");
    let lineage = dir.join("LINEAGE");
    println!("cargo:rerun-if-changed={}", order.display());
    println!("cargo:rerun-if-changed={}", lineage.display());
    println!("cargo:rerun-if-changed={}", dir.display());
    println!("cargo:rerun-if-env-changed=ASHIATO_WEB_DIST");
    println!("cargo:rerun-if-env-changed=ASHIATO_RELEASE");

    // 組み立ての印（ST49 / design D5・D6）。ST48 の台帳が読む `ASHIATO_BUILD_COMMIT` も同じ commit で出し続ける
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let version =
        ashiato_build_stamp::version(env!("CARGO_PKG_VERSION")).unwrap_or_else(|e| panic!("{e}"));
    let (stamp, commit) = ashiato_build_stamp::stamp_with_commit(
        &version,
        &env::var("PROFILE").unwrap_or_default(),
        &repo,
        STAMP_SOURCES,
    );
    ashiato_build_stamp::emit_rerun(&repo, STAMP_SOURCES);
    println!("cargo:rustc-env=ASHIATO_BUILD_STAMP={stamp}");
    println!("cargo:rustc-env=ASHIATO_BUILD_COMMIT={commit}");
    println!("cargo:rustc-env=ASHIATO_VERSION={version}");

    let text = fs::read_to_string(&order)
        .unwrap_or_else(|e| panic!("{} を読めない: {e}", order.display()));
    let mut seen = BTreeSet::new();
    let mut names = Vec::new();
    for line in text.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        assert!(
            seen.insert(line.to_string()),
            "migrations/ORDER に {line} が 2 回ある"
        );
        let sql = dir.join(format!("{line}.sql"));
        assert!(
            sql.is_file(),
            "migrations/ORDER の {line} に当たる {} が無い",
            sql.display()
        );
        names.push((line.to_string(), sql));
    }

    let mut missing: Vec<String> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{} を読めない: {e}", dir.display()))
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|f| f.ends_with(".sql") && !f.ends_with(".down.sql"))
        .map(|f| f.trim_end_matches(".sql").to_string())
        .filter(|n| !seen.contains(n))
        .collect();
    missing.sort();
    assert!(
        missing.is_empty(),
        "migrations/ORDER に無い版がある（末尾に 1 行足す）: {}",
        missing.join(", ")
    );

    let mut body = String::new();
    for (n, p) in &names {
        writeln!(
            body,
            "    ({n:?}, include_str!({:?})),",
            p.display().to_string()
        )
        .expect("String に書く");
    }
    let out = Path::new(&env::var("OUT_DIR").expect("OUT_DIR")).join("migrations.rs");
    fs::write(&out, format!("&[\n{body}]\n"))
        .unwrap_or_else(|e| panic!("{} を書けない: {e}", out.display()));

    let lineage_text = fs::read_to_string(&lineage)
        .unwrap_or_else(|e| panic!("{} を読めない: {e}", lineage.display()));
    let mut lineage_body = String::new();
    // 行末の `# …` は説明。tools/check-migrations.sh・tools/check-nightly-migrations.sh と同じく外してから欄を割る
    for line in lineage_text
        .lines()
        .map(|l| l.split_once('#').map_or(l, |(body, _)| body).trim())
    {
        if line.is_empty() {
            continue;
        }
        let fields: Vec<_> = line.split_whitespace().collect();
        let (kind, old, new) = match fields.as_slice() {
            ["renamed", old, new] => ("Renamed", *old, *new),
            ["removed", old] => ("Removed", *old, ""),
            ["rewritten", old, new] => ("Rewritten", *old, *new),
            _ => panic!("migrations/LINEAGE の行の形が違う: {line}"),
        };
        writeln!(
            lineage_body,
            "    (crate::migration_ledger::LineageKind::{kind}, {old:?}, {new:?}),"
        )
        .expect("String に書く");
    }
    let lineage_out = Path::new(&env::var("OUT_DIR").expect("OUT_DIR")).join("lineage.rs");
    fs::write(&lineage_out, format!("&[\n{lineage_body}]\n"))
        .unwrap_or_else(|e| panic!("{} を書けない: {e}", lineage_out.display()));

    write_web_assets();
}

fn write_web_assets() {
    let dist = env::var_os("ASHIATO_WEB_DIST").map(PathBuf::from);
    let files = match dist.as_deref() {
        None => Vec::new(),
        Some(path) if path.is_dir() => {
            println!("cargo:rerun-if-changed={}", path.display());
            let files = collect_files(path);
            for (_, file) in &files {
                println!("cargo:rerun-if-changed={}", file.display());
            }
            files
        }
        Some(path) => panic!("ASHIATO_WEB_DIST がディレクトリでない: {}", path.display()),
    };
    if env::var("ASHIATO_RELEASE").as_deref() == Ok("1") && files.is_empty() {
        panic!("ASHIATO_RELEASE=1 には画面が必要（ASHIATO_WEB_DIST が空）");
    }

    let mut body = String::from("&[\n");
    for (path, absolute) in files {
        let path = path.replace('\\', "/");
        writeln!(
            body,
            "    crate::web_assets::Asset {{ path: {path:?}, body: include_bytes!({:?}), content_type: {:?} }},",
            absolute.display().to_string(),
            content_type(&path),
        )
        .expect("画面の表を書く");
    }
    body.push_str("]\n");
    let out = Path::new(&env::var("OUT_DIR").expect("OUT_DIR")).join("web_assets.rs");
    fs::write(&out, body).unwrap_or_else(|e| panic!("{} を書けない: {e}", out.display()));
}

fn collect_files(root: &Path) -> Vec<(String, PathBuf)> {
    fn visit(root: &Path, dir: &Path, files: &mut Vec<(String, PathBuf)>) {
        let mut entries: Vec<_> = fs::read_dir(dir)
            .unwrap_or_else(|e| panic!("{} を読めない: {e}", dir.display()))
            .map(|entry| entry.unwrap_or_else(|e| panic!("画面の entry を読めない: {e}")))
            .collect();
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let path = entry.path();
            if path.is_dir() {
                visit(root, &path, files);
            } else if path.is_file() {
                let relative = path
                    .strip_prefix(root)
                    .expect("画面の相対 path")
                    .to_string_lossy()
                    .replace('\\', "/");
                files.push((format!("/{relative}"), path));
            }
        }
    }

    let mut files = Vec::new();
    visit(root, root, &mut files);
    files.sort_by(|a, b| a.0.cmp(&b.0));
    files
}

fn content_type(path: &str) -> &'static str {
    match Path::new(path).extension().and_then(|ext| ext.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js" | "mjs") => "text/javascript; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("ico") => "image/x-icon",
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        _ => "application/octet-stream",
    }
}
