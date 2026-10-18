// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.Manifest
import android.content.ContentUris
import android.content.ContentValues
import android.content.Context
import android.content.pm.PackageManager
import android.graphics.Bitmap
import android.media.ExifInterface
import android.media.MediaMetadataRetriever
import android.net.Uri
import android.os.Bundle
import android.provider.MediaStore
import android.util.Log
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import java.io.ByteArrayOutputStream
import java.io.File
import java.nio.file.Files
import java.time.ZoneId
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Assert.fail
import org.junit.FixMethodOrder
import org.junit.Test
import org.junit.runner.RunWith
import org.junit.runners.MethodSorters

/**
 * **端末の前提を本物の Android で確かめる**（tasks 2.2 / 2.3。premises.md P1 / P3 / P12 / P13）。
 *
 * 写真と動画は**別のアプリが入れたもの**でなければならない —— 位置を伏せるのは他のアプリの項目だけで、
 * 自分が入れた項目は伏せない。なので材料は `adb push` + メディアの再走査で入れる（所有者は `com.android.shell`）。
 * 入れるのは `tools/android-emulator.sh` と CI の `android-instrumented` の台本（[PHOTO_NAME] / [VIDEO_NAME]）。
 * 無ければ**黙って飛ばさず落とす**（前提が確かめられていないのに緑にしない）。
 *
 * **「位置の権限が無い」状態から始まる**ので [NeedsPristinePermissions]（2 段目で走る）。順序は固定する:
 * 1 本目が権限を許可するので、権限が無い状態を見るのは 1 本目だけ。
 */
@NeedsPristinePermissions
@FixMethodOrder(MethodSorters.NAME_ASCENDING)
@RunWith(AndroidJUnit4::class)
class MediaPremisesInstrumentedTest {
    private val context: Context get() = ApplicationProvider.getApplicationContext()
    private val instrumentation get() = InstrumentationRegistry.getInstrumentation()

    private fun grant(permission: String) {
        instrumentation.uiAutomation.grantRuntimePermission(context.packageName, permission)
    }

    private fun held(permission: String) =
        context.checkSelfPermission(permission) == PackageManager.PERMISSION_GRANTED

    /** 観測を計測テストの出力（logcat と instrumentation の status）に残す。位置の値は残さない（有る・無いだけ）。 */
    private fun observe(line: String) {
        Log.i(TAG, line)
        instrumentation.sendStatus(0, Bundle().apply { putString("stream", "$TAG: $line\n") })
    }

    private fun find(collection: Uri, name: String): Uri {
        context.contentResolver.query(
            collection, arrayOf(MediaStore.MediaColumns._ID), "${MediaStore.MediaColumns.DISPLAY_NAME} = ?", arrayOf(name), null,
        )?.use { if (it.moveToFirst()) return ContentUris.withAppendedId(collection, it.getLong(0)) }
        fail("$name が端末の一覧に無い。tools/android-emulator.sh（と CI の台本）が別のアプリとして入れる（adb push + 再走査）")
        error("unreachable")
    }

    private val images: Uri get() = MediaStore.Images.Media.getContentUri(MediaStore.VOLUME_EXTERNAL_PRIMARY)
    private val videos: Uri get() = MediaStore.Video.Media.getContentUri(MediaStore.VOLUME_EXTERNAL_PRIMARY)

    private fun exifOf(uri: Uri): MediaFileMeta =
        checkNotNull(context.contentResolver.openInputStream(uri)) { "開けない" }.use { readImageMeta(ExifInterface(it)) }

    private fun containerOf(uri: Uri): MediaFileMeta {
        val mmr = MediaMetadataRetriever()
        try {
            mmr.setDataSource(context, uri)
            return readVideoMeta(mmr)
        } finally {
            mmr.release()
        }
    }

    // ---- P1 / P12: 位置の権限が無いと、他のアプリの写真と動画の位置は伏せられる -----------------------

