# ST42 深掘りの問いの独立レビュー

対象: `openspec/changes/st42-satellite-api-contract/deep-questions.json`（Q1〜Q7）・`premises.md`（P1〜P8）。
手順 1〜8 は問いの一覧を開く前にやり直した。そのあと一覧と突き合わせた。

## 手順の記録

- 手順 1（要件どうしの衝突）: FR-89 / FR-95 / FR-103 と扉 #31 の関与要件（FR-103）を読んだ。同じ対象について言い分が違う組は 2 つあった。
  (a) FR-103 は契約に「記録の封筒の形」を含めると言う。一方で衛星の書き込みは ST39（`requires: [.., ST42]`）が作るので、ST42 が公開する時点では封筒の形が存在しない（R4）。
  (b) Story の完了の判定は「同じ口を両方に載せるかは衛星の書き込みを作る Story の上流で決める」と言う。一方で Q1 の context の AI の既定 1 は「内部の契約に衛星の口を載せない」と言う（R6）。
  FR-95 は外した後の扱い（外した版を呼ぶ衛星・本人への知らせ）を何も言っていない。FR-99 は衛星のソースを途絶の通知（FR-35）から外している（R1）。
- 手順 2（扉の幅）: 扉 #31（Apache-2.0）はこの Story の文脈で幅が残っていない。utoipa は `info(license(...))` を明示すれば crate の既定（`CARGO_PKG_LICENSE`）を上書きする（`~/.cargo/registry/src/*/utoipa-gen-5.5.0/src/openapi/info.rs:80-81`）。
  ただし `tools/check-spdx.sh` は `SPDX-License-Identifier` のある全ファイルを AGPL の値と照らし、例外は path を直書きした表だけで持つ（`tools/check-spdx.sh:37-47`）。ページに Apache-2.0 の SPDX 表示を書くなら、ここに例外の行が要る（C の節）。
- 手順 3（新たに立つ一方通行）: 公開した契約（Apache-2.0。扉 #31 で決定済み）。v1 が凍る範囲（R2）。外した版の扱い（R1）。衛星の記録が、どの版の口から届いたか（R3）。
  列の型・保存形式・鍵の計算に使う入力は、この Story では新しく立たない（DB に触る変更が無い）。
- 手順 4（日常に影響する選択）: 本人の日常に効くのは、版を外したときに衛星が黙って書けなくなる場面だけ（R1）。常時の通知・電池・容量に効く選択は無い。
- 手順 5（既存コードとの突き合わせ）:
  `jq '.components.securitySchemes' docs/openapi.json` → `null`。`info.license` は AGPL。`info.version` は出所の版（`0.1.1`）。ここまでは premises の P4 のとおり。
  衛星の資格情報は `access_log::middleware` が経路を問わず `Verdict::Forbidden` にする（`crates/server/src/access_log.rs:184`）。`authorize` も 403 を返す（`crates/server/src/lib.rs:917`）。
  **premises と問いに無いのは `satellite_routes_tests::every_route_refuses_satellite` の 2 点**（R5）。1 つ目に、この試験は `lib.rs` の `.route(` の path の集合が内部の契約（`ApiDoc::openapi()`）の path の集合と一致することを assert する（`crates/server/src/satellite_routes_tests.rs:117-127`）。2 つ目に、`.nest(` `.merge(` を禁じる（同 :106-112）。
- 手順 6（過去の取りこぼし）: 当てはまったのは次の 5 つ。
  「配った後に凍るファイル（契約の写し）… premises に最初の印の時期と凍結の検査の対象」→ R2・R12。
  「git の履歴を読む検査 … CI の checkout の深さ」→ R12。
  「送らずに止める決定は、止めたことを本人がどう知るか」→ R1。
  「版が動く記録で、版の意味が読み手に伝わるか」→ R3。
  「docs/handoff の全項目」→ `docs/handoff/ST42.md` は無い。ST42 を名指す申し送りは ST27 C3（MCP。Q1 の既定 4 で扱い済み）と、ST29 の Q7（R7）だけ。
  「時計が進む／戻る」は Q6 の「外してよい日」に当たる。ただし外す操作は人が出す版の変更で、実行時の時計では動かない。R1 の族に 1 行だけ入れた。
