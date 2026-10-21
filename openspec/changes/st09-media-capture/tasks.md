# ST09 実装タスク — 写真と動画をメタごと取り込み、写真は原本も置く

読む順: `deep.md`（**最優先。本人が決めた Q1〜Q17 と、聞かずに決めた C1〜C19**）→ このファイル →
`specs/device-collection/spec.md` → `specs/collection-coverage/spec.md` → `specs/data-durability/spec.md` → `design.md` → `premises.md` →
`docs/stories/ST09.md` → `docs/handoff/`（開始時と PR 前の 2 回）→ `CLAUDE.md`。

**移行は 1 本だけ足す**（design D15）。**名前は作成時刻 `YYYYMMDDHHMM_media_capture.sql`（と `.down.sql`）**（連番にしない）で、
**`migrations/ORDER` の末尾に 1 行**足す（`crates/server/build.rs` がここから `MIGRATIONS` を組む）。
**`record-deletion` の要件と `deletion.rs` の連鎖には触らない**（design D23）。**`/ingest` の形と 2 MB の上限は変えない**（design D10）。

検証は各タスクの本文に書いてある。「動いた」ではなくコマンドと終了コードで判定する。DB を使う検査は `docker compose up -d db` が前提。

## Global Constraints

- **テストには `Scenario: <名前>` の印を置く。** Rust / Kotlin はコメント（`// Scenario: 撮影時刻が記録の出来事の時刻になる`）、bash は `echo`。
  `scripts/check_scenarios.py` が spec の全 Scenario と突き合わせ、印の無い Scenario を FAIL にする。印の名前は spec の `#### Scenario:` と**一字一句合わせる**
- **この change の Scenario は 183 本**（`device-collection` 127・`collection-coverage` 49・`data-durability` 7）。うち **ST09 で新しく足したのは 136 本**
  （`device-collection` 115・`collection-coverage` 14・`data-durability` 7）。**新しく足した 136 本は全部、このファイルのどこかの Task が名指しする。**
  残る 47 本は MODIFIED で写した既存分で、印は既存のテストのまま（名指しするのは WHEN を限った 3 本と、4 つのソースを数えに足す 1 本だけ。8.2 / 4.2）
- **印を置くテストは、その Scenario の THEN の階層を観測する**。THEN が「D-01 に格納」「D-02 に置かれる」なら、本物の PostgreSQL と本物の置き場（一時ディレクトリ）まで通す
  サーバの結合テストに置く。端末の振る舞い（読む・積む・送る）は Robolectric か計測テストに置く。端から端の Scenario は両方に印を置く（Task 10.2）
- **件数つき検証**: このファイルで
  **`CT <名前>`** は `bash -c 'set -a; [ -f .env ] && . ./.env; set +a; scripts/quiet-run ct -- cargo test -p ashiato-server <名前>'` rc=0 かつ
  `cargo test -p ashiato-server <名前> -- --list 2>/dev/null | grep -c ': test$'` が、その項目に書いた本数（`-- --list で N 本以上`）以上、
  **`GT <クラス>`** は `scripts/quiet-run unit -- ./collector-android/gradlew -p collector-android :app:testDebugUnitTest --tests '*<クラス>*'` rc=0 かつ
  `grep -ho 'tests="[0-9]*"' collector-android/app/build/test-results/testDebugUnitTest/TEST-*<クラス>*.xml | tr -dc '0-9\n' | awk '{s+=$1} END{print s+0}'` が
  その項目に書いた本数（`N 本以上`。書いていなければ 1）以上、
  **`AT <クラス>`** は `scripts/quiet-run instr -- tools/android-emulator.sh` rc=0 かつ
  `cat collector-android/app/build/outputs/androidTest-results-stage*/TEST-*.xml | grep -o '<testcase [^>]*classname="[^"]*\.<クラス>"' | wc -l` が同じく本数以上
  （結果のファイル名はクラス名を含まない（`TEST-<AVD>-_app-.xml`）ので、XML の中の `<testcase classname=…>` で数える。
  `tools/android-emulator.sh` が段ごとの XML を `androidTest-results-stage<N>` に残す。deep.md Q18（2026-10-21 本人の答え。review/code.md R14））
  が rc=0 になることを指す（`cargo test` に絞り込みを 2 つ渡すと `unexpected argument` で落ちる。1 つずつ書く。0 本でも rc=0 になるので `-- --list` の下限を必ず見る）
- **計測テスト（`AT`）だけで閉じる Task を作らない。** 手元の `/dev/kvm` が無いことがある（`scripts/verify-run` が `BLOCKED_INFRA` を返す）。
  `AT` を挙げた項目は、同じ振る舞いを `GT`（Robolectric の MediaStore と偽の読み手）でも確かめる。`AT` は CI の `android-instrumented` で必ず走る
