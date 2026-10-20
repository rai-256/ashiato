// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

/** 生存信号を送るメディアの 1 本のソース（design D22）。数えと、区間の失敗の段を持つ。 */
class MediaChannel(val cadence: SourceCadence, val counters: AttemptCounters) {
    val stages = MediaStageLog()
}

/** メディアの 4 つのソース（`c01-photo` / `c01-screenshot` / `c01-image` / `c01-video`）。 */
val MEDIA_CADENCES: List<SourceCadence> = listOf(
    SourceCadence.PHOTO,
    SourceCadence.SCREENSHOT,
    SourceCadence.IMAGE,
    SourceCadence.VIDEO,
)

/**
 * 読み手の契機（知らせ・6 時間の見直し・1 日 1 回の突き合わせ）を、4 つのソースの数えと取得可否、知らせに配る（design D22）。
 *
 * **読み手は 4 つのソースで 1 本**（一覧は 1 つ）。1 回の契機は 4 つのソースのそれぞれに 1 試行を数える —— そのソースの項目が 0 件でも。
 * 位置の取得契機の流れ（親の `collect()`）には乗らない: 「選んだ写真だけ」でも読むので、取得可否で契機を止められない。
 */
class MediaCapture(
    private val channels: List<MediaChannel>,
    private val permissions: () -> MediaPermissionState,
    private val notice: (MediaPermissionState) -> Unit,
    private val listing: MediaListing,
    /** 読み手と刻みを組む。結果の宛先を渡す（読み手の環境に入る） */
    driver: (onOutcome: (MediaScanOutcome) -> Unit) -> MediaScanDriver,
    private val log: (String) -> Unit = {},
) {
    private val driver = driver(::onOutcome)
    private var started = false
    private var unobserve: (() -> Unit)? = null

    /** 最後に知らせに渡した権限の状態（見回りが「変わった」を見る元） */
    @Volatile
    private var lastState: MediaPermissionState? = null

    /** 始める。**呼ばれるたびに知らせは見直す**（権限が戻った後の開き直しで消す）。読み手の刻みは 1 度だけ。 */
    @Synchronized
    fun start() {
        refreshNotice()
        if (started) return
        started = true
        driver.start()
        driver.watch(check = ::permissionsChanged)
        unobserve = runCatching { listing.observe(driver::onMediaChanged) }.getOrElse {
            log(Telemetry.line("media_observe_failed", source = null, error = it.javaClass.simpleName))
            null
        }
    }

    @Synchronized
    fun stop() {
        started = false
        unobserve?.invoke()
        unobserve = null
        driver.stop()
    }

    /** そのソースのいまの取得可否（接続は含めない。D7）。 */
    fun capability(logicalSource: String): Capability {
        val stage = channels.firstOrNull { it.cadence.logicalSource == logicalSource }?.stages?.blockedStage()
        return mediaCapability(permissions(), stage)
    }

    fun refreshNotice() {
        runCatching {
            val state = permissions()
            lastState = state
            notice(state)
        }.onFailure {
            log(Telemetry.line("media_notice_crashed", source = null, error = it.javaClass.simpleName))
        }
    }

    /**
     * 権限の見回り（[MEDIA_PERMISSION_POLL_MS] ごと）。前に見た状態と違えば知らせを見直し、真を返す（読み手がすぐ見直す）。
     * 許可にも取り消しにも同じく動く —— 読み手は読む前に自分で権限を見る（Q15）ので、取り消しで読み直しても何も開かない。
     */
    private fun permissionsChanged(): Boolean {
        val state = runCatching { permissions() }.onFailure {
            log(Telemetry.line("media_permission_unreadable", source = null, error = it.javaClass.simpleName))
        }.getOrNull()
        val changed = state != null && state != lastState
        if (changed) refreshNotice()
        return changed
    }

    private fun onOutcome(outcome: MediaScanOutcome) {
        for (channel in channels) {
            val stage = outcome.failedStages[channel.cadence.logicalSource]
            if (outcome.unavailable == null && stage == null) {
                channel.counters.recordSuccess()
                channel.stages.succeeded()
            } else {
                channel.counters.recordFailure()
                channel.stages.failed(stage)
            }
        }
        refreshNotice()
    }
}

/** メディアの 1 本のソースを [CollectionSource] の口に載せる。契機は [MediaCapture] が自分で配る。 */
class MediaSourceAdapter(
    cadence: SourceCadence,
    private val capabilityOf: (String) -> Capability,
) : CollectionSource {
    override val logicalSource: String = cadence.logicalSource
    override val intervalMs: Long = cadence.intervalMs

    override fun capability(context: android.content.Context): Capability = capabilityOf(logicalSource)

    /** 契機は [MediaCapture] が配る（位置と同じ「自分で配る」）。親は何も積まない */
    override fun collect(window: CollectionWindow): CollectionResult = CollectionResult.Streaming
}
