# ST54 deep-review（問いの独立検査）

対象: `openspec/changes/st54-dev-side-by-side/deep-questions.json`（Q1〜Q9）と `premises.md`（P1〜P14・C1〜C4）。
手順 1〜8 を問いの一覧を見ずにやり直し、そのあとで一覧と突き合わせた。

## 手順ごとの結果（要約）

- 手順 1（要件どうしの衝突）: FR-106 × FR-83（除外の登録。R2）、FR-106 × FR-30（収集した記録は書き換えない。開発版の収集アプリの送り先。R1）、ST54 の完了の判定 × ST52 Q5 の本人の答え（開発版の公開は ST53 の後。R8）
- 手順 2（扉の幅）: 扉 34 は識別子と「別の鍵」を決めているが、鍵の形（RSA 4096・25 年以上）は ST52 D3（仮）のまま。ST54 で鍵が初めて外に出る（R7）。FR-106 の「開発版の S-01 は別の D-01」は収集アプリの送り先を縛っていない（R1）
- 手順 3（一方通行）: 開発版の鍵で署名した APK の公開（exported。R7）、正式版の追記のみの D-01 に開発版の記録が入ること（R1）、除外した本文が開発版の D-01 に入ること（R2）、開発版の DB の作り直し（Q4。R4・R5・R6）
- 手順 4（日常）: 携帯に常駐の通知が 2 つ・位置の取得が 2 本、PC の開発版の初回の履歴の全件取り込み（R11）、同じ host の 2 つの画面がログインのクッキーを取り合う（R10）
- 手順 5（既存コード）: `autostart.rs:17`・`toast.rs:17,37` の固定の名前は Q1・Q7 が受けている。`main.rs:63-77`（二重起動の印）と `main.rs:81`・`exclusion.rs:112`（除外の登録が無ければ空）は一覧に無い（R2・R3）。`nightly.yml` の APK 無し・説明文は C3・Q8 が受けている
- 手順 6（過去の取りこぼし）: 「DB や写しを増やす決定に、本物の記録が入るか・削除が届くか」（R1・R2）、「完了の判定がまだ起きていない本物の操作に乗る」（R8）、「handoff の全項目を突き合わせる」（R4・R7）、「保存場所が丸ごと消えた／入れ直した」（R7 の鍵を替えると開発版の携帯は入れ直し）が当たる。時計の族・画面の寸法・検証コマンドの族はこの段の問いに当たるものが無い（確かめた範囲: Q1〜Q9 の選択肢に時刻を入力にする決定・寸法の Scenario が無い）
- 手順 7（族）: Q1・Q2 の「並べた 2 つが同じ名前の資源を取り合う」族の兄弟（実行ファイルの名前・移行の前の退避の置き場・開発版の D-01 の器とポート・画面のクッキー）が一覧に無い（R10）
- 手順 8（前提）: Q4 が載る P7・P9 は確かさ 3 で、premises の冒頭の「確かさ 4 以上にだけ載せる」と合わない（R6）。Q4 の context の「ほとんどは正式版の DB にもある」に前提の行が無い（R6）。鍵の形の前提が無い（R7）

---

