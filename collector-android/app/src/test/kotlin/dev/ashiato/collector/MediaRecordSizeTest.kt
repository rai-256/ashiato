// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.media.ExifInterface
import java.io.ByteArrayInputStream
import java.io.File
import java.nio.file.Files
import java.time.ZoneId
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

/**
 * **記録の大きさの門**（tasks 2.3 / design D20 / premises.md P13）。
 *
 * 記録 1 件を未送信の置き場（`SegmentStore`）に積んだ 1 行の大きさが [RECORD_SIZE_GATE] 以下であること。
 * 1 万件 × 1 行が 2 GB の 1 割（200 MB）を超えると、過去の写真をまとめて積んだだけで位置の記録を押し出す。
 *
 * **この試験が落ちたら Task 3 以降に進まない。** 原文の上限（design D4）を本人に返す —— 原文を削ると動画のメタは戻らない。
 */
@RunWith(RobolectricTestRunner::class)
class MediaRecordSizeTest {
    private fun resource(name: String): ByteArray =
        checkNotNull(javaClass.getResourceAsStream("/media/$name")) { "材料が無い: $name" }.use { it.readBytes() }

    /** 実物の `SegmentStore` に 1 件積み、置き場のファイルの大きさを測る（`{"enq":…,"item":…}` の枠と改行を含む）。 */
    private fun unsentLineBytes(request: IngestRequest): Long {
        val dir = Files.createTempDirectory("media-size").toFile()
        try {
            val store = SegmentStore(File(dir, "records"), IngestRequest.serializer(), File(dir, "unreadable.jsonl"), {})
            assertTrue(store.append(request, 0L))
            return File(dir, "records").walkTopDown().filter { it.isFile && it.name.endsWith(".jsonl") }.sumOf { it.length() }
        } finally {
            dir.deleteRecursively()
        }
    }

    private fun requestOf(meta: MediaFileMeta, row: MediaStoreRow): IngestRequest =
        buildMediaRecord(row, meta, "a".repeat(64), ZoneId.of("Asia/Tokyo"))
            .toIngestRequest("00000000-0000-4000-8000-000000000000", "user-1", "device-1", "c01-photo")

    /** 端末の一覧（`MediaStore.Images`）の列を、実機に近い数と長さで埋めた行。 */
    private fun fullRow(columns: Int): MediaStoreRow = MediaStoreRow(
        mediaId = 1234,
        volume = "external_primary",
        dateTakenMs = 1_760_000_000_000,
        dateAddedSec = 1_760_000_000,
        mimeType = "image/jpeg",
        sizeBytes = 3_145_728,
        width = 4000,
        height = 3000,
        relativePath = "DCIM/Camera/",
        displayName = "PXL_20261017_053012345.jpg",
        columns = (0 until columns).associate { "column_%02d".format(it) to "value-".padEnd(40, 'v') },
    )

    @Test
    fun record_of_a_camera_photo_fits_the_gate() {
        // 材料の写真（カメラの形: 機種・露出・位置・MakerNote・縮小画像）+ 実機に近い数の列
        val meta = readImageMeta(ExifInterface(ByteArrayInputStream(resource("camera-gps.jpg"))))
        val bytes = unsentLineBytes(requestOf(meta, fullRow(columns = 50)))
        assertTrue("未送信の 1 行が $bytes バイト（門 $RECORD_SIZE_GATE）", bytes <= RECORD_SIZE_GATE)
    }

    @Test
    fun record_with_every_exif_tag_filled_fits_the_gate() {
        // 上限の見積もり: framework が名前を持つ EXIF のタグを**全部**、40 文字の値で埋める。
        // バイナリ（MakerNote など）は 1 KiB 与える —— 落とされるので大きさに響かない
        val names = ExifInterface::class.java.fields
            .filter { it.name.startsWith("TAG_") && it.type == String::class.java }
            .map { it.get(null) as String }
            .distinct()
        val exif = names.associateWith { "value-".padEnd(40, 'v') } + ("MakerNote" to "m".repeat(1024)) +
            ("Xmp" to "x".repeat(8192))
        val meta = MediaFileMeta(MediaKind.IMAGE, exif = exif, takenAtRaw = "2026:10:17 14:30:12", offsetRaw = "+09:00")
        val bytes = unsentLineBytes(requestOf(meta, fullRow(columns = 50)))
        assertTrue("未送信の 1 行が $bytes バイト（門 $RECORD_SIZE_GATE）", bytes <= RECORD_SIZE_GATE)
    }

    @Test
    fun gate_is_one_tenth_of_the_two_gigabyte_store_at_ten_thousand_records() {
        assertEquals(20 * 1024L, RECORD_SIZE_GATE)
        val twoGb = 2L * 1024 * 1024 * 1024
        assertTrue(10_000 * RECORD_SIZE_GATE <= twoGb / 10)
    }

    private companion object {
        /** 20 KiB（design D20）。1 万件で 200 MB = 2 GB の 1 割 */
        const val RECORD_SIZE_GATE: Long = 20 * 1024
    }
}
