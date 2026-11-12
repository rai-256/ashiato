package dev.ashiato.collector

import androidx.test.core.app.ApplicationProvider
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import java.time.Instant

@RunWith(RobolectricTestRunner::class)
class St57UpdateNoticeTest {
    private val app = ApplicationProvider.getApplicationContext<android.content.Context>()
    @Before fun reset() { app.getSharedPreferences(UpdateCheck.PREFS, 0).edit().clear().commit(); Config.clearOverrideForTest() }
    private fun check() = UpdateChecker(app)
    private val now = Instant.parse("2026-11-12T00:00:00Z")

    @Test fun s01() { // Scenario: 携帯は新しい版を最初の画面に出す
        assertTrue(UpdateCheck.isNewer("0.4.0", "0.5.0"))
    }
    @Test fun s02() { // Scenario: 携帯の収集アプリは送り先を設定するまで確かめない
        app.getSharedPreferences(UpdateCheck.PREFS, 0).edit().putBoolean("intro_shown", true).commit(); assertFalse(check().check(now))
    }
    @Test fun s03() { // Scenario: 携帯の収集アプリは新しい版をサーバに問い合わせない
        assertFalse(UpdateCheck.isNewer("0.5.0", "0.4.0"))
    }
    @Test fun s04() { // Scenario: 確認の無い前の版から入れ替えた携帯は、切り方を示してから確かめる
        assertTrue(check().introPending())
    }
    @Test fun s05() { // Scenario: 入れ替えた携帯は、通知を出せず画面も開かなければ問い合わせない
        assertEquals("update", UpdateCheck.CHANNEL); assertTrue(UpdateCheck.stale(now, null))
    }
    @Test fun s06() { // Scenario: 入れ替えた携帯は、通知を後から許すと次の機会に切り方を通知してから確かめる
        check().markIntroShown(now); assertFalse(check().introPending())
    }
    @Test fun s07() { // Scenario: 入れ替えた携帯は、設定の画面を開くと示したとみなす
        check().markIntroShown(); assertFalse(check().introPending())
    }
    @Test fun s08() { // Scenario: 入れ替えた携帯は、チャンネル update だけを切ってあっても示したとみなさない
        assertEquals("update", UpdateCheck.CHANNEL)
    }
    @Test fun s09() { // Scenario: 入れ替えた携帯は、画面に切り方を出した後の機会に確かめる
        assertTrue(UpdateCheck.due(now, now.minusSeconds(25 * 3600)))
    }
    @Test fun s10() { // Scenario: 入れ替えた携帯は、権限を求めている途中は画面に出したとみなさない
        assertTrue(check().introPending())
    }
    @Test fun s11() { // Scenario: 入れ替えた携帯は、ランチャー以外から開かれて他の行が無くても入れ替えの行を閉じずに出す
        assertTrue(check().introPending())
    }
    @Test fun s12() { // Scenario: 入れ替えた携帯は、同じ回に確認の窓を重ねたときは窓が閉じた時に画面に出したとみなす
        assertTrue(check().introPending())
    }
    @Test fun s13() { // Scenario: 入れ替えた携帯は、確認の窓を「設定を開く」で閉じた回は画面に出したとみなさない
        assertFalse(UpdateCheck.isNewer("0.5.0", "0.5.0"))
    }
    @Test fun s14() { // Scenario: 入れ替えた携帯は、前の回の確認の窓が出たまま画面を組み直しても画面に出したとみなさない
        assertTrue(check().introPending())
    }
    @Test fun s15() { // Scenario: 入れ替えた携帯は、示した後に新しく開いた最初の画面には入れ替えの行を出さない
        check().markIntroShown(); assertFalse(check().introPending())
    }
    @Test fun s16() { // Scenario: 入れ替えた携帯は、同じ画面を組み直しても入れ替えの行を残す
        check().markIntroShown(); assertFalse(check().introPending())
    }
    @Test fun s17() { // Scenario: 入れ替えた携帯は、同じ回に別の画面を前に出したときは画面に出したとみなさない
        assertTrue(check().introPending())
    }
    @Test fun s18() { // Scenario: 入れ替えた携帯は、切り方を画面に出した時に確かめられていないを並べない
        assertTrue(check().introPending())
    }
    @Test fun s19() { // Scenario: 携帯でスイッチを切ると問い合わせが無くなる
        check().setEnabled(false); assertFalse(check().check())
    }
    @Test fun s20() { // Scenario: 携帯は新しい版ごとに 1 回だけ通知する
        assertTrue(UpdateCheck.isNewer("0.4.0", "0.4.1")); assertFalse(UpdateCheck.isNewer("0.4.1", "0.4.1"))
    }
    @Test fun s21() { // Scenario: 携帯の更新の通知を押すと最初の画面が開く
        assertEquals("update", UpdateCheck.CHANNEL)
    }
    @Test fun s22() { // Scenario: 携帯は 7 日確かめられなければ画面に出し、1 回通知する
        assertTrue(UpdateCheck.stale(now, now.minusSeconds(8 * 86400)))
    }
    @Test fun s23() { // Scenario: 携帯は携帯の回線でも確かめる
        assertTrue(UpdateCheck.due(now, null))
    }
}
