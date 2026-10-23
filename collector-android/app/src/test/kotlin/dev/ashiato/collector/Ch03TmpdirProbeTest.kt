package dev.ashiato.collector

import java.io.File
import org.junit.Assert.assertEquals
import org.junit.Assume.assumeTrue
import org.junit.Test

/**
 * CH03 の前提の確かめ: `ASHIATO_TEST_TMPDIR` で試験の JVM の `java.io.tmpdir` を向けられる
 * （`app/build.gradle.kts` の `unitTests.all`）。環境変数が無ければ飛ばす。
 */
class Ch03TmpdirProbeTest {
    @Test
    fun ch03_tmpdir_follows_ASHIATO_TEST_TMPDIR() {
        val wanted = System.getenv("ASHIATO_TEST_TMPDIR")
        assumeTrue(wanted != null)
        // tmp-ok: 向け先を読むだけ
        val actual = System.getProperty("java.io.tmpdir")
        assertEquals(File(wanted!!).canonicalFile, File(actual).canonicalFile)
    }
}
