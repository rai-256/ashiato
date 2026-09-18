// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

/**
 * **権限が「未許可・未要求」の状態から始まる必要があるテスト**の印。
 *
 * 自分の権限を自分で外せない —— `pm revoke` も `am force-stop` も**計測テストのプロセスを殺す**
 * （計測テストはアプリのプロセスで走る。実測 2026-09-18: 9 本中 6 本で `Process crashed`）。
 * なので状態を作るのは**テストの外**の仕事にし、実行を 2 段に分ける:
 *
 * ```bash
 * ./gradlew :app:connectedDebugAndroidTest \
 *   -Pandroid.testInstrumentationRunnerArguments.notAnnotation=dev.ashiato.collector.NeedsPristinePermissions
 * adb shell pm clear dev.ashiato.collector          # 権限を未許可・未要求へ戻す
 * ./gradlew :app:connectedDebugAndroidTest \
 *   -Pandroid.testInstrumentationRunnerArguments.annotation=dev.ashiato.collector.NeedsPristinePermissions
 * ```
 *
 * `tools/android-emulator.sh` と CI がこの順で走らせる。
 * Test Orchestrator（`clearPackageData`）でも同じことができるが、**全テストの走り方が変わる**ので採らない。
 */
@kotlin.annotation.Retention(AnnotationRetention.RUNTIME)
@kotlin.annotation.Target(AnnotationTarget.CLASS, AnnotationTarget.FUNCTION)
annotation class NeedsPristinePermissions
