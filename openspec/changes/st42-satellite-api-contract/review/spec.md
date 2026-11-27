# ST42 spec の独立検査（spec review）

対象: `openspec/changes/st42-satellite-api-contract/`（第 1 回の先読み。deep.md の Q1〜Q8 は答え待ちで、推奨の答えで書いた版）。
読んだもの: proposal.md・specs/satellite-platform/spec.md・specs/data-sensitivity/spec.md・design.md・tasks.md・deep.md・premises.md・review/deep.md・deep-questions.json・
`docs/stories/ST42.md`・`docs/stories/INDEX.md`・`docs/requirements.md`（FR-89・FR-95・FR-103）・`docs/handoff/ST39.md`・正典 `openspec/specs/{satellite-platform,data-sensitivity,distribution}/spec.md`・
走っている change の delta（`st59-change-password-from-web`・`st61-auto-version-bump`・`ch21-trial-use-before-verify`・`ch26-nightly-vulns-cleanup`）。
突き合わせたコード: `crates/server/src/access_log.rs`・`crates/server/src/satellite_routes_tests.rs`・`tools/release/changes.sh`・`tools/check-openapi.sh`・`tools/check-vulns.sh`・`.github/workflows/ci.yml`・`migrations/202609290900_access_control.sql`・`migrations/202610241200_satellite_credentials.sql`。

## 機械の検査（先に写す）

| コマンド | 結果 |
|---|---|
| `openspec validate st42-satellite-api-contract --strict` | `Change 'st42-satellite-api-contract' is valid` |
| `python3 scripts/check_chain.py .` | `chain: OK (0 件 / 未回収 0 件 / warn 0 件)`。要件 178 件 / Story 64 本。観点 8（`scripts/check_chain.py:177-178` の `regen_diff`。stories.json からの再生成との一致）も FAIL を出していない |
| `python3 scripts/check_scenarios.py . st42-satellite-api-contract` | `scenarios: FAIL (担保なし 36 件 / 名無しの確認待ち 0 件)`。36 件はどれも satellite-platform の delta の Scenario で（`grep "担保の無い" | grep -vc "st42-satellite-api-contract/specs/satellite-platform"` → 0）、ADDED の Scenario の数 36 と同じ。どれも実装前。36 件すべてを tasks.md の Task が一字一句で名指ししている（スクリプトで照合し、欠け 0 件）。上流の段では予定どおりの FAIL（Task 9.3 が閉じる） |
| `python3 scripts/review_triage.py . st42-satellite-api-contract` | このファイルを書く前: `triage: FAIL (4 件)`。指摘 15 件 / 仮決め 1 件 / 要件へ戻すもの 0 件。`deep.md R1` `R2` `R3` `R6`: 「escalated なのに deep.md に R<n> が無い」→ R4。書いた後: `triage: FAIL (24 件)`。指摘 35 件で、増えた 20 件はこのファイルの R1〜R20 の「処置が無い」（処置は呼び出し元が付ける。kind はどれも読めている） |
| `python3 scripts/spec_overlap.py candidates . st42-satellite-api-contract` | 1 件: `st59-change-password-from-web` / data-sensitivity /「誰がいつ読んだかを、追記のみの読み出しの記録に残す」（kind 名指し・相手 MODIFIED）→ 観点 7 |

---

## 観点 1: deep の推奨と聞かない既定が正典に写っているか

確かめて食い違いが無かったもの: Q1（道に版。spec :54・Scenario :93 ほか）・Q2（自前の台本の規則。spec :114-117。Scenario の欠けは R6）・Q4（版の一覧。Scenario :94・:247）・Q7（小さい版。Scenario :151）・Q8 の断りと予告（Scenario :244-268）。聞かない既定のうち、別の生成・Apache-2.0・LICENSE と README / CONTRIBUTING・`ashsat_` の渡し方・401・JSON の断り・製品の版と別に数える・新しい台本・印を取り寄せる job・封筒は ST39・MCP は入れない、は spec か design・tasks に写っている（差は R3・R5）。第 1 回の先読みなので、当初案を覆したもの・要件へ戻すものは無い。決定の数値（6 か月・`1.0` → `1.1`）は Scenario :212-226・:151 にある。

## R1. Q6 の推奨は「新しい版を手にしてから必ず 6 か月ある」を約束しているが、design D12 は印の commit の日から数えるので 6 か月より短くなりうる。spec は「印の日」が何の日かを決めていない
- 成果物: openspec/changes/st42-satellite-api-contract/specs/satellite-platform/spec.md
- 根拠:
  - deep-questions.json Q6 の推奨の gain:「衛星の作り手が新しい版を手にしてから、必ず 6 か月ある」。spec :194 の導出元も「必ず 6 か月あるようにするため」と書く
  - design.md :146 は「日は印の commit の日」と決める。同じ design の :173（Risks）は「印を付けるのが commit より遅れると、6 か月の数え始めは早い側（作り手の移る期間が短い側）にずれる」と認めている。つまり推奨の約束は D12 の上では成り立たない
  - spec :189 は「配った印（`v*`）の日から 6 か月」とだけ言い、Scenario :212 も「印 `v0.5.0` の日が 2027-01-10」と書く。commit の日・印を作った日・Release の日のどれで作っても、実装の材料の上では Scenario が通る。本人の決めた起点が spec から決まらない
  - 「`git tag --contains` の最も古い印」（design :146）の「最も古い」の並べ方（日か版の番号か）と、月末の丸め（「月末は月の最終日に丸める」）も design にしかない
