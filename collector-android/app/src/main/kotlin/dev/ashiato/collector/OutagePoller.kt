// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import java.time.OffsetDateTime
import java.util.concurrent.ExecutorService
import java.util.concurrent.Executors

/** `GET /outages?device_id=` の 1 回の問い合わせ。試験では偽物に差し替える。 */
fun interface OutageQuery {
    fun fetch(deviceId: String): Outcome
}

@Serializable
private data class OutageItemDto(
    @SerialName("episode_id") val episodeId: Long,
    @SerialName("logical_source") val logicalSource: String,
    @SerialName("display_name") val displayName: String,
    @SerialName("last_alive_at") val lastAliveAt: String,
)

@Serializable
private data class OutagesDto(val status: String, val outages: List<OutageItemDto>? = null)

/**
 * 途切れの問い合わせ（ST14 / design D12）。生存信号の契機（**位置**のスケジューラだけ）で `poll()` が呼ばれ、
 * `GET /outages` → 決め方 → 通知 → 覚えの書き込みを**単一スレッドの executor で直列にする**。
 * ソースごとに並行に走ると、どれも「まだ鳴らしていない」を読んで 2 回以上鳴る。
 * OS の定期実行の仕組みは使わない（ST17 の仕組みと重ならない）。
 */
class OutagePoller(
    private val query: OutageQuery,
    private val notice: OutageNotice,
    private val notifier: OutageNotifier,
    private val deviceId: () -> String,
    private val log: (String) -> Unit = {},
    private val executor: ExecutorService = Executors.newSingleThreadExecutor(),
) {
    /** 契機ごとに 1 回。呼び出し元を待たせない（失敗しても収集は止めない）。 */
    fun poll() {
        executor.execute {
            runCatching { pollOnce() }.onFailure {
                log(Telemetry.line("outage_poll_crashed", source = null, error = it.javaClass.simpleName))
            }
        }
    }

    fun shutdown() {
        executor.shutdownNow()
    }

    private fun pollOnce() {
        val decision = OutageNotice.decide(notice.load(), parse(query.fetch(deviceId())))
        val delivered = when (val action = decision.action) {
            NoticeAction.Nothing -> true
            NoticeAction.Dismiss -> true.also { notifier.dismiss() }
            is NoticeAction.Show -> notifier.show(action.outages, action.sound)
        }
        // 出せなかったときは覚えを進めない（権限が戻ったら鳴る）
        if (delivered) notice.save(decision.next)
    }

    private fun parse(outcome: Outcome): OutageResult {
        val dto = (outcome as? Outcome.Responded)?.takeIf { it.status in HTTP_OK }
            ?.let { runCatching { JSON.decodeFromString<OutagesDto>(it.body) }.getOrNull() }
        return when (dto?.status) {
            "warming_up" -> OutageResult.WarmingUp
            "ready" -> runCatching { OutageResult.Ready(dto.outages.orEmpty().map(::toOutage)) }
                .getOrDefault(OutageResult.Unreachable)
            else -> OutageResult.Unreachable
        }
    }

    private fun toOutage(it: OutageItemDto) = Outage(
        episodeId = it.episodeId,
        logicalSource = it.logicalSource,
        displayName = it.displayName,
        lastAliveAt = OffsetDateTime.parse(it.lastAliveAt).toInstant(),
    )

    private companion object {
        val JSON = Json { ignoreUnknownKeys = true }
        val HTTP_OK = 200..299
    }
}
