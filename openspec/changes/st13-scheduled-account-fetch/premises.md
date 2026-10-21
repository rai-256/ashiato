# ST13 の前提 —— 外の形式・振る舞い

FR-15（アカウント系のソースを登録簿の間隔で定期取得する）が成り立つかは、**定期取得できる口が外にあるか**に依る。
問いを立てる前に確かめた（2026-10-20）。

| 前提 | 何が依るか | 確かめ方 | 結果 | 出所 | 確かさ |
|---|---|---|---|---|---|
| Google の Data Portability API（YouTube・マイアクティビティ・Chrome の履歴などを API で書き出す口）は日本の利用者に提供されていない | Q1（定期取得の対象）。使えれば Takeout の 2 か月の手作業の一部を定期取得に移せた | 公式のヘルプ「Share a copy of your data with a third party」の Common Questions の国の一覧を読む | 一覧は EU 諸国・スイス・英国の 29 か国・地域だけ。**日本は無い**。逐語: "is available in the following countries and regions: Austria, Belgium, … Switzerland, United Kingdom" / 無い地域は Takeout を使うよう案内している | 公式文書 https://support.google.com/accounts/answer/14452558 （開発者向けの入口 https://developers.google.com/data-portability/user-guide/introduction が "verify that the Data Portability API is available to users in your location" としてここを指す） | 2 |
| Data Portability API のスコープは restricted で、公開するには審査とセキュリティ評価が要る | 同上（日本で使えるようになっても、残る障害） | 開発者向けの概要を読む | 逐語: "If you select restricted scopes, your product will be subject to an additional security review during verification." / 審査に "Completing a security assessment" を含む。ST12 の deep Q1 の補足（旧プロジェクトの EXT-11）と一致 | 公式文書 https://developers.google.com/data-portability/user-guide/overview | 2 |
| Data Portability API の時間付きの許可は 30 日か 180 日で、切れたら本人が許可し直す | 同上（使えても、NFR-12 の「2 か月に 1 回」の手作業に収まるかの見積もり） | 開発者向けの入口と release notes を読む | 逐語: "time-based access for 30 or 180 days"。180 日なら 2 か月に 1 回より少ない | 公式文書 https://developers.google.com/data-portability/user-guide/introduction | 2 |
| Google の OAuth の同意画面が「テスト」の間は承認と refresh token が 7 日で切れる | Google の API 一般を自宅の 1 人の道具から使う場合（審査を通さない形）。毎週の許可し直しは NFR-12 を破る | 要件 §5 EXT-A を一次情報で読み直す | 逐語: "Authorizations by a test user will expire seven days from the time of consent." / "If your OAuth client requests an offline access type and receives a refresh token, that token will also expire." | 公式文書 https://support.google.com/cloud/answer/15549945 （EXT-A） | 2 |
| YouTube の視聴履歴を返す API は無い（2016 年に止まった） | Q1（YouTube を API で取る余地） | ST12 の deep Q1 の補足（旧プロジェクトの EXT-7）を引く | 止まった。いまの要件の EXT 表には無い | 既存の調査（`openspec/changes/archive/2026-10-13-st12-archive-ingestion/deep.md` Q1） | 1 |
| マップのタイムラインは Takeout にも API にも無く、端末の設定から書き出すだけ | Q1（位置の履歴を定期取得に移す余地は無い） | 要件 §5 EXT-M を引く | 逐語: "Under "Timeline," tap Export Timeline data. …" | 公式文書 https://support.google.com/maps/answer/6258979 （EXT-M） | 2 |
| 要件に Google 以外のアカウント系のソース（外のサービスにログインして取るもの）は無い | Q1（定期取得の対象が要件の中に 1 つでもあるか） | `docs/requirements.md` の FR-1〜FR-17・§5 の扉と EXT 表、登録簿の行（`migrations/*.sql` の `core.source` への INSERT）を読む | 無い。登録簿は端末（`c01-*`）・PC（`c02-*`）・書庫（`c03-*`）・サーバが作るもの（`s01-*`）だけ。§5 の「やらないこと」に購入履歴が「将来の最初の候補」として挙がっているだけ | repo（`docs/requirements.md` §3.1・§5、`migrations/`） | 3 |
| 同意画面が「本番」（未審査）なら、refresh token は 7 日では切れない。切れるのは取り消し・6 か月使わない・期間付きの許可の満了など | Q1 の「ドライブ経由」（自宅に置いた資格情報で 2 か月ごとに取りに行けるか）。R2 | Google の OAuth の公式文書の Refresh token expiration を読む | 7 日で切れるのは "Testing" の状態だけと書かれる。ほかは取り消し・6 か月の不使用・パスワードの変更（Gmail のスコープ）・期間付きの許可の満了・数の上限。2 か月ごとに使えば 6 か月の不使用には当たらない | 公式文書 https://developers.google.com/identity/protocols/oauth2 （Refresh token expiration） | 2 |
| 未審査の公開アプリは警告の画面を出し、新しい利用者は 100 人まで | 同上（本人 1 人なら上限に当たらない） | 公式のヘルプを読む | 逐語: "100 new users in total, after the app presents the unverified app screen" | 公式文書 https://support.google.com/cloud/answer/7454865 | 2 |
| Takeout は書庫の届け先に Google ドライブを選べ、予約エクスポートと組める | Q1 の「ドライブ経由」。R1 | EXT-B と同じヘルプを読む | 届け先に "Add to Drive"（"We'll add your archive to Google Drive and email you a link to its location."）。予約は "every 2 months for one year" | 公式文書 https://support.google.com/accounts/answer/3024190 （EXT-B） | 2 |
| ドライブの書庫を読むスコープは restricted（`drive.readonly`）。non-sensitive の `drive.file` は本人がその都度選んだファイルしか読めない | Q1 の「ドライブ経由」で自宅に置く資格情報の強さ（**ドライブの全ファイルを読める**） | Drive API の公式文書のスコープの表を読む | `drive.readonly` は Restricted、"View and download all your Drive files."。`drive.file` は "files … that you open with an app or that the user shares with an app"（Picker で選ぶ = 手作業） | 公式文書 https://developers.google.com/workspace/drive/api/guides/api-specific-auth | 2 |
| Android 11 以上は、数か月操作されないアプリを休止させ権限を剥がす。前景サービスや予約の仕事が動いていても「使った」に数えない | Q4（携帯の収集アプリの権限が剥がれる）。R3 | Android の公式文書 App hibernation を読む | 逐語: "the user doesn't interact with your app for a few months, the system places your app in a hibernation state" / "This excludes usage that would not occur if the app is force stopped. For example, running a job does not count as usage"。通知を触るのは使ったに数える。本人に除外を頼む口は `IntentCompat.createManageUnusedAppRestrictionsIntent` | 公式文書 https://developer.android.com/topic/performance/app-hibernation | 2 |
| 収集アプリには休止を防ぐ手が無い（剥がれたことを生存信号で報告するだけ） | Q4 | `grep -rniE "hibernat|unused|revoke" collector-android/app/src/main` と `AndroidCapability.kt:19`・`LocationService.kt:312-313` を読む | 防ぐコードは 0 件 | repo | 3 |

