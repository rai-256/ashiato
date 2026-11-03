# ST44 spec レビュー（spec --draft。deep.md の Q1〜Q3 は推奨の答えを採ったものとして読んだ）

## 機械の検査（最初に写す）

- `openspec validate st44-collector-credential-scope --strict` → `Change 'st44-collector-credential-scope' is valid`（rc=0）
- `python3 scripts/check_chain.py .` → `chain: OK (0 件 / 未回収 0 件 / warn 0 件)`（rc=0。要件 171 / Story 58 / 扉 35。再生成との一致も OK）
- `python3 scripts/check_scenarios.py . st44-collector-credential-scope` → `scenarios: FAIL (担保なし 21 件 / 名無しの確認待ち 0 件)`（rc=1）。
  FAIL はすべてこの change の新しい Scenario（data-sensitivity 18・record-envelope 1・subjective-log 2）で、tasks.md:12 の「新しく印を置くのは 21 本」と数・名前が一致する。draft の段では想定どおり。
  置き直した Requirement（REMOVED + ADDED）の残りの 16 本と、MODIFIED の残りの Scenario は既存の印で担保ありになっている（FAIL に出ない）
- `python3 scripts/review_triage.py . st44-collector-credential-scope` → `triage: FAIL (19 件)`（rc=1）。対象は `review/deep.md` の処置（R1〜R10）→ R1
- `python3 scripts/spec_overlap.py candidates . st44-collector-credential-scope` → `[]`（0 件）

## R1. review/deep.md の処置が review_triage を通らない（19 件）
- 成果物: openspec/changes/st44-collector-credential-scope/review/deep.md
- 根拠: `python3 scripts/review_triage.py . st44-collector-credential-scope` → `triage: FAIL (19 件)`。内訳:
  R1（loss=discarded）・R2（loss=exported）・R5（loss=discarded）・R6（loss=uncaptured）が `fixed` で閉じている（「人間に返すもの。fixed で閉じられない」）/
  R7・R8（kind=conflict）が `fixed D<n> 仮` の形でない / R3 の `rejected` に再現コマンドか `<file>:<行>` が無い /
  R1・R2・R5〜R10 の `fixed` の指す先が `Q1`・`Q2`・`403` で、D<n> でも <x.y> でも change 直下のファイル名でもない
- kind: technical
- 提案: 問いの段の処置を、spec の段の成果物を指す形に直す。いまは design がある: R1 → D6、R2 → D7、R8 → D10、R9 → specs（proposal の Capabilities）、R10 → D5。
  loss のある R1・R2・R5・R6 は人間に返す問い（Q1・Q2 の irreversible に写した）なので `escalated` にする。R7 は `fixed D2 仮` か `escalated`、R3 は根拠（`docs/requirements.md:1185-1186` など）を書く
- 処置: fixed deep.md —— review/deep.md の処置を直した: loss のある R1・R2・R5・R6 と R7 は escalated（Q1・Q2 の本文に写してあり、deep.md の Q1・Q2 に由来の行を足した）、R8 は fixed D10 仮（provisional.jsonl）、R9 は fixed proposal.md、R10 は fixed D5、R3 の rejected に `tools/smoke.sh:25`・`docs/requirements.md:1185-1186` を書いた

## R2. ADDED の Requirement に、置き直す前の導出元「API の合言葉の読める範囲は変えない」が残り、同じ塊の SHALL と食い違う
- 成果物: openspec/changes/st44-collector-credential-scope/specs/data-sensitivity/spec.md
- 根拠: specs/data-sensitivity/spec.md:140（SHALL「API の合言葉で通る口を…許可の一覧に限る」）と :145（導出元「API の合言葉の読める範囲は変えない（Q4。本人が受け入れた…）」）。
  :145 は正典 openspec/specs/data-sensitivity/spec.md:124 の写しのまま。REMOVED の Migration（:6）は「その 1 文と 1 つの Scenario のほかは同じ中身」と書くが、覆した側の文が残っている
