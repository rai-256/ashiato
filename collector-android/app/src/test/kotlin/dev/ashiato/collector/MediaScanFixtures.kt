// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.io.ByteArrayInputStream
import java.io.File
import java.nio.file.Files
import java.time.Instant
import java.time.ZoneId

const val PRIMARY = "external_primary"
const val SD_CARD = "1A2B-3C4D"

/** 端末の一覧の偽物。変えるたびに世代の数が上がる（本物の `MediaStore` と同じ）。 */
class FakeMediaListing : MediaListing {
    val volumeSet = mutableSetOf(PRIMARY)
    val contents = HashMap<Pair<String, Long>, ByteArray>()
    private val items = HashMap<String, MutableMap<Long, MediaListed>>()
    private val versions = HashMap<String, String>()
    private val generations = HashMap<String, Long>()
    var biggestPage = 0
    var contains = 0

    override fun volumes(): Set<String> = volumeSet.toSet()

    override fun version(volume: String): String? = versions[volume] ?: "v1"

    override fun generation(volume: String): Long = generations[volume] ?: 1L

    override fun page(volume: String, afterId: Long, generationAfter: Long?, limit: Int): List<MediaListed> {
        val out = items[volume].orEmpty().values
            .filter { it.row.mediaId > afterId && (generationAfter == null || it.generationModified > generationAfter) }
            .sortedBy { it.row.mediaId }
            .take(limit)
        biggestPage = maxOf(biggestPage, out.size)
        return out
    }

    override fun contains(volume: String, mediaId: Long): Boolean {
        contains++
        return items[volume]?.containsKey(mediaId) == true
    }

    /** 追加か上書き。**世代の数を上げる**。 */
    fun put(volume: String, id: Long, path: String, name: String, body: String) =
        write(volume, id, Stored(path, name, body), bump = true)

    fun putVideo(volume: String, id: Long, path: String, name: String, body: String) =
        write(volume, id, Stored(path, name, body, MediaKind.VIDEO), bump = true)

    private class Stored(
        val path: String,
        val name: String,
        val body: String,
        val kind: MediaKind = MediaKind.IMAGE,
        val trashed: Boolean = false,
    )

    private fun write(volume: String, id: Long, stored: Stored, bump: Boolean) {
        val gen = (generations[volume] ?: 1L) + if (bump) 1 else 0
        generations[volume] = gen
        contents[volume to id] = stored.body.toByteArray()
        val columns = linkedMapOf<String, String?>(
            "_id" to id.toString(),
            "relative_path" to stored.path,
            "_display_name" to stored.name,
            "_size" to stored.body.length.toString(),
            "is_trashed" to if (stored.trashed) "1" else "0",
            "date_modified" to gen.toString(),
            "generation_modified" to gen.toString(),
        )
        val row = MediaStoreRow(
            mediaId = id,
            volume = volume,
            dateTakenMs = null,
            dateAddedSec = 1_700_000_000L,
            mimeType = if (stored.kind == MediaKind.VIDEO) "video/mp4" else "image/jpeg",
            sizeBytes = stored.body.length.toLong(),
            width = 10,
            height = 10,
            relativePath = stored.path,
            displayName = stored.name,
            columns = columns,
        )
        items.getOrPut(volume) { LinkedHashMap() }[id] =
            MediaListed(row, stored.kind, dateModifiedSec = gen, generationModified = gen, trashed = stored.trashed)
    }

    fun remove(volume: String, id: Long) {
        items[volume]?.remove(id)
        contents.remove(volume to id)
        generations[volume] = (generations[volume] ?: 1L) + 1
    }

    fun trash(volume: String, id: Long) {
        val item = checkNotNull(items[volume]?.get(id))
        val stored = Stored(
            item.row.relativePath.orEmpty(),
            item.row.displayName.orEmpty(),
            String(contents.getValue(volume to id)),
            item.kind,
            trashed = true,
        )
        write(volume, id, stored, bump = true)
    }

