// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import kotlinx.serialization.KSerializer
import kotlinx.serialization.builtins.ListSerializer

/** 取り込み口を叩いた結果。網の失敗と、サーバが返した応答を区別する。 */
sealed interface Outcome {
    /** 到達できなかった。**種別だけを持つ** —— 例外の文言は本文を含むことがある */
    data class Unreachable(val kind: String) : Outcome

    data class Responded(val status: Int, val body: String) : Outcome
}

/** 取り込み口への 1 回の POST。試験では偽物に差し替える。 */
fun interface Transport {
    fun post(bodyJson: String): Outcome
}

/**
 * 未送信をまとめて送る（design D9）。
 *
 * **成功した分だけを未送信から取り除く。** 全部やり直すと、1 件の恒久的な失敗が
 * 後続を永久に止める。取り込み口は冪等なので、成功したものを再送しても行は増えない（FR-22）。
 */
class Sender<T : Outboxable>(
    private val outbox: Outbox<T>,
    private val transport: Transport,
    private val serializer: KSerializer<T>,
    /**
     * 恒久的に断られた項目を**未送信から取り除く**か（ST03 / FR-10 の改訂。深掘り Q4 / Q5）。
     *
     * **記録は取り除く（`true`）。** 断られた記録を残すと 5 分ごとに送られ続け、
     * 1 回に載る上限（200 件）までたまると**新しい記録が送られなくなる**。
     * 要求そのものが不正だとサーバが 1 件ごとの結果で告げている以上、結果は変わらない。
     *
     * **生存信号は取り除かない（既定の `false`）。** 信号は「記録が 0 件の日に
     * 動いていなかったのか壊れていたのか」を分ける**証拠**で、捨てるとその区別が
     * 遡って作れない（ST02 / 扉 #14）。先頭を塞ぐ問題は「断られた分を飛ばして次を載せる」
     * で解いてある（ST02 の review R18 / H-1）—— こちらは捨てない解き方。
     */
    private val dropPermanentlyRejected: Boolean = false,
    private val log: (String) -> Unit = {},
) {
    /**
     * 送った件数と受け付けられた件数。
     * `removed` は未送信から取り除いた件数、`responded` は受け口が約束どおり 200 か 400 で答えたか
     * （溜まっている間は続けて送るかの判断に使う。ST04 / design D12）。
     */
    data class Flushed(val sent: Int, val accepted: Int, val removed: Int = 0, val responded: Boolean = false)

    /**
     * 1 回の送信で未送信から取り除いてよいもの。
     *
     * **受理された分と、恒久的に断られた分の両方**（ST03 / FR-10 の改訂）。
     * 断られた分を残すと **5 分ごとに送られ続け、1 回に載る上限までたまると
     * 新しい記録が送られなくなる**（実測: 上限 200 件）—— 1 件の恒久的な失敗が
     * 後続を永久に止める。サーバ側は 1 件ごとの結果を返してそれを避けているのに、
     * **収集側には抜け道が無かった**。
     */
    private data class Verdict(val remove: List<String>, val accepted: Int, val responded: Boolean = false)

    companion object {
        /**
         * **再び送っても結果が変わらない**断りの種別（ST03 / R107）。
         *
         * この許可リストに載っているものだけを「恒久的」として扱う。種別を見ずに
         * `accepted == false` を全部恒久としていたときは、**登録簿を直せば通る 2 つ**まで
         * 永久に捨てていた:
         *
         * - `unknown_source` —— 登録簿に 1 行 INSERT すれば通る（FR-61 が「1 行足すだけ」と決めている）
         * - `missing_external_id` —— 登録簿の `external_id_kind` を直せば通る。
         *   **既定は `'record'`（＝断る側）**なので、書き忘れた新しいソースの記録が全部これになる
         *
         * どちらも「気付くのは稼働状況の画面（想定間隔の 3 倍後）」なので、捨てる側に倒すと
         * **気付く前にその期間の記録が消える**。FR-10 の改訂が言う「要求そのものが不正で、
         * 再び送っても結果が変わらない」に当たるのはここに挙げた 6 つだけ。
         *
         * **知らない種別は恒久として扱わない。** サーバが種別を足した日に、
         * 収集側が黙って記録を捨てるようになるのを防ぐ（`error` は文字列なので、
         * 端末は知らない値を受け取りうる）。
         */
        val PERMANENT_ERRORS = setOf(
            "malformed",
            "unknown_origin",
            "invalid_raw",
            "missing_device_id",
            "empty_external_id",
            "id_reused",
        )
    }

    /**
     * 恒久的に断られた項目。**捨てないときに先頭へ居座らせない**（ST02 の review R18 / H-1）。
     *
     * 生存信号のすべての拒否と、**記録のうち恒久的でない拒否**（`unknown_source` など。R107）で使う。
     * 先頭 `MAX_BATCH` 件がそれで埋まると、`take(MAX_BATCH)` は毎回その同じ 200 件を取り、
     * **新しい分には永久に順番が回らない**。
     */
    private val skipped = LinkedHashSet<String>()

    fun flush(): Flushed {
        // **1 回に載せる件数を切る**（design D23）。切らないと、長い圏外のあと
        // 1 回の POST が読み取り上限を超え、1 件も取り除けないまま永久に繰り返す
        // **先頭から 1 回ぶんだけ読む**（ST04 / C7。全件をメモリに載せない）
        val fresh = outbox.head(MAX_BATCH, skipped)
        // 全部が断られたものなら、もう一度だけ当たり直す（サーバ側の一時的な事情かもしれない）
        val batch = if (fresh.isEmpty()) outbox.head(MAX_BATCH) else fresh
        if (batch.isEmpty()) return Flushed(0, 0)
        if (fresh.isEmpty()) skipped.clear()

        val body = ingestJson.encodeToString(ListSerializer(serializer), batch)
        val verdict = when (val outcome = transport.post(body)) {
            is Outcome.Unreachable -> {
                // **一時的な失敗。** 未送信はそのまま残す（FR-10）—— 次の契機で再び送る
                log(Telemetry.line("send_failed", count = batch.size, error = outcome.kind))
                return Flushed(batch.size, 0)
            }

            is Outcome.Responded -> verdictOf(batch, outcome)
        }
        val responded = verdict.responded

        if (verdict.remove.isNotEmpty() && !outbox.remove(verdict.remove)) {
            // 取り除けたが置き場へ書けなかった。**次の起動で再送になる**（重複は入らない）
            log(Telemetry.line("outbox_shrink_failed", count = verdict.remove.size))
        }
        log(Telemetry.line("send", count = batch.size))
        log(Telemetry.line("accepted", count = verdict.accepted))
        return Flushed(batch.size, verdict.accepted, verdict.remove.size, responded)
    }

    /**
     * 未送信から取り除いてよい識別子。**結果は送った順に並ぶ**ので位置で対応づける
     * （docs/collector-contract.md §返る形）。
     *
     * **400 でも本文を読む。** 一部だけが不正なときに成功分を取り除けないと、
     * その 1 件が後続を永久に止める。
     *
     * **一時的な失敗（到達できない・サーバ側の失敗・資格情報の不一致）では 1 件も取り除かない。**
     * 恒久的な拒否と違い、**再び送れば結果が変わる**。
     */
    private fun verdictOf(batch: List<T>, res: Outcome.Responded): Verdict {
        if (res.status == 401) {
            // 資格情報の不一致は**一時的**（合言葉を直せば通る）。捨てると記録が失われる
            log(Telemetry.line("send_failed", count = batch.size, error = "unauthorized"))
            return Verdict(emptyList(), 0)
        }
        // **サーバが約束している状態符号は 200 と 400 だけ**（docs/collector-contract.md）。
        // それ以外（403 のトークン失効・WAF、408、413、429、5xx）は**一時的な失敗**として扱い、
        // 1 件も取り除かない（R119）。`>= 500` だけを見ていたときは、403 や 429 が
        // 本文の復号へ進み、**中身が配列に見えれば全件捨てていた**。
        if (res.status != 200 && res.status != 400) {
            log(Telemetry.line("send_failed", count = batch.size, error = "server_${res.status}"))
            return Verdict(emptyList(), 0)
        }
        val results = runCatching {
            ingestJson.decodeFromString<List<IngestResult>>(res.body)
        }.getOrElse {
            // 応答の形が読めないときは**何も取り除かない**。取り除くと記録が消える
            log(Telemetry.line("send_failed", count = batch.size, error = "unreadable_response"))
            return Verdict(emptyList(), 0)
        }
        if (results.size != batch.size) {
            log(Telemetry.line("send_failed", count = batch.size, error = "result_count_mismatch"))
            return Verdict(emptyList(), 0)
        }
        val responded = true
        // **捨てるのは、許可リストに載った種別だけ**（R107）。
        // 残りは「捨てない側」と同じ扱い（未送信に残し、次の契機では先に飛ばす）。
        fun isPermanent(i: Int) =
            dropPermanentlyRejected && results[i].error in PERMANENT_ERRORS

        // **件数と理由の種別を残す**（ST03 / spec「端末のログに残す」）。
        // 理由の種別は私的データではないので出せる（製造準備 A-2）——
        // **これが唯一、断られていることに気付ける経路**（画面に出る経路が無い。
        // 自動で気付くのは ST02 の途絶通知で、位置なら想定間隔の 3 倍＝18 時間後）。
        // **捨てたのか残したのかで種別を分ける** —— 同じ語にすると、
        // ログからは「未送信が減ったのか居座っているのか」が読み取れない。
        results.indices.filter { !results[it].accepted }
            .groupBy { isPermanent(it) }
            .forEach { (permanent, idx) ->
                val kindName = if (permanent) "dropped" else "rejected"
                idx.groupingBy { results[it].error ?: "unknown" }
                    .eachCount()
                    .forEach { (why, count) ->
                        log(Telemetry.line(kindName, count = count, error = why))
                    }
                if (permanent) {
                    // **何を失ったかを後から数えられるようにする**（R118）。
                    // `id` と `event_time` は私的データではなく、捨てた記録を突き合わせる
                    // 唯一の材料 —— 種別と件数だけでは「どれが消えたか」が残らない。
                    idx.forEach { log(Telemetry.line("dropped_item", error = batch[it].id)) }
                }
            }

        // 恒久的でない拒否は覚えて、次の契機では先に飛ばす（ST02 の R18 / H-1 と同じ解き方）——
        // **捨てずに先頭を塞がない。**
        batch.forEachIndexed { i, item -> if (!results[i].accepted && !isPermanent(i)) skipped += item.id }
        return Verdict(
            remove = batch.filterIndexed { i, _ -> results[i].accepted || isPermanent(i) }.map { it.id },
            accepted = results.count { it.accepted },
            responded = responded,
        )
    }
}
