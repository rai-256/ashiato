// SPDX-License-Identifier: AGPL-3.0-only
package dev.ashiato.collector

import java.io.File
import java.nio.file.Files

/**
 * 置き場を関心にしない試験のための Outbox。**毎回まっさらなファイルを使う。**
 *
 * `Outbox` に「メモリだけの既定」を持たせない代わりにここへ置く ——
 * 既定があると、本番でうっかりそれが選ばれて未送信が無言で消える
 * （深掘り 第 2 回で実際に起きていた欠陥）。
 */
fun testOutbox(): Outbox =
    Outbox(FileOutboxStore(File(Files.createTempDirectory("outbox").toFile(), "outbox.jsonl")) {})
