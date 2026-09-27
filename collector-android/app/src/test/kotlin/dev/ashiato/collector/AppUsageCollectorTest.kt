// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.app.Application
import android.app.usage.UsageEvents
import android.app.usage.UsageStatsManager
import android.content.res.Configuration
import android.os.PersistableBundle
import android.os.UserManager
import androidx.test.core.app.ApplicationProvider
import java.time.Instant
import org.junit.Assert.assertEquals
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows
import org.robolectric.shadows.ShadowUsageStatsManager

/**
 * アプリ利用のイベントが記録になるところ（tasks 3.1 / design D1 / D2）。
 *
 * **ふるいにかけないことと、原文に表示名を入れないこと**が主題 ——
 * 絞った分は取得元で 10 日しか残らず、表示名を原文に入れると
 * アプリの更新や端末の言語の変更で冪等キーが動いて同じイベントが行を増やす。
 */
@RunWith(RobolectricTestRunner::class)
class AppUsageCollectorTest {
    private val t0 = "2026-05-20T09:00:00Z"

    /**
     * 1 契機ぶん進める（30 分）。最初の契機は保存された終わりを作るためだけに回す ——
     * 「前回以降」を試すには前回が要る。
     */
    private fun UsageTestEnv.firstThenNext(): CollectionResult {
        collect()
        advance(USAGE_INTERVAL_MS)
        return collect()
    }

    // Scenario: 契機ごとに前回以降のイベントが記録になる
    @Test
    fun `前回の取得の後に起きた 3 件がそれぞれ 1 件の記録になる`() {
        val env = UsageTestEnv(
            source = FakeUsageSource(
                storedEvents = listOf(
                    usageEvent("2026-05-20T09:10:00Z"),
                    usageEvent("2026-05-20T09:20:00Z"),
                    usageEvent("2026-05-20T09:25:00Z"),
                ),
            ),
        )
        val result = env.firstThenNext()
        assertTrue("取れたのに $result が返った", result is CollectionResult.Collected)
        assertEquals(3, (result as CollectionResult.Collected).enqueued.size)
        // **1 イベント = 1 記録**（本人の決定 C1）。1 回の取得を 1 件にまとめない
        assertEquals(
            listOf("2026-05-20T09:10:00Z", "2026-05-20T09:20:00Z", "2026-05-20T09:25:00Z"),
            env.records().map { it.eventTime },
        )
        assertEquals(3, env.records().size)
        assertEquals(listOf(APP_USAGE_LOGICAL_SOURCE), env.records().map { it.logicalSource }.distinct())
    }

    /**
     * 記録の組み立て側（`toIngestRequest`）が種別で落としていないこと。
     * **印は本番の変換を通す試験のほうにある**（code-verify R23）——
     * ここは取得元の偽物から `UsageEventSnapshot` を直に受け取るので、
     * 本番の `UsageStatsSource.snapshotOf` を 1 度も通らない。
     */
    @Test
    fun `前景の出入りも画面の入切もロックも待機の階級も全部記録になる`() {
        // 種別の番号は `UsageEvents.Event` のもの:
        // 1 = ACTIVITY_RESUMED / 2 = ACTIVITY_PAUSED / 15 = SCREEN_INTERACTIVE /
        // 16 = SCREEN_NON_INTERACTIVE / 17 = KEYGUARD_SHOWN / 18 = KEYGUARD_HIDDEN /
        // 11 = STANDBY_BUCKET_CHANGED
        val types = listOf(1, 2, 15, 16, 17, 18, 11)
        val env = UsageTestEnv(
            source = FakeUsageSource(
                storedEvents = types.mapIndexed { i, type ->
                    usageEvent("2026-05-20T09:${(10 + i).toString().padStart(2, '0')}:00Z", eventType = type)
                },
            ),
        )
        env.firstThenNext()
        // **1 件も落ちていない**（本人の決定 Q1）。落とした分は 10 日で取り返せない
        assertEquals(types.size, env.records().size)
        assertEquals(types, env.records().map { it.payloadText("event_type")?.toInt() })
    }

    /**
     * 欄の並びと `payload` の鍵の集合。**印は本番の変換を通す試験のほうにある**（code-verify R23）。
     * ここが見ているのは「`UsageEventSnapshot` の全欄が記録に写るか」で、
     * 「取得元が公開している欄が `UsageEventSnapshot` に写るか」ではない。
     */
    @Test
    fun `種別ごとの欄まで解析済みに入る`() {
        val configuration = Configuration().apply { orientation = Configuration.ORIENTATION_LANDSCAPE }
        val env = UsageTestEnv(
            source = FakeUsageSource(
                storedEvents = listOf(fullUsageEvent("2026-05-20T09:10:00Z", configuration)),
            ),
        )
        env.firstThenNext()
        val record = env.records().single()
        // 共通の 4 つ
        assertEquals("dev.ashiato.example", record.payloadText("package"))
        assertEquals("dev.ashiato.example.MainActivity", record.payloadText("class"))
        assertEquals("7", record.payloadText("event_type"))
        assertEquals("2026-05-20T09:10:00Z", record.payloadText("event_time"))
        // 種別ごとに付く 4 つ（設定の変化・ショートカット・操作の中身・待機の階級）
        assertEquals(configuration.toString(), record.payloadText("configuration"))
        assertEquals("shortcut-1", record.payloadText("shortcut_id"))
        assertEquals("android.intent.action.VIEW", record.payloadText("interaction_action"))
        assertEquals("android.intent.category.DEFAULT", record.payloadText("interaction_category"))
        assertEquals("20", record.payloadText("standby_bucket"))
        // **欄はこれで全部**（取得元が公開しているもの ＋ 取得時に解決した表示名）
        assertEquals(
            setOf(
                "package", "class", "event_type", "event_time", "configuration",
                "shortcut_id", "interaction_action", "interaction_category", "standby_bucket",
                "app_label",
            ),
            record.payload.keys,
        )
    }

