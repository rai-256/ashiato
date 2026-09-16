// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.io.File
import java.io.IOException
import kotlinx.serialization.KSerializer
import kotlinx.serialization.SerializationException

/**
 * ST01 / ST02 の 1 本の JSONL（`outbox.jsonl` / `heartbeat.jsonl`）を、区切りの置き場へ取り込む（ST04 / design D1 / D6）。
 *
 * **初回の起動で 1 度だけ**働く。取り込めたら元を消す。取り込めなかったら元を残し、次の起動でもう一度当たる
 * （途中まで取り込んだ分は再送になるが、受け口は冪等なので行は増えない）。
 *
 * - 書き換えの途中で落ちた跡（`.tmp`）は ST01 と同じ規則で拾う
 * - **読めない行は捨てずに退避**する（ST01 の置き場は読み飛ばし、次の書き直しで消していた。C9）。
 *   退避先へ移すのは**元を消せるときだけ** —— 途中で止まって次の起動でもう一度読むと、同じ行を 2 度数える（review R36）
 * - ST01 が脇へ退けた `.unreadable.<時刻>` は、行数を報告に足せてから `salvaged/` へ移す（**消さない**。数えるのは 1 度だけ）
 * - 書けなかった記録は数えない（元のファイルがその記録の置き場。review R7）
 *
 * 1 行ずつ読み、500 件ずつまとめて書く。全件を 1 つの文字列にしない（R13）。積んだ時点の経過は取り込み全体で 1 つ（review R29）。
 */
fun <T : Outboxable> migrateLegacyOutbox(
    legacy: File,
    into: Outbox<T>,
    serializer: KSerializer<T>,
    unreadable: File,
    salvaged: File,
    /** ST01 が退けたファイルの行数を報告に足す。**足せたか**（足せなければ移さずに次の起動でもう一度数える） */
    recordSalvaged: (Int) -> Boolean,
    enqAgeMs: Long,
    log: (String) -> Unit,
): LegacyMigration {
    var imported = 0
    var broken = 0
    var complete = true

    val tmp = File(legacy.parentFile, "${legacy.name}.tmp")
    if (tmp.exists()) {
        if (legacy.exists()) tmp.delete() else if (!tmp.renameTo(legacy)) complete = false
    }

    if (legacy.exists()) {
        val pending = File(legacy.parentFile, "${legacy.name}.unreadable-pending")
        pending.delete()
        try {
            val batch = ArrayList<T>(BATCH)
            fun flush(): Boolean {
                if (batch.isEmpty()) return true
                val ok = into.importLegacy(batch, enqAgeMs)
                if (ok) imported += batch.size
                batch.clear()
                return ok
            }
            legacy.bufferedReader(Charsets.UTF_8).useLines { lines ->
                for (line in lines) {
                    if (line.isBlank()) continue
                    val item = try {
                        ingestJson.decodeFromString(serializer, line)
                    } catch (e: SerializationException) {
                        null
                    } catch (e: IllegalArgumentException) {
                        null
                    }
                    if (item == null) {
                        pending.appendText(line + "\n", Charsets.UTF_8)
                        broken++
                        continue
                    }
                    batch += item
                    if (batch.size >= BATCH && !flush()) {
                        complete = false
                        break
                    }
                }
            }
            if (complete && !flush()) complete = false
        } catch (e: IOException) {
            log(Telemetry.line("legacy_outbox_unreadable", error = e.javaClass.simpleName))
            complete = false
        }
        if (complete && pending.exists()) {
            // 読めない行を退避先へ移す。**移せなかったら元を消さない**（その行が端末から消える）
            complete = try {
                unreadable.parentFile?.mkdirs()
                unreadable.appendBytes(pending.readBytes())
                pending.delete()
                true
            } catch (e: IOException) {
                log(Telemetry.line("legacy_salvage_failed", error = e.javaClass.simpleName))
                false
            }
        }
        if (!complete) {
            pending.delete()
            broken = 0
        }
        if (complete && !legacy.delete()) complete = false
        log(Telemetry.line("legacy_outbox_imported", count = imported))
    }

    // ST01 が脇へ退けたファイル。**消さずに移し、行数を 1 度だけ数える**
    val aside = legacy.parentFile?.listFiles { f -> f.name.startsWith("${legacy.name}.unreadable.") }.orEmpty()
    for (f in aside) {
        val n = try {
            f.bufferedReader(Charsets.UTF_8).useLines { seq -> seq.count { it.isNotBlank() } }
        } catch (e: IOException) {
            log(Telemetry.line("legacy_salvage_unreadable", error = e.javaClass.simpleName))
            continue
        }
        if (n > 0 && !recordSalvaged(n)) continue
        salvaged.mkdirs()
        if (!f.renameTo(File(salvaged, f.name))) log(Telemetry.line("legacy_salvage_move_failed"))
        broken += n
    }
    if (broken > 0) log(Telemetry.line("legacy_outbox_broken", count = broken))
    return LegacyMigration(imported, broken, complete)
}

/** 取り込んだ件数・読めなかった行の数・元を消せたか。 */
data class LegacyMigration(val imported: Int, val unreadable: Int, val complete: Boolean)

private const val BATCH = 500
