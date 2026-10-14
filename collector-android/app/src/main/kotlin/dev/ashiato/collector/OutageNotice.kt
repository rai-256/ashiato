// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.content.SharedPreferences
import java.time.Instant

/** 途切れているソース 1 つ。**記録の中身は持たない**（ST14 / C5）。 */
data class Outage(
    val episodeId: Long,
    val logicalSource: String,
    val displayName: String,
    val lastAliveAt: Instant,
)

/** `GET /outages` の結果（design D13）。 */
sealed interface OutageResult {
    /** サーバに届かなかった・読めなかった */
    data object Unreachable : OutageResult

    /** サーバが起動直後で判定を待っている */
    data object WarmingUp : OutageResult

    data class Ready(val outages: List<Outage>) : OutageResult
}

/** 通知について覚えていること。`sounded` は音を鳴らした途切れの識別子、`shown` は通知が出ているか。 */
data class NoticeState(val sounded: Set<Long> = emptySet(), val shown: Boolean = false)

/** 通知に対してやること。 */
sealed interface NoticeAction {
    data object Nothing : NoticeAction

    /** 黙って消す */
    data object Dismiss : NoticeAction

    /** 全部の表示名を並べて出す。`sound` が false なら音なしの差し替え */
    data class Show(val outages: List<Outage>, val sound: Boolean) : NoticeAction
}

data class NoticeDecision(val action: NoticeAction, val next: NoticeState)

/**
 * 通知の決め方（design D13 の表。Rust の `outage_notice` と同じ表）。
 *
 * 次の `sounded` を「いまの一覧の識別子」にするので、戻った途切れの識別子は消え、
 * 戻った後にまた途切れると新しい識別子になってまた鳴る。
 * 鳴らした覚え（`SharedPreferences`）は収集アプリを起動し直しても残る。
 */
class OutageNotice(private val prefs: SharedPreferences) {
    fun load(): NoticeState = NoticeState(
        sounded = prefs.getStringSet(SOUNDED, emptySet()).orEmpty().mapNotNull { it.toLongOrNull() }.toSet(),
        shown = prefs.getBoolean(SHOWN, false),
    )

    fun save(state: NoticeState) {
        prefs.edit()
            .putStringSet(SOUNDED, state.sounded.map { it.toString() }.toSet())
            .putBoolean(SHOWN, state.shown)
            .apply()
    }

    companion object {
        const val PREFS = "outage_notice"
        private const val SOUNDED = "sounded"
        private const val SHOWN = "shown"

        fun decide(state: NoticeState, result: OutageResult): NoticeDecision = when (result) {
            OutageResult.Unreachable, OutageResult.WarmingUp -> NoticeDecision(NoticeAction.Nothing, state)
            is OutageResult.Ready -> decideReady(state, result.outages)
        }

        private fun decideReady(state: NoticeState, outages: List<Outage>): NoticeDecision {
            if (outages.isEmpty()) {
                val action = if (state.shown) NoticeAction.Dismiss else NoticeAction.Nothing
                return NoticeDecision(action, NoticeState())
            }
            val ids = outages.map { it.episodeId }.toSet()
            val sound = ids.any { it !in state.sounded }
            return NoticeDecision(NoticeAction.Show(outages, sound), NoticeState(ids, shown = true))
        }
    }
}
