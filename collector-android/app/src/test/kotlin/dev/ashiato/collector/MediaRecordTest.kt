// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.media.ExifInterface
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.contentOrNull
import kotlinx.serialization.json.doubleOrNull
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import java.io.ByteArrayInputStream
import java.time.Instant
import java.time.ZoneId

/**
 * 写真・動画の記録の組み立て（tasks 2.1 / design D4）。材料は `src/test/resources/media/`（自作。README.md）。
 * 写真のメタは本物の `ExifInterface`（Robolectric が framework の実装を動かす）で読む。
 */
@RunWith(RobolectricTestRunner::class)
class MediaRecordTest {
    private val tokyo = ZoneId.of("Asia/Tokyo")

    // ---- 材料 ---------------------------------------------------------------------------------

    private fun resource(name: String): ByteArray =
        checkNotNull(javaClass.getResourceAsStream("/media/$name")) { "材料が無い: $name" }.use { it.readBytes() }

    private fun imageMeta(name: String): MediaFileMeta =
        readImageMeta(ExifInterface(ByteArrayInputStream(resource(name))))

    private fun row(
        addedAt: String = "2026-10-17T05:30:15Z",
        dateTakenMs: Long? = null,
        extra: Map<String, String?> = emptyMap(),
    ) = MediaStoreRow(
        mediaId = 1234,
        volume = "external_primary",
        dateTakenMs = dateTakenMs,
        dateAddedSec = Instant.parse(addedAt).epochSecond,
        mimeType = "image/jpeg",
        sizeBytes = 2364,
        width = 64,
        height = 48,
        relativePath = "DCIM/Camera/",
        displayName = "IMG_0001.jpg",
        columns = mapOf("_id" to "1234", "_display_name" to "IMG_0001.jpg") + extra,
    )

    private fun build(
        meta: MediaFileMeta,
        row: MediaStoreRow = row(),
        zone: ZoneId = tokyo,
        sha256: String = "a".repeat(64),
    ) = buildMediaRecord(row, meta, sha256, zone)

    private fun taken(raw: String?, offset: String? = null) =
        MediaFileMeta(MediaKind.IMAGE, takenAtRaw = raw, offsetRaw = offset)

    private fun MediaRecord.str(key: String): String? = payload[key]?.jsonPrimitive?.contentOrNull

    private fun MediaRecord.rawJson(): JsonObject = Json.parseToJsonElement(raw).jsonObject

    // ---- 出来事の時刻（Q14） -------------------------------------------------------------------

    @Test
    fun taken_time_becomes_event_time() {
        // Scenario: 撮影時刻が記録の出来事の時刻になる
        val r = build(
            taken("2026:10:17 14:30:12", "+09:00"),
            row(addedAt = "2026-10-17T05:30:15Z"),
        )
        assertEquals(Instant.parse("2026-10-17T05:30:12Z"), r.eventTime)
        assertEquals("taken", r.str("time_basis"))
        assertEquals("exif", r.str("tz_basis"))
        assertEquals(540, r.tzOffsetMin)
    }

    @Test
    fun image_without_taken_time_falls_back_to_stored_time() {
        // Scenario: 撮影時刻が無い画像は保存された時刻になる
        val r = build(taken(null), row(addedAt = "2026-10-17T05:30:15Z"))
        assertEquals(Instant.parse("2026-10-17T05:30:15Z"), r.eventTime)
        assertEquals("stored", r.str("time_basis"))
    }

    @Test
    fun unreadable_taken_time_falls_back_to_stored_time() {
        // 読めない文字列（空白で埋めた欄など）も「無い」と同じ枝へ
        val r = build(taken("    :  :     :  :  "), row(addedAt = "2026-10-17T05:30:15Z"))
        assertEquals(Instant.parse("2026-10-17T05:30:15Z"), r.eventTime)
        assertEquals("stored", r.str("time_basis"))
    }

    @Test
    fun taken_time_before_2000_is_not_adopted() {
        // Scenario: 2000 年より前の撮影時刻は採らない
        val r = build(taken("1980:01:01 00:00:00"), row(addedAt = "2026-10-17T01:00:00Z"))
        assertEquals(Instant.parse("2026-10-17T01:00:00Z"), r.eventTime)
        assertEquals("stored", r.str("time_basis"))
        assertEquals("1980:01:01 00:00:00", r.str("taken_at_exif"))
    }

