# ST58 の前提（外の形式・振る舞い）

問いを立てる前に確かめたもの。確かさは story-upstream の 1〜5（1 言っただけ / 2 出所を示した / 3 辿った / 4 走らせた / 5 実物で再現した）。
2026-11-15 に確かめた。

**失われるもの（`loss`）のある問いは Q1（PC の入れ方: `uncaptured`）・Q5（途中で止まる: `discarded`）・Q6（「戻した」の形: `rewrite-all`）・Q7（退避の保持: `discarded`）。**
Q1 の推奨の手順が依る P4 は確かさ 1。外れても止まる時間（数秒）は変わらず、手順が「名前を変えて退ける」から「止めてから置き換える」に変わるだけ（下流の Windows の実行時の試験で 4 に上げる）。
Q5〜Q7 は repo の実物（確かさ 3）に載る。携帯の更新の後の再開（仮の決定。defaults.md P1）が依る P6・P7 は確かさ 2 で、エミュレータで 4 に上げることを tasks に置く。

| # | 前提 | 何が依るか | 確かめ方 | 結果 | 出所 | 確かさ |
|---|---|---|---|---|---|---|
| P1 | 配布元のリポジトリ `rai-256/ashiato` には、まだリリースが 1 つも無い | 実物の配布物で入れ替えを確かめられるか（確かめは試験の配布元で行う） | 認証なしの `curl https://api.github.com/repos/rai-256/ashiato/releases?per_page=5`（`User-Agent: ashiato-update-check`）・`…/releases/tags/nightly` | 一覧は `[]`、`nightly` は 404 | 実物 | 5 |
| P2 | リリースの一覧の各配布物は `name`・`browser_download_url`・`digest`（`sha256:<64 桁>`）を持つ | 落とした配布物の中身を、版の一覧の答え（api.github.com から TLS で読んだもの）の指紋と照らせる | 公開のリポジトリ `cli/cli` に認証なしで `releases?per_page=1` | 3 つの欄がある。`digest` は `sha256:180df77f…` の形 | 実物（別のリポジトリ） | 4 |
| P3 | 配布物の `browser_download_url`（`github.com/<owner>/<repo>/releases/download/…`）は、`release-assets.githubusercontent.com` への 302 で中身を返す | 収集アプリが自分で落とすなら、宛先が `api.github.com` の外に 2 つ増える（ST57 の問い合わせの規則は「同じ api.github.com の中の転送だけを追う」） | P2 の配布物に `curl -sI` | `HTTP/2 302`・`location: https://release-assets.githubusercontent.com/github-production-release-asset/…?…sig=…`（期限つきの署名つき URL） | 実物（別のリポジトリ） | 5 |
| P4 | Windows は動いている実行ファイルを上書きできないが、名前を変えることはできる | PC の収集アプリが自分を入れ替える手順（動いたまま古い実行ファイルを退け、新しいものを置き、起こし直す） | 知識（Windows の image section の共有の規則） | — | 知識 | 1 |
| P5 | Android は、入っているアプリより小さい `versionCode` の APK での更新を断る（`INSTALL_FAILED_VERSION_DOWNGRADE`）。署名の鍵が違う APK での更新も断る | 携帯の「古い版へ入れ直す」をアプリの側で止める要があるか | 公式文書 https://developer.android.com/reference/android/content/pm/PackageInstaller （`STATUS_FAILURE_CONFLICT` / version downgrade）・`versionCode` の正典（「携帯の versionCode は版番号から計算する」） | 断る（adb の `-d` を除く） | 公式文書 | 2 |
| P6 | Android は更新の後、更新されたアプリに `ACTION_MY_PACKAGE_REPLACED` を送り、アプリが止まった状態（stopped）にならない | Q1: 更新の後に収集を自分で起き直させられるか | 公式文書 https://developer.android.com/reference/android/content/Intent#ACTION_MY_PACKAGE_REPLACED | 送る | 公式文書 | 2 |
| P7 | `ACTION_MY_PACKAGE_REPLACED` を受けた時は、裏から前景のサービスを始めてよい例外に当たる。位置の型の前景のサービスは、裏の位置の権限（`ACCESS_BACKGROUND_LOCATION`。マニフェストで宣言しているが、本人が断っても収集は始まる「あるとよい」権限。`MainActivity.kt:172-176`・`:409-411`）を許された端末でだけ、裏から始めて位置を読める | Q1 の推奨（受け手から `LocationService` を起こす）が targetSdk 36 で成り立つか | 公式文書 https://developer.android.com/develop/background-work/services/fgs/restrictions-bg-start#background-start-restriction-exemptions ・`collector-android/app/src/main/AndroidManifest.xml:8`・`:117` | 例外に当たる（文書の上） | 公式文書・実物（repo） | 2 |
| P8 | 携帯の収集アプリは、いま `LocationService` を画面からしか起こさない（起動の後の受け手は気分の通知だけ） | Q1（更新の後と同じく、端末の起動の後も開くまで止まっている） | `AndroidManifest.xml:129-135`（`BOOT_COMPLETED` の受け手は `FeelingReminderBootReceiver` だけ）・申し送り st51 R10（`MainActivity.kt:183`） | 画面からだけ | 実物（repo） | 2 |
| P9 | Android 8 以降、アプリから APK を入れる画面を開くには `REQUEST_INSTALL_PACKAGES` の宣言と、本人がそのアプリに「提供元不明のアプリ」を許すことが要る | Q3（携帯の入れ方） | 公式文書 https://developer.android.com/reference/android/Manifest.permission#REQUEST_INSTALL_PACKAGES ・ FR-114 の本文 | 要る | 公式文書 | 2 |
| P10 | サーバのインストーラを新しい版で走らせ直しても、いまは更新にならない。プログラムの置き場に別の版のファイルがあると、写しの段で何も替えずに止まる（移行にもサーバの起動にも届かない）。移行も、完了の印があれば飛ばす | サーバの更新の手順をこの Story が新しく作る（今は更新の道そのものが無い） | `crates/installer/src/main.rs:840-861`・`:883-904`（`program_file_plan` が `Differs` で止まる）・`crates/installer/src/commands.rs:245-248`（「版を替える口はまだ無い」）・`main.rs:1303`（`if !migration_marker.exists()`）。review/deep.md R2 で直した | 止まる | 実物（repo） | 3 |
| P11 | インストーラは今、前の版の実行ファイルを退けて残す手順を持たない（上書きもしない） | 「前の版のサーバで起動し直す」（FR-119）には、置き場を替える前に前の版を退けておく要がある | `crates/installer/src/main.rs:829-935`（`copy_program_files` は無いファイルだけを写す）。review/deep.md R2 | 持たない | 実物（repo） | 3 |
| P12 | 移行の前の退避は、表の所有者で取った `pg_dump --format=custom` の 1 ファイル（`backups\pre-migrate\pre-migrate-<時刻>.dump`） | 戻しは `pg_restore` で、表の所有者の持つものだけが戻る（役割と権限の付与は DB の外の役割に依る） | `crates/server/src/migrate_backup.rs:102-150` | custom 形式・時刻つきの名前 | 実物（repo） | 3 |
| P13 | 移行は版ごとに版の記録と一緒に当たり一緒に戻るので、途中の版で落ちると、それより前の版は当たったまま残る | rc=1 のとき DB は「前の版のサーバが知らない版が当たった」状態になりうる（前の版のサーバは `db_ahead` で起動しない）—— 戻さないと前の版でも動かない | 正典 distribution「これから足す移行は、版の記録と一緒に当たり、一緒に戻る」 | 残る | 実物（repo の正典） | 2 |
| P14 | 配るサーバの Windows の配布物は zip で、中のインストーラ `ashiato-setup.exe` を本人が走らせる（管理者の確認が 1 回出る） | Q4（サーバの更新の手順） | 正典 distribution「サーバのインストールは、管理者の確認を 1 回だけ出し…」・「印（タグ）の名前で…4 つの配布物」 | zip | 実物（repo の正典） | 2 |
| P15 | PC の収集アプリの配布物は zip の中の実行ファイル 1 つで、入れる手順（インストーラ）が無い。本人が置いた場所から自動起動の台本を仕込む（`--install-autostart`） | Q2（FR-114 の「Windows のインストーラ」に当たるものがまだ無い） | `tools/release/build.sh:203-230`・`crates/collector-windows/src/autostart.rs:108-120`・`docs/collector-setup.md` | 無い | 実物（repo） | 3 |
| P16 | 時刻サーバの検査は、PC の子プロセスを `w32tm`・`wevtutil`・自分自身（`current_exe()`）の 3 つの形に限る | Q2（入れ替えた新しい実行ファイルを起こすのは「自分自身」ではない） | `tools/check-no-time-server.sh:7-10`・`:46` | 3 つだけ | 実物（repo） | 3 |
| P17 | 更新の履歴の表は `via` に `migrate`・`startup`、`outcome` に `attempted`・`applied`・`nothing_to_apply`・`failed`・`backup_failed` だけを受け付ける（CHECK）。追記のみの門を持つ | Q6（「戻した」を足すには CHECK を作り替える。ST58 の移行を持たない前の版の DB には書けない） | `migrations/202610300900_update_history.sql:10`・`:13`・門の関数 | 受け付けない | 実物（repo） | 3 |
| P18 | 移行は、退避の前に止まる回でも終了コード 1 を返しうる（錠が取れない・DB に繋げない・退避の失敗）。退避のファイルの名前は呼び手に返さず、壁時計の UTC の秒で付く | 戻す退避の特定（いちばん新しいファイルを選ぶと、前の回の退避を戻しうる） | `crates/server/src/migration_ledger.rs:528-547`・`:110-116`・`crates/server/src/lib.rs:321-326`・`:400-411`・`migrate_backup.rs:113`。review/deep.md R1 | 返さない | 実物（repo） | 3 |
| P19 | サーバと DB のサービスは開始の種類「自動」で、異常に終われば 10 秒後に起こし直す（3 回） | Q5（更新や戻しの途中で止まると、PC の起動のときにサーバが半分の DB で起きうる） | `crates/installer/src/commands.rs:958-976`・正典 distribution「サーバのインストールは…自動で起きるサービスにする」 | 起こし直す | 実物（repo） | 3 |
