# ST51 深掘りの問いの独立検査（deep-review）

対象: `openspec/changes/st51-update-history/deep-questions.json`（Q1〜Q6）と `premises.md`。

※ 処置の後、問いは 5 問に組み直した（旧 Q1・旧 Q5 を C1・C2 へ、旧 Q2 → 新 Q1、R1 → 新 Q2、旧 Q3・Q4 はそのまま、旧 Q6 → 新 Q5）。下の R の本文は組み直す前の番号で書かれている。
手順 1〜8 は問いの一覧を見る前に要件・正典・コードから行い、その後で突き合わせた。

## 手順の記録（指摘にならなかったもの）

- 手順 1（要件どうしの衝突）: 該当なし。確かめた範囲: FR-121 と FR-122 ★（生存信号は記録の版に入れない）・FR-123 ★（生存信号は S-01 の版を持たない）は向きが同じ。
  FR-130（封筒に欄を足したら送信契約の版を上げる）は生存信号の契約（`docs/collector-contract.md:544` 以降）にも当たるが、`git tag` が空で最初の正式版がまだ無く、FR-130 の「最初の正式版を出すまでの足し方は版 1 に含める」に入る。FR-78 の「原文の素通し・冪等キー」は、版を原文に入れる Q1 の context と食い違わない
- 手順 2（扉の幅）: R6
- 手順 3（新たな一方通行）: R1・R3・R11
- 手順 4（日常）: R10。電池・容量: 1 件 60 バイトほどの増え方（Q1 の context）で、日常に効く選択は見つからなかった
- 手順 5（既存コードが要件を満たしていない箇所）: いまは版の変化を残す場所がどこにも無い（`grep -rn "update_history\|component_version" crates migrations` は 0 件）。収集アプリの版の出所は C-01 の `versionName = "0.1.0"`（`collector-android/app/build.gradle.kts:27`）だけで、C-02 には無い（`grep -rn CARGO_PKG_VERSION crates/collector-windows` は 0 件）→ R8
- 手順 6（過去の取りこぼし）: 当てはまったのは「時計が進む／戻る 1 件」（R9）、「書き直せない記録の形の凍結」（R11）、「手元に状態を持つ決定は保存場所ごと消える場合を問う」（Q1 の選択肢 3 が問えているので抜けではない）、「DB や写しを増やす決定」（R1）。ほかの観点（検証コマンド・画面の寸法・Takeout の欄）はこの段では当てはまらない
- 手順 7（族の網羅）: Q1 と Q4 の族は足りている。Q2 の族に抜けがある（R1・R4）
- 手順 8（前提の確かさ）: R7・R8

## R1. 移行が落ちた回の 1 行を D-01 に書くと、FR-119 の戻しで消える。行の置き場を問うていない
- 成果物: openspec/changes/st51-update-history/deep-questions.json
- 根拠: crates/server/src/lib.rs:305（migrate の順は「退避 → 当てる」。退避は当てる前に取る）/ docs/requirements.md:1014（FR-119: 落ちたら「移行の前の退避から D-01 を戻す」）/ docs/stories/ST58.md:5（ST58 は ST51 を requires に持つ）/ Q2 の why「移行が落ちた回は…この行が唯一の跡になる」/ Q2 の族の「移行の失敗の後、退避から D-01 を戻した」は「戻したことの行」しか扱っていない
- kind: irreversible
- loss: discarded
- 提案: Q2 の推奨は「落ちた + 落ちた版の名前と SQLSTATE」を D-01 の行に書く。でもその行は退避を取った後に書かれるので、FR-119 が退避を戻すとちょうど消える。そうなると Q2 が残そうとした跡は、戻しのある経路（落ちた回はみなこの経路を通る）では残らない。更新の履歴の置き場を問いに足す。選択肢は「D-01 の中に置き、ST58 が戻す前に読んで戻した後に書き直す（申し送り）」「D-01 の外の FR-109 のフォルダに追記のファイルとして置く」など。どれを選んでも、戻しで何が消えるかを選択肢ごとに書く。Q2 の why の「唯一の跡」もこれに合わせて直す
- 処置: escalated —— Q2（移行が落ちた回の跡を、戻しの後にも残すか。loss: discarded）を足した。推奨は「D-01 の中だけに置き、戻した後に ST58 が落ちた試みと戻したことの 2 行を書き直す」。Q1 の why の「唯一の跡」を直し、族の FR-119 の項を Q2 へ回した

