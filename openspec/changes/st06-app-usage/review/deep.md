# st06-app-usage 深掘りの独立レビュー

`openspec/schemas/ashiato/schema.yaml` の `deep` の手順 1〜5 を、問いの一覧を見る前に独立にやり直し、
そのあとで `openspec/changes/st06-app-usage/deep-questions.json`（6 問。A 5 / B 1）と突き合わせた。

## 手順ごとの結果（確かめた範囲）

- **手順 1（要件どうしの衝突）**: `satisfies` は FR-2 の 1 件のみ。FR-2 と NFR-1（Q6 が持つ）、
  FR-2 と NFR-13 の主語の仕分け（Q2 が持つ）のほかに、FR-2 と FR-8 / FR-9 / NFR-7（端末バッファを
  全ソース通しで食い合う）を見たが、正典が「ソースを問わず、全ソースを通して古い順」と明示しており
  （`openspec/specs/device-collection/spec.md`）衝突ではない。代償の記述が Q1 に無い点だけ R9 に挙げた。
- **手順 2（扉の幅）**: ST06 の `doors` は空（`docs/stories/ST06.md`）。関与要件の側から扉 #6（時刻を 2 本持つ）と
  扉 #14（稼働記録）を見た。#14 はこの Story の文脈でも幅が無い（担い手は FR-78 / FR-80 で既に立っている。R5 参照）。
  #6 だけ幅が残る（過去のイベントにどの地域を付けるか。R12）。
- **手順 3（新たに立つ一方通行）**: 5 件見つけた —— 取る種別（Q1 が持つ）／過去の集計（Q5 が持つ）／
  権限が無いときの振る舞い（Q3 が**片方向だけ**持つ。R1）／**1 イベントのどの欄を `raw` に載せるか**（R2）／
  **取れなかった窓を進めてしまう経路**（R3 / R4）。後ろ 3 件が一覧に無い。
- **手順 4（日常に影響する選択）**: 常時通知（増えない。文言だけが位置のまま＝R10 に畳んだ）、電池
  （契機 1 本増。Q6 が持つ）、容量（R9）、毎日目に入るもの（稼働状況の格子に 6 本目が出るか＝R7）、
  手作業（初回の設定画面＝Q3 が持つ）を見た。**新しく立てるべき `daily` の問いは見つからなかった。**
- **手順 5（既存コードが要件を満たしていない箇所）**: `collector-android/` を 2 本目のソースが乗る前提で
  読み直し、3 件見つけた（R1 / R10 / R11）。

## 前提の確認（依頼された 2 件。どちらも**正しい**）

- `UsageStatsDatabase.prune()` は 年 -2 年 / 月 -6 か月 / 週 -4 週 / 日 -10 日。
  逐語: `mCal.addYears(-2)` … `mCal.addDays(-10); pruneFilesOlderThan(mIntervalDirs[UsageStatsManager.INTERVAL_DAILY], …)`
  （https://android.googlesource.com/platform/frameworks/base/+/refs/heads/main/services/usage/java/com/android/server/usage/UsageStatsDatabase.java 、2026-09-18 取得、`prune(long)`）。
  `queryEvents` が日ごとの箱だけを読むことも裏が取れた —— `UserUsageStatsService.queryEvents` は
  `queryStats(INTERVAL_DAILY, …)` を呼ぶ（同 `UserUsageStatsService.java:557`）。
- `queryEvents` の範囲は [begin, end)。逐語: "beginTime long: **The inclusive beginning** of the range of events to include in the results."
  / "endTime long: **The exclusive end** of the range of events to include in the results."
  （https://developer.android.com/reference/android/app/usage/UsageStatsManager 、2026-09-18 取得）。

なお **A に置いた 5 問（Q1・Q2・Q3・Q4・Q5）は、どれも「計算し直せば戻る」ものではなかった**
（Q4 だけは根拠が違う。R5）。B の Q6 にも失われるものは無い。分類違いは 1 件も見つかっていない。

---

## R1. 「アプリ利用の権限が無いとき位置をどうするか」の**裏面**（位置の権限が無いときアプリ利用をどうするか）が問われていない

