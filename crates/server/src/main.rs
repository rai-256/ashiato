// SPDX-License-Identifier: AGPL-3.0-only
//! S-01 の起動口。中身は lib.rs にある（`bin/openapi.rs` から同じ型を読むため）。

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    ashiato_server::run().await
}
