# C-01 収集アプリ（Android / Kotlin）

**まだ骨格だけ。** 実装は ST04 / ST06 / ST09 / ST11 で行う。

## ビルドを 1 回通す（製造準備 B の未達を閉じる）

この環境には JDK / Android SDK が無く、`sudo` も使えないため**未検証**。
Windows 側（または JDK を入れた WSL）で 1 回通す。

```
cd collector-android
./gradlew :app:assembleDebug
```

**`gradle` を別途入れる必要はない。** wrapper（`gradlew` と `gradle/wrapper/`）は
リポジトリに入れてあり、初回実行時に Gradle 8.13 を自分で取ってくる。
**apt の `gradle`（4.4.1）は使わないこと** —— Android Gradle Plugin 8.x は Gradle 8 以上が要る。

## 道具は WSL に入れてある（2026-09-08）

**`sudo` を使わず `~/.local/opt` に入れた。** 消すときは `rm -rf ~/.local/opt/{jdk21,android-sdk}` だけ。

| 道具 | 版 | 置き場 |
|---|---|---|
| Temurin JDK | 21.0.12.1 LTS | `~/.local/opt/jdk21` |
| Gradle | 8.13（wrapper が自動取得） | `~/.gradle/wrapper/dists` |
| Android command-line tools | 13114758 | `~/.local/opt/android-sdk` |

使うときは環境変数を読み込む:

```bash
. ./tools/android-env.sh
```

## 残り 1 手 —— ライセンスへの同意

**これは代わりに押さない。** あなた自身が Google のライセンス条項に同意する行為だから。

```bash
. ./tools/android-env.sh
sdkmanager --licenses                       # 内容を読んで y/n で答える
sdkmanager "platforms;android-36" "build-tools;36.0.0"
cd collector-android && ./gradlew :app:assembleDebug
```

通ったら `docs/production-prep.md` の B 節にコマンドと終了コードを記録し、`block_b: done` にする。

## wrapper の出所（供給網の記録）

- `gradle-wrapper.jar` — https://raw.githubusercontent.com/gradle/gradle/v8.13.0/gradle/wrapper/gradle-wrapper.jar
  - sha256 `81a82aaea5abcc8ff68b3dfcb58b3c3c429378efd98e7433460610fecd7ae45f`（2026-09-08 取得）
- `gradlew` / `gradlew.bat` — 同じタグ `v8.13.0` から取得

通ったら `docs/production-prep.md` の B 節にコマンドと終了コードを記録し、
`block_b: done` にする。

## 権限について

`AndroidManifest.xml` の 5 つの権限は、いずれも**宣言しない期間が取り返せない**か、
収集そのものの前提になっている。とくに:

- `READ_HEALTH_DATA_HISTORY` —— 宣言しない期間の 30 日より前は永久に読めない（要件 EXT-C / 扉 #21）
- `ACCESS_MEDIA_LOCATION` —— 宣言しないと OS が写真の位置を落として返す（要件 EXT-D）
