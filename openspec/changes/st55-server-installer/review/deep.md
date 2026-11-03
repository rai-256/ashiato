# ST55 深掘りの問いの独立レビュー（deep-review）

対象: `openspec/changes/st55-server-installer/deep-questions.json`（第 1 回。Q1〜Q5、どれも `open`）・`premises.md`（P1〜P17・聞かない既定 C1〜C15）。
手順 1〜5 は要件・申し送り・監査・コードだけで先にやり、そのあとで問いの一覧と聞かない既定に突き合わせた。問いの JSON は触っていない。
Windows の上では何も走らせていない（この席も WSL の中。P3・P4 の Windows 側、サービスの登録、ACL は未走行のまま）。

## 手順ごとの確かめ

- **手順 1（要件どうしの衝突）**: FR-108（requirements.md:982-1002）・FR-109（:1003-1011）・FR-110（:1012）・FR-111（:1014-1016）・FR-132（:1116-1119）と、
  関わる FR-106（:974-979）・FR-112（:1017-1023）・FR-119（:1040）・FR-133（:1120-1123）を読んだ。満たす要件どうしの字面の衝突は見つからなかった。
  要件と聞かない既定・問いの族の食い違いは 2 つ: (a) FR-109 と C3 は「本人は読むだけ」なのに、Q4 の族は「記録を消すのは本人がフォルダを消す」→ R10。
  (b) FR-108 の理由「ログオンしていなくても携帯の記録を受け取れる」（:987）は、携帯が届く経路（`tailscale serve`。docs/network.md:15）がログオン前に動くことに乗るが、どこにも書かれていない → R3
- **手順 2（扉の幅）**: ST55 の doors は空（docs/stories/ST55.md:6・:48）。該当なし（確かめた範囲: ST55.md の「触れる扉」と requirements.md §5 の関与要件に FR-108〜FR-111・FR-132 が無いこと）
- **手順 3（新たに立つ一方通行）**:
  (a) DB を作るときの文字の並べ方・符号化（initdb で固定。変えるには全部を書き出して入れ直す）→ C1 が受けている。A にしない判断は妥当（R9 に前提の確かさだけ）。
  (b) 記録の鍵の入力: 取り込み器の利用者 `ASHIATO_ARCHIVE_USER_ID` は記録の `user_id` になり、門が書き換えを拒む（contract_version.sql:173）。既定が C に無い → R1。
  (c) 外に出るもの: サービスにすると S-01 の標準出力と D-01 のログの行き先が変わる。production-prep.md:51「一度出たログは消せない」→ R2。
  (d) 捨てるもの: 止まったまま出口の無い状態の間、携帯の未送信は FR-8 / FR-9 の上限（90 日・2 GB）で捨てられる → R4・R5
- **手順 4（日常に影響する選択）**: 容量（C: に固定される原本・退避・拠点内のバックアップ・書庫の写し）→ R6。切り替えの日に収集アプリを全部設定し直す手間 → R4。
  常駐（起動時から 2 つのサービス）は要件が決めた（FR-108）。毎日目に入るものは無い
- **手順 5（既存コードが要件を満たしていない箇所）**: 申し送り（docs/handoff/ST55.md K1・K2・C5/K3・C7・C13）は P10〜P15 と C3・C4・C6・C7・C9 が受けていることを grep で確かめた
  （`crates/server/src/migrate_backup.rs:27-33`・`media.rs:41-43`・`archive/config.rs:36-64`・`lib.rs:4365-4388`・`main.rs:6-10`）。
  それ以外に見つけたもの: 取り込み器は `ASHIATO_ARCHIVE_USER_ID` が無いと黙って起きない（lib.rs:4433-4448・scheduled.rs:64-67）→ R1。
  ログは `tracing_subscriber::fmt()` で標準出力にだけ出す（lib.rs:4359）→ R2。DB の 3 つの役割を作る口は `docker compose exec` にしか無い（tools/db-roles.sh:43）→ R7。
  画面の合言葉はいま平文で持ち、印は平文の上の HMAC（web_session.rs:98-105・:171-175）→ R12
