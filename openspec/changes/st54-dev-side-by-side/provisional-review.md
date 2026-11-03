# ST54 仮で進めた決定の確かめ

## D5
- 判定: 否
- 根拠: 条件 4（`rests_on` に土台の決定が抜けている）。条件 1〜3 は満たす。
  - 条件 4（否）: D5 の検査は**動く**印 `nightly` 1 つを比べの基準にし、取り寄せも動く印を前提にした `git fetch --force --no-tags origin '+refs/tags/nightly:refs/tags/nightly'` にしている（design.md:81-83、tasks.md:82-83「`git show nightly:migrations/ORDER`」）。この形は Q5 の答え（「動かす方式のまま」。deep-answers-1.txt の Q5）に依っている。Q5 が版ごとの印に変われば、基準の印の選び方と取り寄せ方が変わる。それなのに `rests_on` は `["D4", "Q4"]` だけで、Q5 が載っていない（provisional.jsonl の D5 の行）
  - 条件 1（満たす）: 止める側は CI で PR を落とすだけで、DB にも記録にも触らない。直しの移行と `migrations/LINEAGE` の 1 行を足せば通る（design.md:84）。`uncaptured` / `discarded` / `exported` / `rewrite-all` のどれも起きない
  - 条件 2（満たす）: 台本 1 本と CI の 1 手順。`rust` の job はもう `fetch-depth: 0` で（`.github/workflows/ci.yml:71`）、`check-released-migrations.sh` の隣に置ける（ci.yml:97-98）。要件の本文（requirements.md:635「凍結の検査は `nightly` を数えない」・正典 distribution:197）は変えない。凍結の検査とは別の台本で、`rewritten` を足せば中身を変えられるので、Q4 の選択肢 3（凍結）にはあたらない。直しのたびに移行が 1 つ増える点は、本人が選んだ Q4 の選択肢 1 の detail（「中身の直しは直しの移行を足して開発版の DB にも当てる」）にもう含まれている
  - 条件 3: `follows` は null。止めの向きは Q4 の答え（「DB は書き換えない。main 側で直す」）と同じ向き
  - 付記（否の理由ではない）: 中身の比べはバイト列なので、コメントだけの直しでも `rewritten` と直しの移行（実質は何もしない移行）が要る（design.md:85）。その直しの移行も後で `v*` に載って凍結される

## D7
- 判定: 否
- 根拠: 条件 1（失わない側でも `discarded` が起きる）。
  - 携帯の収集アプリの未送信には保持の上限があり、積んでから 90 日を超えた記録と、2 GB を超えた分を古い順に捨てる。捨てるときに送れない理由は見ない（`collector-android/app/src/main/kotlin/dev/ashiato/collector/Retention.kt:16-18`「90 日 / 2 GB」、:42「送れない理由を見ない」、:47「捨てた記録はその場で破棄の報告に数える」）。D7 で止めた記録（記録・生存信号・破棄の報告・原本の欠け・原本を何も送らない。design.md:105-106）も、この上限で捨てられる。なので D7 の `keeps`「送らなければ未送信は手元に残り、送り先を直せばそのまま送られる」は携帯では 90 日までしか成り立たない
  - 気づく手がかりは、上限の 7 日前に 1 回だけ鳴る知らせ（Retention.kt:19 `alertAgeMs` 83 日）と `Log.w` だけ。D7 は系統の違いを ST50 の「サーバを更新してください」の通知に数えない（design.md:105-106、spec の Scenario）。PC の収集アプリは未送信に上限を持たない（`crates/collector-windows/src/outbox.rs:4`「上限も破棄も持たない」）ので、PC だけなら失わない
  - 止めない側では、逆向き（正式版の収集アプリ → 開発版のサーバ）の記録は開発版の DB に入る。どちらを選んでも、送り先を取り違えると失うものがある。片方は 90 日を過ぎると捨てる（`discarded`）、もう片方は正式版の DB に入らず開発版の DB にしか残らない。両側に失うものがあるので、逆向きの止めは A（人間に問う）にあたる。Q6 は開発版の収集アプリの向きだけを聞いていて、Q6 の選択肢 1 の detail にも 90 日の上限は書かれていない（deep-questions.json の Q6）
  - いまは配布物の APK が送らない（premises P14。ST56 まで）ので、今日は起きない。ST56 で送り先を入れる口ができた時点から起きうる
  - 条件 2〜4: 費用（見出し 1 つと、2 つのゲートに比べを 1 つずつ）は小さい。答えの本文は変えず見出しで運ぶので、正典 record-envelope:1055 の答えの形は変わらない。`follows` は null。`rests_on` の D1・Q6 で足りている

