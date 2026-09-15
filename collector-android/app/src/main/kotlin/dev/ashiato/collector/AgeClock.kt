// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.content.Context
import android.os.SystemClock
import android.provider.Settings
import java.io.File
import java.io.IOException

/** 端末の 3 つの時計。**試験だけが差し替える。** */
interface DeviceClock {
    /** 壁時計（エポックからのミリ秒）。本人や網が動かしうる */
    fun wallMs(): Long

    /** 起動からの単調な経過（ミリ秒）。眠っている間も進み、起動をまたぐと 0 に戻る */
    fun monoMs(): Long

    /** 起動の回数。取れない端末では null（そのときは単調時計が戻ったことで起動を知る） */
    fun bootCount(): Int?
}

class AndroidDeviceClock(private val context: Context) : DeviceClock {
    override fun wallMs(): Long = System.currentTimeMillis()

    override fun monoMs(): Long = SystemClock.elapsedRealtime()

    override fun bootCount(): Int? =
        runCatching { Settings.Global.getInt(context.contentResolver, Settings.Global.BOOT_COUNT) }.getOrNull()
}

/**
 * **積んでからの経過**を数える時計（ST04 / 深掘り Q4 / C10 / design D2）。
 *
 * 値は「端末の保持が始まってからの、時計の飛びに影響されない経過（ミリ秒）」で、**減らない**。
 * 1 件の「積んでからの経過」は `now() - 積んだときの now()`。
 *
 * - **同じ起動のあいだは単調時計の差で進む** —— 壁時計が 120 日先へ飛んでも 90 日の側では捨てない（C10）
 * - **起動をまたぐときは、止まっていた間の壁時計の差で進む**。ただし **30 日まで**（30 日の値は仮。design D2）、
 *   **負（時計が戻った）なら 0**。単調時計は起動をまたいで比べられないので、長い空白は時計の飛びとして扱う
 *
 * 2 GB の上限はこの時計と無関係にかかるので、時計がどう狂っても端末の空きは守られる。
 *
 * **見回りのたびに「最後に見た 壁時計・単調時計・起動回数・経過」を小さなファイルに書く**（design D2）。
 * 書けなくても落とさない —— 前回書けた値から数えるので、経過が少なく見積もられる（捨てない側）だけ。
 */
class AgeClock(
    private val device: DeviceClock,
    private val file: File,
    private val log: (String) -> Unit,
    /** 起動をまたぐ空白を数える上限（design D2（仮））。 */
    private val maxGapMs: Long = MAX_REBOOT_GAP_MS,
) {
    private data class Seen(val ageMs: Long, val monoMs: Long, val wallMs: Long, val boot: Int?)

    private var last: Seen = load() ?: Seen(0, device.monoMs(), device.wallMs(), device.bootCount())
        .also { save(it) }

    /** いまの経過。**呼ぶたびに最後に見た値を進めて書く。** */
    @Synchronized
    fun now(): Long {
        val mono = device.monoMs()
        val wall = device.wallMs()
        val boot = device.bootCount()
        val sameBoot = mono >= last.monoMs && (boot == null || last.boot == null || boot == last.boot)
        val advance = if (sameBoot) {
            val monoDelta = mono - last.monoMs
            // 同じ起動のあいだに壁時計と単調時計が 1 時間を超えて食い違ったら、時計の飛びを残す（件数だけ）
            if (kotlin.math.abs((wall - last.wallMs) - monoDelta) > CLOCK_JUMP_LOG_MS) {
                log(Telemetry.line("clock_jump"))
            }
            monoDelta
        } else {
            // 起動をまたいだ。**止まっていた間の壁時計の差を 30 日まで数え、戻っていたら 0**
            (wall - last.wallMs).coerceIn(0, maxGapMs)
        }
        last = Seen(last.ageMs + advance, mono, wall, boot)
        save(last)
        return last.ageMs
    }

    private fun load(): Seen? = try {
        if (!file.exists()) {
            null
        } else {
            val p = file.readText().trim().split(" ")
            Seen(p[0].toLong(), p[1].toLong(), p[2].toLong(), p.getOrNull(3)?.takeIf { it != "-" }?.toInt())
        }
    } catch (e: IOException) {
        log(Telemetry.line("age_clock_unreadable", error = e.javaClass.simpleName))
        null
    } catch (e: RuntimeException) {
        log(Telemetry.line("age_clock_unreadable", error = e.javaClass.simpleName))
        null
    }

    private fun save(s: Seen) {
        try {
            file.parentFile?.mkdirs()
            // **書いてから差し替える**（review R37）。途中で落ちて壊れると経過が 0 に戻り、90 日の上限と知らせが止まる
            val tmp = File(file.parentFile, "${file.name}.tmp")
            tmp.writeText("${s.ageMs} ${s.monoMs} ${s.wallMs} ${s.boot ?: "-"}")
            if (!tmp.renameTo(file)) throw IOException("rename")
        } catch (e: IOException) {
            log(Telemetry.line("age_clock_save_failed", error = e.javaClass.simpleName))
        }
    }

    companion object {
        const val DAY_MS: Long = 24 * 60 * 60 * 1000L

        /** 起動をまたぐ空白を数える上限（design D2（仮）。反転条件は design）。 */
        const val MAX_REBOOT_GAP_MS: Long = 30 * DAY_MS

        private const val CLOCK_JUMP_LOG_MS: Long = 60 * 60 * 1000L
    }
}
