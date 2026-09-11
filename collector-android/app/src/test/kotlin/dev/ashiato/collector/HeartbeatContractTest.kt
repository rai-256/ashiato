// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * 生存信号の**欄名**が契約と一致すること（`docs/openapi.json` / `docs/collector-contract.md`）。
 *
 * **grep では守れない。** `tools/check-openapi.sh` が Kotlin のファイルを grep する形は、
 * 同じ語が docstring や別の行にも出るので**空振りする**（実測: `capturable` を
 * `capturableX` に改名しても検査は緑のままだった）。
 * ここは**実際に直列化した結果の鍵**を見るので、改名すればその場で落ちる。
 *
 * 欄名がずれると、サーバは断り、未送信に居座り、logcat に 1 行出るだけになる
 * —— 実機を持って歩いてから気付く型の失敗。
 */
class HeartbeatContractTest {
    // CONTRACT-FIELDS-BEGIN（tools/check-openapi.sh が docs/openapi.json と突き合わせる）
    private val contractFields = listOf(
        "id",
        "user_id",
        "logical_source",
        "device_id",
        "emitted_at",
        "capturable",
        "blockers",
        "attempts",
        "successes",
        "raw",
    )
    // CONTRACT-FIELDS-END

    @Test
    fun `直列化した生存信号の欄名が契約と一致する`() {
        val request = HeartbeatRequest(
            id = "11111111-1111-4111-8111-111111111111",
            userId = "00000000-0000-0000-0000-000000000000",
            logicalSource = LOGICAL_SOURCE,
            deviceId = "device-1",
            emittedAt = "2026-05-01T00:00:00Z",
            capturable = true,
            blockers = emptyList(),
            attempts = 360,
            successes = 230,
            raw = """{"alive":true}""",
        )
        val keys = (Json.parseToJsonElement(ingestJson.encodeToString(request)) as JsonObject).keys
        assertEquals(contractFields.sorted(), keys.sorted())
    }

    /** **`explicitNulls = true`** なので、省いた欄も鍵としては出る（契約どおり）。 */
    @Test
    fun `端末識別子を省いても欄そのものは出る`() {
        val request = HeartbeatRequest(
            id = "a",
            userId = "b",
            logicalSource = LOGICAL_SOURCE,
            emittedAt = "2026-05-01T00:00:00Z",
            capturable = true,
            attempts = 1,
            successes = 1,
            raw = "{}",
        )
        val keys = (Json.parseToJsonElement(ingestJson.encodeToString(request)) as JsonObject).keys
        assertEquals(contractFields.sorted(), keys.sorted())
    }
}
