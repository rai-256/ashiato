// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
package dev.ashiato.collector

import android.content.Context
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import java.io.File

/** Android の自動バックアップが、未送信と設定を別々に扱うことを本物の framework で確かめる。 */
@RunWith(AndroidJUnit4::class)
class St56BackupTest {
    private val context: Context get() = ApplicationProvider.getApplicationContext()

    @Test
    fun prepareBackup() {
        clearAppDataWithoutKillingInstrumentation()
        val outbox = File(context.filesDir, "outbox/st56-backup-test.jsonl")
        outbox.parentFile!!.mkdirs()
        outbox.writeText("unsent-st56-record")
        context.getSharedPreferences(Config.SETUP_PREFS, Context.MODE_PRIVATE).edit()
            .putString("setup", "secret-setting-for-st56-test")
            .commit()
    }

    @Test
    fun verifyRestored() {
        val outbox = File(context.filesDir, "outbox/st56-backup-test.jsonl")
        assertTrue("未送信はバックアップから戻る", outbox.exists())
        assertTrue("未送信の内容は変わらない", outbox.readText() == "unsent-st56-record")
        assertFalse(
            "設定の SharedPreferences はバックアップから戻らない",
            context.getSharedPreferences(Config.SETUP_PREFS, Context.MODE_PRIVATE).contains("setup"),
        )
    }

    private fun clearAppDataWithoutKillingInstrumentation() {
        context.getSharedPreferences(Config.SETUP_PREFS, Context.MODE_PRIVATE).edit().clear().commit()
        context.filesDir.listFiles().orEmpty().forEach(File::deleteRecursively)
        context.cacheDir.listFiles().orEmpty().forEach(File::deleteRecursively)
        context.noBackupFilesDir.listFiles().orEmpty().forEach(File::deleteRecursively)
    }
}
