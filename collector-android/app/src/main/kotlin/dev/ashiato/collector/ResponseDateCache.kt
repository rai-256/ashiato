// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

/**
 * 受け口から最後に受け取った応答の `Date`（ST05 / design D2）。**メモリだけ**に持つ。
 *
 * - **取り出すと空になる** —— 「前回の測定より後の応答だけを使う」を構造で守る
 * - **時計の変更の通知で空になる** —— 変更より前の応答の日付は、いまの時計と比べる基準にならない。
 *   空にした事実を持ち、次に取り出すときに `clock_changed_since` として返す
 *
 * 網も送信も知らない。値を置くのは送る側、読むのは測る側。
 */
class ResponseDateCache {
    /** 応答の `Date` 見出し（文字列そのまま）と、それを受け取ったときの単調時計・壁時計。 */
    data class Received(val date: String?, val monoBeforeMs: Long, val monoAfterMs: Long, val wallAfterMs: Long, val host: String? = null)

    sealed interface Taken {
        data class Got(val received: Received) : Taken

        /** 使える応答が無い。`reason` は `no_response_since_last` / `clock_changed_since`。 */
        data class Missing(val reason: String) : Taken
    }

    private var last: Received? = null
    private var clearedByClockChange = false

    /** 受け取った最後の 1 件に置き換える。 */
    @Synchronized
    fun put(received: Received) {
        last = received
        clearedByClockChange = false
    }

    /** 時計の変更の通知。それまでに受け取った応答は使わない。 */
    @Synchronized
    fun clockChanged() {
        last = null
        clearedByClockChange = true
    }

    /** 取り出して空にする。 */
    @Synchronized
    fun take(): Taken {
        val got = last
        val changed = clearedByClockChange
        last = null
        clearedByClockChange = false
        return when {
            got != null -> Taken.Got(got)
            changed -> Taken.Missing(CLOCK_CHANGED_SINCE)
            else -> Taken.Missing(NO_RESPONSE_SINCE_LAST)
        }
    }

    companion object {
        const val NO_RESPONSE_SINCE_LAST = "no_response_since_last"
        const val CLOCK_CHANGED_SINCE = "clock_changed_since"
    }
}