    /** 付かない種別の欄は**入れない**（取得元が返していない値を記録に混ぜない）。 */
    @Test
    fun `付かない欄は省かれる`() {
        val env = UsageTestEnv(
            source = FakeUsageSource(storedEvents = listOf(usageEvent("2026-05-20T09:10:00Z"))),
        )
        env.firstThenNext()
        val record = env.records().single()
        assertEquals(
            setOf("package", "class", "event_type", "event_time", "app_label"),
            record.payload.keys,
        )
        assertNull(record.payloadText("standby_bucket"))
    }

    // Scenario: 表示名は解析済みにだけ入る
    @Test
    fun `表示名は解析済みに入り原文には入らない`() {
        val env = UsageTestEnv(
            source = FakeUsageSource(storedEvents = listOf(usageEvent("2026-05-20T09:10:00Z"))),
            label = "例のアプリ",
        )
        env.firstThenNext()
        val record = env.records().single()
        assertEquals("例のアプリ", record.payloadText("app_label"))
        // **原文に表示名は入っていない**（design D2）。冪等キーは原文から作られる
        assertFalse("原文に表示名が入っている: ${record.raw}", record.raw.contains("例のアプリ"))
        assertNull(record.rawText("app_label"))
    }

    /** 表示名が引けない（アプリが消えている）ときは、欄ごと省く。 */
    @Test
    fun `表示名が引けなければ欄ごと省く`() {
        val env = UsageTestEnv(
            source = FakeUsageSource(storedEvents = listOf(usageEvent("2026-05-20T09:10:00Z"))),
            label = null,
        )
        env.firstThenNext()
        assertNull(env.records().single().payloadText("app_label"))
    }

    // Scenario: 同じイベントの原文は毎回同じ文字列になる
    @Test
    fun `重ねた窓で 2 回読んでも原文のバイト列は一致する`() {
        // 窓は重ねて進むので（本人の決定 C4）、境界の手前のイベントは次の契機でもう 1 度返る
        val env = UsageTestEnv(
            source = FakeUsageSource(storedEvents = listOf(usageEvent("2026-05-20T08:59:30Z"))),
            label = "旧い名前",
        )
        env.collect()
        val first = env.records().single()
        // **取得のあいだにアプリの表示名が変わった**（更新・端末の言語の変更）
        env.label = "新しい名前"
        env.advance(USAGE_INTERVAL_MS)
        env.collect()
        val again = env.records().last()
        assertEquals(2, env.records().size)
        assertArrayEquals(
            "同じイベントの原文が変わった: ${first.raw} / ${again.raw}",
            first.raw.toByteArray(Charsets.UTF_8),
            again.raw.toByteArray(Charsets.UTF_8),
        )
        // 解析済みの側は取得時点の表示名で、こちらは変わってよい（鍵に入らない）
        assertEquals("旧い名前", first.payloadText("app_label"))
        assertEquals("新しい名前", again.payloadText("app_label"))
    }

    /**
     * **本番の取得元（`UsageStatsManager`）越しに 1 契機を回す足場**（code-verify R23）。
     *
     * 偽物（[FakeUsageSource]）は `UsageEventSnapshot` を直に返すので、
     * **本番の変換（`UsageStatsSource.snapshotOf`）を 1 度も通らない** ——
     * 実測では、種別を捨てる・欄を `null` にする 5 通りの変異を本番側に入れても
     * 単体 341 本が全部緑のままだった。ここだけが本物の `UsageEvents.Event` から
     * 記録までを通す経路で、Q1 / Q8（ふるわない・全欄）を固定する。
     */
    private fun realSourceEnv(vararg events: UsageEvents.Event): UsageTestEnv {
        val app: Application = ApplicationProvider.getApplicationContext()
        Shadows.shadowOf(app.getSystemService(UserManager::class.java)).setUserUnlocked(true)
        val stats = Shadows.shadowOf(app.getSystemService(UsageStatsManager::class.java))
        events.forEach(stats::addEvent)
        lateinit var env: UsageTestEnv
        env = UsageTestEnv(UsageStatsSource(app, now = { env.now }))
        return env
    }

