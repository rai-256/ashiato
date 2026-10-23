// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.content.ContentResolver
import android.content.ContentUris
import android.content.Context
import android.database.ContentObserver
import android.database.Cursor
import android.media.ExifInterface
import android.media.MediaMetadataRetriever
import android.net.Uri
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.provider.MediaStore

/** 写真と動画へのアクセスの状態（Q15 の入口の判定は tasks 4）。 */
enum class MediaAccess {
    /** 写真と動画の全部 */
    FULL,

    /** 「選んだ写真だけ」。一覧は選んだ分しか見えないので、消えたかの判定はしない（D5） */
    PARTIAL,

    /** 読めない */
    NONE,

    /** 写真の位置の権限が無い: ファイルを開かない（Q15。開くと位置が 0.0 に塗られる） */
    NO_LOCATION,
}

/** 端末の一覧（`MediaStore`）の 1 行。記録が使う分は [row]、台帳が見る分はその外の欄。 */
data class MediaListed(
    val row: MediaStoreRow,
    val kind: MediaKind,
    val dateModifiedSec: Long,
    val generationModified: Long,
    val trashed: Boolean,
)

/** 端末の一覧を読む口。実物は [ContentResolverMediaListing]。試験は偽物を渡す。 */
interface MediaListing {
    /** つながっている置き場（`MediaStore.getExternalVolumeNames`） */
    fun volumes(): Set<String>

    /** 一覧の版（`MediaStore.getVersion`）。番号が振り直されると変わる */
    fun version(volume: String): String?

    /** 世代の数（`MediaStore.getGeneration`）。変わった行があると上がる */
    fun generation(volume: String): Long

    /**
     * 番号が [afterId] より大きい行を番号順に [limit] 件まで。[generationAfter] があれば
     * `GENERATION_MODIFIED` がそれより大きい行だけ。**ごみ箱の行も含める**（D5）。
     */
    fun page(volume: String, afterId: Long, generationAfter: Long?, limit: Int): List<MediaListed>

    /** その番号の行がいま一覧にあるか（ごみ箱を含む） */
    fun contains(volume: String, mediaId: Long): Boolean

    /** 画像と動画の表に知らせを張る。外すときは戻り値を呼ぶ。偽物は何もしない（既定）。 */
    fun observe(onChange: () -> Unit): () -> Unit = {}
}

/** ファイルを開いて読んだ結果。 */
class MediaFileContent(val sha256: String, val meta: MediaFileMeta)

/** ファイルを 1 件読んだ結果。 */
sealed interface MediaFileAttempt {
    class Opened(val content: MediaFileContent) : MediaFileAttempt

    class Failed(val stage: MediaStage) : MediaFileAttempt
}

/** ファイルを開く口。**位置の権限が無い間は呼ばない**（Q15。呼び手が門を持つ）。読めなければ null。 */
interface MediaFileReader {
    fun read(item: MediaListed): MediaFileContent?

    /**
     * 読んだ結果を、失敗した段つきで返す（C19）。既定は [read] の null を「開けない」とする。
     * 実物は開けない（[MediaStage.READ]）と、開けたのにハッシュを計算できない（[MediaStage.HASH]）を分ける。
     */
    fun attempt(item: MediaListed): MediaFileAttempt =
        read(item)?.let { MediaFileAttempt.Opened(it) } ?: MediaFileAttempt.Failed(MediaStage.READ)
}

private const val MEDIA_TYPE_IMAGE = MediaStore.Files.FileColumns.MEDIA_TYPE_IMAGE
private const val MEDIA_TYPE_VIDEO = MediaStore.Files.FileColumns.MEDIA_TYPE_VIDEO

/** `MediaStore` の実物。画像と動画をまとめて 1 つの表（Files）から読む。 */
class ContentResolverMediaListing(private val context: Context) : MediaListing {
    private val resolver: ContentResolver get() = context.contentResolver

    override fun volumes(): Set<String> = MediaStore.getExternalVolumeNames(context)

    override fun version(volume: String): String? = MediaStore.getVersion(context, volume)

    override fun generation(volume: String): Long = MediaStore.getGeneration(context, volume)

    override fun page(volume: String, afterId: Long, generationAfter: Long?, limit: Int): List<MediaListed> {
        val args = mutableListOf(afterId.toString())
        var selection = "${MediaStore.Files.FileColumns.MEDIA_TYPE} IN ($MEDIA_TYPE_IMAGE, $MEDIA_TYPE_VIDEO) " +
            "AND ${MediaStore.Files.FileColumns._ID} > ?"
        if (generationAfter != null) {
            selection += " AND ${MediaStore.Files.FileColumns.GENERATION_MODIFIED} > ?"
            args += generationAfter.toString()
        }
        val query = Bundle().apply {
            putString(ContentResolver.QUERY_ARG_SQL_SELECTION, selection)
            putStringArray(ContentResolver.QUERY_ARG_SQL_SELECTION_ARGS, args.toTypedArray())
            putString(ContentResolver.QUERY_ARG_SQL_SORT_ORDER, "${MediaStore.Files.FileColumns._ID} ASC")
            putInt(ContentResolver.QUERY_ARG_LIMIT, limit)
            putInt(MediaStore.QUERY_ARG_MATCH_TRASHED, MediaStore.MATCH_INCLUDE)
        }
        val out = mutableListOf<MediaListed>()
        resolver.query(MediaStore.Files.getContentUri(volume), null, query, null)?.use { c ->
            while (c.moveToNext()) out += listedOf(volume, c)
        }
        return out
    }

