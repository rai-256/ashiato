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
 * - **読めない行は捨てずに退避**して件数を返す（ST01 の置き場は読み飛ばし、次の書き直しで消していた。C9）
 * - ST01 が脇へ退けた `.unreadable.<時刻>` は、行数を数えて `salvaged/` へ移す（**消さない**。数えるのは 1 度だけ）
 *
 * 1 行ずつ読む。全件を 1 つの文字列にしない（R13）。
 */
fun <T : Outboxable> migrateLegacyOutbox(
    legacy: File,
    into: Outbox<T>,
    serializer: KSerializer<T>,
    unreadable: File,
    salvaged: File,
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
        try {
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
                        unreadable.parentFile?.mkdirs()
                        unreadable.appendText(line + "\n", Charsets.UTF_8)
                        broken++
                    } else if (into.add(item)) {
                        imported++
                    } else {
                        // 書けなかった（メモリには積まれた）。元は消さない
                        complete = false
                    }
                }
            }
        } catch (e: IOException) {
            log(Telemetry.line("legacy_outbox_unreadable", error = e.javaClass.simpleName))
            complete = false
        }
        if (complete && !legacy.delete()) complete = false
        log(Telemetry.line("legacy_outbox_imported", count = imported))
    }

    // ST01 が脇へ退けたファイル。**消さずに移し、行数を 1 度だけ数える**
    val aside = legacy.parentFile?.listFiles { f -> f.name.startsWith("${legacy.name}.unreadable") }.orEmpty()
    for (f in aside) {
        val n = try {
            f.bufferedReader(Charsets.UTF_8).useLines { seq -> seq.count { it.isNotBlank() } }
        } catch (e: IOException) {
            log(Telemetry.line("legacy_salvage_unreadable", error = e.javaClass.simpleName))
            continue
        }
        salvaged.mkdirs()
        if (f.renameTo(File(salvaged, f.name))) {
            broken += n
        } else {
            log(Telemetry.line("legacy_salvage_move_failed"))
        }
    }
    if (broken > 0) log(Telemetry.line("legacy_outbox_broken", count = broken))
    return LegacyMigration(imported, broken, complete)
}

/** 取り込んだ件数・読めなかった行の数・元を消せたか。 */
data class LegacyMigration(val imported: Int, val unreadable: Int, val complete: Boolean)
