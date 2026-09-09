// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * **本人が決めた間隔を回帰から守る**（tasks 9.6）。
 *
 * この 2 つは技術判断ではなく、**電池と通信量に効くので本人に問うて決めた**もの
 * （深掘り 第 1 回 / design D7 / D9）。どのテストからも参照されていないと、
 * 値を書き換えても全部が緑のまま通り、決定が黙って消える。
 *
 * **変えたくなったら、まず deep.md に戻すこと。** ここを直して済ませてはいけない。
 */
class IntervalTest {
    @Test
    fun `位置の取得は60秒間隔`() {
        // FR-1 が定めている。本人の操作を必要とする収集は途切れる（成功条件 1）
        assertEquals(60_000L, FIX_INTERVAL_MS)
    }

    @Test
    fun `送信は5分間隔`() {
        // design D9（深掘りで本人が決定）。60 秒ごとに 1 件ずつ HTTP を叩くと
        // 電池と通信量を最も消費する。NFR-1 の上限は 1 時間あり余裕がある
        assertEquals(300_000L, SEND_INTERVAL_MS)
    }

    @Test
    fun `送信の間隔は取得の間隔より長い`() {
        // 逆転すると「まとめて送る」が成り立たない（1 件ずつ送ることになる）
        assertTrue(
            "送信 $SEND_INTERVAL_MS ms が取得 $FIX_INTERVAL_MS ms を上回っていない",
            SEND_INTERVAL_MS > FIX_INTERVAL_MS,
        )
    }

    // Scenario: 到達できる間は 1 時間以内に届く
    @Test
    fun `送信の間隔は NFR-1 の 1 時間に収まる`() {
        // **遅延の本体はここ**（review R8）。smoke 手順 18 は台本自身が event_time に
        // 「いま」を入れて即 POST しているので、サーバ側の即時性しか測っていない。
        // 生成から格納までの上限を決めているのは、この送信間隔と再送の周期
        assertTrue(
            "送信間隔 $SEND_INTERVAL_MS ms が NFR-1 の 1 時間を超えている",
            SEND_INTERVAL_MS < 3_600_000L,
        )
    }

    @Test
    fun `1回の送信にたまる件数は5件`() {
        // 契約（docs/collector-contract.md §送る間隔）が言っている「5 分ぶん」の実体。
        // ここがずれると SenderTest の「5 件たまっていても送信は 1 回」が意味を失う
        assertEquals(5L, SEND_INTERVAL_MS / FIX_INTERVAL_MS)
    }
}