    /** 一覧が作り直された: 版が変わり、番号が振り直される。世代の数は変えない（作り直しで戻りうる）。 */
    fun rebuild(volume: String, shift: Long) {
        val old = items[volume].orEmpty().values.toList()
        val oldContents = old.associate { it.row.mediaId to contents.getValue(volume to it.row.mediaId) }
        old.forEach {
            items[volume]?.remove(it.row.mediaId)
            contents.remove(volume to it.row.mediaId)
        }
        versions[volume] = "v-${(versions[volume] ?: "v1")}-$shift"
        for (o in old) {
            val id = o.row.mediaId + shift
            val columns = o.row.columns + ("_id" to id.toString())
            contents[volume to id] = oldContents.getValue(o.row.mediaId)
            items.getOrPut(volume) { LinkedHashMap() }[id] =
                o.copy(row = o.row.copy(mediaId = id, columns = columns))
        }
    }

    /** 世代の数も更新日時も触らずに一覧へ足す（変わったかの印が上がらない外れ方）。 */
    fun putSilently(volume: String, id: Long, path: String, name: String, body: String) {
        write(volume, id, Stored(path, name, body), bump = false)
    }
}

/** ファイルを開く偽物。開いた回数を数える。 */
class FakeMediaFiles(private val listing: FakeMediaListing) : MediaFileReader {
    var opened = 0
    var unreadable = false

    override fun read(item: MediaListed): MediaFileContent? {
        if (unreadable) return null
        opened++
        val bytes = listing.contents.getValue(item.row.volume to item.row.mediaId)
        return MediaFileContent(sha256Hex(ByteArrayInputStream(bytes)), MediaFileMeta(kind = item.kind))
    }
}

/** 仮想の時間で動く契機の口。 */
class VirtualTimer : MediaScanTimer {
    private class Job(val at: Long, val task: () -> Unit) {
        var cancelled = false
    }

    private val jobs = mutableListOf<Job>()
    var nowMs = 0L

    override fun after(delayMs: Long, task: () -> Unit): () -> Unit {
        val job = Job(nowMs + delayMs, task)
        jobs += job
        return { job.cancelled = true }
    }

    fun advance(ms: Long, onTick: (Long) -> Unit = {}) {
        val end = nowMs + ms
        while (true) {
            val next = jobs.filter { !it.cancelled && it.at <= end }.minByOrNull { it.at } ?: break
            jobs.remove(next)
            nowMs = next.at
            onTick(nowMs)
            next.task()
        }
        nowMs = end
    }
}

class MediaScanEnv {
    val dir: File = Files.createTempDirectory("st09-media").toFile()
    val listing = FakeMediaListing()
    val files = FakeMediaFiles(listing)
    val lines = mutableListOf<String>()
    var outbox: Outbox<IngestRequest> = testOutbox()
    var wall: Instant = Instant.parse("2026-10-01T00:00:00Z")
    var access = MediaAccess.FULL
    var location = true
    private var ids = 0

    fun newScanner() = MediaScanner(
        listing = listing,
        files = files,
        outbox = outbox,
        store = MediaLedgerStore(File(dir, MEDIA_LEDGER_DIR)) { lines += it },
        env = MediaScanEnvironment(
            access = { if (!location) MediaAccess.NO_LOCATION else access },
            now = { wall },
            zone = { ZoneId.of("Asia/Tokyo") },
            userId = { "user-1" },
            deviceId = "device-1",
            newId = { "r${ids++}" },
        ),
        log = { lines += it },
    )

    var scanner: MediaScanner = newScanner()

    fun scan(): MediaScanResult = scanner.scan()

    fun sent(): List<IngestRequest> = outbox.snapshot()

    fun photos() = sent().filter { it.payload["kind"].toString() == "\"media\"" }

    fun vanished() = sent().filter { it.payload["kind"].toString() == "\"vanished\"" }

    /** 今日から [hours] 時間進める。 */
    fun advanceHours(hours: Long) {
        wall = wall.plusSeconds(hours * 3600)
    }
}