- **試験の名前の接頭辞は Task ごとに重ならない**: `st09_mig_`（1）/ `st09_up_`（3.2・6）/ `st09_miss_`（7.1）/ `st09_sum_`（7.2）/ `st09_cov_`（8）/ `st09_exp_`（9）/ `st09_del_`（9）。
  Kotlin はクラスで分ける（`MediaRecordTest` / `MediaScannerTest` / `MediaCapabilityTest` / `OriginalUploaderTest` / `MediaPremisesInstrumentedTest` / `MediaCaptureInstrumentedTest`）
- **本人の決定（下流は変えない）**: Q3 全部取る / Q4 写しを作らない / Q5 端末の画像の全部 / **Q6 動画はメタだけ（静止画も送らない）** / Q7 変わった後の版も取る /
  Q8 消えた事実を残す / Q14 撮影時刻（無い・狂っていれば保存の時刻 + 印）/ Q15 位置の権限が戻るまで待つ（知らせる）/ **Q16 滞在を消しても写真は消えない** /
  Q17 写真を消す口と画面は作らない。B の Q9〜Q13 は design の（仮）に反転条件がある
- **本人の決定と仮決めの値は定数にして試験が名指しで固定する**（`docs/testing.md` §3）: `TAKEN_AT_FLOOR = 2000-01-02T00:00:00Z`・`TAKEN_AT_MAX_AHEAD = 24 時間`（Q14）・
  `BACKLOG_AGE = 24 時間`（D8）・`OBSERVER_DEBOUNCE = 10 秒`・`SCAN_BATCH = 200`（D1）・`FULL_RECONCILE_EVERY = 24 時間`（D1）・`OMIT_BINARY_OVER = 256 バイト`（D4）・`ORIGINAL_BODY_LIMIT = 1 GiB`（D10）・`NAME_MAX_CHARS = 120`（D10a）・`HASH_SUFFIX_CHARS = 8`（Q12）・
  `UPLOAD_RETRY = 1 分から倍・上限 1 時間`・`RECORD_MISSING_GIVE_UP = 7 日`（D9）・`RECORD_SIZE_GATE = 20 KiB`（D20）
- **D1 / D2 / D3 / D4 / D5 / D6 / D7 / D9 / D10 / D12 / D13 / D18 / D22 / D23 / D24 は（仮）決め。** 反転条件は `design.md` にある。変えたらその D 番号を書き直す
- **原本を未送信の置き場（`SegmentStore`）に入れない。** 原本の送り待ちの台帳（design D8）は 90 日・2 GB の `Retention` の対象にしない（Story の「壊してはいけないもの」）
- **写真の位置の権限が無い間は、写真・画像・動画のファイルを開かない**（Q15）。`setRequireOriginal` を使わない読み方を作らない
- **ログに位置・ファイル名・保存先のフォルダ・原文・内容ハッシュ・`media_id` を出さない**（端末と S-01）。`tools/check-log-private.sh` rc=0
- **縦串・書き換え禁止・偽データのシナリオは Story ごとの新しいファイルに置く**: `tools/smoke.d/<作成時刻>_st09-media-capture.sh`・
  `tools/check-immutable.d/<作成時刻>_st09-media-capture.sh`。共有の台本の末尾に書き足さない（`tools/check-shared-appends.sh`）
- 長いコマンド（`cargo test` / gradle / `tools/smoke.sh` / `tools/android-emulator.sh`）は repo の根から `scripts/quiet-run` で 1 回で待つ

## Task 1: 移行 —— 登録簿・収集開始日の印・生存信号の接続の欄・3 つの表（design D7 / D13 / D15 / D17）

- [x] 1.1 移行 `YYYYMMDDHHMM_media_capture.sql`（と `.down.sql`）を書き、`migrations/ORDER` の末尾に 1 行足す。中身は design D15 の 1〜4:
  `c01-screenshot` / `c01-image` / `c01-video` を `c01-photo` と同じ値で足す・4 つの `external_id_kind` を記録が 0 件のときだけ `record` に・
  `core.source.started_on_basis`（4 つを `heartbeat`）・`core.heartbeat.network boolean NULL`・`core.media_original`・`core.media_missing`・`core.text_extraction`
  （それぞれ書き換え禁止の門とアプリの役割の `SELECT` / `INSERT`）・4 つの `collection_started_on` の当て直し（D13。記録は変えない）。
  `.down.sql` は表と欄を落とす前に、表に行があれば落とさずに残す（ST19 の移行の前例）。
  Scenario: `4 つのソースが登録簿に 6 時間の間隔で並ぶ` / `メディアのソースは識別子を欠く記録を断る` / `文字抽出の版を入れる列がある` /
  `文字抽出の行は書き換えられない` / `原本の欠けは書き換えられない` / `汚れた写真の収集開始日は移行で直る`。
  検証: `CT st09_mig_`（`-- --list` で 6 本以上）/ `tools/check-migrations.sh` rc=0 / `scripts/quiet-run ct -- cargo test -p ashiato-server` rc=0（既存が全部緑）