    override fun contains(volume: String, mediaId: Long): Boolean {
        val query = Bundle().apply {
            putString(ContentResolver.QUERY_ARG_SQL_SELECTION, "${MediaStore.Files.FileColumns._ID} = ?")
            putStringArray(ContentResolver.QUERY_ARG_SQL_SELECTION_ARGS, arrayOf(mediaId.toString()))
            putInt(MediaStore.QUERY_ARG_MATCH_TRASHED, MediaStore.MATCH_INCLUDE)
        }
        val projection = arrayOf(MediaStore.Files.FileColumns._ID)
        return resolver.query(MediaStore.Files.getContentUri(volume), projection, query, null)
            ?.use { it.moveToFirst() } ?: false
    }

    /** 画像と動画の表に知らせを張る。外すときは戻り値を呼ぶ。 */
    override fun observe(onChange: () -> Unit): () -> Unit {
        val observer = object : ContentObserver(Handler(Looper.getMainLooper())) {
            override fun onChange(selfChange: Boolean) = onChange()
        }
        val uris = listOf(MediaStore.Images.Media.EXTERNAL_CONTENT_URI, MediaStore.Video.Media.EXTERNAL_CONTENT_URI)
        uris.forEach { resolver.registerContentObserver(it, true, observer) }
        return { resolver.unregisterContentObserver(observer) }
    }

    private fun listedOf(volume: String, c: Cursor): MediaListed {
        val columns = LinkedHashMap<String, String?>()
        for (i in 0 until c.columnCount) {
            // 値が BLOB の列（本物の端末の一覧にある）は `getString` で落ちる。中身は載せず大きさだけ（計測テストの rowOf と同じ）
            columns[c.getColumnName(i)] = when (c.getType(i)) {
                Cursor.FIELD_TYPE_NULL -> null
                Cursor.FIELD_TYPE_BLOB -> "blob:${c.getBlob(i).size}"
                else -> c.getString(i)
            }
        }
        fun long(name: String): Long? = columns[name]?.toLongOrNull()
        val kind = if (long("media_type") == MEDIA_TYPE_VIDEO.toLong()) MediaKind.VIDEO else MediaKind.IMAGE
        val row = MediaStoreRow(
            mediaId = checkNotNull(long("_id")),
            volume = volume,
            dateTakenMs = long("datetaken"),
            dateAddedSec = long("date_added") ?: 0L,
            mimeType = columns["mime_type"],
            sizeBytes = long("_size"),
            width = long("width")?.toInt(),
            height = long("height")?.toInt(),
            relativePath = columns["relative_path"],
            displayName = columns["_display_name"],
            columns = columns,
        )
        return MediaListed(
            row = row,
            kind = kind,
            dateModifiedSec = long("date_modified") ?: 0L,
            generationModified = long("generation_modified") ?: 0L,
            trashed = long("is_trashed") == 1L,
        )
    }
}

/**
 * ファイルを実際に開く（ハッシュと、`ExifInterface` / `MediaMetadataRetriever` のメタ）。
 * 位置を伏せさせないため `setRequireOriginal` で開く —— 権限が無いと開けないので、
 * **呼ぶのは位置の権限があるときだけ**（Q15。[MediaScanner] が門を持つ）。
 */
class ContentResolverMediaFileReader(private val context: Context) : MediaFileReader {
    override fun read(item: MediaListed): MediaFileContent? = (attempt(item) as? MediaFileAttempt.Opened)?.content

    override fun attempt(item: MediaListed): MediaFileAttempt = try {
        open(item)
    } catch (_: java.io.IOException) {
        MediaFileAttempt.Failed(MediaStage.READ)
    } catch (_: IllegalArgumentException) {
        MediaFileAttempt.Failed(MediaStage.READ)
    } catch (_: SecurityException) {
        MediaFileAttempt.Failed(MediaStage.READ)
    } catch (_: UnsupportedOperationException) {
        MediaFileAttempt.Failed(MediaStage.READ)
    }

    private fun open(item: MediaListed): MediaFileAttempt {
        val base = if (item.kind == MediaKind.VIDEO) {
            MediaStore.Video.Media.getContentUri(item.row.volume)
        } else {
            MediaStore.Images.Media.getContentUri(item.row.volume)
        }
        val uri: Uri = MediaStore.setRequireOriginal(ContentUris.withAppendedId(base, item.row.mediaId))
        // 開けない（READ）と、開けたのに計算できない（HASH）は別の段（C19）
        val stream = context.contentResolver.openInputStream(uri) ?: return MediaFileAttempt.Failed(MediaStage.READ)
        val sha = try {
            stream.use { sha256Hex(it) }
        } catch (_: java.io.IOException) {
            null
        }
        val meta = if (sha == null) null else metaOf(item.kind, uri)
        return when {
            sha == null -> MediaFileAttempt.Failed(MediaStage.HASH)
            meta == null -> MediaFileAttempt.Failed(MediaStage.READ)
            else -> MediaFileAttempt.Opened(MediaFileContent(sha, meta))
        }
    }

    private fun metaOf(kind: MediaKind, uri: Uri): MediaFileMeta? =
        if (kind == MediaKind.VIDEO) {
            val retriever = MediaMetadataRetriever()
            try {
                retriever.setDataSource(context, uri)
                readVideoMeta(retriever)
            } finally {
                retriever.release()
            }
        } else {
            context.contentResolver.openInputStream(uri)?.use { readImageMeta(ExifInterface(it)) }
        }
}
