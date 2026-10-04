// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.io.File
import java.io.IOException
import java.time.Instant

/**
 * 次の契機が取り直す幅（本人の決定 C4「窓は重ねて進める」。design D8（仮））。
 *
 * 取得元の範囲は `[begin, end)` なので、保存した終わりちょうどのイベントは
 * 次の窓の始まりに入る。それでも**手前から**取り直すのは、取得元が 1 件を書き終える
 * 時点と問い合わせの時点がずれうるため。重なった分はサーバの冪等が畳む
 * （同じイベントの原文は毎回同じ文字列になる ＝ design D2 / tasks 3.1）。
 *
 * **反転条件**: 重ねても境界のイベントが落ちる実測が出たら広げ、
 * 端末の未送信が重複で膨らむ実測（tasks 7.1）が出たら狭める。
 * 30 分の契機に対して 1 分は約 3 %。計算し直せば戻る値なので（仮）で置く。
 */
const val USAGE_WINDOW_OVERLAP_MS: Long = 60_000

/**
 * 端末の時計の前進と単調な経過の食い違いを「飛んだ」と見なす幅（本人の決定 C6。design D8（仮））。
 *
 * **狭すぎると、時刻合わせのたびに取得が止まる**（網の時刻合わせは数秒〜数分ずれうる）。
 * **広すぎると、ずれた時刻のまま取り直して行が増える**（取得元は時計の変化で統計をずらす）。
 * 1 時間は [AgeClock] が「時計の飛び」をログに残すときの幅と同じ ——
 * 同じ現象に 2 つの閾値を持たない。
 *
 * **反転条件**: 時刻合わせで取得が止まる実測が出たら広げる。計算し直せば戻る値。
 */
const val USAGE_CLOCK_SKEW_TOLERANCE_MS: Long = 60 * 60 * 1000L

/**
 * 1 度も取れていないときに遡る幅（本人の決定 C3「遡りは OS が持っている分だけ。
 * こちらで短く切らない」。design D8（仮））。
 *
 * **見込みの保持（[UsageRetention] の 10 日）で切らない** —— 10 日は AOSP の実装から
 * 読んだ見込みで、端末の造りや OS の版でずれうる（spec レビュー R3）。
 * 切る側に倒すと、**まだ取得元に残っているイベントを飛ばす**（飛ばした分は消える）。
 * 見込みの 3 倍まで手を伸ばし、取得元が持っている分だけ返させる。
 *
 * **反転条件**: 取得元が 10 日より長く持っている端末が見つかったら広げる。
 * 広げても費用は初回の 1 問い合わせだけ（返るのは取得元が持っている分だけ）。
 */
const val USAGE_FIRST_WINDOW_MS: Long = 30 * 24 * 60 * 60 * 1000L

/**
 * 保存された「どこまで取ったか」（tasks 3.2）。
 *
 * 単調な経過（[AgeClock]）を**終わりと一緒に**持つ ——
 * 端末の時計が飛んだかどうかは、2 つの時計の**差の食い違い**でしか分からない。
 * 片方だけを保存すると、次の契機に比べる相手が無い。
 */
data class UsageWindowMark(
    /** 前回の取得の窓の終わり（端末の壁時計） */
    val end: Instant,
    /** その終わりを保存したときの単調な経過（[AgeClock.now]） */
    val ageMs: Long,
    /**
     * そのときの**数えなかった前進の累計**（[AgeClock.discardedMs]）。
     *
     * 次の契機との差を取ると「この区間で経過が数え落とした分」が出る ——
     * 長い電源断はその分だけ壁時計と経過を食い違わせるので、差し引けば
     * **見かけの食い違いだけが消え、本物の時計の飛びは残る**（独立レビュー Important 1）。
     */
    val discardedMs: Long,
)

/** 窓の置き場の名前。**ソースごとに別ファイル**（数えの置き場と同じ規律）。 */
fun usageWindowFile(dir: File, logicalSource: String): File = File(dir, "usage-window-$logicalSource.txt")

/**
 * 窓の終わりを端末の保存領域に置く（spec「収集の停止と再開をまたいで保持する」）。
 *
 * **インスタンスの中だけに持たない** —— `START_STICKY` の立て直しで消えると、
 * そのたびに初回として遡ることになり、取り直した分だけ未送信が膨らむ。
 *
 * 読めなければ**新品として始める**（落とさない）。失うのは「どこまで取ったか」だけで、
 * イベントそのものは取得元にまだある。
 */
class UsageWindowStore(
    private val file: File,
    /** この置き場が覚えているソース。**ログはその名を名乗る**（tasks 1.2）。 */
    private val logicalSource: String,
    private val log: (String) -> Unit,
) {
    fun load(): UsageWindowMark? = try {
        if (!file.exists()) {
            null
        } else {
            val parts = file.readText().trim().split(" ")
            UsageWindowMark(Instant.parse(parts[0]), parts[1].toLong(), parts[2].toLong())
        }
    } catch (e: RuntimeException) {
        log(Telemetry.line("usage_window_unreadable", source = logicalSource, error = e.javaClass.simpleName))
        null
    } catch (e: IOException) {
        log(Telemetry.line("usage_window_unreadable", source = logicalSource, error = e.javaClass.simpleName))
        null
    }

    /**
     * 書く。**書いてから差し替える**（[AgeClock] と同じ）—— 途中で落ちて壊れると
     * 次の起動が初回として遡り、取り直した分が未送信に積み上がる。
     */
    fun save(mark: UsageWindowMark) {
        try {
            file.parentFile?.mkdirs()
            val tmp = File(file.parentFile, "${file.name}.tmp")
            tmp.writeText("${mark.end} ${mark.ageMs} ${mark.discardedMs}")
            if (!tmp.renameTo(file)) throw IOException("rename")
        } catch (e: IOException) {
            log(Telemetry.line("usage_window_save_failed", source = logicalSource, error = e.javaClass.simpleName))
        }
    }
}