- [x] 1.2 `tools/check-immutable.d/<作成時刻>_st09-media-capture.sh` を足し、`core.media_original` / `core.media_missing` / `core.text_extraction` の `UPDATE` と `DELETE` が
  アプリの役割で断られることを psql で 1 本ずつ確かめる（`echo "Scenario: 原本の欠けは書き換えられない"` / `echo "Scenario: 文字抽出の行は書き換えられない"`）。
  検証: `tools/check-immutable.sh` rc=0 / `tools/check-shared-appends.sh` rc=0

## Task 2: 端末の前提を本物の Android で確かめ、記録の形を作る（premises.md P1 / P3 / P12 / P13。design D4 / D20）

**この Task の 2.3 が落ちたら Task 3 以降に進まない**（記録の大きさの門。premises.md P13）。門は `GT MediaRecordSizeTest` だけで判定する（`AT` は追加の確かめ）。
**2.2 は「計測テストだけで閉じる Task を作らない」の例外**: 他のアプリの写真の位置が伏せられるかは本物の framework でしか確かめられない。
手元で `BLOCKED_INFRA`（`/dev/kvm` を開けない）のときは、CI の `android-instrumented` の結果（`MediaPremisesInstrumentedTest` の XML）を証跡にして閉じ、Task 3 以降は 2.3 の `GT` の門だけで進めてよい。

- [x] 2.1 記録の組み立て（`MediaRecord`。design D4）を純粋な関数で作る: MediaStore の行と EXIF（`ExifInterface`）・入れ物のメタ（`MediaMetadataRetriever`）の値の写像から
  `payload` と原文の文字列を組む。出来事の時刻（Q14）・地域（C7）・縮小画像とバイナリの塊の除き方・原文の上限（D4）・外部識別子（`sha256`）を含む。
  ハッシュはファイル全体を流して SHA-256（`MessageDigest`）。
  Scenario: `撮影時刻が記録の出来事の時刻になる` / `撮影時刻が無い画像は保存された時刻になる` / `2000 年より前の撮影時刻は採らない` /
  `保存より 1 日以上後の撮影時刻は採らない` / `3 つの時刻が記録に残る` / `撮影時刻の地域がメタに無ければ端末の地域を当てて印を付ける` /
  `位置の無い写真は端末の位置で埋めない` / `写真の記録が撮影位置と機種を持つ` / `動画の記録が長さと撮影位置を持つ` / `内容ハッシュはファイル全体の SHA-256` /
  `原文に EXIF の欄が端末の返した値のまま入る` / `原文に埋め込みの縮小画像が入らない` / `端末の一覧にだけ撮影時刻がある画像は保存された時刻になる` /
  `2000-01-02 ちょうどの撮影時刻は採り、その直前は採らない` / `撮影時刻が保存のちょうど 24 時間後なら保存された時刻になる` / `文字の欄は長くても原文から落ちない`。
  材料: `app/src/test/resources/media/` に、GPS・機種・`OffsetTimeOriginal`・縮小画像・`MakerNote` を持つ JPEG と、`OffsetTimeOriginal` の無い JPEG、
  位置つきの短い MP4 を置く（自作。ライセンスは `tools/check-licenses.sh` の対象外の自作物として README に書く）。
  検証: `GT MediaRecordTest`（16 本以上）
- [x] 2.2 計測テスト `MediaPremisesInstrumentedTest` を書き、エミュレータで前提を確かめる（確かさ 2 → 4）: (P1) `ACCESS_MEDIA_LOCATION` が無いと他のアプリの写真の
  EXIF の位置が伏せられ、`setRequireOriginal` が例外になる / (P3) 写真を 1 枚足すと `getGeneration` が上がり、`GENERATION_MODIFIED` で拾える /
  (P12) 位置つきの動画の入れ物の位置が `ACCESS_MEDIA_LOCATION` の有無で変わるかを記録する（どちらでも落とさず、結果をテストの出力に残す）。
  写真は `MediaStore` に別のアプリとして入れる（位置を伏せるのは他のアプリの写真だけ。`adb shell content insert` か `adb push` + メディアの再走査を、`tools/android-emulator.sh` と CI の `android-instrumented` の `script` の両方の段に足す）。
  検証: `AT MediaPremisesInstrumentedTest`。結果（P1 / P3 / P12 の観測）を `premises.md` の表の「結果」「確かさ」に書き戻す