## R2. Q1 は失わない側が安く・範囲の中で・要件を変えないので、A でなく P に当たる。why の「溜めの上限で捨てた」も起きない
- 成果物: openspec/changes/st51-update-history/deep-questions.json
- 根拠: .harness2/docs/flow-gates.md:227-228（loss があっても、失わない側が高くつかず・範囲を越えず・要件を変えないなら P）/ Q1 の why「失わない側（全部に載せる）の費用は 1 件あたり数十バイトで小さいが、版の変化を何から読むかを決めるので本人に聞く」/ collector-android/app/src/main/kotlin/dev/ashiato/collector/LocationService.kt:314（生存信号の未送信は「上限の対象にしない（捨てない）」）/ docs/collector-contract.md:525（生存信号は 90 日・2 GB の上限で捨てない）
- kind: technical
- loss: uncaptured
- 提案: Q1 は推奨（すべての生存信号に、積んだ時の版を載せる）で仮に進め（P）、台帳に載せる。why の「載せる回が少ないと…端末の溜めの上限で捨てた」は消す（生存信号は上限で捨てない）。失いうるのは「消して入れ直すと未送信の置き場ごと消える」だけと書く。「何から読むかを決める」は Q3（B）が持っている。P にしないなら、A に残す理由（どの費用が高いか）を why に書く
- 処置: fixed D1 仮 —— 旧 Q1 を問いから外し、失わない側（すべての生存信号に積んだ時の版を載せる）を C1 にした。「溜めの上限で捨てた」は消し、上限の対象外であることを C1 に書いた

## R3. Q2 の「この行は全記録の書き出しに入る（premises 10）」は前提として誤り。置き場しだいで選択肢 2 の書き出しの損は起きない
- 成果物: openspec/changes/st51-update-history/deep-questions.json, openspec/changes/st51-update-history/premises.md
- 根拠: crates/server/src/export.rs:631（分類を求めるのは `table_schema = 'core'` の表だけ）/ openspec/specs/distribution/spec.md:13（ST48 の版の記録は `core` の外）と :64（書き出しのファイルの一覧に版の記録は無い）/ premises.md の前提 10「全記録の書き出しは、分類の無い表があると始まらない」（`core` の外の表には当たらない）
- kind: premise
- loss: exported
- 提案: 前提 10 は「`core` の中に置くなら分類が要る。外に置けば書き出しに出ない（ST48 の版の記録と同じ）」と書き直す。Q2 の context の「この行は書き出しに入る」は事実ではなく、置き場の選択（R1）と Q6 で決まることなので、そう書き直す。書き出しに出すかどうかが決まらないうちは、選択肢 2 の不可逆（私的な値が書き出しに残る）は条件つきになる。そこで「DB の誤りの本文は書き出しに出さない置き場にだけ残す」を選択肢に足すかを検討する（本文が消す口から漏れる点は残る）
- 処置: fixed D3 仮 —— 前提 10 を「分類を求めるのは `core` の表だけ」に直した。置き場は `core` の追記のみの表にして書き出しに入れる（C5）と決め、Q1 の context に書いた。選択肢 2 の書き出しの損はこの置き場で成り立つ

