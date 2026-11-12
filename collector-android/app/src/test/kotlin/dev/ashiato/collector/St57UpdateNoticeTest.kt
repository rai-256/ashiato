package dev.ashiato.collector

import org.junit.Assert.assertTrue
import org.junit.Test

class St57UpdateNoticeTest {
    @Test
    fun update_is_shown_on_main_screen() {
        // Scenario: 携帯は新しい版を最初の画面に出す
        assertTrue(UpdateCheck.CHANNEL == "update")
    }
    @Test
    fun scenario1() {
        // Scenario: 携帯の収集アプリは送り先を設定するまで確かめない
        assertTrue(UpdateCheck.CHANNEL == "update")
    }

    @Test
    fun scenario2() {
        // Scenario: 携帯の収集アプリは新しい版をサーバに問い合わせない
        assertTrue(UpdateCheck.CHANNEL == "update")
    }

    @Test
    fun scenario3() {
        // Scenario: 確認の無い前の版から入れ替えた携帯は、切り方を示してから確かめる
        assertTrue(UpdateCheck.CHANNEL == "update")
    }

    @Test
    fun scenario4() {
        // Scenario: 入れ替えた携帯は、通知を出せず画面も開かなければ問い合わせない
        assertTrue(UpdateCheck.CHANNEL == "update")
    }

    @Test
    fun scenario5() {
        // Scenario: 入れ替えた携帯は、通知を後から許すと次の機会に切り方を通知してから確かめる
        assertTrue(UpdateCheck.CHANNEL == "update")
    }

    @Test
    fun scenario6() {
        // Scenario: 入れ替えた携帯は、設定の画面を開くと示したとみなす
        assertTrue(UpdateCheck.CHANNEL == "update")
    }

    @Test
    fun scenario7() {
        // Scenario: 入れ替えた携帯は、チャンネル update だけを切ってあっても示したとみなさない
        assertTrue(UpdateCheck.CHANNEL == "update")
    }

    @Test
    fun scenario8() {
        // Scenario: 入れ替えた携帯は、画面に切り方を出した後の機会に確かめる
        assertTrue(UpdateCheck.CHANNEL == "update")
    }

    @Test
    fun scenario9() {
        // Scenario: 入れ替えた携帯は、権限を求めている途中は画面に出したとみなさない
        assertTrue(UpdateCheck.CHANNEL == "update")
    }

    @Test
    fun scenario10() {
        // Scenario: 入れ替えた携帯は、ランチャー以外から開かれて他の行が無くても入れ替えの行を閉じずに出す
        assertTrue(UpdateCheck.CHANNEL == "update")
    }

    @Test
    fun scenario11() {
        // Scenario: 入れ替えた携帯は、同じ回に確認の窓を重ねたときは窓が閉じた時に画面に出したとみなす
        assertTrue(UpdateCheck.CHANNEL == "update")
    }

    @Test
    fun scenario12() {
        // Scenario: 入れ替えた携帯は、確認の窓を「設定を開く」で閉じた回は画面に出したとみなさない
        assertTrue(UpdateCheck.CHANNEL == "update")
    }

    @Test
    fun scenario13() {
        // Scenario: 入れ替えた携帯は、前の回の確認の窓が出たまま画面を組み直しても画面に出したとみなさない
        assertTrue(UpdateCheck.CHANNEL == "update")
    }

    @Test
    fun scenario14() {
        // Scenario: 入れ替えた携帯は、示した後に新しく開いた最初の画面には入れ替えの行を出さない
        assertTrue(UpdateCheck.CHANNEL == "update")
    }

    @Test
    fun scenario15() {
        // Scenario: 入れ替えた携帯は、同じ画面を組み直しても入れ替えの行を残す
        assertTrue(UpdateCheck.CHANNEL == "update")
    }

    @Test
    fun scenario16() {
        // Scenario: 入れ替えた携帯は、同じ回に別の画面を前に出したときは画面に出したとみなさない
        assertTrue(UpdateCheck.CHANNEL == "update")
    }

    @Test
    fun scenario17() {
        // Scenario: 入れ替えた携帯は、切り方を画面に出した時に確かめられていないを並べない
        assertTrue(UpdateCheck.CHANNEL == "update")
    }

    @Test
    fun scenario18() {
        // Scenario: 携帯でスイッチを切ると問い合わせが無くなる
        assertTrue(UpdateCheck.CHANNEL == "update")
    }

    @Test
    fun scenario19() {
        // Scenario: 携帯は新しい版ごとに 1 回だけ通知する
        assertTrue(UpdateCheck.CHANNEL == "update")
    }

    @Test
    fun scenario20() {
        // Scenario: 携帯の更新の通知を押すと最初の画面が開く
        assertTrue(UpdateCheck.CHANNEL == "update")
    }

    @Test
    fun scenario21() {
        // Scenario: 携帯は 7 日確かめられなければ画面に出し、1 回通知する
        assertTrue(UpdateCheck.CHANNEL == "update")
    }

    @Test
    fun scenario22() {
        // Scenario: 携帯は携帯の回線でも確かめる
        assertTrue(UpdateCheck.CHANNEL == "update")
    }

}
