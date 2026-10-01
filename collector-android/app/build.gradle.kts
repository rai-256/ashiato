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
    testOptions {
        unitTests { isIncludeAndroidResources = true }
        // **Robolectric を offline にできる口。** 既定では `$HOME` 直下に
        // `.robolectric-download-lock` を作ろうとする（`MavenDependencyResolver`）。
        // Codex の sandbox（`-s workspace-write`）は cwd / /tmp しか書けないので、
        // 2026-09-21 の実測では単体テスト 160 本のうち 4 本がここで落ちた。
        // `$HOME` を丸ごと書けるようにすると `~/.ssh` と `~/.config/gh` の token まで
        // 開くことになるので採らない。offline なら resolver を通らない。
        //
        // 環境変数が無いときは**何も変えない** —— CI は立てないので今までどおり動く。
        // 立てるのは harness（`scripts/codex_env.sh`）で、値は android-all の jar を
        // 集めた場所。読むだけなので、その場所は書けなくてよい。
        unitTests.all { test ->
            System.getenv("ASHIATO_ROBOLECTRIC_JARS")?.let { jars ->
                test.systemProperty("robolectric.offline", "true")
                test.systemProperty("robolectric.dependency.dir", jars)
            }
            // **人が読む契約表も入力**（tasks 7.3 / code-verify R33）。
            // `CollectorContractFixture` は期待値をこの md から読むのに、Gradle は
            // ソースと classpath しか入力に数えない —— **表だけを 1 行変えると
            // `UP-TO-DATE` で rc=0 のまま通り**、手元では 7.3 のガードが効かなかった
            // （`--rerun` を付けたときだけ落ちる。実測 2026-09-26）。
            test.inputs.file(rootProject.file("../docs/collector-contract.md"))
                .withPropertyName("collectorContract")
                .withPathSensitivity(PathSensitivity.RELATIVE)
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}

/**
 * 平文 HTTP を **loopback（`localhost` / `127.0.0.1`）だけ**に許す設定を生成する（ST28 design D13）。
 *
 * **例外を `ashiato.baseUrl` の host から作らない。** 接続先は `https://`（`tailscale serve`）で、
 * 平文が要るのは端末の中のテスト用サーバ（計測テスト）だけ。接続先の host に平文を許す形だと、
 * 設定を間違えただけで網の外へ暗号化なしで出られる。
 * `usesCleartextTraffic="true"` にもしない —— それだと**どこへでも**平文で出られる。
 *
 * `ashiato.baseUrl` が `http://` で host が loopback でなければ**組み立てを落とす**。
 */
abstract class GenerateNetworkSecurityConfig : DefaultTask() {
    @get:Input
    abstract val baseUrl: Property<String>

    @get:OutputDirectory
    abstract val outputDir: DirectoryProperty

    @TaskAction
    fun generate() {
        val m = Regex("^([a-zA-Z][a-zA-Z0-9+.-]*)://(\\[[^\\]]*\\]|[^/:?#]+)").find(baseUrl.get())
        if (m != null && m.groupValues[1].lowercase() == "http" &&
            // 生成する domain-config と同じ 2 つだけ（`[::1]` は許可に無いので、通すと組み立てた後に送れない。final review R10）
            m.groupValues[2].lowercase() !in setOf("localhost", "127.0.0.1")
        ) {
            throw GradleException(
                "ashiato.baseUrl が http:// で、接続先が暗号化されていない（loopback 以外へ平文で送らない）。https:// にする",
            )
        }
        val dir = outputDir.get().asFile.resolve("xml")
        dir.mkdirs()
        dir.resolve("network_security_config.xml").writeText(
            "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n" +
                "<!-- 生成物。app/build.gradle.kts の GenerateNetworkSecurityConfig が作る -->\n" +
                "<network-security-config>\n" +
                "    <base-config cleartextTrafficPermitted=\"false\" />\n" +
                "    <domain-config cleartextTrafficPermitted=\"true\">\n" +
                "        <domain includeSubdomains=\"false\">localhost</domain>\n" +
                "        <domain includeSubdomains=\"false\">127.0.0.1</domain>\n" +
                "    </domain-config>\n" +
                "</network-security-config>\n",
        )
    }
}

val generateNetworkSecurityConfig =
    tasks.register<GenerateNetworkSecurityConfig>("generateNetworkSecurityConfig") {
        baseUrl.set(project.findProperty("ashiato.baseUrl")?.toString().orEmpty())
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