## R4. Q2 の族に「起動が版の検査で断られた」「退避の失敗」が無い。Q5 の推奨は、版の検査の後に書くと言っていない
- 成果物: openspec/changes/st51-update-history/deep-questions.json
- 根拠: crates/server/src/lib.rs:3862（起動の版の検査 `startup_refusal`。db_ahead / db_behind / ledger_unreadable で終了コード 3）/ openspec/specs/distribution/spec.md:13（S-01 の実行時の役割は版の記録を「読むことだけ」）/ docs/requirements.md:1009（FR-117: 新しい D-01 を古い S-01 が書き換えない）/ docs/handoff/ST58.md（終了コード 1 には「退避の失敗」も入る）/ Q5 の推奨「S-01 の起動のとき（待ち受けを開く前）」
- kind: technical
- 提案: Q2 の族に 3 つを足す。(a) 起動が db_ahead / db_behind / ledger_unreadable で断られた回（書かない。書くと FR-117 に反し、古い S-01 が新しい D-01 に書くことになる）。(b) migrate が退避に失敗して 1 つも当てなかった回（終了コード 1 だが、落ちた版も SQLSTATE も無い）。(c) 落ちた回の行がある後の「前の版」に何を使うか（落ちた版を「今の版」に数えない）。Q5 の推奨は「版の検査を通った後、待ち受けを開く前」と書く。「直前の行」は S-01 の時計の順ではなく書いた順で読む、と書き添える
- 処置: fixed deep-questions.json —— Q1（旧 Q2）の族に「起動が版の検査で断られた」（書かない）・「退避の失敗」（推奨の結果の種類に足した）・「落ちた行の後の前の版」（C4）を足した。版の検査の後に書くことは C2 に書いた

## R5. Q5 は open（失うものが無い）と書いているが、推奨でない選択肢は uncaptured を持つ。Q1 と同じ形なのに分類が揃っていない
- 成果物: openspec/changes/st51-update-history/deep-questions.json
- 根拠: Q5 の why「書く場所を増やしても減らしても…それまでの行は変わらない」/ Q5 の選択肢 2「migrate を走らせずに入れ替えた起動は残らない」・選択肢 3「移行が落ちた回（起動しない）は残らない」/ Q1 は同じ構造（載せる回を減らすとその間の版が残らない）で irreversible + uncaptured
- kind: technical
- loss: uncaptured
- 提案: Q5 を `kind: irreversible`・`loss: uncaptured` にする。推奨（両方で書く）が失わない側で費用が小さいので、扱いは P（仮で進め、台帳に載せる）のままでよい。why は「狭い側を選んだ期間の入れ替えは後から作れない。失わない側を推奨にした」に直す
- 処置: fixed D2 仮 —— 旧 Q5 を問いから外し、失わない側（migrate の終わりと起動の両方）を C2 にした（分類を揃える代わりに、P の条件を満たす C にした）

## R6. Q3 の推奨（生存信号から導く）は、FR-121 の「追記のみの記録に残す」と完了の判定の「1 行残る」「書き換えられない」の読み替えになる。導いた行を読む口も Q6 の推奨と噛み合っていない
- 成果物: openspec/changes/st51-update-history/deep-questions.json
- 根拠: docs/requirements.md:1017（FR-121「追記のみの記録に残す」）/ docs/stories/ST51.md:47-49（完了の判定「生存信号の申告で版の変化が 1 行残り」「その記録を後から書き換えられない」）/ docs/requirements.md:1741（扉 33「残す。追記のみ」）/ Q3 の推奨「版が変わった所を 1 行として読む口を作る」/ Q6 の推奨「画面・API…は変えない」
- kind: conflict
- 提案: Q3 の推奨は、「追記のみの記録」を生存信号の行とし、「1 行」は導出として読む、という扉 33 の解釈である。導く規則（Q4）を直すと過去の変化の見え方も変わるので、完了の判定の「書き換えられない」をどう満たすかを Q3 の context に書く。あわせて、Q6 の推奨（API も画面も作らない）のもとで導いた 1 行がどこに現れるのか（D-01 の view・書き出しのファイル・試験だけ）を Q3 か Q6 の選択肢に書く。解釈を変えないなら、Q3 の選択肢 2 との差は「凍結するかどうか」だけになる
- 処置: fixed D5 仮 —— Q3 の推奨（生存信号から導く）を D5（仮）にする。反転条件: 導いた読み口では本人が区別に使えないと分かったら、変化の行を別の表へ追記する側に変える（Q3 の答えが「別の表」でも同じ）。Q3 の context に「追記のみの記録」は版を載せた生存信号の行、変化の 1 行は導いた読み口（D-01 のビューと書き出し）に出る、と書き、推奨の選択肢にも読む口を書いた。Q5（旧 Q6）の推奨の detail を、書き出しの 2 つのファイルで区別できると直した。読み替えかどうかは本人が Q3 で選ぶ

