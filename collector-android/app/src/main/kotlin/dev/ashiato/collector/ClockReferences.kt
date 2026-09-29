// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.os.Build
import android.os.SystemClock
import java.time.DateTimeException
import java.time.ZonedDateTime
import java.time.format.DateTimeFormatter

/**
 * OS の時計の読み取り（ST05 / design D2）。**試験だけが差し替える。**
 * 実装は通信を起こさない —— OS が持っている値を読むだけ。
 */
interface SystemTimeSources {
    /** OS の版（`Build.VERSION.SDK_INT`）。基準の口が無い版を「いま取れない」と区別するのに使う */
    val sdkInt: Int

    /** ネットワーク時刻（エポックからのミリ秒）。**取れないとき `DateTimeException`** */
    fun networkMs(): Long

    /** 衛星の時刻（エポックからのミリ秒）。**取れないとき `DateTimeException`** */
    fun gnssMs(): Long

    /** 端末の壁時計 */
    fun wallMs(): Long

    /** 起動からの単調な経過 */
    fun monoMs(): Long
}

class AndroidSystemTimeSources : SystemTimeSources {
    override val sdkInt: Int get() = Build.VERSION.SDK_INT

    override fun networkMs(): Long = SystemClock.currentNetworkTimeClock().millis()

    override fun gnssMs(): Long = SystemClock.currentGnssTimeClock().millis()

    override fun wallMs(): Long = System.currentTimeMillis()

    override fun monoMs(): Long = SystemClock.elapsedRealtime()
}

/** 1 つの基準を読んだ結果。取れなければ `timeMs` と `skewMs` が null で `reason` がある。 */
data class ClockReading(
    val source: String,
    val timeMs: Long?,
    /** 端末の壁時計 − 基準の時刻 */
    val skewMs: Long?,
    val monoBeforeMs: Long,
    val monoAfterMs: Long,
    val reason: String?,
)

/**
 * 端末の時計の基準（`network` / `gnss` / `s01-date`）を読む（ST05 / design D2）。
 *
 * - **差に使う壁時計は、その基準を読む直前と直後の間で読む。** 測定の先頭で読んだ壁時計を使い回さない
 * - **例外を外へ出さない。** 失敗はその基準の `reason` に種別で残る（`error:<型名>`）
 * - **通信を起こさない。** `s01-date` は送信がすでに受け取った応答の日付を取り出すだけ
 */
class ClockReferences(
    private val sources: SystemTimeSources,
    private val responseDates: ResponseDateCache,
) {
    fun readAll(): List<ClockReading> = listOf(network(), gnss(), s01Date())

    private fun network(): ClockReading = live(SOURCE_NETWORK, minSdk = 33) { sources.networkMs() }

    private fun gnss(): ClockReading = live(SOURCE_GNSS, minSdk = 34) { sources.gnssMs() }

    /** OS の口を 1 回読む。単調時計は呼び出しの前後、壁時計はその間で読む。 */
    private fun live(source: String, minSdk: Int, read: () -> Long): ClockReading {
        val before = safeMono()
        if (sources.sdkInt < minSdk) return ClockReading(source, null, null, before, before, REASON_UNSUPPORTED)
        return try {
            val time = read()
            val wall = sources.wallMs()
            val after = sources.monoMs()
            ClockReading(source, time, wall - time, before, after, null)
        } catch (e: DateTimeException) {
            ClockReading(source, null, null, before, safeMono(), REASON_NOT_AVAILABLE)
        } catch (e: Exception) {
            ClockReading(source, null, null, before, safeMono(), "error:${e.javaClass.simpleName}")
        }
    }

    private fun s01Date(): ClockReading {
        val now = safeMono()
        return when (val taken = responseDates.take()) {
            is ResponseDateCache.Taken.Missing -> ClockReading(SOURCE_S01_DATE, null, null, now, now, taken.reason)
            is ResponseDateCache.Taken.Got -> {
                val r = taken.received
                val time = r.date?.let(::parseHttpDate)
                if (time == null) {
                    ClockReading(SOURCE_S01_DATE, null, null, r.monoBeforeMs, r.monoAfterMs, REASON_UNREADABLE)
                } else {
                    ClockReading(SOURCE_S01_DATE, time, r.wallAfterMs - time, r.monoBeforeMs, r.monoAfterMs, null)
                }
            }
        }
    }

    private fun safeMono(): Long = try {
        sources.monoMs()
    } catch (e: Exception) {
        0
    }

    private fun parseHttpDate(text: String): Long? = try {
        ZonedDateTime.parse(text.trim(), DateTimeFormatter.RFC_1123_DATE_TIME).toInstant().toEpochMilli()
    } catch (e: DateTimeException) {
        null
    }

    companion object {
        const val SOURCE_NETWORK = "network"
        const val SOURCE_GNSS = "gnss"
        const val SOURCE_S01_DATE = "s01-date"

        /** OS の版がその基準の口を持たない（いま取れないのとは別） */
        const val REASON_UNSUPPORTED = "unsupported"
        const val REASON_NOT_AVAILABLE = "not_available"
        const val REASON_UNREADABLE = "unreadable"
    }
}