**失われるもの（`loss`）のある決定（Q1）は、確かさ 4 以上の前提にだけ載せる**の規則について:
Q1 の `loss`（`uncaptured`）は「本人が使っていて、取得元の保持が短いアカウントのサービスがあるのに、定期取得を作らない」側にある。
それが**あるかどうかは本人しか知らない**（実物は本人の手元）ので、Q1 そのものが本人に実物を聞く問いになっている。
Data Portability API が日本で使えないことは確かさ 2（公式文書）で止めた —— 本物で確かめるには Google Cloud の
プロジェクトと OAuth の同意画面を本人の資格で作る必要があり、外へ出る操作になる。**この前提が外れても失われるものは無い**
（外れたら Takeout の 2 か月の経路のまま、定期取得を後から足せる。書庫の経路は残る）。**ただし Takeout 側が古い分をいつまで持つかは分かっていない**（FR-35 の ★ C8。マイアクティビティには自動削除の設定もある）ので、「後から足せる」は Takeout の書庫を 2 か月ごとに取り続ける限りの話ので、Q1 の推奨はこの前提に依らない形にした。

**ドライブ経由（Q1 の選択肢）の前提は確かさ 2 で止まっている**（2026-10-20。deep-review R1 / R2 で足した 4 行）。
本物で確かめるには、本人の資格で Google Cloud のプロジェクトと同意画面を作り、`drive.readonly` の許可を 1 回取って
数週間おいて使えるかを見るしかない（外へ出る操作）。**本人が Q1 でこの道を選んだら、spec の前に第 2 回で実物を求める**
（同意画面を作って許可を取り、取れた refresh token で 8 日目以降もドライブの一覧が読めたかだけを貼ってもらう）。
Q4（休止）の前提も確かさ 2。エミュレータでは「数か月」を早送りできない（`adb shell cmd app_hibernation` は休止を手で入れるだけ）ので、
Q4 の推奨は前提が外れても失わない側（除外を頼む口を置く。外れていれば押されないだけ）にした。