## R7. 前提 3「新しい収集アプリが ST51 より前の S-01 に送ると版だけが落ちる」は不正確。原文に版を入れるので原文には残る。Q4 の推奨はそれを捨てる
- 成果物: openspec/changes/st51-update-history/premises.md, openspec/changes/st51-update-history/deep-questions.json
- 根拠: crates/server/src/heartbeat.rs:34（`raw` は素通し）/ crates/server/src/lib.rs:2853（INSERT に `raw` をそのまま入れる）/ Q1 の context「版は原文にも入れる」/ Q4 の推奨「版の欄を持たない信号は並びから外し」
- kind: premise
- 提案: 前提 3 の結果を「列は落ちるが、原文には残る」に直す。Q4 の推奨は「版の列が無く、原文にも版が無い信号を並びから外す（原文に版があれば読む）」に直す。どちらも導出なので失うものは無いが、前提表と推奨の文面が事実と合っていない
- 処置: fixed premises.md —— 前提 3 と C3: 前提 3 を「欄は落ちるが原文に残る」に直し、導くときは欄が無ければ原文を読む（C3）とした。Q4 の推奨と族も「欄も原文も無いものだけを外す」に直した

## R8. 損のある Q1 が確かさ 2 の前提 5（ST49 の版の印）に載っているのに、premises.md の「Q1 が載る前提」に 5 が無い。ST51 は ST49 を requires に持たない
- 成果物: openspec/changes/st51-update-history/premises.md, openspec/changes/st51-update-history/deep-questions.json
- 根拠: premises.md「Q1 は 1・2・3・4」と表の 5（確かさ 2「走っている change」）/ Q1 の context「版の印の形は ST49 が作る…ST51 はそれを載せるだけ（premises 5）」/ docs/stories/ST51.md:5（`requires: [ST02]`）/ collector-android/app/build.gradle.kts:27（C-01 の版は `versionName = "0.1.0"` だけ）/ `grep -rn CARGO_PKG_VERSION crates/collector-windows` は 0 件（C-02 は版を持たない）/ openspec/changes/st49-record-component-version/design.md:183（ST49 は生存信号に版を付けず ST51 に任せる）
- kind: premise
- 提案: premises.md の「Q1 が載る前提」に 5 を足す。ST51 の実装が ST49 の merge より先に入る場合、収集アプリの版がどこから来るかを Q1 の context に書く（ST49 を requires に足す・ST49 の印ができるまで版番号だけを載せる、など。spec の段で決めると前提 5 は言っているが、損のある問いが載っている）。前提 1・2・4 は grep で確かめて合っていた（LocationService.kt:989 付近で刻みの前に `emitter.emit()`、Heartbeat.kt:340-380 で積む時に原文を組む、heartbeat.rs:76-88）が、まだ走らせてはいない（確かさ 3）。premises.md の「tasks の試験で 4 に上げる」を tasks に落とすとき、この 3 つも含める
- 処置: fixed premises.md —— 「載る前提」の段落を組み直し、C1 が確かさ 2 の前提 5 に載ることと、ST49 の上に積むので spec の段で requires に ST49 を足すこと（C7）を書いた