## R1. 開発版の収集アプリが**どのサーバへ送るか**が問われていない。Q2 の推奨のまま「正式版と同じ設定」を渡すと、開発版の記録が正式版の追記のみの D-01 に入る
- 成果物: openspec/changes/st54-dev-side-by-side/deep-questions.json
- 根拠: docs/requirements.md:974-975（FR-106 が別の D-01 を求めるのは「開発版の S-01」だけで、開発版の C-01 / C-02 の送り先は書いていない）/ docs/requirements.md:403（FR-30 収集した記録は書き換えない）/ crates/collector-windows/src/config.rs:104-127（`ASHIATO_BASE_URL` は既定なしで本人が渡す）/ deep-questions.json Q2 の選択肢 1 の detail「正式版と同じ設定を渡しても取り合わない。開発版のサーバに届く端末識別子は `pc-01-dev`」（同じ設定なら `ASHIATO_BASE_URL` も正式版のサーバで、届く先は開発版のサーバではない）/ docs/handoff/ST54.md の ST51 の節（「同じ端末識別子で交互に信号を送ると版が行き来したように見える」は、2 つが同じ S-01 に送るときにしか起きない。別の D-01 なら混ざらない）
- kind: irreversible
- loss: rewrite-all
- 提案: 問いを 1 つ足す:「開発版の収集アプリは、正式版のサーバに送ってよいか（開発版のサーバにだけ送る / 正式版にも送ってよい）」。正式版に送ると、開発版の不具合が作った記録が `pc-01-dev` などの端末で正式版の D-01 に凍結され、外すには凍結した行の書き直しになる。稼働状況の格子に開発版の端末が並び、外すと途絶として残る。開発版の封筒の版が正式版のサーバより新しいと FR-125 で手元に残り続ける。「開発版にだけ送る」なら仕組み（開発版は開発版のサーバかを確かめて送る等）を選択肢に書く。Q2 の detail と why の ST51 の文は、この答えに合わせて直す
- 処置: escalated —— Q6（開発版の収集アプリの送り先。A・loss rewrite-all、推奨「開発版のサーバにだけ送る」）を足した。Q2 の detail から「同じ設定を渡しても」の送り先の含意を外した

## R2. Q2 の推奨（開発版は置き場の下に `dev` を掘る）は、正式版で登録した**除外が開発版に効かない** —— FR-83 と衝突し、除外した題名・URL が開発版の D-01 に入る
- 成果物: openspec/changes/st54-dev-side-by-side/deep-questions.json
- 根拠: crates/collector-windows/src/main.rs:81（除外の登録は `state_dir/exclusions.json` から読む）/ crates/collector-windows/src/exclusion.rs:112（ファイルが無ければ `Self::default()` = 除外なし）/ docs/requirements.md:292-299（FR-83「除外する対象が登録されている THE SYSTEM SHALL その対象の本文を記録せず」）/ deep-questions.json Q2 の選択肢 1「置き場の下に `dev` を掘り」/ ST52 以前の upstream_misses「DB や写しを増やす決定に、その写しに本物の記録が入るか・削除が届くかを確かめる」（ST48 Q6）
- kind: conflict
- loss: exported
- 提案: Q2 に「除外の登録をどうするか」を足す（開発版も正式版の除外を読む / 開発版は除外を写してから始める / 除外が無ければ開発版は起動しない など）。置き場を分けると、除外のほかに履歴の取得の印（`browser-history/last_success.json`）も新しくなり、開発版が初回の全件の取り込みをする（R11）。一度記録して送った本文は、開発版の D-01 から消す口が無い（本文ごと消す口は正式版の画面の操作だけ）。携帯も Android が置き場を分ける（C1）ので、携帯の収集アプリに本人が登録した設定があれば同じ族として並べる
- 処置: fixed deep-questions.json —— Q2 の推奨を「除外の登録は正式版の置き場のものを読む」に直し、選択肢 2・3 に除外が効かない失うものを書いた。初回の全件の取り込みは context に書いた

## R3. Q2 の why「どの選択肢でも取り合いは防げ、作り直せば戻る」が誤り。選択肢 3 は防がず、同じ置き場では正式版の PC の収集アプリが**起動しない**
- 成果物: openspec/changes/st54-dev-side-by-side/deep-questions.json
- 根拠: crates/collector-windows/src/main.rs:63-77（二重起動の止め。置き場の印が新しければ 2 つ目は `already_running` で何もせず終わる）/ deep-questions.json Q2 の選択肢 3 の detail「同じ値を渡すと正式版の未送信が開発版のサーバへ流れる」/ Q4 の推奨（開発版の DB は作り直してよい）
- kind: premise
- loss: discarded
- 提案: why の文を「選択肢 3 では、同じ置き場を渡すとログオンで先に立った方だけが動き、もう一方は黙って終わる。開発版が先に立つと、正式版の未送信と以後の記録が開発版のサーバへ送られ、Q4 の作り直しで消える」に直す。選択肢 3 に `irreversible` を書く（失わない側の選択肢 1・2 が安いので、問いは B のままでよいが、選択肢 3 を「作り直せば戻る」と並べない）
- 処置: fixed deep-questions.json —— Q2 の why を二重起動の止めの振る舞いに直し、選択肢 3 に irreversible を書いた

