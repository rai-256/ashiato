// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonPrimitive
import java.io.File
import java.io.IOException
import java.time.Duration
import java.time.Instant
import java.util.UUID

/** 原本の送り待ちの台帳の名前（`media/` の下。design D8）。 */
const val ORIGINALS_FILE: String = "originals.jsonl"

/** 収集がその項目を見つけた時刻より、保存の時刻がこれ以上前なら過去ぶん（design D8（仮）/ Q9）。 */
val BACKLOG_AGE: Duration = Duration.ofHours(BACKLOG_HOURS)

private const val BACKLOG_HOURS = 24L

/** 記録の送り待ちの印: 記録がまだ受理されていない */
const val ORIGINAL_AWAITING_RECORD: String = "awaiting_record"

/** 記録が受理された（重複を含む）。原本を送ってよい */
const val ORIGINAL_READY: String = "ready"

/**
 * 原本の送り待ちの 1 行（design D8）。1 行は数百バイトで、**未送信の置き場にも 90 日・2 GB にも入らない**（Q4）。
 *
 * D8 の列に [recordId]（送った記録の識別子）・[dateAdded]・[missingSince] を足してある。
 * 記録が破棄された・断られたときに「どの行か」を引くのと、過去ぶんを保存の新しい順に並べるのと、
 * 最初の 409 から 7 日を数えるのに要る。
 */
@Serializable
data class OriginalEntry(
    val sha256: String,
    val volume: String,
    @SerialName("media_id") val mediaId: Long,
    val source: String,
    val size: Long,
    val backlog: Boolean,
    /** `DATE_ADDED`（秒）。過去ぶんどうしの並び */
    @SerialName("date_added") val dateAdded: Long,
    /** 積んだ時刻（エポックミリ秒）。新しいものの並び */
    @SerialName("queued_at") val queuedAt: Long,
    val state: String,
    @SerialName("record_id") val recordId: String,
    /** 最初に `409 record_missing` と返された時刻（エポックミリ秒）。返されていなければ null */
    @SerialName("missing_since") val missingSince: Long? = null,
)

/**
 * 原本の送り待ちの台帳（`media/originals.jsonl`。design D8）。
 *
 * - 画像の記録を未送信に積んだら `awaiting_record`、送信の結果で受理（重複を含む）と分かったら `ready`
 * - **`Retention` の対象にしない** —— 記録の置き場とは別のファイルで、保持の上限をかけるコードがここを見ない
 * - 書くときは全体を書いてから差し替える（途中で落ちても前の版が残る）。書けなければメモリの変更を戻す
 * - 読めない行は捨てて続ける（失うのは 1 行の送り待ちで、写真そのものは端末にある）
 *
 * 1 つの内容ハッシュに 1 行（同じ中身の別名は記録が 1 件のままなので、行も 1 つ）。
 */
class OriginalLedger(private val file: File, private val log: (String) -> Unit = {}) {
    private val json = Json {
        ignoreUnknownKeys = true
        encodeDefaults = true
    }
    private val rows = LinkedHashMap<String, OriginalEntry>()

    init {
        load()
    }

    private fun load() {
        if (!file.exists()) return
        try {
            file.forEachLine { line ->
                if (line.isBlank()) return@forEachLine
                try {
                    val entry = json.decodeFromString(OriginalEntry.serializer(), line)
                    rows[entry.sha256] = entry
                } catch (_: IllegalArgumentException) {
                    log(Telemetry.line("original_ledger_line_unreadable", source = null))
                }
            }
        } catch (e: IOException) {
            log(Telemetry.line("original_ledger_unreadable", source = null, error = e.javaClass.simpleName))
        }
    }

    /** 全部の行（試験と診断のため）。 */
    @Synchronized
    fun all(): List<OriginalEntry> = rows.values.toList()

    @Synchronized
    fun get(sha256: String): OriginalEntry? = rows[sha256]

    /** 送ってよい行（記録が受理された）。順は呼び手（送り手）が決める。 */
    @Synchronized
    fun ready(): List<OriginalEntry> = rows.values.filter { it.state == ORIGINAL_READY }

