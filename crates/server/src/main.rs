// SPDX-License-Identifier: AGPL-3.0-only
//! S-01 の起動口。中身は lib.rs にある（`bin/openapi.rs` から同じ型を読むため）。

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    match std::env::args().nth(1).as_deref() {
        None => ashiato_server::run().await,
        Some("migrate") => ashiato_server::run_migrate().await,
        Some(other) => anyhow::bail!("知らない引数: {other}（使えるのは migrate だけ）"),
    }
}