## R4. Q4 の選択肢に、ST52 R11 の申し送りが書いた「DB を書き換えず次の版で直す」側が無い。選択肢 2 の `irreversible` は失うものでなく費用
- 成果物: openspec/changes/st54-dev-side-by-side/deep-questions.json
- 根拠: docs/handoff/ST54.md の「ST52 R11」の節（「不整合を検出して原因を記録し、無理に DB を書き換えず次の版で直す手順を組み込む」）/ deep-questions.json Q4 の選択肢 1（DB を作り直し、記録を捨てる）・選択肢 2（nightly の移行も凍結）/ crates/server/src/migration_ledger.rs:178-189（`Ahead` は名前の集合の比べで決まる）
- kind: irreversible
- 提案: 選択肢を 1 つ足す:「止まった開発版の DB はそのまま置き、main 側で直す（名前を変えた・消した移行は台帳に旧名の対応を残す・中身の直しは直しの移行を開発版の DB にも当てる）。直るまで開発版のサーバは止まったまま」。失わない側で、選択肢 2 より安い（凍結の検査を変えず、ST52 Q10 の本人の答え「nightly は配った印に数えない」も変えない）。選択肢 2 の `irreversible`「直しの移行が DB に残り続ける」は 4 つの loss のどれでもない（行は失われない）ので、`detail` の費用に移す。選択肢 2 は ST52 Q10 の本人の答えと正典 distribution:197・635 を変えることも detail に書く
- 処置: fixed deep-questions.json —— Q4 に「DB は書き換えず main 側で直す」を足して推奨にした。凍結の選択肢の irreversible は detail の費用へ移し、ST52 Q10 と正典を変えることを書いた

## R5. Q4 の context の頻度（「中身を直した commit が 42。多くは merge の前の直し」）は、開発版の DB が実際にずれる回数を示していない。main の上での直しは 9 回、うち 4 回は 10-27〜10-31
- 成果物: openspec/changes/st54-dev-side-by-side/deep-questions.json
- 根拠: `git log --first-parent main --diff-filter=M -m -- 'migrations/*.sql'` は 9 commit（926ccb71 2026-10-31・90ce5b0f 10-30・af7c4542 10-29・5688f2e7 10-27 はどれも確認バッチの merge で、`202610231200_event_end_time.sql`・`202610251000_record_component_version.sql`・`202609271716_deletion_ledger.sql`・`202609120943_version_and_ledger.sql` を書き換えた）/ premises.md P8（42 は枝の commit も数えている）
- kind: premise
- 提案: context の数を「main に入った後に中身を直したことは 9 回、直近の 1 週間で 4 回（確認バッチの直し）」に直し、P8 に `--first-parent` の数を足す。Q4 の推奨（ずれたら止めて作り直す）を採ると、いまの頻度では開発版の DB を週に 1 回ほど作り直すことになる —— 人間が選ぶ材料なので、選択肢 1 の detail に書く
- 処置: fixed deep-questions.json —— Q4 の context と P8 を main に入った後の 9 回（うち 4 回は 10-27〜10-31）に直し、作り直しの選択肢に「週に 1 回ほど」を書いた