## D8
- 判定: 否
- 根拠: 条件 4（`rests_on` に土台の決定が抜けている）。条件 1〜3 は満たす。
  - 条件 4（否）: どの既定も系統で選ぶ（`net_guard::default_bind(channel)`・`web_session::cookie_name(channel)`・`migrate_backup` の既定を系統で決める。tasks.md:44。実行ファイルの名前は「`tools/release/build.sh` が版番号で決める」。design.md:120）。これは D1（系統は組み立てのときの版番号で決める。Q1 の答え）に依っている。それなのに `rests_on` は `[]`（provisional.jsonl の D8 の行）。D1 が変われば（たとえば実行時の環境変数にする）、どの関数の引数の出どころも変わる
  - 条件 1（満たす）: ポート・クッキーの名前・退避の置き場・DB の器の project 名・実行ファイル名は、どれも並べた 2 つが互いを上書きしないために分ける側。分けたせいで捨てるもの・出すものは無い。数値は割り当ての外にあることを確かめた。`tools/ports.sh:52-62` の割り当ては DB が 55432・55499・55500+NN・55600+n（n<100）、サーバが 18787・19000+2×NN・19200+2×n なので、55433 と 18788 は被らない。`docker-compose.yml` は名前の付いた volume を宣言していない（28 行に `volume` / `name:` が無い）ので、`-p ashiato-nightly` で器のデータの置き場も分かれる。退避の既定 `backups/pre-migrate` を固定で読むものは、ほかに無い（`grep -rln pre-migrate` でヒットしたのは `migrate_backup.rs`・`lib.rs:386` の案内文・`tests/server_startup.rs:676` のファイル名の頭だけ。`docs/handoff/ST58.md` に `backups` は無い）
  - 条件 2（満たす）: 系統で選ぶ関数 3 つと台本の名前 1 つ、手順書の数行。要件の本文は変えない
  - 条件 3: `follows` は null

## D12
- 判定: 確か
- 根拠: 条件 1〜4 を満たす。
  - 条件 1（満たす）: 選んだ側は知らせを足すだけで、送る・捨てる・書き換える処理に触れない。ST50 の出し直しの仕組み（`ContractNotifier.kt:62-77`。24 時間ごとに出し直し、状態を抜けたら消し、出した時刻は `ContractWaitStore`（`ContractGate.kt:76-121`、prefs `contract-wait`）に持つ）は通知と prefs のキーを書くだけ。PC も同じで（`contract_notice.rs:5-6,17,37-`）、札で分ける口はもうある（`toast.rs:19-22,70-96` `build_toast_tagged` / `show_tagged`）。別の札と id を使うので（tasks.md:110,128。携帯の id 5 は常駐 1〜写真 4 と重ならない。`ContractNotifier.kt:108-109`）、ST50 の通知や出した時刻を上書きしない。失う側は「1 回だけ」のほうで、通知を払うと次に気づけるのは 83 日目の 1 回だけになり、90 日で `discarded` が起きる（`Retention.kt`、監査 C5）。`uncaptured` / `discarded` / `exported` / `rewrite-all` のどれも、選んだ側では起きない
  - 条件 2（満たす）: 費用は、決め方（`contract_notice::decide`・`ContractNotifier.update`）を札と id を替えてもう 1 つ持つことと、7 日前の知らせへ添える 1 枝（`contractHeadlineFor`。`ContractGate.kt:222`、`LocationService.kt:663`）。要件の本文は変えない。FR-106 ★（`docs/requirements.md:982`）は「ST50 の知らせと同じ口 —— 携帯は通知、PC は通知の差出人」としか言わず、出し直しの間隔は監査 C5 が spec の段に回し、仮で決めるよう指示している（`docs/audit/2026-11-03-st54-q12-consistency.md` C5「直すところ」）。携帯の画面の行は要件に足すだけで、要件と食い違わない
  - 条件 3: `follows` は null。向きは Q13 の答え（「知らせる（ST50 の知らせと同じ形で…）」。deep-answers-5.txt）と同じで、選択肢の説明の「1 回」より知らせを強める側
  - 条件 4: `rests_on`（Q12・Q13・D7）で足りている。知らせを出す契機は D14（`Version` の答えからだけ系統を読む）でも決まるが、D14 が変わっても変わるのは契機の数だけで、D12 で決めた口と間隔は変わらない
  - 付記（否の理由ではない）: 再起動の後は `ContractGate.decided` が偽なので、7 日前の知らせは prefs の写し（`store.notice`）を読む（`ContractGate.kt:222-223`）。系統の違いの写しも prefs に持たないと、再起動の後の 7 日前の知らせに文が添わない。tasks.md:128 には「前に出した時刻を手元に持つ」とだけあり、画面の行と 7 日前の知らせのための写しをどこに持つかは書かれていない

