// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.time.Instant

/**
 * 試験用の `LocationFix`。時計の 3 項目と起動の識別を省けるのは**試験の組み立てだけ**
 * （本番の `LocationFix` は既定値を持たない。review R17）。
 */
fun testFix(
    latitude: Double,
    longitude: Double,
    accuracyMeters: Float,
    at: Instant,
    receivedDeviceTime: Instant = at,
    fixElapsedNs: Long = 0L,
    receivedElapsedMs: Long = 0L,
    bootCount: Int? = null,
): LocationFix = LocationFix(latitude, longitude, accuracyMeters, at, receivedDeviceTime, fixElapsedNs, receivedElapsedMs, bootCount)