    @Test
    fun taken_time_two_days_after_stored_is_not_adopted() {
        // Scenario: 保存より 1 日以上後の撮影時刻は採らない
        val r = build(taken("2026:10:19 05:30:15", "+00:00"), row(addedAt = "2026-10-17T05:30:15Z"))
        assertEquals(Instant.parse("2026-10-17T05:30:15Z"), r.eventTime)
        assertEquals("stored", r.str("time_basis"))
    }

    @Test
    fun three_times_stay_in_record() {
        // Scenario: 3 つの時刻が記録に残る
        val r = build(
            taken("2026:10:17 14:30:12", "+09:00"),
            row(addedAt = "2026-10-17T05:30:15Z", dateTakenMs = Instant.parse("2026-10-17T05:30:13Z").toEpochMilli()),
        )
        assertEquals("2026:10:17 14:30:12", r.str("taken_at_exif"))
        assertEquals("+09:00", r.str("offset_exif"))
        assertEquals("2026-10-17T05:30:13Z", r.str("date_taken_store"))
        assertEquals("2026-10-17T05:30:15Z", r.str("date_added"))
    }

    @Test
    fun device_zone_is_applied_and_marked_when_meta_has_no_offset() {
        // Scenario: 撮影時刻の地域がメタに無ければ端末の地域を当てて印を付ける
        val r = build(taken("2026:10:17 14:30:12"), zone = tokyo)
        assertEquals(540, r.tzOffsetMin)
        assertEquals("Asia/Tokyo", r.tzId)
        assertEquals("device", r.str("tz_basis"))
        assertEquals(Instant.parse("2026-10-17T05:30:12Z"), r.eventTime)
    }

    @Test
    fun image_known_only_to_the_device_list_gets_stored_time() {
        // Scenario: 端末の一覧にだけ撮影時刻がある画像は保存された時刻になる
        val listed = Instant.parse("2026-10-17T05:00:00Z").toEpochMilli()
        val r = build(taken(null), row(addedAt = "2026-10-17T05:30:15Z", dateTakenMs = listed))
        assertEquals(Instant.parse("2026-10-17T05:30:15Z"), r.eventTime)
        assertEquals("stored", r.str("time_basis"))
        assertEquals("2026-10-17T05:00:00Z", r.str("date_taken_store"))
    }

    @Test
    fun floor_is_inclusive_and_the_second_before_is_not() {
        // Scenario: 2000-01-02 ちょうどの撮影時刻は採り、その直前は採らない
        val added = row(addedAt = "2026-10-17T05:30:15Z")
        val at = build(taken("2000:01:02 00:00:00", "+00:00"), added)
        val before = build(taken("2000:01:01 23:59:59", "+00:00"), added)
        assertEquals(Instant.parse("2000-01-02T00:00:00Z"), at.eventTime)
        assertEquals("taken", at.str("time_basis"))
        assertEquals(Instant.parse("2026-10-17T05:30:15Z"), before.eventTime)
        assertEquals("stored", before.str("time_basis"))
        assertEquals(Instant.parse("2000-01-02T00:00:00Z"), TAKEN_AT_FLOOR)
    }

    @Test
    fun exactly_24_hours_after_stored_is_not_adopted_but_one_minute_less_is() {
        // Scenario: 撮影時刻が保存のちょうど 24 時間後なら保存された時刻になる
        val added = row(addedAt = "2026-10-17T05:30:15Z")
        val exact = build(taken("2026:10:18 05:30:15", "+00:00"), added)
        val under = build(taken("2026:10:18 05:29:15", "+00:00"), added)
        assertEquals(Instant.parse("2026-10-17T05:30:15Z"), exact.eventTime)
        assertEquals("stored", exact.str("time_basis"))
        assertEquals(Instant.parse("2026-10-18T05:29:15Z"), under.eventTime)
        assertEquals("taken", under.str("time_basis"))
        assertEquals(24 * 3600L, TAKEN_AT_MAX_AHEAD.seconds)
    }