    @Test
    fun step1_location_is_hidden_without_the_permission_and_visible_with_it() {
        grant(Manifest.permission.READ_MEDIA_IMAGES)
        runCatching { grant(Manifest.permission.READ_MEDIA_VIDEO) }
        assertFalse("前提: ACCESS_MEDIA_LOCATION は未許可から始まる（2 段目）", held(Manifest.permission.ACCESS_MEDIA_LOCATION))
        val photo = find(images, PHOTO_NAME)

        // (P1) 権限が無い: そのまま開くと位置が無く、伏せない原本を求めると例外になる
        val hidden = exifOf(photo)
        // **伏せられた位置は「欄が消える」のでなく「0 に塗られる」**（実測 2026-10-19, API 35）。緯度経度が 0.0 で読めてしまう
        val hiddenAs = when {
            hidden.latitude == null && hidden.longitude == null -> "absent"
            hidden.latitude == 0.0 && hidden.longitude == 0.0 -> "zeroed"
            else -> "visible"
        }
        assertTrue("権限が無いのに位置が見えた（$hiddenAs）", hiddenAs != "visible")
        assertTrue("位置以外は読める（機種）", hidden.exif["Make"] != null)
        // 例外が出る場所（URI を作るとき / 開くとき）は問わない。どの例外かを残す
        var thrown: String? = null
        try {
            context.contentResolver.openInputStream(MediaStore.setRequireOriginal(photo))?.close()
        } catch (e: UnsupportedOperationException) {
            thrown = e.javaClass.simpleName
        } catch (e: SecurityException) {
            thrown = e.javaClass.simpleName
        }
        assertNotNull("setRequireOriginal が例外にならなかった", thrown)
        observe("P1 without_permission photo_location=hidden($hiddenAs) set_require_original=$thrown")

        // (P12) 動画の入れ物の位置（結果を残すだけ。どちらでも落とさない）
        val videoWithout = observeVideo("without_permission")

        grant(Manifest.permission.ACCESS_MEDIA_LOCATION)
        assertTrue(held(Manifest.permission.ACCESS_MEDIA_LOCATION))

        // (P1) 権限がある: 伏せない原本を開くと位置が見える
        val shown = exifOf(MediaStore.setRequireOriginal(photo))
        assertEquals(35.6812, checkNotNull(shown.latitude), 1e-4)
        assertEquals(139.7671, checkNotNull(shown.longitude), 1e-4)
        observe("P1 with_permission photo_location=visible")
        val videoWith = observeVideo("with_permission")
        // 動画の入れ物の読み方（readVideoMeta）が長さと位置を拾う
        val video = containerOf(find(videos, VIDEO_NAME))
        assertEquals(12_500L, video.durationMs)
        assertEquals(35.6812, checkNotNull(video.latitude), 1e-4)
        assertEquals(139.7671, checkNotNull(video.longitude), 1e-4)
        observe("P12 summary without=$videoWithout with=$videoWith")
    }

    /** 動画の入れ物の `LOCATION` が見えたかを、権限の状態つきで残す。動画が一覧に無い・開けないときは理由を残す。 */
    private fun observeVideo(state: String): String {
        val result = try {
            val uri = find(videos, VIDEO_NAME)
            val container = containerOf(uri)
            if (container.container["LOCATION"] != null) "location=visible" else "location=hidden"
        } catch (e: AssertionError) {
            "not_in_media_store"
        } catch (e: RuntimeException) {
            "unreadable:${e.javaClass.simpleName}"
        }
        observe("P12 $state video_$result")
        return result
    }

    // ---- P3: 写真を足すと世代が上がり、GENERATION_MODIFIED で拾える ---------------------------------

    @Test
    fun step2_adding_a_photo_raises_the_generation_and_is_found_by_it() {
        grant(Manifest.permission.READ_MEDIA_IMAGES)
        val before = MediaStore.getGeneration(context, MediaStore.VOLUME_EXTERNAL_PRIMARY)
        val inserted = insertOwnPhoto("ashiato-premise-gen-${System.nanoTime()}.jpg")
        try {
            val after = MediaStore.getGeneration(context, MediaStore.VOLUME_EXTERNAL_PRIMARY)
            assertTrue("世代が上がらない: $before -> $after", after > before)
            val found = mutableListOf<Long>()
            context.contentResolver.query(
                images, arrayOf(MediaStore.MediaColumns._ID), "${MediaStore.MediaColumns.GENERATION_MODIFIED} > ?",
                arrayOf(before.toString()), null,
            )?.use { while (it.moveToNext()) found += it.getLong(0) }
            assertTrue("GENERATION_MODIFIED で拾えない", ContentUris.parseId(inserted) in found)
            observe("P3 generation_before_after_raised=true found_by_generation_modified=true")
        } finally {
            context.contentResolver.delete(inserted, null, null)
        }
    }