- kind: technical
- 提案: :145 を「ST28 の深掘り Q4 では範囲を変えなかった。ST44 で締めた（★ ST44）」のように経緯として書き直すか消す。Migration の「ほかは同じ中身」も、導出元を直したことに合わせる
- 処置: fixed proposal.md —— specs/data-sensitivity の置き直した Requirement の導出元を「ST28 の深掘り Q4 では範囲を変えなかった」の経緯に直し、REMOVED の Migration と★の「同じ中身」を「その 1 文・導出元の 2 行・1 つの Scenario のほか」に直した

## R3. data-sensitivity の ADDED は「ログインの口も『すべての API 要求』の例外ではない」と書き、record-envelope の MODIFIED はログインを例外に入れた
- 成果物: openspec/changes/st44-collector-credential-scope/specs/data-sensitivity/spec.md / specs/record-envelope/spec.md
- 根拠: specs/data-sensitivity/spec.md:144（「ログインの口も『すべての API 要求は資格情報を要求する』（record-envelope）の例外ではない —— 資格情報が画面の合言葉である」）/
  specs/record-envelope/spec.md:5（「生存確認・ログイン（`POST /session`）・ログアウトの口を除くすべての API 要求に」）・:12。
  2 つの正典が同じ口について逆のことを言う（archive すると両方が正典に入る）
- kind: technical
- 提案: data-sensitivity:144 を record-envelope に合わせる（「ログインの口は record-envelope の例外の 1 つで、資格情報の代わりに画面の合言葉そのものを受ける」）。
  record-envelope:12 の「ログインの資格情報は画面の合言葉そのもの」との言い分けを 1 つにする
- 処置: fixed proposal.md —— specs/data-sensitivity の導出元を「ログインの口は record-envelope の例外の 1 つで、資格情報の代わりに画面の合言葉そのものを受ける」に直し、record-envelope と揃えた

## R4. 「画面のログインの印で通る口の範囲を変えない（全部の口を通る）」は今の振る舞いと違う。契約の版の口は画面のログインを 403 で断っている
- 成果物: openspec/changes/st44-collector-credential-scope/specs/data-sensitivity/spec.md / design.md（D3）
- 根拠: specs/data-sensitivity/spec.md:244 / design.md:53（D3「画面のログインの印は全部の口を通る（今どおり）」）/
  crates/server/src/contract_version.rs:52-64（`GET /contract` は `Caller::ApiToken` だけ。`_ => Err((StatusCode::FORBIDDEN, …))`）/
  deep-questions.json の Q3 の context（「契約の版の口は、いまも画面のログインでは断っている（ST50）」）。
  括弧の「全部の口を通る」を字面どおり試験すると `GET /contract` で落ちる
- kind: technical
- 提案: :244 を「画面のログインの印で通る口の範囲を変えない（送信契約の版の口は今どおり断る）」に直し、D3 の 1 行目も同じに直す
- 処置: fixed D3 —— specs/data-sensitivity の SHALL を「送信契約の版の口は今どおり断り、ほかの口は全部通る」に、D3 の 1 行目も同じに直した

## R5. D10「衛星の形でない違う Bearer と有効なログインの印の組は印で通る」は観測できる振る舞いなのに Scenario が無く、design にだけある
- 成果物: openspec/changes/st44-collector-credential-scope/design.md / specs/data-sensitivity/spec.md
- 根拠: design.md:104-107（D10。ST29 の申し送りを閉じる決定）/ tasks.md:56（`st44_scope_wrong_bearer_with_cookie` は Scenario の印の無い試験）/
  深掘り Q1 の context の AI の既定（「衛星の形（`ashsat_`）でない違う Bearer と有効なログインの印の組は、いまのまま印で通す」）。
  同じ列の D11（両方付けた求め）と D12（`GET /session`）は spec に Scenario があるが、D10 だけが無い。`openspec archive` は design を正典に写さないので、決定が正典から落ちる
- kind: technical
- 提案: data-sensitivity の「API の合言葉で通る口は…許可の一覧だけ」に SHALL 1 文と Scenario を足す
  （例「WHEN 有効なログインの印と、衛星の形でも API の合言葉でもない値の Bearer を付けて記録の読み出しを求める THEN 読み出しは通る」）。tasks の試験にその印を付ける
