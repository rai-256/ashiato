package dev.ashiato.collector

import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class St57UpdateQueryTest {
    @Test
    fun query_has_fixed_contract() {
        // Scenario: 携帯の問い合わせは版の一覧の取得だけである
        assertTrue(UpdateCheck.USER_AGENT == "ashiato-update-check")
    }

    @Test
    fun accepts_gzip_response() {
        // Scenario: 携帯は圧縮した応答を受ける
        assertTrue(UpdateCheck.ACCEPT == "application/vnd.github+json")
    }
}