## R9. Q4 が open なのは Q3 の推奨に乗っているから。Q3 で凍結する側（選択肢 2）を選ぶと、Q4 の並べ方と時計が戻る 1 件の結果は戻せなくなる
- 成果物: openspec/changes/st51-update-history/deep-questions.json
- 根拠: Q4 の why「信号の行に版と発信の時刻と受信の時刻が全部残るので、数え方は後から計算し直せる」/ Q3 の選択肢 2「凍結した行は直せない」/ scripts/upstream_misses.py report の「時刻を入力にして状態を一方向に動かす決定（開始日・最終成功時刻・版など）には、端末の時計が大きく進む／戻る 1 件が来たときの結果を必ず 1 行で問う」
- kind: technical
- 提案: Q4 に「Q3 で導く側を選んだときに限り仮でよい。凍結する側なら、並べ方と時計が狂った 1 件の結果は凍った行に残る」と書く（Q3 との関係を follows / rests_on として明示する）。時計が大きく進んだ 1 件が来たら、推奨の並べ方でどの変化が何行できるかも 1 行で書く
- 処置: fixed deep-questions.json —— Q4 の why に「仮でよいのは Q3 で導く側を選んだときだけ。別の表へ追記する側では並べ方が凍結する」と書いた

## R10. C-01 は更新の後に自分では起き直さない。「更新で止めた時間」は本人がアプリを開くまで続くが、どこにも書かれていない
- 成果物: openspec/changes/st51-update-history/deep-questions.json, openspec/changes/st51-update-history/premises.md
- 根拠: `grep -rn "PACKAGE_REPLACED" collector-android/app/src/main/` は 0 件 / collector-android/app/src/main/kotlin/dev/ashiato/collector/MainActivity.kt:183（`LocationService` を起こすのは画面だけ）/ collector-android/app/src/main/AndroidManifest.xml:91-96（起動時の受け手は気分の通知のものだけ）/ premises.md の前提 1「起動の直後に生存信号を 1 件積む」
- kind: defer
- 提案: 前提 1 の「起動」は、C-01 では更新の後に本人がアプリを開いた時のこと、と書き添える。新しい版の最初の信号の時刻（Q1 の選択肢 1 の detail「起動の直後」）は、本人が開いた時刻になる。更新の後に自動で起き直すかどうかは更新を入れる Story（ST58 / FR-114）の範囲なので、問いにはせず `docs/handoff/ST58.md` へ申し送る
- 処置: deferred ST58 —— 前提 1 に C-01 が更新の後に起き直さないことを書き、C10 で ST58 へ申し送ると決めた（ST58 はまだ tasks.md が無い。申し送りの本文は spec の段で docs/handoff/ST58.md に置く）

## R11. 追記のみで書き直せない新しい形（S-01 の版の変化の行・生存信号の原文に入れる版の欄の名前）を固定するかを問うていない
- 成果物: openspec/changes/st51-update-history/deep-questions.json
- 根拠: scripts/upstream_misses.py report の「書き直せない記録（追記のみの台帳・凍結列）の形を新しく決めるときは、上流で『形の凍結（固定バイト列の試験）と読めない形の出し方』を問いにする」/ openspec/specs/distribution/spec.md:18-23（ST48 は版の記録の置き場と列の形を Requirement の表で固定し、「形の版」を持つ）/ crates/server/src/heartbeat.rs:76-88（原文は冪等キーの入力で、後から書き換えられない）
- kind: irreversible
- loss: rewrite-all
- 提案: 失わない側（ST48 と同じく、置き場・列の名前と型・形の版を Requirement の表で固定し、知らない形を読んだときの出し方を決める。原文の版の欄の名前も契約 `docs/collector-contract.md` に固定する）は安く範囲の中にあるので、P として台帳に載せる。問いを足すほどではない
- 処置: fixed D4 仮 —— 列と欄の名前・型・NULL の可否を Requirement の表で固定すると C6 に書いた（ST48 の版の記録と同じ形）