    /**
     * 画像の記録を未送信に積んだ後に、原本の送り待ちを足す。保存できたか。
     * 同じ内容ハッシュの行がすでにあれば、新しい記録に付け替えるだけ（記録だけを送り直す更新。原本は 1 つ）。
     */
    @Synchronized
    fun queue(entry: OriginalEntry): Boolean = change {
        val existing = rows[entry.sha256]
        when {
            existing == null -> rows[entry.sha256] = entry
            existing.state == ORIGINAL_AWAITING_RECORD -> rows[entry.sha256] = existing.copy(recordId = entry.recordId)
            else -> Unit
        }
    }

    /** 受理された記録の原本を、送ってよい状態にする。保存できたか。 */
    @Synchronized
    fun markReady(shas: Collection<String>): Boolean = change {
        for (sha in shas) {
            val row = rows[sha]
            if (row?.state == ORIGINAL_AWAITING_RECORD) rows[sha] = row.copy(state = ORIGINAL_READY)
        }
    }

    /** 最初の 409 の時刻を残す（すでにあれば変えない）。 */
    @Synchronized
    fun markMissingSince(sha256: String, atMs: Long): Boolean = change {
        rows[sha256]?.takeIf { it.missingSince == null }?.let { rows[sha256] = it.copy(missingSince = atMs) }
    }

    /** 行を消す。消せたか（行が無ければ真）。 */
    @Synchronized
    fun removeAll(shas: Collection<String>): Boolean = change { shas.forEach { rows.remove(it) } }

    /**
     * 変更を適用して保存する。保存できなければ変更を戻す（メモリと台帳を食い違わせない）。
     * 送り手はこの戻りで「次に何をするか」を決める。
     */
    private fun change(apply: () -> Unit): Boolean {
        val before = LinkedHashMap(rows)
        apply()
        val saved = before == rows || write()
        if (!saved) {
            rows.clear()
            rows.putAll(before)
        }
        return saved
    }

    private fun write(): Boolean = try {
        file.parentFile?.mkdirs()
        val tmp = File(file.parentFile, "${file.name}.tmp")
        tmp.writeText(rows.values.joinToString("") { json.encodeToString(OriginalEntry.serializer(), it) + "\n" })
        if (!tmp.renameTo(file)) throw IOException("rename")
        true
    } catch (e: IOException) {
        log(Telemetry.line("original_ledger_write_failed", source = null, error = e.javaClass.simpleName))
        false
    }
}

/** 原本の欠けの理由（`crates/server` の `POST /media/missing` と同じ語。design D11）。 */
enum class OriginalMissingReason(val wire: String) {
    VANISHED("vanished"),
    CHANGED("changed"),
    UNREADABLE("unreadable"),
    TOO_LARGE("too_large"),
    RECORD_DROPPED("record_dropped"),
    RECORD_REJECTED("record_rejected"),
    RECORD_MISSING("record_missing"),
}

/**
 * 原本の欠けの報告 1 件（design D11）。**`DropReport` と同じく未送信の置き場に乗せて送る**が、
 * 90 日・2 GB の保持の対象になる記録の置き場とは別の置き場に置く（R6。生存信号・破棄の報告と同じ扱い）。
 * 位置・ファイル名・原文を持たない（内容ハッシュと理由だけ）。
 */
@Serializable
data class MediaMissingReport(
    override val id: String,
    @SerialName("user_id") val userId: String,
    @SerialName("device_id") val deviceId: String,
    @SerialName("logical_source") override val logicalSource: String,
    val sha256: String,
    val reason: String,
    @SerialName("noticed_at") val noticedAt: String,
) : Outboxable

/** 原本の欠けを報告の置き場に積む。 */
class OriginalMissingReporter(
    private val outbox: Outbox<MediaMissingReport>,
    private val userId: () -> String,
    private val deviceId: String,
    private val now: () -> Instant,
    private val newId: () -> String,
    private val log: (String) -> Unit = {},
) {
    /**
     * 積む。積めなくても `Outbox.add` はメモリに持つので、呼び手は台帳の行を消してよい
     * （積めなかったことはログに種別だけ残す）。
     */
    fun report(entry: OriginalEntry, reason: OriginalMissingReason) {
        val report = MediaMissingReport(
            id = newId(),
            userId = userId(),
            deviceId = deviceId,
            logicalSource = entry.source,
            sha256 = entry.sha256,
            reason = reason.wire,
            noticedAt = now().toString(),
        )
        if (!outbox.add(report)) log(Telemetry.line("original_missing_not_persisted", source = entry.source, count = 1))
    }
}