- kind: conflict
- 提案: 起点を「その印を作った日と印の commit の日の遅い方」（作り手の期間が 6 か月を下回らない側）にするか、Release を公開した日にする（印を作った日が取れるのは注釈つきの印だけ。この repo の印をどう作るかは確かめていない —— `tools/` にも `.github/` にも `git tag` を作る行は無かった）。どちらにするかを D12 に書き、spec :189 の (a) にその日の定義を入れる。Scenario を 1 本足す: 印の commit の日が 2027-01-01、印を作った日が 2027-01-10 なら、外せる日は 2027-07-10。月末の丸め（例: 2027-08-31 の 6 か月後は 2028-02-29）も Scenario にする
- 処置: fixed D12 仮 —— 配った日を「印を作った日と印の commit の日の遅い方」にし、暦の月と月末の丸めを spec の (a) に写した。Scenario「印を commit より後に作ったときは、印を作った日から数える」「同じ日が無い月は、月の最終日まで外せない」を足した（Task 7.1）。反転条件は Release の日

## R2. Q8 の「知らせの画面ができるまで版は外せない」を守る (b) は、`notice` にどんな文字列を書いても満たせる
- 成果物: openspec/changes/st42-satellite-api-contract/specs/satellite-platform/spec.md
- 根拠:
  - spec :190「(b) 版の一覧の外す版の項目が、本人に知らせる画面の Requirement の名前（`notice`）を持つ」。design D12 :147 も「`notice` がある」ことだけを見る
  - Scenario :216-220 は `notice` が無い場合だけを見る。`notice: "x"` のように正典に無い名前を書いた木が通るか落ちるかを、どの Scenario も決めていない
  - Q8 の推奨の cost「知らせの画面ができるまで、版は外せない」は、名前が実在する Requirement を指すときにだけ成り立つ。名前の有無を見るだけなら、1 行足せば外せる
- kind: technical
- 提案: (b) を「`notice` の名前の Requirement が `openspec/specs/` の正典にある」に強める。Scenario を 1 本足す: 6 か月たち、`notice` に正典に無い名前を書いて版 1 を消した木では検査が落ち、その名前が示される
- 処置: fixed specs/satellite-platform/spec.md —— (b) を「`notice` が正典の Requirement の名前と一字一句同じ」に強め、Scenario「知らせの画面の名前が正典に無ければ前の版は外せない」を足した（D12・Task 7.1）

## R3. Q3 の推奨で「main に入った日から凍る」形のうち、確かめる口の欄の名前と 403 の応答が spec に無く、design D6 と Task 2.1 にしかない
- 成果物: openspec/changes/st42-satellite-api-contract/specs/satellite-platform/spec.md
- 根拠:
  - deep-questions.json Q3 の points「main に入った日から版 1 が凍る（道・資格情報・401/403・断りの本文・確かめる口）」。Q1 の context の既定 3「401（資格情報が違う）と 403（承認がまだ）を書き」
  - 確かめる口の応答の欄の名前（`satellite.id`・`satellite.name`・`api_versions[].version`・`api_versions[].sunset`）は design.md :82 にしかない。Scenario :94 は「衛星の識別子…・名前「読書ログ」と、版の一覧 `[...]` を持ち」と言うだけで、欄の名前を決めていない。版の一覧の欄の名前は Scenario :247 の `api_versions` で決まるが、確かめる口の側は決まらない
  - 契約に 403 を載せることは Task 2.1（tasks.md :45「401・403 の応答と断りの型を付ける」）だけが言う。Requirement :9 と Scenario :34 は 401 だけを求めている。403 を載せない契約でも Scenario は通る
  - Scenario :94 の「記録・件数・承認・資格情報の値も本体の中の番号も含まない」は、何を見れば真偽が決まるかが書かれていない（「件数」「承認」を欄の名前で見るのか値で見るのかが分からない）
- kind: technical
- 提案: Scenario :94 の THEN を「本文の欄は `satellite.id`・`satellite.name`・`api_versions` だけで、`api_versions` は `[{"version": "1.0", "sunset": null}]`」に替え、欄の一覧で否定を測る。Requirement :9 に「403（`not_satellite`）の応答も載せる」を足し、Scenario :34 の THEN に 403 を加える
- 処置: fixed specs/satellite-platform/spec.md —— 確かめる口の本文の欄の名前を Requirement と Scenario に写し（ほかの欄を持たないで否定を測る）、契約のすべての口に 403 を載せることを Requirement と Scenario「衛星向けの契約は資格情報の渡し方を持ち…」に足した

## R4. deep.md に、escalated で閉じた review/deep.md R1・R2・R3・R6 の出所が無い（triage の FAIL 4 件）
- 成果物: openspec/changes/st42-satellite-api-contract/deep.md
- 根拠:
  - `python3 scripts/review_triage.py . st42-satellite-api-contract` → `deep.md R1: escalated なのに deep.md に R1 が無い`（R2・R3・R6 も同じ）
  - review/deep.md の処置は、R1 → Q8、R2 → Q3、R3 と R6 → Q1。deep.md :5-38 の Q1・Q3・Q8 には「出所」の行が無い
  - st59 も先読みの段（71b749bff）では deep.md に出所の行が無く、spec の独立レビューを処置した commit（ce689eb44）で Q ごとに `- 出所: review/deep.md R2・R9（…）` の 3 行を足した。今の st59 は `triage: OK`。先読みを書く席が出所を書かず、spec の段の処置で埋める流れになっている
