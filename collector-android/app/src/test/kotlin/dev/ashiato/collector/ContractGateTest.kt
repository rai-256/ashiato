package dev.ashiato.collector

import org.junit.Assert.assertEquals
import org.junit.Test

/** ST50 契約ゲートの互換入口。検証台本が `*ContractGate*` で実行する。 */
class ContractGateTest {
    @Test
    fun `ST50 contract answers keep their existing classifications`() {
        assertEquals(ContractAnswer.NoEndpoint, readContractAnswer(Outcome.Responded(404, "")))
        assertEquals(ContractAnswer.Unreadable, readContractAnswer(Outcome.Responded(200, "not-json")))
    }
}