- 手順 7（族の網羅）: 版の食い違いの族は 4 つ。並べた結果、決まっていないのは 3 つ（R1・R10）。
  - 衛星が古い → FR-95 で決まっている
  - サーバが古い → Q7
  - 衛星がサーバに無い大きい版を呼ぶ → 未決
  - 外した版を衛星が呼ぶ → 未決
- 手順 8（前提の確かさ）: loss のある決定は Q1 だけで、P1（確かさ 5）に乗る。確かさ 4 未満の前提は P3（2）と P5（2）。
  P3 に乗るのは Q5（loss 無し）だけ。P5 に乗るのは C の既定 2（loss 無し。上書きできなくても AGPL で出るだけで、扉 #31 は緩める向きには変えられる）。
  どちらも「前提が誤りうる」の指摘には当たらない。P5 は上の utoipa のソースを読めば 3 に上げられる。P6 の確かさ 4 は、手順 5 の試験が抜けている点を除けば中身と合う（R5）。

## R1. 前の版を外した後に衛星が黙って書けなくなる族が、一覧に無い（外した版への応答・本人への知らせ・衛星への予告）
- 成果物: openspec/changes/st42-satellite-api-contract/deep-questions.json
- 根拠: docs/requirements.md:930（FR-95 は 6 か月以上並べるとだけ言い、外した後を言わない）・docs/requirements.md:939（FR-99: 衛星のソースは途絶の通知 FR-35 から外す）・docs/requirements.md:919-921（FR-89 の理由: 衛星が黙って壊れると、書くはずだった記録が取れない）・Q6 の points「外してよい日の前に版の口を消すと CI が落ちる」（外した後の振る舞いは無い）
- kind: irreversible
- loss: uncaptured
- 提案: Q6 と兄弟の問いを 1 つ足す。中身は 3 つ。(1) 外した版の道を呼ばれたときの応答（404 か、外したと分かる 410 と本文か）。この応答は v1 の衛星が受け取るので v1 の契約に書く。(2) 外す前に、本人がそれを知る手段（外す予定の版を最近使った衛星を設定の画面に出す、など）。(3) 衛星への予告（応答の見出し `Deprecation` / `Sunset` など）。衛星の記録は途絶の通知に乗らないので、(2) が無いと本人は格子の空白で初めて気づく。
- 処置: escalated Q8 —— Q8 を足した（外した・無い版への応答と予告の見出しを版 1 に書き、本人への知らせの画面ができるまで版を外す変更を CI が止める）。推奨は失わない側
## R2. v1 が ST42 の merge で凍る範囲が問われていない。ST38 / ST39 / ST45 の上流を縛るのに、その代償が Q3 に無い
- 成果物: openspec/changes/st42-satellite-api-contract/deep-questions.json
- 根拠: Q3 の推奨（main の写しと比べる＝ merge した日から v1 を守る）・Q6 の推奨（6 か月は配った印から数える）・premises P8（印は 0 個）・docs/stories/ST38.md / ST39.md / ST45.md（衛星が呼ぶ口は全部 ST42 の後に来る）・openspec/specs/satellite-platform/spec.md:226-233（403 の本文は「どの口でも同じ 1 つの文字列」。いまは `"forbidden"` の平文。`crates/server/src/lib.rs:918`）・Q1 の既定 3（401 / 403 を契約に書く）
- kind: conflict
- 提案: Q3 の context に、v1 として凍るものを列挙する（道の前置き・資格情報の方式・401 / 403 の意味・断りの本文の形（平文か JSON か）・自分が誰かを確かめる口の応答）。そのうえで「後の Story がこれを変えたくなったら、衛星がまだ 1 つも無くても v2 にし、v1 を印から 6 か月並べる」（Q3 と Q6 の推奨の組み合わせ）を cost に書く。選択肢には「衛星が書ける・読める口が出るまで（ST39 / ST45）は版 0 として壊してよいと契約に書く」を足す。この案は FR-89 の本文に例外を足すので、選ぶなら A。断りの本文の形は、後で JSON の理由（未承認の種類など）を返したくなる ST45 の入口なので、少なくとも既定（C）として明記する。
- 処置: escalated Q3 —— Q3 の points・context に版 1 で凍るものを並べ、推奨の cost に版 2 の代償を書き、「版 0」の案を 3 つ目に足した（要件に例外を足す案だと書いた）。断りの本文は種別の欄を持つ JSON にする既定を Q1 の context に足した
## R3. 衛星の記録が、どの版の口から届いたかを持つかが問われていない（ST39 への申し送りも無い）
- 成果物: openspec/changes/st42-satellite-api-contract/deep-questions.json
- 根拠: docs/requirements.md:1098-1099（FR-123: 衛星の記録に衛星自身の版は持たせない。「衛星との契約は API の版（FR-89）が持つ」）・FR-95（v1 と v2 が並んで動く間は、記録が持つ S-01 の版（FR-123）からは、どちらの口から届いたかが分からない）・requirements.md:1899（扉 32: 持たなかった期間の記録は、どの版で書かれたかを後から知れない）
- kind: defer
- loss: uncaptured
- 提案: ST42 には書き込みの口が無いので、ここで問う必要は無い。ただし「並べて動かす」を作るのは ST42 なので、`docs/handoff/ST39.md` に申し送りを 1 件足すことを deep.md に書く。中身は「衛星の記録 1 件ごとに、届いた口の API の版（大きい番号と小さい番号）を持たせるか。持たないと、v1 と v2 で意味が違う欄を後から読み分けられない」。読み出しの記録では、Q1 で道に版を入れれば `MatchedPath` に版が残る。見出しで渡すなら残らない（R8）。
- 処置: escalated Q1 —— Q1 の context に「記録 1 件ごとに届いた口の版を持たせるかは ST39 が決める。道に版を入れれば読み出しの記録には版が残る」と書いて本人に見せる。docs/handoff/ST39.md に申し送りを足した
## R4. FR-103 の「記録の封筒の形」は ST42 では満たせない。どこで満たすかが一覧にも既定にも無い
- 成果物: openspec/changes/st42-satellite-api-contract/deep-questions.json
- 根拠: docs/requirements.md:923（FR-103 の括弧に「記録の封筒の形」）・docs/stories/ST39.md（衛星の書き込みは ST39。`requires` に ST42）・docs/audit/2026-10-11-license-consistency.md:21-27（C2: 書きを取り込み口 1 本にするかは未決）
- kind: defer
- 提案: 聞かない既定（C）に 1 行足す。中身は「封筒の形は ST39 が衛星向けの契約の同じ版に、追加として載せる。ST42 は口・資格情報の渡し方・版だけを載せ、FR-103 の封筒の部分は ST39 で満たす」。これを deep.md と ST39 への申し送りに写す。
- 処置: deferred ST39 —— 記録の封筒の形は ST39 が衛星向けの契約の同じ版に追加として載せ、FR-103 の封筒の部分は ST39 で満たす。Q1 の context の既定に足し、docs/handoff/ST39.md にも書いた
## R5. 既存の試験が、衛星の口を内部の契約の外に置く形と、道に版を入れる形の両方で落ちる。premises と context に無い
- 成果物: openspec/changes/st42-satellite-api-contract/premises.md
- 根拠: `crates/server/src/satellite_routes_tests.rs:117-127`（`lib.rs` の `.route(` の path の集合と `ApiDoc::openapi()` の path の集合が一致しないと落ちる）・同 :106-112（`.nest(` `.merge(` `.nest_service(` を禁じる。D14）・同 :105 以降（全経路が衛星に 403。`/healthz` `/session` のほかの例外は無い）・`crates/server/src/access_log.rs:184`
- kind: premise
- 提案: premises に P9 を足す。中身は「経路の試験は内部の契約と `lib.rs` の経路の一致を見て、nest を禁じる。確かさ 5」。Q1 と Q4 の context に「衛星の口を内部の契約の外に置くなら、この試験を 2 つの契約の和で見る形に直す。道の版を nest で組むなら抽出を広げる。自分が誰かを確かめる口を、衛星に 403 を返す試験の例外に足す（正典の MODIFIED）」と書く。
- 処置: fixed premises.md —— P9 を足した。Q1・Q4 の context に経路の試験の直しを書いた
## R6. AI の既定 1「内部の契約に衛星の口を載せない」が、既定 5 と Story の完了の判定が後に回したことを先に決めている
- 成果物: openspec/changes/st42-satellite-api-contract/deep-questions.json
- 根拠: Q1 context の既定 1 と既定 5・docs/stories/ST42.md「完了の判定」4 つ目（同じ口を両方に載せるかは衛星の書き込みを作る Story の上流で決める）・docs/audit/2026-10-11-license-consistency.md:25（取り込み口 `/ingest` を両方で受けると、分ける検査が必ず落ちる）
- kind: conflict
- 提案: 既定 1 を「衛星だけが使う口（ST42 では自分が誰かを確かめる口）は内部の契約に載せない。内部の口を衛星向けの契約にも載せるかは ST39 が決める」に狭める。Q1 の context には「道に版を入れる案で、ST39 が `/ingest` を両方で使うと決めたら、同じ処理を `/satellite/v1/…` にも別の道として生やす」と、道の案と後に回した決定の関係を 1 行書く。
- 処置: escalated Q1 —— 既定 1 を衛星だけが使う口に狭め、内部の口を両方に載せるかは ST39 と書いた。道の案と取り込みの口の関係を Q1 の context に書いて本人に見せる
## R7. 不要: Q4「置くか」は ST29 の Q7 で本人が答え済み。幅が残っているのは返す中身だけ
- 成果物: openspec/changes/st42-satellite-api-contract/deep-questions.json
- 根拠: openspec/changes/archive/2026-10-27-st29-satellite-credentials/deep.md:40-43（答え: 「置かない（ST42 が版の付いた契約と一緒に置く）」。選択肢の文に ST42 が置くことが入っている）・openspec/specs/satellite-platform/spec.md:240（正典: 衛星向けの口は版の付いた契約（ST42）と一緒に置く）
- kind: defer
- 提案: Q4 の「置かない」の選択肢を消し、置くことは既定（C）に移す。問いを残すなら、返す中身に絞る。(a) 呼んだ版だけを返すか、そのサーバが受ける版の一覧（並べている間の v1・v2 と、それぞれの小さい番号）を返すか。これは Q7 の答えに乗る。(b) 「サーバの契約の版」は、収集アプリの送信契約の版（ST50 の `GET /contract`。`crates/server/src/contract_version.rs`）と取り違えて読める。「衛星向けの API の版」と言い換える。
- 処置: fixed deep-questions.json —— Q4 を「返すもの」の問いにした。置くことは既定に移した。「サーバの契約の版」を「衛星向けの API の版」に言い換え、送信契約の版と別だと書いた
## R8. 分類違いの疑い: Q1 の `loss: exported` は、失われるものを名付けていない
- 成果物: openspec/changes/st42-satellite-api-contract/deep-questions.json
- 根拠: Q1 の 2 つの選択肢は、どちらも示し方を公開する（外に出るものは選択で変わらない）。irreversible の欄に書いてあるのは「全部直しになる」という手間で、記録の喪失ではない。FR-95 により、示し方を変えるときも v2 として出し、前の版を 6 か月並べるので、衛星は黙って壊れない（docs/requirements.md:930）。harm.who は「いまは本人だけ」。ST42 の時点で衛星の資格情報で通る口は、自分が誰かを確かめる口だけになる（spec.md:226-233・ST38 / ST39 / ST45 は後）
- kind: technical
- 提案: 選択で差が出る喪失を名付けられないなら、loss を外して B（推奨: 道に版を入れる）に落とす。A に残すなら、差の出る喪失を書く。候補として確かめられたのは 1 つ。見出しで渡す案では、読み出しの記録（`access_log` は経路を `MatchedPath` で持つ。`crates/server/src/access_log.rs:160-195`）に、どの版で読んだかが残らない（`uncaptured`）。道の案では残る。
- 処置: fixed deep-questions.json —— Q1 の loss を外して仮でよいに落とした。読み出しの記録に版が残らないことは見出しの案の cost に書いた
## R9. 不要の疑い: Q5 の 3 案のうち 2 案は要件の本文「リポジトリの中のページ」を変える
- 成果物: openspec/changes/st42-satellite-api-contract/deep-questions.json
- 根拠: docs/requirements.md:923-924（FR-103「リポジトリの中のブラウザで読めるページとして公開する」）・premises P2（Pages は無効）・P3（GitHub は Markdown を整形し、HTML はソースのまま見せる）・Q5 の 3 案目の cost 自身が「要件の『リポジトリの中のページ』には別に何か要る」と書く
- kind: defer
- 提案: 要件の文と P2 / P3 から、Markdown 以外の幅はほぼ無い。Q5 を既定（C: 契約から Markdown を機械で作って `docs/satellite-api/v1/` に置き、CI で契約と照らす）に移す。残すなら、2 案目と 3 案目に「要件 FR-103 の本文を変える」と書く。
- 処置: fixed deep-questions.json —— Q5 を外し、Markdown を機械で作って docs/satellite-api/v1/ に置く形を既定に移した
## R10. 版の食い違いの族のうち、衛星がサーバに無い版を呼ぶ場合と、新しい衛星が古いサーバへ欄を送る場合が無い
- 成果物: openspec/changes/st42-satellite-api-contract/deep-questions.json
- 根拠: Q7（サーバが古い場合だけを問う）・`crates/server/src/contract_version.rs:218,256-266`（収集アプリの取り込みは、知らない欄を `unknown_field` で断る）
- kind: technical
- 提案: Q7 の context に兄弟を並べる。
  - 衛星が v2 を呼んだのにサーバが v1 しか持たない: 応答を決めておく（R1 の外した版の応答と同じ形にするか）
  - 1.1 の衛星が 1.1 で足した任意の欄を 1.0 のサーバへ送る: 知らない欄を断るか読み捨てるか。断るなら、衛星がそれを捨てたときに記録が失われる（`discarded`）。決めるのは書き込みの ST39
  後者は ST39 への申し送りにする。
