# C-01 収集アプリ（Android / Kotlin）

**ST01 で位置の収集が入った。** 残りのソース（アプリ利用・写真・健康）は
ST04 / ST06 / ST09 / ST11 で足す。

## 何をするか

60 秒ごとに位置を取り（FR-1）、5 分ごとにその時点の未送信をまとめて送る（design D9）。

| ファイル | 役割 | 実機なしで試験できるか |
|---|---|---|
| `DeviceId.kt` | 端末識別子を初回だけ採番して置く（design D6） | できる |
| `LocationFix.kt` | 1 回の取得を契約どおりの 1 件にする | できる |
| `Outbox.kt` | 未送信の置き場（上限と破棄は ST04） | できる |
| `Sender.kt` | まとめ送りと**部分失敗の扱い**（design D9） | できる |
| `Telemetry.kt` | ログに出してよいものだけを組み立てる（A-2） | できる |
| `LocationService.kt` | 前景サービス + 位置取得（design D7） | **実機が要る** |
| `MainActivity.kt` | 権限の要求（前景 → 背景 → 通知の 3 段） | **実機が要る** |
| `HttpTransport.kt` | 取り込み口への POST | **実機が要る** |

**判断のあるところは Android の API から切り離してある。** 実機でしか動かない部分に
条件分岐を置くと、CI では一度も通らない経路ができる。

```bash
./gradlew :app:testDebugUnitTest      # 23 件。CI でも走る
```

## 接続先と資格情報

**コミットしない**（製造準備 A-2）。`~/.gradle/gradle.properties` に置くか `-P` で渡す:

```properties
ashiato.baseUrl=http://<自宅 PC>:18787
ashiato.apiToken=<.env の API_TOKEN と同じ値>
ashiato.userId=<.env の ASHIATO_USER_ID と同じ値>
```

揃っていなければ**送信を始めないが、取得は続ける** —— 記録は未送信に積まれ、
設定してから送られる（捨てない）。

送る前に、登録簿へ 1 行入れておく（FR-61。API のコードは変えない）:

```sql
INSERT INTO core.source (logical_source, display_name, expected_gap_sec)
VALUES ('c01-location','携帯端末の位置',300);
```

## ビルド

```bash
. ./tools/android-env.sh          # JDK / Android SDK の場所（~/.local/opt に入れてある）
cd collector-android
./gradlew :app:assembleDebug
./gradlew :app:testDebugUnitTest
```

**`gradle` を別途入れる必要はない。** wrapper（`gradlew` と `gradle/wrapper/`）は
リポジトリに入れてあり、初回実行時に Gradle 8.13 を自分で取ってくる。
**apt の `gradle`（4.4.1）は使わないこと** —— Android Gradle Plugin 8.x は Gradle 8 以上が要る。

道具は **`sudo` を使わず `~/.local/opt` に入れてある**。消すときは
`rm -rf ~/.local/opt/{jdk21,android-sdk}` だけ。

| 道具 | 版 | 置き場 |
|---|---|---|
| Temurin JDK | 21.0.12.1 LTS | `~/.local/opt/jdk21` |
| Gradle | 8.13（wrapper が自動取得） | `~/.gradle/wrapper/dists` |
| Android command-line tools | 13114758 | `~/.local/opt/android-sdk` |
| platforms / build-tools | android-36 / 36.0.0 | 同上 |

Google のライセンス条項への同意（`sdkmanager --licenses`）は**本人が済ませてある**
（`~/.local/opt/android-sdk/licenses/`）。**これは代わりに押さない。**

## wrapper の出所（供給網の記録）

- `gradle-wrapper.jar` — https://raw.githubusercontent.com/gradle/gradle/v8.13.0/gradle/wrapper/gradle-wrapper.jar
  - sha256 `81a82aaea5abcc8ff68b3dfcb58b3c3c429378efd98e7433460610fecd7ae45f`（2026-09-08 取得）
- `gradlew` / `gradlew.bat` — 同じタグ `v8.13.0` から取得

## 権限について

`AndroidManifest.xml` の 5 つの権限は、いずれも**宣言しない期間が取り返せない**か、
収集そのものの前提になっている。とくに:

- `READ_HEALTH_DATA_HISTORY` —— 宣言しない期間の 30 日より前は永久に読めない（要件 EXT-C / 扉 #21）
- `ACCESS_MEDIA_LOCATION` —— 宣言しないと OS が写真の位置を落として返す（要件 EXT-D）
