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
import java.time.Instant
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
    private lateinit var outbox: Outbox<IngestRequest>
    private lateinit var heartbeatOutbox: Outbox<HeartbeatRequest>
    private lateinit var fixSource: FixSource
    private lateinit var deviceId: String
    private var flusher: FlushScheduler? = null
    private var beater: FlushScheduler? = null

    /**
     * 前回の生存信号からの取得の試行と成功（第 5 回 Q17）。
     *
     * **端末の保存領域に置く**（review/code.md の R16）。インスタンスの中だけに持つと、
     * `START_STICKY` の立て直しで `since` ごと新品になり、**死んでいた区間が観測から落ちる**
     * —— 6 時間のうち 5 時間 50 分死んで 10 分前に立て直されると、次の信号は
     * `10 / 10` で「取得率 100 %」になる。それは見分けたかった当の区間。
     */
    private lateinit var counters: AttemptCounters

    private lateinit var callback: FixCollector

    /** 未送信の置き場を試験から覗く口。**本番の経路は変えない**（review R1）。 */
    internal val outboxForTest: Outbox<IngestRequest> get() = outbox

    /** 生存信号の未送信を試験から覗く口。同上。 */
    internal val heartbeatOutboxForTest: Outbox<HeartbeatRequest> get() = heartbeatOutbox

    /** 取得元。**試験だけが差し替える**（review R1）。本番は Play Services（design D7）。 */
    protected open fun newFixSource(): FixSource = FusedFixSource(this)

    /** 送信の刻み。同上。1 本の糸で回す —— 送信が重なると同じ記録を 2 回送る。 */
    protected open fun newScheduler(): FlushScheduler = ExecutorFlushScheduler()

    /** 未送信の置き場。**端末の保存領域**（深掘り 第 2 回 / design D17 / D22）。 */
    protected open fun newOutbox(): Outbox<IngestRequest> =
        Outbox(FileOutboxStore(File(filesDir, "outbox.jsonl"), IngestRequest.serializer()) { Log.w(TAG, it) })

    /**
     * 生存信号の未送信。**記録とは別のファイル**にする —— 同じ JSONL に混ぜると、
     * 読み戻しで片方が「壊れた行」に見えて退避に回る（`FileOutboxStore.parse`）。
     * 仕組み（追記・書きかけの回収・壊れた行の退避）は記録とまったく同じものを使う。
     */
    protected open fun newHeartbeatOutbox(): Outbox<HeartbeatRequest> =
        Outbox(FileOutboxStore(File(filesDir, "heartbeat.jsonl"), HeartbeatRequest.serializer()) { Log.w(TAG, it) })

    /** 生存信号の刻み。試験だけが差し替える。 */
    protected open fun newHeartbeatScheduler(): FlushScheduler = ExecutorFlushScheduler()

    /** 数えの置き場。**端末の保存領域**（review/code.md の R16）。試験だけが差し替える。 */
    protected open fun newCounterStore(): CounterStore =
        FileCounterStore(File(filesDir, "heartbeat-counters.txt")) { Log.w(TAG, it) }

    /**
     * いま取得できる状態か（深掘り Q5）。**試験だけが差し替える。**
     *
     * 権限が剥がれると**プロセスは生きたまま位置が 0 件**になる ——
     * Android は長期間使っていないアプリの権限を自動で剥がす。
     * 稼働だけを送っていると、壊れているのに「動いていた」と残る。
     */
    protected open fun readCapability(): Capability = androidCapability(this)

    override fun onCreate() {
        super.onCreate()
        deviceId = resolveDeviceId(AndroidIdStore(this)) { UUID.randomUUID().toString() }
        // **未送信は端末の保存領域へ**（深掘り 第 2 回）—— START_STICKY で立て直されたときに
        // インスタンスの中だけに積んでいると、最大 5 分ぶんが無言で消える
        counters = AttemptCounters(now = { Instant.now() }, store = newCounterStore())
        outbox = newOutbox()
        heartbeatOutbox = newHeartbeatOutbox()
        fixSource = newFixSource()
        callback = FixCollector(
            outbox = outbox,
            deviceId = deviceId,
            userId = Config.userId,
            zone = ZoneId.systemDefault(),
            newId = { UUID.randomUUID().toString() },
            log = { Log.i(TAG, it) },
            onFix = { counters.recordSuccess() },
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
        startBeating()
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
        // **記録は恒久的に断られたら捨てる**（ST03 / FR-10 の改訂。深掘り Q4 / Q5）——
        // 残すと 5 分ごとに送られ続け、200 件たまると新しい記録が送られなくなる。
        val sender = Sender(
            outbox,
            HttpTransport(Config.baseUrl, Config.apiToken, "/ingest"),
            IngestRequest.serializer(),
            dropPermanentlyRejected = true,
        ) { Log.i(TAG, it) }
        // **生存信号も同じ契機で送る**（specs「記録と同じ未送信の仕組みに乗せて再送する」）。
        // 別の刻みを立てると、送信の契機が 2 つになって電池と網の使い方が読めなくなる。
        val beatSender = Sender(
            heartbeatOutbox,
            HttpTransport(Config.baseUrl, Config.apiToken, "/heartbeat"),
            HeartbeatRequest.serializer(),
        ) { Log.i(TAG, it) }
        // **design D9 が決めた 5 分。** 本人が決めた値なので、ここをリテラルに書き換えない
        flusher = newScheduler().also { scheduler ->
            scheduler.every(SEND_INTERVAL_MS) {
                // **記録の送信が落ちても生存信号は送る。** 1 つの `runCatching` にまとめると、
                // 記録が送れない期間の稼働がまるごと残らなくなる。
                // 例外の握りつぶし自体は `ExecutorFlushScheduler` が構造で持つ（design D26）
                runCatching { sender.flush() }.onFailure {
                    Log.w(TAG, Telemetry.line("flush_crashed", error = it.javaClass.simpleName))
                }
                beatSender.flush()
            }
        }
    }

    /**
     * 想定間隔ごとに生存信号を積む（FR-78 / specs/device-collection）。
     *
     * **記録の生成に相乗りさせない**（tasks 7.4）—— 記録が 1 件も生成されない期間に
     * 稼働を残すことが FR-78 の目的そのもの。取得の契機から呼ぶと意味が消える。
     *
     * **起動のたびに 1 件積む。** 6 時間の刻みだけに任せると、OS が 5 時間ごとに
     * 立て直す端末では**生存信号が 1 件も出ない**まま「途絶」に見える。
     */
    private fun startBeating() {
        if (beater != null) return
        val emitter = HeartbeatEmitter(
            outbox = heartbeatOutbox,
            counters = counters,
            userId = Config.userId,
            deviceId = deviceId,
            capability = ::readCapability,
            now = { Instant.now() },
            newId = { UUID.randomUUID().toString() },
            log = { Log.i(TAG, it) },
        )
        // **起動時の 1 発も守る**（review/code.md の R36 / I7）。
        // `readCapability()`（端末を読む）も `outbox.add()`（filesDir への追記）も投げうる。
        // ここが裸だと `onStartCommand` を貫通してプロセスごと落ち、
        // START_STICKY と合わさってクラッシュループになる ——
        // このファイル自身が権限拒否のところで立てた規律（**落とさずに何もしない**）と食い違う。
        runCatching { emitter.emit() }.onFailure {
            Log.w(TAG, Telemetry.line("heartbeat_crashed", error = it.javaClass.simpleName))
        }
        // **登録簿の想定間隔に合わせる**（tasks 7.1）。ずらすと正常な運用が途絶に見える
        beater = newHeartbeatScheduler().also { scheduler ->
            scheduler.every(HEARTBEAT_INTERVAL_MS) { emitter.emit() }
        }
    }

    override fun onDestroy() {
        fixSource.stop(callback)
        flusher?.cancel()
        flusher = null
        beater?.cancel()
        beater = null
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

    /**
     * **例外をここで受け止める**（review/code.md の R30 / M-7）。
     *
     * `scheduleWithFixedDelay` は task が投げると**以後の実行を静かに打ち切る**
     * （ログも例外も出ない）。呼び出し側が毎回 `runCatching` を書くことに安全性を
     * 委ねると、1 度忘れた日に**収集は前景通知を出したまま送信も生存信号も永久に止まり、
     * logcat に 1 行も残らない**。性質は構造で持たせる。
     */
    override fun every(periodMs: Long, task: () -> Unit) {
        pool = Executors.newSingleThreadScheduledExecutor().also {
            it.scheduleWithFixedDelay(
                {
                    runCatching { task() }.onFailure { e ->
                        Log.w("ashiato", Telemetry.line("tick_crashed", error = e.javaClass.simpleName))
                    }
                },
                periodMs,
                periodMs,
                TimeUnit.MILLISECONDS,
            )
        }
    }

    override fun cancel() {
        pool?.shutdownNow()
        pool = null
    }
}
