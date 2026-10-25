// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.io.File
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.Description
import org.junit.runners.model.Statement

/** 試験の一時フォルダは、試験が例外で落ちても消える（CH03 / design D2・D4）。 */
class Ch03TmpRuleTest {
    @Test
    fun ch03_folder_is_removed_even_if_the_test_throws() {
        // Scenario: 携帯の単体試験が例外で落ちても、試験の一時フォルダは消える
        val rule = TemporaryFolder() // tmp-ok: 規則の後始末を手で当てて確かめる試験
        var written: File? = null
        val failing = object : Statement() {
            override fun evaluate() {
                val dir = rule.newFolder()
                File(dir, "left.txt").writeText("残るはずのもの")
                written = dir
                throw IllegalStateException("試験が落ちた")
            }
        }

        val thrown = try {
            rule.apply(failing, Description.EMPTY).evaluate()
            null
        } catch (e: IllegalStateException) {
            e
        }

        assertEquals("試験が落ちた", thrown?.message)
        val dir = written ?: error("statement が走っていない")
        assertFalse("例外の後も一時フォルダが残っている: $dir", dir.exists())
    }
}
