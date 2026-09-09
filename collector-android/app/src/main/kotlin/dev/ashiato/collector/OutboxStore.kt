// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.io.File
import java.io.IOException
import kotlinx.serialization.SerializationException

/**
 * 未送信の置き場の**永続化先**。Android のファイルと試験用の偽物を差し替えられるようにしてある。
 *
 * **既定を持たせない。** 持たせるとインスタンスの中だけに積む実装が黙って選ばれ、
 * プロセスが立て直されたときに未送信が消える —— それがこの型を足した理由そのもの。
 */
interface OutboxStore {
    fun load(): List<IngestRequest>

    /** 書けたか。**呼び出し側が知れる形にする**（design D22）—— Unit だと失敗が上に届かない。 */
    fun save(requests: List<IngestRequest>): Boolean

    /**
     * 1 件を足す。**既定は全件の書き直し**で、追記できる置き場だけが上書きする。
     *
     * `after` を関数で受けるのは、追記できる置き場が全件を組み立てずに済ませるため ——
     * 未送信が伸びたときに、書かないリストを毎分作るのが無駄になる。
     */
    fun append(request: IngestRequest, after: () -> List<IngestRequest>): Boolean = save(after())
}

/**
 * 未送信を端末の保存領域に置く（深掘り 第 2 回 / specs「未送信を、収集の停止と再開をまたいで保持する」）。
 *
 * **これが無いと、`START_STICKY` でプロセスが立て直されるたびに最大 5 分ぶんが無言で消える。**
 * この Story は「捨てたものは復元できない」を根拠に精度フィルタを外している（design D11）のに、
 * 同じ理由で失われる経路が残っていた。
 *
 * 形は **JSONL（1 行 1 件）**（design D22）。全件を書き直す形だと、圏外が続いて未送信が
 * 伸びたときに 60 秒ごとの全書き直しがフラッシュ寿命と電池に効く（実測見積り: 1 日圏外で約 0.6 GB）。
 * 追記なら `add` は 1 件ぶんで済む。まとめて書き直すのは送信後の `remove` のときだけ。
 */
class FileOutboxStore(
    private val file: File,
    /** **既定を持たせない。** 既定があると失敗の報告が黙って捨てられる（review R7 / HIGH-7）。 */
    private val log: (String) -> Unit,
) : OutboxStore {
    private val tmp = File(file.parentFile, "${file.name}.tmp")

    override fun load(): List<IngestRequest> {
        // **書き換えの途中で落ちた跡を先に拾う。** `.tmp` には最新の全件が入っている
        // 可能性があり、見ないまま次の save で上書きすると、そのぶんが無言で消える（review CRITICAL-2）。
        recoverInterrupted()
        if (!file.exists()) return emptyList()
        val text = try {
            file.readText()
        } catch (e: IOException) {
            // **読めなかっただけで捨てない。** 一過性の失敗でも退けると、正常な未送信が消える
            log(Telemetry.line("outbox_read_failed", error = e.javaClass.simpleName))
            return emptyList()
        }
        return parse(text) ?: emptyList<IngestRequest>().also { salvage("unparseable") }
    }

    /**
     * JSONL を 1 行ずつ読む。**1 行壊れていても残りを捨てない** ——
     * 追記の途中で電源が落ちると最後の 1 行だけが半端になる。
     * 全部読めなければ null（呼び出し側が退避する）。
     */
    private fun parse(text: String): List<IngestRequest>? {
        val lines = text.lineSequence().filter { it.isNotBlank() }.toList()
        val out = ArrayList<IngestRequest>(lines.size)
        var broken = 0
        for (line in lines) {
            try {
                out += ingestJson.decodeFromString<IngestRequest>(line)
            } catch (e: SerializationException) {
                broken++
                log(Telemetry.line("outbox_line_broken", count = broken, error = e.javaClass.simpleName))
            }
        }
        // 1 件も読めず、かつ中身があった → 形そのものが違う（旧い版など）。退避に回す
        if (out.isEmpty() && lines.isNotEmpty()) return null
        return out
    }

    /** `.tmp` が残っていて本体が無いなら、書き換えの途中で落ちている。`.tmp` を採る。 */
    private fun recoverInterrupted() {
        if (!tmp.exists()) return
        if (file.exists()) {
            // 本体が在るなら `.tmp` は差し替え前の書きかけ。捨ててよい（本体のほうが確定している）
            tmp.delete()
            return
        }
        if (tmp.renameTo(file)) {
            log(Telemetry.line("outbox_recovered"))
        } else {
            log(Telemetry.line("outbox_recover_failed", error = "rename"))
        }
    }

    /**
     * 読めなかったものを**捨てずに脇へ退ける**。上書きして消すと、
     * 何が失われたのか後から誰にも分からない（FR-18 と同じ理由）。
     *
     * **退避先の名前に時刻を入れる。** 固定名だと 2 回目の破損が 1 回目の退避を
     * 無言で上書きする（`File.renameTo` は Unix で置き換える。review R4 で実測）。
     */
    private fun salvage(kind: String) {
        var aside = File(file.parentFile, "${file.name}.unreadable.${System.currentTimeMillis()}")
        var n = 0
        while (aside.exists() && n < 100) {
            aside = File(file.parentFile, "${file.name}.unreadable.${System.currentTimeMillis()}.${++n}")
        }
        if (aside.exists()) {
            // 名前を空けられなかった。**上書きしない** —— 消すより残すほうがまし
            log(Telemetry.line("outbox_salvage_no_name", error = kind))
            return
        }
        val moved = file.renameTo(aside)
        log(Telemetry.line(if (moved) "outbox_unreadable" else "outbox_salvage_failed", error = kind))
    }

    /** 1 件を追記する。**全件を書き直さない**（design D22）。 */
    override fun append(request: IngestRequest, after: () -> List<IngestRequest>): Boolean = try {
        file.appendText(ingestJson.encodeToString(request) + "\n")
        true
    } catch (e: IOException) {
        log(Telemetry.line("outbox_append_failed", error = e.javaClass.simpleName))
        false
    }

    override fun save(requests: List<IngestRequest>): Boolean {
        // 一時ファイルへ書いてから差し替える。途中で落ちても半端なファイルが残らない。
        // **電源断には対して原子的ではない**（fsync していない）—— rename 後もデータが
        // ページキャッシュにある窓が残る。プロセス死には効く。
        return try {
            tmp.writeText(requests.joinToString("") { ingestJson.encodeToString(it) + "\n" })
            if (tmp.renameTo(file)) {
                true
            } else {
                // **黙って諦めない。** `.tmp` は次の load が拾う（recoverInterrupted）
                log(Telemetry.line("outbox_save_failed", count = requests.size, error = "rename"))
                false
            }
        } catch (e: IOException) {
            log(Telemetry.line("outbox_save_failed", count = requests.size, error = e.javaClass.simpleName))
            false
        } catch (e: SerializationException) {
            // **位置取得の糸へ投げ返さない。** 投げると LocationCallback を貫通して収集が止まる
            log(Telemetry.line("outbox_save_failed", count = requests.size, error = e.javaClass.simpleName))
            false
        }
    }
}