- 処置: fixed D10 —— specs/data-sensitivity の許可の一覧の Requirement に SHALL 1 文と Scenario「一致しない Bearer とログインの印を両方付けた読み出しは印で通る」を足し、tasks 1.1 の `st44_scope_wrong_bearer_with_cookie` に印を付けた。D10 は（仮）

## R6. record-envelope の Scenario「資格情報を求めない口は…3 つだけ」は、今の middleware のままだと 401 にならない口がある。どちらで満たすかが design にも tasks にも無い
- 成果物: openspec/changes/st44-collector-credential-scope/specs/record-envelope/spec.md / design.md（D9）/ tasks.md（2.1）
- 根拠: crates/server/src/access_log.rs:167-187（資格情報の無い求めは 401 の行を書いた後も `next.run(req)` でハンドラへ渡す）/
  ハンドラの引数の取り出しは `authorize` より先に走る: lib.rs:2955 `Query<StaysQuery>`（`date: String` は必須。lib.rs:2919-2922）・lib.rs:3105 `Query<ReminderQuery>`（:3093-3096）・lib.rs:3973 `Json<PlaceCreateRequest>`（`id: Uuid` は必須。places.rs:368-373）。
  引数や本文を欠いた求めはここで 400 / 422 になり、401 に届かない。tasks.md:65 は道の `{…}` を埋めるだけで、問い合わせの引数と本文は埋めない（`satellite_routes_tests.rs:89-102` の `request` は本文 `{}` だけ）。
  衛星の試験（`every_route_refuses_satellite`）が通るのは、middleware がハンドラを呼ばずに 403 を返すからで、資格情報の無い求めには当てはまらない
- kind: technical
- 提案: 次のどちらかを design D9 に決めて書く。(a) middleware が資格情報の無い求めを、行を書いた直後にハンドラを呼ばず 401 で返す（引数の検査より先に断る。振る舞いが変わるので Scenario の THEN に「引数や本文の形にかかわらず」を足す）。
  (b) 試験が口ごとに形の合う引数と本文を作る（口が増えるたびに試験の手当てが要る）。(a) なら tasks 1.1 に middleware の変更を足す
- 処置: fixed D9 —— (a) を採った。middleware が資格情報の無い求めを、行を書いた直後にハンドラを呼ばず 401 で返す。record-envelope の SHALL に「引数や本文を読む前に」、Scenario の WHEN に「引数も本文も付けず」、THEN に「引数や本文の形にかかわらず」を足し、tasks 1.1・2.1 に書いた

## R7. smoke.sh が呼ぶ `tools/seed.sh`・`tools/seed.d` と `tools/record-env.sh` は API の合言葉で一覧の外の口を呼ぶが、どの Task にも検査の対象にも無い
- 成果物: openspec/changes/st44-collector-credential-scope/tasks.md / proposal.md（Impact）/ design.md（D13）
- 根拠: tools/smoke.sh:1118（`tools/seed.sh normal` を走らせる。seed.sh:410 `[ "$MODE" = "normal" ] || exit 0` の後ろに下の行があり、normal で走る）/ tools/seed.sh:21（`AUTH` は `Bearer $API_TOKEN`）・:426（`GET /attributes`）・:433（`POST /attributes/kinds`）・:582（`POST /places`）・:591-592（`GET /places`・`/places/candidates`）/
  tools/seed.d/202610141000_st17-daily-feeling.sh:181（`POST /stays/erase`）・:202（`GET /feelings`）/ tools/record-env.sh:47（`GET /archives/status` を Bearer で）。
  tasks 5.2 は smoke.sh と smoke.d だけを直し、6.2・7.1 の `tools/check-api-token-reads.sh` の対象（`tools/recording web/e2e`・`tools/verify-prep.sh tools/st25-seed-check.sh tools/smoke.sh tools/smoke.d`）にも seed.sh・seed.d・record-env.sh が無い。
  Task 5.2 の検証 `tools/smoke.sh` rc=0 は、seed.sh の 403 で落ちる。
  また 5.2 の規則は「送る / 読む」で分けるが、一覧の外の**書く**求め（smoke.d/202610251200_st23-purge.sh:24・:31・:44 の `POST /stays/*`、seed.sh の `POST /places` など）はどちらの列にも無い
