// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * 破棄の報告の**欄名**が契約と一致すること（ST04 / `docs/openapi.json` / `docs/collector-contract.md`）。
 * `HeartbeatContractTest` と同じ理由で、grep ではなく**実際に直列化した鍵**を見る。
 *
 * 欄名がずれると、サーバは断り、**破棄の報告は断られても取り除かない**ので未送信に居座り続ける（C2）。
 */
class DropReportContractTest {
    // CONTRACT-FIELDS-BEGIN（tools/check-openapi.sh が docs/openapi.json と突き合わせる）
    private val contractFields = listOf(
        "id",
        "user_id",
        "logical_source",
        "device_id",
        "reason",
        "created_at",
        "range_start",
        "range_end",
        "count",
        "hourly",
        "raw",
    )
    // CONTRACT-FIELDS-END

    private fun keys(r: DropReport) = (Json.parseToJsonElement(ingestJson.encodeToString(DropReport.serializer(), r)) as JsonObject).keys

    @Test
    fun `直列化した破棄の報告の欄名が契約と一致する`() {
        val r = DropDraft("a", LOGICAL_SOURCE, "age", "2026-09-14T00:00:00Z", 1_000L, 2_000L, 1_000L, 1, mapOf(0L to 1))
            .toReport("user-1", "device-1")
        assertEquals(contractFields.sorted(), keys(r).sorted())
        // 原文も同じ欄（raw を除く）を持つ
        assertEquals((contractFields - "raw").sorted(), (Json.parseToJsonElement(r.raw) as JsonObject).keys.sorted())
    }

    @Test
    fun `範囲を持たない報告でも欄そのものは出る`() {
        val r = DropDraft("a", LOGICAL_SOURCE, "unreadable", "2026-09-14T00:00:00Z", count = 2).toReport("user-1", "device-1")
        assertEquals(contractFields.sorted(), keys(r).sorted())
    }
}