- kind: technical
- 提案: st59 と同じく、deep.md の Q1 に `- 出所: review/deep.md R3・R6`、Q3 に `R2`、Q8 に `R1` の行を足す（答えの欄には触れない）。先読みを書く席が出所を書かないことは、ハーネス側の直しの候補（ST42 の成果物の外）
- 処置: fixed deep.md —— Q1 に R3・R6・R8、Q3 に R2、Q8 に R1 の出所の行を足した（答えの欄には触れていない）

## R5. 本人に見せた既定・points の字面と spec に、小さな差が 2 つある
- 成果物: openspec/changes/st42-satellite-api-contract/specs/satellite-platform/spec.md
- 根拠:
  - Q1 の context の既定 3 つ目は「`docs/satellite-api/v1/` に契約の JSON・LICENSE と並べる」と言う。spec :12 と design D2 :46 は LICENSE を `docs/satellite-api/LICENSE`（版の外）に置いていて、置き場を替えた理由が書かれていない
  - Q6 の points は「版の一覧（版・出した印・外してよい日）を置き」と言う。design D8 :96 の版の一覧の形は `major`・`version`・`sunset`・`notice` で、出した印と外してよい日は持たない（検査が git から計算する）
- kind: technical
- 提案: D2 に「版をまたいで 1 枚の LICENSE を置く（版ごとに写さない）」の理由を 1 行書く。D8 に「出した印と外してよい日は一覧に持たず、検査が印から計算する（Q6 の points の一覧の欄とは違う）」を書く。どちらも振る舞いは変わらない
- 処置: fixed D2 —— LICENSE を版の外に 1 枚置く理由を D2 と spec の導出元に書いた。版の一覧の欄が Q6 の points と違う理由を D8 と spec に書いた

---

## 観点 2: Scenario が検証可能か

## R6. 壊す変更の規則（D9）のうち、Requirement の本文だけが言い、Scenario が無いものが 7 つある
- 成果物: openspec/changes/st42-satellite-api-contract/specs/satellite-platform/spec.md
- 根拠: Requirement :114-117 と design D9 の表 :106-116 にあるが、Scenario :127-183 が 1 つも当てていないもの:
  1. 口の応答の状態の番号を消す（D9 の 2 行目）
  2. 応答の型の欄を必須でなくする（:115「欄を必須でなくする」。Scenario :133 は消す・型を変えるだけ）
  3. 欄や引数の `format` を変える（Scenario :135 は `type` の文字列 → 整数だけ）
  4. 送る側に、比べる相手に無い必須の欄や引数を足す（Scenario :141 は任意 → 必須だけで、引数は当てていない）
  5. 口の `security` を変える（Scenario :147 は securitySchemes の方式を消すだけ）
  6. 応答の状態の番号か任意の送る欄を足したのに、小さい版を上げていない（Scenario :151 は口を足す場合だけ）
  7. 前の版を「サーバの経路から」外す（spec :188・design :145 は 3 つの置き場を挙げるが、Scenario :204-226 は契約と版の一覧の項目を消す場合だけ）
  - どれも実装で抜いても、36 本の Scenario は全部通る
- kind: technical
- 提案: 1〜6 は `--self-test` の場合として Scenario に足す（2 本ずつ束ねず 1 規則 1 本）。7 は「版 1 の写しと一覧は残し、`SatelliteApiDoc` から版 1 の口を消した木では検査が落ちる」の 1 本にする。tasks 6.1・7.1 の名指しに加える
- 処置: fixed specs/satellite-platform/spec.md —— 1〜6 を 1 規則 1 本の Scenario に足し（状態の番号を消す・必須でなくする・形式・新しい必須の欄か引数・口の security・状態の番号と任意の送る欄の上げ忘れ）、7 は「サーバの経路から前の版の口を消しても外す変更に数える」にした（Task 6.1・7.1）

## R7. 「外す日を持つ版の道へのすべての応答に `Sunset`」と言う Requirement を、Scenario は 200 の 1 つでしか見ておらず、Task 3.1 はハンドラの応答だけに狭めている
- 成果物: openspec/changes/st42-satellite-api-contract/specs/satellite-platform/spec.md
- 根拠:
  - spec :238「その大きい版の道への**すべての**応答に、見出し `Sunset`」
  - tasks.md :55「その大きい版の道への応答（確かめる口・版の受け口の not_found）に見出し `Sunset`」。middleware が先に返す 401・403（D11 :139）は入らない
  - Scenario :259-263 は `GET /satellite/v1/me` の 200 だけを見る。401・403・404 `not_found` の応答に `Sunset` が付かない実装でも通る