    // ---- 位置・機種・長さ -----------------------------------------------------------------------

    @Test
    fun photo_without_location_has_no_coordinates() {
        // Scenario: 位置の無い写真は端末の位置で埋めない
        val r = build(imageMeta("camera-no-offset.jpg"))
        assertFalse(r.payload.containsKey("lat"))
        assertFalse(r.payload.containsKey("lon"))
    }

    @Test
    fun photo_record_has_location_make_and_model() {
        // Scenario: 写真の記録が撮影位置と機種を持つ
        val r = build(imageMeta("camera-gps.jpg"))
        assertEquals(35.6812, r.payload["lat"]!!.jsonPrimitive.doubleOrNull!!, 1e-9)
        assertEquals(139.7671, r.payload["lon"]!!.jsonPrimitive.doubleOrNull!!, 1e-9)
        assertEquals("AshiatoTest", r.str("make"))
        assertEquals("Fixture One", r.str("model"))
    }

    @Test
    fun video_record_has_duration_and_location() {
        // Scenario: 動画の記録が長さと撮影位置を持つ
        val (lat, lon) = checkNotNull(parseIso6709("+35.6812+139.7671/"))
        val meta = MediaFileMeta(
            kind = MediaKind.VIDEO,
            container = mapOf("DURATION" to "12500", "LOCATION" to "+35.6812+139.7671/"),
            durationMs = 12_500,
            latitude = lat,
            longitude = lon,
        )
        val r = build(meta, row().copy(mimeType = "video/mp4"))
        assertEquals(12_500L, r.payload["duration_ms"]!!.jsonPrimitive.content.toLong())
        assertEquals(35.6812, r.payload["lat"]!!.jsonPrimitive.doubleOrNull!!, 1e-9)
        assertEquals(139.7671, r.payload["lon"]!!.jsonPrimitive.doubleOrNull!!, 1e-9)
        assertEquals("12500", r.rawJson()["container"]!!.jsonObject["DURATION"]!!.jsonPrimitive.content)
    }

    // ---- 内容ハッシュ ---------------------------------------------------------------------------

    @Test
    fun content_hash_is_sha256_of_the_whole_file() {
        // Scenario: 内容ハッシュはファイル全体の SHA-256
        // 期待は `sha256sum collector-android/app/src/test/resources/media/camera-gps.jpg` の値（独立に求めた）
        val expected = "c2d24b1ef0bbde4b0e19c8d85cee19d2db4ad5dd637806dc677c8c08d7c5b762"
        val hash = sha256Hex(ByteArrayInputStream(resource("camera-gps.jpg")))
        assertEquals(expected, hash)
        val r = build(imageMeta("camera-gps.jpg"), sha256 = hash)
        assertEquals(expected, r.str("sha256"))
        assertEquals(expected, r.toIngestRequest("id-1", "u", "d", "c01-photo").externalId)
    }

    @Test
    fun hash_reads_a_stream_larger_than_its_buffer() {
        // バッファ（64 KiB）をまたぐ長さでも全体を流す
        val big = ByteArray(200_000) { (it % 251).toByte() }
        val expected = java.security.MessageDigest.getInstance("SHA-256").digest(big)
            .joinToString("") { "%02x".format(it) }
        assertEquals(expected, sha256Hex(ByteArrayInputStream(big)))
    }

    // ---- 原文 ----------------------------------------------------------------------------------

    @Test
    fun raw_keeps_exif_values_as_the_device_returned_them() {
        // Scenario: 原文に EXIF の欄が端末の返した値のまま入る
        val exif = ExifInterface(ByteArrayInputStream(resource("camera-gps.jpg")))
        val r = build(readImageMeta(exif))
        val inRaw = r.rawJson()["exif"]!!.jsonObject
        for (tag in listOf("DateTimeOriginal", "ExposureTime", "FNumber", "Model", "Make", "OffsetTimeOriginal")) {
            val returned = checkNotNull(exif.getAttribute(tag)) { "材料に $tag が無い" }
            assertEquals(tag, returned, inRaw[tag]!!.jsonPrimitive.content)
        }
        assertEquals("2026:10:17 14:30:12", inRaw["DateTimeOriginal"]!!.jsonPrimitive.content)
    }

