// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.io.InputStream
import java.security.MessageDigest
import java.time.DateTimeException
import java.time.Duration
import java.time.Instant
import java.time.LocalDateTime
import java.time.ZoneId
import java.time.ZoneOffset
import java.time.format.DateTimeFormatter
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive

/**
 * 出来事の時刻に撮影時刻を採る下限（ST09 Q14）。これより前は「時計が狂っていた」とみなして採らない。
 * `2000-01-01` ちょうどではなく 2000-01-02 なのは、時計を持たない機器が出す `2000-01-01T00:00:00` を落とすため。
 */
val TAKEN_AT_FLOOR: Instant = Instant.parse("2000-01-02T00:00:00Z")

/** 撮影時刻が端末に保存された時刻よりこれ以上後なら採らない（Q14。ちょうどを含む）。 */
val TAKEN_AT_MAX_AHEAD: Duration = Duration.ofHours(24)

/** 型が UNDEFINED / BYTE のタグがこの長さを超えたら、値を原文に入れず名前だけ `omitted` に挙げる（design D4（仮））。 */
const val OMIT_BINARY_OVER: Int = 256

/** 内容ハッシュを流すときの読み込みの大きさ。ファイル全体をメモリに持たない。 */
private const val HASH_BUFFER_BYTES = 64 * 1024

/**
 * 端末の一覧（`MediaStore`）の 1 行のうち、記録が名指しで使う列と、原文に入れる全部の列。
 *
 * [columns] は**端末が返した列の全部**を文字列にしたもの（原文の `store`）。列の名前の並びは問わない
 * （原文を組むときに名前順に並べ直す）。
 */
data class MediaStoreRow(
    val mediaId: Long,
    val volume: String,
    /** `DATE_TAKEN`（ミリ秒）。端末が EXIF から作るか、無ければ保存の時刻で埋める。**採らない**（design D4） */
    val dateTakenMs: Long?,
    /** `DATE_ADDED`（秒）。端末に保存された時刻 */
    val dateAddedSec: Long,
    val mimeType: String?,
    val sizeBytes: Long?,
    val width: Int?,
    val height: Int?,
    val relativePath: String?,
    val displayName: String?,
    val columns: Map<String, String?>,
)

enum class MediaKind { IMAGE, VIDEO }

/**
 * ファイルの中のメタ（`ExifInterface` / `MediaMetadataRetriever` が返した値）。読み方は `MediaMetaReader.kt`。
 *
 * - 写真は [exif]（タグ名 → 端末が返した文字列）。動画は [container]（入れ物のキー名 → 文字列）
 * - [takenAtRaw] / [offsetRaw] は文字列のまま（写真は `DateTimeOriginal` / `OffsetTimeOriginal`、動画は入れ物の作成時刻）
 * - 位置は**そのファイル自身のメタのもの**だけ（C6）。端末の位置で埋めない
 */
data class MediaFileMeta(
    val kind: MediaKind,
    val exif: Map<String, String> = emptyMap(),
    /**
     * [exif] の各タグの**値のバイト数**（無ければ文字列の長さで見る）。UNDEFINED / BYTE は 0 のバイトを含み、
     * 端末が返す文字列はそこで切れる —— 文字列の長さでは 256 バイトを超えたかが分からない。
     */
    val exifBytes: Map<String, Int> = emptyMap(),
    val container: Map<String, String> = emptyMap(),
    /** 埋め込みの縮小画像がある（バイトは持ち込まず、名前だけ `omitted` に挙げる） */
    val hasThumbnail: Boolean = false,
    val takenAtRaw: String? = null,
    val offsetRaw: String? = null,
    val latitude: Double? = null,
    val longitude: Double? = null,
    val durationMs: Long? = null,
)

/** 組み立てた記録 1 件。[sha256] が外部識別子（design D4 / C17）。 */
data class MediaRecord(
    val sha256: String,
    val eventTime: Instant,
    val tzOffsetMin: Int,
    val tzId: String,
    val payload: JsonObject,
    /** 原文（JSON の文字列）。同じ入力は毎回同じ文字列になる（冪等キーの材料） */
    val raw: String,
)

