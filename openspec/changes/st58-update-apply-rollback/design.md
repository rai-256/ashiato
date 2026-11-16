# ST58 design — 更新を入れ、移行が失敗したら自動で戻る

## Context

動機は `proposal.md` の Why。振る舞いの正本は `specs/distribution/spec.md`（ADDED 14・MODIFIED 4）と `specs/data-durability/spec.md`（MODIFIED 1）。
いまの形（premises の実物）:

- サーバのインストーラ（`crates/installer`）は、プログラムの置き場に別の版のファイルがあれば写しの段で止まる（P10）。前の版を退ける手順を持たない（P11）。走らせ直しは移行の前にサーバを止めない（review/deep.md R10）
- `ashiato-server migrate` は終了コード 0・1・2・3・4 を返す。1 でも退避が無い回がある。退避のファイルの名前は呼び手に返さず、壁時計の UTC で付く（P18）。退避の置き場は `MIGRATE_BACKUP_DIR` で渡せる（`migrate_backup.rs`・`commands::migrate_command`）
- サービスは「自動」で、落ちれば 10 秒後に起こし直す（P19）
- 更新係（ST57）は `C:\ProgramData\ashiato-updater` に結果を書き、サーバの `GET /update-status` が読む
- PC と携帯の収集アプリの更新の確認は、見つけた最新の版の番号だけを持つ（P21）。PC の通知は押されると窓を開ける（P22）。PC は 2 つ同時に動くのを止めない（P23）
- 表の所有者 `ashiato_owner` は `CREATEDB` を持ち、DB の持ち主である（P20）

## Goals / Non-Goals

**Goals**: 3 つの部品で「押すと入れる口」を開く。サーバの更新を、移行が落ちても記録を失わずに前の版へ戻せる手順にする。途中で止まっても半端な DB でサーバが起きない。

**Non-Goals**:
- 自動で入れる設定（FR-114 の「後で足す」）
- Linux のサーバの更新と戻し（D8。ST60）
- 同梱の PostgreSQL の大きい版を上げる（D5）
- 携帯の端末を起動し直した後の収集の再開（proposal の「範囲の外として本人に返すもの」。defaults.md X1）
- 新しい版が待ち受けを開いた後に落ち続けるときの直し（Q5 の推奨。直した版を出すまでサーバは止まる）

## Decisions

### D1. サーバの更新の手順の順序と、前の版の退け先（defaults.md C1）

インストーラは、入っている版（プログラムの置き場のサーバの組み立ての印。`ashiato-server --version`）と自分の版が違えば更新の手順に入る。別の口は作らない。順:

1. 印 `update\state.json` を書く（`run_id`・前の版・新しい版・この回の退避の置き場・段 `started`）
2. サーバ（と更新係）のサービスを止め、サーバの開始の種類を「無効」にする（D20（仮））。段 `stopped`
3. DB のサービスを止める（同梱の PostgreSQL の小さい版を替えうるので。D5）
4. プログラムの置き場の中身を `<プログラムの置き場>\previous\` へ移す（前の `previous` は消してから）。段 `moved`
5. 新しい版の配布物で置き換え、DB のサービスを起こす。段 `replaced`
6. 新しい版の `migrate` を、`MIGRATE_BACKUP_DIR = <状態の置き場>\backups\pre-migrate\<run_id>\`（空の置き場）で走らせる。段 `migrating` → 終わったら終了コードを印に書く
7. 新しい版のサーバを起こし、180 秒以内に待ち受けが開くのを待つ（D16（仮））。段 `starting`
8. 開いたら: まず段 `listening` を印に書く（これ以後は DB もプログラムも戻さない —— 走らせ直しも `rollback` の口も 8 の残りを進める。review/spec.md R1）。開始の種類を「自動」・「アプリと機能」の版・更新係を（切ってなければ）起こす・退避の保持（D23（仮））・更新の記録に `applied`・控えの写し（D18（仮））・印を消す

前の版を退けるのは 1 世代だけ。設定・秘密・更新係の置き場には触れない（D6）。新しい版が要る鍵は「足りない鍵を足す」だけを許す。

**なぜ「先に新しい版の migrate を zip から走らせ、通ってから置き場を替える」でないか**: zip の中の実行ファイルは本人の置いた場所（ダウンロード）から動き、退避の置き場・`pg_dump`・ログの場所の指定がインストールの形と食い違う（ST55 の申し送り K1）。置き場を替えてから走らせ、戻しは置き場ごと戻す方が、戻した後の状態が「更新の前」と同じになる。

### D2. 戻す退避は「この回の置き場にある、取り終えた退避」だけ（defaults.md C2）

`migrate` に回ごとの空の置き場を渡す。`migrate` は退避を `pre-migrate-<時刻>.dump.part` に書き、`pg_dump` が通って中身があるときだけ `.dump` へ名前を変える（`migrate_backup.rs` は今 `pg_dump` の前に空のファイルを作る —— 書きかけと取り終えたものを名前で分ける。review/spec.md R5）。終わった後にその置き場に `.dump` が 1 つあれば「この回の退避がある」とみなす。
戻すのは、終了コードが 0・2・3・4 以外（1 と、落ちた・止められた）でこの回の退避がある回と、移行は通ったが新しい版が待ち受けを開かない回（D16（仮））。終了コード 2・3・4 は退避より前に止まる（申し送り st48 R21・st54 D3・`lib.rs:402-403`）。
終了コード 2・3・4 と、この回の退避が無い回は、DB を戻さずにプログラムの置き場だけを前の版に戻す（review/spec.md R4）。いちばん新しいファイルを選ぶ形は採らない（時計が戻ったときと 2 回目の試みで、前の退避を戻しうる。review/deep.md R1）。

### D3. 戻しは、別の名前の DB に入れてから入れ替える（defaults.md C3）

順（review/spec.md R6）: 新しい版のプログラムを `<プログラムの置き場>\failed\` へ退け、`previous\` を置き場へ戻し、DB のサービスを起こしてから、表の所有者（D25（仮））で:

1. `ashiato_restore` を（あれば消してから）、インストールの `CREATE DATABASE ashiato`（`commands::create_database_sql`。持ち主のほかの DB の水準の設定は持たない）と同じ形で作り、前の版の同梱の `pg_restore --exit-on-error --no-password --dbname=ashiato_restore <この回の退避>` で入れる
2. 入り終えたら、DB `postgres` に繋いで `ALTER DATABASE ashiato RENAME TO ashiato_failed_<UTC 時刻>` → `ALTER DATABASE ashiato_restore RENAME TO ashiato`
3. 段 `restored` を印に書き、`failed\` の新しい版の `ashiato-server update-history record-rollback` で更新の履歴に足す（移行が落ちた回は 2 行、待ち受けを開かずに戻した回は「戻した」の 1 行。書けなければ `record-rollback` が控えのファイルに書いて終了コード 5。D18（仮）・D26）。`failed\` は次に更新が通ったときに消す

どの役割で行うかは D25（仮）。サーバは止めてあるので、入れ替えるときに繋いでいる者はいない。途中で止まっても、元の名前の DB（落ちた状態）は 2 の前まで壊れない。
落ちた DB は `ashiato_failed_<時刻>` で残し、次に更新が通ったときに消す（D23（仮））。

### D4. 更新係は、プログラムの入れ替えの間だけ止める（defaults.md C4 を直した）

C4 は「更新係のサービスは止めない」としたが、更新係の実行ファイルはプログラムの置き場にあり、動いている実行ファイルを含む置き場は移せない。そのため D1 の 2 で止め、最後に（切ってなければ）起こす。更新係は DB に触れないので、止めても記録は失わない（止まる数分は確かめの機会が 1 つ減るだけ）。
切った印（`settings\check.json` の `enabled: false`）は更新でも戻しでも書き換えない。切ってあれば開始の種類「無効」のままにし、起こさない。戻した後は、更新係の結果の「見つけた最新の版」がそのまま残るので、`GET /update-status` は戻した新しい版を `available` で返す（同じ版をもう一度入れられる）。
PC と携帯の切った確認も、置き場（PC は `LOCALAPPDATA` の `update-check.json`、携帯は SharedPreferences `update_check.xml`）を入れ替えで書き換えない。

### D5. 同梱の PostgreSQL の大きい版が違う更新は止める（defaults.md C5）

インストーラは組み立てのときに `tools/release/pgsql.lock` の大きい版を持つ。入っている DB の置き場の `PG_VERSION` と違えば、印を書く前に止まる（サービス・プログラム・DB は替えない）。更新の記録には `not_applied`・`pg_major_mismatch` の 1 行を足し、帯と設定にも出す（review/spec.md R29）。
同じ大きい版の中では、新しい小さい版の `pg_dump` の custom 形式を前の小さい版の `pg_restore` が読み、データの置き場は小さい版を下げても起きる前提に立つ（P25。確かさ 2。CI の 2 つの zip は同じ `pgsql.lock` なので見ない —— Risks）。小さい版の違いは、プログラムの置き場（`pgsql\`）と一緒に入れ替え、戻すときは一緒に戻す（D1 で DB のサービスを止めてから移す）。

### D6. 戻すもの・戻さないもの（defaults.md C6）

戻すのは DB（D3）・プログラムの置き場（サーバ・更新係・同梱の PostgreSQL・インストーラの写し）・「アプリと機能」の版の表示。設定（`server.json`・`owner.json`）と更新係の置き場は、更新で書き換えない（足りない鍵を足すことだけ。前の版は知らない鍵を読み飛ばす —— `service.rs:226-233`）ので戻さない。

### D7. PC の収集アプリの古い版への入れ直し（defaults.md C7）

更新の窓は、見つけた最新の版が入っている版より新しいときだけ入れ替える。ただし D12（仮）の自動の戻しは「新しい版の後に前の版を動かす」場面で、ST52 D5・R14 の逆行に当たる（C7 の「現れない」を直す。review/spec.md R10）。新しい版は動き出したと書く前に手元の置き場の形を変えないので、前の版は未送信と設定を読める。新しい版が積んだ行は送信契約の版を焼いてあり、前の版は知らない版の行を送らずに残す（Scenario「PC は前の版に戻しても、新しい版が積んだ未送信を消さない」）。本人が手で古い版に置き換えることは止めない。未送信は行ごとに焼いた送信契約の版で送るので、古い版でも消えず、古い形でも送らない（`contract.rs:401-403`・`outbox.rs:134`）。

### D8. Linux のサーバの更新は扱わない（defaults.md C8）

Linux のサーバは帯を出さないので（ST57 D10）、FR-114 の「押す」が起きない。Linux の更新と戻しは ST60（FR-131）で扱い、`docs/handoff/ST60.md` に 1 行足す（この段で足した）。

### D9. 落とした配布物は指紋と照らす（defaults.md C9）

携帯と PC が落とした配布物は、版の一覧の答え（`api.github.com` から TLS で読んだもの）の `digest`（`sha256:<64 桁>`）と照らし、違えば・無ければ入れない（P2）。サーバの配布物は本人がブラウザで落とすので、インストーラは照らさない（配布元のページが指紋を示す）。

### D10. 携帯は更新の後に、受け手から収集のサービスを起こし、裏の位置の権限が無ければ位置を除いた種別で起こす（deep.md Q8）

> **本人の答え（deep.md Q8・第 2 回。推奨と同じ側）**: 位置を除いた種別で起こして、位置以外を取り続ける（位置は開くまで止まる）。
> 仮の形（起こさずに「開いてください」の通知だけ）は、別の席が「失わない」を否とした（provisional-review.md D10）。

`ACTION_MY_PACKAGE_REPLACED` の受け手 `PackageReplacedReceiver` を足し、送り先が設定してあれば `LocationService` を前景のサービスとして起こす（裏から始めてよい例外。P7）。送り先が無ければ起こさない。
種別を選ぶ `foregroundServiceType()`（`LocationService.kt:877`。今は `ACCESS_FINE_LOCATION` だけを見る）は引数の無い `onCreate()`（:397。Intent を受け取らない）で呼ばれるので、「裏から起こした」は Intent では渡さない。受け手は起こす前に 1 回きりの印のファイル（`noBackupFilesDir/update/replaced-start`）を置き、`onCreate` はそれを読んで消し、「受け手から起こした」をサービスの中に持つ（プロセスが落ちれば消える）。印があれば **位置の権限と裏の位置の権限が両方あるときだけ `location`、そうでなければ `specialUse`**、印が無ければ今のまま（位置の権限があれば `location`）。
画面（`MainActivity.kt:414`）が起こす Intent には「前景から起こした」の印を付け、`onStartCommand`（:899）でその印があり、受け手から起こしたサービスで、今の種別が `specialUse` で位置の権限があるときだけ、`startForeground` を `location` で呼び直して上げる（`startForeground` を `onStartCommand` へ動かさない —— :401 の「前景に上がってから置き場を開く」の期限を守る）:

| 起こし方 | 位置の権限 | 裏の位置の権限 | `onCreate` の種別 | 画面から開いた後 | 位置のソース | 通知 ID 12 |
|---|---|---|---|---|---|---|
| 画面から・立て直し（`START_STICKY`。Intent が `null`）（今のまま） | あり | 問わない | `location` | `location` | 取る | — |
| 画面から・立て直し（今のまま） | なし | — | `specialUse` | `specialUse` | 欠け（ST06 Q7） | — |
| 受け手から（印のファイルあり） | あり | あり | `location` | `location` | 取る | 出さない |
| 受け手から（印のファイルあり） | あり | なし | `specialUse` | `location` に上げる | 開くまで欠け（満たされていないもの「権限」） | 1 回出し、上げたら消す |
| 受け手から（印のファイルあり） | なし | — | `specialUse` | `specialUse` | 欠け（今と同じ） | 出さない（開いても変わらない） |

位置のソースの取得できる状態（`AndroidCapability.kt:27-46`。今は `ACCESS_FINE_LOCATION` だけを見る）に、「受け手から起こした」を入力として足す: 受け手から `specialUse` で起こしたサービスで、位置の権限があり裏の位置の権限が無い間は「取得できない（満たされていないもの: 権限）」と生存信号に載せ（spec の本文と同じ条件。`specialUse` の間に本人が端末の設定から裏の位置の権限も許したら、次の取得契機から今どおり位置を取りにいく）、`collect()` を呼ばない（深掘り（ST06）Q5 の「壊れているのに動いていた」に見せない）。
通知の文は「更新の後、位置の収集が止まっています。開いてください」。通知の権限が無ければ出ない（位置は開くまで止まるが、位置以外は取り続ける）。位置の種別に上がったら通知 ID 12 を消す。
画面からと立て直しは今の振る舞いを変えない（欠けの扱い・上げ直し・通知はどれも受け手から起こしたサービスだけに効く。画面から位置の権限なしで `specialUse` で立った後に位置の権限を許した端末は、今どおり立て直すまで `specialUse` のまま。review/spec.md R43。印のファイルは受け手だけが置き、`onCreate` が読んだら消すので、後の立て直しに残らない）。受け手から起こした後、開く前に OS が立て直したときは今の立て直しと同じ形になる（この Story では変えない）。
前提 P26: 動いているサービスの種別は `startForeground` の呼び直しで変えられ、前景の画面から起こした `onStartCommand` の中なら位置の型で呼んでよい（確かさ 2）。外れたら、上げるときにサービスを止めて画面から起こし直す（`stopSelf` → `startForegroundService`）形に替える（位置以外のソースは数秒止まる）。
- 本人の答え: 位置以外（アプリ利用・生存信号）を更新の直後から取り続ける。取らなかった期間の記録は後から作れない（loss: uncaptured）
- 確かめ: Task 9 の計測テスト（`adb install -r` の後に、裏の位置の権限のある端末は `location`、無い端末は `specialUse` で `LocationService` が動き、生存信号が積まれる。開いた後に同じプロセスのまま `location` に上がる）と Robolectric の種別の表の試験
- 前提が外れたとき: エミュレータで、targetSdk 36 の受け手から `specialUse` の前景のサービスを起こせないと分かったら（P7 が確かさ 4 で外れたら）、通知だけの形（本人が退けた側）には戻さず、上流へ返して本人に聞き直す

### D11（仮）. 押したときに版の一覧を取り直し、決まった 2 つの宛先から落とす

収集アプリは配布物の場所と指紋を持っていない（P21）ので、押したときに版の一覧を ST57 と同じ形で取り直し、自分の部品の配布物の `browser_download_url` と `digest` を読む。`browser_download_url` が `https://github.com/rai-256/ashiato/releases/download/` で始まらなければ入れない。
転送は `https://release-assets.githubusercontent.com/` への 302 だけを追う（P3。期限つきの署名つきの道）。取り直しは 24 時間の確かめの数えと記録に入れない（押すのは本人の操作で、定期の確かめではない）。
- 反転条件: 配布元が配布物の宛先を別の名前へ移した（P3 の転送先が変わった）と分かったら、許す宛先を足す（ST57 と同じく組み立ての定数）
- 試験の組み立てでは、版の一覧と配布物の宛先を `ASHIATO_COLLECTOR_TEST_UPDATE_URL`（PC）・`BuildConfig.UPDATE_CHECK_URL`（携帯の試験の組み立て）の手元の受け手に差し替え、転送先の許す宛先も同じ受け手の別の道にする