- 成果物: openspec/changes/st06-app-usage/deep-questions.json
- 根拠: `collector-android/app/src/main/kotlin/dev/ashiato/collector/MainActivity.kt:77-79`
  （前景の位置が無ければ `finish()`。`startForegroundService` に到達しない）/
  `LocationService.kt:288-292`（`fixSource.start` の `SecurityException` で `stopSelf()` + `START_NOT_STICKY`）/
  同 `:294-295`（`startFlushing()` と `startBeating()` はその**後**にある＝生存信号も出ない）/
  `openspec/specs/device-collection/spec.md`「Android は長期間使っていないアプリの権限を自動で剥がす」
- kind: irreversible
- loss: uncaptured
- 処置: escalated —— Q7 を新設した（deep.md の第 1 回の表に R1 として残した）
- 提案: Q3 と対になる問いを 1 本足す —— 「**ソースが 2 本になったとき、片方の取得条件が欠けたら、もう片方の収集をどうするか**」。
  いまの実装は「位置が欠けたら全部止まる」で、止まっている間はアプリ利用も生存信号も出ない
  （画面には③「取れない状態」ではなく⑥「途絶」が出る）。10 日を越えればそのアプリ利用は二度と取れない。
  ST06 は**C-01 に 2 本目のソースを載せる最初の Story**なので、ここで決めた形を ST09 / ST11 / ST34 / ST35 が引き継ぐ。
  あわせて `AndroidManifest.xml` の `foregroundServiceType="location"` 1 本と `targetSdk = 36`
  （`collector-android/app/build.gradle.kts:14`）の組み合わせで、位置の権限が無いまま前景サービスを
  立てられるかは下流で確かめる必要がある（この問いの選択肢の実現可能性に効く）。

## R2. 1 イベントの**どの欄を `raw` に載せるか**が問われていない（Q1 は「種別」しか聞いていない）

- 成果物: openspec/changes/st06-app-usage/deep-questions.json
- 根拠: `UsageEvents.Event` の公開 getter は `getPackageName` / `getClassName` / `getEventType` / `getTimeStamp` /
  `getConfiguration`（CONFIGURATION_CHANGE のときだけ）/ `getShortcutId`（SHORTCUT_INVOCATION のときだけ）/
  `getExtras`（USER_INTERACTION のときだけ。`EXTRA_EVENT_ACTION` / `EXTRA_EVENT_CATEGORY`）/
  `getAppStandbyBucket`（STANDBY_BUCKET_CHANGED のときだけ）の 8 つ
  （https://developer.android.com/reference/android/app/usage/UsageEvents.Event 、2026-09-18 取得）/
  `docs/collector-contract.md`「冪等キー: サーバが `logical_source` + `event_time` + `raw` から SHA-256 で作る」/
  `migrations/202609082001_immutable_collected.sql:11-19`（`raw` / `payload` / `event_time` の書き換えを DB が拒む）
- kind: irreversible
- loss: uncaptured
- 処置: escalated —— Q8 を新設した（表示名を `raw` に入れない既定は deep.md の C2 に R2 として残した）
- 提案: 問いを 1 本足す（または Q1 に 2 つ目の軸として畳む）。「種別を全部取る」を選んでも、
  **1 件から読む欄を絞れば、絞った分は 10 日で消える** —— 種別の選択と欄の選択は別の一方通行。
  同時に、いまの C（「アプリの表示名を取得時に解決して載せる」）は**冪等キーの入力を動かす**:
  表示名はアプリの更新・端末の言語変更で変わるので、重ねた窓で取り直したときに `raw` が変わり、
  別の鍵として**行が増える**（`docs/collector-contract.md`「収集側は 1 件を同じ形で直列化し続けなければならない」）。
  表示名を `raw` に入れるか `payload` だけに入れるかは、C のまま黙って決めてよい種類ではない。

## R3. 「取りに行ったが取れなかった」を**空振りと取り違える経路**が 2 つあり、C の一覧に入っていない

- 成果物: openspec/changes/st06-app-usage/deep-questions.json
- 根拠: `queryEvents` の逐語「Note: Starting from Android R, if the user's device is **not in an unlocked state**
  (as defined by `UserManager.isUserUnlocked()`), then **null will be returned**」
  （https://developer.android.com/reference/android/app/usage/UsageStatsManager 、2026-09-18 取得）/
  `UserUsageStatsService.java:440-441` `validRange`: `return beginTime <= currentTime && beginTime < endTime;`
  （偽なら `queryEvents` は **null** を返す。https://android.googlesource.com/platform/frameworks/base/+/refs/heads/main/services/usage/java/com/android/server/usage/UserUsageStatsService.java 、2026-09-18 取得）
