// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

/**
 * 取り込み口へ送る 1 件。**Rust 側（crates/server/src/ingest.rs）と同じ形**でなければならない。
 * 契約の正典は docs/collector-contract.md。片方だけ直すと、同じ 1 件が別物として入る。
 */
data class IngestRequest(
    val id: String,
    val userId: String,
    val logicalSource: String,
    val externalId: String?,
    val deviceId: String?,
    val origin: String,
    val eventTime: String,
    val tzOffsetMin: Int,
    val tzId: String,
    val schemaVersion: Int,
    val raw: String,
    val payload: String,
)
