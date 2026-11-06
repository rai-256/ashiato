# ST54 の前提

この Story の決定が依る、外の振る舞い（Android の識別子・Windows のスタートアップと通知の差出人・GitHub のタグ）と、
既存のコードの振る舞いを表にする。`loss` のある問い（Q4・Q6・Q9）の失うものは、確かさ 4 以上の前提（Q4 は P6、Q9 は P15）に載せる。P7・P16 は Q4 の補足で、失うものの根拠にしない。

| # | 前提 | 何が依るか | 確かめ方 | 結果 | 出所 | 確かさ |
|---|---|---|---|---|---|---|
| P1 | Android はアプリ識別子が違えば別のアプリとして入れ、端末内の記録の置き場（アプリのデータ）・権限・通知の設定・Health Connect の許可を別に持つ | 完了の判定 1（正式版の記録が残る） | 公式文書（applicationId: https://developer.android.com/build/configure-app-module ）。組み立ては `applicationIdSuffix = ".dev"` を `-dev.` の版番号のときだけ付ける（`collector-android/app/build.gradle.kts:95`） | 有り | 公式文書・既存のコード | 2 |
| P2 | 携帯の収集アプリは、識別子に依らない共有の名前（content provider の authority・独自の permission・sharedUserId）を宣言していない。宣言していれば、識別子が違っても 2 つ目が入らない | 完了の判定 1 | `grep -n "authorit\|<permission\|sharedUserId" collector-android/app/src/main/AndroidManifest.xml` が 0 件。ライブラリが足す provider（androidx.startup など）は `${applicationId}` から名前を作る | 0 件（ライブラリの側は未確認） | 2 つの APK を同じ端末へ入れ、正式版の `files/outbox/records/*.jsonl` の件数が維持される `tools/android-side-by-side.sh` | 4（APK の並列インストールと正式版の未送信区切りファイル維持を実機で確認） |
| P3 | 携帯の収集アプリの名前（ランチャーの表示）は正式版と開発版で同じ「あしあと。」 | Q7（見分け方） | `AndroidManifest.xml:73` の `android:label` | 同じ | 既存のコード | 3 |
| P4 | PC の収集アプリの自動起動はスタートアップの置き場にファイル 1 つ（`ashiato-collector.cmd`）、通知の差出人は登録簿の `dev.ashiato.collector`（表示名 `ashiato`）で、どちらも名前が固定。同じ PC で 2 つ目を入れると上書きし合う | Q1 | `crates/collector-windows/src/autostart.rs:17`・`toast.rs:17`・`toast.rs:30-38` | 固定 | 既存のコード | 3 |
| P5 | PC の収集アプリの置き場（未送信・印・除外の登録）と端末識別子は、環境変数 `ASHIATO_STATE_DIR`・`ASHIATO_DEVICE_ID` で本人が渡し、既定を持たない。同じ値を渡せば 2 つの収集アプリが同じ未送信を読み書きする | Q2 | `crates/collector-windows/src/config.rs:65・104-127`（既定で埋めない） | 既定なし | 既存のコード | 3 |
| P6 | サーバは、DB の移行の記録に自分の知らない版があれば Ahead として起動も移行も止める（終了コード 3）。名前を変えた・消した移行が main に入ると、それを当てた開発版の DB は次の開発版のサーバで止まる | Q4 | `cargo test -p ashiato-server --lib st48_compare` rc=0（`st48_compare_db_only_name_is_ahead` ほか 5 本。2026-11-02）・`crates/server/src/migration_ledger.rs:296-` | 止まる | 走らせた | 4 |
| P7 | 中身だけを変えた移行は警告だけで当て直さない。開発版の DB の形は黙って main とずれる | Q4 | `crates/server/src/migration_ledger.rs:518`（`warn_changed_fingerprints`）・ST52 の review/deep.md R11 | 警告だけ | 既存のコード | 3 |
| P8 | main で移行の名前を変えた・消したことは過去にある（2026-09-12 の連番から時刻の名前への改名、2026-09-29 の削除）。中身の変更は 42 commit | Q4 | `git log --diff-filter=RD -- 'migrations/*.sql'`・`--diff-filter=M` | 名前の変更・削除 2 回。main に入った後の中身の直し 9 回（`git log --first-parent --diff-filter=M main`。うち 4 回は 10-27〜10-31）。枝を含めると 42 commit | 実物（この repo の履歴） | 5 |
| P9 | 配った移行の凍結の検査は印 `v*` だけを数え、開発版の印 `nightly` を数えない | Q4 の選択肢 2 | `tools/check-released-migrations.sh:26`（`git tag --list 'v[0-9]*'`）・正典 distribution:197 | 数えない（`tools/check-released-migrations.sh --self-test` rc=0。2026-11-02） | 走らせた | 4 |
| P10 | 印 `nightly` を動かす方式では、古い `nightly` を持つ別の clone の `git fetch --tags` が rc=1 になる（`git fetch` は rc=0） | Q5 | ST52 の review/deep.md R11 の実測（git 2.53.0、使い捨ての repo） | rc=1 | 実物 | 5 |
| P11 | 開発版のリリースの説明文は、最初の公開で 1 回だけ書かれ（`gh release create --notes`）、その後の公開では書き直さない。いまの文は「開発版（main の毎日の組み立て）。携帯の APK は ST54 まで含まない」で、「サポートしない」が無い | Q8 | `.github/workflows/nightly.yml` の publish の job | 書き直さない | 既存のコード | 3 |
| P12 | サーバの既定の待ち受けは `127.0.0.1:18787` で、正式版と開発版で同じ。同じ機械で 2 つを既定のまま起こすと 2 つ目が待ち受けられない | C2 | `crates/server/src/net_guard.rs:31`・`lib.rs:4388` | 同じ | 既存のコード | 3 |
| P13 | サーバの DB の繋ぎ先は `DATABASE_URL` で本人が渡し、既定を持たない（無ければ起動しない） | Q3 | `crates/server/src/lib.rs:4365` | 既定なし | 既存のコード | 3 |
| P14 | 配布物の APK（正式版も開発版も）は接続先と資格情報を持たず、設定の口ができる ST56 まで送らない（取得は続けて未送信に積む） | 完了の判定 1 の読み方 | ST52 の review/deep.md R1・`tools/release/build.sh:150-151`（BuildConfig が空であることの検査） | 送らない | 既存のコード | 3 |
| P15 | 鍵を作る台本は RSA 4096・有効期限 10000 日の鍵だけを作り、満たさない鍵を落とす（開発版も同じ台本。`make-key.sh dev`） | Q9 | `tools/release/make-key.sh --self-test` rc=0（2026-11-02）・`make-key.sh:37` | 作る | 走らせた | 4 |
| P16 | 開発版の収集アプリが開発版のサーバにだけ送り、正式版も並べて動いていれば、開発版の DB の記録の多くは正式版の DB にもある。開発版にしか無い種類の記録は除く。いまは配布物の APK は送らない（P14） | Q4 の補足 | P1・P5 と Q6 の答えからの導出 | 条件つき | 導出 | 2 |
| P17 | ブラウザのクッキーはポートで分かれない。同じ機械で 2 つのサーバの画面を開くと、同じ名前のクッキー `ashiato_session` を上書きし合う | C7 | RFC 6265 §8.5（https://www.rfc-editor.org/rfc/rfc6265#section-8.5） | 分かれない | 公式文書 | 2（下流の e2e で 2 つを開いて 4 に上げる） |