- **手順 6（過去の取りこぼし。`python3 scripts/upstream_misses.py report`）**:
  - 「〜が解消するまで止める」には解消しないまま上限を越えたときを書く → 該当（C5・C10 の「止まる」。R5）
  - 「送らずに残すので失わない」には残した行の上限を挙げる → 該当（R4・R5）
  - 手元の状態を持つ決定には「保存場所が丸ごと消えた／入れ直した」を問う → 該当（C10 の「DB はあるのに秘密が無い」。R5）
  - OS のサービス・コマンドに依る決定は、既定・権限・止まっているときを実機で確かめて前提に書く → 該当（R8）
  - 申し送りの全項目を突き合わせる → `docs/handoff/ST55.md` の C6（正典 2 つの MODIFIED と印つきの試験）だけが C の表に写っていない（R14）。ほかは写っている
  - 「後のために残す」なら後でどう見つけるか → Q4 は「入れ直せば続く」で受けている。該当なし
  - DB や写しを増やす決定に削除が届くか → 書庫の写しの置き場を変えるだけで、写しを増やさない。該当なし
  - 時計が狂う・戻る族 → ST55 に時刻で状態を一方向に動かす決定は無い（C11 の移行はインストールの時だけ。版の検査は ST48）。該当なし
  - 画面の寸法・proto の族 → 画面の構造を問う問いは無い（Q3 は口の置き場で、格子や配置ではない）。該当なし
  - 検証コマンドの族（11 件）→ tasks の段の観点。この段では該当なし
- **手順 7（族の網羅）**: Q3 の族「画面の合言葉」に「決めずに閉じた」、族として「秘密の欠け」が無い → R5。Q4 の族「入れた後の片付け」の「記録を消す」が C3 と合わない → R10
- **手順 8（前提の確かさ）**: C2・C3・C11 が依る「`pg_ctl register` と仮想アカウント・サービスの依存・回復・準備ができたと告げる時点」に P の行が無い。P17 は 2 → R8。
  C1 は P4（c）= 2（Windows 未走行）に乗る → R9。Q5 の推奨はサンドボックスの UAC に乗るが P16 に無い → R13

## 一覧との突き合わせのまとめ

- **抜け**: R1・R2・R3・R4・R5・R6・R7
- **不要**: なし（Q1〜Q5 はどれも要件が決めていない幅がある。Q3 の「決める口の置き場」も FR-111 は「インストールの最後」としか言わない）
- **分類違い / 不可逆の根拠が無い**: なし（5 問とも `loss` 無しの `open` で、失われるものは見つからなかった。Q3 の案 3 だけ外に出る経路がある → R11）
- **既定の無い仮の問い**: なし（5 問とも `recommended` がある）
- **文字で決めさせている画面の問い**: なし
- **書き手の判断「A は無い・C1 は聞かない」**: C1 は失わない側（OS に依らない並べ方）が安く、範囲の中で、要件を変えないので、聞かない既定でよい。
  ただし C の中に、失うものがあるのに表に出ていない既定の穴が 2 つある（R1 の利用者の値・R2 のログの行き先）。どちらも失わない側が安いので A にはならず、C に足せば済む。
  R5 の出口と R6 の容量は、失わない側に費用があるので、B の問いにするかどうかを呼び出し元が決める

---

## R1. 取り込み器の利用者（`ASHIATO_ARCHIVE_USER_ID`）を渡す既定が C に無い。渡し忘れると Takeout が黙って入らず、別の値を渡すと記録が画面に出ないまま書き換えられない
- 成果物: openspec/changes/st55-server-installer/premises.md
- 根拠: crates/server/src/lib.rs:4433-4448（`user_id` が無ければ `archive_disabled` をログに出すだけで取り込み器を起こさない）・crates/server/src/scheduled.rs:64-67（定期取得も同じ）・crates/server/src/archive/config.rs:24-26。
  画面は利用者を名乗らずに既定（`Uuid::default()` = nil）を読む（lib.rs:2599・:2662・:2965 の `unwrap_or_default()`）。docs/archive-inbox.md:21-23「`.env` の `ASHIATO_USER_ID` と同じ値にする … 別の値を入れると読んだ書庫が画面にも稼働状況の格子にも出ない」。
  収集アプリの既定も nil（crates/collector-windows/src/autostart.rs:83）。記録の `user_id` は門が書き換えを拒む（migrations/202610300030_contract_version.sql:173）。
  ST55 の完了の判定「ダウンロードのフォルダに落とした Takeout は取り込まれる」（ST55.md:63）。premises.md の C7（秘密）・C9（置き場）にこの値は無い
