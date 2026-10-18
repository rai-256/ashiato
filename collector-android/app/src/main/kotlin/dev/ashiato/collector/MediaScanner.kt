// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import java.security.MessageDigest
import java.time.Instant
import java.time.ZoneId

const val MEDIA_PHOTO_SOURCE: String = "c01-photo"
const val MEDIA_SCREENSHOT_SOURCE: String = "c01-screenshot"
const val MEDIA_IMAGE_SOURCE: String = "c01-image"
const val MEDIA_VIDEO_SOURCE: String = "c01-video"

private val SCREENSHOT_DIRS = listOf("DCIM/Screenshots/", "Pictures/Screenshots/")

/** 論理ソースを保存先のフォルダで決める（design D3（仮）の表）。最初に決めたもので固定する呼び手の責任。 */
fun mediaLogicalSource(kind: MediaKind, relativePath: String?): String {
    val path = relativePath.orEmpty()
    return when {
        kind == MediaKind.VIDEO -> MEDIA_VIDEO_SOURCE
        SCREENSHOT_DIRS.any { path.startsWith(it) } -> MEDIA_SCREENSHOT_SOURCE
        path.startsWith("DCIM/") -> MEDIA_PHOTO_SOURCE
        else -> MEDIA_IMAGE_SOURCE
    }
}

/**
 * 原文の指紋に入れない列。番号の振り直しや世代の数の上がりだけでは「原文が変わった」と数えない
 * （一覧が作り直されても記録を送り直さない。Scenario「一覧が作り直されても記録は増えない」）。
 */
private val VOLATILE_COLUMNS = setOf("_id", "generation_added", "generation_modified")

sealed interface MediaScanResult {
    /** 読めた。[enqueued] は未送信への登録を試みた記録の数（消えた事実を含む） */
    data class Scanned(val enqueued: Int, val fullReconcile: Boolean) : MediaScanResult

    /** 読めない状態。何もしていない（台帳も印も動かさない） */
    data class Unavailable(val reason: String) : MediaScanResult

    /** 途中で積めなかった・開けなかったものがある。印を進めていないので次の契機に取り直す */
    data class Held(val enqueued: Int) : MediaScanResult
}

/** 読み手が端末と利用者について聞く口（時計・地域・利用者・端末・記録の識別子・アクセスの状態）。 */
class MediaScanEnvironment(
    /** 写真の位置の権限が無ければ [MediaAccess.NO_LOCATION]（Q15）。それ以外の状態より先に答える */
    val access: () -> MediaAccess,
    val now: () -> Instant,
    val zone: () -> ZoneId,
    val userId: () -> String,
    val deviceId: String,
    val newId: () -> String,
)

/**
 * 端末の写真・画像・動画の一覧を読んで、記録を未送信に積む（tasks 3.1 / design D1・D2・D3・D5）。
 * 1 回の呼びで: 置き場ごとに世代の数を聞き、変わっていれば変わった行を 200 件ずつ読む。全部の突き合わせが
 * 要るとき（前回から 24 時間・時計が戻った・一覧の版が変わった・初回）は一覧の全部を読み、消えた事実も判定する。
 *
 * - **ハッシュを計算し直すのは、新しい項目か、大きさ・`DATE_MODIFIED`・`GENERATION_MODIFIED` が台帳と違う項目だけ**
 * - 同じハッシュの記録をすでに送っていれば新しい記録にしない（C17）。原文だけ違えば同じ外部識別子で送り直す（D4）
 * - 中身が変わったら `previous_sha256` を持つ新しい記録（Q7）
 * - **ファイルを開く前に位置の権限を見る**（Q15）。無ければ何も読まない
 */