## R6. Q4（loss あり）が確かさ 3 の前提に載っている。premises の冒頭の「確かさ 4 以上にだけ載せる」と合わない。Q4 の context の「ほとんどは正式版の DB にもある」に前提の行が無い
- 成果物: openspec/changes/st54-dev-side-by-side/premises.md
- 根拠: premises.md 冒頭「`loss` のある問い（Q4）は確かさ 4 以上の前提（P6・P7）にだけ載せる」と P7 の確かさ 3（コードを読んだだけ）/ P9 の確かさ 3（Q4 の context と選択肢 2 が載る）/ 走らせる手段がある: `crates/server/tests/server_startup.rs:748` `st48_migrate_fingerprint_mismatch_warns`・`:1187` `st48_server_starts_with_fingerprint_warning`、`bash tools/check-released-migrations.sh --self-test`（2026-11-02 にこの席で走らせ rc=0、`Scenario: 開発版の印は、配った印に数えない` を通った）/ premises.md P14（携帯の配布物は正式版も開発版も ST56 まで送らない）
- kind: premise
- 提案: P7 を上の 2 本の試験で、P9 を `--self-test` で走らせて 4 にする。Q4 の context の「開発版の DB の記録のほとんどは正式版の DB にもある」を前提の行にし、成り立つ条件（開発版の収集アプリが開発版のサーバにだけ送る —— R1 の答え、正式版の収集アプリも並べて動いている）と、いまは携帯の記録がどちらの D-01 にも届かない（P14）ことを書く。成り立たなければ選択肢 1 の `irreversible` の範囲が広がる
- 処置: fixed premises.md —— P9 を --self-test rc=0 で 4 にした。Q4 の失うものの根拠は P6（4）とし、P7・P16 は補足で根拠にしないと冒頭に書いた。P16（導出・2）を足し、Q4 の context は条件つきの文にした。P7 は DB の要る試験で、ここでは走らせていない（3 のまま）

## R7. 申し送り「開発版の鍵を作る前に、形と作る手順を確かめる」が一覧にも premises にも無い。ST52 で D3（仮）を確かとした理由（ST52 は鍵を作らない）は ST54 で切れる
- 成果物: openspec/changes/st54-dev-side-by-side/premises.md
- 根拠: docs/handoff/ST54.md の「ST52 Q4・Q11」の節（「開発版の鍵は一度配ると替えられない…ST54 で鍵を作る前に、形（ST52 の design D3（仮）の RSA 4096・25 年以上）と作る手順を確かめる」）/ openspec/changes/archive/2026-11-02-st52-release-channels/provisional-review.md:13-15（D3 を確かとした根拠は「ST52 の間は何も出ない（exported は起きない）」）/ 同 premises.md:25 P18（鍵の条件の一次情報は確かさ 2）/ tools/release/make-key.sh:37（`-keysize 4096 -validity 10000`）/ docs/requirements.md:1769-1778（扉 34。替えると入れ直し）/ premises.md の P1〜P14 と C1〜C4 に鍵の形・作る手順の行が無い（`grep -n "make-key\|4096\|D3" premises.md` は 0 件）
- kind: irreversible
- loss: exported
- 提案: premises に「開発版の鍵は `make-key.sh dev` で RSA 4096・10000 日」の行を足し、`tools/release/make-key.sh --self-test` を走らせて 4 にする。開発版は Play へ出さないなら P18 の条件は効かないことも書く。そのうえで形を C（聞かずに決める。D3 と同じ）に置くか、形に選ぶ幅があれば A の問いにする。鍵を替えると開発版を入れた携帯は消して入れ直しで、送らない間（ST56 まで）に手元に溜めた記録が全部消える（`discarded` も伴う）ことを書く
- 処置: fixed premises.md —— P15（make-key.sh --self-test rc=0・RSA 4096・10000 日）を足した。鍵を作る時と確かめ方は Q9（A・loss exported）で問い、鍵を替えると手元の未送信が消えることを Q9 の context に書いた

