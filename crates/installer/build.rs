use std::path::Path;

/// 組み立ての印が見る入力。`main.rs` が `include_bytes!` で埋める `tools/db-roles.sql` も入れる
/// （入れないと、commit していない SQL を埋めても印に `.dirty` が付かない。review R85）
const INPUTS: &[&str] = &[
    "crates/installer",
    "crates/build-stamp",
    "tools/db-roles.sql",
    "Cargo.toml",
    "Cargo.lock",
];

fn main() {
    println!("cargo:rerun-if-env-changed=ASHIATO_RELEASE");
    println!("cargo:rustc-check-cfg=cfg(ashiato_test_build)");
    if std::env::var("ASHIATO_RELEASE").as_deref() != Ok("1") {
        println!("cargo:rustc-cfg=ashiato_test_build");
    }
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let version = ashiato_build_stamp::version(env!("CARGO_PKG_VERSION"))
        .expect("CARGO_PKG_VERSION は作業場の版番号の形である必要があります");
    let stamp = ashiato_build_stamp::stamp(
        &version,
        &std::env::var("PROFILE").unwrap_or_default(),
        &repo,
        INPUTS,
    );
    ashiato_build_stamp::emit_rerun(&repo, INPUTS);
    println!("cargo:rustc-env=ASHIATO_BUILD_STAMP={stamp}");
    println!("cargo:rustc-env=ASHIATO_VERSION={version}");
    // 同梱する PostgreSQL の大きい版（D5）。入っている DB の `PG_VERSION` と比べる
    let lock = repo.join("tools/release/pgsql.lock");
    println!("cargo:rerun-if-changed={}", lock.display());
    let text = std::fs::read_to_string(&lock).expect("tools/release/pgsql.lock を読めない");
    let major = text
        .lines()
        .find_map(|line| line.strip_prefix("version="))
        .and_then(|version| version.split('.').next())
        .filter(|major| !major.is_empty() && major.bytes().all(|b| b.is_ascii_digit()))
        .expect("pgsql.lock の version= から大きい版を読めない");
    println!("cargo:rustc-env=ASHIATO_PG_MAJOR={major}");
}