- kind: technical
- 提案: 本文と tasks のどちらかに合わせる。外す日の予告を衛星に届けるのが目的（Q8）なら、資格情報の断り（401・403）にも付ける側に寄せ、Scenario を 1 本足す（外す日を持つ版の道に資格情報無しで呼んでも、401 の応答が `Sunset` を持つ）。狭めるなら、本文 :238 を「資格情報が通った求めの応答」に直す
- 処置: fixed D11 —— `Sunset` は middleware の外側の層で付け、401・403 にも付ける側に寄せた（spec の本文に断りを含むと書いた）。Scenario「外す日を持つ版の道では、資格情報の断りにも Sunset の見出しが付く」を足した（Task 3.1）

## R8. 版の受け口に当たらない `/satellite/` の道（`/satellite/v9`・`/satellite/v1/`）と、口はあるがメソッドが違う求め（`POST /satellite/v1/me`）について、断りの形と読み出しの記録の行が決まっていない
- 成果物: openspec/changes/st42-satellite-api-contract/specs/satellite-platform/spec.md
- 根拠:
  - design D5 :71「衛星向けの道（`/satellite/` で始まる）の断りは、すべて `application/json` で `{"error": "<種別>"}`」。spec :272「`/satellite/` で始まる道に届いた**すべての**求めを…1 行」。spec :273 は口の名前を「当たった経路の形」で書く
  - 版の受け口は `/satellite/{version}/{*rest}`（design :66）。axum 0.8 の文書は ``/{*key}` doesn't match empty segments`` と書く（`~/.cargo/registry/src/*/axum-0.8.9/src/docs/routing/route.md:53`。走らせてはいない）
  - 読み出しの記録の middleware は `route_layer` で掛かっている（`crates/server/src/lib.rs:4515-4519`）ので、どの経路にも当たらない求めは middleware を通らず、行が残らない。応答は画面の配り口の fallback（`lib.rs:4406` の `web_assets::fallback`）が返す（コードを読んだだけで、走らせてはいない）。spec :272 の「すべての求めを 1 行」も、D5 の「すべて JSON」も、この道では成り立たない
  - Scenario :244-257 と :284-287 が見るのは `/satellite/v9/me`・`/satellite/v1/nothing-here`・`/satellite/v9/abc-secret-path` だけで、どれも受け口に当たる。メソッド違いの求めはどの Scenario にも無い
  - 版 1 は main に入った日から凍る（R3）ので、ここで返す形も衛星が受け取る形になる
- kind: technical
- 提案: D5 と D11 に、受け口に当たらない道とメソッド違い（405）の応答の形と記録の行（口の名前に何を書くか）を決めて書き、spec :235-238 に移す。Scenario を 2 本足す: 有効な衛星の資格情報で `GET /satellite/v9` を呼ぶと 404 `unsupported_version` で、行が 1 行増える / `POST /satellite/v1/me` は 405 で、本文は `{"error": …}` の JSON である
- 処置: fixed D11 —— 受け口を `/satellite`・`/satellite/`・`/satellite/{*rest}` の 3 本にし、メソッド違いは fallback の 405 `method_not_allowed` にした。spec に写し、Scenario「版の後ろに道の無い求めも、版の一覧つきの 404 になる」「契約にある道へ違うメソッドで送ると、種別つきの 405 になる」を足した（Task 3.1）

## R9. 読み出しの記録の「実際に返した状態」は、今の middleware の作り（ハンドラの前に固定の状態で書く）と合わず、どちらにするかが design にも tasks にも無い。断りの行と書けないときの 500 の Scenario も無い
- 成果物: openspec/changes/st42-satellite-api-contract/design.md
- 根拠:
  - `crates/server/src/access_log.rs:158`「1 求め 1 行を**ハンドラの前**に書く。書けなければハンドラを呼ばずに 500」、:188-191 は通す求めを `("ok", 200)` で固定する
  - design D13 :157「通した求めの行は、ハンドラが返した状態で書く: …版の受け口の 404 → 結果 `forbidden`・状態 404」、:158「行を書けなければ、今の middleware と同じく 500 で断る」。ハンドラの後で書くなら「ハンドラを呼ばずに 500」は成り立たない。ハンドラの前で書くなら、状態は経路の形から決め打ちになる。どちらかが design に無い
  - spec :272 は、資格情報の無い 401 → `unauthorized`、`not_satellite` の 403 → `forbidden` の行も言うが、Scenario は :279（200 `ok`）と :284（404 `forbidden`）だけ。書けないときの 500 は design :158 にだけあり、正典 data-sensitivity :209 の 500 の Requirement は「記録の読み出しの求め」が対象なので、確かめる口には当たらない
- kind: technical
- 提案: D13 に「経路の形から状態を決めて、ハンドラの前に書く（確かめる口は 200、受け口は 404）」か「ハンドラの後で書く」かを決めて書き、Task 2.1・3.1 に写す。spec :272 に「行を書けなければ 500 で断り、ハンドラの処理を行わない」を足し、Scenario を 2 本足す（`not_satellite` の 403 の行 / 書けない状態で `GET /satellite/v1/me` が 500 で、衛星の情報を返さない）
- 処置: fixed D13 仮 —— `/satellite` の道だけ、行はハンドラの後に返す状態で書き、書けなければ応答を捨てて 500 `internal` にすると決めた（記録を返さず DB を変えない口なので行の無い読み出しは生まれない）。spec に写し、Scenario「衛星でない資格情報で…forbidden・403 の行が残る」「読み出しの記録に書けないときは…500…」を足した（Task 2.1）

## R10. 正典 data-sensitivity の「衛星の資格情報で断った求めの行の結果を、実際に返した応答（403）のとおりに書く」が、この change の後には偽になる
- 成果物: openspec/changes/st42-satellite-api-contract/specs/data-sensitivity/spec.md
- 根拠:
  - `openspec/specs/data-sensitivity/spec.md:208`「衛星の資格情報で断った求め…の行の結果を、実際に返した応答（403）のとおりに書く」。:220（導出元）「ST29 の衛星の行の結果はどれも 403 になる」
  - この change では、衛星の資格情報が版の受け口に届くと 404（結果 `forbidden`・状態 404。spec :284-287・design D13 :157）で断る。衛星の資格情報で断る応答は 403 だけではなくなる
  - design D13 :159 は、ST59 がこの Requirement を書き直しているので触らないと言う。ただし st59 の delta はこの文（st59 spec.md:63）と :76 を変えていない（字面が正典と同じ）
- kind: technical
- 提案: data-sensitivity の delta に、この Requirement の :208 の文だけを「（403。衛星向けの契約の道では、返した 404 も）」と直す MODIFIED を足し、導出元に ★ ST42 を 1 行足す。st59 が変える文とは別の行なので、archive の 3-way（`spec_merge.py`）で合わせられる（観点 7 の判定は独立のまま）
- 処置: fixed specs/data-sensitivity/spec.md —— 「誰がいつ読んだかを…」を MODIFIED にし、「（403）のとおり」の 1 文だけを 404・405 も含む形に直して ★ ST42 を足した。ST59 が変える文とは別。proposal の Capabilities と D13 にも書いた

## R11. MODIFIED の題「衛星の資格情報では、まだどの口も通らない」が本文と合わない。本文の「契約に載った口だけ通す」と、契約に載らない版の受け口が通ることも食い違う
- 成果物: openspec/changes/st42-satellite-api-contract/specs/satellite-platform/spec.md
- 根拠:
  - spec :291 の題は「まだどの口も通らない」のまま。:294 と :299 は確かめる口を通すと言う。正典の目次で題だけを読む人は、衛星の資格情報で通る口が無いと読む
  - spec :299「衛星の資格情報で通す口を、衛星向けの契約に載った口だけにする」。一方で、design D13 :156 は `/satellite/` で始まる経路をすべて `Verdict::Pass` にし、tasks.md :56 は「版の受け口は契約の口にしない」と言う。版の受け口は契約に載らないのに、衛星の資格情報が通ってハンドラに届く
- kind: technical
- 提案: 題を RENAMED で「衛星の資格情報では、衛星向けの契約の口のほかは通らない」にする（Scenario の名前は変えないので、印は今のまま）。:299 を「ハンドラに届くのは `/satellite/` で始まる道だけで、2xx を返すのは契約に載った口だけ」に直す
- 処置: fixed specs/satellite-platform/spec.md —— 本文の最後の文を「ハンドラに届けるのは `/satellite` の道だけで、2xx は契約に載った口だけ（ほかは受け口が 404・405）」に直した。題の RENAMED はしない: 題は正典の別の Requirement の本文（登録簿の導出元）・申し送り・試験の注釈から名前で引かれていて、変えるとそれらが黙って古くなる。本文と導出元の ★ ST42 が例外を言う

## R12. 1 つの Scenario が 2 つの木か 2 つの主張を束ねているものが、少なくとも 13 本ある
- 成果物: openspec/changes/st42-satellite-api-contract/specs/satellite-platform/spec.md
- 根拠: :36（契約のずれ / ページのずれ）・:96（取り消した / 外した）・:106（API の合言葉 / 画面のログイン）・:180（`openapi.json` だけ / `versions.json` だけ / `docs/x.md`）・:210（2027-07-09 / 2027-07-10）・:133（欄を消す / 型を変える）・:139（必須にする / 値を減らす）・:145（資格情報 / 許諾）・:151（上げ忘れで落ちる / 上げれば通る）・:175（読めない / 写しが無い）・:222（前の日 / 当日）・:309（MODIFIED。403 で断る / router の口の集合が一致する）・:47（LICENSE の中身 / README と CONTRIBUTING / 欠けた木）。
  印は Scenario に 1 つなので、片方の場合だけを当てる試験でも突き合わせは緑になる。:309 は、ST29 の試験 `every_route_refuses_satellite` が持つ印に、集合の一致という別の主張を足している
- kind: technical
- 提案: 落ちる側と通る側の組（:151・:175・:180・:210・:222）と、同じ規則の 2 つの入り口（:96・:106）は 1 本のままでよい。規則の違う組（:133・:139・:145）と :309 の集合の一致は分ける。:309 の集合の一致は「router の口は、内部の口の契約・衛星向けの契約・生存確認・版の受け口の口を合わせたものにちょうど一致する」の新しい Scenario にし、印の置き場を Task 4.1 に書く
- 処置: fixed specs/satellite-platform/spec.md —— 規則の違う組（:133・:139・:145 とその後の R6 で足したもの）と、契約・ページのずれ・LICENSE の正と誤を 1 本ずつに分けた。MODIFIED の Scenario から集合の一致を外し、ADDED の「サーバの経路は、2 つの契約の口と…ちょうど一致する」にした（Task 4.1）。落ちる側と通る側の組・同じ規則の 2 つの入り口は 1 本のまま

---

## 観点 3: 置き場の誤り

design の D 番号のうち観測できる振る舞いで specs に無いものは、R1（D12 の起点・丸め）・R3（D6 の欄の名前）・R8（D5 の「すべて JSON」）・R9（D13 の書き方と 500）で挙げた。specs に実装の名前（関数名・crate 名・列名）は無い（`tools/release/changes.sh code` と契約の欄の名前だけ。既存の正典と同じ使い方）。

## R13. INDEX の capability 表と proposal の Capabilities が合っていない。proposal の「ADDED 8 本」も、実際には 7 本
- 成果物: openspec/changes/st42-satellite-api-contract/proposal.md
- 根拠:
  - `docs/stories/INDEX.md:107` の `data-sensitivity` の行の Story に ST42 が無い。この change は data-sensitivity に MODIFIED を 1 本置いている（specs/data-sensitivity/spec.md）。INDEX にもその理由が無い
  - `docs/stories/INDEX.md:98` は ST42 を `record-envelope` に割り当てている。proposal :38-39 は delta を置かない理由（封筒は ST39）を書くが、INDEX には無い
  - proposal :34「ADDED 8 本」。`awk '/^## MODIFIED/{m=1} /^### Requirement/{...}' specs/satellite-platform/spec.md` → `ADDED 7 MODIFIED 1`
- kind: technical
- 提案: INDEX の data-sensitivity の行に ST42 を足し、注記に「衛星向けの契約の口を、API の合言葉・画面のログインの範囲から外す（MODIFIED 1 本。前倒しではない）」を書く。record-envelope の行の ST42 には「封筒は ST39 が同じ版に追加する」の注記を付けるか、ST39 へ移す。proposal の「8 本」を「7 本」に直す
- 処置: fixed proposal.md —— docs/stories/INDEX.md も直した。data-sensitivity の行に ST42 を足し、record-envelope の ST42 の注記（封筒は ST39）を訂正の節に書いた。proposal の本数を ADDED 7 本・MODIFIED は satellite-platform 1 本と data-sensitivity 2 本に直した

---

## 観点 4: tasks が検証を持つか

## R14. Task 4.1 の `satellite_routes` の下限 5 は満たせない（今ある試験は 2 本で、Task 4.1 はこの module に試験を足さない）
- 成果物: openspec/changes/st42-satellite-api-contract/tasks.md
- 根拠:
  - `grep -n "#\[tokio::test" crates/server/src/satellite_routes_tests.rs` → :104 `every_route_refuses_satellite`・:177 `exempt_routes_ignore_satellite` の 2 本。`satellite_routes` を名前に持つ試験は、ほかの file に無い（`grep -rn satellite_routes crates/server/src` は module の宣言と `use` だけ）
  - tasks.md :63-66 は既存の試験を直し、新しい試験は `st42_tests.rs` に置く。:67 の検証は `-ge 5`
- kind: technical
- 提案: 下限を `-ge 2` にする（増えないことを見るなら、試験の名前を `--exact` で 2 本名指す）。新しい試験の下限は、すでにある `st42_routes_ -ge 1` が持つ
- 処置: fixed 4.1 —— 下限を `satellite_routes -ge 2` にした。新しい試験は `st42_routes_ -ge 2`

## R15. 台本の `--self-test` の検証が rc=0 だけで、場合の数の下限が無い
- 成果物: openspec/changes/st42-satellite-api-contract/tasks.md
- 根拠:
  - tasks.md :80・:89・:97・:104 の検証は `tools/check-satellite-contract.sh --self-test` rc=0 だけ。Rust の検証が `-- --list` の本数で下限を見る（tasks.md :8）のと揃っていない
  - Task 9.3 の `check_scenarios.py` は印の字面が台本にあることしか見ず、その場合が走ったかは見ない。場合を 1 つも走らせない `--self-test` でも、全部の検証が通る
- kind: technical
- 提案: 各 Task の検証を `bash -c 'out="$(tools/check-satellite-contract.sh --self-test)" && test "$(printf "%s\n" "$out" | grep -c "^== Scenario: ")" -ge <N>'` の形にする。N は `--self-test` に置く Scenario の累計。Task 5 は「今の木に当てる」本番の場合と使い捨ての木の場合が混じっている（tasks.md :78「台本の `--self-test` と本番」）ので、Scenario ごとにどちらに置くかを tasks に書いてから N を決める（Task 6 の 9 本と Task 7 の 5 本、Task 8 の 1 本は、どれも `--self-test` と書かれている）
- 処置: fixed tasks.md —— `--self-test` の検証を `== Scenario:` の行の数の下限つき（Task 5 で 4・6 で 23・7 で 32・8 で 34）にし、本番も同じ形（3・4）にした。Scenario ごとに本番か `--self-test` かを tasks に書き分けた

## R16. `--self-test` で「作り直した契約」を差し替える口が design に無い。場合ごとに Rust を組み直す形になりうる
- 成果物: openspec/changes/st42-satellite-api-contract/design.md
- 根拠:
  - Scenario :129・:147・:153・:159・:171 の GIVEN は「作り直した版 1 の契約から…消した使い捨ての木」と言う。design D10 :126-128 は、作り直した契約を bin で作る（`cargo run`）とだけ言い、検査に作り直した契約を外から渡す口を持たない
  - 渡す口が無いと、場合ごとに使い捨ての木の Rust を書き換え、`ashiato-server` を組み直すことになる（D9 の場合だけで 9 つ）。手本の `tools/check-contract-version.sh` は、使い捨ての木に JSON の材料を置いて当てていて、組み直しはしない（:174 以降）
  - 本番の検査も `cargo run` を含む（D10 (1)）。tasks.md :80・:89・:97 は、それを `scripts/quiet-run` を通さずに走らせている（AGENTS.md「数十秒以上かかるもの」）
- kind: technical
- 提案: D10 に、作り直した契約とページを置いたフォルダを受ける引数（例: `--generated <dir>`。`--self-test` の中だけで使い、本番の CI では使わない）を足す。「写しを書き換えても通らない」（:169）は、作り直したもの・コミットされた写し・比べる相手の 3 つが別の材料であることで確かめる。tasks の本番の検証は `scripts/quiet-run` を通す
- 処置: fixed D10 —— `--generated <フォルダ>` を足し、`--self-test` は材料の JSON で当てる（Rust を組み直さない）と書いた。本番の cargo を含む検証は `scripts/quiet-run` を通した

## R17. `changes.sh code` は `.md` で終わるファイルを除くので、ページ（`docs/satellite-api/**/README.md`）だけの PR や、根の README・CONTRIBUTING だけの PR では検査が走らない
- 成果物: openspec/changes/st42-satellite-api-contract/specs/satellite-platform/spec.md
- 根拠:
  - `tools/release/changes.sh:7` `grep -vE '^(docs/|openspec/)|\.md$'`。`docs/satellite-api/` を先に拾う行を足しても、根の `README.md`・`CONTRIBUTING.md` だけの PR では `rust` の job が飛ぶ
  - spec :120 は「`docs/satellite-api/` の下だけを変えた PR でも、この検査を走らせる」と言う。Scenario :180-183 が渡すのは `v1/openapi.json` と `versions.json` の 2 つだけで、`.md` の除外に当たるページは渡していない
  - 検査は README・CONTRIBUTING のライセンスの節を見る（D10 (2)・Scenario :47-50）。根の README の節だけを消す PR は、検査に当たらずに main に入る
- kind: technical
- 提案: Scenario :182 の一覧に `docs/satellite-api/v1/README.md` だけの場合を足す。README・CONTRIBUTING の節の確かめは、`.md` だけの PR でも走る job（`chain` など）に分けて置くか、`code` に `^(README|CONTRIBUTING)\.md$` を足す。足すなら R18 の判断と合わせる
- 処置: fixed D10 —— 新しい判定 `satellite` が `docs/satellite-api/` の下（`.md` を含む）と根の `README.md`・`CONTRIBUTING.md` を拾う。Scenario の一覧に `v1/README.md` と `CONTRIBUTING.md` を足した（Task 8.1）

## R18. ST61 は `changes.sh code` を「製品に触れたか」の判定（番号を付けるか）に使っている。ST42 が `code` を広げると、`docs/satellite-api/` だけの merge に製品の番号が付き、ST61 の spec の文が偽になる。proposal の「走っている change との重なり」にこの件が無い
- 成果物: openspec/changes/st42-satellite-api-contract/design.md
- 根拠:
  - `openspec/changes/st61-auto-version-bump/specs/distribution/spec.md:5-6`「差分が製品（`docs/`・`openspec/`・名前が `.md` で終わるファイル以外…）に触れていれば、次の番号を 1 つ付ける」「差分が `docs/`・`openspec/`・`*.md` だけの commit には番号を付けない」
  - `openspec/changes/st61-auto-version-bump/design.md:68`「差分が `tools/release/changes.sh code` で真」。tasks.md :57「差分の判定は `tools/release/changes.sh code` を呼ぶ（判定を写さない）」。ST61 の tasks 6.1 は `changes.sh` に `base` と `--self-test` も足す
  - ST42 design D10 :132・tasks 8.1 は、`code` に `^docs/satellite-api/` を「コードとして数える」行で足す。両方が入ると、`docs/satellite-api/` だけの merge は `code` が真になり、ST61 の番号付けが製品の番号を 1 つ進める（ST61 spec:6 と食い違う）
  - proposal :52 は ST61 との重なりを `check-openapi.sh`・`docs/openapi.json`・`ci.yml` だけで書いていて、`changes.sh` に触れていない。spec_overlap の候補には出ない（ST42 は distribution に delta を持たない）
- kind: conflict
- 提案: `code` の意味を変えず、`changes.sh` に別の判定（例: `satellite`。`docs/satellite-api/` に触れたら真）を足し、`changes` の job の出力と `rust` の job の `if` を `code || satellite` にする。こうすれば ST61 の「製品」の意味は変わらず、2 つの change は順に依らない。proposal の重なりの節に `changes.sh` を足す。あわせて、main への push の比べる相手（D10 の `github.event.before`）を ST61 の「前に成功した main の run」（ST61 spec の Requirement「main への push の CI は、前に成功した main の run からの差分で出し分ける」）に揃えるかを D10 に書く（取り消された run の commit の差を飛ばさないため）
- 処置: fixed D10 仮 —— 提案どおり `code` の意味を変えず、`changes.sh satellite` を足して `rust` の job の `if` を `code || satellite` にした。Scenario「衛星向けの契約の置き場だけの変更は、製品に触れた変更に数えない」を足し、proposal の重なりの節に ST61 の `changes.sh` を書いた。main への push の比べる相手は `changes` の job に合わせる（ST61 が変えたら追う）と D10 に書いた

## R19. 契約のページがブラウザで整形されて見えることを、機械も人も確かめていない（premises P3 の確かさは 2）
- 成果物: openspec/changes/st42-satellite-api-contract/tasks.md
- 根拠:
  - FR-103「リポジトリの中のブラウザで読めるページとして公開する」。premises P3（GitHub は Markdown を整形して見せる）の確かさは 2 で、出所は「いつもの使い方」
  - Scenario :42-45 が見るのは文字列が含まれることだけ。入口のページから版のページ・契約の JSON・LICENSE へのリンクの先があることは、どの Task も見ない。確認バッチに出す項目も tasks に無い
- kind: technical
- 提案: 検査（Task 5.1）に、2 枚のページの相対リンクの先が木にあることを足す（Scenario の THEN :45 にも「リンクの先がある」を加える）。整形の見え方は、merge の後の確認バッチで GitHub 上の `docs/satellite-api/README.md` を開く項目として tasks に書く
- 処置: fixed 5.1 —— 検査がページの相対リンクの先を見ることを Requirement・Scenario・D10 に足した。整形の見え方は人間の確認でなく、Task 5.1 で `gh api /markdown`（GFM）の出力を見て premises P3 を確かさ 4 に上げる

---

## 観点 5: Story が要件の今の本文と一致しているか

`check_chain.py` の観点 8 は通っている。`docs/stories/ST42.md` の満たす要件の逐語（FR-89・FR-95・FR-103）は、`docs/requirements.md:919-931` の今の本文と一致する。完了の判定 4 つは specs の Requirement と Scenario に写っていて、deep の推奨とも矛盾しない（「同じ口を両方に載せるかは衛星の書き込みを作る Story の上流」は、spec :6 が「衛星だけが使う口」に狭めて守っている）。壊してはいけないもの 2 つは Task 1.1（`tools/check-openapi.sh`）と Task 4.2 が持つ。

## R20. FR-103 の「記録の封筒の形」を ST39 に送ったが、ST39 の satisfies に FR-103 が無い。ST42 が archive されると、封筒の部分を持つ Story が鎖の上で見えなくなる
- 成果物: openspec/changes/st42-satellite-api-contract/proposal.md
- 根拠:
  - proposal :44「記録の封筒の形: …ST39 が衛星向けの契約の同じ版に追加として載せ、そこで FR-103 の封筒の部分を満たす（deferred ST39。`docs/handoff/ST39.md`）」。`docs/handoff/ST39.md:61` にも書かれている
  - `docs/stories/INDEX.md:63` の ST39 の満たす要件は `FR-25, FR-64, FR-65, FR-94, FR-96, FR-98, FR-99` で、FR-103 が無い。鎖（`check_chain.py`）は FR-103 を ST42 だけで回収済みと数える
- kind: defer
- 提案: ST39 の satisfies に FR-103 を足す（INDEX と `docs/stories/ST39.md`）か、INDEX の ST39 の行に「FR-103 の封筒の部分（ST42 から）」の注記を足す。申し送りだけでは、鎖の検査にもページにも出ない
- 処置: deferred ST39 —— 封筒の形は `docs/handoff/ST39.md:61` にあり、ST39 の上流がブリーフの「この Story に宛てられたもの」で読む。ST39 の satisfies に FR-103 を足すか（stories.json の判断）は ST39 の上流が決める。INDEX の訂正の節にも ST39 が載せると書いた

---

## 観点 6: 答えが写ったか

観点 6: 該当なし（第 1 回の先読み `spec --draft`。`--answers` ではない）。

---

## 観点 7: 走っている change との前提関係
- st59-change-password-from-web / 誰がいつ読んだかを、追記のみの読み出しの記録に残す: 独立（ST42 はこの Requirement を書き直さず、導出元で名指すだけ。衛星向けの道の行は satellite-platform の ADDED に置いていて、使う列（状態 `status`・結果 `ok` / `unauthorized` / `forbidden`）は main の移行 `202609290900_access_control.sql`・`202610241200_satellite_credentials.sql` にすでにあり、ST59 が足す振る舞いを当てにしていない。R10 の直しを入れても、直す文（正典 :208）を st59 は変えていない（st59 spec.md:63 は字面が正典と同じ）ので、3-way で合わせられる）

ST29（requires）は archive 済み。前提で requires に無い change は無いので、requires の R は立てない。
spec_overlap の候補の外で、ST61 とは `tools/release/changes.sh` の `code` の意味で食い違う（R18）。仕様の前提ではなく、道具の判定を 2 つの change が別の意味で使う件なので、ここの判定には入れない。CH21・CH26 は `ci.yml` を触るが、どちらも satellite-platform・data-sensitivity に delta を持たず、ST42 の Scenario が当てにする振る舞いも足さない。
