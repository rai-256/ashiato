// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.Manifest
import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.content.ContentValues
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.graphics.Bitmap
import android.graphics.Color
import android.media.ExifInterface
import android.provider.MediaStore
import android.widget.Toast
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import androidx.test.uiautomator.By
import androidx.test.uiautomator.Direction
import androidx.test.uiautomator.UiDevice
import androidx.test.uiautomator.Until
import java.io.File
import java.util.regex.Pattern
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.jsonPrimitive
import org.junit.After
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Assume.assumeTrue
import org.junit.Test
import org.junit.runner.RunWith

/**
 * 録画専用（人間が後から動画で見る）。ST09「写真と動画をメタごと取り込む」の端末の側を 1 本の流れで撮る:
 * 初めて起動 → 位置・通知の後に**写真と動画と写真の位置をまとめて 1 度だけ**求める画面 → 許可しない →
 * それでも収集は始まり、「写真と動画を取れていません」の知らせが出る（Q15）→ 知らせを押すとこのアプリの設定 →
 * 写真と動画を許可 → 開き直すと知らせが消える → 撮った写真の記録（メタと撮影位置）が未送信に積まれる（結果の札）。
 *
 * 位置は断る（位置の許可の流れは ST06 の録画が撮る）。写真は**この録画が端末の一覧に入れる 1 枚**（撮影位置つき）。
 * 自分が入れた項目なので OS は位置を伏せない —— 他のアプリの写真の位置が伏せられないことは計測テスト
 * （`MediaPremisesInstrumentedTest`）が持つ。送信は 5 分ごとなので、動画の間にサーバへは届かない
 * （届くことは `MediaCaptureInstrumentedTest` と `tools/smoke.sh` が持つ）。
 *
 * **録画のときだけ走る**（`-e recording 1`）。入れたばかりの状態から始める（record-run が入れ直してから起動する）。
 * 結果の札は録画のテストが出す通知（「【録画の結果】」）で、中身はアプリの未送信の置き場を読んだもの。
 */
@RunWith(AndroidJUnit4::class)
class St09RecordingTest {
    private val instrumentation get() = InstrumentationRegistry.getInstrumentation()
    private val context: Context get() = ApplicationProvider.getApplicationContext()
    private val device: UiDevice get() = UiDevice.getInstance(instrumentation)
    private val manager: NotificationManager get() = context.getSystemService(NotificationManager::class.java)

    @After
    fun cleanUp() {
        context.contentResolver.delete(
            MediaStore.Images.Media.getContentUri(MediaStore.VOLUME_EXTERNAL_PRIMARY),
            "${MediaStore.MediaColumns.DISPLAY_NAME} = ?",
            arrayOf(PHOTO_NAME),
        )
        manager.cancel(CARD_ID)
    }

