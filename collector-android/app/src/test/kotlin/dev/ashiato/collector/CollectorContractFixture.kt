// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.io.File
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.longOrNull
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue

internal data class PayloadContractField(
    val name: String,
    val type: String,
    val omission: String,
)

/** 人が読む契約表を固定試験の期待値にする。表の全列と payload の実際の形を照合する。 */
internal fun assertPayloadMatchesContract(
    logicalSource: String,
    complete: JsonObject,
    requiredOnly: JsonObject,
) {
    val fields = payloadContractFields(logicalSource)
    val required = fields.filter { it.omission == "省略しない" }.map { it.name }
    val optional = fields.filter { it.omission != "省略しない" }.map { it.name }

    assertEquals("契約表の欄または並びが payload と違う", fields.map { it.name }, complete.keys.toList())
    assertEquals("必須欄の指定が payload の省略規則と違う", required, requiredOnly.keys.toList())
    assertEquals(
        "任意欄の指定が payload の省略規則と違う",
        optional,
        complete.keys.filterNot(requiredOnly::containsKey),
    )
    fields.forEach { field -> assertJsonType(field, complete.getValue(field.name)) }
}

private fun payloadContractFields(logicalSource: String): List<PayloadContractField> {
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
        .map { line ->
            val cells = line.removePrefix("|").removeSuffix("|").split('|').map(String::trim)
            check(cells.size == 3) { "契約表が3列でない: $line" }
            PayloadContractField(cells[0].removeSurrounding("`"), cells[1], cells[2])
        }
        .toList()
        .also { check(it.isNotEmpty()) { "契約の payload 表が空: $heading" } }
}

private fun assertJsonType(field: PayloadContractField, value: kotlinx.serialization.json.JsonElement) {
    val primitive = value as? JsonPrimitive
    assertNotNull("${field.name} が JSON primitive でない", primitive)
    when {
        field.type.startsWith("integer") -> {
            assertFalse("${field.name} は integer だが JSON string", primitive!!.isString)
            assertNotNull("${field.name} は integer だが整数でない", primitive.longOrNull)
        }
        field.type == "text" || field.type.startsWith("RFC3339") || field.type.startsWith("`") ->
            assertTrue("${field.name} は文字列型だが JSON string でない", primitive!!.isString)
        else -> error("契約表に未対応の型がある: ${field.name}=${field.type}")
    }
}
