// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import java.time.Instant
import java.time.ZoneId
import java.util.UUID
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

/**
 * `tools/usage-volume.sh` が実際の UsageStatsManager から直近の窓を採寸するための口。
 *
 * 窓の長さは計測の引数 `windowSeconds`（既定 3600 = エミュレータで流し込んだ 1 時間）。
 * 実機の経路（`--device`）は 10 日（864000）を渡し、**取得元が既に持っている本人の実イベント**を数える
 * （deep.md の Q10。流し込みの回数を選ばない）。
 *
 * 書くもの（アプリの `files/` の中）:
 * - `usage-volume.jsonl` —— 未送信の置き場と同じ形の 1 行 1 件（バイト数を実物で測るため）
 * - `usage-volume.meta` —— `window_seconds=` と `span_seconds=`（窓の終わりから最も古いイベントまで。
 *   端末が 10 日を持っていないとき、1 日あたりの件数を実際に観測できた長さで割るため）
 *
 * **1 行ずつ書く。** 10 日ぶんは数万〜十数万行になり、全部を 1 本の文字列にするとアプリのヒープを越えうる。
 */
@RunWith(AndroidJUnit4::class)
class UsageVolumeInstrumentedTest {
    @Test
    fun writeOneHourOfBufferedUsageEvents() {
        val context = ApplicationProvider.getApplicationContext<android.content.Context>()
        val windowSeconds = InstrumentationRegistry.getArguments().getString("windowSeconds")?.toLong() ?: 3_600L
        val end = Instant.now()
        val result = UsageStatsSource(context).events(CollectionWindow(end.minusSeconds(windowSeconds), end))
        val events = (result as? EventsResult.Events)?.events.orEmpty()
        assertTrue("直近 $windowSeconds 秒のアプリ利用イベントが 0 件（$result）", events.isNotEmpty())

        val zone = ZoneId.systemDefault()
        val enq = android.os.SystemClock.elapsedRealtime()
        val labels = HashMap<String, String?>()
        context.filesDir.resolve("usage-volume.jsonl").bufferedWriter().use { out ->
            for (event in events) {
                val label = event.packageName?.let { packageName ->
                    labels.getOrPut(packageName) {
                        runCatching {
                            val info = context.packageManager.getApplicationInfo(packageName, 0)
                            context.packageManager.getApplicationLabel(info).toString()
                        }.getOrNull()
                    }
                }
                val request = event.toIngestRequest(
                    UUID.randomUUID().toString(),
                    "00000000-0000-0000-0000-000000000000",
                    "usage-volume-emulator",
                    zone,
                    label,
                )
                out.write("{\"enq\":$enq,\"item\":${ingestJson.encodeToString(IngestRequest.serializer(), request)}}\n")
            }
        }
        val oldest = events.minOf { it.at }
        val span = maxOf(1L, end.epochSecond - oldest.epochSecond)
        context.filesDir.resolve("usage-volume.meta").writeText("window_seconds=$windowSeconds\nspan_seconds=$span\n")
    }
}