/**
 * 写真・動画 1 件の記録を組む（tasks 2.1 / design D4）。**純粋な関数**: 端末にも時計にも触らない（地域は [zone] で渡す）。
 *
 * 出来事の時刻（Q14）は撮影時刻だけを見る。端末の一覧の `DATE_TAKEN` は持つが採らない
 * （端末が保存の時刻で埋めるので、採ると「撮影時刻が無い」枝が消える）。
 * 採れない（無い・読めない・2000-01-02 より前・保存より 24 時間以上後）ときは保存された時刻 + `time_basis = stored`。
 */
fun buildMediaRecord(row: MediaStoreRow, meta: MediaFileMeta, sha256: String, zone: ZoneId): MediaRecord {
    val added = Instant.ofEpochSecond(row.dateAddedSec)
    val offset = meta.offsetRaw?.let(::parseOffset)
    val taken = meta.takenAtRaw?.let { parseTakenAt(it, offset ?: zone) }
    val useTaken = taken != null && taken >= TAKEN_AT_FLOOR && taken < added.plus(TAKEN_AT_MAX_AHEAD)
    val eventTime = if (useTaken) checkNotNull(taken) else added
    // 地域: 撮影時刻を採り、メタにずれがあればそれ。それ以外は端末の地域をその時刻に当てる（C7）
    val fromExif = useTaken && offset != null
    val tz: ZoneId = if (fromExif) checkNotNull(offset) else zone
    val tzOffsetMin = tz.rules.getOffset(eventTime).totalSeconds / SECONDS_PER_MINUTE

    val payload = buildMap<String, JsonElement> {
        put("kind", JsonPrimitive("media"))
        put("sha256", JsonPrimitive(sha256))
        put("media_id", JsonPrimitive(row.mediaId))
        put("volume", JsonPrimitive(row.volume))
        meta.takenAtRaw?.let { put("taken_at_exif", JsonPrimitive(it)) }
        meta.offsetRaw?.let { put("offset_exif", JsonPrimitive(it)) }
        row.dateTakenMs?.let { put("date_taken_store", JsonPrimitive(Instant.ofEpochMilli(it).toString())) }
        put("date_added", JsonPrimitive(added.toString()))
        put("time_basis", JsonPrimitive(if (useTaken) "taken" else "stored"))
        put("tz_basis", JsonPrimitive(if (fromExif) "exif" else "device"))
        meta.latitude?.let { put("lat", JsonPrimitive(it)) }
        meta.longitude?.let { put("lon", JsonPrimitive(it)) }
        meta.exif["Make"]?.let { put("make", JsonPrimitive(it)) }
        meta.exif["Model"]?.let { put("model", JsonPrimitive(it)) }
        row.width?.let { put("width", JsonPrimitive(it)) }
        row.height?.let { put("height", JsonPrimitive(it)) }
        row.mimeType?.let { put("mime", JsonPrimitive(it)) }
        row.sizeBytes?.let { put("size_bytes", JsonPrimitive(it)) }
        meta.durationMs?.let { put("duration_ms", JsonPrimitive(it)) }
        row.relativePath?.let { put("relative_path", JsonPrimitive(it)) }
        row.displayName?.let { put("display_name", JsonPrimitive(it)) }
    }
    return MediaRecord(
        sha256 = sha256,
        eventTime = eventTime,
        tzOffsetMin = tzOffsetMin,
        tzId = tz.id,
        payload = JsonObject(payload),
        raw = rawText(row, meta),
    )
}

/** 取り込み口へ送る 1 件にする。外部識別子は内容ハッシュそのもの（`external_id_kind = record` の登録は移行にある）。 */
fun MediaRecord.toIngestRequest(
    id: String,
    userId: String,
    deviceId: String,
    logicalSource: String,
): IngestRequest = IngestRequest(
    id = id,
    userId = userId,
    logicalSource = logicalSource,
    externalId = sha256,
    deviceId = deviceId,
    origin = "collected",
    eventTime = eventTime.toString(),
    tzOffsetMin = tzOffsetMin,
    tzId = tzId,
    schemaVersion = 1,
    unitSystem = null,
    crs = null,
    raw = raw,
    payload = payload,
)

