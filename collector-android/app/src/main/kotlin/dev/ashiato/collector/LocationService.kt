// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.Manifest
import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.Service
import android.content.Intent
import android.content.pm.PackageManager
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.IBinder
import android.os.SystemClock
import android.util.Log
import java.io.File
import java.time.Instant
import java.time.ZoneId
import java.time.format.DateTimeParseException
import java.util.UUID
import java.util.concurrent.Executors
import java.util.concurrent.ScheduledExecutorService
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger
import java.util.concurrent.atomic.AtomicLong

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
    private lateinit var dropsOutbox: Outbox<DropReport>
    private lateinit var ageClock: AgeClock
    private lateinit var ledger: DropLedger
    private lateinit var writeFailed: WriteFailedLedger
    private lateinit var retention: Retention<IngestRequest>
    private lateinit var notifier: RetentionNotifier

    /**
     * 置き場の読めない行の件数。**その場で報告に足さず、見回りで拾う** ——
     * 読んでいる置き場の錠を持ったまま報告の錠を取ると、逆の順で錠を取る糸と行き違って止まる。
     */
    private val unreadableSeen = AtomicInteger(0)
    private val lastMaintenanceMs = AtomicLong(Long.MIN_VALUE)
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

    /** 破棄の報告の未送信を試験から覗く口。同上。 */
    internal val dropsOutboxForTest: Outbox<DropReport> get() = dropsOutbox

    /** 破棄の報告の下書きを試験から覗く口。同上。 */
    internal val ledgerForTest: DropLedger get() = ledger

    /** 上限の見回りを試験から呼ぶ口（間引かない）。 */
    internal fun maintainForTest() = maintain(force = true)

    /** 取得元。**試験だけが差し替える**（review R1）。本番は Play Services（design D7）。 */
    protected open fun newFixSource(): FixSource = FusedFixSource(this)

    /** 受け口への POST。**試験だけが差し替える**（到達できない 1 時間・偽のサーバ。design D15）。 */
    protected open fun newTransport(path: String): Transport = HttpTransport(Config.baseUrl, Config.apiToken, path)

    /** 送信の刻み。同上。1 本の糸で回す —— 送信が重なると同じ記録を 2 回送る。 */
    protected open fun newScheduler(): FlushScheduler = ExecutorFlushScheduler()

    /** 未送信の置き場の根。**端末の保存領域**（深掘り 第 2 回 / ST04 design D1）。 */
    protected open fun outboxDir(): File = File(filesDir, OUTBOX_DIR)

    /** 端末の時計。**試験だけが差し替える**（90 日の数え。design D2）。 */
    protected open fun newDeviceClock(): DeviceClock = AndroidDeviceClock(this)

    /** 知らせの出し先。**試験では Robolectric の本物の NotificationManager** を通す。 */
    protected open fun newRetentionAlerts(): RetentionAlerts = AndroidRetentionAlerts(this) { notification(it) }

    private fun store(name: String) = File(outboxDir(), name)

    /**
     * 記録の未送信。**区切りファイル**（ST04 / C7 / design D1）。書けなかった記録は固定長の数えに残し（D5）、
     * 積むたびに保持の上限を見回る（新しい契機を起こさない）。
     */
    private fun newOutbox(): Outbox<IngestRequest> = Outbox(
        SegmentStore(
            store("records"), IngestRequest.serializer(), store(UNREADABLE), { Log.w(TAG, it) },
            onUnreadable = { unreadableSeen.addAndGet(it) },
        ),
        age = ageClock::now,
        writeFailures = object : WriteFailures<IngestRequest> {
            override fun failed(item: IngestRequest) {
                eventInstant(item)?.let(writeFailed::failed)
            }

            override fun recovered(item: IngestRequest) {
                eventInstant(item)?.let(writeFailed::recovered)
            }

            override fun lost(item: IngestRequest) {
                // メモリにも持ちきれず手放した —— その場で「書けなかった」破棄として報告する
                val t = eventInstant(item) ?: return
                writeFailed.recovered(t)
                ledger.dropped(item.logicalSource, DropReason.WRITE_FAILED, t)
                ledger.endBatch(item.logicalSource, DropReason.WRITE_FAILED, null)
            }
        },
        afterAdd = { maintain(force = false) },
    )

    /**
     * 生存信号の未送信。**記録とは別の置き場**にする —— 上限の対象にしない（捨てない。C1 / design D13）。
     * 仕組み（追記・読めない行の退避）は記録とまったく同じものを使う。
     */
    private fun newHeartbeatOutbox(): Outbox<HeartbeatRequest> = Outbox(
        SegmentStore(
            store("heartbeats"), HeartbeatRequest.serializer(), store(UNREADABLE), { Log.w(TAG, it) },
            onUnreadable = { unreadableSeen.addAndGet(it) },
        ),
        age = ageClock::now,
    )

    /** 破棄の報告の未送信。上限の対象にしない（C2 / design D13）。 */
    private fun newDropsOutbox(): Outbox<DropReport> = Outbox(
        SegmentStore(
            store("drops"), DropReport.serializer(), store(UNREADABLE), { Log.w(TAG, it) },
            onUnreadable = { unreadableSeen.addAndGet(it) },
        ),
        age = ageClock::now,
    )

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
        openStores()
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
        startForeground(NOTIFICATION_ID, notification(RetentionNotifier.BASE_TEXT), ServiceInfo.FOREGROUND_SERVICE_TYPE_LOCATION)
        // 起動の時点で 1 度見回る（設定が揃っていなくても上限はかかる。深掘り Q1）
        maintain(force = true)
    }

    /**
     * 置き場を開く（ST04）。**上限・報告・知らせは記録の置き場の上に載る。**
     * ST01 / ST02 の 1 本の JSONL が残っていれば、ここで区切りへ取り込む（design D1 / D6）。
     */
    private fun openStores() {
        val logW: (String) -> Unit = { Log.w(TAG, it) }
        val logI: (String) -> Unit = { Log.i(TAG, it) }
        outboxDir().mkdirs()
        ageClock = AgeClock(newDeviceClock(), store("age-clock.txt"), logW)
        dropsOutbox = newDropsOutbox()
        ledger = DropLedger(
            store("drops-open.json"), dropsOutbox, { Config.userId }, deviceId,
            now = { Instant.now() }, newId = { UUID.randomUUID().toString() }, log = logI,
        )
        // **前のプロセスで書けないまま失われた記録**（固定長の数えが 0 でない。design D5）
        writeFailed = WriteFailedLedger(store("write-failed.bin"), logW)
        val (slots, overflow) = writeFailed.take()
        if (slots.isNotEmpty() || overflow > 0) ledger.writeFailed(LOGICAL_SOURCE, slots, overflow)

        outbox = newOutbox()
        heartbeatOutbox = newHeartbeatOutbox()
        val records = migrateLegacyOutbox(
            File(filesDir, "outbox.jsonl"), outbox, IngestRequest.serializer(),
            store(UNREADABLE), store("salvaged"), logW,
        )
        val beats = migrateLegacyOutbox(
            File(filesDir, "heartbeat.jsonl"), heartbeatOutbox, HeartbeatRequest.serializer(),
            store(UNREADABLE), store("salvaged"), logW,
        )
        unreadableSeen.addAndGet(records.unreadable + beats.unreadable)

        retention = Retention(outbox, ledger, ageClock::now, log = logI)
        notifier = RetentionNotifier(outbox, ageClock::now, newRetentionAlerts(), store("retention-alerted"), log = logI)
    }

    /**
     * 保持の上限の見回りと、知らせの更新（ST04 / design D2 / D3 / D11）。
     * **積む契機（60 秒）と送信の契機に相乗りする**。積む契機からは 1 分に 1 度まで。
     */
    private fun maintain(force: Boolean) {
        if (!::notifier.isInitialized) return
        val now = SystemClock.elapsedRealtime()
        val last = lastMaintenanceMs.get()
        if (!force && last != Long.MIN_VALUE && now - last < MAINTENANCE_MIN_GAP_MS) return
        lastMaintenanceMs.set(now)
        val broken = unreadableSeen.getAndSet(0)
        if (broken > 0) ledger.unreadable(LOGICAL_SOURCE, broken)
        runCatching { retention.enforce() }.onFailure {
            Log.w(TAG, Telemetry.line("retention_crashed", error = it.javaClass.simpleName))
        }
        runCatching { notifier.update() }.onFailure {
            Log.w(TAG, Telemetry.line("notifier_crashed", error = it.javaClass.simpleName))
        }
    }

    private fun eventInstant(item: IngestRequest): Instant? = try {
        Instant.parse(item.eventTime)
    } catch (e: DateTimeParseException) {
        null
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
            newTransport("/ingest"),
            IngestRequest.serializer(),
            dropPermanentlyRejected = true,
        ) { Log.i(TAG, it) }
        // **生存信号も同じ契機で送る**（specs「記録と同じ未送信の仕組みに乗せて再送する」）。
        // 別の刻みを立てると、送信の契機が 2 つになって電池と網の使い方が読めなくなる。
        val beatSender = Sender(
            heartbeatOutbox,
            newTransport("/heartbeat"),
            HeartbeatRequest.serializer(),
        ) { Log.i(TAG, it) }
        // **破棄の報告は断られても取り除かない**（ST04 / C2 / design D13）—— 扉 #14 の唯一の証拠
        val dropSender = Sender(
            dropsOutbox,
            newTransport("/drops"),
            DropReport.serializer(),
            dropPermanentlyRejected = false,
        ) { Log.i(TAG, it) }
        val drainer = Drainer(
            records = sender,
            recordsOutbox = outbox,
            beats = beatSender,
            drops = dropSender,
            ledger = ledger,
            maintenance = { maintain(force = true) },
            log = { Log.i(TAG, it) },
        )
        // **design D9 が決めた 5 分。** 本人が決めた値なので、ここをリテラルに書き換えない。
        // 溜まっている間だけ、1 回の契機の中で続けて送る（ST04 / 深掘り Q6 / design D12）。
        // 例外の握りつぶしは `Drainer` と `ExecutorFlushScheduler` が構造で持つ（design D26）
        flusher = newScheduler().also { scheduler ->
            scheduler.every(SEND_INTERVAL_MS) { drainer.tick() }
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

    /** 常駐の通知。**本文だけを差し替える**（未送信の日数。ST04 / 深掘り Q5）。常時出ている通知は増やさない */
    private fun notification(text: String): Notification {
        val manager = getSystemService(NotificationManager::class.java)
        manager.createNotificationChannel(
            NotificationChannel(CHANNEL, "位置の記録", NotificationManager.IMPORTANCE_LOW),
        )
        return Notification.Builder(this, CHANNEL)
            .setContentTitle("あしあと。")
            .setContentText(text)
            .setSmallIcon(android.R.drawable.ic_menu_mylocation)
            .setOngoing(true)
            .setOnlyAlertOnce(true)
            .build()
    }

    internal companion object {
        const val TAG = "ashiato"
        const val CHANNEL = "location"
        const val NOTIFICATION_ID = 1

        /** 上限の 7 日前の知らせのチャネル（音が鳴る。design D11） */
        const val RETENTION_CHANNEL = "retention"
        const val RETENTION_NOTIFICATION_ID = 2

        /** 未送信の置き場の根（`filesDir` の下） */
        const val OUTBOX_DIR = "outbox"
        const val UNREADABLE = "unreadable.jsonl"

        /** 積む契機からの見回りの間引き */
        const val MAINTENANCE_MIN_GAP_MS: Long = 60_000
    }
}