class MediaScanner(
    private val listing: MediaListing,
    private val files: MediaFileReader,
    private val outbox: Outbox<IngestRequest>,
    private val store: MediaLedgerStore,
    private val env: MediaScanEnvironment,
    private val log: (String) -> Unit = {},
) {
    /** 読み進めた印の保存や台帳の更新が重ならないよう 1 本ずつ。 */
    @Synchronized
    fun scan(): MediaScanResult {
        val level = env.access()
        val blocked = when (level) {
            MediaAccess.NONE -> REASON_NO_ACCESS
            MediaAccess.NO_LOCATION -> REASON_NO_LOCATION
            else -> null
        }
        if (blocked != null) return MediaScanResult.Unavailable(blocked)
        val pass = Pass(level, env.now(), store.loadCursor(), listing.volumes())
        pass.run()
        log(Telemetry.line("media_scan", source = null, count = pass.enqueued))
        return if (pass.held) MediaScanResult.Held(pass.enqueued) else MediaScanResult.Scanned(pass.enqueued, pass.full)
    }

    /** 1 回の呼びの作業場。 */
    private inner class Pass(
        val level: MediaAccess,
        val at: Instant,
        val cursor: MediaCursor,
        val volumes: Set<String>,
    ) {
        var enqueued = 0
        var held = false
        val full: Boolean = fullReconcileDue(
            cursor,
            at,
            versionChanged = volumes.any { listing.version(it) != cursor.volumes[it]?.version },
        )
        private val ledgers = HashMap<String, MutableMap<Long, LedgerEntry>>()
        private val heldVolumes = HashSet<String>()
        private val versions = HashMap<String, String?>()
        private val generations = HashMap<String, Long>()
        private val untrusted = HashSet<String>()
        private val index = HashIndex()
        private val seen = HashSet<Pair<String, Long>>()
        private val stale = ArrayList<Slot>()

        fun run() {
            for (volume in volumes) {
                versions[volume] = listing.version(volume)
                generations[volume] = listing.generation(volume)
                openLedger(volume)
            }
            for (volume in volumes) scanVolume(volume)
            if (full && level == MediaAccess.FULL && !held) judgeVanished()
            advanceCursor()
        }

        private fun openLedger(volume: String) {
            val trust = versions[volume] == cursor.volumes[volume]?.version
            val loaded = store.loadLedger(volume)
            if (trust) {
                ledgers[volume] = loaded
                loaded.values.forEach { index.add(Slot(volume, it, stale = false, fresh = false)) }
            } else {
                // 一覧が作り直された: 番号は信じず、台帳の行は「同じハッシュを探す材料」にだけ使う
                untrusted += volume
                ledgers[volume] = LinkedHashMap()
                for (entry in loaded.values) {
                    val slot = Slot(volume, entry, stale = true, fresh = false)
                    index.add(slot)
                    stale.add(slot)
                }
            }
        }

        private fun scanVolume(volume: String) {
            val previous = cursor.volumes[volume]
            val since = if (full) null else previous?.generation
            val untouched = !full && previous != null && generations[volume] == previous.generation
            var afterId = 0L
            var page: List<MediaListed> = emptyList()
            while (!untouched) {
                page = listing.page(volume, afterId, since, SCAN_BATCH)
                page.forEach { handle(volume, it) }
                if (page.isNotEmpty()) {
                    if (!store.saveLedger(volume, ledgers.getValue(volume).values)) markHeld(volume)
                    afterId = page.last().row.mediaId
                }
                if (page.size < SCAN_BATCH) break
            }
        }

        private fun markHeld(volume: String) {
            held = true
            heldVolumes += volume
        }

        private fun handle(volume: String, item: MediaListed) {
            val id = item.row.mediaId
            seen += volume to id
            val entry = if (volume in untrusted) null else ledgers.getValue(volume)[id]
            if (entry != null && unchanged(entry, item)) return
            val content = files.read(item)
            if (content == null) markHeld(volume) else record(volume, item, entry, content)
        }

        /** 読めた項目 1 件を、記録にして積み、台帳を直す。積めなければ台帳は変えず、印も進めない。 */
        private fun record(volume: String, item: MediaListed, entry: LedgerEntry?, content: MediaFileContent) {
            val digest = rawDigestOf(item, content)
            val decision = decide(volume, item, entry, content.sha256, digest)
            val request = if (decision.send) requestFor(decision, item, content, env) else null
            if (request != null) enqueued++
            if (request != null && !outbox.add(request)) {
                markHeld(volume)
            } else {
                decision.consumed?.let { slot ->
                    index.remove(slot.volume, slot.entry)
                    ledgers[slot.volume]?.remove(slot.entry.mediaId)
                    stale.remove(slot)
                }
                entry?.let { index.remove(volume, it) }
                val next = LedgerEntry(
                    mediaId = item.row.mediaId,
                    sha256 = content.sha256,
                    size = item.row.sizeBytes ?: 0L,
                    dateModified = item.dateModifiedSec,
                    generationModified = item.generationModified,
                    source = decision.source,
                    trashed = item.trashed,
                    rawDigest = digest,
                )
                ledgers.getValue(volume)[next.mediaId] = next
                index.add(Slot(volume, next, stale = false, fresh = true))
            }
        }

        private fun decide(
            volume: String,
            item: MediaListed,
            entry: LedgerEntry?,
            sha: String,
            digest: String,
        ): Decision {
            val twin = index.twinOf(sha, volume, item.row.mediaId)
            return when {
                entry != null && entry.sha256 == sha -> Decision(entry.source, digest != entry.rawDigest, null, null)
                // 中身が変わった（Q7）。変わった後の中身がすでに別の項目として記録済みなら増やさない
                entry != null -> Decision(entry.source, twin == null, entry.sha256, null)
                twin == null -> Decision(mediaLogicalSource(item.kind, item.row.relativePath), true, null, null)
                // 同じ中身が別の名前でも端末にある: 記録は 1 件のまま
                isLive(twin) -> Decision(twin.entry.source, false, null, null)
                // 動かした・番号が振り直された: 同じ記録の更新（原文が違えば送り直す）
                else -> Decision(twin.entry.source, digest != twin.entry.rawDigest, null, twin)
            }
        }

        private val isLive: (Slot) -> Boolean = { slot ->
            when {
                slot.stale -> false
                slot.fresh -> true
                slot.volume !in volumes -> false
                else -> listing.contains(slot.volume, slot.entry.mediaId)
            }
        }

        /** 全部の突き合わせでだけ、台帳にあって一覧に無い項目を消えた事実にする（D5）。 */
        private fun judgeVanished() {
            val unseen = volumes.flatMap { v ->
                ledgers.getValue(v).values
                    .filter { (v to it.mediaId) !in seen }
                    .map { Slot(v, it, stale = false, fresh = false) }
            }
            val liveHashes = index.hashesWhere { !it.stale && (it.fresh || (it.volume to it.entry.mediaId) in seen) }
            for (slot in stale.toList() + unseen) {
                val stored = slot.entry.sha256 in liveHashes || vanishedStored(slot)
                if (stored) {
                    ledgers[slot.volume]?.remove(slot.entry.mediaId)
                    stale.remove(slot)
                }
            }
            for (volume in volumes) if (!store.saveLedger(volume, ledgers.getValue(volume).values)) markHeld(volume)
        }

        private fun vanishedStored(slot: Slot): Boolean {
            enqueued++
            val stored = outbox.add(vanishedRequest(slot.entry, slot.volume, at, env))
            if (!stored) markHeld(slot.volume)
            return stored
        }

        /** 置き場ごとに、積めなかったものが無い置き場だけ読み進めた印を進める。 */
        private fun advanceCursor() {
            val next = HashMap(cursor.volumes)
            for (volume in volumes) {
                if (volume !in heldVolumes) next[volume] = VolumeCursor(generations.getValue(volume), versions[volume])
            }
            val lastFull = if (full && !held) at.toString() else cursor.lastFullReconcile
            if (!store.saveCursor(MediaCursor(next, lastFull))) held = true
        }
    }

    private companion object {
        const val REASON_NO_ACCESS = "no_media_access"
        const val REASON_NO_LOCATION = "no_location_permission"
    }
}

