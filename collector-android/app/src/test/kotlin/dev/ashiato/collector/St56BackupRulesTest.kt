// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
package dev.ashiato.collector

import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.File

class St56BackupRulesTest {
    @Test
    fun backupRulesExcludeCollectorSetupFromCloudAndDeviceTransfer() {
        // Scenario: 両方のバックアップの規則が設定のファイルを外している
        val xmlDir = File("src/main/res/xml")
        val fullBackup = File(xmlDir, "health_full_backup_content.xml").readText()
        val extractionRules = File(xmlDir, "health_data_extraction_rules.xml").readText()

        assertTrue(fullBackup.contains("domain=\"sharedpref\" path=\"collector_setup.xml\""))
        assertTrue(extractionRules.contains("domain=\"sharedpref\" path=\"collector_setup.xml\""))
        assertTrue(extractionRules.contains("<cloud-backup>"))
        assertTrue(extractionRules.contains("<device-transfer>"))
    }
}
