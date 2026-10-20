// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.Manifest
import android.app.Notification
import android.app.NotificationManager
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.provider.MediaStore
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import androidx.test.rule.GrantPermissionRule
import androidx.test.uiautomator.By
import androidx.test.uiautomator.UiDevice
import androidx.test.uiautomator.Until
import kotlinx.serialization.json.jsonPrimitive
import org.junit.After
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import java.io.ByteArrayOutputStream
import java.io.File
import java.io.InputStream
import java.net.ServerSocket
import java.nio.file.Files
import java.time.Instant
import java.time.ZoneId
import java.util.UUID
import java.util.regex.Pattern

/**
 * 写真と動画の権限の求め方と、取れていないことの知らせを**本物の端末（エミュレータ）**で確かめる（ST09 tasks 4.1 / 4.3）。
 * 権限を求める画面は、未許可から始める同じ流れの `PermissionDeniedInstrumentedTest` が見る。
 *
 * 端から端（Task 10.2）: 端末の一覧（MediaStore）にある写真と動画を**本物の読み手**で読み、記録を未送信に積み、
 * 本物の HTTP で端末の中のテスト用の受け手へ送る。受け手には、写真の記録・動画の記録・写真の原本の `PUT` が届き、
 * 動画の原本は問い合わせも `PUT` も届かない。
 * 材料は `tools/android-media-fixtures.sh` が別のアプリとして入れた写真と動画（`MediaPremisesInstrumentedTest` と同じ）。
 * 受け手は `HttpTransportInstrumentedTest` と同じく自前の `ServerSocket`（`MockWebServer` の依存を足さない）。
 * 前景サービス全体は通さない: 送信の刻みが 5 分で、テストから刻みを差し替えられないため。サービスの組み立ては
 * `LocationServiceTest`（Robolectric）が見る。
 */
@RunWith(AndroidJUnit4::class)
class MediaCaptureInstrumentedTest {
    @get:Rule
    val notifications: GrantPermissionRule = GrantPermissionRule.grant(Manifest.permission.POST_NOTIFICATIONS)

    @get:Rule
    val mediaAccess: GrantPermissionRule =
        GrantPermissionRule.grant(*mediaPermissionsToAsk(android.os.Build.VERSION.SDK_INT).toTypedArray())

    private val instrumentation get() = InstrumentationRegistry.getInstrumentation()
    private val context: Context get() = ApplicationProvider.getApplicationContext()
    private val device: UiDevice get() = UiDevice.getInstance(instrumentation)
    private val manager: NotificationManager get() = context.getSystemService(NotificationManager::class.java)

    // Scenario: 写真の位置の権限が無いと端末で知らせる
    @Test
    fun a_notice_is_posted_when_the_photo_location_permission_is_missing_and_opens_the_app_settings() {
        val notifier = MediaNotifier(context)

        notifier.update(MediaPermissionState(MediaAccess.FULL, location = false))

        // 通知は非同期で並ぶので、出るまで少し待つ
        val deadline = System.currentTimeMillis() + NOTICE_WAIT_MS
        var notice = manager.activeNotifications.firstOrNull { it.id == MediaNotifier.MEDIA_NOTIFICATION_ID }
        while (notice == null && System.currentTimeMillis() < deadline) {
            Thread.sleep(250)
            // 付与が通知の側へ行き渡るまで出せないことがある（出せなければ覚えず、次の更新で出し直す）
            notifier.update(MediaPermissionState(MediaAccess.FULL, location = false))
            notice = manager.activeNotifications.firstOrNull { it.id == MediaNotifier.MEDIA_NOTIFICATION_ID }
        }
        assertNotNull("知らせが出ていない", notice)
        assertEquals(MediaNotifier.MEDIA_CHANNEL, notice!!.notification.channelId)
        assertEquals(MEDIA_NOTICE_TITLE, notice.notification.extras.getString(Notification.EXTRA_TITLE))
        // 押すとこのアプリの設定が開く: 本物の PendingIntent を送り、設定アプリが前面に出ることを見る
        notice.notification.contentIntent.send()
        val opened = device.wait(Until.hasObject(By.pkg(Pattern.compile(".*settings.*"))), DIALOG_TIMEOUT_MS)
        assertTrue("知らせを押してもアプリの設定が開かない（前面: ${device.currentPackageName}）", opened == true)
        device.pressBack()

        // 権限が戻ったら消える
        notifier.update(MediaPermissionState(MediaAccess.FULL, location = true))
        assertNull(
            "権限が戻ったのに知らせが残っている",
            manager.activeNotifications.firstOrNull { it.id == MediaNotifier.MEDIA_NOTIFICATION_ID },
        )
    }

