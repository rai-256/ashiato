package dev.ashiato.collector

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import java.time.Instant

class UpdateCheckTest {
    @Test
    fun `3 つの部品は同じ材料で同じ版を選ぶ`() {
        // Scenario: 3 つの部品は同じ材料で同じ版を選ぶ
        val releases = """
            [{"tag_name":"v0.5.0","draft":false,"assets":[
              {"name":"ashiato-server-0.5.0-windows-x86_64.zip"},
              {"name":"ashiato-collector-windows-0.5.0-x86_64.zip"},
              {"name":"ashiato-collector-0.5.0.apk"}]}]
        """.trimIndent()
        assertEquals("0.5.0", UpdateCheck.choose("server", "0.4.0", releases).latest)
        assertEquals("0.5.0", UpdateCheck.choose("collector-windows", "0.4.0", releases).latest)
        assertEquals("0.5.0", UpdateCheck.choose("android", "0.4.0", releases).latest)
    }

    @Test
    fun `due は 24 時間と未来の時計を扱う`() {
        val now = Instant.parse("2026-11-12T00:00:00Z")
        assertTrue(UpdateCheck.due(now, now.minusSeconds(24 * 60 * 60)))
        assertTrue(UpdateCheck.due(now, now.plusSeconds(1)))
    }
}