## D13
- 判定: 否
- 根拠: 条件 4（`rests_on` に土台が抜けている）と、`keeps` の根拠の事実誤り。
  - 条件 4（否）: D13 が「止めなくてよい」とする理由は「系統の違いは D12 の知らせで本人に届く」（provisional.jsonl の D13 の `keeps`、design.md:178）と、「系統の違いの知らせが出ているので本人は気づける」（design.md:179）。どちらも Q13（知らせる）と D12（出し直しの間隔）に依っている。それなのに `rests_on` は `["Q12", "D7"]` だけ。D12 の反転条件（design.md「24 時間ごとの出し直しがうるさいと本人が言ったら、…1 回だけにする」）で D12 が変わると、誤った途絶の知らせに本人が気づく手がかりが弱まる。それでも D13 を見直す道が台帳に無い
  - `keeps` の事実誤り: 「読む口はサーバに何も書かず」（provisional.jsonl の D13、design.md:178）はコードと合わない。`GET /outages` は判定と同時に、相手のサーバの `core.outage_ledger` に書く（`crates/server/src/lib.rs:3728` → `outage_store::judge_and_record`。`outage_store.rs:205` で `closed`、`:222` で `opened`、`:235` で `delivered`（端末の `device_id` を含む））。系統の違うサーバを読み続けると、そのサーバの DB に途絶の台帳の行と、取り違えた端末の識別子が書かれる。端末の手元でも `OutagePoller.kt:70` が相手のサーバの `episode_id` を鳴らした覚えとして保存する（`OutageNotice.kt:78-79`）
  - 条件 1（結論は満たす）: 書かれるのは取り違えたほうの DB の台帳の行だけ。`delivered` を読むものは無い（`grep -rn delivered crates/server/src web/src` は `outage_store.rs` の書き込みだけ）。正しいサーバの記録も、手元の未送信も、この選択では失わない。正しいサーバの途絶が届かない点は、読む口を止める側でも同じで、選択による差ではない。鳴らした覚えの id が重なると、直した後の正しいサーバの途絶が音なしで出ることがある（`OutageNotice.kt:78`）。ただ通知は出るので（`Show`）、失うものにはあたらない。とはいえ `keeps` の「何も書かず」は誤りなので、書く行と端末の識別子が取り違えたサーバに残ることを、`keeps`・design の残るものに書き直す必要がある
  - 条件 2（満たす）: 費用は無い（今の読む口のまま）。要件の本文は変えない（FR-106 ★ の「何も送らず」の範囲は監査 C6 が spec の段に回した）
  - 条件 3: `follows` は null
  - 付記: design.md:177 が挙げる読む口は「携帯の `GET /outages`」だけだが、PC の収集アプリも `GET /outages` を引いている（`crates/collector-windows/src/sender.rs:441`）。tasks.md の D13 は携帯（9.2）にしか書かれていない

## D14
- 判定: 確か
- 根拠: 条件 1〜4 を満たす。
  - 条件 1（満たす）: 系統を読むのは `Version`（版を読めた 200）からだけ。送るのも `Version` の答えのときだけ（`ContractGate.kt:14-16`「版 A を得たときだけ送る。ほかは何も送らずに残す」、`readContractAnswer` は `:51-58`）。なので、系統を確かめずに送る場合は無い。404（`NoEndpoint`）・読めない 200（`Unreadable`）・401 など（`Transient`）では、今でも何も送らない（`ContractGate.kt:158-185`）。系統の判定に使わなくても、送り先を取り違えて `exported` が起きることは無い。答えは契機ごとに引き直し、覚えない（`ContractGuard`、`ContractGate.kt:226-235`「答えを覚えない」。PC は `runtime.rs:815` が契機ごとに `/contract` を引く）。前の答えで別のサーバへ送ることも無い。系統を 2 つにすると、ベータ版の収集アプリは今どおり送る。3 つにする側では、ベータ版の携帯の記録が上限で `discarded` になる（監査 C8）。失う側は選ばなかったほう
  - 条件 2（満たす）: 費用は `Version` の枝に比べを 1 つ足すだけ。要件の本文はもう同じことを言っている（`docs/requirements.md` FR-106 ★「系統は開発版と、それ以外（正式版・ベータ版）の 2 つ。」「系統は契約の版を読めた答えからだけ読み…」）。D14 は要件を変えない。組み立ても `-dev.` だけで開発版を分けている（`collector-android/app/build.gradle.kts:36,95`）ので、ベータ版を `official` に含める形と合う
  - 条件 3: `follows` は null
  - 条件 4: `rests_on`（Q12・Q13・D1・D7）で足りている。ST50 の答えの 4 つの分け方は既存のコードで、決定ではない
  - 付記（否の理由ではない）: 系統を答えない古い開発版のサーバ（ST54 より前の nightly）へは、正式版の収集アプリが送る（監査 C7）。D14 の「残るもの」に書かれているのは開発版の収集アプリの向きだけ。ただしこれは FR-106 ★ の「系統を持たない古い S-01 は正式版とみなす」と、Q12 の context（「系統を答えない古いサーバは正式版とみなす」）から来ている。D14 の選択（どの答えから読むか・系統の数）では変わらない
