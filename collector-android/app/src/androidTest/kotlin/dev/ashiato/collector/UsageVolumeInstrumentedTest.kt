// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import java.time.Instant
import java.time.ZoneId
import java.util.UUID
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

/** `tools/usage-volume.sh` が実際の UsageStatsManager から直近 1 時間を採寸するための口。 */
@RunWith(AndroidJUnit4::class)
class UsageVolumeInstrumentedTest {
    @Test
    fun writeOneHourOfBufferedUsageEvents() {
        val context = ApplicationProvider.getApplicationContext<android.content.Context>()
        val end = Instant.now()
        val result = UsageStatsSource(context).events(CollectionWindow(end.minusSeconds(3_600), end))
        val events = (result as? EventsResult.Events)?.events.orEmpty()
        assertTrue("直近 1 時間のアプリ利用イベントが 0 件", events.isNotEmpty())

        val zone = ZoneId.systemDefault()
        val enq = android.os.SystemClock.elapsedRealtime()
        val lines = events.joinToString(separator = "", transform = { event ->
            val label = event.packageName?.let { packageName ->
                runCatching {
                    val info = context.packageManager.getApplicationInfo(packageName, 0)
                    context.packageManager.getApplicationLabel(info).toString()
                }.getOrNull()
            }
            val request = event.toIngestRequest(
                UUID.randomUUID().toString(),
                "00000000-0000-0000-0000-000000000000",
                "usage-volume-emulator",
                zone,
                label,
            )
            "{\"enq\":$enq,\"item\":${ingestJson.encodeToString(IngestRequest.serializer(), request)}}\n"
        })
        context.filesDir.resolve("usage-volume.jsonl").writeText(lines)
    }
}