- [x] 2.3 **記録の大きさの門**（D20 / P13）: 2.1 の材料と、2.2 で入れた本物のカメラの形の写真 1 枚から作った記録の未送信の 1 行の大きさを測り、
  `RECORD_SIZE_GATE`（20 KiB = 1 万件で 200 MB = 2 GB の 1 割）以下であることを試験で固定する。**超えたら止めて、design D4 の原文の上限を本人に返す**（deep の問いにする）。
  検証: `GT MediaRecordSizeTest`（1 本以上。これだけで門を判定する）/ `AT MediaPremisesInstrumentedTest`（追加）

## Task 3: 読み手と見分けの台帳と消えた事実（design D1 / D2 / D3 / D5。`collector-android`）

- [x] 3.1 読み手（`MediaScanner`）と見分けの台帳（`media/ledger-<置き場>.jsonl`・`media/cursor.json`）を作る。契機は 3 つ（知らせ・6 時間・1 日 1 回）、
  1 回 200 件ずつ、ハッシュは変わった項目だけ計算し直す、一覧の版が変わったら全部を計算し直してハッシュで結び直す（D2）。位置とは別のスレッドで走らせる。
  論理ソースは D3 の表で決め、最初に決めたもので固定する。
  Scenario: `導入した時点で端末にある過去の写真が取られる` / `収集を止めていた間に保存された写真も取られる` / `保存の知らせで 6 時間の見直しを待たずに取られる` /
  `知らせが届かなくても 6 時間の見直しで取られる` / `変わったかの印が上がらなくても 1 日 1 回の突き合わせで取られる` / `一覧が作り直されても記録は増えない` /
  `2 つの置き場の写真がどちらも取られる` / `同じ写真を 2 回見ても記録は増えない` / `カメラで撮った写真は写真のソースに入る` /
  `スクリーンショットはスクリーンショットのソースに入る` / `受け取った画像はカメラ以外の画像のソースに入る` / `動画は動画のソースに入る`。
  検証: `GT MediaScannerTest`（12 本以上。Robolectric の `ShadowContentResolver` に一覧を持たせ、時計は差し替える）
- [x] 3.2 見分け（C17）と変わった後の版（Q7）: 同じハッシュは 1 件、原文だけ変わったら同じ外部識別子で送り直す、中身が変わったら `previous_sha256` を持つ新しい記録。
  Scenario: `同じ中身の写真が 2 つ保存されても記録は 1 件` / `原文だけ変わった写真は 1 件のまま前の版が残る` / `中身が変わった写真は新しい記録になり前の記録も残る` / `消えた事実を作っても写真の記録は前の版にならない`。
  端末の側（送る記録の数と外部識別子）は `GT`、格納（1 件のまま・前の版が履歴に残る・前の記録が読み出しに残る）は `CT st09_up_ident_`（サーバの結合テストで同じ形の要求を送る）に印を置く。
  検証: `GT MediaScannerTest` / `CT st09_up_ident_`（`-- --list` で 4 本以上）
- [x] 3.3 消えた事実（D5）: 全部の突き合わせでだけ判定し、ごみ箱・外れた置き場・選んだ写真だけ・別の番号の同じハッシュを消えたと数えない。
  消えた事実の外部識別子は `vanished:<sha256>:<volume>:<media_id>`。
  Scenario: `端末から消えた写真は消えた事実が 1 件残る` / `消えた事実を 2 度見ても 1 件のまま` / `ごみ箱に入っただけでは消えたと数えない` /
  `ごみ箱から消えた時点で消えたと数える` / `外れた置き場の写真は消えたと数えない` / `選んだ写真だけの許可に変わっても消えたと数えない` / `動かした写真は消えたと数えない` /
  `写真と動画へのアクセスを外して戻しても消えたと数えない` / `一覧が作り直されて番号が振り直されても消えたと数えない` / `名前を変えた写真は消えたと数えない`。
  検証: `GT MediaVanishTest`（10 本以上）

## Task 4: 権限・取得可否・知らせ・4 本の生存信号（design D6 / D7 / D22。`collector-android`）