- 処置: fixed deep-questions.json —— Q7 の context に族を並べた（無い版は Q8）。知らない欄を断るか読み捨てるかは docs/handoff/ST39.md に申し送った
## R11. どれくらい困るかに無い事実: Q2 の oasdiff の cost「脆弱性の検査の対象が増える」
- 成果物: openspec/changes/st42-satellite-api-contract/deep-questions.json
- 根拠: `tools/check-vulns.sh:4`（見るのは `Cargo.lock`・`web/package-lock.json`・gradle の lockfile だけ）。取り寄せた Go の道具の実行ファイルは、どの lockfile にも載らない（osv-scanner 自身と同じ扱い。`tools/check-vulns.sh:23-38`）
- kind: premise
- 提案: cost を「Go の道具の実行ファイルを版で固定して取り寄せる。いまの脆弱性の検査（lockfile を読む）の外に置かれる」に直す。why の「記録は失わない」には条件を付ける。ST39 / ST45 で衛星が書き・読み始めた後は、見逃した壊す変更が FR-89 の理由（書くはずだった記録が取れない）に当たる。どちらの案でも見逃しはありうるので、kind は open のままでよい。
- 処置: fixed deep-questions.json —— Q2 の oasdiff の cost と why を直した
## R12. premises に「最初の印の時期」と、印を読む検査の置き場（印を取り寄せる job）が無い
- 成果物: openspec/changes/st42-satellite-api-contract/premises.md
- 根拠: premises P8（印は 0 個。いつ最初に配るかは無い）・`.github/workflows/ci.yml:126-148`（印 `v*` を取り寄せるのは rust の job だけ。「別の job では印が 0 個に見えて黙って通る」）・Q6 の points（外してよい日の前に版の口を消すと CI が落ちる＝印の日付を読む）・`scripts/upstream_misses.py report` の「配った後に凍るファイル…最初の印の時期」「git の履歴を読む検査…checkout の深さ」
- kind: premise
- 提案: premises に P10 を足す。中身は「印を読む検査は rust の job（`fetch-depth: 0`、印を取り寄せる段の後）に置く」と「最初の印の予定（未定なら未定）」。Q6 の context に「印が出るまで、前の版を外してよい日は決まらない」を書く。
- 処置: fixed premises.md —— P10 を足した。Q6 の context にも書いた
## R13. 前提が誤り: Q6 の points「本人のサーバは配った版で動く」は、いまの事実と合わない
- 成果物: openspec/changes/st42-satellite-api-contract/deep-questions.json
- 根拠: `git ls-remote --tags origin` → 空（`v*` も `nightly` も無い）・docs/requirements.md:1899（「本人の環境を配布の経路に切り替える日」は先のこととして書かれている）
- kind: premise
- 提案: 「配布の経路に切り替えた後は、本人のサーバは配った版で動く（いまは印が 0 個で、本人は配った版で動かしていない）」に直す。推奨の理由が切り替えの後の話であることを読めるようにする。
- 処置: fixed deep-questions.json —— Q6 の points を「配布の経路に切り替えた後は」に直した
## R14. Q7 の推奨は Q2 と Q4 の答えに乗るのに、それが書かれていない
- 成果物: openspec/changes/st42-satellite-api-contract/deep-questions.json
- 根拠: Q7 の points「足したのに上げ忘れると CI が落ちる」（Q2 の台本が、壊す変更だけでなく追加も見分ける必要がある）・「確かめる口も番号を返す」（Q4 の口が要る）
- kind: technical
- 提案: Q7 の context（または follows）に「Q2 の判定が追加も数えること」「Q4 の口が返す版が小さい番号を含むこと」を書く。R7 で Q4 を中身の問いにするなら、Q7 と同じ回に並べる。
- 処置: fixed deep-questions.json —— Q7 の context に Q2・Q4 への依りを書いた
## R15. 説明の無い言葉: points が Story と前提の番号だけで語っている
- 成果物: openspec/changes/st42-satellite-api-contract/deep-questions.json
- 根拠: Q4 の points（「ST29 は…」「ST38 が口を足すまで」）・Q3 の points（「premises P8」）・Q5 の points（「P2」）
- kind: technical
- 提案: 番号を言い換える。「衛星の資格情報を作った前の作業は」「承認の作業が衛星の口を足すまで」「配った印はまだ 0 個」「GitHub Pages は有効にしていない」。番号は括弧の中に残してよい。
- 処置: fixed deep-questions.json —— Q3〜Q5 の番号だけの語を言い換えた

