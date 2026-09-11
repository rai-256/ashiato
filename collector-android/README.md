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
./gradlew :app:testDebugUnitTest      # 71 件。CI でも走る
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
VALUES ('c01-location','携帯端末の位置',21600);
```

> **`expected_gap_sec` は 21600（6 時間）。** FR-35 が「想定間隔の初期値は 位置 = 6 時間」と
> 定めており、その **3 倍**を超えると「ソースが止まっている」と通知される。
> **送信間隔（5 分）を入れてはいけない** —— Doze の空きは実測で最長 14.2 分あり、
> 300 秒だと閾値 15 分の 95 % まで届いて**誤報が出る**（2026-09-10 実測）。
> ここが表すのは「送信の周期」ではなく「**これを超えたら異常とみなす無通信の長さ**」。

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

### 求め方（design D27）

**`onRequestPermissionsResult` の結果コードは見ない。** Android 11 以降、
`ACCESS_BACKGROUND_LOCATION` は許可ダイアログを出せず設定画面へ送られるので、
**結果は必ず拒否で返る**（そのあと設定画面で許可しても拒否のまま）。
結果を信じて終わると、前景を許可した直後に必ず終了してサービスが一度も起動しない
（**実機で実際に起きた**）。

見るのは**そのつどの実際の権限状態**だけ。`onResume` からも見直すので、
設定画面から戻ってきた許可も拾える。

| 権限 | 無いとどうなるか |
|---|---|
| `ACCESS_FINE_LOCATION` | **必須。** 取るものが無いので収集を始めない |
| `ACCESS_BACKGROUND_LOCATION` | 始めるが degraded —— `START_STICKY` の立て直しで位置を取れず、次にアプリを開くまで収集が止まる。`kind=degraded error=no_background_location` を残す |
| `POST_NOTIFICATIONS` | 始める。常時通知が出ないだけ |

### 初回に端末側でやること

| | なぜ |
|---|---|
| 位置情報を**「常に許可」** | 背景の位置が無いと、立て直し後に収集が止まる（上の表） |
| **「使用していないアプリを管理する」をオフ**（設定 → アプリ → あしあと。→ 一番下） | 既定でオンで、**権限を削除してアプリをアーカイブする**。この収集アプリは本人がめったに開かないので、数か月後に無言で収集が止まる |

> **「電池の最適化」の一覧は探さなくてよい。** Android 15 では `特別なアプリアクセス` から
> 外れており、アプリ側から要求したときだけダイアログが出る（実機で確認）。
> per-app の「バックグラウンドでの使用を許可」は既定で ON で、これは Doze の除外とは別物。

初回起動では **位置=「常に許可」** まで進めてほしい。設定画面へ送られたら、
許可して戻ってくればそのまま収集が始まる。

## 生存信号（ST02 / FR-78）

記録が 1 件も生成されない期間でも、**6 時間ごとに「生きている」信号**を `POST /heartbeat` へ送る。
記録が 0 件の日に「動きが無かったのか / 収集が壊れていたのか」を分ける材料は、
**その時点で送らないと後から作れない**（扉 #14）。

信号が運ぶもの:

| | 何のために |
|---|---|
| `capturable` と `blockers` | **プロセスは生きたまま取れていない**状態を残す（深掘り Q5）。Android は長期間使っていないアプリの権限を自動で剥がす |
| `attempts` / `successes` | 想定間隔より細かい空きを**取得率**として残す（第 5 回 Q17） |

未送信は記録と**同じ仕組み**（`FileOutboxStore` の追記 JSONL）に乗せるが、
**ファイルは分ける**（`heartbeat.jsonl`）—— 同じ JSONL に混ぜると、読み戻しで片方が
「壊れた行」に見えて退避に回る。

### Doze との関係（ST01 の R46 に従う）

**Doze の除外を要求しない**（ST01 の R46 = B。ST02 では決め直さない）。
実測で Doze の維持間隔は**最長 14.2 分**であり、生存信号の想定間隔（6 時間）より
2 桁小さいので、**生存信号は Doze の維持時間帯に乗る** —— 眠ったまま丸 1 日
信号が出ない、という経路は無い。

一方で R46 が残した宿題（1 日 24 回・最長 14.2 分の取得の空きを、
「眠っていた」のか「死んでいた」のかで分けられない）は、**信号の間隔では埋まらない**
——6 時間の刻みは 14 分の空きの中に入らない。埋めるのは `attempts` / `successes` の比で、
区間の満点（6 時間 ÷ 60 秒 = 360 回）に対する取得率がそのまま残る。

- 信号が来ている＋取得率が高い → 生きていた
- 信号が来ている＋取得率が低い → **眠っていた**
- 信号が来ない → **死んでいた**（受け手が「途絶」として検知する。FR-80）