- [x] 4.1 `AndroidManifest.xml` に `READ_MEDIA_VIDEO`・`READ_MEDIA_VISUAL_USER_SELECTED`・`READ_EXTERNAL_STORAGE`（`maxSdkVersion="32"`）を足し、初回の起動で
  位置の後に写真と動画と写真の位置の権限を求める（拒まれても収集は始める）。
  Scenario: `初回の起動で写真と動画と写真の位置の権限を求める` / `写真の権限が無くても位置は集まる`。
  権限を求める画面の計測テストは「未許可・未要求」から始めるので `@NeedsPristinePermissions` を付ける（CI の 2 段目で走る）。
  検証: `GT MediaPermissionTest`（2 本以上。マニフェストの権限を読む試験を含む）/ `AT MediaCaptureInstrumentedTest`
- [x] 4.2 メディアの取得可否（`mediaCapability`。D6 の語彙）と、`SourceCadence` の 4 つ（6 時間・6 時間）と、`network` を載せる生存信号（D7）と、試行と成功の数え方（D22）。
  C19 の各段の失敗を数え、その区間のすべての試行が同じ段で失敗したらその段の語を載せる。写真の位置の権限が無い間は読み手がファイルを開かない（Q15）。
  Scenario: `写真の位置の権限が無ければ写真を 1 件も読まない` / `写真の位置の権限が無いと生存信号は取れない状態になる` / `権限が戻ると待っていた間の写真も取る` /
  `選んだ写真だけの許可では選んだものを取り、取れない状態と報告する` / `圏外でも写真の生存信号は取れる状態のまま接続が無かったことが載る` / `接続があれば接続ありが載る` /
  `ハッシュの計算に失敗し続けると取れない状態になる` / `記録を未送信に積めないと取れない状態になる` / `アプリ利用の取得可否は接続を含めたまま` /
  `メディアの 4 つのソースがそれぞれ生存信号を送る` / `メディアの生存信号の試行は見直しと知らせの回数で数える`。
  `HeartbeatCountersTest` の `どのソースも生存信号の区間に取得契機が 1 回以上入る` が 4 つのメディアのソースも数えるようにする（`SourceCadence` の全部を回す形なら、足すだけで入ることを確かめる）。
  検証: `GT MediaCapabilityTest`（11 本以上）/ `GT HeartbeatCountersTest` / 既存の `GT CollectionSourceTest` と `GT SourceIndependenceTest` が緑のまま
- [x] 4.3 知らせ（D6。チャンネル `media-capture`・文言は design の表のとおり・押すとアプリの設定）。権限が戻ったら消す。「選んだ写真だけ」では出さない。
  Scenario: `写真の位置の権限が無いと端末で知らせる` / `選んだ写真だけの許可では通知を出さない`。
  検証: `GT MediaNoticeTest`（3 本以上。出る・消える・選んだ写真だけでは出ない）/ `AT MediaCaptureInstrumentedTest`（通知が出て、その `PendingIntent` がアプリの設定を指す）

## Task 5: 原本の送り待ちと送り手（design D8 / D9。`collector-android`）

- [x] 5.1 原本の送り待ちの台帳（`media/originals.jsonl`）: 画像の記録を積んだら `awaiting_record`、送信の結果で受理（重複を含む）と分かったら `ready`。
  `Retention` の対象にしない。記録が未送信の上限で破棄されたら行を消して欠け（`record_dropped`）を、恒久的に断られたら欠け（`record_rejected`）を積む
  （D8 / D11 の報告。Task 7 の受け口に送る）。原本の欠けの報告は生存信号・破棄の報告と同じく上限で捨てない側に置く（R6）。
  Scenario: `原本は未送信の置き場の大きさを増やさない` / `原本の送り待ちは 90 日を過ぎても捨てない` / `記録が未送信から破棄されると原本も欠けになる` /
  `恒久的に断られた記録の原本は欠けになる` / `原本の欠けの報告は上限を超えても捨てられない`。
  検証: `GT OriginalLedgerTest`（5 本以上。`RetentionPolicy.overrideForTest` で 90 日と 2 GB を縮めて確かめる）/ 既存の `GT RetentionTest` が緑のまま
- [x] 5.2 送り手（`OriginalUploader`）: 送る前の問い合わせ（`HEAD`。受け取り済みなら読まずに行を消す）・2 回読み（1 回目でハッシュを確かめ、2 回目で流して `PUT`）・順（新しいものを先、過去ぶんは保存の新しい順）・通信（過去ぶんは
  `NET_CAPABILITY_NOT_METERED` のときだけ）・応答ごとの扱いと試し直しの間隔（D9）・動画は送らない。
  Scenario: `動画を撮るとメタだけが入り原本は送られない` / `原本は形式を変えずに送られる` / `送るときに中身が変わっていたら古い版のバイトとして送らない` /
  `新しく撮った写真の原本はモバイル通信でも送られる` / `過去ぶんの原本はモバイル通信では送られない` / `過去ぶんの原本は Wi-Fi になると送られる` /
  `新しく撮った写真の原本は過去ぶんより先に送られる` / `D-02 に書けなければ送り直せる失敗になり、端末に送り待ちが残る` / `送る前に端末から消えた写真の原本は欠けとして残る` /
  `受け取り済みの原本は送り直さない` / `大きすぎて受け取られない原本は欠けになる` / `保存から 24 時間たっていなければ過去ぶんにしない` /
  `撮影が古くても保存が新しい画像は新しいものとして送る` / `過去ぶんどうしは保存の新しいものから送られる`。
  検証: `GT OriginalUploaderTest`（14 本以上。偽の `Transport` に届いたバイトを端末のファイルと突き合わせる）