## R8. 完了の判定（「開発版のリリースに『サポートしない』と書いてある」）と Q9 の推奨は、まだ起きていない本物の公開に乗る。ST52 Q5 の本人の答え（開発版の公開は ST53 の後）と衝突する。Q9 は失う側があるのに open
- 成果物: openspec/changes/st54-dev-side-by-side/deep-questions.json
- 根拠: docs/stories/ST54.md の完了の判定 3 / openspec/specs/distribution/spec.md:777（公開の設定が入っていなければ予定の実行は何も公開しない）/ openspec/changes/archive/2026-11-02-st52-release-channels/deep.md Q5（本人の答え:「開発版の公開は ST53 の後に本人が main に打って始める」）/ docs/stories/ST54.md の `requires: [ST52]`（ST53 を待たない。`ls openspec/changes/archive | grep st53` は 0 件）/ 同 archive の design.md:183-（D22。ST52 は同じ型を「下書きのリリースと使い捨ての鍵で確かめる」と本人が決めた。upstream_misses の族「ST52 の完了の判定が、まだ起きていない本物の操作に乗る」）/ deep-questions.json Q9 の推奨「本人が鍵を置いて開発版を 1 回公開した後に確かめる」（本物の開発版の鍵で署名した APK が外に出る）
- kind: conflict
- loss: exported
- 提案: 問いを 1 つ足す（または Q9 を広げる）:「ST54 の完了を何で見るか —— ST52 D22 と同じく下書きのリリースと使い捨ての鍵で見る / ST53 の後に本人が公開を始めてから見る（ST54 の merge が ST53 を待つ）」。後者は本物の開発版の鍵で APK を公開するので、Q9 の `kind` を `irreversible`（`loss: exported`）にし、R7 の鍵の形の確かめを先に置く。完了の判定 3（「サポートしない」の文）も同じ問いで扱う
- 処置: escalated —— 旧 Q9 を「完了を何で確かめるか」（A・loss exported、推奨は ST52 D22 と同じ下書きのリリースと使い捨ての鍵）に組み直した

## R9. Q6 は不要。正典と FR-104 が、鍵が無いときの答え（公開せず前の開発版を残す）をすでに決めている。選択肢 2 は要件の本文を変える
- 成果物: openspec/changes/st54-dev-side-by-side/deep-questions.json
- 根拠: openspec/specs/distribution/spec.md:633（「配布物が 1 つでも組めなければ公開しない（…開発版は前の版のリリースを残す）」）/ 同 :759（Scenario「鍵が置き場に無ければ、署名しない APK を公開しない」）/ docs/requirements.md:969（FR-104 ★ 2026-11-01「ST54 が開発版の C-01 を足した時点で、開発版も 3 部品になる」。選択肢 2 の「3 つで出す」はこの文と食い違う）/ tools/release/sign-apk.sh:41-43（指紋が無ければ署名しない）
- kind: technical
- 提案: Q6 を一覧から外し、design に「鍵が無ければ作り直しを落とし前の開発版を残す。落ちた理由に『開発版の鍵が無い』を出す」を正典から導いた決定として置く（C3 と同じ扱い）
- 処置: fixed deep-questions.json —— Q6（鍵が無い間）を外し、premises の C5 に正典から導いた決定として置いた。Q6 の番号は送り先の問いに使った

## R10. Q1・Q2 の族（並べた 2 つが同じ名前の資源を取り合う）の兄弟が一覧に無い: 実行ファイルの名前・移行の前の退避の置き場・開発版の D-01 の器とポート・画面のクッキー
- 成果物: openspec/changes/st54-dev-side-by-side/deep-questions.json
- 根拠:
  - 実行ファイル: tools/release/build.sh:63-77（PC の収集アプリの zip の中身は正式版も開発版も `ashiato-collector-windows.exe`）/ crates/collector-windows/src/autostart.rs:58-62（自動起動の `.cmd` は `current_exe()` の道を書く）。同じ場所に展開すると正式版の自動起動が開発版の実行ファイルを起こす。Q1 の推奨（開発版かは組み立ての版番号で決まる）と合わせると、正式版の `.cmd` の設定（正式版のサーバ）のまま開発版として動き、R1・R2 の経路に入る
  - 移行の前の退避: crates/server/src/migrate_backup.rs:6・30（`MIGRATE_BACKUP_DIR` の既定は作業ディレクトリからの相対 `backups/pre-migrate`）。同じ場所から起こすと 2 つの DB の退避が 1 つの置き場に混ざり、Q3 の why の「退避から戻す」で取り違えうる
  - D-01 の器: docker-compose.yml の `ports: 127.0.0.1:${ASHIATO_DB_PORT:-55432}`（既定は正式版と同じ）/ tools/ports.sh:52-60（作業ツリーは DB 55500+n・サーバ 19000+2n、確認バッチは 55499・18787）。C2 が決める開発版の既定のポートと、開発版の D-01 の置き場（器・ポート・データの置き場。FR-109）はこの割り当てと被らないことが要る。Q3 は守りだけを問い、別の D-01 をどう立てるかは一覧にも C にも無い
  - クッキー: crates/server/src/web_session.rs:28・196（`ashiato_session`、`Domain` 無し・`Path=/`）。クッキーはポートで分かれない（RFC 6265 §8.5）ので、同じ host の正式版と開発版の画面は 1 つのクッキーを上書きし合い、片方にログインするともう片方がログアウトする