- kind: technical
- 処置: fixed deep.md —— C5 に足した。`null` と 0 件を区別し、`null` のときは窓を進めない
- 提案: 問いにはしない（扉を開けたままにする既定が明らかで、費用も小さい）。
  ただし **C の一覧に 2 項足す** —— (1) 再起動後に一度も解錠されていない端末では `null` が返るので、
  **`null` と「0 件」を区別して、`null` のときは窓を進めない**。(2) 端末の時計が先へ飛んで
  窓の始まりが現在時刻を超えると `validRange` が偽になり、**時計が追いつくまで全部 `null`** になる
  （ST04 は端末の時計が数か月飛ぶ前提で `AgeClock` を作っている）。
  いまの C は「窓は重ねて進める（冪等で畳まれる）」「時計が戻ったときは窓を進めない」の 2 つだけで、
  どちらもこの 2 経路を塞がない。塞がないまま窓を進めると、その期間は 10 日で OS から消える。

## R4. 「端末の時計が戻ったときは窓を進めない」の根拠が事実と違う —— OS 側が**過去の記録を丸ごとずらす**

- 成果物: openspec/changes/st06-app-usage/deep-questions.json（C の一覧 / 問いの前提）
- 根拠: `UserUsageStatsService.checkAndGetTimeLocked()` は時計の食い違いが閾値を超えると
  `onTimeChanged(expectedSystemTime, actualSystemTime)` を呼び（同 `:283-287`）、
  `UsageStatsDatabase.onTimeChanged(long)` が**すべての統計ファイルを差分だけ改名する**
  （`final long newTime = files.keyAt(i) + timeDiffMillis; … file.getBaseFile().renameTo(newFile)`。
  `newTime < 0` のファイルは削除される。UsageStatsDatabase.java:698-742、2026-09-18 取得）/
  `checkAndGetTimeLocked()` は `queryEvents` の入口で呼ばれる（`validRange(checkAndGetTimeLocked(), …)`）
- kind: premise
- 処置: fixed deep.md —— C6 を書き直した。単調な経過時間で窓を押さえ、飛びが直るまで取り直さない
- 提案: C の当該項を書き直す。**窓を進めないだけでは足りない** —— 時計が直った瞬間に、
  既に送ったイベントが**別の時刻**で再び読めるようになる。冪等キーは `logical_source` + `event_time` + `raw`
  なので、ずれた時刻で取り直したものは**畳まれずに行が増え**、しかも `event_time` は凍結されていて
  後から直せない（`migrations/202609082001_immutable_collected.sql`）。
  「重ねた窓は冪等で畳まれる」という C の前提が、この 1 点でだけ成り立たない。
  問いにするか C に落とすかは呼び出し元の判断だが、**いまの書きぶりのままでは事実と違う**。

## R5. Q4 の `why` と `context` が誤り —— 挙げられた 3 例はどれも受け手側で⑥「途絶」になる

- 成果物: openspec/changes/st06-app-usage/deep-questions.json（Q4）
- 根拠: `crates/server/src/coverage.rs:940-981` の `decide()` —— 記録が 0 件で生存信号も無く、
  想定間隔（登録簿の `c01-app-usage` は 21600 秒 = 6 時間。`migrations/202609111111_coverage_rebuild.sql:129`）の
  近傍に活動が無い日は `DayState::Outage`（⑥途絶）を返す。②`AliveNoRecord` になるのは
  **取得できる状態の生存信号が届いている日だけ**（`Some(true) => AliveNoRecord`）/
  権限が無い期間は生存信号の `blockers` により③`AliveNotCapturable`（`:964-965`）/
  導入前は⑦`BeforeStart`（`:947-948`、FR-79）
