// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import android.security.NetworkSecurityPolicy
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

/** 収集アプリの通信の設定は loopback にだけ平文を許す（ST28 design D13）。端末の上でしか分からない。 */
@RunWith(AndroidJUnit4::class)
class CleartextPolicyInstrumentedTest {
    // Scenario: 収集アプリは loopback への平文の接続を許す
    @Test
    fun cleartext_to_loopback_is_permitted() {
        assertTrue(NetworkSecurityPolicy.getInstance().isCleartextTrafficPermitted("127.0.0.1"))
    }
}