## 他の Story へ回すもの

- なし（ST56 の設定の口ができた後も、開発版と正式版は別のアプリなので設定も別に持つ。ST56 が決める）

## 聞かずに決めるもの（扉を開けたままにする既定。spec の段で design の D に置く）

- C1. 携帯の開発版は識別子だけを変える（ST52 の `applicationIdSuffix`）。データの置き場・通知の設定・権限は Android が識別子ごとに分ける（P1）。下流の計測テストで 2 つを並べて入れ、正式版の記録が残ることを確かめる（P2 を 4 に上げる）
- C2. 開発版のサーバの既定の待ち受けは、正式版（`127.0.0.1:18787`）と別のポートにする（P12。被らない割り当て）
- C3. 開発版のリリースは携帯の APK を含めて 4 つ（`ashiato-collector-dev-<版番号>.apk`）。配布物の検査の「ST54 まで」の例外と、正典 distribution の例外の文・Scenario を外す（ST52 の申し送り）
- C4. 開発版の署名は `nightly` の変数の指紋を `ASHIATO_EXPECT_DEV_CERT_SHA256` で署名の台本に渡す（ST52 code_verify R26 の申し送り）
- C5. 開発版の鍵が無ければ、開発版の作り直しを落とし前の開発版を残す。落ちた理由に「開発版の鍵が無い」を出す（正典 distribution の「配布物が 1 つでも組めなければ公開しない（開発版は前の版のリリースを残す）」と FR-104 から導く。review/deep.md R9）
- C6. 開発版の PC の収集アプリの実行ファイル名に `-dev` を付ける。開発版のサーバの移行の前の退避の既定の置き場（`MIGRATE_BACKUP_DIR`）を正式版と分ける（review/deep.md R10）
- C7. 開発版のサーバの画面のログインのクッキー名を正式版と分ける（P17。review/deep.md R10）
- C8. 開発版の DB の器とポートは、`tools/ports.sh` の割り当ての範囲と被らない値にする（review/deep.md R10）