/** ファイル全体のバイトの SHA-256（16 進の小文字）。流して読むので大きい動画でもメモリに載せない。 */
fun sha256Hex(input: InputStream): String {
    val digest = MessageDigest.getInstance("SHA-256")
    val buffer = ByteArray(HASH_BUFFER_BYTES)
    while (true) {
        val n = input.read(buffer)
        if (n < 0) break
        digest.update(buffer, 0, n)
    }
    return digest.digest().joinToString("") { "%02x".format(it) }
}

private const val SECONDS_PER_MINUTE = 60

/** 縮小画像の本体と、その場所を指す欄。値は持ち込まない。 */
private const val THUMBNAIL_TAG = "ThumbnailImage"
private val THUMBNAIL_POINTER_TAGS = setOf("JPEGInterchangeFormat", "JPEGInterchangeFormatLength")

/** 型が UNDEFINED / BYTE のタグ。長さが [OMIT_BINARY_OVER] を超えたら値を持ち込まない。**文字のタグは長さで落とさない**（review R4） */
private val BINARY_TYPED_TAGS = setOf(
    "MakerNote", "OECF", "SpatialFrequencyResponse", "DeviceSettingDescription", "CFAPattern", "Xmp",
)

private val EXIF_DATE: DateTimeFormatter = DateTimeFormatter.ofPattern("yyyy:MM:dd HH:mm:ss")
private val CONTAINER_DATE: DateTimeFormatter = DateTimeFormatter.ofPattern("yyyyMMdd'T'HHmmss[.SSS]'Z'")

private fun parseOffset(raw: String): ZoneOffset? = try {
    ZoneOffset.of(raw.trim())
} catch (_: DateTimeException) {
    null
}

/**
 * 撮影時刻の文字列を読む。写真は `yyyy:MM:dd HH:mm:ss`（ずれが無ければ [zoneOrOffset] で読む）、
 * 動画の入れ物は `yyyyMMdd'T'HHmmss.SSS'Z'`（UTC）。読めなければ null。
 */
private fun parseTakenAt(raw: String, zoneOrOffset: ZoneId): Instant? {
    val text = raw.trim()
    return try {
        LocalDateTime.parse(text, EXIF_DATE).atZone(zoneOrOffset).toInstant()
    } catch (_: DateTimeException) {
        try {
            LocalDateTime.parse(text, CONTAINER_DATE).toInstant(ZoneOffset.UTC)
        } catch (_: DateTimeException) {
            null
        }
    }
}

private val rawJson = Json { encodeDefaults = true }

/** 原文（C5）。並びは名前順に固める —— 同じ入力が別の文字列になると冪等キーが動く。 */
private fun rawText(row: MediaStoreRow, meta: MediaFileMeta): String {
    val store = JsonObject(
        row.columns.toSortedMap().mapValues { (_, v) -> if (v == null) JsonNull else JsonPrimitive(v) },
    )
    val omitted = sortedSetOf<String>()
    val body: Pair<String, JsonObject> = when (meta.kind) {
        MediaKind.IMAGE -> {
            val kept = sortedMapOf<String, JsonElement>()
            for ((name, value) in meta.exif) {
                when {
                    name in THUMBNAIL_POINTER_TAGS -> omitted += name
                    name in BINARY_TYPED_TAGS && (meta.exifBytes[name] ?: value.length) > OMIT_BINARY_OVER -> omitted += name
                    else -> kept[name] = JsonPrimitive(value)
                }
            }
            if (meta.hasThumbnail) omitted += THUMBNAIL_TAG
            "exif" to JsonObject(kept)
        }
        MediaKind.VIDEO ->
            "container" to JsonObject(meta.container.toSortedMap().mapValues { (_, v) -> JsonPrimitive(v) })
    }
    val whole = JsonObject(
        mapOf(
            "store" to store,
            body.first to body.second,
            "omitted" to JsonArray(omitted.map { JsonPrimitive(it) }),
        ),
    )
    return rawJson.encodeToString(JsonObject.serializer(), whole)
}