### D12（仮）. PC の入れ替えは、版ごとの置き場に置いて起こし、動き出してから自動起動を向ける

- 通知（札 `update`）を押すと、動いている収集アプリが `current_exe() update` で窓を開く（`setup` の窓と同じ形。P22）。`update` の口は本人が直に使える（通知は版ごとに 1 回なので、見逃した後も入れられる）
- 窓は `y` を受けたら、配布物を `<状態の置き場>\versions\<版>\`（`LOCALAPPDATA` の収集アプリの置き場。系統ごとに別。起点を固定し、2 回目の更新でも入れ子にならない。review/spec.md R11）に落として照らし（zip を開いて実行ファイルを取り出す）、動いている収集アプリに置き場の印（`update-handoff.json`）で止まるよう頼み、止まった（印の応え）のを待ってから新しい版を起こす。同時に 2 つが収集しない（P23）
- 新しい版は、送る仕組みと見回りを始めた後に `update-alive.json`（版と時刻）を書く。窓は 60 秒（D17（仮））待ち、書かれたら自動起動の台本を新しい版の場所に書き換え、前の版より古い版の置き場を消す（前の版は 1 つ残す）
- 新しい版は、窓の子でも窓を閉じて止まらないように、`CommandExt::creation_flags` の `DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP` で起こす（P24。`unsafe` は要らない。review/spec.md R9）。入れ替えの途中で窓が閉じられたら、新しい版は `update-alive.json` を書いたうえで動き続け、自動起動の台本は前の版のまま（次に窓か `update` の口を開いたときに台本を向け直す）
- 書かれなければ、新しい版のプロセスを止め（窓が起こした子なので `Child::kill`）、前の版を起こし直し、台本はそのまま、「新しい版 <版> が動き出さなかったので、<前の版> に戻しました」を通知する
- 動いている実行ファイルの名前を変えない（P4 に依らない）。前の版の実行ファイルは元の場所に残る。前の版より古い `versions\<版>\` は消し、残るのは今の版と前の版の 2 つ
- 時刻サーバの検査は、`Command::new(update_executable(&staged)?)`（版ごとの置き場の中の、決まった名前の実行ファイルだけを返す関数）の 1 つの形を許す（`tools/check-no-time-server.sh` の許可と自己検査の植え付け）
- 反転条件: Windows の実行時の試験で、版ごとの置き場からの起動が自動起動・通知の差出人・除外の登録のどれかを壊すと分かったら、元の場所での置き換え（P4 を確かさ 4 に上げてから）に替える

### D13（仮）. PC の窓の形

窓は `setup` と同じ端末の窓（コンソール）。文は「入っている版 0.4.0 / 新しい版 0.5.0 / 入れ替えますか [y/N]」。落とす・照らす・待つの間は進みを 1 行ずつ出す。終わったら「0.5.0 に入れ替えました」か理由を出し、Enter で閉じる。
- 反転条件: 本人が窓の形に違和感を返したら（確認バッチ）、文と並びを直す

### D14（仮）. 名前

- サーバ: 印の段 `started`・`stopped`・`moved`・`replaced`・`migrating`・`starting`・`listening`・`restored`。新しい版の退け先（戻しの間）`<プログラムの置き場>\failed\`。印と記録の置き場 `<状態の置き場>\update\`（`state.json`・`outcomes.jsonl`・`history-pending.jsonl`）。回の id `run_id` = `<UTC の YYYYMMDDTHHMMSSZ>-<16 進 8 桁の乱数>`。退避 `backups\pre-migrate\<run_id>\pre-migrate-<時刻>.dump`。前の版の退け先 `<プログラムの置き場>\previous\`。DB `ashiato_restore`・`ashiato_failed_<UTC>`。インストーラの口 `rollback`。
  サーバの口 `ashiato-server update-history record-rollback`（戻した行を書く。移行が落ちた回は 2 行、待ち受けを開かずに戻した回は 1 行。書けなければ `--pending-out <控えのファイル>` に書いて終了コード 5）・`ashiato-server update-history replay`（控えを写す）
- 更新の記録の 1 行: `{"at", "result": "applied"|"rolled_back"|"not_applied", "to_version", "from_version", "reason": {"kind", "failed_version", "sqlstate"}}`
- `GET /update-status` の `last_update`: 同じ形（`result`・`at`・`to_version`・`from_version`・`reason`）
- PC: 口 `update`・置き場 `<状態の置き場>\versions\<版>\`・置き場のファイル `update-handoff.json`・`update-alive.json`・札 `update-rollback`（戻した通知）
- 携帯: `UpdateInstall.kt`・`PackageReplacedReceiver.kt`・通知 ID 12（「更新の後、位置の収集が止まっています。開いてください」。ID 9〜11 は ST57・13 は送り先）・`noBackupFilesDir/update/`
- 試験の差し込み（試験の組み立てだけ）: `ASHIATO_SETUP_TEST_STOP_AT`（`migrating`・`after-migrate`・`after-listen`・`restoring`）・`ASHIATO_SERVER_TEST_MIGRATE_FAIL` の `during-backup`（退避を書く途中で止まる）・`ASHIATO_SETUP_TEST_PG_MAJOR`・`ASHIATO_SERVER_TEST_MIGRATE_FAIL`（`sqlstate:<符号>`・`before-backup`）・`ASHIATO_SERVER_TEST_EXIT_AFTER_LISTEN`・`ASHIATO_SERVER_TEST_NO_LISTEN`・`ASHIATO_COLLECTOR_TEST_UPDATE_NO_ALIVE`

### D15（仮）. 理由の短い説明

表は spec（「更新が入らなかったことを…」の Requirement）に置いた（review/spec.md R16）。ここは文の選び方と反転条件。SQLSTATE は級（先頭 2 文字）で短い説明にする: `23` 制約に反した・`42` 定義が当たらない・`53` 資源が足りない（空き・記憶）・`57` 止められた・`08` 繋がらない・ほかは「DB の誤り」。種別は `db_ahead`「DB に新しい版の知らない版がある」・`ledger_unreadable`「版の記録を読めない」・`channel_mismatch`「DB の系統が違う」・`db_not_loopback`「DB の接続先が手元でない」・`backup_failed`「移行の前の退避に失敗した」・`connect_failed`「DB に繋げない」・`server_not_listening`「新しい版のサーバが起きない」・`aborted`「移行が途中で終わった」・`pg_major_mismatch`「同梱の PostgreSQL の大きい版が違う」。
- 反転条件: 本人が文に違和感を返したら直す（値の一覧は変えない）

### D16（仮）. 新しい版の待ち受けを待つのは 180 秒

今のインストーラと同じ 180 秒（`wait_for_port`）。開かなければ D2 の戻しへ。
- 反転条件: 本物の PC で、移行の多い更新の後の最初の起動が 180 秒を超えると分かったら延ばす

### D17（仮）. PC の新しい版を待つのは 60 秒

Q1 の推奨の「1 分以内」。
- 反転条件: Windows の実行時の試験で、正常な起動が 60 秒の半分を超えると分かったら延ばす

### D18（仮）. 「戻した」の行の向きと、控えの写し

「戻した」の行の前の版は落ちた新しい版、今の版は戻した先の版（版の変化の向きを、更新の行と同じ「前 → 今」で読めるようにする）。「落ちた試みの結果」の行の前の版と今の版は、戻した DB の最後の試みの行と同じにする。足す行（移行が落ちた回は 2 行、待ち受けを開かずに戻した回は「戻した」の 1 行。D26）は同じ取引で書く。書けない（終了コード 5）ときは、インストーラが渡した控えのファイル（`--pending-out <状態の置き場>\update\history-pending.jsonl`）に、書けなかった行を 1 行 1 件の JSON で `record-rollback` が足す（行の中身を `record-rollback` の 1 か所で作る。サーバの口のログは標準出力に出るので、標準出力では渡さない。review/spec.md R35）。インストーラは終了コード 5 を「控えに書いた」と読む。
控え（`history-pending.jsonl`）は 1 行 1 件で、`written_at`（写した時刻。写すまで無い）を持つ。写すのは `update-history replay`（インストーラが、更新が通った後に表の所有者で走らせる）で、`written_at` の無い行だけを書いた順に足し、足した後に控えの行に `written_at` を足す（写した印。控えは管理者だけが書けるので、インストーラが書き直す）。
- 反転条件: 書き出しを読む側（衛星）が `rolled_back` の行の向きを逆に読むと分かったら、書き出しの説明書の文を直す（行は直せない）

### D19（仮）. `last_update` は記録の最後の 1 行だけを読み、1 MB を超えたら読まない

更新の記録は追記のみで、1 年に数十行。サーバは最後の行だけを読む（帯に要るのは最後の結果だけ）。
- 反転条件: 記録が 1 MB に近づく使い方（開発版を毎日入れて数年）が見えたら、古い半分を退ける（ST57 D19 と同じ形）

### D20（仮）. 途中で止まった更新は、まず前の版へ戻し終えてからやり直す

印が残っていれば、どちらの版のインストーラ（ST58 以後）も、段が `listening` なら D1 の 8 の残りを進めて終え（DB もプログラムも戻さない）、それより前の段ならまず前の版へ戻し終える（段が `migrating` 以後で、この回の退避があれば DB も D3 で戻す。`restored` 以後なら DB は戻してある）。その後、自分の版が前の版と違えば D1 を初めから（新しい `run_id` で）やり直す。
「続きから前へ進める」形は採らない —— 途中の版で止まった DB の上で移行を続けると、どの退避を戻せばよいかが 2 つの回にまたがる。
開始の種類「無効」は、新しい版の待ち受けを確かめた後か、前の版で起こし直すときにだけ「自動」へ戻す。戻しそのものが落ちたら「無効」のまま、退避の場所と `rollback` の口を示して止まる。
戻しは段ごとに印を書き、どこで落ちても同じ判断で再開する（review/code.md R53・R54）: 最初に戻しに入ったとき「DB を戻すか」を印に残す（再開は段 `program_*` から決め直さない）。プログラムの置き場は `program_restoring`（新しい版を `failed\` へ退ける。前の版はまだ `previous\` にあるので、やり直せる）→ `program_returning`（`previous\` の前の版を戻す。再開は残りを移すだけで、前の版を `failed\` へ動かさない）→ `program_restored` の順。段 `moved` で前の版を `previous\` へ移し終えていない回（移し終えたら印に `program_moved` を足す。無ければ途中）は、program に残るのは前の版の未移動の分だけなので、退ける段を飛ばして `program_returning` から戻す（review/code.md R18・R64）。既にある `failed\` は消さない。DB を戻した回だけ `rolled_back` を書き、書いたことも印に残す（再開で 2 行にしない）。段 `started` / `stopped` からの戻しは止めたサービスを起こし直すだけで、起きているサービスへの開始は成功とみなす。
- 反転条件: 本人が「前の版のインストーラで走らせ直したら新しい版へ進めてほしい」と返したら、D20 を直す（戻し終える部分は変えない）

### D21（仮）. 携帯は `PackageInstaller` の手続きで入れる

APK を `noBackupFilesDir/update/<版>.apk` に落とし、照らしてから `PackageInstaller` の手続き（session）に書き込んで確認の画面を出す（`FileProvider` で他のアプリへ APK を見せない）。手続きの結果（成功・取り消し・失敗）を受けたら APK を消す。アプリを開いたときにも、残っている APK を消す。
- 反転条件: エミュレータで、手続きの確認の画面が本人の操作なしに出ない端末（Android の版）があると分かったら、`ACTION_VIEW` + `FileProvider` に替える

### D22（仮）. 携帯の更新の通知は、最初の画面を開いて落とし始める

ST57 の通知（`update` のチャンネル）の押した先を、最初の画面に「落とし始める」の印を付けた形にする。最初の画面は、行を押したのと同じ処理を 1 回だけ始める（画面を回しても 2 度始めない）。
- 反転条件: 本人が「通知を押したら、画面を開くだけにしてほしい」と返したら、印を外す

### D23（仮）. 退避と落ちた DB を消すのは、待ち受けを確かめた直後

D1 の 8 で、`backups\pre-migrate\` の下の、この回の置き場以外の置き場と `pre-migrate-*.dump`（ST58 より前の版がじかに置いたもの）を消し、`ashiato_failed_` で始まる DB を表の所有者で消す。戻した回・入らなかった回は消さない。
- 反転条件: 本人が Q7 で別の保持を選んだら、その数え方に替える

### D24（仮）. Windows の CI の通しは、版の違う 2 つの zip で見る

`tools/release/update-apply-install.sh` は、同じ commit から版番号だけを替えた 2 つの試験の組み立ての zip（`0.4.0-test` 相当と `0.5.0-test` 相当。`ASHIATO_VERSION` を替えて 2 回組む）を使い、古い方を入れてから新しい方で更新・戻し・途中で止まる・やり直しを見る。
差し込みは Scenario の GIVEN の形に合わせる（review/code.md R36・R58）: インストーラの `STOP_AT` は誤りを返して自動の戻しを通るので、「途中で止まった印が残る」形は `ASHIATO_SETUP_TEST_CRASH_AT`（戻さずにその場で終わる）、「移行の段で待つ間を外から見る」形は `ASHIATO_SETUP_TEST_PAUSE_AT`（`update\paused` を置き `update\resume` を待つ）で作る。サーバの起動の差し込み（`NO_LISTEN` / `EXIT_AFTER_LISTEN`）は SCM が起こすサービスに届くようサービスの環境（`Environment`）に置き、値を版番号にしてその版のサーバだけが従う（戻した前の版は従わない）。2 つの zip は同じ移行を持ち当てる版が無いので、`MIGRATE_FAIL=sqlstate:…` はそのときも最後の版で落ちたことにする。DB の版の記録の細工は Scenario の後に戻す（DB は Scenario をまたいで同じ）。PC の収集アプリの戻しの試験は `ASHIATO_COLLECTOR_TEST_UPDATE_ALIVE_WAIT_MS`・`ASHIATO_COLLECTOR_TEST_PREVIOUS_EXE`（どれも試験の組み立てだけ）。
- 反転条件: CI の時間が 2 つ組むことで job の上限に近づくと分かったら、2 つ目の組み立てを版の印だけを替える形に替える

### D25（仮）. 戻しは表の所有者 `ashiato_owner` で行う（D3 から分けた。review/spec.md R14）

`ashiato_owner` は `CREATEDB` を持ち、DB の持ち主なので、DB の管理者なしで行える（P20。確かさ 3）。DB の管理者の合言葉はインストールの後に捨ててあり（`run_psql`）、戻しのために控え直さない。
- 失わない側: 所有者で行う（管理者の資格を新しく控えない）。所有者で足りなければ戻しが落ち、D20（仮）の「戻しが落ちたら止まる」で DB は元の名前のまま残る（記録は失わない）
- 反転条件: Windows の CI の通し（Task 6）で、所有者の `pg_restore` が通らないと分かったら、戻しの間だけ `repair` と同じ一時の接続規則で管理者として入る形に替え、spec の「戻しを、DB の管理者でない表の所有者の役割で行う」と Scenario「戻しは DB の管理者の役割を使わない」を直す（上流へ返す）

### D26. 新しい版が待ち受けを開かずに戻した回の更新の履歴は「戻した」の 1 行で、当てた版の名前を持つ（deep.md Q9）

> **本人の答え（deep.md Q9・第 2 回。推奨と同じ側）**: 結果の行は書き直さず、「戻した」の 1 行に当てた版の名前を持たせる。
> 仮の形（`applied` / `nothing_to_apply` の行を書き直してから `rolled_back`）は、別の席が否とした（provisional-review.md D26）—— ST51 の `previous_version`（`update_history.rs:128-136`）がその行の今の版（動いていない新しい版）を「前の版」と読み、以後の試み・起動の行の `from_version` が誤ったまま書き出しに出る。

この回は移行が通っているので「落ちた試みの結果」（`failed`）は事実に合わない。退避を取った後に書いた結果の行は戻すと消えるが、書き直さない。`rolled_back` の 1 行だけを足し、その行の `applied_versions` にこの回に当てた版の名前を入れる（当てる版が無い更新 —— 多くの版はこれ —— なら空）。
当てた版の名前は `record-rollback`（`failed\` の新しい版のプログラム）が、自分の並び（`MIGRATIONS`）にあって戻した DB の版の記録に無い版を、並びの順に並べて作る（インストーラから名前を渡さない。新しい版の並びは新しい版のプログラムだけが知る）。
`previous_version` は `rolled_back` を数えないので、戻した後も前の版は戻した先の版のまま。前の版のサーバが起きても起動の行は足されず、次の更新の試みの行の前の版は戻した先の版になる。
`migrate` は当てる版が無い回も退避を取る（`migration_ledger.rs:647`）ので、この回の退避はあり、D23（仮）の「この回の 1 つを残す」も成り立つ。退避に失敗した回（`backup_failed`）は退避が無いので戻さず、行も足さない。
前の版の表が `rolled_back` を受け付けない（終了コード 5）ときも、`record-rollback` が書けなかった行（当てた版の名前を含む）を控えのファイルに書く（D18（仮））。控えから写す回も同じ 1 行で、`applied` / `nothing_to_apply` を写さない。
書き出しの説明書には「`rolled_back` の直前に `failed` が無ければ、移行は通ったが待ち受けを開かずに戻した回」と書き、ST51 の「試みの後に結果の行が無ければ戻した回」の読み方から `rolled_back` を外す（distribution の「S-01 の版が変わると、更新の履歴に 1 行残る」を MODIFIED。audit 2026-11-15 C4）。
- 本人の答え: この形の行も書き出しに出て直せない（loss: exported）。ただし前の版は正しく読め、何を当てて戻したかも `rolled_back` の行から読める
- 確かめ: Task 1 の試験（当てた版あり・なしの 2 本と、戻した後の起動と次の試みの前の版）と、Windows の通し（Task 6）

### D27（仮）. 待ち受けを開かずに戻した回の文は「待ち受けを開かなかったので」で、再開でも落ちたときの分類を使う（review/code.md R46・R61）

新しい版が待ち受けを開かずに戻した回は、窓・画面の帯とも「移行が落ちたので」ではなく「待ち受けを開かなかったので、<前の版> に戻しました」と出す（理由の種別 `server_not_listening`）。インストーラは落ちた時点の分類（結果と理由）を印に書いてから戻し、戻しが途中で落ちて走らせ直した回も、更新の記録の行はその分類で書く（`Aborted` に決め直さない）。分類の無い回（電源が落ちて止まった更新を走らせ直した）は、DB を戻していれば `rolled_back`、戻していなければ `not_applied` で、理由は「移行が途中で終わった」。
- 反転条件: 本人が「戻した理由は 1 つの文でよい」と返したら、または新しい版のサーバが待ち受けを開けなかった理由（番号の取り合いなど）を印に残せるようになったら、その理由を文に出す形に直す

## Risks / Trade-offs

- [ST58 より前の版へ戻した回は、帯に「戻した」が出ない] → 前の版の画面とサーバは `last_update` を知らない。窓には出る。更新の記録は DB の外にあるので、次に ST58 以後の版が入ると帯に出る（最後の行が「戻した」のままなら）。ST58 を初めて入れる回の失敗だけに起きる
- [ST58 より前のインストーラで走らせ直しても、印を読まない] → 前の版のインストーラは印を知らず、別の版のファイルで止まる（P10）ので、何も替えない。開始の種類「無効」のサーバは起きないまま。窓とインストールの文書（`docs/install.md`）に「`rollback` の口は新しい版の `ashiato-setup.exe` で走らせる」と書く
- [新しい版が待ち受けを開いた後に落ち続ける] → 戻さない（Q5 の推奨）。サービスの回復が起こし直すが、直した版が出るまでサーバは止まり、記録は収集アプリの手元に溜まる（上限 90 日・2 GB）。窓にログの場所を出す
- [戻しの `pg_restore` は DB 全体を入れ直すので、大きい DB では時間がかかる] → その間サーバは止まっていて、記録は手元に溜まる。失うものは無い。窓に進みを出す
- [P20 は確かさ 3] → 非管理者の `pg_restore` が、所有者の持つものだけで足りるかは、Windows の CI の通し（Task 6）で確かめる（D25（仮）の反転条件）
- [P25 は確かさ 2。小さい版の違う PostgreSQL の戻しは CI で見ない] → CI の 2 つの zip は同じ `pgsql.lock` を使う。`pgsql.lock` の小さい版を上げる PR では、その PR の CI で前の zip（前の小さい版）から更新と戻しを通すことを `docs/release.md` に書く
- [PC の収集アプリが落とす宛先が 2 つ増える] → 押したときだけ。どの版を落としたかが配布元に見える（ブラウザで落としても同じ）
- [退避を 1 つしか残さない] → 2 つ前より古い更新の前の写しは無くなる（Q7 の不可逆）。毎日のバックアップ（30 日）は残る

## Migration Plan

- 移行 `migrations/<時刻>_update_history_rolled_back.sql`: `core.update_history` の `outcome` の CHECK を落として、`rolled_back` を足した CHECK を作り直す（列と今の値は変えない）。`.down.sql` は `rolled_back` の行が無いときだけ元の CHECK に戻す（あれば止まる）。`migrations/ORDER` の末尾へ 1 行
- 配った版から ST58 の版への最初の更新は、ST58 の新しいインストーラが手順を持つので、前の版のインストーラに依らない。最初の更新が落ちると、前の版（ST58 より前）へ戻る —— Risks の 1 つ目
- 戻し方（この Story を取り消す）: 前の版のインストーラは、ST58 で入れた版の上では別の版のファイルで止まる（今と同じ）。ST58 の `rollback` の口で前の版へ戻す

## Open Questions

（無し。仮の決定は D11〜D25（仮）・D27（仮）で、反転条件を書いた。D10・D26 は別の席が否とし、deep.md Q8・Q9 の本人の答えで決めた）