    @Test
    fun raw_leaves_out_embedded_thumbnail_and_binary_blobs_and_names_them() {
        // Scenario: 原文に埋め込みの縮小画像が入らない
        val meta = imageMeta("camera-gps.jpg")
        assertTrue("材料に縮小画像が無い", meta.hasThumbnail)
        val raw = build(meta).rawJson()
        val exif = raw["exif"]!!.jsonObject
        val omitted = raw["omitted"]!!.jsonArray.map { it.jsonPrimitive.content }
        assertTrue("ThumbnailImage" in omitted)
        assertTrue("MakerNote" in omitted) // 300 バイトの UNDEFINED
        assertTrue("JPEGInterchangeFormat" in omitted)
        for (name in omitted) assertFalse("$name は値を持たない", exif.containsKey(name))
        assertEquals(omitted.sorted(), omitted)
    }

    @Test
    fun short_binary_tags_stay_in_raw() {
        // 256 バイト以下の UNDEFINED は落とさない
        val r = build(taken(null).copy(exif = mapOf("MakerNote" to "x".repeat(OMIT_BINARY_OVER))))
        assertTrue(r.rawJson()["exif"]!!.jsonObject.containsKey("MakerNote"))
        val over = build(taken(null).copy(exif = mapOf("MakerNote" to "x".repeat(OMIT_BINARY_OVER + 1))))
        assertFalse(over.rawJson()["exif"]!!.jsonObject.containsKey("MakerNote"))
    }

    @Test
    fun long_text_tags_are_never_dropped_from_raw() {
        // Scenario: 文字の欄は長くても原文から落ちない
        val text = "あ".repeat(4_000)
        val r = build(taken(null).copy(exif = mapOf("ImageDescription" to text)))
        val kept = r.rawJson()["exif"]!!.jsonObject["ImageDescription"]!!.jsonPrimitive.content
        assertEquals(4_000, kept.length)
        assertEquals(text, kept)
        assertEquals(JsonArray(emptyList()), r.rawJson()["omitted"])
    }

    @Test
    fun raw_is_the_same_string_whatever_order_the_inputs_arrive_in() {
        // 同じ項目は毎回同じ原文（冪等キーの材料）。入力の並びに依らない
        val a = build(
            taken(null).copy(exif = linkedMapOf("Make" to "m", "Model" to "x")),
            row(extra = linkedMapOf("a" to "1", "b" to null)),
        )
        val b = build(
            taken(null).copy(exif = linkedMapOf("Model" to "x", "Make" to "m")),
            row(extra = linkedMapOf("b" to null, "a" to "1")),
        )
        assertEquals(a.raw, b.raw)
        assertNull(a.rawJson()["store"]!!.jsonObject["b"]?.takeIf { it !is kotlinx.serialization.json.JsonNull })
    }

    @Test
    fun ingest_request_carries_the_record() {
        val r = build(imageMeta("camera-gps.jpg"), sha256 = "b".repeat(64))
        val req = r.toIngestRequest("id-1", "user-1", "dev-1", "c01-photo")
        assertEquals("b".repeat(64), req.externalId)
        assertEquals("c01-photo", req.logicalSource)
        assertEquals("collected", req.origin)
        assertEquals(r.eventTime.toString(), req.eventTime)
        assertEquals(r.raw, req.raw)
        assertNotNull(req.payload["kind"])
        assertEquals(JsonPrimitive("media"), req.payload["kind"])
    }

    @Test
    fun gps_coordinates_read_signs_and_reject_garbage() {
        assertEquals(-33.871333, gpsCoordinate("33/1,52/1,1680/100", "S")!!, 1e-9)
        assertEquals(-122.4194, gpsCoordinate("122/1,25/1,984/100", "W")!!, 1e-9)
        assertNull(gpsCoordinate("33/1,52/1", "N"))
        assertNull(gpsCoordinate("33/0,52/1,1/1", "N"))
        assertNull(gpsCoordinate(null, null))
        assertNull(parseIso6709("nonsense"))
    }
}
