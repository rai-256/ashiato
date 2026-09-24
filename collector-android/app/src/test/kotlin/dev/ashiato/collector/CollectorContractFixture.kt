// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.io.File

/** 人が読む契約表を固定試験の期待値にする。表の欄・順序を変えれば payload 試験が落ちる。 */
internal fun payloadContractFields(logicalSource: String): List<String> {
    val cwd = File(System.getProperty("user.dir"))
    val contract = generateSequence(cwd) { it.parentFile }
        .map { File(it, "docs/collector-contract.md") }
        .firstOrNull(File::isFile)
        ?: error("docs/collector-contract.md が見つからない: user.dir=$cwd")
    val heading = "## C-01（`$logicalSource`）が送る `payload` の形"
    val section = contract.readText().substringAfter(heading, missingDelimiterValue = "")
    check(section.isNotEmpty()) { "契約に節が無い: $heading" }
    return section.substringBefore("\n## ")
        .lineSequence()
        .filter { it.startsWith("| `") }
        .map { it.substringAfter("| `").substringBefore('`') }
        .toList()
        .also { check(it.isNotEmpty()) { "契約の payload 表が空: $heading" } }
}