    /** 取得元に溜まる 1 件（種別ごとの欄は [build] で足す）。 */
    private fun sourceEvent(
        type: Int,
        at: String,
        build: ShadowUsageStatsManager.EventBuilder.() -> Unit = {},
    ): UsageEvents.Event = ShadowUsageStatsManager.EventBuilder.buildEvent()
        .setPackage("dev.ashiato.example")
        .setClass("dev.ashiato.example.MainActivity")
        .setTimeStamp(Instant.parse(at).toEpochMilli())
        .setEventType(type)
        .apply(build)
        .build()

    // Scenario: 種別でふるい落とさない
    @Test
    fun `本番の変換を通しても種別は 1 件も落ちない`() {
        // 1 = ACTIVITY_RESUMED / 2 = ACTIVITY_PAUSED / 5 = CONFIGURATION_CHANGE /
        // 7 = USER_INTERACTION / 8 = SHORTCUT_INVOCATION / 11 = STANDBY_BUCKET_CHANGED /
        // 15 = SCREEN_INTERACTIVE / 16 = SCREEN_NON_INTERACTIVE /
        // 17 = KEYGUARD_SHOWN / 18 = KEYGUARD_HIDDEN
        val types = listOf(1, 2, 5, 7, 8, 11, 15, 16, 17, 18)
        val env = realSourceEnv(
            *types.mapIndexed { i, type ->
                sourceEvent(type, "2026-05-20T09:${(10 + i).toString().padStart(2, '0')}:00Z") {
                    if (type == 5) setConfiguration(Configuration())
                }
            }.toTypedArray(),
        )

        env.firstThenNext()

        // **1 件も落ちていない**（本人の決定 Q1）。落とした分は 10 日で取り返せない
        assertEquals(types, env.records().map { it.payloadText("event_type")?.toInt() })
    }

    // Scenario: 1 件が取得元の公開しているすべての欄を持つ
    @Test
    fun `本番の変換が種別ごとの欄まで解析済みに残す`() {
        val configuration = Configuration().apply { orientation = Configuration.ORIENTATION_LANDSCAPE }
        val env = realSourceEnv(
            sourceEvent(5, "2026-05-20T09:10:00Z") { setConfiguration(configuration) },
            sourceEvent(8, "2026-05-20T09:11:00Z") { setShortcutId("shortcut-1") },
            sourceEvent(7, "2026-05-20T09:12:00Z") {
                setExtras(
                    PersistableBundle().apply {
                        putString(UsageStatsManager.EXTRA_EVENT_ACTION, "android.intent.action.VIEW")
                        putString(UsageStatsManager.EXTRA_EVENT_CATEGORY, "android.intent.category.DEFAULT")
                    },
                )
            },
            sourceEvent(11, "2026-05-20T09:13:00Z") { setAppStandbyBucket(20) },
        )

        env.firstThenNext()

        val records = env.records()
        assertEquals(4, records.size)
        // どの種別でも共通の 4 つは落ちない
        for (record in records) {
            assertEquals("dev.ashiato.example", record.payloadText("package"))
            assertEquals("dev.ashiato.example.MainActivity", record.payloadText("class"))
            assertNotNull("event_type が無い: ${record.payload}", record.payloadText("event_type"))
            assertNotNull("event_time が無い: ${record.payload}", record.payloadText("event_time"))
        }
        // 種別ごとに付く欄（設定の変化・ショートカット・操作の中身・待機の階級）
        assertEquals(configuration.toString(), records[0].payloadText("configuration"))
        assertEquals("shortcut-1", records[1].payloadText("shortcut_id"))
        assertEquals("android.intent.action.VIEW", records[2].payloadText("interaction_action"))
        assertEquals("android.intent.category.DEFAULT", records[2].payloadText("interaction_category"))
        assertEquals("20", records[3].payloadText("standby_bucket"))
    }

    /** 名乗りと刻みは登録簿のもの（`SourceCadence`）。ここを取り違えると別のソースの数えになる。 */
    @Test
    fun `名乗りと取得の刻みはアプリ利用のもの`() {
        val env = UsageTestEnv(source = FakeUsageSource())
        assertEquals(APP_USAGE_LOGICAL_SOURCE, env.adapter().logicalSource)
        assertEquals(USAGE_INTERVAL_MS, env.adapter().intervalMs)
    }

    /** ログに出るのは件数とソース名だけ（Global Constraints / 製造準備 A-2）。 */
    @Test
    fun `ログに表示名も原文も出ない`() {
        val env = UsageTestEnv(
            source = FakeUsageSource(storedEvents = listOf(usageEvent("2026-05-20T09:10:00Z"))),
            label = "例のアプリ",
        )
        env.firstThenNext()
        val usage = env.lines.filter { it.contains("source=$APP_USAGE_LOGICAL_SOURCE") }
        assertTrue("アプリ利用のログが 1 行も無い: ${env.lines}", usage.isNotEmpty())
        assertTrue(
            "ログに私的な内容が出ている: $usage",
            env.lines.none { it.contains("例のアプリ") || it.contains("dev.ashiato.example") },
        )
    }
}