- kind: daily
- 提案: 失うものは無い（実行ファイルの上書きは R1・R2 の答えしだい）ので、B として推奨つきで足すか、C に並べる: 開発版の実行ファイル名に `-dev` を付ける・開発版の `MIGRATE_BACKUP_DIR` の既定を分ける・開発版の D-01 の器とポートを `tools/ports.sh` の範囲の外に決める・開発版のクッキー名を分ける。クッキーの振る舞いは外の形式なので premises に行を足す（確かさは実物のブラウザで 2 つを開いて上げる）
- 処置: fixed premises.md —— C6（実行ファイル名・退避の置き場）・C7（クッキー名。P17 を足した）・C8（DB の器とポート）に置いた

## R11. 日常に効く選択が一覧に無い: 携帯は常駐の通知が 2 つ・位置の取得が 2 本になり、PC の開発版は初回に履歴を全件取り込む
- 成果物: openspec/changes/st54-dev-side-by-side/deep-questions.json
- 根拠: collector-android/app/src/main/AndroidManifest.xml:19-29（`FOREGROUND_SERVICE_LOCATION`・`FOREGROUND_SERVICE_SPECIAL_USE`。開発版も同じ常駐を立てる）/ premises.md P1・C1（識別子だけを変え、全部の収集をそのまま持つ）/ crates/collector-windows/src/main.rs:117（開発版も履歴を読む）と置き場を分けたときの `browser-history/last_success.json` の不在（R2）/ `python3 scripts/upstream_misses.py report` の 2026-10-17-st08-browser-history Q9（初回の取り込み 15 万件で見回りが 148 秒止まった）
- kind: daily
- 提案: B を 1 つ足す:「開発版の携帯の収集アプリは、正式版と同じ全部の収集を常に動かすか（電池と常駐の通知が 2 倍）/ 本人が選んだ収集だけを動かすか」。PC は「開発版の初回の履歴の取り込みの量」を detail に書く。Q7（見分け方）の推奨で常駐の通知の題も分かるかを確かめる
- 処置: fixed deep-questions.json —— Q10（開発版の携帯が動かす収集。B）を足し、Q7 の context に常駐の通知を、Q2 の context に初回の全件の取り込みを書いた

## R12. Q3 の why「開発版のサーバが正式版の DB に繋がると、main の新しい移行を当てて先に進めます」はサーバの振る舞いと合わない。先に進めるのは `migrate` だけ
- 成果物: openspec/changes/st54-dev-side-by-side/deep-questions.json
- 根拠: openspec/specs/distribution/spec.md:168（Scenario「まだ当たっていない版のある DB では、サーバは起動しない」）/ crates/server/src/lib.rs:4412-4417（起動の前の版の検査で断ると終了コード 3）/ crates/server/src/lib.rs:386-401（移行を当てるのは `migrate` の道）/ docs/requirements.md:1076-（FR-129。起動の台本が移行を当てるのは台本を使ったとき）
- kind: premise
- 提案: why を「開発版の `migrate` を正式版の DB に向けて走らせると、main の新しい移行を当てて先に進めます（サーバを起こすだけなら、当たっていない版があるので起動しません）」に直す。選択肢 1 の「最初に移行を当てたサーバが印を書く」は、印を見て止まる場所に `migrate` を含めると detail に書く
- 処置: fixed deep-questions.json —— Q3 の why を「`migrate` が先に進める。サーバは起動しない」に直し、選択肢 1 の detail に `migrate` も印を見ることを書いた

