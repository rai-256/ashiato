// SPDX-License-Identifier: AGPL-3.0-only
plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("org.jetbrains.kotlin.plugin.serialization")
}

android {
    namespace = "dev.ashiato.collector"
    compileSdk = 36
    defaultConfig {
        applicationId = "dev.ashiato.collector"
        minSdk = 30          // Health Connect と scoped storage の前提
        targetSdk = 36
        versionCode = 1
        versionName = "0.1.0"

        // 接続先と資格情報は**コミットしない**（製造準備 A-2）。
        // ~/.gradle/gradle.properties か -P で渡す。既定は空で、空のまま動かすと送信は行われない。
        buildConfigField("String", "BASE_URL",
            "\"${project.findProperty("ashiato.baseUrl") ?: ""}\"")
        buildConfigField("String", "API_TOKEN",
            "\"${project.findProperty("ashiato.apiToken") ?: ""}\"")
        buildConfigField("String", "USER_ID",
            "\"${project.findProperty("ashiato.userId") ?: ""}\"")
        // 計測テスト（src/androidTest）。エミュレータでも実機でも同じものが走る（2026-09-14）
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }
    buildFeatures { buildConfig = true }
    // Robolectric は本物の framework を JVM 上で動かすので資源が要る。
    // **本番経路（LocationCallback / Activity の権限フロー）を実機なしで通すための唯一の道具**
    testOptions { unitTests { isIncludeAndroidResources = true } }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}

/**
 * 平文 HTTP を**設定した接続先 1 ホストだけ**に許す設定を生成する。
 *
 * **これが無いと 1 件も届かない。** targetSdk 28 以降、Android は平文 HTTP を既定で遮断し、
 * `UnknownServiceException`（`IOException` の子）を投げる。`HttpTransport` はそれを
 * `Unreachable` に畳むので、**アプリは動き続け、未送信は積まれ続け、logcat に 1 行出るだけ**になる。
 * 実機を持って歩いてから気付く型の失敗なので、ここで塞ぐ。
 *
 * `usesCleartextTraffic="true"` にはしない —— それだと**どこへでも**平文で出られる。
 * 接続先は Tailscale 網内の 1 台（PERM-7）なので、そのホストだけを開ける。
 * 接続先が設定されていない（CI のビルド）ときは**全部拒否**したまま。
 */
abstract class GenerateNetworkSecurityConfig : DefaultTask() {
    @get:Input
    abstract val host: Property<String>

    @get:OutputDirectory
    abstract val outputDir: DirectoryProperty

    @TaskAction
    fun generate() {
        val h = host.get()
        val allow = if (h.isBlank()) {
            ""
        } else {
            "\n    <domain-config cleartextTrafficPermitted=\"true\">" +
                "\n        <domain includeSubdomains=\"false\">$h</domain>" +
                "\n    </domain-config>"
        }
        val dir = outputDir.get().asFile.resolve("xml")
        dir.mkdirs()
        dir.resolve("network_security_config.xml").writeText(
            "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n" +
                "<!-- 生成物。app/build.gradle.kts の GenerateNetworkSecurityConfig が作る -->\n" +
                "<network-security-config>\n" +
                "    <base-config cleartextTrafficPermitted=\"false\" />$allow\n" +
                "</network-security-config>\n",
        )
    }
}

// `java.net.URI` は Gradle の `java` 拡張と名前がぶつかるので、素直に切り出す
val configuredHost: String =
    Regex("^[a-zA-Z][a-zA-Z0-9+.-]*://([^/:?#]+)")
        .find(project.findProperty("ashiato.baseUrl")?.toString().orEmpty())
        ?.groupValues?.get(1)
        .orEmpty()

val generateNetworkSecurityConfig =
    tasks.register<GenerateNetworkSecurityConfig>("generateNetworkSecurityConfig") {
        host.set(configuredHost)
        outputDir.set(layout.buildDirectory.dir("generated/res/nsconfig"))
    }

androidComponents {
    onVariants { variant ->
        variant.sources.res?.addGeneratedSourceDirectory(
            generateNetworkSecurityConfig,
            GenerateNetworkSecurityConfig::outputDir,
        )
    }
}

// Kotlin 2.x の DSL。kotlinOptions { jvmTarget } は非推奨。
kotlin {
    compilerOptions {
        jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17)
    }
}

dependencies {
    implementation("androidx.core:core-ktx:1.17.0")
    implementation("com.google.android.gms:play-services-location:21.3.0")
    implementation("org.jetbrains.kotlinx:kotlinx-serialization-json:1.9.0")
    testImplementation("junit:junit:4.13.2")
    testImplementation("org.robolectric:robolectric:4.16")
    testImplementation("androidx.test:core:1.7.0")
    // 計測テスト（端末の上で走る。`tools/android-emulator.sh` / CI の android-instrumented）。
    // 「実機が要る」と README に書いた 3 クラス（前景サービス・権限の入口・HTTP）を本物の framework で通す
    androidTestImplementation("androidx.test:core:1.7.0")
    androidTestImplementation("androidx.test:runner:1.7.0")
    androidTestImplementation("androidx.test:rules:1.7.0")
    androidTestImplementation("androidx.test.ext:junit:1.3.0")
    // 権限ダイアログは**システム UI**（permissioncontroller）。自分のプロセスの外なので
    // Espresso では触れない。拒否したときの振る舞いを機械で確かめるために UI Automator を入れる（2026-09-18）
    androidTestImplementation("androidx.test.uiautomator:uiautomator:2.3.0")
}