    /** 受け取った要求 1 つ。本文はバイトのまま。 */
    private class Received(val method: String, val path: String, val body: ByteArray)

    /** 何度でも受ける小さな受け手。`/ingest` は全部受理、原本は `HEAD` なら 404、`PUT` なら 201。 */
    private class Receiver : AutoCloseable {
        private val socket = ServerSocket(0)
        val port: Int get() = socket.localPort
        val requests: MutableList<Received> = java.util.Collections.synchronizedList(ArrayList())

        init {
            Thread {
                while (!socket.isClosed) {
                    val client = runCatching { socket.accept() }.getOrNull() ?: break
                    Thread { client.use { serve(it) } }.apply {
                        isDaemon = true
                        start()
                    }
                }
            }.apply {
                isDaemon = true
                start()
            }
        }

        private fun serve(client: java.net.Socket) {
            val input = client.getInputStream()
            val head = readHead(input) ?: return
            val lines = head.split("\r\n")
            val (method, path) = lines[0].split(" ").let { it[0] to it[1] }
            val length = lines.drop(1).firstOrNull { it.lowercase().startsWith("content-length:") }
                ?.substringAfter(":")?.trim()?.toInt() ?: 0
            val body = ByteArray(length)
            var read = 0
            while (read < length) {
                val n = input.read(body, read, length - read)
                if (n < 0) break
                read += n
            }
            requests += Received(method, path, body)
            val (status, reply) = when {
                path.startsWith("/ingest") -> {
                    val n = ingestJson.decodeFromString<List<IngestRequest>>(String(body, Charsets.UTF_8)).size
                    200 to List(n) { """{"accepted":true}""" }.joinToString(",", "[", "]")
                }
                method == "HEAD" -> 404 to ""
                else -> 201 to ""
            }
            val bytes = reply.toByteArray(Charsets.UTF_8)
            client.getOutputStream().apply {
                val head = "HTTP/1.1 $status X\r\nContent-Type: application/json\r\n" +
                    "Content-Length: ${bytes.size}\r\nConnection: close\r\n\r\n"
                write(head.toByteArray())
                if (method != "HEAD") write(bytes)
                flush()
            }
        }

        private fun readHead(input: InputStream): String? {
            val out = ByteArrayOutputStream()
            while (true) {
                val b = input.read()
                if (b < 0) return null
                out.write(b)
                val s = out.toByteArray()
                if (endsWithBlankLine(s)) return String(s, 0, s.size - 4, Charsets.ISO_8859_1)
            }
        }

        private fun endsWithBlankLine(s: ByteArray): Boolean =
            s.size >= 4 && String(s, s.size - 4, 4, Charsets.ISO_8859_1) == "\r\n\r\n"

        override fun close() = socket.close()
    }

    /** 端末の一覧から名前で探した項目のバイト（位置を伏せさせない開き方。原本の送り手と同じ）。 */
    private fun deviceBytes(collection: Uri, name: String): ByteArray {
        val id = context.contentResolver.query(
            collection,
            arrayOf(MediaStore.MediaColumns._ID),
            "${MediaStore.MediaColumns.DISPLAY_NAME} = ?",
            arrayOf(name),
            null,
        )?.use { if (it.moveToFirst()) it.getLong(0) else null }
            ?: error("$name が端末の一覧に無い。tools/android-media-fixtures.sh が別のアプリとして入れる")
        val uri = MediaStore.setRequireOriginal(android.content.ContentUris.withAppendedId(collection, id))
        return checkNotNull(context.contentResolver.openInputStream(uri)) { "$name を開けない" }.use { it.readBytes() }
    }

    /** 端から端に要る道具一式（本物の読み手・未送信の置き場・送り手・原本の送り手）。 */
    private inner class Rig(dir: File, baseUrl: String, user: String) {
        val scanLog = StringBuffer()
        private val outbox = Outbox(
            SegmentStore(File(dir, "records"), IngestRequest.serializer(), File(dir, "unreadable.jsonl"), {}),
            age = { 0L },
        )
        private val ledgers = MediaLedgerStore(File(dir, "media"))
        private val pipeline = OriginalPipeline(
            ledgers.originals,
            Outbox(
                SegmentStore(
                    File(dir, "missing"),
                    MediaMissingReport.serializer(),
                    File(dir, "unreadable2.jsonl"),
                    {},
                ),
                age = { 0L },
            ),
            { user },
            "e2e-device",
            {},
        )
        val scanner = MediaScanner(
            listing = ContentResolverMediaListing(context),
            files = ContentResolverMediaFileReader(context),
            outbox = outbox,
            store = ledgers,
            env = MediaScanEnvironment(
                access = { mediaPermissionState(context).forScanner() },
                now = { Instant.now() },
                zone = { ZoneId.systemDefault() },
                userId = { user },
                deviceId = "e2e-device",
                newId = { UUID.randomUUID().toString() },
            ),
            log = { scanLog.append(it).append('\n') },
        )
        val sender = Sender(
            outbox,
            HttpTransport(baseUrl, TOKEN, "/ingest"),
            IngestRequest.serializer(),
            dropPermanentlyRejected = true,
        ).also { it.listener = pipeline.link }
        val uploader = pipeline.uploader(
            ContentResolverOriginalSource(context),
            HttpOriginalTransport(baseUrl, TOKEN) { user },
            OriginalEnvironment(
                access = { mediaPermissionState(context).forScanner() },
                unmetered = { true },
                now = { Instant.now() },
            ),
        ) {}
    }

