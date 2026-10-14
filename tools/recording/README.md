# 録画の差し込み口（ashiato2 側）

録画の仕組みはハーネス（harness2 の `scripts/record-run`。README の「録画」）が持つ。ここにあるのは
**ashiato2 だけのこと** —— 何を撮るか（シナリオ）と、どう起動するか（専用環境）。

| ファイル | 役目 |
|---|---|
| `recording.json` | 置き場（`C:\dev\ashiato2-recordings`）・Playwright の設定・環境の台本・古いシナリオの一覧 |
| `scenarios.d/*.json` | **Story ごとのシナリオ**（1 Story 1 ファイル。説明は各シナリオの `note`）。下の表は scenarios.d より前からあるシナリオだけ —— **この表に行を足さない**（並走する Story が表の末尾を取り合う） |
| `playwright.recording.config.ts` | 録画用の Playwright の設定（video・trace on、操作の注釈と工程の見出しを焼き込む、`retries: 0`、`forbidOnly`）。録画のたびに対象コミットの `web/` へ写される |
| `st22/st22-erase-reload.rec.ts` | ST22「記録の削除」（PC の幅・ライト）: 稼働状況（消す前）→ 削除前 → 確認で「やめる」と消えない（確認文に位置の件数）→ 削除 → 削除後 → 再読み込み後も消えている → 稼働状況は消す前と同じ → 後始末で戻す。アサーションは `web/e2e/day-erase.spec.ts` の写しに spec の同名の Scenario を足したもの。`day-erase.spec.ts` の 3 本も一緒に撮る |
| `st22/st22-phone-dark.rec.ts` | ST22（スマホの幅 390 px・ダーク）: 開く → 消す → 消した行 → 戻す。横にはみ出さないことも見る |
| `st28/st28-login.rec.ts` | ST28 の画面のログイン: 未ログイン（記録は 1 件も返らない）→ 違う合言葉は断られる → 正しい合言葉 → 1 日の画面 → ログアウト → 再読み込みしても出ない。`web/e2e/login.spec.ts` も一緒に撮る。**私設網のホスト名で届く／網の外から届かないは撮らない**（本物の網が要る。人間の確認に残る） |
| `st12/st12-archive.rec.ts` | ST12（書庫）: 稼働状況の「直近に置いた書庫」の箱（形の確認待ちが 1 冊・直近の書庫は確認待ち）→ 取り込んだ YouTube の視聴履歴の格子（見出しに「〜まで（N 日前）」）→ その週を開く。書庫は `../record-env.sh` の `archive_prepare` が録画の前に一時の置き場へ置く（合成の 2 冊。1 冊は形を本人の代わりに確認して取り込み、1 冊は確認待ちのまま）。**本物の Google の書き出しは使わない**（人間の確認 13.1） |
| `st06/St06RecordingTest.kt` | ST06（Android）: 入れたばかりのアプリを初めて起動 → 位置（使用時のみ → 常に）→ 通知 → 利用状況へのアクセス → 常駐の通知 → もう一度開いても設定画面へ送られない → **結果の札**（端末に集まったアプリ利用） |
| `st06/St06LaterGrantRecordingTest.kt` | ST06（Android）: 位置を許可しない・利用状況へのアクセスを許さずに戻る → それでも収集は始まる → 後から常駐の通知をたどって許可 → 次の契機（開き直し）からアプリ利用が集まる（結果の札） |
| `st05/St05RecordingTest.kt` | ST05（Android）: 収集を始めるとその場で測る（結果の札）→ 端末の時計を 5 分進める → その場で測り直す（差が約 +300000 ms。結果の札）→ 時計を戻す。PC の時計は動かさない（Windows の実行時テストに任せる） |
| `st17/st17-feel.rec.ts` | ST17「その日どう感じたか」（幅 360 px・暗）: 今日の主観入力 → 快を 1 回押すと保存 → ひとこと → 元気さ → 書き直した履歴 → 前の日へ → 暦（未記入の破線と「未」）→ 滞在の行と履歴 → 通知の時刻を変える（後始末で 21:00 に戻す）→ 1 日の画面の頭の気分（記入あり・未記入から「書く」・使い始める前）と滞在の行の気分 → 明。書くのは乱数で選んだ 2100 年以降の日（`page.clock`）。アサーションは `web/e2e/feel*.spec.ts` `day-feeling.spec.ts` の写し |
| `st17/St17RecordingTest.kt` | ST17（Android）: 書いていない日の 21:05 に受け手が起き、本物のサーバに聞いて「今日はどうでしたか」が出る → 押すとその日の主観入力のアドレスが開く（結果の札）→ 書いた日は出ない（結果の札）。時計だけ差し替え、予約は張らない。**押した先で本物のブラウザは開かない**（APK は Web のアドレスを持たない）—— 受け皿の画面で受けたアドレスを見せ、開いた先の画面は ST17-feel が撮る |
| `../record-env.sh` | 専用環境。`up` は対象コミットのコードをビルドし、専用の compose project・空いている port・乱数の合言葉・作り直した偽データで `tools/stack.sh up` を立てる。Android のシナリオがあれば、録画用のサーバへ送る APK（接続先は端末の中の `127.0.0.1:18787`。ハーネスが `adb reverse` で API へ渡す）も作る。`down` はその compose project だけを volume ごと消し、残りがあれば rc=1 |