    private fun insertOwnPhoto(name: String): Uri {
        val bytes = ByteArrayOutputStream().also {
            Bitmap.createBitmap(8, 8, Bitmap.Config.ARGB_8888).compress(Bitmap.CompressFormat.JPEG, 80, it)
        }.toByteArray()
        val values = ContentValues().apply {
            put(MediaStore.MediaColumns.DISPLAY_NAME, name)
            put(MediaStore.MediaColumns.MIME_TYPE, "image/jpeg")
            put(MediaStore.MediaColumns.RELATIVE_PATH, "Pictures/ashiato-premise")
        }
        val uri = checkNotNull(context.contentResolver.insert(images, values)) { "挿入できない" }
        context.contentResolver.openOutputStream(uri)!!.use { it.write(bytes) }
        return uri
    }

    // ---- P13: 本物の写真 1 枚の記録の大きさ（門。下限の判定は GT MediaRecordSizeTest） ---------------

    @Test
    fun step3_record_of_the_real_photo_fits_the_gate() {
        grant(Manifest.permission.READ_MEDIA_IMAGES)
        grant(Manifest.permission.ACCESS_MEDIA_LOCATION)
        val uri = find(images, PHOTO_NAME)
        val row = rowOf(uri)
        val original = MediaStore.setRequireOriginal(uri)
        val meta = checkNotNull(context.contentResolver.openInputStream(original)).use { readImageMeta(ExifInterface(it)) }
        val sha = checkNotNull(context.contentResolver.openInputStream(original)).use { sha256Hex(it) }
        val request = buildMediaRecord(row, meta, sha, ZoneId.systemDefault())
            .toIngestRequest("00000000-0000-4000-8000-000000000000", "user-1", "device-1", "c01-photo")
        assertNotNull(request.payload["lat"])

        val dir = Files.createTempDirectory("media-size").toFile()
        try {
            val store = SegmentStore(File(dir, "records"), IngestRequest.serializer(), File(dir, "unreadable.jsonl"), {})
            assertTrue(store.append(request, 0L))
            val bytes = File(dir, "records").walkTopDown().filter { it.isFile && it.name.endsWith(".jsonl") }.sumOf { it.length() }
            observe("P13 unsent_line_bytes=$bytes columns=${row.columns.size} exif_tags=${meta.exif.size}")
            assertTrue("未送信の 1 行が $bytes バイト（門 $RECORD_SIZE_GATE）", bytes <= RECORD_SIZE_GATE)
        } finally {
            dir.deleteRecursively()
        }
    }

    /** 端末の一覧の 1 行を、列の全部を文字列にして取る。 */
    private fun rowOf(uri: Uri): MediaStoreRow {
        context.contentResolver.query(uri, null, null, null, null)!!.use { c ->
            check(c.moveToFirst())
            val columns = linkedMapOf<String, String?>()
            for (i in 0 until c.columnCount) {
                columns[c.getColumnName(i)] = when (c.getType(i)) {
                    android.database.Cursor.FIELD_TYPE_NULL -> null
                    android.database.Cursor.FIELD_TYPE_BLOB -> "blob:${c.getBlob(i).size}"
                    else -> c.getString(i)
                }
            }
            fun long(name: String) = columns[name]?.toLongOrNull()
            return MediaStoreRow(
                mediaId = checkNotNull(long("_id")),
                volume = columns["volume_name"] ?: MediaStore.VOLUME_EXTERNAL_PRIMARY,
                dateTakenMs = long("datetaken"),
                dateAddedSec = checkNotNull(long("date_added")),
                mimeType = columns["mime_type"],
                sizeBytes = long("_size"),
                width = long("width")?.toInt(),
                height = long("height")?.toInt(),
                relativePath = columns["relative_path"],
                displayName = columns["_display_name"],
                columns = columns,
            )
        }
    }

    private companion object {
        const val TAG = "MediaPremises"

        /** 台本が `adb push` + 再走査で入れる名前（所有者は shell）。`tools/android-emulator.sh` / CI と同じ */
        const val PHOTO_NAME = "ashiato-premise-camera.jpg"
        const val VIDEO_NAME = "ashiato-premise-video.mp4"

        /** 20 KiB（design D20）。判定は `MediaRecordSizeTest` */
        const val RECORD_SIZE_GATE: Long = 20 * 1024
    }
}