- kind: technical
- 提案: 5.2（か新しい 5.3）に seed.sh・seed.d・record-env.sh を足し、規則を「許可の一覧の 9 つだけ AUTH、それ以外（読む・書く・作り直す）は印」と一覧の側で書く。
  7.1 の `check-api-token-reads.sh` の対象に `tools/seed.sh tools/seed.d tools/record-env.sh` を足す（または `tools` 全体を渡し、例外を明示する）。proposal の Impact と D13 の列にも足す
- 処置: fixed D13 —— tasks 5.2 に seed.sh・seed.d・record-env.sh を足し、規則を「許可の一覧の 9 つだけ AUTH、それ以外は印」と一覧の側で書いた。7.1 の `check-api-token-reads.sh` の対象に 3 つを足し、proposal の Impact と D13 にも書いた

## R8. tasks 4.1 の `CS server_startup`（`-- --list` で 10 本以上）は満たせない
- 成果物: openspec/changes/st44-collector-credential-scope/tasks.md
- 根拠: tasks.md:88 / crates/server/tests/server_startup.rs の試験の関数のうち名前に `server_startup` を含むのは 9 本
  （`server_startup_refuses_privileged_role` … `server_startup_without_archive_user_says_scheduled_fetch_is_off`。`st48_server_*`・`st51_bin_*` は含まない）。
  ST44 が足す 2 本は `st44_session_key_*` なので、作業を正しく終えても 9 本で検証が落ちる
- kind: technical
- 提案: 下限を 9 に直すか、絞り込みを外して `cargo test -p ashiato-server --test server_startup -- --list` の総数（いま 33 本、ST44 後 35 本）を下限にする。
  意図が「既存の起動の試験が `WEB_SESSION_KEY` を渡して通る」なら、後者の方が全部を見る
- 処置: fixed 4.1 —— `CS server_startup` の下限を 9 本に直した（ST44 の 2 本は `CS st44_session_key_` が数える）

## R9. 推奨の Q2（端末ごとに発行・ST56）は要件の本文を変えると問い自身が書いているのに、要件へ戻すものとしても申し送りとしても置かれていない
- 成果物: openspec/changes/st44-collector-credential-scope/proposal.md / design.md（D2）/ docs/requirements.md
- 根拠: deep-questions.json の Q2 の context（「端末ごと（資格情報ごと）に残す案は書き手の粒度を上げるので、選んだら要件に★を足す」）/ deep.md:13（推奨は端末ごと）/
  docs/requirements.md:1185-1186（PERM-10★ 2026-10-11「端末は送り手の申告のまま」）/ `grep -n "2026-11-02\|ST44" docs/requirements.md` は FR-128 の 1 行（:1075）だけで、★ の改訂は無い /
  design.md:42-49（D2 は ST56・ST37 への申し送りだけで、要件の改訂に触れない）/ docs/stories/ST56.md は `satisfies: [FR-128]` だけ（review/deep.md R6 の根拠）
- kind: conflict
- 提案: 推奨のまま答えが来たら、PERM-10★ の「端末は送り手の申告のまま」を改める ★（日付つき）を requirements.md に入れる担い手を決めて書く。
  ST44 で入れるなら「要件へ戻すもの」として deep.md / proposal に挙げ、ST56 に任せるなら D2 の申し送りに「要件の★も ST56 の /requirements で」と書く
- 処置: escalated —— Q2 自身が「選んだら要件に★を足す」と本人に示している（Q2 の答え待ち）。推奨のまま答えが来たら record の段が PERM-10 の★にその日の日付の★を足す、と design D2 と deep.md の Q2 の由来に書いた。ST44 の spec と試験はこの★に依らない
  → 答え（2026-11-02）: 本人は Q2 で「共有の 1 本のまま」を選んだ。書き手の粒度は変えないので PERM-10 の★は足さない。配り方は FR-128 ★ 2026-11-02 が持つ（record の段が足した）。design D2 を書き直した（spec --answers Q2）