/** 送るか・どのソースか・何を台帳から外すかの判断。 */
private class Decision(
    val source: String,
    val send: Boolean,
    /** 中身が変わったときの、変わる前のハッシュ */
    val previousSha: String?,
    /** 台帳から外す行（動かした・番号が振り直された元の行） */
    val consumed: Slot?,
)

private fun fullReconcileDue(cursor: MediaCursor, at: Instant, versionChanged: Boolean): Boolean {
    val last = cursor.lastFullReconcile?.let(Instant::parse)
    // 前回の時刻が今より先なら（時計が戻った）すぐ行う
    return versionChanged || last == null || last.isAfter(at) || !at.isBefore(last.plus(FULL_RECONCILE_EVERY))
}

private fun requestFor(
    decision: Decision,
    item: MediaListed,
    content: MediaFileContent,
    env: MediaScanEnvironment,
): IngestRequest {
    var record = buildMediaRecord(item.row, content.meta, content.sha256, env.zone())
    if (decision.previousSha != null) {
        record = record.copy(
            payload = JsonObject(record.payload + ("previous_sha256" to JsonPrimitive(decision.previousSha))),
        )
    }
    return record.toIngestRequest(env.newId(), env.userId(), env.deviceId, decision.source)
}

private fun unchanged(entry: LedgerEntry, item: MediaListed): Boolean =
    entry.size == (item.row.sizeBytes ?: 0L) &&
        entry.dateModified == item.dateModifiedSec &&
        entry.generationModified == item.generationModified

/** 原文の指紋（番号と世代の数を除く）。送った原文と同じかを比べる。 */
private fun rawDigestOf(item: MediaListed, content: MediaFileContent): String {
    val stable = item.row.copy(columns = item.row.columns.filterKeys { it !in VOLATILE_COLUMNS })
    val raw = buildMediaRecord(stable, content.meta, content.sha256, ZoneId.of("UTC")).raw
    return MessageDigest.getInstance("SHA-256").digest(raw.toByteArray()).joinToString("") { "%02x".format(it) }
}

private const val SECONDS_PER_MINUTE_SCAN = 60

/** 消えた事実の記録（D5）。出来事の時刻は気づいた時刻、位置は持たない。外部識別子は写真の記録と重ならない。 */
private fun vanishedRequest(entry: LedgerEntry, volume: String, at: Instant, env: MediaScanEnvironment): IngestRequest {
    val noticed = at.toString()
    val payload = JsonObject(
        linkedMapOf<String, JsonElement>(
            "kind" to JsonPrimitive("vanished"),
            "media_id" to JsonPrimitive(entry.mediaId),
            "noticed_at" to JsonPrimitive(noticed),
            "sha256" to JsonPrimitive(entry.sha256),
            "volume" to JsonPrimitive(volume),
        ),
    )
    val zone = env.zone()
    return IngestRequest(
        id = env.newId(),
        userId = env.userId(),
        logicalSource = entry.source,
        externalId = "vanished:${entry.sha256}:$volume:${entry.mediaId}",
        deviceId = env.deviceId,
        origin = "collected",
        eventTime = noticed,
        tzOffsetMin = zone.rules.getOffset(at).totalSeconds / SECONDS_PER_MINUTE_SCAN,
        tzId = zone.id,
        schemaVersion = 1,
        unitSystem = null,
        crs = null,
        raw = payload.toString(),
        payload = payload,
    )
}