/**
 * 知らせを端末の通知に出す（ST04 / 深掘り Q5 / design D11（仮））。
 *
 * 常駐の通知は本文を差し替えるだけ。上限の 7 日前の知らせは**別のチャネル（音が鳴る）**で 1 回。
 * 通知の権限（`POST_NOTIFICATIONS`）が無い端末では出せないので false を返す（呼び出し側がログに 1 行残す）。
 */
class AndroidRetentionAlerts(
    private val service: Service,
    private val ongoingNotification: (String) -> Notification,
) : RetentionAlerts {
    private val manager: NotificationManager get() = service.getSystemService(NotificationManager::class.java)

    override fun ongoing(text: String) {
        manager.notify(LocationService.NOTIFICATION_ID, ongoingNotification(text))
    }

    override fun alert(days: Int): Boolean {
        if (Build.VERSION.SDK_INT >= 33 &&
            service.checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED
        ) {
            return false
        }
        if (!manager.areNotificationsEnabled()) return false
        manager.createNotificationChannel(
            NotificationChannel(LocationService.RETENTION_CHANNEL, "未送信の保持", NotificationManager.IMPORTANCE_DEFAULT),
        )
        val left = ((RetentionPolicy.current.maxAgeMs / AgeClock.DAY_MS) - days).coerceAtLeast(0)
        manager.notify(
            LocationService.RETENTION_NOTIFICATION_ID,
            Notification.Builder(service, LocationService.RETENTION_CHANNEL)
                .setContentTitle("あしあと。未送信が $days 日たまっています")
                .setContentText("自宅 PC に届いていません。あと $left 日で古いものから捨てます")
                .setSmallIcon(android.R.drawable.stat_notify_error)
                .setAutoCancel(true)
                .build(),
        )
        return true
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