- [x] 5.3 端末のログ（D21）: 原本の送信の失敗で `Telemetry.line` に種別と件数だけが出る。
  Scenario: `原本の送信の失敗がログに出てもファイル名と位置は出ない`（端末の側。S-01 の側は 6.3）。
  検証: `GT OriginalUploaderTest` / `tools/check-log-private.sh` rc=0

## Task 6: S-01 の原本の受け口と D-02（design D10 / D10a。`crates/server/src/media.rs`）

- [x] 6.1 置き場の設定（`ASHIATO_MEDIA_DIR` → `LOCALAPPDATA\ashiato\media` → `XDG_DATA_HOME/ashiato/media` → `HOME/.local/share/ashiato/media`。書き出しの `export_dir` と同じ並び）。
  相対の場所なら起動しない。既定の置き場だけは起動時に作り、設定された置き場は作らない。起動時に `.incoming/` の `.part` を消す。
  Scenario: `置き場の設定が無ければ既定の置き場になる` / `置き場を設定で変えられる` / `相対の置き場では起動しない` /
  `設定した置き場が無ければ作らずに書けない失敗にする` / `起動すると書きかけの一時のファイルが消える`。
  検証: `CT st09_up_dir_`（`-- --list` で 5 本以上）
- [x] 6.2 `PUT /media/originals/{sha256}`（この経路だけ `DefaultBodyLimit` 1 GiB。超えたら `413`）と `HEAD /media/originals/{sha256}`: 流しながら `.incoming/<uuid>.part` に書いてハッシュを計算し、D10 の順で判定して
  `fsync` → 名前を変える → `core.media_original` に 1 行。並びと名前（D10 / D10a）。資格情報は `/ingest` と同じ。
  Scenario: `ハッシュの合う原本は受け取られる` / `ハッシュの合わない原本は置かれない` / `記録より先に届いた原本は送り直せる失敗になる` / `動画の原本は受け取られない` /
  `同じ原本が 2 度届いても 1 つだけ置かれる` / `書いている途中で止まっても壊れたファイルは並びに残らない` / `D-02 に書けなければ送り直せる失敗になり、端末に送り待ちが残る` /
  `資格情報の無い原本の要求は断られる` / `D-01 は原本のファイルの場所を持たない` / `原本は撮影の年/月のフォルダに撮影日時と元の名前で置かれる` /
  `同じ名前の別の中身にはハッシュの頭が足される` / `元の名前に区切りがあっても置き場の外に書かない` / `受け取り済みかを問い合わせられる` /
  `1 GiB を超える原本は送り直せない失敗になる` / `中身が変わった写真は新旧どちらの原本も D-02 に残る`。
  置き場はテストごとの一時ディレクトリ。「書けない」は置き場を消した状態で作る。「途中で止まる」は本文の流れを途中で切る。
  検証: `CT st09_up_put_`（`-- --list` で 15 本以上）/ `cargo clippy -p ashiato-server --all-targets -- -D warnings` rc=0
- [x] 6.3 S-01 のログ（D21）: 受け口の失敗で種別だけを出す。`tools/check-log-private.sh` の対象に `media.rs` を入れる。
  Scenario: `原本の送信の失敗がログに出てもファイル名と位置は出ない`（S-01 の側）。
  検証: `CT st09_up_log_`（1 本以上）/ `tools/check-log-private.sh` rc=0

## Task 7: 原本の欠けの受け口と量の読み出し（design D11 / D12）

- [x] 7.1 `POST /media/missing`（`/drops` と同じ形。1 件ごとの結果）と `core.media_missing`。冪等キーは `id`。破棄の報告と稼働状況の破棄の件数に載せない。
  Scenario: `送る前に端末から消えた写真の原本は欠けとして残る`（S-01 の側: 欠けが理由とハッシュつきで 1 件）/ `原本の欠けは破棄の報告に載らない` /
  `原本の欠けの報告を送り直しても 1 件のまま` / `資格情報の無い原本の欠けの報告は断られる`。
  検証: `CT st09_miss_`（`-- --list` で 4 本以上）
