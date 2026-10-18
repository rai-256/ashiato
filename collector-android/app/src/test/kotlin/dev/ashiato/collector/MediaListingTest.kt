// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.content.ContentProvider
import android.content.ContentValues
import android.database.Cursor
import android.database.MatrixCursor
import android.net.Uri
import android.os.Bundle
import android.os.CancellationSignal
import android.provider.MediaStore
import androidx.test.core.app.ApplicationProvider
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.shadows.ShadowContentResolver

/** 実物の読み口（[ContentResolverMediaListing]）が、一覧の行を [MediaListed] に写す（tasks 3.1）。 */
@RunWith(RobolectricTestRunner::class)
class MediaListingTest {
    private class FakeMediaProvider : ContentProvider() {
        var lastArgs: Bundle? = null
        var lastSelection: String? = null

        class Row(val id: Long, val generation: Long, val values: Array<Any?>)

        val rows = listOf(
            Row(7, 9, arrayOf(7L, 1L, "DCIM/Camera/", "a.jpg", 1234L, 1_700_000_000L, 1_700_000_100L, 9L, 1L, null)),
            Row(8, 10, arrayOf(8L, 3L, "Movies/", "b.mp4", 99L, 1_700_000_000L, 1_700_000_100L, 10L, 0L, 5L)),
        )
        override fun onCreate() = true
        override fun query(uri: Uri, p: Array<String>?, s: String?, a: Array<String>?, o: String?): Cursor? = null
        override fun query(uri: Uri, p: Array<String>?, queryArgs: Bundle?, signal: CancellationSignal?): Cursor {
            lastArgs = queryArgs
            val args = queryArgs ?: Bundle()
            val selection = args.getString(android.content.ContentResolver.QUERY_ARG_SQL_SELECTION).orEmpty()
            val values = args.getStringArray(android.content.ContentResolver.QUERY_ARG_SQL_SELECTION_ARGS).orEmpty()
            lastSelection = selection
            // 実物の selection の形に従って絞る: `_id > ?`（頁送り）・`generation_modified > ?`・`_id = ?`（contains）
            val rows = rows.filter { row ->
                when {
                    selection.startsWith("_id = ?") -> row.id == values[0].toLong()
                    else -> row.id > values[0].toLong() &&
                        (!selection.contains("generation_modified > ?") || row.generation > values[1].toLong())
                }
            }.take(args.getInt(android.content.ContentResolver.QUERY_ARG_LIMIT, Int.MAX_VALUE))
            val cursor = MatrixCursor(
                arrayOf(
                    "_id", "media_type", "relative_path", "_display_name", "_size", "date_added", "date_modified",
                    "generation_modified", "is_trashed", "datetaken",
                ),
            )
            rows.forEach { cursor.addRow(it.values) }
            return cursor
        }
        override fun getType(uri: Uri): String? = null
        override fun insert(uri: Uri, values: ContentValues?): Uri? = null
        override fun delete(uri: Uri, s: String?, a: Array<String>?) = 0
        override fun update(uri: Uri, v: ContentValues?, s: String?, a: Array<String>?) = 0
    }

    @Test
    fun rows_of_images_and_videos_are_mapped_and_trashed_rows_are_asked_for() {
        val provider = FakeMediaProvider()
        ShadowContentResolver.registerProviderInternal(MediaStore.AUTHORITY, provider)
        val listing = ContentResolverMediaListing(ApplicationProvider.getApplicationContext())
        val page = listing.page(PRIMARY, afterId = 0, generationAfter = 5, limit = SCAN_BATCH)
        assertEquals(listOf(7L, 8L), page.map { it.row.mediaId })
        assertEquals(listOf(MediaKind.IMAGE, MediaKind.VIDEO), page.map { it.kind })
        assertTrue(page[0].trashed)
        assertFalse(page[1].trashed)
        assertEquals("DCIM/Camera/", page[0].row.relativePath)
        assertEquals(1234L, page[0].row.sizeBytes)
        assertEquals(9L, page[0].generationModified)
        assertEquals(5L, page[1].row.dateTakenMs)
        assertEquals("a.jpg", page[0].row.columns["_display_name"])
        val args = checkNotNull(provider.lastArgs)
        assertEquals(MediaStore.MATCH_INCLUDE, args.getInt(MediaStore.QUERY_ARG_MATCH_TRASHED))
        assertEquals(SCAN_BATCH, args.getInt(android.content.ContentResolver.QUERY_ARG_LIMIT))
    }

    @Test
    fun paging_follows_the_id_and_generation_the_scan_asks_for() {
        val provider = FakeMediaProvider()
        ShadowContentResolver.registerProviderInternal(MediaStore.AUTHORITY, provider)
        val listing = ContentResolverMediaListing(ApplicationProvider.getApplicationContext())
        assertEquals(listOf(8L), listing.page(PRIMARY, afterId = 7, generationAfter = null, limit = 10).map { it.row.mediaId })
        assertEquals(listOf(8L), listing.page(PRIMARY, afterId = 0, generationAfter = 9, limit = 10).map { it.row.mediaId })
        assertTrue(checkNotNull(provider.lastSelection).contains("generation_modified > ?"))
        assertEquals(listOf(7L), listing.page(PRIMARY, afterId = 0, generationAfter = null, limit = 1).map { it.row.mediaId })
        assertTrue(listing.contains(PRIMARY, 8))
        assertFalse(listing.contains(PRIMARY, 99))
    }
}
