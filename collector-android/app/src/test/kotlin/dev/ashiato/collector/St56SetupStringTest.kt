// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
package dev.ashiato.collector

import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import org.junit.Assert.assertEquals
import org.junit.Test
import java.io.File

class St56SetupStringTest {
    private val vectorFile = generateSequence(File(System.getProperty("user.dir"))) { it.parentFile }
        .map { File(it, "docs/contracts/collector-setup-vectors.json") }
        .firstOrNull { it.isFile }
        ?: error("docs/contracts/collector-setup-vectors.json が見つからない")
    private val vectors = Json.parseToJsonElement(
        vectorFile.readText(),
    ).jsonObject

    private fun expected(vector: JsonObject): SetupString? = vector["output"]?.let {
        if (it is kotlinx.serialization.json.JsonNull) {
            null
        } else {
            SetupString(
                it.jsonObject["url"]!!.jsonPrimitive.content,
                it.jsonObject["token"]!!.jsonPrimitive.content,
            )
        }
    }

    @Test
    fun st56_setup_string_valid_vectors() {
        for (vector in vectors["valid"]!!.jsonArray) {
            val item = vector.jsonObject
            val result = expected(item)!!
            assertEquals(result, parseSetupString(item["input"]!!.jsonPrimitive.content))
        }
        val basic = expected(vectors["valid"]!!.jsonArray.first().jsonObject)!!
        assertEquals(
            vectors["valid"]!!.jsonArray.first().jsonObject["input"]!!.jsonPrimitive.content,
            buildSetupString(basic.url, basic.token)
        )
    }

    @Test
    fun st56_setup_string_invalid_vectors() {
        for (vector in vectors["invalid"]!!.jsonArray) {
            assertEquals(null, parseSetupString(vector.jsonObject["input"]!!.jsonPrimitive.content))
        }
    }
}
