// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.media.ExifInterface
import android.media.MediaMetadataRetriever
import kotlin.math.roundToLong

/**
 * `ExifInterface` / `MediaMetadataRetriever` が返した値を [MediaFileMeta] に写す（tasks 2.1）。
 * 端末の API を呼ぶ**薄い皮**で、記録の組み立て（[buildMediaRecord]）は触らない。
 *
 * 位置の権限（`ACCESS_MEDIA_LOCATION`）が無い間は framework が位置の欄を伏せる。ここでは「欄が無い」とだけ読む
 * （伏せられたまま送らない判断は、ファイルを開く前の権限の確認が持つ。Q15）。
 */

/** `ExifInterface` が名前を持つ全部のタグ。定数から集める —— 端末の版が足したタグも拾う。 */
private val EXIF_TAG_NAMES: List<String> by lazy {
    ExifInterface::class.java.fields
        .filter { it.name.startsWith("TAG_") && it.type == String::class.java }
        .map { it.get(null) as String }
        .distinct()
}

fun readImageMeta(exif: ExifInterface): MediaFileMeta {
    val values = sortedMapOf<String, String>()
    val bytes = sortedMapOf<String, Int>()
    for (name in EXIF_TAG_NAMES) {
        val value = exif.getAttribute(name) ?: continue
        values[name] = value
        // [offset, length]。読めなければ（ファイルが変わったなど）文字列の長さで見る
        runCatching { exif.getAttributeRange(name)?.get(1)?.toInt() }.getOrNull()?.let { bytes[name] = it }
    }
    return MediaFileMeta(
        kind = MediaKind.IMAGE,
        exif = values,
        exifBytes = bytes,
        hasThumbnail = exif.hasThumbnail(),
        takenAtRaw = values[ExifInterface.TAG_DATETIME_ORIGINAL],
        offsetRaw = values[ExifInterface.TAG_OFFSET_TIME_ORIGINAL],
        latitude = gpsCoordinate(values[ExifInterface.TAG_GPS_LATITUDE], values[ExifInterface.TAG_GPS_LATITUDE_REF]),
        longitude = gpsCoordinate(values[ExifInterface.TAG_GPS_LONGITUDE], values[ExifInterface.TAG_GPS_LONGITUDE_REF]),
    )
}

/** `MediaMetadataRetriever` のキーの定数から集める（`METADATA_KEY_` を除いた名前）。 */
private val CONTAINER_KEYS: List<Pair<String, Int>> by lazy {
    val prefix = "METADATA_KEY_"
    MediaMetadataRetriever::class.java.fields
        .filter { it.name.startsWith(prefix) && it.type == Int::class.javaPrimitiveType }
        .map { it.name.removePrefix(prefix) to it.getInt(null) }
        .sortedBy { it.first }
}

fun readVideoMeta(retriever: MediaMetadataRetriever): MediaFileMeta {
    val values = sortedMapOf<String, String>()
    for ((name, key) in CONTAINER_KEYS) retriever.extractMetadata(key)?.let { values[name] = it }
    val location = values["LOCATION"]?.let(::parseIso6709)
    return MediaFileMeta(
        kind = MediaKind.VIDEO,
        container = values,
        takenAtRaw = values["DATE"],
        durationMs = values["DURATION"]?.toLongOrNull(),
        latitude = location?.first,
        longitude = location?.second,
    )
}

private const val MICRO = 1_000_000.0
private const val MINUTES_PER_DEGREE = 60.0
private const val SECONDS_PER_DEGREE = 3600.0
private const val DMS_PARTS = 3

/** 度分秒の有理数（`35/1,40/1,5232/100`）と N/S・E/W を、符号つきの十進の度にする（小数 6 桁。約 0.1 m）。 */
internal fun gpsCoordinate(dms: String?, ref: String?): Double? {
    val parts = dms?.split(',')?.map { rational(it) }
    if (parts == null || parts.size != DMS_PARTS || parts.any { it == null }) return null
    val (deg, min, sec) = parts.filterNotNull()
    val degrees = deg + min / MINUTES_PER_DEGREE + sec / SECONDS_PER_DEGREE
    val signed = if (ref == "S" || ref == "W") -degrees else degrees
    return (signed * MICRO).roundToLong() / MICRO
}

private fun rational(text: String): Double? {
    val halves = text.trim().split('/')
    val num = halves.getOrNull(0)?.toDoubleOrNull()
    val den = (halves.getOrNull(1) ?: "1").toDoubleOrNull()
    return if (num == null || den == null || den == 0.0) null else num / den
}

private val ISO_6709 = Regex("^([+-]\\d+(?:\\.\\d+)?)([+-]\\d+(?:\\.\\d+)?)")

/** ISO 6709 の `+35.6812+139.7671/`（高度が付くこともある）を（緯度, 経度）にする。 */
internal fun parseIso6709(text: String): Pair<Double, Double>? {
    val m = ISO_6709.find(text.trim())
    val lat = m?.groupValues?.get(1)?.toDoubleOrNull()
    val lon = m?.groupValues?.get(2)?.toDoubleOrNull()
    return if (lat == null || lon == null) null else lat to lon
}
