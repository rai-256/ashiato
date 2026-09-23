// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

/**
 * ログに出してよいものだけを組み立てる。
 *
 * **位置の値・アプリの表示名・原文・利用者を特定できる内容は出さない**（製造準備 A-2 /
 * specs/device-collection「私的な内容を記録の外に出さない」）。
 * ログは記録本体と違って感度の制御（PERM-2）が効かないので、
 * 一度出たものは後から締められない。
 *
 * 出してよいのは **件数・ソース名・所要時間・エラーの種別** の 4 つだけ。
 * だから引数の型を `Int` と「種別」の文字列に絞ってある —— 値を渡す口を用意しない。
 *
 * **ソース名は書き手が名乗る**（独立レビュー R10 / tasks 1.2）。ここに 1 本の定数を
 * 焼き込んでいたときは、2 本目のソースが書いたログまで `source=c01-location` と出て、
 * 「位置は取れているのにアプリ利用が断られている」がログから読めなかった。
 * **既定値は置かない** —— 既定があると、名乗り忘れが黙って他人の名前になる。
 */
object Telemetry {
    /**
     * @param source そのログを書いたソースの名前。
     *   **どのソースにも属さない配管**（置き場・時計・刻み）は `null` を渡し、`source=` を出さない
     *   —— 誰の名も騙らないほうが、騙るより読める。
     */
    fun line(
        kind: String,
        source: String?,
        count: Int? = null,
        elapsedMs: Long? = null,
        error: String? = null,
    ): String =
        buildString {
            append("kind=").append(kind)
            source?.let { append(" source=").append(it) }
            count?.let { append(" count=").append(it) }
            elapsedMs?.let { append(" elapsed_ms=").append(it) }
            error?.let { append(" error=").append(it) }
        }
}