    @Test
    fun mediaCaptureForRecording() {
        assumeTrue("録画のときだけ走る（-e recording 1）", InstrumentationRegistry.getArguments().getString("recording") == "1")
        assertFalse(
            "入れたばかりの状態から始める（写真と動画がもう許可されている）",
            mediaPermissionsToAsk(android.os.Build.VERSION.SDK_INT).any { granted(it) },
        )

        step("1. あしあと。を初めて起動する")
        device.pressHome()
        launchApp()

        step("2. 位置: 許可しない（位置の流れは ST06 の録画）")
        click(DENY)

        step("3. 通知の送信は許可")
        click("^(許可|Allow)$")

        step("4. 写真と動画（と写真の位置）をまとめて 1 度だけ求める → 許可しない")
        assertNotNull(
            "写真と動画の権限の画面が出ない（前面: ${device.currentPackageName}）",
            device.wait(Until.findObject(By.res(Pattern.compile(".*permissioncontroller:id/permission_message"))), TIMEOUT_MS),
        )
        hold(3_000)
        click(DENY)

        step("5. 利用状況へのアクセスの画面は戻る（ST06）")
        assertTrue("利用状況へのアクセスの画面が開かない", device.wait(Until.hasObject(By.pkg(SETTINGS_PACKAGE)), TIMEOUT_MS))
        hold()
        device.pressBack()
        device.pressHome()

        step("6. 収集は始まり、写真と動画を取れていないことを知らせる")
        assertTrue("知らせが出ない（写真と動画を断ったのに）", waitUntil { notice() != null })
        device.openNotification()
        val shown = device.wait(Until.findObject(By.text(MEDIA_NOTICE_TITLE)), TIMEOUT_MS)
        assertNotNull("知らせが通知の一覧に出ない", shown)
        assertNotNull(
            "知らせの本文が違う",
            device.wait(Until.findObject(By.textContains("写真と動画へのアクセスを許可するまで")), TIMEOUT_MS),
        )
        hold(4_000)

        step("7. 知らせを押すと、このアプリの設定が開く → 写真と動画を許可")
        shown.click()
        assertTrue("知らせからアプリの設定が開かない", device.wait(Until.hasObject(By.pkg(SETTINGS_PACKAGE)), TIMEOUT_MS))
        click("^(権限|Permissions)$")
        click("^(写真と動画|Photos and videos)$")
        click("常にすべて許可|すべて許可|Allow all")
        assertTrue(
            "写真と動画と写真の位置が許可にならない",
            waitUntil { mediaPermissionState(context).let { it.access == MediaAccess.FULL && it.location } },
        )
        hold()
        device.pressBack()
        device.pressBack()
        device.pressHome()

        step("8. 開き直す → 知らせは消える")
        launchApp()
        // 位置はまた断る（2 度目の起動でも求める）
        device.wait(Until.findObject(By.text(Pattern.compile(DENY))), 5_000)?.let { hold(); it.click() }
        assertTrue("権限が戻ったのに知らせが残っている", waitUntil { notice() == null })
        device.pressHome()
        device.openNotification()
        assertTrue("知らせが通知の一覧に残っている", device.wait(Until.gone(By.text(MEDIA_NOTICE_TITLE)), TIMEOUT_MS))
        hold(3_000)
        device.pressBack()

        step("9. 写真を 1 枚撮った（撮影位置つき）→ 保存の知らせから記録が積まれる")
        insertPhoto()
        assertTrue(
            "写真の記録が未送信に積まれない",
            waitUntil(45_000) { photos().isNotEmpty() },
        )
        showCard("【録画の結果】写真の記録（未送信）", summary())
    }

    private fun launchApp() =
        context.startActivity(Intent(context, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))

    private fun notice() = manager.activeNotifications.firstOrNull { it.id == MediaNotifier.MEDIA_NOTIFICATION_ID }

    /** 端末の一覧へ写真を 1 枚入れる（カメラの代わり）。撮影日時と撮影位置（東京駅）を EXIF に持つ */
    private fun insertPhoto() {
        val resolver = context.contentResolver
        val uri = checkNotNull(
            resolver.insert(
                MediaStore.Images.Media.getContentUri(MediaStore.VOLUME_EXTERNAL_PRIMARY),
                ContentValues().apply {
                    put(MediaStore.MediaColumns.DISPLAY_NAME, PHOTO_NAME)
                    put(MediaStore.MediaColumns.MIME_TYPE, "image/jpeg")
                    put(MediaStore.MediaColumns.RELATIVE_PATH, "DCIM/Camera")
                    put(MediaStore.MediaColumns.IS_PENDING, 1)
                },
            ),
        ) { "写真を端末の一覧に入れられない" }
        resolver.openOutputStream(uri)!!.use {
            Bitmap.createBitmap(640, 480, Bitmap.Config.ARGB_8888).apply { eraseColor(Color.rgb(120, 170, 90)) }
                .compress(Bitmap.CompressFormat.JPEG, 90, it)
        }
        resolver.openFileDescriptor(uri, "rw")!!.use { fd ->
            ExifInterface(fd.fileDescriptor).apply {
                setAttribute(ExifInterface.TAG_DATETIME_ORIGINAL, "2026:10:21 09:30:00")
                setAttribute(ExifInterface.TAG_OFFSET_TIME_ORIGINAL, "+09:00")
                setAttribute(ExifInterface.TAG_MAKE, "ashiato")
                setAttribute(ExifInterface.TAG_MODEL, "recording")
                // 東京駅（35°40'52.32" N, 139°46'1.56" E）。android.media.ExifInterface には setLatLong が無い
                setAttribute(ExifInterface.TAG_GPS_LATITUDE, "35/1,40/1,5232/100")
                setAttribute(ExifInterface.TAG_GPS_LATITUDE_REF, "N")
                setAttribute(ExifInterface.TAG_GPS_LONGITUDE, "139/1,46/1,156/100")
                setAttribute(ExifInterface.TAG_GPS_LONGITUDE_REF, "E")
                saveAttributes()
            }
        }
        resolver.update(uri, ContentValues().apply { put(MediaStore.MediaColumns.IS_PENDING, 0) }, null, null)
    }