## 聞かない既定（C）

Q1 の context の 5 つは次のとおり見た。

1. 別のファイル・別の生成にする → 妥当。ただし「内部の契約に衛星の口を載せない」は、衛星だけが使う口に狭める（R6）。経路の試験を 2 つの契約の和で見る形に直すことも併せて書く（R5）。
2. Apache-2.0 の明示、LICENSE、README と CONTRIBUTING の書き分け → 妥当（扉 #31）。utoipa は `info(license(...))` で上書きできる（`utoipa-gen-5.5.0/src/openapi/info.rs:80-81`）。ページに SPDX の表示を書くなら、`tools/check-spdx.sh` の例外の表に、その path を Apache-2.0 で足す。
3. `Authorization: Bearer ashsat_…` と 401 / 403 → 妥当。断りの本文の形（いまは平文の `forbidden`）も v1 として凍ることを明記する（R2）。
4. MCP は入れない → 妥当（`docs/handoff/ST27.md` C3 が ST27 の上流に回している）。
5. 同じ口を両方に載せるかは決めない → 妥当（Story の完了の判定と同じ）。

足してよい既定は次の 6 つ。

- 自分が誰かを確かめる口を置くこと（ST29 の Q7 の答えと正典 spec.md:240。R7）。正典「衛星の資格情報では、まだどの口も通らない」を MODIFIED にし、この口を例外に足す
- 衛星向けの契約の `info.version` は衛星向けの API の版で、出所の版（`tools/release/version.sh source`）から引かない。`tools/check-openapi.sh` の「出所の版」の Scenario は内部の契約にだけ当てる
- 衛星向けの契約の検査は、新しい台本（例: `tools/check-satellite-contract.sh`）に置き、`tools/check-openapi.sh` に書き足さない。st61 が `check-openapi.sh` を触っている（`openspec/changes/st61-auto-version-bump/proposal.md:35`）。共有の台本の末尾の取り合いも避ける（AGENTS.md）
- 印を読む検査（Q6 の外してよい日。Q3 で印の案を採る場合も）は、印を取り寄せる rust の job に置く（R12）
- 封筒の形は ST39 が同じ版に追加として載せる（R4）
- 申し送り `docs/handoff/ST39.md` に 2 件足す。衛星の記録に届いた口の API の版を持たせるか（R3）と、知らない欄を断るか読み捨てるか（R10）


### 処置で足した既定（C）

上の 6 つはすべて採った（Q1 の context の「AI の既定」に写した。封筒の形と ST39 への申し送りは R3・R4・R10 の deferred ST39）。Q5 の置き場（Markdown を機械で作る）も既定に移した（R9）。
