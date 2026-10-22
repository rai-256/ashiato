# 写真・動画の試験材料（ST09）

**全部自作**。外の写真・動画・EXIF の見本は 1 つも入れていない（出所の許諾の問題が無い。`tools/check-licenses.sh` は依存の許諾を見る道具で、
この自作物は対象外）。作り方は `tools/media-fixtures/gen.py`（標準ライブラリだけ。EXIF は手で組む）と `MkBase.java`（`ImageIO` で絵を作る）。

| ファイル | 中身 | 使う試験 |
|---|---|---|
| `camera-gps.jpg` | 64×48 の JPEG。`Make=AshiatoTest` `Model=Fixture One`、`DateTimeOriginal=2026:10:17 14:30:12`、`OffsetTimeOriginal=+09:00`、露出（`ExposureTime` `FNumber` `ISOSpeedRatings`）、GPS 35.6812 N / 139.7671 E、300 バイトの `MakerNote`、埋め込みの縮小画像（IFD1） | `MediaRecordTest` `MediaRecordSizeTest`、計測テストに入れる写真（`MediaPremisesInstrumentedTest`） |
| `camera-no-offset.jpg` | 同じ絵。`OffsetTimeOriginal` と GPS と縮小画像が無い | `MediaRecordTest` |
| `video-gps.mp4` | 長さ 12.5 秒・`©xyz` で +35.6812+139.7671・作成時刻 2026-10-17T05:30:12Z の最小の MP4（映像の標本は 1 つで、復号はできない。メタだけを読む材料） | `MediaRecordTest`（ハッシュ）、`MediaPremisesInstrumentedTest`（入れ物のメタ） |

作り直すと内容ハッシュが変わる。`MediaRecordTest` の期待（`camera-gps.jpg` の SHA-256）は `sha256sum` の値で固定してある —— 作り直したら合わせる。
