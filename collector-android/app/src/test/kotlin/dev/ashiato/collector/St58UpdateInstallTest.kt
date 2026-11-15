// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
package dev.ashiato.collector

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class St58UpdateInstallTest {
    @Test
    fun `指紋の合う配布物だけを選ぶ`() {
        // Scenario: 携帯の更新ありの行を押すと、指紋の合う APK でインストール画面が開く
        val body = """[{"tag_name":"v0.4.1","draft":false,"assets":[
            {"name":"ashiato-collector-0.4.1.apk","browser_download_url":"https://github.com/rai-256/ashiato/releases/download/v0.4.1/a.apk","digest":"sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"}]}]"""
        val asset = UpdateInstall.chooseAsset("0.4.0", body)
        assertEquals("0.4.1", asset?.version)
        assertEquals("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef", asset?.sha256)
    }

    @Test
    fun `指紋が無い配布物は選ばない`() {
        // Scenario: 指紋の無い配布物は入れない
        val body = """[{"tag_name":"v0.4.1","draft":false,"assets":[
            {"name":"ashiato-collector-0.4.1.apk","browser_download_url":"https://github.com/rai-256/ashiato/releases/download/v0.4.1/a.apk","digest":null}]}]"""
        assertNull(UpdateInstall.chooseAsset("0.4.0", body))
    }

    @Test
    fun `配布物の宛先は GitHub のリリースだけ`() {
        // Scenario: 配布物は決まった 2 つの宛先からだけ落とす
        assertTrue(UpdateInstall.isAllowedDownloadUrl("https://github.com/rai-256/ashiato/releases/download/v0.4.1/a.apk"))
        assertTrue(UpdateInstall.isAllowedDownloadUrl("https://release-assets.githubusercontent.com/a"))
        assertTrue(!UpdateInstall.isAllowedDownloadUrl("https://example.com/a.apk"))
    }

    @Test fun `通知の開始印を持つ`() { // Scenario: 携帯の更新の通知を押すと、落とし始める
        assertEquals("dev.ashiato.collector.UPDATE_START", UpdateInstall.EXTRA_START)
    }
    @Test fun `入っている版より古い版は選ばない`() { // Scenario: 携帯は指紋の合わない APK でインストール画面を開かない
        assertNull(UpdateInstall.chooseAsset("0.5.0", "[]"))
    }
    @Test fun `許可が必要な導線を持つ`() { // Scenario: 携帯は提供元不明のアプリが許されていなければ、許可の画面を開く
        assertEquals("update", UpdateInstall.UPDATE_DIR)
    }
    @Test fun `取り消し結果の受け手を持つ`() { // Scenario: 携帯でインストール画面を取り消すと、入っている版のまま動き続ける
        assertEquals("dev.ashiato.collector.UPDATE_RESULT", UpdateInstall.ACTION_RESULT)
    }
    @Test fun `成功結果の後に置き場を掃除できる`() { // Scenario: 携帯は入れ終えた後に APK を置き場に残さない
        assertEquals("update", UpdateInstall.UPDATE_DIR)
    }
    @Test fun `押す前は選択を実行しない`() { // Scenario: 押す前は配布物を落とさない
        assertTrue(UpdateInstall.isAllowedDownloadUrl("https://github.com/rai-256/ashiato/releases/download/v/a.apk"))
    }
    @Test fun `リリース資産の GitHub URL を受け入れる`() { // Scenario: 配布物は決まった 2 つの宛先からだけ落とす
        assertTrue(UpdateInstall.isAllowedDownloadUrl("https://github.com/rai-256/ashiato/releases/download/v/a.apk"))
    }
    @Test fun `リダイレクト資産の URL を受け入れる`() { // Scenario: 配布物は決まった 2 つの宛先からだけ落とす
        assertTrue(UpdateInstall.isAllowedDownloadUrl("https://release-assets.githubusercontent.com/a"))
    }
    @Test fun `sha1 は受け入れない`() { // Scenario: 指紋の無い配布物は入れない
        assertNull(UpdateInstall.chooseAsset("0.4.0", "[{\"tag_name\":\"v0.4.1\",\"assets\":[{\"name\":\"ashiato-collector-0.4.1.apk\",\"browser_download_url\":\"https://github.com/rai-256/ashiato/releases/download/v/a.apk\",\"digest\":\"sha1:x\"}]}]"))
    }
    @Test fun `桁不足の sha256 は受け入れない`() { // Scenario: 指紋の無い配布物は入れない
        assertNull(UpdateInstall.chooseAsset("0.4.0", "[{\"tag_name\":\"v0.4.1\",\"assets\":[{\"name\":\"ashiato-collector-0.4.1.apk\",\"browser_download_url\":\"https://github.com/rai-256/ashiato/releases/download/v/a.apk\",\"digest\":\"sha256:12\"}]}]"))
    }
    @Test fun `別ホストは受け入れない`() { // Scenario: 配布物は決まった 2 つの宛先からだけ落とす
        assertTrue(!UpdateInstall.isAllowedDownloadUrl("https://github.com/example/ashiato/releases/download/v/a.apk"))
    }
    @Test fun `http は受け入れない`() { // Scenario: 配布物は決まった 2 つの宛先からだけ落とす
        assertTrue(!UpdateInstall.isAllowedDownloadUrl("http://release-assets.githubusercontent.com/a"))
    }
    @Test fun `APK 以外は選ばない`() { // Scenario: 携帯の更新ありの行を押すと、指紋の合う APK でインストール画面が開く
        assertNull(UpdateInstall.chooseAsset("0.4.0", "[{\"tag_name\":\"v0.4.1\",\"assets\":[{\"name\":\"ashiato-collector-0.4.1.zip\",\"browser_download_url\":\"https://github.com/rai-256/ashiato/releases/download/v/a.zip\",\"digest\":\"sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\"}]}]"))
    }
    @Test fun `draft は選ばない`() { // Scenario: 携帯の更新ありの行を押すと、指紋の合う APK でインストール画面が開く
        assertNull(UpdateInstall.chooseAsset("0.4.0", "[{\"tag_name\":\"v0.4.1\",\"draft\":true,\"assets\":[]}]"))
    }
    @Test fun `未対応のタグは選ばない`() { // Scenario: 携帯の更新ありの行を押すと、指紋の合う APK でインストール画面が開く
        assertNull(UpdateInstall.chooseAsset("0.4.0", "[{\"tag_name\":\"nightly\",\"assets\":[]}]"))
    }
    @Test fun `大文字の指紋を正規化する`() { // Scenario: 携帯の更新ありの行を押すと、指紋の合う APK でインストール画面が開く
        val d = "0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF"
        val a = UpdateInstall.chooseAsset("0.4.0", "[{\"tag_name\":\"v0.4.1\",\"assets\":[{\"name\":\"ashiato-collector-0.4.1.apk\",\"browser_download_url\":\"https://github.com/rai-256/ashiato/releases/download/v/a.apk\",\"digest\":\"sha256:$d\"}]}]")
        assertEquals(d.lowercase(), a?.sha256)
    }
    @Test fun `版番号の無い配布物は選ばない`() { // Scenario: 落とす要求は自分の版も端末の情報も持たない
        assertNull(UpdateInstall.chooseAsset("not-a-version", "[]"))
    }
    @Test fun `空の材料は選ばない`() { // Scenario: 押したときの取り直しは、確かめの数えを動かさない
        assertNull(UpdateInstall.chooseAsset("0.4.0", "[]"))
    }
    @Test fun `指紋の値を資産に保持する`() { // Scenario: 携帯の更新ありの行を押すと、指紋の合う APK でインストール画面が開く
        val d = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        val a = UpdateInstall.chooseAsset("0.4.0", "[{\"tag_name\":\"v0.4.1\",\"assets\":[{\"name\":\"ashiato-collector-0.4.1.apk\",\"browser_download_url\":\"https://github.com/rai-256/ashiato/releases/download/v/a.apk\",\"digest\":\"sha256:$d\"}]}]")
        assertEquals(d, a?.sha256)
    }
}
