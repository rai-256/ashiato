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