- [x] 7.2 `GET /media/summary`（D12）。記録の値を載せない。`docs/openapi.json` を `src/bin/openapi.rs` で再生成する（`PUT /media/originals/{sha256}`・`POST /media/missing`・
  `/heartbeat` の `network` も入る）。
  Scenario: `記録の件数とバイトがソースごとに読める` / `置いた原本と待っている原本の件数が読める` / `原本の欠けが理由ごとに読める` / `量の読み出しに記録の値が出ない` /
  `書けなかった失敗は起動し直すまで読み出しに出る` / `資格情報の無い量の読み出しは断られる`。
  検証: `CT st09_sum_`（`-- --list` で 6 本以上）/ `tools/check-openapi.sh` rc=0（生成物と一致）

## Task 8: 生存信号の接続の欄・収集開始日・状態の判定（design D7 / D13 / D14 / D24。`heartbeat.rs` / `coverage.rs`）

- [x] 8.1 `/heartbeat` の任意の `network` を受けて `core.heartbeat.network` に残す（無ければ `NULL`）。
  Scenario: `圏外の写真の生存信号は取れる状態のまま接続が無かったことが残る` / `接続の有無の欄を持たない生存信号も受け付ける`。
  検証: `CT st09_cov_net_`（`-- --list` で 2 本以上）/ 既存の `cargo test -p ashiato-server heartbeat` が緑のまま
- [x] 8.2 `touch_started_on` と収集開始日の引き直しが `started_on_basis = heartbeat` のソースで記録を数えないようにする（D13）。既存の Scenario
  `後から古い記録が届くと開始日が遡る` / `登録より前の時刻の記録は開始日を動かす` / `汚れた収集開始日は引き直せる` の印の先（`api_tests.rs` ほか）が
  メディアでないソースを使っていることを確かめる（使っていれば直す）。
  Scenario: `撮影日が 5 年前の写真が届いても写真の収集開始日は動かない` / `写真の収集開始日は最初の生存信号の日になる` /
  `生存信号の無い写真のソースは記録があっても開始していない` / `入れ直した後の生存信号では写真の収集開始日は後ろへ動かない` /
  `スクリーンショットと画像と動画の収集開始日も最初の生存信号の日になる`（`汚れた写真の収集開始日は移行で直る` は 1.1）。
  検証: `CT st09_cov_start_`（`-- --list` で 5 本以上）/ `CT started_on`（既存が緑のまま）
- [x] 8.3 状態の判定順の (1) の例外（D14）と、達成と途絶が変わらないこと。成功条件 1 の 5 ソースの写真が `c01-photo` だけであること（D24）。
  Scenario: `見始めた日より前の撮影日の写真がある日は記録ありになる` / `見始めた日より前で写真の無い日は導入前のまま` /
  `見始めた日より前の記録ありの日は達成の分母に入らない` / `成功条件 1 の写真はカメラの写真だけを数える` /
  `生存信号の無い写真のソースでも記録のある日は記録あり` / `消えた事実の記録がある日は記録ありになる` / `見始めた日より前の写真は途絶の判定に使われない`。
  検証: `CT st09_cov_state_`（`-- --list` で 7 本以上）/ `scripts/quiet-run ct -- cargo test -p ashiato-server coverage` rc=0（既存が全部緑）

## Task 9: 書き出しの分類と、滞在を消したときの写真（design D16 / D18 / D23）

- [x] 9.1 `export/excluded.rs` の `SPAN_RULES` と `COORD_SOURCES` に 3 つを足し、`CATALOG` に 3 つの表（入れない）と `core.heartbeat.network`（出す）を足し、
  説明書の「入らないもの」に表の名前を挙げる。
  Scenario: `消した滞在の時間に撮った撮影位置つきのスクリーンショットと画像は書き出しに出ない` / `消した時間の前に撮り始め、長さで重なる動画は書き出しに出ない` /
  `消した時間の後に撮った写真は書き出しに出る` / `生存信号の接続の有無は書き出しに出る` / `原本の台帳と原本の欠けと文字抽出の表は書き出しに入らない` /
  `ST09 が足したソースと表と列はすべて分類されている` / `説明書の入らないものに原本の台帳と欠けと文字抽出が挙がる`。書き出しのテストの規律は ST33 の tasks のとおり（`EXPORT_SERIAL` を取る・`unique_day()` の日に置く・その利用者の行だけを見る）。
  検証: `CT st09_exp_`（`-- --list` で 7 本以上）/ `scripts/quiet-run ct -- cargo test -p ashiato-server export` rc=0（既存の `いまの記録の schema の表と列はすべて分類されている` を含む）