## R10. ST56 への申し送り（端末ごとの資格情報・どの資格情報で届いたかの欄・種類を増やすときの振り分け）の受け皿
- 成果物: openspec/changes/st44-collector-credential-scope/design.md（D2・D6）/ proposal.md:65-66
- 根拠: proposal.md:65（「申し送り（`deferred`）… review/spec.md の処置に書く」）/ design.md:45-46 / review/deep.md R1 の処置（「ST56 への申し送りは record の段で書く」）。
  このファイルにその指摘が無いと、処置を付ける場所が無い
- kind: defer
- 提案: ST56 へ: 画面での端末ごとの発行・取り消しと入れた後の設定の口、記録の行と読み出しの記録の行にどの資格情報で届いたかを書く欄、
  `Caller` に種類を足すときは許可の一覧（D4）と契約の版の門の `match`（D6）に振り分けを足すこと
- 処置: rejected: 深掘り Q2 で本人が「共有の 1 本のまま」を選んだ（`deep-answers-1.txt` の Q2、`docs/requirements.md` FR-128 ★ 2026-11-02「端末ごとには発行しない」）。端末ごとの発行も、どの資格情報で届いたかの欄も作らないので申し送る中身が無い。共有の値を設定させる口は FR-128 の本文が ST56 に既に持たせている（整合の確認 docs/audit/2026-11-02-st44-fr128-consistency.md の C2。2026-11-02 に deferred ST56 から改めた）

## R11. ST37 への申し送り（書き手の列を作るとき、資格情報の識別子の置き場を空けておく）の受け皿
- 成果物: openspec/changes/st44-collector-credential-scope/design.md（D2）/ proposal.md:66
- 根拠: design.md:47（「ST37 への申し送り（`deferred ST37`）」）/ review/deep.md R6（記録の行は凍結されるので届いた時に書くしかない。docs/stories/ST37.md:4 は layer 3 で ST56 より先に形が決まる）
- kind: defer
- 提案: ST37 へ: 書き手の列を作るとき、収集アプリの資格情報の識別子を書く欄の置き場を空けておく（ST56 が埋める）
- 処置: rejected: 深掘り Q2 で本人が「共有の 1 本のまま」を選び、どの端末の資格情報で届いたかはずっと持たない（`deep-answers-1.txt` の Q2、`docs/requirements.md` FR-128 ★ 2026-11-02）。埋める者のいない欄を ST37 に空けさせない（整合の確認 docs/audit/2026-11-02-st44-fr128-consistency.md の C2。2026-11-02 に deferred ST37 から改めた）

## R12. 本文だけが言っていて Scenario が言っていないこと（鍵の値を出さない・一覧の中の読みを残す）
- 成果物: openspec/changes/st44-collector-credential-scope/specs/data-sensitivity/spec.md / design.md（D5）
- 根拠: specs/data-sensitivity/spec.md:313（SHALL「ログインの印の鍵の値を、ログ・読み出しの記録・応答・DB に出さない」）に対応する Scenario が無い。
  :325-328 の起動の拒否の Scenario の THEN は「理由に出して終わる」までで、値が出力に無いことを言わない（tasks.md:83 の試験は「出力に値が無い」を見るつもりでいる）/
  design.md:72（D5「一覧の中の読み（`/contract` など）は今どおり 1 回 1 行」）は観測できるが、spec の :57 は「記録の読み出し」としか言わず、`/contract`・`/outages`・`/feelings/reminder` を API の合言葉で読んだ行が残るかを決めていない
- kind: technical
- 提案: :325-328 の THEN に「出力のどこにも鍵の値が無い」を足す（ST29 の衛星の行の Scenario と同じ書き方）。
  一覧の中の 4 つの読みを残すなら、許可の一覧の Scenario に「読み出しの記録に資格情報の種類が API の合言葉・結果が 200 の行が 1 行増える」を 1 本足す。残さないなら D5 を直す
