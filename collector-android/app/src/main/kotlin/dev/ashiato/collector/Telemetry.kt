// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

/**
 * ログに出してよいものだけを組み立てる。
 *
 * **位置の値・原文・利用者を特定できる内容は出さない**（製造準備 A-2 /
 * specs/device-collection「私的な内容を記録の外に出さない」）。
 * ログは記録本体と違って感度の制御（PERM-2）が効かないので、
 * 一度出たものは後から締められない。
 *
 * 出してよいのは **件数・ソース名・所要時間・エラーの種別** の 4 つだけ。
 * だから引数の型を `Int` と「種別」の文字列に絞ってある —— 値を渡す口を用意しない。
 */
object Telemetry {
    fun line(kind: String, count: Int? = null, elapsedMs: Long? = null, error: String? = null): String =
        buildString {
            append("kind=").append(kind)
            append(" source=").append(LOGICAL_SOURCE)
            count?.let { append(" count=").append(it) }
            elapsedMs?.let { append(" elapsed_ms=").append(it) }
            error?.let { append(" error=").append(it) }
        }
}