/** 記録の送信の結果を受ける口（`Sender`）。 */
interface SendListener<T> {
    /** 受理された（重複を含む）項目。未送信から取り除く前に呼ぶ */
    fun accepted(items: List<T>)

    /** 恒久的に断られて未送信から取り除く項目 */
    fun rejectedForGood(items: List<T>)
}

/**
 * 画像の記録と原本の送り待ちの結びつき（design D8）。
 *
 * - 記録が受理された → `ready`
 * - 記録が未送信の上限で破棄された（`Retention`）→ 行を消して `record_dropped`
 * - 記録が恒久的に断られた（`Sender`）→ 行を消して `record_rejected`
 *
 * 動画と、画像でも記録の種類が違うもの（消えた事実）は行を持たないので何もしない。
 * 破棄・拒否は**行がその記録に付いているとき**だけ（記録だけを送り直した新しい版が生きていれば、行はそちらに付け替わっている）。
 */
class OriginalRecordLink(
    private val ledger: OriginalLedger,
    private val reporter: OriginalMissingReporter,
) : SendListener<IngestRequest> {
    override fun accepted(items: List<IngestRequest>) {
        val shas = items.mapNotNull(::imageSha)
        if (shas.isNotEmpty()) ledger.markReady(shas)
    }

    override fun rejectedForGood(items: List<IngestRequest>) = gone(items, OriginalMissingReason.RECORD_REJECTED)

    /** `Retention` が破棄した記録 */
    fun dropped(items: List<IngestRequest>) = gone(items, OriginalMissingReason.RECORD_DROPPED)

    private fun gone(items: List<IngestRequest>, reason: OriginalMissingReason) {
        val rows = items.mapNotNull { item ->
            imageSha(item)?.let(ledger::get)?.takeIf { it.state == ORIGINAL_AWAITING_RECORD && it.recordId == item.id }
        }
        rows.forEach { reporter.report(it, reason) }
        if (rows.isNotEmpty()) ledger.removeAll(rows.map { it.sha256 })
    }

    /** 画像の記録の内容ハッシュ。動画・消えた事実など、原本を持たない記録は null */
    private fun imageSha(item: IngestRequest): String? {
        val isMedia = item.payload["kind"]?.jsonPrimitive?.content == "media"
        val isImageRecord = isMedia && item.logicalSource != MEDIA_VIDEO_SOURCE
        return if (isImageRecord) item.payload["sha256"]?.jsonPrimitive?.content else null
    }
}

/**
 * 原本の送りに要る部品の組み立て（`LocationService` に置くと大きくなりすぎるので外に出した）。
 * 原本の欠けの報告の置き場は、記録の置き場とは別（上限の対象にしない。R6 / design D11）。
 */
class OriginalPipeline(
    val originals: OriginalLedger,
    val missingOutbox: Outbox<MediaMissingReport>,
    userId: () -> String,
    deviceId: String,
    log: (String) -> Unit,
) {
    val reporter = OriginalMissingReporter(
        missingOutbox,
        userId,
        deviceId,
        now = { Instant.now() },
        newId = { UUID.randomUUID().toString() },
        log = log,
    )

    /** 記録の送信の結果（受理・恒久的な拒否）と、記録の破棄（`Retention`）を原本の行に結ぶ */
    val link = OriginalRecordLink(originals, reporter)

    /** 原本の欠けの報告の送り手。断られても取り除かない（破棄の報告と同じ。生存信号と同じ扱い） */
    fun missingSender(
        transport: Transport,
        dates: ResponseDateCache?,
        log: (String) -> Unit,
    ): Sender<MediaMissingReport> =
        Sender(
            missingOutbox,
            transport,
            MediaMissingReport.serializer(),
            dropPermanentlyRejected = false,
            responseDates = dates,
            log = log,
        )

    fun uploader(
        source: OriginalSource,
        transport: OriginalTransport,
        env: OriginalEnvironment,
        log: (String) -> Unit,
    ): OriginalUploader = OriginalUploader(originals, source, transport, reporter, env, log)
}