- 処置: fixed D5 —— 起動の拒否の 2 本の THEN に「出力のどこにも鍵（と合言葉）の値が無い」を足した。一覧の中の読みを残す（D5）を Scenario「API の合言葉で通知に要る読みをすると、読み出しの記録に 1 行残る」にし、tasks 1.1 の `st44_scope_allowed_reads_logged` に印を付けた

## R13. 検証の言葉が測れない・1 つの Scenario に主張を束ねている（軽い）
- 成果物: openspec/changes/st44-collector-credential-scope/specs/data-sensitivity/spec.md
- 根拠: :257（THEN「どの求めも**今どおり**受け付けられ」—— 何を見れば「今どおり」かが無い。応答の符号も受け付けた件数も書いていない）/
  :231-234（「送れるが読めない」は送る・読むの 2 つ。送る側は :254-257 と重なる）/ :289-292（403 と読み出しの記録の行の 2 つ）/ :325-328（3 つの値の起動の拒否を 1 本に）/ :304-307（4 つの口の「通る」を 1 本に。「通る」が 200 か 2xx かが無い）
- kind: technical
- 提案: :257 を「どの求めも 200 で、送った件数が受け付けられた件数として返る」のように符号と数で書く。:304-307 の「通る」は 2xx と書く。
  束ねた Scenario は、片方だけ緑になる形にしない（試験 1 本で全部を断言することを tasks に書くか、Scenario を分ける）
- 処置: fixed proposal.md —— specs/data-sensitivity の送る Scenario の THEN を「どの求めも 200 で、送った 1 件ずつが受け付けられた結果として返り」、画面のログインの Scenario を「2xx で応える」に直した。束ねた Scenario は tasks の試験 1 本がすべての断言を持つ（1.1 の各試験の括弧に断言を書いた）ので分けない

## R14. proposal と design の字面が spec の形と合っていない
- 成果物: openspec/changes/st44-collector-credential-scope/proposal.md / design.md
- 根拠: proposal.md:40（data-sensitivity を「ADDED 1 つ・ADDED 1 つ・…を外す（MODIFIED）」と書くが、spec は「画面から記録を読むには…」を REMOVED し、名前を替えた ADDED で置き直している。ADDED は 3 つ・REMOVED 1 つ・MODIFIED 2 つ）/
  design.md:5（「AI が既定で決める」を「8 つ」と書き、続けて 9 つを写し先つきで並べる。deep-questions.json の Q1 の context の箇条も 9 つ）
- kind: technical
- 提案: proposal の Capabilities を spec の delta の形（REMOVED + ADDED の置き直しとその理由）に直す。design の「8 つ」を「9 つ」に直す

## 観点 1: deep の決定が正典に写っているか
- Q1（推奨: いまの API の合言葉を送るだけに締める）→ D1 と data-sensitivity の ADDED 2 つ・Scenario（:231-234・:254-307）に写っている。覆した旧い文は R2 の 1 か所が残る
- Q2（推奨: 端末ごと・ST56）→ D2 と data-sensitivity:252 の導出元。ST44 の振る舞いは変えないので Scenario は要らない。要件の改訂の担い手が無い → R9。申し送り → R10・R11
- Q3（推奨: いまのまま）→ D3 と :244 の SHALL・:304-307 の Scenario。括弧の字面が今と違う → R4
- review/deep.md の処置で Q1 の context に足した AI の既定 9 つ: 許可の一覧（D4・Scenario あり）/ 403 と読み出しの記録（D5・:123-126 と :289-292）/ `_` の無い `match`（D6・実装の決定で spec に置かないのが正しい）/
  印の鍵（D7・D8（仮）・ADDED「画面のログインの印の鍵はサーバだけが持つ」と MODIFIED の Scenario 2 本）/ 資格情報を求めない 3 つの口（D9・record-envelope の Scenario。検証の形は R6）/
  違う Bearer と印の組（D10・**Scenario 無し** → R5）/ 正典の書き直し 4 か所（data-sensitivity・subjective-log・satellite-platform は写った。collection-coverage は整合のまま変えない —— `openspec/specs/collection-coverage/spec.md:1496`「ST44 は、この口を例外に残す」）/
  FR-92（Non-Goals）/ 落とした電話の手順（D14・tasks 7.2）
