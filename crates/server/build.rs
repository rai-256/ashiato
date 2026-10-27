// SPDX-License-Identifier: AGPL-3.0-only
//! `migrations/ORDER` から `MIGRATIONS`（当てる版の名前と中身の配列）を組む。
//!
//! 並びを Rust の配列に手で書いていたときは、並走する Story がみな配列の末尾と要素数の行を書き換え、
//! どの 2 本を合わせても衝突した（2026-10-15 実測: ST08・ST14・ST17・ST47 の 6 組すべて）。
//! ORDER は 1 行 1 版で、`.gitattributes` の `merge=union` で追記どうしが衝突しない。
//!
//! 落とすもの（ビルドを止める）: ORDER に無い `.sql`（当て忘れ）・ORDER にあるがファイルが無い版・同じ版が 2 回。
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::Path;
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
    println!("cargo:rerun-if-changed={}", order.display());
    println!("cargo:rerun-if-changed={}", dir.display());

    // 組み立ての印（ST49 / design D5・D6）。ST48 の台帳が読む `ASHIATO_BUILD_COMMIT` も同じ commit で出し続ける
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let stamp = ashiato_build_stamp::stamp(
        env!("CARGO_PKG_VERSION"),
        &env::var("PROFILE").unwrap_or_default(),
        &repo,
        STAMP_SOURCES,
    );
    ashiato_build_stamp::emit_rerun(&repo, STAMP_SOURCES);
    println!("cargo:rustc-env=ASHIATO_BUILD_STAMP={stamp}");
    println!("cargo:rustc-env=ASHIATO_BUILD_COMMIT={}", commit_of(&stamp));

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
}

/// 印 `<版番号>+<commit>[.dirty][.debug]` の commit（40 桁か `unknown`、環境変数で渡された値）。
fn commit_of(stamp: &str) -> &str {
    let (_, rest) = stamp.split_once('+').expect("印は + を持つ");
    rest.trim_end_matches(".debug").trim_end_matches(".dirty")
}