    // Scenario: 写真を撮るとメタが D-01 に、原本が D-02 に入る
    // Scenario: 動画を撮るとメタだけが入り原本は送られない
    @Test
    fun a_photo_sends_its_record_and_original_and_a_video_sends_only_its_record() {
        val images = MediaStore.Images.Media.getContentUri(MediaStore.VOLUME_EXTERNAL_PRIMARY)
        val videos = MediaStore.Video.Media.getContentUri(MediaStore.VOLUME_EXTERNAL_PRIMARY)
        val photoBytes = deviceBytes(images, PHOTO_NAME)
        val videoBytes = deviceBytes(videos, VIDEO_NAME)
        val photoSha = sha256Hex(photoBytes.inputStream())
        val videoSha = sha256Hex(videoBytes.inputStream())

        val dir = Files.createTempDirectory("media-e2e").toFile()
        Receiver().use { receiver ->
            try {
                val baseUrl = "http://127.0.0.1:${receiver.port}"
                val user = UUID.randomUUID().toString()
                val rig = Rig(dir, baseUrl, user)
                val scanner = rig.scanner
                val sender = rig.sender
                val uploader = rig.uploader
                val scanLog = rig.scanLog

                val scanned = scanner.scan()
                assertFalse("読み手が読めない: $scanned / $scanLog", scanned is MediaScanResult.Unavailable)
                assertTrue("記録が送られる", sender.flush().accepted > 0)
                uploader.run()

                val records = receiver.requests.filter { it.path.startsWith("/ingest") }
                    .flatMap { ingestJson.decodeFromString<List<IngestRequest>>(String(it.body, Charsets.UTF_8)) }
                fun recordOf(name: String) =
                    records.filter { it.payload["display_name"]?.jsonPrimitive?.content == name }
                val photoRecord = recordOf(PHOTO_NAME).single()
                val videoRecord = recordOf(VIDEO_NAME).single()
                // 材料は Pictures/ の下に入る（DCIM/ の外）ので画像のソース（design D3）。原本は写真と同じく送る
                assertEquals("c01-image", photoRecord.logicalSource)
                assertEquals(photoSha, photoRecord.payload["sha256"]?.jsonPrimitive?.content)
                assertEquals("c01-video", videoRecord.logicalSource)
                assertEquals(videoSha, videoRecord.payload["sha256"]?.jsonPrimitive?.content)

                // 写真の原本の PUT が届き、バイトが端末のファイルと一致する
                val photoPut = receiver.requests
                    .single { it.method == "PUT" && it.path.startsWith("/media/originals/$photoSha") }
                assertArrayEquals("原本のバイトが端末のファイルと違う", photoBytes, photoPut.body)
                assertTrue(photoPut.path.contains("user_id=$user"))
                // 動画の原本は問い合わせも PUT も届かない
                assertFalse(
                    "動画の原本について受け口へ何かが届いた",
                    receiver.requests.any { it.path.startsWith("/media/originals/$videoSha") },
                )
            } finally {
                dir.deleteRecursively()
            }
        }
    }

    @After
    fun cleanUp() {
        manager.cancel(MediaNotifier.MEDIA_NOTIFICATION_ID)
        context.stopService(Intent(context, LocationService::class.java))
    }

    private companion object {
        const val DIALOG_TIMEOUT_MS = 15_000L
        const val NOTICE_WAIT_MS = 10_000L
        const val TOKEN = "e2e-token-0123456789abcdef"

        /** `tools/android-media-fixtures.sh` が別のアプリとして入れる名前（`MediaPremisesInstrumentedTest` と同じ） */
        const val PHOTO_NAME = "ashiato-premise-camera.jpg"
        const val VIDEO_NAME = "ashiato-premise-video.mp4"
    }
}