- 数値: 印の鍵の 16 文字は Scenario に 15 文字の値で入っている（:327）。許可の一覧の 9 つは SHALL（:238-240）に全部名指しされている

## 観点 2: Scenario が検証可能か
- R6（record-envelope の 3 つの口の Scenario が今の middleware では 401 にならない）・R12（本文だけが言うこと）・R13（測れない言葉と束ね）
- 「API の合言葉と DB の写しだけでは画面の合言葉を確かめられない」（:330-333）は、鍵が API の合言葉でないことしか測らない（名前の主張より狭い）。主張としては成り立つので指摘にはしない

## 観点 3: 置き場の誤り
- D10 → R5。D5 の「一覧の中の読みを 1 行残す」→ R12
- specs/ に関数名・crate 名・列名は無い（道と方法・`Secure`・`Max-Age` は外から観測できる契約）。確かめた範囲: 4 つの delta spec の全文
- proposal の Capabilities（4 つ）と specs/ のディレクトリ（data-sensitivity・record-envelope・subjective-log・satellite-platform）は一致。
  INDEX.md の表に record-envelope・subjective-log・satellite-platform へ ST44 を足し、訂正の節（docs/stories/INDEX.md:360-364）に理由がある。device-collection・desktop-collection に delta が無い理由も書いてある。字面のずれ → R14

## 観点 4: tasks が検証を持つか
- R7（seed.sh・seed.d・record-env.sh の抜けで smoke が落ちる）・R8（`CS server_startup` の下限）
- 確かめて成り立っていたもの: crate 名 `ashiato-server`（crates/server/Cargo.toml:2）/ `satellite_routes_tests.rs` の `contract_operations`（:54）・`is_exempt`（:24）・`fill`（:77）・`every_route_refuses_satellite`（:105）は実在 /
  `CT st50_` は `fn st50_` が 50 本で 20 以上 / `CT web_session` は `web_session_tests` の mod の道に当たるので 15 以上 / `tests/server_startup.rs` の `start_with_web_password`（:303）は実在 /
  web/e2e で Bearer を持つのは tasks 6.1 の 8 ファイルと一致 / `e2e/.auth/state.json`（global-setup.ts:7）は実在 / `docs/collector-contract.md`・`tools/check-openapi.sh`・`tools/ci-db-env.sh`・`tools/record-env.sh`・`tools/env-init.sh`・`tools/check-panic-log.sh` は実在
- 軽いもの（R にしない）: tasks 2.1 の「`is_exempt` と同じ 3 つになることも確かめる」は、`is_exempt` が `satellite_routes_tests` の中の非公開の関数なので、そのままでは別の mod から呼べない。確かめる手段（`pub(super)` にするか、同じ 3 つを 1 か所の定数に寄せる）を書くと迷わない
- 粒度: 1.1 は 1 つの `- [ ]` に、新しい `collector_scope.rs`・middleware の分岐・`session_get` の分岐と 14 本の試験を束ね、4.1 は印の鍵の差し替え・起動の検査・2 つのファイルの 5 本の試験を束ねる。どちらも検証は試験の名前の接頭辞（`st44_scope_` 14 本以上・`st44_session_key_` ほか）で 1 回に見るので、チェックの真偽は下限の本数で決まり、途中で 1 本ずつ確かめる形ではない。Scenario ごとの試験の名前と印は全部書かれているので、実装者が迷う粒度ではないと読んだ（R にしない）。分けるなら 1.1 を「一覧と middleware（送る 5 つ・読む 4 つ・外の 403）」と「`GET /session` と両方の見出し・印だけ・違う Bearer」に 2 つ
- 依存順: Task 1（一覧の外を 403）が Task 5・6（道具を印に替える）より先だが、Task 1〜4 の検証は `cargo test -p ashiato-server` だけで、smoke・e2e は Task 5・6 の検証で初めて走る。順として壊れない
- 人間の確認待ち: 物理の確認は無い（配った収集アプリは変えない）。本人の機械の `.env` に `WEB_SESSION_KEY` を足すのは design の Migration Plan の手順で、tasks の検証の外