- kind: premise
- 処置: fixed deep-questions.json —— Q4 の why と context を書き直した。途絶になる 3 例を外し、生存信号が届いているのに窓が届かない 3 経路に置き換えた
- 提案: Q4 の `why`「そもそも OS から取れなかった期間は、いまのどの経路にも乗りません」と
  `context`「取れた分だけ送ると、受け手には『その期間は使っていなかった』と同じに見えます」を直す。
  **端末を 10 日以上止めた・アプリが 10 日以上動かなかった・端末を初期化した**の 3 例は、
  いずれも記録も生存信号も来ないので⑥「途絶」になり、「使っていなかった」（②）とは既に区別されている。
  本物の穴は**生存信号が「取れる状態」で届き続けているのに、窓が 10 日より後ろへ届かない**場合だけ
  （R3 の 2 経路・R4 の時計・契機だけが長く止まった場合）。この形に書き直さないと、
  答える人は「要らないのでは」と読むか、誤った理由で必要と読むかのどちらかになる。
  問いそのものは残す —— 書き直した後でも `uncaptured` は消えない。

## R6. Q2 の答えは `collection-coverage`（ST12 が走っている capability）の変更を要求するのに、その着地点が書かれていない

- 成果物: openspec/changes/st06-app-usage/deep-questions.json（Q2）
- 根拠: 主語の仕分けは**正典の Requirement 本文**にある ——
  `openspec/specs/collection-coverage/spec.md:501-506`「**端末が主語**（位置・アプリ利用）は**記録が 1 件以上ある日**」/
  実装も定数で持つ `crates/server/src/coverage.rs:28` `pub const DEVICE_SUBJECT: [&str; 2] = ["c01-location", "c01-app-usage"];` /
  ST12 は同じ capability に `MODIFIED` を書いている（`openspec/changes/st12-archive-ingestion/specs/collection-coverage/spec.md`）/
  `python3 scripts/board.py` は ST12 を［下流］に置いている（走行中）/ CLAUDE.md「同じ capability を 2 本が同時に触ると差し戻しが起きる」
- kind: premise
- 処置: fixed deep-questions.json —— Q2 の context に着地点を足した。正典と定数は ST12 の archive 後、ST14 への申し送りに残す
- 提案: Q2 の `context` に、**選んだ結果がどこへ行くか**を 2 行足す。Q4 は同じ制約を明示して
  device-collection 内に閉じているのに（「破棄の報告に理由を増やす形は取りません。それは
  `collection-coverage` の変更にあたり、いま ST12 がその capability を走らせています」）、
  Q2 だけが同じ扱いになっていない。書くべきは「ST06 が触るのは `docs/requirements.md` の NFR-13 の逐語までで、
  `collection-coverage` の spec と `coverage.rs` の定数は ST12 の archive 後に拾う」。
  **放置すると要件と正典が食い違ったまま残る型の事故が既に 1 件ある** —— ST07 が NFR-13 を 4 回目に訂正して
  「PC 上の C-02 が集める 2 ソースだけは生存信号が届いた日を分母とする」を足したが、
  `collection-coverage` の spec:501-524 にその但し書きは入っていない（ST07 の spec delta は
  `desktop-collection` だけ。`openspec/changes/archive/2026-09-14-st07-active-window/specs/`）。

## R7. Q5 の「登録簿へ 1 行足すだけで、契約の形は変えません」が誤り

- 成果物: openspec/changes/st06-app-usage/deep-questions.json（Q5）
- 根拠: `migrations/202609120940_source_columns.sql` —— `external_id_kind` の**既定は `'record'`（＝断る側）**で、
  `'none'` に倒されたのは「**列が生まれた瞬間に居た行だけ**」（`IF just_added THEN UPDATE …`）。
  新しく足す行は既定のままだと `missing_external_id` で断られ、しかもそれは**恒久ではない**扱いなので
  未送信から取り除かれず端末に溜まり続ける（`openspec/specs/device-collection/spec.md`「断られた理由が
  受け手側の設定で変わりうるとき … その記録を**未送信に残す**」）/
  `crates/server/src/lib.rs:1547-1556` の `coverage_get` は **`must_sources()` の 5 本しか返さない**ので、
  6 本目の格子は画面に出ない（FR-54 は「各ソースについて」8 状態の表示を求めている。表示を足すのは
  `collection-coverage` ＝ ST12 の領分）/ `expected_gap_sec` を与えないと FR-35 / FR-80 の判定が定まらない
  （`migrations/202609111111_coverage_rebuild.sql:126-133`）
