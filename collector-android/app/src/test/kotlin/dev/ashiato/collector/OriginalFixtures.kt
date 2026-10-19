// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.io.File
import java.io.InputStream
import java.security.MessageDigest
import java.time.Instant

/** 端末のファイルを開く偽物。中身は [FakeMediaListing] の `contents`（消せば「端末から消えた」になる）。 */
class FakeOriginalSource(private val listing: FakeMediaListing) : OriginalSource {
    /** 開いた回数（読んだかどうかの目安） */
    var opens = 0

    /** 2 回目以降に開いたときだけ返す別のバイト（送っている間に編集された） */
    var afterFirstOpen: ByteArray? = null
    var denied = false
    var unreadable = false
    private val openedBy = HashMap<Long, Int>()

    override fun open(entry: OriginalEntry): OriginalOpen {
        opens++
        val bytes = listing.contents[entry.volume to entry.mediaId]
        return when {
            denied -> OriginalOpen.Denied
            unreadable -> OriginalOpen.Unreadable
            bytes == null -> OriginalOpen.Vanished
            else -> OriginalOpen.Opened(bytesFor(entry, bytes).inputStream())
        }
    }

    private fun bytesFor(entry: OriginalEntry, onDevice: ByteArray): ByteArray {
        val nth = (openedBy[entry.mediaId] ?: 0) + 1
        openedBy[entry.mediaId] = nth
        return afterFirstOpen?.takeIf { nth >= 2 } ?: onDevice
    }
}

/** 原本の受け口の偽物。受け取ったバイトを S-01 と同じ規則（ハッシュが合うものだけ置く）で `stored` に置く。 */
class FakeOriginalTransport : OriginalTransport {
    val stored = HashMap<String, ByteArray>()
    val heads = ArrayList<String>()

    /** `PUT` を受けた順（ハッシュ） */
    val puts = ArrayList<String>()
    var unreachable = false

    /** 決めた応答を返す（`PUT`）。null なら実際に置く */
    var putStatus: Int? = null
    val lengths = ArrayList<Long>()

    override fun head(sha256: String): Outcome {
        heads += sha256
        if (unreachable) return Outcome.Unreachable("ConnectException")
        return Outcome.Responded(if (sha256 in stored) 200 else 404, "")
    }

    override fun put(sha256: String, length: Long, body: InputStream): Outcome {
        puts += sha256
        lengths += length
        if (unreachable) return Outcome.Unreachable("ConnectException")
        val bytes = body.readBytes()
        val intact = bytes.size.toLong() == length && sha256Of(bytes) == sha256
        if (putStatus == null && intact) stored[sha256] = bytes
        return Outcome.Responded(putStatus ?: if (intact) 201 else 422, "")
    }
}

fun sha256Of(bytes: ByteArray): String =
    MessageDigest.getInstance("SHA-256").digest(bytes).joinToString("") { "%02x".format(it) }

/** 受け口が何でも受理する偽物（`/ingest`）。 */
val acceptEverything = Transport { body ->
    val n = ingestJson.decodeFromString<List<IngestRequest>>(body).size
    Outcome.Responded(200, verdicts(n, """{"accepted":true}"""))
}

/** 受け口が何でも恒久的に断る偽物（`/ingest`）。 */
val rejectEverything = Transport { body ->
    val n = ingestJson.decodeFromString<List<IngestRequest>>(body).size
    Outcome.Responded(200, verdicts(n, """{"accepted":false,"error":"malformed"}"""))
}

/** 受け口の応答（1 件ごとの結果の配列）。 */
fun verdicts(count: Int, each: String): String = List(count) { each }.joinToString(",", "[", "]")

/** 読み手 → 記録の送信 → 原本の送り手までを 1 つに組む試験の足場。 */
class OriginalHarness(val st: TestStores = TestStores()) {
    val env = MediaScanEnv().also {
        it.outbox = st.records
        it.scanner = it.newScanner()
    }
    val missing = Outbox(
        SegmentStore(File(st.dir, "media-missing"), MediaMissingReport.serializer(), st.unreadable, st.log),
        st.age::now,
    )
    private var seq = 0
    val reporter = OriginalMissingReporter(
        missing,
        { "user-1" },
        "device-1",
        { env.wall },
        { "m${seq++}" },
        st.log,
    )
    val link = OriginalRecordLink(env.originals, reporter)
    val source = FakeOriginalSource(env.listing)
    val transport = FakeOriginalTransport()
    var unmetered = true
    var access = MediaAccess.FULL
    val lines = mutableListOf<String>()
    val uploader = OriginalUploader(
        env.originals,
        source,
        transport,
        reporter,
        OriginalEnvironment(access = { access }, unmetered = { unmetered }, now = { env.wall }),
        log = { lines += it },
    )

    /** 保存の時刻が [hoursAgo] 時間前の写真を 1 枚足す。 */
    fun photo(id: Long, body: String, hoursAgo: Long = 1, name: String = "IMG_$id.jpg") {
        env.listing.dateAddedSec = env.wall.minusSeconds(hoursAgo * 3600).epochSecond
        env.listing.put(PRIMARY, id, "DCIM/Camera/", name, body)
    }

    /** 未送信の記録を、S-01 が受理した形で送る（原本は `ready` になる）。 */
    fun acceptRecords(): Sender.Flushed = recordSender(acceptEverything).flush()

    /** 記録の送り手。本番と同じく、結果を原本の送り待ちへ繋ぐ。 */
    fun recordSender(transport: Transport): Sender<IngestRequest> =
        Sender(st.records, transport, IngestRequest.serializer(), dropPermanentlyRejected = true).also {
            it.listener = link
        }

    fun shaOf(id: Long): String = sha256Of(env.listing.contents.getValue(PRIMARY to id))

    fun scan(): MediaScanResult = env.scan()

    fun advanceMs(ms: Long) {
        env.wall = env.wall.plusMillis(ms)
    }

    fun missingReports(): List<MediaMissingReport> = missing.snapshot()

    fun now(): Instant = env.wall
}