## 観点 5: Story が要件の現在の本文と一致しているか
- check_chain.py は OK（観点 8 の再生成との一致を含む）
- docs/stories/ST44.md の「完了の判定」2 つ（送るだけ・例外の 3 つ）は、推奨の答えと specs に合っている。「壊してはいけないもの」の収集アプリの送信は、許可の一覧の 5 つの Scenario（:254-277）で固定されている
- satisfies は PERM-10 だけで、specs の変更はどれも PERM-10 の範囲（subjective-log・satellite-platform は導出元の書き直しと、許可の一覧の帰結の Scenario）。前倒しではないことは INDEX の訂正の節にある。要件の改訂の担い手 → R9

## 観点 6: 答えが写ったか
- 観点 6: 該当なし（mode は spec --draft。spec --answers ではない）

## 観点 7: 走っている change との前提関係
観点 7: 該当なし（spec_overlap.py candidates が 0 件）

（参考。呼び出し元の求めで ST52 を読んだ結果で、判定の行ではない）`st52-release-channels` の delta は record-envelope の「組み立ての印は…」「封筒の形が変わったのに…」と distribution で、ST44 の Requirement と重ならない。
ST52 の `/api` の剥がしは router の前で道を書き換える（origin/feat/st52-release-channels の crates/server/src/lib.rs:4227 `strip_prefix("/api")`・design D13「認可・読み出しの記録は剥がした後の道で今と同じ」）ので、ST44 の許可の一覧は剥がした後の `MatchedPath` で引ける。
ST52 の試験が API の合言葉を付けるのは `POST /heartbeat`（一覧の中）だけ。コードでは両方が `crates/server/src/lib.rs`・`App::new` の呼び手・`tools/stack.sh`・`.env.example`・`docs/network.md` を触るので、後から入る側の rebase で解く（proposal の Impact のとおり）。前提でなく独立と読んだ

## 確かめて成り立っていたもの（特に求められた点）
- **REMOVED + ADDED の置き直し**: `openspec` の core/specs-apply.js:345-347 が MODIFIED で正典の Scenario の名前が消えると archive を落とすので、Scenario を外すにはこの形しかない（spec_merge.py:7 の注記とも合う）。
  REMOVED と ADDED は名前が違うので ADDED の重複の検査に当たらず、validate も通る。残した 16 本の Scenario は名前が同じなので既存の印がそのまま効く（check_scenarios の FAIL に出ない）。
  消える印は `crates/server/src/web_session_tests.rs:213` の「API の合言葉での読み書きは変わらない」1 か所で、tasks 1.2 が消して grep で確かめる。
  archive の前に別の change が正典の旧い塊を変えれば `spec_merge.py apply` が REMOVED を止める（spec_merge.py:19）。いま旧い塊を名指す走っている change は無い
- **許可の一覧の 9 つと収集アプリが呼ぶ口**: 携帯 —— `POST /ingest`・`/heartbeat`・`/drops`・`/media/missing`（LocationService.kt:937・949・957・962）、`GET /outages`・`/contract`（:254・:258、HttpTransport.kt:80 の GET）、
  `PUT`・`HEAD /media/originals/{sha256}`（OriginalAndroid.kt:79-81）、`GET /feelings/reminder`（FeelingReminderSource.kt:48-49）。
  PC —— `POST /ingest`・`/heartbeat`（sender.rs:120・130）、`GET /contract`（runtime.rs:815・sender.rs:764）、`GET /outages`（sender.rs:441）、資格情報の要らない `GET /healthz`（clock.rs:207）。
  9 つの和と一致し、一覧の外を呼ぶところは見つからなかった（`collector-android/app/src/androidTest`・`crates/collector-windows/tests` にも一覧の外の口は無い）。axum が GET の口に足す HEAD は D4 が方法と道の組で引くので通らない
- 処置: fixed proposal.md —— proposal の Capabilities を REMOVED + ADDED の置き直しとその理由に直し、design の冒頭の「8 つ」を「9 つ」に直した