- kind: premise
- 処置: fixed deep-questions.json —— Q5 の context の「1 行足すだけ」を直した。想定間隔と外部識別子の粒度が要り、画面には当面出ない
- 提案: Q5 の `context` の当該文を直す。足すのは 1 行だが、その行には**想定間隔**と
  **外部識別子の粒度**が要り、既定のままだと**端末の未送信が永久に詰まる**。さらに、
  取り込んだ集計は**稼働状況の画面に出ない**（ST12 が走っている間は出せない）。
  「粗いが他のどこにも無い過去」が入ることは変わらないので、答えが変わる話ではないが、
  「1 行だけ」と読むと下流が踏む。

## R8. Q5 の `context` に、集計の側の事実が 2 つ欠けている

- 成果物: openspec/changes/st06-app-usage/deep-questions.json（Q5）
- 根拠: `UsageStatsDatabase.pruneStats()` は**インストールされていないパッケージの
  `packageStats` と `events` を落とす**（`if (timeInstalled == null || timeInstalled > usageStats.mEndTimeStamp)`。
  UsageStatsDatabase.java:668-694、2026-09-18 取得）。`pruneUninstalledPackagesData()` も同じことを
  全ファイルに対して行う（同 :597-631）/
  `queryUsageStats` の逐語「Note: The begin and end times of the time range **may be expanded to the nearest
  whole interval period**.」「The returned list will contain **one or more** `UsageStats` objects **for each package**」
  （https://developer.android.com/reference/android/app/usage/UsageStatsManager 、2026-09-18 取得）
- kind: premise
- 処置: fixed deep-questions.json —— Q5 の context に 2 行足した。消したアプリの過去は取れない / 範囲は箱の境界まで広がる
- 提案: `context` に 2 行足す。(1) **既に消したアプリの過去は取れない** ——
  「約 2 年ぶんの『どのアプリを何時間使ったか』を既に持っています」は、
  **いま端末に入っているアプリについてだけ**真。QS-7（いちばん時間を使ったアプリ）と
  QS-10（関心の移り変わり）に効く話なので、答える人の判断材料になる。
  (2) 要求した範囲は**区間の境界まで広げられ**、同じアプリが複数件返る ——
  「2026 年の年ごとの箱」と名乗って入れた 1 件の実際の範囲が要求と違いうるので、
  `raw` に何を入れるか（＝冪等キー。R2 と同じ型）がここでも効く。

## R9. Q1 の選択肢 1 の代償に、端末バッファ（全ソース通しの 2 GB / 90 日）を共有することが書かれていない

- 成果物: openspec/changes/st06-app-usage/deep-questions.json（Q1）
- 根拠: `openspec/specs/device-collection/spec.md`「IF 未送信の記録の合計が 2 GB を超える THEN …
  **端末に積んだ順が古いもの**から、2 GB 以下になるまで破棄する（**ソースを問わず、全ソースを通して古い順**）」/
  同「位置だけでも 90 日で約 94 MB・129,600 件」/ Q1 の `context`「件数は実測していません」
- kind: technical
- 処置: fixed deep-questions.json —— Q1 の context に、端末の置き場を位置と食い合う代償を足した
- 提案: Q1 の `context` に 1 行足す。年 3〜6 GB（NFR-5）の側は「あとから間引ける」ので軽いが、
  **端末側の 2 GB は位置と食い合う**ので、種別を増やすと長い圏外のときに**位置の記録が先に捨てられる**
  （古い順だから）。下流の実測で決めればよいのは容量の側だけで、この代償は選ぶ前に見えているほうがよい。

## R10. 収集側が**単一ソース前提**で書かれている箇所が 4 つある（2 本目を足すと数えとログが混ざる）

- 成果物: openspec/changes/st06-app-usage/deep-questions.json（問いではなく deep.md の「確かめたが問わなかったこと」へ）
- 根拠: `collector-android/app/src/main/kotlin/dev/ashiato/collector/Telemetry.kt:19`
  `append(" source=").append(LOGICAL_SOURCE)`（定数 `"c01-location"`。`LocationFix.kt:10`）——
  アプリ利用のログも `source=c01-location` と名乗る（正典「ログに出すものを、件数・**ソース名**・所要時間・
  エラーの種別に限る」に対して、名乗りが誤る）/
  `Heartbeat.kt:107` `private val intervalMs: Long = FIX_INTERVAL_MS`（満点の刻みが 60 秒固定。
  アプリ利用の契機が 30 分なら、6 時間の生存信号は試行 360 / 成功 12 になり、画面上は取得率 3 %）/
  `LocationService.kt:153` `FileCounterStore(File(filesDir, "heartbeat-counters.txt"))`（**ファイル名が 1 本**。
  2 つ目の数えを同じ名前で開くと互いに潰し合う）/ `Heartbeat.kt:250` `logicalSource: String = LOGICAL_SOURCE` /
  `Retention.kt:164` `BASE_TEXT = "位置を記録しています"` と `LocationService.kt:410` `CHANNEL = "location"`（常駐通知の文言）