```bash
scripts/record-run setup                 # 初回: 前提の道具と差し込み口
scripts/record-run run 23938e7           # 手で 1 回撮る（コミットを省くと HEAD）
```

確認バッチ（`scripts/hx verify`）は、束ねた Story にシナリオがあれば自動で撮り、手順書の「違和感」の問いに動画の場所を添える。

PC（Windows）のシナリオは、**本人の画面では撮らない** —— ハーネスが毎回まっさらな Windows Sandbox を起こし、中のデスクトップを
ffmpeg で撮る（Sandbox は管理者の PowerShell で `Enable-WindowsOptionalFeature -Online -FeatureName Containers-DisposableClientVM -All`
→ 再起動。ffmpeg は `recording.json` の `windows.ffmpeg` に Windows の `ffmpeg.exe`）。画面の下端の帯が工程の見出しで、前景は取らないので記録には出ない。
Sandbox の網は切ってあり、収集アプリは送れずに未送信に積む。閉じた後に `record-env.sh after-windows` が未送信を録画用のサーバへ送り、
その時間に 1 か所の位置を置いて滞在を作る（1 日の画面は PC の記録を滞在の中に出す）。PC の収集アプリは `up` が Windows 側の cargo で
release に組む（写しと target は `C:\dev\ashiato2-rec-build` に道具のキャッシュとして残る）。
`crates/collector-windows/src/` を変えた Story は、merge gate がこの面のシナリオを求める（`windows.src_dir`）。

Android のシナリオは `-e recording 1` のときだけ走る計測テストで、各工程の見出しは Toast、
アプリに画面が無いところは**結果の札**（録画のテストが出す「【録画の結果】」の通知。中身はアプリの未送信の置き場を読んだもの）で見せる。
送信は 5 分ごとなので、動画の間にサーバへは届かない（届くことは計測テストと `tools/smoke.sh` が持つ）。

シナリオを足すとき: `scenarios.d/<YYYYMMDDHHMM>_st<NN>.json` を新しく置く（中身は `recording.json` の `scenarios` と同じ形の配列で、
`id` / `story` / `spec` / 一緒に走らせる既存のテスト `with` / 対象コミットに要るファイル `requires` / 説明 `note`）。`tools/recording/st<NN>/` に
`.rec.ts`。**`recording.json` とこの README の表には書き足さない**。**アサーションは既存の e2e から写して弱めない**。
見るための停止は `test.step` の見出しと短い `waitForTimeout` だけにする。