- kind: irreversible
- loss: rewrite-all
- 提案: 聞かない既定に「インストーラは `ASHIATO_ARCHIVE_USER_ID` に収集アプリ・画面と同じ既定の利用者（nil）を渡す。インストールごとに作り直さない」を足す（失わない側が安いので問わない）。
  Windows の CI の試験で、`archive_disabled` が出ないことと、ダウンロードに置いた `takeout-*.zip` の記録が既定の利用者の読み出しに出ることを見る
- 処置: fixed D16 仮 —— premises.md C16（nil の既定の利用者を渡す。CI で見る）。spec の段で design.md の D16（仮）に写す

## R2. サービスにしたときの S-01 と D-01 のログの行き先が決まっていない。行き先によっては、ほかのアカウントが読める所へ出て消せない
- 成果物: openspec/changes/st55-server-installer/premises.md
- 根拠: crates/server/src/lib.rs:4359（`tracing_subscriber::fmt()` = 標準出力だけ。ファイルに書く口は無い）。docs/production-prep.md:195「データベースのエラーは … 詳細はログにだけ出す」・:51「ログ … 一度出たログは消せない」。
  premises.md の C6 は D-01 のログの 3 つの設定だけで、`log_destination` / `logging_collector` を決めていない。C14 は置き場に `logs` の名前を並べるだけ。
  サービスの標準出力はどこにも残らないので、起動の拒否（lib.rs:4379・:4408 の `eprintln!`。合言葉の欠け・役割の誤り）も本人に見えない。
  Windows の「アプリケーション」のイベントログをほかのアカウントが読めるかは、この席でも書き手でも確かめていない（P7 は登録簿だけ）
- kind: irreversible
- loss: exported
- 提案: 聞かない既定に「D-01 は `logging_collector=on` で C3 の置き場の `logs` に書き、イベントログには出さない。S-01 も同じ置き場のファイルに書く（起動の拒否の理由を含む）」を足し、
  P に「イベントログ（アプリケーション）の読み手」を足す。Windows の CI で `SHOW log_destination` と、ログのファイルの ACL を C3 と同じに見る
- 処置: escalated —— 仮（D17）で閉じていたが、別の席（provisional-review.md）が否とし、第 2 回の Q9（deep-questions-r2.json）で人間に返した。本人の答え（`logs\db` に DB のサービスの書きを足し、起動の数行がイベントログに出るのは受け入れる）を design D17・D3 と spec に写した（2026-11-03）

## R3. 「ログオンしていなくても携帯の記録を受け取れる」が、携帯が届く経路（`tailscale serve`）のログオン前の動きと待ち受けの番号に乗っているのに、前提にも問いにも無い
- 成果物: openspec/changes/st55-server-installer/premises.md
- 根拠: docs/requirements.md:987（FR-108 がサービスにした理由）。docs/network.md:4・:15（網へ出すのは `tailscale serve --bg --https=18787 http://127.0.0.1:18787` だけ）。
  premises.md C5 は配るサーバの待ち受けを `18700` にする（network.md:15 の転送先と合わない）。
  Windows の Tailscale は、既定では接続した利用者がログオンしている間だけ動き、ログオン前・ログオフ後も動かすには「無人で動かす」（unattended）の設定が要る —— この席の知識で、確かさ 1（走らせていない）
- kind: premise
- 提案: P に「本人の PC の Tailscale がログオン前に網に繋がっているか」（`tailscale debug prefs` の `ForceDaemon`、または再起動してログオンせずに携帯から `/healthz`）を足す。
  成り立たなければ、インストールの最後に「無人で動かす」と `tailscale serve` の転送先（18700）を示すかを B の問いにする（Tailscale を ashiato が設定するなら範囲を越えるので人間に返す）。
  完了の判定の「ログオンしなくても動いている」を CI で見る項目（Q5 の context）には、この経路が入らないことを書く
- 処置: escalated —— Q6（B。インストーラは Tailscale に触らず、無人で動かすと転送先を示すのが推奨）を足した。premises.md P18

## R4. 本人の切り替え（docs/release.md §7 を ST55 が書き直す）で、いまの収集アプリの資格情報と繋ぎ先が新しいサーバと合わなくなる。ST56 が無い間の渡し方が無い
- 成果物: openspec/changes/st55-server-installer/premises.md
- 根拠: docs/release.md:80-82（「本人の環境を切り替えるのは ST55 が済んでから … ST55 がインストーラの手順に書き直す」）。
  premises.md C7（API の合言葉を乱数で作る）・C5（待ち受け 18700）。いまの収集アプリは組み立ての時の `API_TOKEN` と `127.0.0.1:18787` を使う（requirements.md:1095-1096・crates/collector-windows/src/config.rs:35）。
  設定し直す口（FR-128）は ST56 で、ST56 は ST55 を requires に持たない（docs/stories/ST56.md:5）。合言葉が合わない間、携帯は溜め続けて FR-9 の上限（90 日・2 GB。requirements.md:193）を越えた分を捨てる