- kind: technical
- 処置: fixed deep.md —— 「確かめたが問わなかったこと」に残した。tasks で 4 か所とも塞ぐ
- 提案: 問いにしない（人間が決めることが無い）。deep.md の「確かめたが問わなかったこと」に書き、
  tasks で 4 か所とも塞ぐ。とくに `intervalMs` は、塞がないと**アプリ利用の生存信号が
  『ずっと眠っていた』と読める数を毎回送る**ので、FR-78 の目的（取得率で区間を見分ける）が逆に働く。

## R11. `Retention` が破棄の報告の範囲の終わりに「**置き場全体**の最も古い記録」を使っている（正典は「同じソース」）

- 成果物: openspec/changes/st06-app-usage/deep-questions.json（同上。tasks / design へ）
- 根拠: `collector-android/app/src/main/kotlin/dev/ashiato/collector/Retention.kt:87-88`
  `val remaining = records.oldest()?.item?.eventTime?.let(::instantOf)` →
  `for (source in touched) ledger.endBatch(source, reason, remaining)`（**touched の全ソースに同じ値を渡す**）/
  `Outbox.kt:101` `fun oldest(): Stored<T>? = store.head(1).firstOrNull() ?: unwritten.firstOrNull()`
  （ソースで絞っていない。記録の置き場は `store("records")` 1 本。`LocationService.kt:103-127`）/
  正典 `openspec/specs/device-collection/spec.md`「範囲の終わりを、**同じソースで**破棄せずに残った
  最も古い記録の出来事の時刻が …」/ 受け取り側の畳み込み `DropReport.kt:222-232`
- kind: technical
- 処置: fixed deep.md —— 同上。2 本目を同じ置き場に載せる前に塞ぐ
- 提案: 問いにしない。**いまは位置 1 本なので「置き場の先頭」＝「同じソースの先頭」で偶然一致している**が、
  ST06 が 2 本目を同じ置き場に載せた瞬間に、片方の破棄の報告の範囲がもう片方の記録の時刻で閉じる
  （1 時間以内なら `endBatch` がそのまま採用する）。扉 #14 の証拠の範囲が静かに誤るので、tasks に入れる。

## R12. 扉 #6 の幅 —— **過去の**イベント（最大 10 日前、Q5 なら 2 年前）に、取得時点の地域を付けることになる

- 成果物: openspec/changes/st06-app-usage/deep-questions.json（問いにするか C に落とすかは呼び出し元）
- 根拠: `LocationFix.kt:57-58` `val offsetMin = zone.rules.getOffset(at)…` / `FixCollector.kt:23`
  `zone = ZoneId.systemDefault()`（**サービス起動時に 1 回読む**）——
  位置は「いま取れた 1 点」なので取得時刻＝出来事の時刻だが、アプリ利用は遡って取る /
  扉 #6「出来事の時刻を 2 本持ち、UTC ずれとタイムゾーン識別子の両方を持つ」（決定済）/
  `migrations/202609100000_immutable_origin.sql:17`「**`tz_id` / `tz_offset_min` は凍結しない**」
- kind: technical
- 処置: fixed deep.md —— C12 に残した。design の D 番号に反転条件つきで書く
- 提案: **失われるものは無い**（`tz_id` / `tz_offset_min` は後から直せると DB 側が明示している）ので、
  A の問いにはしない。ただし「遡って取ったイベントに、取得時点の地域を付ける」ことは
  黙って決まってよい種類ではない（移動した日の 10 日ぶん、Q5 なら 2 年ぶんの箱が全部それになる）。
  C の一覧に 1 項足して deep.md に残し、design の D 番号（仮）にする。
