// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.Service
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.IBinder
import android.util.Log
import java.io.File
import java.time.ZoneId
import java.util.UUID
import java.util.concurrent.Executors
import java.util.concurrent.ScheduledExecutorService
import java.util.concurrent.TimeUnit

/**
 * 60 秒ごとに位置を取り、5 分ごとにまとめて送る（FR-1 / design D7 / design D9）。
 *
 * **前景サービスにする理由**: Android は継続的な位置取得を前景サービスなしに許さない。
 * 常時通知が出るのは本人が受け入れ済み（design D7）。
 * 本人の操作を必要とする収集は途切れる —— 成功条件 1 は「1 年間途切れない」こと。
 */
open class LocationService : Service() {
    private lateinit var outbox: Outbox
    private lateinit var fixSource: FixSource
    private lateinit var deviceId: String
    private var flusher: FlushScheduler? = null

    private lateinit var callback: FixCollector

    /** 未送信の置き場を試験から覗く口。**本番の経路は変えない**（review R1）。 */
    internal val outboxForTest: Outbox get() = outbox

    /** 取得元。**試験だけが差し替える**（review R1）。本番は Play Services（design D7）。 */
    protected open fun newFixSource(): FixSource = FusedFixSource(this)

    /** 送信の刻み。同上。1 本の糸で回す —— 送信が重なると同じ記録を 2 回送る。 */
    protected open fun newScheduler(): FlushScheduler = ExecutorFlushScheduler()

    /** 未送信の置き場。**端末の保存領域**（深掘り 第 2 回 / design D17 / D22）。 */
    protected open fun newOutbox(): Outbox =
        Outbox(FileOutboxStore(File(filesDir, "outbox.jsonl")) { Log.w(TAG, it) })

    override fun onCreate() {
        super.onCreate()
        deviceId = resolveDeviceId(AndroidIdStore(this)) { UUID.randomUUID().toString() }
        // **未送信は端末の保存領域へ**（深掘り 第 2 回）—— START_STICKY で立て直されたときに
        // インスタンスの中だけに積んでいると、最大 5 分ぶんが無言で消える
        outbox = newOutbox()
        fixSource = newFixSource()
        callback = FixCollector(
            outbox = outbox,
            deviceId = deviceId,
            userId = Config.userId,
            zone = ZoneId.systemDefault(),
            newId = { UUID.randomUUID().toString() },
            log = { Log.i(TAG, it) },
        )
        startForeground(NOTIFICATION_ID, notification(), ServiceInfo.FOREGROUND_SERVICE_TYPE_LOCATION)
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        try {
            // **FR-1 が定めた 60 秒。** 本人が決めた値なので、ここをリテラルに書き換えない
            fixSource.start(FIX_INTERVAL_MS, callback)
        } catch (e: SecurityException) {
            // 権限が無い。**落とさずに何もしない**（tasks 6.2）——
            // 落ちると次の起動まで収集が止まり、成功条件 1 に直接効く
            Log.w(TAG, Telemetry.line("no_permission", error = e.javaClass.simpleName))
            stopSelf()
            return START_NOT_STICKY
        }
        startFlushing()
        // 落とされても OS に立て直させる。1 年間途切れないことが成功条件 1
        return START_STICKY
    }

    /** 5 分ごとにその時点の未送信をまとめて送る（design D9）。 */
    private fun startFlushing() {
        if (flusher != null) return
        if (!Config.isComplete) {
            // 設定が無いなら送らない。**取得は続ける** —— 記録は未送信に積まれ、後から送れる
            Log.w(TAG, Telemetry.line("not_configured"))
            return
        }
        val sender = Sender(outbox, HttpTransport(Config.baseUrl, Config.apiToken)) { Log.i(TAG, it) }
        // **design D9 が決めた 5 分。** 本人が決めた値なので、ここをリテラルに書き換えない
        flusher = newScheduler().also { scheduler ->
            scheduler.every(SEND_INTERVAL_MS) {
                // 1 回の失敗で以後の送信を止めない。**ただし黙らない**（review CRITICAL-4）——
                // 握り潰すと、送信が毎回失敗していても logcat に 1 行も残らない。
                // 種別だけを出すので、私的データは漏れない（製造準備 A-2）
                runCatching { sender.flush() }.onFailure {
                    Log.w(TAG, Telemetry.line("flush_crashed", error = it.javaClass.simpleName))
                }
            }
        }
    }

    override fun onDestroy() {
        fixSource.stop(callback)
        flusher?.cancel()
        flusher = null
        super.onDestroy()
    }

    override fun onBind(intent: Intent?): IBinder? = null

    private fun notification(): Notification {
        val manager = getSystemService(NotificationManager::class.java)
        manager.createNotificationChannel(
            NotificationChannel(CHANNEL, "位置の記録", NotificationManager.IMPORTANCE_LOW),
        )
        return Notification.Builder(this, CHANNEL)
            .setContentTitle("あしあと。")
            .setContentText("位置を記録しています")
            .setSmallIcon(android.R.drawable.ic_menu_mylocation)
            .setOngoing(true)
            .build()
    }

    private companion object {
        const val TAG = "ashiato"
        const val CHANNEL = "location"
        const val NOTIFICATION_ID = 1
    }
}

/** 本番の刻み。1 本の糸で回す —— 送信が重なると同じ記録を 2 回送る。 */
class ExecutorFlushScheduler : FlushScheduler {
    private var pool: ScheduledExecutorService? = null

    override fun every(periodMs: Long, task: () -> Unit) {
        pool = Executors.newSingleThreadScheduledExecutor().also {
            it.scheduleWithFixedDelay({ task() }, periodMs, periodMs, TimeUnit.MILLISECONDS)
        }
    }

    override fun cancel() {
        pool?.shutdownNow()
        pool = null
    }
}