- kind: daily
- loss: discarded
- 提案: B の問いを 1 つ足す:「切り替えの日に、いまの `API_TOKEN` を新しいサーバに引き継ぐか（収集アプリはそのまま）／新しく作って APK と PC の収集アプリを作り直すか」。
  推奨と、§7 の書き直しに「収集アプリの繋ぎ先と合言葉を合わせ終えるまで」の手順と、上限の 90 日を書く
- 処置: fixed D18 仮 —— premises.md C18（引き継がない。収集アプリを繋ぐのは ST56 から。いま捨てる記録は無い）

## R5. C10・C5 の「理由を示して止まる」に出口が無い。秘密の欠け・途中で落ちたインストール・合言葉を決めずに閉じた、が族として一覧に無い
- 成果物: openspec/changes/st55-server-installer/premises.md
- 根拠: premises.md C10「DB はあるのに秘密が無い・秘密はあるのに DB が無い、のときは何も書き換えずに理由を示して止まる」・C5「待ち受けが塞がっていたら … 止まる」。
  Q3 の族（deep-questions.json:53 以降）は「決める口・忘れた・変えたい・持ち方・走らせ直し」で、「最後の段で決めずに閉じた」が無い。サーバは画面の合言葉が無ければ起動しない（lib.rs:4371-4381）。
  C3 で本人は置き場に書けないので、欠けた秘密を手で直せない。止まっている間、携帯の未送信は FR-9 の上限で捨てられる（requirements.md:193）。
  upstream_misses の観点「解消するまで止めるには上限を書く」「保存場所が丸ごと消えた／入れ直したを問う」
- kind: daily
- loss: discarded
- 提案: Q3 の族に「最後の段で決めずに閉じた」（C10 の走らせ直しで足りないものを足す、と明記）を、別の族「秘密の欠け」に「DB はあるのに秘密が無い」を足す。
  後者の出口（インストーラが DB の役割の合言葉を作り直す口。費用がある）は B の問いにするか、作らないなら GO 資料に「止まったまま携帯は 90 日で捨て始める」を書く。
  C10 に「秘密を書いてから `initdb` する」順を足し、途中で落ちても「秘密はあるのに DB が無い」側（走らせ直しで足せる）に倒す
- 処置: escalated —— 仮（D10）で閉じていたが、別の席（provisional-review.md）が否とし、第 2 回の Q8（deep-questions-r2.json）で人間に返した。本人の答え（この Story で出口 `ashiato-setup.exe repair` を作る）を design D10・D27 と spec に写した（2026-11-03）

## R6. 状態の置き場が C: の `ProgramData` に固定され、容量の見積もりがどこにも無い
- 成果物: openspec/changes/st55-server-installer/deep-questions.json
- 根拠: requirements.md:1003（FR-109「Windows では `ProgramData` の下」）。NFR-6（:1274-1279。写真の原本は年 6 GB 前後、導入時に端末の過去の画像を一度に取り込む。端末は写真と動画で約 29 GB）・NFR-5（:1272。記号データ年 3〜6 GB）・
  FR-70（:949。拠点内に 30 日分のバックアップ）・書庫の写し（C9。`ASHIATO_ARCHIVE_KEEP_COPIES` の既定は true。archive/config.rs:19-23）が全部同じドライブに載る。
  空きが尽きると D-01 が書けなくなり、携帯は FR-9 の上限まで溜めて捨てる。premises.md・deep-questions.json に容量の語は無い
- kind: daily
- 提案: B の問いを足す:「置き場をインストールの時に選べるようにするか（FR-109 の本文を変える）／`ProgramData` に固定するか（要件どおり）」。推奨と、C: に載る量の見積もり（初年度）を context に書く。
  選べるようにする側を選ぶなら要件が変わるので、そのとき人間に返す
- 処置: escalated —— Q7（B。ProgramData に固定して空きを知らせるのが推奨。選べる側は要件 FR-109 が変わる）を足した

