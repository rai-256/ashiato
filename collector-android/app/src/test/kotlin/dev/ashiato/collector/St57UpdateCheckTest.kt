package dev.ashiato.collector

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.After
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import java.time.Instant
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.JsonNull
import java.io.File

@RunWith(RobolectricTestRunner::class)
class St57UpdateCheckTest {
    @After fun cleanup() {
        Config.clearOverrideForTest()
        androidx.test.core.app.ApplicationProvider.getApplicationContext<android.content.Context>()
            .getSharedPreferences(UpdateCheck.PREFS, 0).edit().clear().commit()
    }

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
    fun `共有ベクトルの全ケースと時期を同じ選び方で評価する`() {
        // Scenario: 3 つの部品は同じ材料で同じ版を選ぶ
        val root = Json.parseToJsonElement(File("../../docs/contracts/update-check-vectors.json").readText()).jsonObject
        root["cases"]!!.jsonArray.forEach { item ->
            val value = item.jsonObject
            val releases = value["releases"].toString()
            val choice = UpdateCheck.choose(value["component"]!!.jsonPrimitive.content, value["installed"]!!.jsonPrimitive.content, releases)
            val expectedLatest = value["latest"]?.takeUnless { it is JsonNull }?.jsonPrimitive?.content
            assertEquals(expectedLatest, choice.latest)
            assertEquals(value["update_available"]!!.jsonPrimitive.content.toBoolean(), choice.available)
        }
        val now = Instant.parse("2026-11-12T00:00:00Z")
        root["timing"]!!.jsonArray.forEach { item ->
            val value = item.jsonObject
            val hours = value["elapsed_hours"]?.jsonPrimitive?.content?.toLongOrNull()
            val days = value["elapsed_days"]?.jsonPrimitive?.content?.toLongOrNull()
            val seconds = value["elapsed_seconds"]?.jsonPrimitive?.content?.toLongOrNull()
            val expected = value["expected"]!!.jsonPrimitive.content.toBoolean()
            when (value["kind"]!!.jsonPrimitive.content) {
                "due" -> assertEquals(expected, UpdateCheck.due(now, now.minusSeconds((hours ?: 0) * 3600)))
                "stale", "stale_seconds" -> assertEquals(expected, when {
                    seconds != null -> UpdateCheck.stale(now, now.minusSeconds(seconds))
                    days != null -> UpdateCheck.stale(now, now.minusSeconds(days * 86400))
                    else -> UpdateCheck.stale(now, null)
                })
            }
        }
    }

    @Test
    fun `確認記録は noBackupFilesDir に置く`() {
        // Scenario: 携帯の確かめた記録はバックアップから外れる
        val context = androidx.test.core.app.ApplicationProvider.getApplicationContext<android.content.Context>()
        context.getSharedPreferences(UpdateCheck.PREFS, 0).edit().clear().putBoolean("intro_shown", true).commit()
        Config.overrideForTest("http://127.0.0.1", "token", Config.DEFAULT_USER_ID)
        UpdateChecker(context, "http://127.0.0.1:1").check(Instant.parse("2026-11-12T00:00:00Z"))
        val expected = File(context.noBackupFilesDir, UpdateCheck.RECORDS_FILE)
        assertTrue(expected.isFile)
        assertEquals(context.noBackupFilesDir, expected.parentFile)
        Config.clearOverrideForTest()
    }

    @Test
    fun `nightly の単一応答も選べる`() {
        // Scenario: 携帯は新しい版をサーバに問い合わせない
        val body = """{"tag_name":"nightly","draft":false,"assets":[{"name":"ashiato-collector-dev-0.4.0-dev.20261110.apk"}]}"""
        assertEquals("0.4.0-dev.20261110", UpdateCheck.choose("android", "0.4.0-dev.20261102", body).latest)
    }

    @Test
    fun `送り先が未設定なら更新確認を記録せずに止める`() {
        // Scenario: 携帯の収集アプリは送り先を設定するまで確かめない
        val context = androidx.test.core.app.ApplicationProvider.getApplicationContext<android.content.Context>()
        context.getSharedPreferences(UpdateCheck.PREFS, 0).edit().clear().putBoolean("intro_shown", true).commit()
        Config.clearOverrideForTest()
        assertEquals(false, UpdateChecker(context).check(Instant.parse("2026-11-12T00:00:00Z")))
        assertEquals(null, context.getSharedPreferences(UpdateCheck.PREFS, 0).getString("checked_at", null))
    }

    @Test
    fun `due は 24 時間と未来の時計を扱う`() {
        val now = Instant.parse("2026-11-12T00:00:00Z")
        assertTrue(UpdateCheck.due(now, now.minusSeconds(24 * 60 * 60)))
        assertTrue(UpdateCheck.due(now, now.plusSeconds(1)))
    }
}