- [x] 9.2 滞在を消しても写真・動画・スクリーンショットの記録に印が付かず、D-02 の原本が残ることを固定する（`deletion.rs` は変えない）。
  Scenario: `滞在を消しても撮影位置つきの写真は読み出しに出る` / `滞在を消しても動画とスクリーンショットの記録は読み出しに出る` / `滞在を消しても D-02 の原本は残る`。
  検証: `CT st09_del_`（`-- --list` で 3 本以上）/ `git diff --quiet origin/main -- crates/server/src/deletion.rs` rc=0

## Task 10: 契約・端から端・縦串（`docs/collector-contract.md` / `tools/smoke.d/`）

- [x] 10.1 `docs/collector-contract.md` に、C-01 の写真・画像・動画の `payload`（design D4 の表）・消えた事実・外部識別子・原本の受け口（D10 の応答の一覧）・
  原本の欠け（D11）・生存信号の `network` とメディアの `blockers` の語彙（D6 / D7）を書く。取得可否の表（`capturable` / `blockers` の行）にメディアの例外を足す。
  検証: `grep -c "c01-video" docs/collector-contract.md` が 1 以上 / `grep -c "/media/originals" docs/collector-contract.md` が 1 以上 /
  `grep -c "media_location" docs/collector-contract.md` が 1 以上 / `python3 scripts/check_chain.py .` rc=0
- [x] 10.2 端から端: 計測テスト `MediaCaptureInstrumentedTest` が、エミュレータで写真を 1 枚と動画を 1 本 MediaStore に入れ、テストの中の HTTP の受け手（`MockWebServer`）に
  （端末の中のテスト用の受け手。CI と同じく `-Pashiato.baseUrl=http://127.0.0.1:18787`）に、写真の記録・動画の記録・写真の原本の `PUT`（バイトが端末のファイルと一致）が届き、動画の原本の `PUT` が届かないことを確かめる。サーバの側は
  `tools/smoke.d/<作成時刻>_st09-media-capture.sh` が、本物のサーバに写真と動画の記録を `/ingest` で送り、写真の原本を `PUT` し、D-01 の行と D-02 のファイル（バイト一致）と、
  動画の原本の `PUT` が断られ D-02 に何も無いことを確かめる。
  Scenario: `写真を撮るとメタが D-01 に、原本が D-02 に入る` / `動画を撮るとメタだけが入り原本は送られない`（両方に印）。
  検証: `AT MediaCaptureInstrumentedTest` / `GT OriginalUploaderTest`（同じ流れを Robolectric の偽の受け手で）/ `scripts/quiet-run smoke -- tools/smoke.sh` rc=0

## Task 11: 仕上げ

- [x] 11.1 Scenario の印の突き合わせ・鎖・私的データの検査を通す。
  検証: `python3 scripts/check_scenarios.py .` rc=0 / `python3 scripts/check_chain.py .` rc=0 / `tools/check-log-private.sh` rc=0 / `tools/check-private.sh` rc=0 /
  `scripts/quiet-run unit -- ./collector-android/gradlew -p collector-android detekt :app:assembleDebug :app:testDebugUnitTest` rc=0 /
  `scripts/quiet-run ct -- cargo test -p ashiato-server` rc=0 / `openspec validate st09-media-capture --strict` rc=0
- [x] 11.2 `premises.md` の P1 / P3 / P12 / P13 の確かさを 2.2 / 2.3 の結果で書き直す（種類の割当て（design D19）は上流で `docs/handoff/ST38.md` に送り済み。変えない）。
  検証: `for s in c01-photo c01-screenshot c01-image c01-video; do grep -q "$s" docs/handoff/ST38.md || exit 1; done` rc=0 / `grep -cE "^\| P(1|3|12|13) .*\| [45] \|$" openspec/changes/st09-media-capture/premises.md` が 4

## 人間の確認待ち

本物のカメラで撮った写真（HEIC・本物の EXIF・本物の位置）と本人の端末の一覧は、エミュレータの材料で再現できない。Story の完了の判定そのもの。

- Scenario: 写真を撮るとメタが D-01 に、原本が D-02 に入る
  > 物理: device —— 本人の端末のカメラで 1 枚撮り、自宅 PC の D-02 にその写真のファイルがあり、撮影位置と撮影時刻が記録に入っていることを見る
- Scenario: 動画を撮るとメタだけが入り原本は送られない
  > 物理: device —— 本人の端末で動画を 1 本撮り、記録に長さと撮影位置が入り、D-02 に何も増えないことを見る