## R7. DB の 3 つの役割（管理者・所有者・実行時）を作り、所有を移す口が `docker compose exec` にしか無い
- 成果物: openspec/changes/st55-server-installer/premises.md
- 根拠: tools/db-roles.sh:7-12（役割を作り、`ashiato_app` から付与を外し、所有を `ashiato_owner` へ移す）・:43（`docker compose exec -T … db psql`）。
  サーバは管理者・所有者の接続では起動しない（lib.rs:4400-4410 `check_db_role`）。premises.md C7 は 3 つの合言葉を作るとだけ書き、誰が役割を作るか・所有を移すかを書いていない
- kind: technical
- 提案: P に「役割の用意は docker の中の psql に依る」を足し、C7 に「インストーラが同梱の `psql` で db-roles.sh と同じ操作をする（台本を 2 つに分けない形は design で）」を足す。
  Windows の CI で、`ashiato_app` が所有者の役割を持たないことと、サーバが `check_db_role` を通って起きることを見る
- 処置: fixed premises.md C7（同梱の psql で役割の用意。CI で check_db_role を通る）

## R8. C2・C3・C11 が依るサービスの振る舞いに前提の行が無い（走らせていない）。仮想アカウントが使えないと、置き場を共有のアカウントが読める側に落ちうる
- 成果物: openspec/changes/st55-server-installer/premises.md
- 根拠: premises.md C2（`pg_ctl register` で DB をサービスにする）・C3（仮想アカウント `NT SERVICE\ashiato-db` / `NT SERVICE\ashiato-server` だけに読み書き）・C11（依存・回復）。
  これらを確かめた P の行は無い。P17（サービスの登録）は 2、P11（サービスのプロファイルの行き先）は「走らせていない」。
  サーバは起動の時に DB へ 1 度繋ぎ、繋げなければ終わる（lib.rs:4396-4399）ので、「DB のサービスが動いている」と「接続を受ける」の間が空くと起動の時に落ちる。
  `pg_ctl register` が仮想アカウントを受けないときに `NetworkService` などの共有のアカウントへ倒すと、同じアカウントで動くほかのサービスが置き場を読める（FR-109 の「サービスのアカウントだけ」が崩れる）。
  upstream_misses の観点「OS のサービス・コマンドに依る決定は、既定・権限・止まっているときを実機で確かめて前提に書く」
- kind: premise
- loss: exported
- 提案: P に 3 行足す（`pg_ctl register -U "NT SERVICE\ashiato-db"` が通り、その SID で `postgres` が置き場に書ける／DB のサービスが RUNNING を告げるのは接続を受けてからか／サーバのサービスの回復と依存）。
  確かさが 4 になるまで、C3 に「仮想アカウントで登録できなければ共有のアカウントへ倒さず止まる」を足す。tasks の Windows の CI の最初の項目にする
- 処置: fixed D3 仮 —— premises.md P19・C3（共有のアカウントへ倒さず止まる）・C11（DB が接続を受けるまで待つ）

## R9. C1 を A にしない判断は妥当。ただし Windows の上で `builtin` の並べ方が効くことは確かさ 2 のままで、黙って OS の地域の設定に倒れたことを見る検査が無い
- 成果物: openspec/changes/st55-server-installer/premises.md
- 根拠: premises.md P4（a）（b）は Linux の PostgreSQL 17 で 4、（c）Windows は公式文書の 2。C1 は後から変えると全部を書き出して入れ直す（rewrite-all）と自分で書いている。
  失わない側（OS に依らない並べ方）が安く、範囲の中で、要件を変えないので、聞かない既定でよい（この席も同じ判断）。
  並べ方のほかに文字の種類の判定（`lower()`・正規表現の `\d` `\s`）も DB の設定に依り、書き出しの座標の除外（crates/server/src/export/excluded.rs:400-410）がそれを使う。C1 の理由は並びだけを挙げている
- kind: premise
- 提案: Windows の CI の試験に、記録を 1 件も入れる前に `pg_database` の `datlocprovider = 'b'`・`datlocale = 'C.UTF-8'`・`encoding = UTF8` と `'a' < 'B'` が偽であることを見る項目を置き、P4（c）を 4 に上げる。
  C1 の理由に「文字の種類の判定（書き出しの座標の除外が使う）も開発の DB と同じ符号の規則になる」を足す
- 処置: fixed premises.md C1（文字の種類の判定も理由に・CI で datlocprovider などを見る）

