// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

/** `GET /outages` の応答の例（サーバの `OutagesResponse` と同じ形）。 */
object OutageFixtures {
    const val READY_LOCATION = "{" +
        """"status":"ready","judged_at":"2026-10-12T00:00:00Z","outages":[""" +
        """{"episode_id":1,"logical_source":"c01-location","display_name":"位置",""" +
        """"last_alive_at":"2026-10-11T23:15:00Z"}]}"""

    const val READY_EMPTY = """{"status":"ready","judged_at":"2026-10-12T01:00:00Z","outages":[]}"""
}
