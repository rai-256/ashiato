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
    /** 積んだ時の契約の版（ST50）は全要求に付く欄で、`tools/check-contract-version.sh` が見る。この一覧は欄の名前の突き合わせ（check-openapi.sh）用 */
    private val contractVersionField = "contract_version"

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
        "client_version",
    )
    // CONTRACT-FIELDS-END

    /** 版を持つ下書き（ST49）。版の無い下書きの報告は `client_version` を欄ごと送らない（`St49DropDraftTest` が見る） */
    private val version = "0.1.0+0123456789abcdef0123456789abcdef01234567.debug"

    private fun keys(r: DropReport): Set<String> {
        val json = Json.parseToJsonElement(ingestJson.encodeToString(DropReport.serializer(), r)) as JsonObject
        return json.keys - contractVersionField
    }

    @Test
    fun `直列化した破棄の報告の欄名が契約と一致する`() {
        val r = DropDraft(
            id = "a",
            source = LOGICAL_SOURCE,
            reason = "age",
            createdAt = "2026-09-14T00:00:00Z",
            startMs = 1_000L,
            endMs = 2_000L,
            lastMs = 1_000L,
            count = 1,
            hourly = mapOf(0L to 1),
            clientVersion = version,
        ).toReport("user-1", "device-1")
        assertEquals(contractFields.sorted(), keys(r).sorted())
        // 原文も同じ欄（raw と、鍵に入れない client_version を除く）を持つ
        val rawKeys = (Json.parseToJsonElement(r.raw) as JsonObject).keys
        assertEquals((contractFields - "raw" - "client_version").sorted(), rawKeys.sorted())
    }

    @Test
    fun `範囲を持たない報告でも欄そのものは出る`() {
        val r = DropDraft(
            id = "a",
            source = LOGICAL_SOURCE,
            reason = "unreadable",
            createdAt = "2026-09-14T00:00:00Z",
            count = 2,
            clientVersion = version,
        ).toReport("user-1", "device-1")
        assertEquals(contractFields.sorted(), keys(r).sorted())
    }
}