## R10. Q4 の族「記録を消す: 本人がフォルダを消す」が C3（本人は読むだけ・管理者にも付けない）と合わない
- 成果物: openspec/changes/st55-server-installer/deep-questions.json
- 根拠: deep-questions.json:95（「消すのは本人がフォルダを消す。示すだけ」）。premises.md C3（継承を切り、サービスのアカウントに読み書き・本人に読みだけ。`Users` は付けない。「管理者は所有権を取れば読める」）・P6。
  FR-109（requirements.md:1003）「サービスのアカウントだけが書け」
- kind: premise
- 提案: 族の「記録を消す」の decided を「管理者として所有権を取ってから消す（手順を示す）」に直す。消す口から記録まで消せる案を足すなら、それは `discarded` のある A になるので別の問いにする
- 処置: fixed deep-questions.json Q4 の族（所有権を取ってから消す手順を示す）

## R11. Q3 の案 3 に、先に決めたほかのプログラムが何を得るかが書かれていない
- 成果物: openspec/changes/st55-server-installer/deep-questions.json
- 根拠: deep-questions.json:82-83（「決めるまでの間は、同じ PC のほかのプログラムが先に決められる」）。先に決めた者は画面にログインでき、すべての記録を読める。
  同じ PC で第三者の衛星が動くことは設計の前提（docs/production-prep.md:194）
- kind: irreversible
- loss: exported
- 提案: 案 3 の detail に「先に決めたプログラムは、本人が気づくまで画面から全部の記録を読める（読まれた記録は戻らない）」を足す。推奨は案 1 のままでよい（失わない側が安い）
- 処置: escalated —— Q3 の案 3 の detail に、先に決めたプログラムが全部の記録を読める（戻らない）ことを書いた。人間が Q3 で選ぶ

## R12. C8（画面の合言葉は照らすための値だけを持つ）が、走っている ST44 の D8（仮）の「印の中身は変えない」と食い違う
- 成果物: openspec/changes/st55-server-installer/premises.md
- 根拠: crates/server/src/web_session.rs:98-105（印は `"ashiato-web-session-v1" || WEB_PASSWORD` の平文の上の HMAC）・:171-175（サーバは平文を持って照らす）。
  openspec/changes/st44-collector-credential-scope/design.md:83「HMAC の内容（`"ashiato-web-session-v1" || WEB_PASSWORD`）は変えない」。C8 では平文を持たないので、印は照らすための値の上で作り直すことになる
- kind: technical
- 提案: C8 に「印は照らすための値から作る（ST44 の D8 の内容を変える。ログインが 1 回切れるだけで失わない）」を足し、ST44 の provisional.jsonl の D8 に印を付ける
- 処置: fixed premises.md C8（印は照らすための値から作り直す。ST44 へは差し戻さない）

## R13. Q5 の推奨（サンドボックスで UAC が 1 回出るのを録画する）が、サンドボックスの UAC と「本人」の形を確かめていない
- 成果物: openspec/changes/st55-server-installer/premises.md
- 根拠: deep-questions.json:127。premises.md P16 は「入っている」と「再起動の後に状態が残るか（1）」だけで、サンドボックスの利用者が管理者で UAC の確認が出るか、C3 の「昇格する前の本人の SID を控える」とほかのアカウントの読みをそこで確かめられるかは書いていない
- kind: premise
- 提案: P16 に「サンドボックスの利用者の種類と UAC の出方」を足し、確かめるまで Q5 の案 1 の detail に「UAC の確認が出ないなら、録画で見るのは合言葉を決めてログインするまで」を添える
- 処置: fixed deep-questions.json Q5 の案 1 の detail・premises.md P16

## R14. 申し送り C6（正典 2 つの既定の置き場の MODIFIED と印つきの試験）が聞かない既定に写っていない
- 成果物: openspec/changes/st55-server-installer/premises.md
- 根拠: docs/handoff/ST55.md:15（`openspec/specs/data-durability/spec.md:26`・`:114`、`openspec/specs/device-collection/spec.md:1867`・`:1896`、`export_tests.rs:73`・`media_tests.rs:211`）。premises.md C9・C14 は置き場の並びを書くが、正典の既定（`%LOCALAPPDATA%`）を書き直すことには触れていない
- kind: defer
- 提案: C14 に「正典の 2 つの Requirement を MODIFIED で FR-109 の置き場へ（印つきの試験も）」を 1 行足し、spec の段へ送る
- 処置: fixed premises.md C14（正典 2 つの MODIFIED と印つきの試験を spec の段へ）