    /** 未送信の置き場を読む（アプリと同じ置き場・同じ形。St06 の録画と同じ読み方） */
    private fun outbox(): List<IngestRequest> {
        val dir = File(context.filesDir, LocationService.OUTBOX_DIR)
        return SegmentStore(File(dir, "records"), IngestRequest.serializer(), File(dir, LocationService.UNREADABLE), {}).readAll()
    }

    private fun photos() = outbox().filter {
        it.logicalSource == SourceCadence.PHOTO.logicalSource && it.payload["display_name"]?.jsonPrimitive?.content == PHOTO_NAME
    }

    private fun summary(): List<String> {
        val p = photos().last().payload
        fun v(key: String) = p[key]?.takeUnless { it is JsonNull }?.jsonPrimitive?.content ?: "なし"
        return listOf(
            "写真: ${v("display_name")}（${v("relative_path")}・${v("width")}×${v("height")}）",
            "撮影時刻: ${v("taken_at_exif")} ${v("offset_exif")}（時刻の元: ${v("time_basis")}）",
            "撮影位置: ${v("lat")}, ${v("lon")}",
            "機種: ${v("make")} ${v("model")} / ハッシュ: ${v("sha256").take(12)}…",
            "記録の種類ごとの件数: " + outbox().groupingBy { it.logicalSource }.eachCount().entries.joinToString(" / ") { "${it.key} ${it.value}" },
            "送信は 5 分ごと（この動画の間にはサーバへ送らない）",
        )
    }

    /** 結果の札を通知で出し、通知の一覧を開いて見せる（アプリには画面が無いので。St06 の録画と同じ出し方） */
    private fun showCard(title: String, lines: List<String>) {
        manager.createNotificationChannel(NotificationChannel(CARD_CHANNEL, "録画の結果", NotificationManager.IMPORTANCE_HIGH))
        manager.notify(
            CARD_ID,
            Notification.Builder(context, CARD_CHANNEL)
                .setSmallIcon(android.R.drawable.ic_dialog_info)
                .setContentTitle(title)
                .setContentText(lines.first())
                .setGroup(CARD_CHANNEL)
                .setStyle(Notification.BigTextStyle().bigText(lines.joinToString("\n")))
                .build(),
        )
        device.openNotification()
        assertNotNull("結果の札が通知の一覧に出ない", device.wait(Until.findObject(By.text(title)), TIMEOUT_MS))
        val lastLine = By.textContains(lines.last().take(12))
        if (!device.hasObject(lastLine)) {
            device.findObject(By.text(title))?.swipe(Direction.DOWN, 0.8f)
            if (!device.wait(Until.hasObject(lastLine), 3_000)) {
                var node = device.findObject(By.text(title))
                var depth = 0
                while (node != null && depth < 4) {
                    val expand = node.findObject(By.res("android:id/expand_button"))
                    if (expand != null) { expand.click(); break }
                    node = node.parent; depth++
                }
                device.wait(Until.hasObject(lastLine), 3_000)
            }
        }
        assertTrue("結果の札を広げられない（最後の行が見えない）", device.hasObject(lastLine))
        hold(8_000)
        device.pressBack()
        manager.cancel(CARD_ID)
    }

    private fun step(label: String) {
        instrumentation.runOnMainSync { Toast.makeText(context, label, Toast.LENGTH_LONG).show() }
        hold()
    }

    private fun hold(ms: Long = HOLD_MS) = Thread.sleep(ms)

    private fun click(text: String) {
        val target = device.wait(Until.findObject(By.text(Pattern.compile(text))), TIMEOUT_MS)
        assertNotNull("「$text」が画面に出ない（前面: ${device.currentPackageName}）", target)
        hold()
        target.click()
    }

    private fun granted(permission: String) =
        context.checkSelfPermission(permission) == PackageManager.PERMISSION_GRANTED

    private fun waitUntil(timeoutMs: Long = TIMEOUT_MS, check: () -> Boolean): Boolean {
        val deadline = System.currentTimeMillis() + timeoutMs
        while (System.currentTimeMillis() < deadline) {
            if (check()) return true
            Thread.sleep(250)
        }
        return false
    }

    private companion object {
        const val TIMEOUT_MS = 15_000L
        const val HOLD_MS = 1_500L
        const val DENY = "^(許可しない|Don.t allow)$"
        const val SETTINGS_PACKAGE = "com.android.settings"
        const val CARD_CHANNEL = "recording-card"
        const val CARD_ID = 9_009
        const val PHOTO_NAME = "ashiato-st09-recording.jpg"
    }
}
