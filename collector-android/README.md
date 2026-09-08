# C-01 収集アプリ（Android / Kotlin）

**まだ骨格だけ。** 実装は ST04 / ST06 / ST09 / ST11 で行う。

## ビルドを 1 回通す（製造準備 B の未達を閉じる）

この環境には JDK / Android SDK が無く、`sudo` も使えないため**未検証**。
Windows 側（または JDK を入れた WSL）で 1 回通す。

```
cd collector-android
gradle wrapper            # 初回だけ。gradlew と wrapper jar を生成する
./gradlew :app:assembleDebug
```

通ったら `docs/production-prep.md` の B 節にコマンドと終了コードを記録し、
`block_b: done` にする。

## 権限について

`AndroidManifest.xml` の 5 つの権限は、いずれも**宣言しない期間が取り返せない**か、
収集そのものの前提になっている。とくに:

- `READ_HEALTH_DATA_HISTORY` —— 宣言しない期間の 30 日より前は永久に読めない（要件 EXT-C / 扉 #21）
- `ACCESS_MEDIA_LOCATION` —— 宣言しないと OS が写真の位置を落として返す（要件 EXT-D）
