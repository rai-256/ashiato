// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.Service
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.IBinder
import android.os.Looper
import android.util.Log
import com.google.android.gms.location.FusedLocationProviderClient
import com.google.android.gms.location.LocationRequest
import com.google.android.gms.location.LocationServices
import com.google.android.gms.location.Priority
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
class LocationService : Service() {
    private lateinit var outbox: Outbox
    private lateinit var client: FusedLocationProviderClient
    private lateinit var deviceId: String
    private var flusher: ScheduledExecutorService? = null

    private lateinit var callback: FixCollector

    override fun onCreate() {
        super.onCreate()
        deviceId = resolveDeviceId(AndroidIdStore(this)) { UUID.randomUUID().toString() }
        // **未送信は端末の保存領域へ**（深掘り 第 2 回）—— START_STICKY で立て直されたときに
        // インスタンスの中だけに積んでいると、最大 5 分ぶんが無言で消える
        outbox = Outbox(FileOutboxStore(File(filesDir, "outbox.json")) { Log.w(TAG, it) })
        callback = FixCollector(
            outbox = outbox,
            deviceId = deviceId,
            userId = Config.userId,
            zone = ZoneId.systemDefault(),
            newId = { UUID.randomUUID().toString() },
            log = { Log.i(TAG, it) },
        )
        client = LocationServices.getFusedLocationProviderClient(this)
        startForeground(NOTIFICATION_ID, notification(), ServiceInfo.FOREGROUND_SERVICE_TYPE_LOCATION)
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        val request = LocationRequest.Builder(Priority.PRIORITY_HIGH_ACCURACY, FIX_INTERVAL_MS).build()
        try {
            client.requestLocationUpdates(request, callback, Looper.getMainLooper())
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
        flusher = Executors.newSingleThreadScheduledExecutor().also {
            it.scheduleWithFixedDelay(
                { runCatching { sender.flush() } },   // 1 回の失敗で以後の送信を止めない
                SEND_INTERVAL_MS,
                SEND_INTERVAL_MS,
                TimeUnit.MILLISECONDS,
            )
        }
    }

    override fun onDestroy() {
        client.removeLocationUpdates(callback)
        flusher?.shutdownNow()
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
