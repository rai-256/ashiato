// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.Manifest
import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
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
import java.util.concurrent.ExecutorService
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
    private lateinit var dropsOutbox: Outbox<DropReport>
    private lateinit var ageClock: AgeClock
    private lateinit var ledger: DropLedger

    /**
     * 置き場に書けなかった記録の数え。**ソースごとに別ファイル**（tasks 5.1 / 独立レビュー R10）——
     * 1 本にまとめると、アプリ利用の書き込みが失敗した件数が**位置の破棄として報告される**。
     */
    private lateinit var writeFailed: Map<String, WriteFailedLedger>
    private lateinit var retention: Retention<IngestRequest>
    private lateinit var notifier: RetentionNotifier

    /** 見回りを 1 本の糸ずつにする錠（位置の糸と送信の糸が同時に入らない。review R14）。 */
    private val maintenanceLock = Any()
    private var lastMaintenanceMs = Long.MIN_VALUE

    /**
     * 走らせているソース。**親はこの口だけを見る**（design D5 / tasks 1.1 / 5.1）——
     * 窓も取り込み済みの粒度も積んだ原文の指紋も、**全部ソースの中**にある。
     */
    private lateinit var sources: List<Running>
    private lateinit var deviceId: String
    private var flusher: FlushScheduler? = null

    private lateinit var callback: FixCollector

    /**
     * 走らせている 1 本のソース（design D5）。
     *
     * **数え・生存信号・取得の刻みをソースごとに持つ。** どれかを共有すると、
     * 1 本が欠けたときに他が道連れになる（本人の決定 Q7 が禁じている当のこと）。
     */
    private class Running(
        val cadence: SourceCadence,
        val source: CollectionSource,
        /**
         * 前回の生存信号からの取得の試行と成功（第 5 回 Q17）。
         *
         * **端末の保存領域に置く**（review/code.md の R16）。インスタンスの中だけに持つと、
         * `START_STICKY` の立て直しで `since` ごと新品になり、**死んでいた区間が観測から落ちる**
         * —— 6 時間のうち 5 時間 50 分死んで 10 分前に立て直されると、次の信号は
         * `10 / 10` で「取得率 100 %」になる。それは見分けたかった当の区間。
         */
        val counters: AttemptCounters,
    ) {
        /**
         * 取得元が契機を**自分で配っている**（位置。[CollectionResult.Streaming]）。
         * 立った後の刻みでは何もしない —— 張り直すのは `onStartCommand` の役目（復旧の経路）。
         */
        @Volatile
        var selfDriven: Boolean = false

        /** 取得の刻み。**取得条件が欠けていても張る** —— 後から許可されたら次の契機で戻るため */
        var ticker: FlushScheduler? = null

        /** 生存信号の刻み。**取得できなくても出し続ける**（本人の決定 Q7） */
        var beater: FlushScheduler? = null

        /**
         * 直前の契機で「取れなかった」理由。**同じ理由を毎回ログに出さない**ため。
         *
         * 位置の契機は 60 秒ごとなので、権限を拒んだままの端末では 1 日 1440 行になる ——
         * `logcat` は環状の置き場なので、それだけで**他の行が押し出される**
         * （計測テストも `logcat -t 400` で読んでいる）。
         * 取れない状態そのものは生存信号（`blockers`）が区間ごとに持っている。
         */
        var lastUnavailable: String? = null
    }

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

    /**
     * アプリ利用の取得元。**試験だけが差し替える**（tasks 5.1）——
     * `UsageStatsManager` に溜まった統計は単体では作れないので、単体はこの口の偽物に当たる。
     */
    protected open fun newUsageSource(): UsageSource = UsageStatsSource(this)

    /** 取得時点のアプリの表示名。同上（端末に触らずに記録の組み立てを試験するため）。 */
    protected open fun newAppLabels(): AppLabels = PackageManagerAppLabels(this)

    /**
     * 取得の刻み。**ソースごとに 1 本**（tasks 5.1）—— 1 本にまとめると、
     * 間隔の違うソース（位置 60 秒 / アプリ利用 30 分 / 集計 6 時間）が同じ契機で動く。
     */
    protected open fun newSourceScheduler(logicalSource: String): FlushScheduler = ExecutorFlushScheduler()

    /**
     * 起動の契機の取得を回す（tasks 5.1）。**試験だけが差し替える**（その場で回して順番を決めるため）。
     *
     * **主糸で回さない。** `onStartCommand` は主糸で呼ばれ、初回の集計の取り込みは
     * 4 粒度ぶんの問い合わせと数千件の追記になる —— 主糸を塞ぐと前景サービスが
     * 「応答なし」に見える。刻みの側（[newSourceScheduler]）は既に自分の糸で回っている。
     */
    protected open fun runCollection(task: () -> Unit) {
        collectorThread.execute(task)
    }

    /** 起動の契機の取得を回す糸（1 本）。**使われるまで糸は作られない。** */
    private val collectorThread: ExecutorService = Executors.newSingleThreadExecutor()

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
        ),
        age = ageClock::now,
        writeFailures = object : WriteFailures<IngestRequest> {
            override fun failed(item: IngestRequest) {
                // **その記録のソースの数えに入れる**（tasks 5.1）—— 1 本にまとめると、
                // アプリ利用の書き込みの失敗が位置の破棄として報告される
                val counter = writeFailedFor(item.logicalSource) ?: return
                eventInstant(item)?.let(counter::failed)
            }

            override fun recovered(item: IngestRequest) {
                val counter = writeFailedFor(item.logicalSource) ?: return
                eventInstant(item)?.let(counter::recovered)
            }

            override fun lost(item: IngestRequest) {
                // メモリにも持ちきれず手放した —— その場で「書けなかった」破棄として報告する。
                // **下書きを保存できたときだけ固定長の数えから引く**（review R1）。保存できなければ数えに残し、
                // 次の起動の「書けなかった」報告に任せる（固定長の数えは空きが尽きても上書きが通る側の置き場）
                val t = eventInstant(item)
                if (ledger.record(DropReason.WRITE_FAILED, listOf(item.logicalSource to t)) != null) {
                    ledger.endBatch(item.logicalSource, DropReason.WRITE_FAILED, null)
                    val counter = writeFailedFor(item.logicalSource)
                    if (counter != null) t?.let(counter::recovered)
                }
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
        ),
        age = ageClock::now,
    )

    /** 破棄の報告の未送信。上限の対象にしない（C2 / design D13）。 */
    private fun newDropsOutbox(): Outbox<DropReport> = Outbox(
        SegmentStore(
            store("drops"), DropReport.serializer(), store(UNREADABLE), { Log.w(TAG, it) },
        ),
        age = ageClock::now,
    )

    /** 生存信号の刻み。試験だけが差し替える。**ソースごとに 1 本**（tasks 5.1）。 */
    protected open fun newHeartbeatScheduler(logicalSource: String): FlushScheduler = ExecutorFlushScheduler()

    /**
     * 数えの置き場。**端末の保存領域**（review/code.md の R16）。**ソースごとに別ファイル**
     * （tasks 1.3 / 独立レビュー R10）—— 2 本目が同じ名前を開くと互いの数えを潰し合う。
     * ST06 より前の 1 本は位置の名前へ移る。試験だけが差し替える。
     */
    protected open fun newCounterStore(logicalSource: String): CounterStore =
        FileCounterStore(migrateLegacyCounters(filesDir, logicalSource), logicalSource) { Log.w(TAG, it) }

    /**
     * いま取得できる状態か（深掘り Q5）。**試験だけが差し替える。**
     *
     * 権限が剥がれると**プロセスは生きたまま位置が 0 件**になる ——
     * Android は長期間使っていないアプリの権限を自動で剥がす。
     * 稼働だけを送っていると、壊れているのに「動いていた」と残る。
     */
    protected open fun readCapability(source: CollectionSource): Capability = source.capability(this)

    override fun onCreate() {
        super.onCreate()
        deviceId = resolveDeviceId(AndroidIdStore(this)) { UUID.randomUUID().toString() }
        // **前景に上がってから置き場を開く**（review R29）。取り込みが長いと、前景に上がる期限（10 秒）を越えて落ちる
        val type = foregroundServiceType()
        startForeground(NOTIFICATION_ID, notification(ongoingBaseText(SourceCadence.entries.size)), type)
        // **どちらの種別で前景に上がれたかを残す**（tasks 1.4）。立てられなかったときは
        // `startForeground` が投げて `onCreate` ごと落ちるので、**この 1 行が出たこと自体が
        // 「前景サービスが立った」の証拠**になる（計測テストがこれを見る）
        Log.i(TAG, Telemetry.line(foregroundServiceTypeKind(type), source = null))
        openStores()
        callback = FixCollector(
            outbox = outbox,
            deviceId = deviceId,
            userId = Config.userId,
            zone = ZoneId.systemDefault(),
            newId = { UUID.randomUUID().toString() },
            log = { Log.i(TAG, it) },
            onFix = { locationCounters?.recordSuccess() },
        )
        sources = buildSources()
        // 起動の時点で 1 度見回る（設定が揃っていなくても上限はかかる。深掘り Q1）
        maintain(force = true)
    }

    /**
     * 位置の数え。**契機を配るのは取得元（Play Services）**なので、成功を数えるのは
     * その callback の側になる（`onFix`）。`sources` が組み上がる前に呼ばれても落とさない。
     */
    private val locationCounters: AttemptCounters?
        get() = if (::sources.isInitialized) {
            sources.firstOrNull { it.cadence == SourceCadence.LOCATION }?.counters
        } else {
            null
        }

    /**
     * 収集する 3 本を組み立てる（design D5 / 本人の決定 C11）。
     *
     * **記録の置き場は 3 本とも同じ**（C11）—— 保持の上限は全ソースを通して古い順にかかる。
     * 窓・取り込み済みの粒度・積んだ原文の指紋の置き場は**ソースごとに別ファイル**で、
     * どれもソースの中にある（親はその中身を知らない）。
     */
    private fun buildSources(): List<Running> {
        val logI: (String) -> Unit = { Log.i(TAG, it) }
        val logW: (String) -> Unit = { Log.w(TAG, it) }
        // 地域は**契機ごとに読む**（design D7（仮））—— 引っ越しても次の記録から追う
        val zone = { ZoneId.systemDefault() }
        val newId = { UUID.randomUUID().toString() }
        val user = { Config.userId }
        // 表示名は 2 本のアプリ利用で**同じものを使い回す**（端末への問い合わせを重ねない）
        val labels = newAppLabels()
        val usage = newUsageSource()
        val built = listOf(
            LocationSourceAdapter(newFixSource(), callback),
            AppUsageSourceAdapter(
                source = usage,
                outbox = outbox,
                windowStore = UsageWindowStore(
                    usageWindowFile(filesDir, APP_USAGE_LOGICAL_SOURCE), APP_USAGE_LOGICAL_SOURCE, logW,
                ),
                labels = labels,
                userId = user,
                deviceId = deviceId,
                zone = zone,
                age = ageClock::now,
                discardedMs = ageClock::discardedMs,
                newId = newId,
                capabilityOf = ::usageAccessCapability,
                log = logI,
            ),
            AppUsageRollupSourceAdapter(
                source = usage,
                outbox = outbox,
                progressStore = UsageRollupProgressStore(
                    usageRollupProgressFile(filesDir, APP_USAGE_ROLLUP_LOGICAL_SOURCE),
                    APP_USAGE_ROLLUP_LOGICAL_SOURCE,
                    logW,
                ),
                seenStore = UsageRollupSeenStore(
                    usageRollupSeenFile(filesDir, APP_USAGE_ROLLUP_LOGICAL_SOURCE),
                    APP_USAGE_ROLLUP_LOGICAL_SOURCE,
                    logW,
                ),
                labels = labels,
                userId = user,
                deviceId = deviceId,
                zone = zone,
                newId = newId,
                capabilityOf = ::usageAccessCapability,
                log = logI,
            ),
        )
        return built.map { source ->
            // **刻みはソースが名乗る名前から引く**（親がリテラルで持たない）。
            // `SourceCadence` の `init` が「生存信号の区間 ≥ 取得契機の間隔」を守っている
            val cadence = SourceCadence.entries.first { it.logicalSource == source.logicalSource }
            Running(
                cadence = cadence,
                source = source,
                counters = AttemptCounters(
                    now = { Instant.now() },
                    // **満点の刻みはそのソースの取得間隔**（design D5 / 独立レビュー R10）
                    intervalMs = cadence.intervalMs,
                    // **数えの置き場もソースごと**（2 本目が同じ名前を開くと互いに潰し合う）
                    store = newCounterStore(cadence.logicalSource),
                ),
            )
        }
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
            // 下書きのファイルごと読めなくなったときだけ使う名前（**焼き込まない**。tasks 5.1）
            unattributedSource = UNATTRIBUTED_SOURCE,
        )
        // **前のプロセスで書けないまま失われた記録**（固定長の数えが 0 でない。design D5）。
        // **ソースごとに 1 本**（tasks 5.1）—— ST06 より前の 1 本は位置の名前へ移る
        writeFailed = SourceCadence.entries.associate { cadence ->
            cadence.logicalSource to WriteFailedLedger(
                migrateLegacyWriteFailed(outboxDir(), cadence.logicalSource), logW,
            )
        }
        for ((source, counter) in writeFailed) {
            val (slots, overflow) = counter.peek()
            // **報告を保存できてから数えを 0 に戻す**（review R15）。先に戻すと、空きが尽きたまま立て直したときに痕跡が消える
            if ((slots.isNotEmpty() || overflow > 0) && ledger.writeFailed(source, slots, overflow)) {
                counter.clear()
            }
        }

        outbox = newOutbox()
        heartbeatOutbox = newHeartbeatOutbox()
        // 読めない行は退避先に書かれ、見回りが退避先の行数から報告に数える。ST01 が脇へ退けたファイルだけはここで数える。
        // **ここだけは位置と名乗ってよい**（tasks 5.1 / R10）—— 取り込む `outbox.jsonl` /
        // `heartbeat.jsonl` は ST01 / ST02 の版が書いたもので、**位置しか無かった時代のファイル**。
        // 2 本目が積まれるのは区切りの置き場（`outbox/records`）のほうで、そちらは `reportUnreadable` が
        // 行ごとに名前を読む
        val recordSalvaged: (Int) -> Boolean = { ledger.unreadable(SourceCadence.LOCATION.logicalSource, it) }
        migrateLegacyOutbox(
            File(filesDir, "outbox.jsonl"), outbox, IngestRequest.serializer(),
            store(UNREADABLE), store("salvaged"), recordSalvaged, ageClock.now(), logW,
        )
        migrateLegacyOutbox(
            File(filesDir, "heartbeat.jsonl"), heartbeatOutbox, HeartbeatRequest.serializer(),
            store(UNREADABLE), store("salvaged"), recordSalvaged, ageClock.now(), logW,
        )

        retention = Retention(outbox, ledger, ageClock::now, log = logI)
        notifier = RetentionNotifier(
            outbox,
            ageClock::now,
            newRetentionAlerts(),
            store("retention-alerted"),
            // **位置の決め打ちをやめ、走らせているソースの数に合わせる**（tasks 5.4 / R10）
            baseText = ongoingBaseText(SourceCadence.entries.size),
            log = logI,
        )
    }

    /**
     * 保持の上限の見回りと、知らせの更新（ST04 / design D2 / D3 / D11）。
     * **積む契機（60 秒）と送信の契機に相乗りする**。積む契機からは 1 分に 1 度まで。
     */
    private fun maintain(force: Boolean) {
        if (!::notifier.isInitialized) return
        synchronized(maintenanceLock) {
            val now = SystemClock.elapsedRealtime()
            val last = lastMaintenanceMs
            if (!force && last != Long.MIN_VALUE && now - last < MAINTENANCE_MIN_GAP_MS) return
            lastMaintenanceMs = now
            runCatching { reportUnreadable() }.onFailure {
                Log.w(TAG, Telemetry.line("unreadable_report_crashed", source = null, error = it.javaClass.simpleName))
            }
            runCatching { retention.enforce() }.onFailure {
                Log.w(TAG, Telemetry.line("retention_crashed", source = null, error = it.javaClass.simpleName))
            }
            runCatching { notifier.update() }.onFailure {
                Log.w(TAG, Telemetry.line("notifier_crashed", source = null, error = it.javaClass.simpleName))
            }
        }
    }

    /**
     * 読めない行の件数を報告に足す（design D6）。**数えは退避先の行数から取る**（review R21）——
     * 件数をメモリにだけ持つと、見回りの前に立て直されたとき、行は退避先にあるのに報告が作られない。
     * 報告に足せた行数を小さなファイルに書き、次はその先だけを数える。
     */
    private fun reportUnreadable() {
        val aside = store(UNREADABLE)
        if (!aside.exists()) return
        val mark = store("unreadable-reported.txt")
        val reported = runCatching { mark.readText().trim().toLong() }.getOrDefault(0L)
        // **行ごとにソースの名前を読む**（tasks 5.1 / R10）—— 2 本目を同じ置き場に載せた後は、
        // 全部を位置の破棄として報告すると扉 #14 の証拠がソースごとに誤る。
        // **数えるのは改行で閉じた行だけ**（書きかけの最後の 1 行を数えると、続きが書かれたとき二重になる）
        val fresh = HashMap<String, Int>()
        var total = 0L
        var unattributed = 0
        aside.inputStream().buffered().reader(Charsets.UTF_8).use { input ->
            val head = StringBuilder()
            var c = input.read()
            while (c != -1) {
                if (c == '\n'.code) {
                    total++
                    if (total > reported) {
                        val read = sourceOfUnreadableLine(head.toString())
                        if (read == null) unattributed++
                        val source = read ?: UNATTRIBUTED_SOURCE
                        fresh[source] = (fresh[source] ?: 0) + 1
                    }
                    head.setLength(0)
                } else if (head.length < UNREADABLE_SNIFF) {
                    // 名前は行の頭のほうにある（`logical_source` は 3 番目の欄）。
                    // **壊れた行を丸ごとメモリに載せない**
                    head.append(c.toChar())
                }
                c = input.read()
            }
        }
        if (fresh.isEmpty()) return
        if (unattributed > 0) {
            // **名前を読めなかった行の数**（[UNATTRIBUTED_SOURCE] を名乗らせた分）。件数だけを出す
            Log.w(TAG, Telemetry.line("unreadable_source_unknown", source = null, count = unattributed))
        }
        // **1 度の保存で全部入れる。** ソースごとに分けて呼ぶと、途中で保存に失敗したとき
        // 済んだ分が印に残らず、次の見回りで**二重に数えられる**
        if (ledger.unreadable(fresh)) {
            runCatching { mark.writeText(total.toString()) }.onFailure {
                Log.w(TAG, Telemetry.line("unreadable_mark_failed", source = null, error = it.javaClass.simpleName))
            }
        }
    }

    /**
     * そのソースの「書けなかった記録」の数え。知らない名前なら null（**黙って他のソースに混ぜない**）。
     *
     * 失うのは固定長の数えの 1 件だけで、**その記録そのものの破棄は `lost` の側が
     * `item.logicalSource` で報告する**（`newOutbox`）。
     */
    private fun writeFailedFor(logicalSource: String): WriteFailedLedger? {
        val counter = writeFailed[logicalSource]
        if (counter == null) {
            Log.w(TAG, Telemetry.line("write_failed_unknown_source", source = logicalSource))
        }
        return counter
    }

    private fun eventInstant(item: IngestRequest): Instant? = try {
        Instant.parse(item.eventTime)
    } catch (e: DateTimeParseException) {
        null
    }

    /**
     * 前景サービスの種別（design D5 のリスク / tasks 1.4）。
     *
     * **`location` は位置の権限がある間だけ要求する。** targetSdk 34 以降、権限を持たないまま
     * その種別で `startForeground` を呼ぶと `SecurityException` で立てられず、
     * **2 本目のソース（アプリ利用）まで道連れに止まる** —— 本人の決定 Q7
     * 「欠けたソースだけを止め、他は取り続ける」が壊れる。
     *
     * **権限があるときは `location` 1 本のまま**にする（振る舞いを変えない）。
     *
     * 位置が無い間の種別は **`specialUse`**（tasks 5.1 / design D5 のリスクの落とし所）。
     * `dataSync` は Android 15 以降**24 時間のうち 6 時間で打ち切られる**ので、
     * 位置を拒んだままの端末（＝アプリ利用だけを取る端末）では**毎日 18 時間
     * 収集が止まる**ことになり、本人の決定 Q7「欠けたソースだけ止め、他は取り続ける」を
     * 半分しか守れない。`specialUse` にその上限は無い。
     * 打ち切られた場合の備えは [onTimeout]（畳んで落ちない）。
     */
    private fun foregroundServiceType(): Int =
        if (checkSelfPermission(Manifest.permission.ACCESS_FINE_LOCATION) == PackageManager.PERMISSION_GRANTED) {
            ServiceInfo.FOREGROUND_SERVICE_TYPE_LOCATION
        } else {
            ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE
        }

    /** ログに出す種別の名前。**値そのものは出さない**（数でも私的でもないが、読めない）。 */
    private fun foregroundServiceTypeKind(type: Int): String = when (type) {
        ServiceInfo.FOREGROUND_SERVICE_TYPE_LOCATION -> FOREGROUND_LOCATION
        ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE -> FOREGROUND_SPECIAL_USE
        else -> FOREGROUND_OTHER
    }

    /**
     * 収集を始める。**取得条件を見ずに始める**（design D5 / 本人の決定 Q7 / spec
     * 「収集の開始を、ソースの取得条件が満たされているかに依らず行う」）。
     *
     * ST06 より前はここで位置の権限を見て `stopSelf()` していた —— **止まっている間は
     * 生存信号も出ない**ので、受け手の画面には③「動いていたが取れない状態」ではなく
     * ⑥「途絶」が出て、アプリ利用はその間 1 件も取れなかった（10 日を越えれば二度と取れない）。
     */
    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        // **1 本ずつ独立に。** 取得条件が欠けたソースは `collect()` を呼ばず、生存信号だけを出す。
        // **主糸では回さない**（[runCollection]）—— 初回の集計の取り込みは 4 粒度ぶんの問い合わせと
        // 数千件の追記になり、`onStartCommand` は主糸で呼ばれる
        for (running in sources) runCollection { collect(running) }
        startFlushing()
        startBeating()
        startTicking()
        // 落とされても OS に立て直させる。1 年間途切れないことが成功条件 1
        return START_STICKY
    }

    /**
     * そのソースの取得契機を 1 回回す。**取得条件が欠けていれば `collect()` を呼ばない**
     * （design D5）。呼ばなくても生存信号は出続けるので、区間は③として残る。
     *
     * **1 本が投げても他を止めない**（本人の決定 Q7）。落ちると次の起動まで収集が止まり、
     * `START_STICKY` と合わさってクラッシュループになる。
     */
    private fun collect(running: Running) {
        val source = running.source
        val capability = capabilityOf(running)
        if (!capability.capturable) {
            // **何が満たされていないかはログにも残す**（出すのは種別だけ。製造準備 A-2）。
            // **同じ理由が続くあいだは 1 度だけ**（[Running.lastUnavailable]）
            val reason = capability.blockers.joinToString("+")
            if (running.lastUnavailable != reason) {
                Log.i(TAG, Telemetry.line("source_unavailable", source = source.logicalSource, error = reason))
                running.lastUnavailable = reason
            }
            return
        }
        // 取れる状態に戻った。次に取れなくなったらまた 1 行出す
        running.lastUnavailable = null
        val now = Instant.now()
        // **窓の長さはそのソースの取得間隔**（位置は FR-1 の 60 秒）。ソースが自分で名乗る
        val window = CollectionWindow(now.minusMillis(source.intervalMs), now)
        val result = runCatching { source.collect(window) }.getOrElse { e ->
            // 権限が無い（`SecurityException`）か、取得元が投げた。**落とさずに何もしない**（tasks 6.2）
            val kind = if (e is SecurityException) "no_permission" else "collect_crashed"
            Log.w(TAG, Telemetry.line(kind, source = source.logicalSource, error = e.javaClass.simpleName))
            return
        }
        when (result) {
            // **取得できた回数を数える。0 件でも成功**（本人の決定 C7）——
            // 「置き場に書けた件数」ではない。書けなかった分は破棄の報告が持つ
            is CollectionResult.Collected -> running.counters.recordSuccess()
            // **読めなかったは成功に数えない**（0 件と取り違えると、その期間は取り直されない）
            is CollectionResult.Unavailable ->
                Log.w(TAG, Telemetry.line("collect_unavailable", source = source.logicalSource, error = result.reason))
            // 契機は取得元が配る（位置）。数えはその callback（`onFix`）が進める
            CollectionResult.Streaming -> running.selfDriven = true
        }
    }

    /**
     * いま取得できる状態か。**読めなければ「取れない」に倒す**（`androidCapability` の R31 と同じ規律）——
     * 貫通させると起動時の生存信号がプロセスごと落とし、`START_STICKY` でクラッシュループになる。
     */
    private fun capabilityOf(running: Running): Capability =
        runCatching { readCapability(running.source) }.getOrElse { e ->
            Log.w(
                TAG,
                Telemetry.line(
                    "capability_unreadable",
                    source = running.source.logicalSource,
                    error = e.javaClass.simpleName,
                ),
            )
            // 理由の無い「取れない」は組み立てられない（`Capability` の `require`）。
            // 読めなかったのは端末の口なので、権限の側に寄せる（アプリ利用に `sensor` は無い。C9）
            Capability(capturable = false, blockers = listOf(Capability.PERMISSION))
        }

    /**
     * ソースごとの取得の刻みを張る（design D5）。
     *
     * **取得条件を見ずに張る** —— 張らないと、後から許可した本人の端末が
     * 次の取得契機で戻ってこられない（spec「後から許可すると次の契機から集まる」）。
     */
    private fun startTicking() {
        for (running in sources) {
            if (running.ticker != null) continue
            running.ticker = newSourceScheduler(running.cadence.logicalSource).also { scheduler ->
                scheduler.every(running.cadence.intervalMs) {
                    // **契機を自分で配るソース（位置）には何もしない** ——
                    // 登録を張り直すのは `onStartCommand` の役目（ST06 より前からの復旧の経路）
                    if (!running.selfDriven) collect(running)
                }
            }
        }
    }

    /** 5 分ごとにその時点の未送信をまとめて送る（design D9）。 */
    private fun startFlushing() {
        if (flusher != null) return
        if (!Config.isComplete) {
            // 設定が無いなら送らない。**取得は続ける** —— 記録は未送信に積まれ、後から送れる
            Log.w(TAG, Telemetry.line("not_configured", source = null))
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
        // **ソースごとに 1 本**（design D5 / tasks 5.1）。取得できないソースも出し続ける ——
        // 出さないと、受け手には③「取れない状態」ではなく⑥「途絶」が出る
        for (running in sources) {
            if (running.beater != null) continue
            val emitter = HeartbeatEmitter(
                outbox = heartbeatOutbox,
                counters = running.counters,
                userId = Config.userId,
                deviceId = deviceId,
                logicalSource = running.cadence.logicalSource,
                capability = { capabilityOf(running) },
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
                Log.w(TAG, Telemetry.line("heartbeat_crashed", source = null, error = it.javaClass.simpleName))
            }
            // **登録簿の想定間隔に合わせる**（tasks 7.1 / 本人の決定 C8）。ずらすと正常な運用が途絶に見える
            running.beater = newHeartbeatScheduler(running.cadence.logicalSource).also { scheduler ->
                scheduler.every(running.cadence.heartbeatIntervalMs) { emitter.emit() }
            }
        }
    }

    override fun onDestroy() {
        if (::sources.isInitialized) {
            for (running in sources) {
                running.source.stop()
                running.ticker?.cancel()
                running.ticker = null
                running.beater?.cancel()
                running.beater = null
            }
        }
        flusher?.cancel()
        flusher = null
        collectorThread.shutdownNow()
        super.onDestroy()
    }

    /**
     * 前景サービスが OS に打ち切られた（Android 15 以降。design D5 のリスク / tasks 5.1）。
     *
     * **`dataSync` の前景サービスは 24 時間のうち 6 時間で打ち切られる。** 打ち切りの通知を
     * 受けて数秒のうちに畳まないと、OS が `ForegroundServiceDidNotStopInTimeException` で
     * プロセスを落とし、`START_STICKY` と合わさって**クラッシュループ**になる ——
     * 位置を拒んだままの端末では 6 時間ごとにそれが起きる。
     *
     * 上限のある種別を**そもそも使わない**（位置が無い間は `specialUse`）ようにしてあるので、
     * ここは**保険**。畳むと収集は止まるが、
     *
     * - アプリ利用は**遡って取れる**（保存した窓の終わりから取り直す。取得元の保持は見込み 10 日）ので、
     *   次にアプリを開いた時点で取りこぼした期間が積まれる
     * - 落ちる側を選ぶと、通知も生存信号も出ないまま 6 時間ごとに立ち上がり続ける
     *
     * どちらの引数でも同じことをする —— Android 15 は種別を伴わない口を、16 は種別つきの口を呼ぶ。
     */
    override fun onTimeout(startId: Int) = foldAfterTimeout(null)

    override fun onTimeout(startId: Int, fgsType: Int) = foldAfterTimeout(fgsType)

    private fun foldAfterTimeout(fgsType: Int?) {
        // 出すのは種別の名前だけ（製造準備 A-2）
        Log.w(
            TAG,
            Telemetry.line(
                "foreground_timeout",
                source = null,
                error = fgsType?.let { foregroundServiceTypeKind(it) } ?: "unknown",
            ),
        )
        stopSelf()
    }

    override fun onBind(intent: Intent?): IBinder? = null

    /** 常駐の通知。**本文だけを差し替える**（未送信の日数。ST04 / 深掘り Q5）。常時出ている通知は増やさない */
    private fun notification(text: String): Notification {
        val manager = getSystemService(NotificationManager::class.java)
        manager.createNotificationChannel(
            // **ソースの名前を持たせない**（tasks 5.4 / R10）。チャネルの識別子は変えない ——
            // 変えると本人が設定した音や表示の扱いが捨てられ、新しいチャネルとして出直す
            NotificationChannel(CHANNEL, "記録の収集", NotificationManager.IMPORTANCE_LOW),
        )
        return Notification.Builder(this, CHANNEL)
            .setContentTitle("あしあと。")
            .setContentText(text)
            .setSmallIcon(android.R.drawable.ic_menu_mylocation)
            .setOngoing(true)
            .setOnlyAlertOnce(true)
            // **常駐の通知から「利用状況へのアクセス」の設定画面へたどれるようにする**
            // （design D6（仮））—— 自動で送るのは初回の 1 度だけなので、
            // 後から許す気になった本人がたどり着ける道がここにしか無い
            .setContentIntent(usageAccessPendingIntent())
            .build()
    }

    /** 常駐の通知を押したときに開く設定画面（design D6（仮））。 */
    private fun usageAccessPendingIntent(): PendingIntent = PendingIntent.getActivity(
        this,
        0,
        usageAccessSettingsIntent(),
        PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
    )

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

        /** 前景サービスを `location` の種別で立てた（位置の権限がある） */
        const val FOREGROUND_LOCATION = "foreground_location"

        /** 前景サービスを `specialUse` の種別で立てた（位置の権限が無い。design D5 のリスク） */
        const val FOREGROUND_SPECIAL_USE = "foreground_special_use"

        /** 上の 2 つ以外（OS が別の種別で打ち切りを知らせてきたときだけ出る） */
        const val FOREGROUND_OTHER = "foreground_other"

        /**
         * ソースの名前が読めない破棄に名乗らせる名前。
         *
         * 退避先の行が**壊れていて `logical_source` を読めない**ときに使う。
         * 報告を落とす側には倒せない（扉 #14 の証拠は捨てたら戻らない）が、
         * 契約は**登録簿にある名前しか受け付けない**（`unknown_source` で断られる）ので、
         * どれか 1 つを名乗るしかない。位置を選ぶのは、**この置き場のどの版にも在った
         * 唯一のソース**だから（古い行ほど位置である確率が高い）。
         * どれだけ当たったかは `unreadable_source_unknown` の件数で見える。
         */
        val UNATTRIBUTED_SOURCE: String = SourceCadence.LOCATION.logicalSource

        /** 壊れた行からソースの名前を探す幅（`logical_source` は 3 番目の欄）。 */
        const val UNREADABLE_SNIFF = 512
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
                        Log.w("ashiato", Telemetry.line("tick_crashed", source = null, error = e.javaClass.simpleName))
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
