# ST08 上流成果物の独立レビュー（spec-review）

**やり方**: `deep.md` の本人の答え 3 件と、context で見せた R2 / R5〜R9（`deep-questions.json` の文面そのもの）を
1 件ずつ `specs/` / `design.md` / `tasks.md` / `docs/requirements.md` / `docs/stories/ST08.md` まで追った。
MODIFIED の 5 本は正典 `openspec/specs/desktop-collection/spec.md` と見出し単位で diff を取った。
取り込み口（`crates/server/src/lib.rs` / `ingest.rs` / `migrations/`）と C-02（`crates/collector-windows/`）は読んで確かめた。
**成果物は編集していない。**

## 機械の検査（先に走らせた。2026-09-15）

```
$ openspec validate st08-browser-history --strict
Change 'st08-browser-history' is valid                                   rc=0

$ python3 scripts/check_chain.py .
要件 116 件 / Story 36 本 / 扉 26 項
[ok] どれかの Story に拾われた要件: 114/116 件
chain: OK (0 件 / 未回収 0 件 / warn 0 件)                                rc=0

$ python3 scripts/check_scenarios.py . st08-browser-history
Scenario 215 件 / 印 227 個 / 担保あり 178 / 人間の確認待ち 1
[FAIL] 担保の無い Scenario: 36 件（すべて st08-browser-history の desktop-collection）
scenarios: FAIL (担保なし 36 件)                                         rc=1

$ python3 scripts/review_triage.py . st08-browser-history
指摘 10 件 / 仮決め 1 件 / 要件へ戻すもの 2 件
triage: OK                                                               rc=0
```

- `check_scenarios` の FAIL 36 件は **ST08 で新しく足した Scenario の数と一致**する（ADDED 26 + MODIFIED で足した 10）。
  MODIFIED で写した ST07 の既存 13 本は印があり FAIL に出ていない。上流ではテストが 0 本なので想定どおり。
  36 本はどれも `tasks.md` のどこかのタスクに名前が 1 回以上書かれている（突き合わせた）
- MODIFIED 5 本は、正典の本文と Scenario を**1 行も落とさず**、行を足しただけ（diff で確認）。見出しは正典と一字一句一致する
- `check_chain` の観点 8（再生成との一致）は通っている
- `review_triage` の OK はこのファイルを書く前の値。このファイルの指摘には処置が無いので、次に走らせると FAIL になる

---

## R1. 同期で入った訪問の識別子が、本人に見せた Q3 の context と逆になっている —— 「発生元の印と番号で見分ける」を、design が「PC の番号で作る」に変えた

- 成果物: openspec/changes/st08-browser-history/design.md
- 根拠: `deep-questions.json:70`（Q3 の context。「見分けに使える値（どの選択肢でも同じ）」の箇条）は
  「同期で入った他端末の訪問は、**発生元の端末の印と発生元での番号で見分ける**」と書いており、`deep.md:97` もこれを
  「答えと一緒に受け取ったもの（R7）」として記録している。一方 `design.md:165-167` D9 は
  「**識別子は D6 と同じ形（PC の DB の番号で作る）** —— 発生元の番号で作ると…版が毎日積む」と、見分け方を変えた。
  spec `specs/desktop-collection/spec.md:95` の識別子の組も PC 側の番号で、発生元は「載せる」（`:161`）だけ。
  `deep.md:183-187`「当初案を覆したもの」は**なし**としている。Q3 は `loss: rewrite-all`（見分け方は 1 行でも入った後に変えると対応が切れる）
- kind: irreversible
- loss: rewrite-all
- 提案: D9 の理由（2 台の PC が同じ識別子で交互に更新を送る）は筋が通っているので、本人に「context と違う形にした」と返し、
  `deep.md` の「当初案を覆したもの」に記録する。spec の識別子の Requirement に「同期の訪問も PC 側の番号で作る」を明記する
- 処置: escalated — 第 2 回 Q4 として本人に返した（`deep-questions-r2.json`）。推奨は design D9 の形（PC 側の番号）で、spec と design は推奨で書いて「回答待ち」の印を付けた。deep.md の「当初案を覆したもの」に記録

## R2. 識別子に訪問時刻の生の値と、塩の無い URL のハッシュが入る —— 本文を物理削除しても識別子は凍結されて残り、URL は辞書で引ける

- 成果物: openspec/changes/st08-browser-history/specs/desktop-collection/spec.md
- 根拠: `design.md:126` D6 `v1:<family>:<browser>:<profile_dir>:<visit_id>:<visit_time_raw>:<sha256(url) の先頭 32 桁>`。
  `design.md:132-133` はハッシュにする理由として「本文を識別子の列へ漏らさない目的もある」と書いている。
  しかし `migrations/202609120944_gates.sql:49` は収集した記録の `external_id` の書き換えを拒むので、FR-51 の本文の消去
  （`docs/requirements.md:392-395`）の後も識別子は残る。よく知られた URL は `sha256` を総当たりすれば一致が取れるので、
  **消した訪問の「どの URL を・いつ・どのプロファイルで」が識別子から読める**。さらに `vanished` 記録
  （`spec.md:184`「消えた訪問の識別子を載せる」）は訪問の記録と別の行なので、訪問を消しても残る。
  spec の Scenario「識別子に URL の文字列が現れない」（`spec.md:127-130`）は文字列が現れないことしか見ないので、この状態でも緑になる
- kind: irreversible
- loss: rewrite-all
- 提案: 行が 0 件のうちに、組全体を 1 つのハッシュにする（`sha256(family|browser|profile_dir|visit_id|visit_time_raw|url)`）
  などで URL だけの辞書引きを防ぐ。そうすると「消えた」の 90 日の手がかりは識別子からは読めなくなるので、別の欄で持つ。
  spec には「識別子から URL を推定できない」を観測できる言葉（例: 同じ URL の 2 つの訪問の識別子に共通する部分が無い）で置く。
  消去が `vanished` にも及ぶかどうかは ST22 への申し送りに足す
- 処置: escalated — 第 2 回 Q5 として本人に返した。推奨は「組全体を 1 つのハッシュ」。spec の Requirement を「識別子から URL・訪問時刻・プロファイルを読み取れない形」にし、Scenario を観測できる言葉（どの文字列も含まない / 接頭辞のほかに共通部分が無い）に直した。design D6 は推奨で書いて「答え待ち」の印。ST22 への申し送りは不要と判断（識別子から読めなくなるので、消去が `vanished` に及ぶかは問題にならない）

## R3. Q1 の選択肢では「プロファイル名を載せる」と見せたのに、design はプロファイルの表示名を載せない

- 成果物: openspec/changes/st08-browser-history/design.md
- 根拠: `deep-questions.json:23`（Q1 の推奨の選択肢の detail）「**記録にブラウザ名とプロファイル名を載せる**ので、
  後から『仕事のプロファイルだけ除く』を読み分けられる」。`design.md:96` D4 は `profile_dir`（`Default` / `Profile 1`）だけで
  「**表示名は載せない**」、`:108-110` はその理由と「要ると分かったら `profiles` 記録として足す」を反転条件に置いている。
  spec は `spec.md:30`「どのブラウザのどのプロファイルの訪問かが記録から判別できる」で、表示名かディレクトリ名かを言っていない。
  ディレクトリ名だけでは「Profile 1 が仕事用か」を記録から読めない。表示名とディレクトリの対応は**その時点で取らないと**、
  改名や、プロファイルを消した後（D10 の `profile_gone`）には取れない
- kind: conflict
- loss: uncaptured
- 提案: D4 の改名で版が一斉に積む懸念は正しいので、訪問の本文には入れないまま、反転条件にある「取得 1 回につき
  `profiles` 記録 1 件（ディレクトリ名 → 表示名）」を最初から取る（列を持つ既定）。spec にその Scenario を置く
- 処置: escalated — deep.md「第 2 回で目に入れておくもの」に置いた（Q1 で本人に見せた「プロファイル名を載せる」のとおりに塞いだ）。取得ごとではなく**初回と対応が変わったとき**に `profiles` 記録を 1 件（毎日同じ対応を送ると版が毎日積むため）。spec に Requirement の SHALL と Scenario 2 本、design D1 / D4 / D6

## R4. 履歴のソースの生存信号で、区間の中に取得が 1 回も無いとき「取得できる状態か」が決まっていない

- 成果物: openspec/changes/st08-browser-history/specs/desktop-collection/spec.md
- 根拠: `design.md:207` D12「起動直後に 1 回出す」、`:208`「試行と成功は**プロファイル 1 つの読み 1 回**を 1 試行と数える」、
  `:209-210`「`blockers` は…**区間の間に一度でも欠けたものを残す**」。一方 D3（`design.md:74`）では、
  前回の成功から 24 時間経っていない起動では取得しない。読みは別のスレッドで数十秒かかりうる（`:80-81`）。
  つまり**起動直後の信号の区間には、読みが 1 回も無いことが普通にある**。そのとき `capturable` を真にするか偽にするか、
  その場で読めるか確かめるのかが spec にも design にも無い。spec の Scenario（`spec.md:342-350`）は
  「読めない状態で契機に達する」だけを見ている。
  `crates/server/src/coverage.rs:29-30` は `c02-browser-history` を「**取得できる状態の生存信号があった日**」で数え、
  生存信号は `migrations/202609111112_immutable_heartbeat.sql:17` で書き換えられない
- kind: technical
- loss: uncaptured
- 提案: 区間に読みが無いときの扱い（例: 信号を出す前に対象の置き場を探して写しが取れるかだけ試す / 読みが無ければ
  `blockers` に `history-not-attempted` を載せる）を D12 に決め、spec に Scenario を 1 本置く
- 処置: escalated — deep.md「第 2 回で目に入れておくもの」に置いた。区間に読みが無ければ信号の前に写しを取って開けるかを確かめる（厳しい側）。spec の生存信号の MODIFIED に IF と Scenario、design D12、tasks 8.2

## R5. 「消えた」と「除外した」の識別子に取得の時刻が入るので、取得をやり直すと二重に入る —— 「数え直されない」の Scenario と食い違う

- 成果物: openspec/changes/st08-browser-history/design.md
- 根拠: `design.md:127-128` D6 `vanished : v1:vanished:<browser>:<profile_dir>:<取得時刻のマイクロ秒>:<n>`、
  `excluded : v1:excluded:<browser>:<profile_dir>:<取得時刻のマイクロ秒>`。
  `design.md:79` D3「積む前に落ちたら、次の取得が同じ訪問を再び積む（**D6 で畳まれる**）」は訪問には成り立つが、
  上の 2 つは取得のたびに識別子が変わるので畳まれない。未送信に積んだ後・帳面を書く前に落ちると（D3 は「積み終えて帳面を書いた後」を成功とする）、
  次の取得は同じ「消えた」を別の識別子で積み、同じ除外を再び数える。
  spec `spec.md:285-288`「その訪問は除外した件数に 1 回だけ数えられている」、`:197-198`「『消えた』記録が 1 件入り」と食い違う
- kind: technical
- 提案: 識別子を中身から決まる形にする（例: 消えた訪問の識別子の並びのハッシュ / 除外した訪問の識別子の並びのハッシュ）。
  tasks に「積んだ後・帳面を書く前に落として取得し直しても、`vanished` / `excluded` の行が増えない」テストを足す
- 処置: fixed D6 — `vanished` / `excluded` / `profiles` の識別子を中身から決まる形にした。spec に「取得をやり直しても『消えた』記録は増えない」「取得をやり直しても除外の件数は増えない」、tasks 6.2 / 7.2

## R6. 取り込みの規則に使う `source_updated_at` に「取得した時刻」を入れる決定が design だけにあり、正典 `record-envelope` の定義と食い違う

- 成果物: openspec/changes/st08-browser-history/design.md
- 根拠: `design.md:137-140` D6「`source_updated_at` には取得した時刻を載せる。契約はこの欄を『外部サービス側の更新時刻で、受信時刻ではない』と定めている」。
  正典 `openspec/specs/record-envelope/spec.md:291`「外部サービス側の更新時刻（または版）を記録に持たせる」、
  `docs/collector-contract.md:33` / `:44`。この値で古い到着を捨てるかどうかが決まる（`crates/server/src/lib.rs:714-718`）ので、
  外から見える振る舞いである。proposal `:73` と tasks `:8` は `record-envelope` に**触らない**としている。
  `openspec archive` は design を正典に写さないので、この読み替えは正典に残らない
- kind: conflict
- 提案: `desktop-collection` の識別子の Requirement に「ブラウザ履歴の記録の更新時刻は、その内容を読んだ時刻とする」と
  Scenario（未送信の再送で古い題名が新しい題名を書き戻さない）を置く。「版」の読みで `record-envelope` と矛盾しないことを導出元に書く
- 処置: fixed D15 仮 — `source_updated_at` に読んだ時刻を載せる決定を D15（仮）に分け、反転条件を書いた。観測できる振る舞い（古い観測で書き戻さない）は spec の識別子の Requirement の SHALL と Scenario に置き、`record-envelope` の「版」として読むことを導出元に書いた

## R7. D10 の「消えた」の決まり事のうち、外から見える 3 つが design だけにあり、1 つは spec と食い違う

- 成果物: openspec/changes/st08-browser-history/specs/desktop-collection/spec.md
- 根拠: `design.md:176`「`expired` —— …（**Chromium だけ。Firefox は日数で消さないので付けない**）」。
  spec `spec.md:184-185` は手がかりを「訪問時刻が 90 日を過ぎていたか」とだけ書き、ブラウザで分けていない ——
  **Firefox の訪問で 90 日を過ぎていたかどうかを載せるかが、spec と design で違う**。
  `design.md:181`「**読めなかったプロファイルでは消えたと判定しない**」は、読めなかった回に「消えた」が何件出るかを決める
  （数える / 数えない）が、spec に Requirement も Scenario も無い。`design.md:179` の手がかり `profile_gone` も spec の手がかりの一覧（`spec.md:185`）に無い
- kind: technical
- 提案: spec の手がかりを「訪問時刻と取得時刻の差が 90 日を超えていたか」のように**事実**で書いてブラウザで分けない（判定はしない方針にも合う）か、
  Chromium だけにするなら spec に書く。「読めなかったプロファイルでは『消えた』を出さない」と `profile_gone` の手がかりを Scenario にする
- 処置: fixed D10 — 手がかりを「訪問から取得までの日数」の事実にして全ブラウザで載せ（真偽にしない）、`profile_gone` と「経路を名指しする値を置かない」「読めなかったプロファイルでは出さない」を spec の SHALL / Scenario にした

## R8. 除外の決まり事（D11）の大半が design だけにある —— 題名の登録の当て先・登録を後から足した / 外したとき

- 成果物: openspec/changes/st08-browser-history/specs/desktop-collection/spec.md
- 根拠: `design.md:195` D11「`title-contains` → **ページの題名**」は、spec では導出元の説明文（`spec.md:246-247`）にしか無く、
  SHALL にも Scenario にも無い。`design.md:201`「**登録を後から足した**とき: …以後その訪問の内容が変わっても送らない。『消えた』の比べからも外す」、
  `:202`「**登録を後から外した**とき: 除外済みの訪問がまだ DB にあれば、次の取得で送る」、`:197`「`browser-profile` はウィンドウの記録に当てない」、
  `:200`「新しく除外した数が 0 なら `excluded` を書かない」も、どれも取り込み口へ送る / 送らないを決めるのに spec に無い。
  `tasks.md:109-118`（7.1 / 7.2）にも、後から足した / 外したときのテストが無い。
  題名の登録が履歴に効かない実装でも spec は落ちず、外した瞬間に送られる本文は凍結される
- kind: technical
- loss: exported
- 提案: 4 つを spec の除外の Requirement の SHALL と Scenario にする（特に「題名の部分一致はページの題名に当たる」と「登録を外した後の次の取得で送られる」）。
  7.2 の検証に対応するテスト名を足す
- 処置: escalated — deep.md「第 2 回で目に入れておくもの」に置いた（Q1 の答え + R9 の context のとおり）。題名の登録 → ページの題名、登録を後から足した / 外したとき、プロファイルの登録はウィンドウに当てない、を spec の SHALL と Scenario にし、tasks 7.3 を足した

## R9. URL の部分一致の登録がウィンドウの記録に当たったとき、アプリ名と窓の題名も落とすのかが決まっていない

- 成果物: openspec/changes/st08-browser-history/specs/desktop-collection/spec.md
- 根拠: `spec.md:275-278` の THEN は「ウィンドウの記録にも履歴の記録にも、**その URL は現れない**」だけ。
  既存の本文（`spec.md:229-230`）は「その対象の**アプリ名・ウィンドウ題名・URL** を記録しない」だが、既存の 3 通りの規則は
  前景 1 つを丸ごと当てる形（`crates/collector-windows/src/exclusion.rs:48-57`）で、URL で当てたときの「対象」が何かは書かれていない。
  **URL だけを伏せて窓の題名（ページの題名）を残す実装でも Scenario は緑になる**。除外した件数に数えるかも Scenario に無い
- kind: technical
- loss: exported
- 提案: THEN を「その前景の変化のアプリ名・ウィンドウ題名・URL がどの記録にも現れず、除外した件数に 1 回数えられる」にし、主張ごとに Scenario を分ける
- 処置: escalated — deep.md「第 2 回で目に入れておくもの」に置いた。URL の部分一致がウィンドウに当たったときは前景の変化を丸ごと除外して件数に数える（厳しい側）。spec の Scenario を 2 本に分けた。design D11 / tasks 7.1

## R10. Q1 で本人が選んだ「Chromium 系 5 つと Firefox」を、どの Scenario も名指ししていない —— tasks は Chrome / Firefox の実行時テストを飛ばせる

- 成果物: openspec/changes/st08-browser-history/specs/desktop-collection/spec.md
- 根拠: 本人の答え `deep.md:57`「Chromium 系 5 つと Firefox の、全プロファイル」。spec では Requirement の本文（`spec.md:6`）にだけあり、
  Scenario は「2 つのブラウザ」（`spec.md:28`）。Opera や Firefox を探さない実装でも Scenario は落ちない。
  `tasks.md:146`（10.2）は「無ければその 1 本を飛ばした旨を出力」、`design.md:227-228` D14 の反転条件は「その 1 本を Edge だけにする」。
  `tasks.md:64-65`（4.1）の `history_locate_finds_all_profiles` は 6 つを探すことを検証の条件にしていない。
  さらに D1（`design.md:49`）は Firefox を `%APPDATA%\Mozilla\Firefox\Profiles\<dir>` だけから探し、`profiles.ini` を読まない。
  Firefox はプロファイルを別の場所に置けるので、その場合は「見つからない」まま**生存信号にも出ない**（`design.md:237-238` は
  `history-none-found` が出るのは 1 つも見つからないときだけと認めている）
- kind: technical
- loss: uncaptured
- 提案: 「6 つそれぞれの既知の置き場に作ったプロファイルが全部見つかる」を Scenario にし、4.1 の終了条件に 6 種を列挙する。
  Firefox は `profiles.ini` の一覧と `Profiles\` の走査の和を取る（取りこぼさない側）
- 処置: escalated — deep.md「第 2 回で目に入れておくもの」に置いた（本人が選んだ 6 つを名指し）。spec に「6 つのブラウザの既知の置き場にあるプロファイルが全部見つかる」「Firefox の一覧にある既定の外の置き場も見つかる」、design D1（`profiles.ini` との和）/ D14（Chrome と Firefox の実行時テストを飛ばさない）、tasks 4.1 / 10.2

## R11. Scenario の言葉のうち、何を見れば真偽が決まるかが書かれていないもの

- 成果物: openspec/changes/st08-browser-history/specs/desktop-collection/spec.md
- 根拠:
  - `spec.md:35`「起動した後、**24 時間を待たずに**取得が行われる」—— 23 時間後に取得しても真になる。D3（`design.md:74`）は起動時に取る
  - `spec.md:90`「タイムゾーンが取得したときの PC のものであることが**判別できる**」、`:30`「どのブラウザのどのプロファイルの訪問かが**判別できる**」、
    `:208` / `:213` / `:218`「手がかりから…**読み取れる**」—— 記録のどこに何があれば判別できたことになるかが無い
  - `spec.md:203`「取得したときの**内容のまま**である」—— 題名が変わって版が積んだ後に消えた訪問では、どの版を「取得したとき」とするかが決まらない
- kind: technical
- 提案: 「起動の直後の最初の見回りで取得が始まる」、「記録の本文に、ゾーンの出どころが取得時であることを示す値がある」、
  「本文に 90 日を過ぎていたかどうかの真偽がある」のように、観測する値を書く（列名は書かずに済む）
- 処置: fixed tasks.md — spec の THEN を観測する値で書き直した（起動の直後の最初の見回り / 本文に…の値がある / 「消えた」記録が入る前と同じ内容）。tasks の Scenario 名を合わせた

## R12. Requirement の本文だけが言っていて、Scenario が言っていないこと

- 成果物: openspec/changes/st08-browser-history/specs/desktop-collection/spec.md
- 根拠:
  - `spec.md:9`「**動作中に**前回の取得の成功から 24 時間経った場合」—— Scenario は起動時の 2 本（`:32-40`）だけ
  - `spec.md:55-56` の載せる項目のうち**滞在時間**と**ページの題名**に、「載っている」を見る Scenario が無い。`:99`「題名・**滞在時間**などが違う → 更新」も題名の Scenario（`:116-120`）だけ。
    滞在時間は「取得の瞬間に開いていたタブは 0」（`deep-questions.json:70`）なので、Q3 で本人が選んだ理由そのもの
  - `spec.md:58`「履歴 DB にある文字列から**補正せず**」—— クエリとフラグメントの Scenario（`:82-85`）だけで、ST07 の「表示されている文字列を補正しない」に当たるものが無い
  - `spec.md:187`「**消えた経路を判定しない**」—— Scenario が無い（本人が消した / 同期を切った、と書き分ける実装でも落ちない）
  - `spec.md:235`「ブラウザのプロセスを指す登録を、そのブラウザの**すべてのプロファイル**の履歴に効かせる」—— `:269-273` の Scenario はプロファイルが 1 つでも通る
- kind: technical
- 提案: 5 つにそれぞれ 1 本ずつ Scenario を足す（滞在時間は「開いていたタブを閉じた後の取得で、同じ訪問の滞在時間が更新され前の版が残る」）
- 処置: fixed tasks.md — spec に Scenario を足した（動作中に 24 時間 / 題名と滞在時間が載る / 滞在時間の更新と版 / URL を補正しない / 経路を名指ししない / プロセスの登録は全プロファイル）。tasks 2.2 / 4.3 / 5.4 / 6.1 / 7.2 に割り当てた

## R13. 1 本の Scenario に、別々に通る主張を 2 つ束ねているもの

- 成果物: openspec/changes/st08-browser-history/specs/desktop-collection/spec.md
- 根拠: `spec.md:29-30`（全部入る AND どのプロファイルか判別できる）、`:119-120`（1 行のまま新しい題名 AND 前の版が残る）、
  `:171-172`（発生元の印 AND 記録の端末は読んだ PC）、`:272-273`（URL と題名が送られない AND 除外の件数が残る）、
  `:339-340`（1 件届く AND ウィンドウとは別の件）、`:345`（取得できない状態 と 読めなかった対象を持つ）、
  `:429-430`（種別と件数を含む AND URL などを含まない）。どれも片方だけを検査するテストで印が付けられる
- kind: technical
- 提案: 少なくとも `:171-172`（端末の決定は R7 の本人の決定）と `:272-273`（件数は FR-83 の「事実と件数を残す」）は Scenario を分ける
- 処置: fixed tasks.md — 束ねていた 7 本を分けた（端末は読んだ PC / 除外の件数 / 別の件 / 読めなかった対象 / 題名の版 など）。ログの Scenario は THEN を「含まれない」だけにした。tasks に割り当てた

## R14. design の反転条件の中に、spec の Requirement を破る手が本人に返す印なしで入っている

- 成果物: openspec/changes/st08-browser-history/design.md
- 根拠: `design.md:83-84` D3 の反転条件「90 日より前の訪問は読まない（Firefox だけ）」は、spec `spec.md:139-141`
  「訪問時刻が前回の取得より古いかどうかに関わらず送る」「初めて取得する…過去の履歴を**すべて**取り込む」を破る。
  Firefox は日数で消さないので、90 日より前の訪問を読まなくなった後に同期で入った古い訪問は取れない。
  `:84`「値（24 時間・1 分）は FR-13 の間隔の範囲で変えてよい」と spec `spec.md:13`「契機の値は**仮**」は、
  24 時間を要件（`docs/requirements.md:123` FR-13「24 時間間隔」）が決めていることと合わない。
  D10 の反転条件（`design.md:185-186`）は「deep.md に追記して本人に返す」と書いているが、D3 には無い。
  D14 の反転条件（`design.md:227-228`）は R10 と同じ
- kind: technical
- loss: uncaptured
- 提案: D3 の反転条件に「spec の Requirement を変えることになるので本人に返す」を書く。
  spec の導出元の「契機の値は仮」は、仮なのが 1 分（再試行）だけであることが分かる書き方にする
- 処置: escalated — deep.md「第 2 回で目に入れておくもの」に置いた。D3 の反転条件から「古い訪問を読まない」を外し、spec を破る手は本人に返すと書いた。仮なのは試し直しの間隔（1 分）だけと spec の導出元と D3 の見出しに明記。D14 も飛ばす形に倒さない

## R15. 履歴の読みが見回りを止めると、ST07 のウィンドウのソースに眠りの記録が入る —— 「壊してはいけないもの」なのに Scenario もテストも無い

- 成果物: openspec/changes/st08-browser-history/tasks.md
- 根拠: `docs/stories/ST08.md:36`「壊してはいけないもの: ST07 のウィンドウ記録」。`design.md:80-81` D3
  「Firefox が何年分も持っていると最初の読みは数十秒かかりうるので、見回りを止めると眠りの判定（ST07 D19。2 分）に化ける」。
  ST07 D19（`openspec/changes/archive/2026-09-14-st07-active-window/design.md:298-300`）は 2 分の飛びを `suspended` の出入り 2 件として残す。
  これは `c02-window` の記録なので凍結される。`tasks.md:86-90`（5.4）は「読みは別スレッド」と書くだけで、検証のテスト
  （`history_schedule` / `history_success_only_after_outbox`）は見回りが止まらないことを見ない。spec の `spec.md:47-50` は件数を
  「取得の前後で変わらない」と言うが、読みが長いときを WHEN にしていない
- kind: technical
- 提案: 5.5 に「読みが 3 分かかる読み手を差し込んでも、その間の見回りが続き `c02-window` に `suspended` が入らない」テストを足す。
  spec の `:47-50` の WHEN を「読みに見回りの間隔より長くかかる取得」にする
- 処置: fixed 5.6 — 読みに 3 分かかる読み手を差し込んでも見回りが続き `c02-window` に `suspended` が入らないテストを足し、spec の WHEN を長い読みにした（design D3）

## R16. Scenario の印を置くテストが、その Scenario の THEN を確かめない形になっているタスク

- 成果物: openspec/changes/st08-browser-history/tasks.md
- 根拠:
  - `tasks.md:35-41`（2.1）は Scenario「識別子を欠いた履歴の記録は断られる」を担うが、テストの (a)〜(c) は登録簿の値だけを見る。
    **識別子なしの要求を送って `missing_external_id` で断られることを見る項目が無い**（断る処理は `crates/server/src/lib.rs:345-352`）
  - `tasks.md:86-90`（5.4）は「取り込み口が止まっている間に取得した履歴が後から**届く**」を、収集側の単体
    `history_success_only_after_outbox` に置いている。THEN は「記録が**格納されている**」
  - `tasks.md:143-145`（10.1）は「前日に見たページが翌日の取得で入っている」を担うが、中身は「読み手で読める」で、
    取得契機にも取り込み口への格納にも触れていない
- kind: technical
- 提案: 2.1 に (d)「識別子なしの `c02-browser-history` の要求が断られる」を足す。5.4 と 10.1 は、格納まで見る検査
  （`tools/smoke.sh` の psql、または結合テスト）に印を移すか、THEN を収集側で観測できる言葉に直す
- 処置: fixed 2.1 — (d) 識別子なしの要求が断られる、を足して印をそこに置いた。5.5 と 10.3 は取り込み口まで通す検査（smoke の psql）に印を移した

## R17. 終了条件が弱い、またはタスクが無いもの

- 成果物: openspec/changes/st08-browser-history/tasks.md
- 根拠:
  - `tasks.md:148-149`（10.3）「走った本数の下限を足した分だけ上げる」は数が無く、検証は「job が緑」。10.2 が飛ばしを許すので、何本を下限にするかが決まらない
  - `tasks.md:57-58`（3.3）の検証は `grep -c "c02-browser-history"` が 1 以上。識別子の形も `source_updated_at` の理由も書かずに通る
  - `tasks.md:113`（7.1）は README の `url-contains` だけを grep し、`browser-profile` を見ない
  - D12 の「起動直後に 1 回出す」（`design.md:207`）と、D10 の「読めなかったプロファイルでは判定しない」（`design.md:181`）に、名前の付いたテストが無い
    （6.1 は本文に書くが検証は `cargo test history_vanished` の絞り込みだけで、該当テストが 0 本でも rc=0）
- kind: technical
- 提案: 10.3 に下限の数（例: 既存 + 2）と、飛ばした 1 本を下限に数えないことを書く。3.3 は識別子の接頭辞 `v1:` と `source_updated_at` の両方を grep する。
  7.1 に `browser-profile` を足す。D12 / D10 の 2 件にテスト名を付ける
- 処置: fixed 10.4 — 下限を既存 + 3 と書いた。3.3 は `v1:` と `source_updated_at` も grep、7.1 は `browser-profile` も grep、D12 / D10 の 2 件にテスト名（8.2 / 6.2）を付け、絞り込みは `-- --list` の本数も見る規律を 0 章に置いた

## R18. 版を積む更新でタイムゾーンが「その更新を読んだ時点のゾーン」に上書きされるが、spec の「取得したときの PC のもの」はどの取得かを言っていない

- 成果物: openspec/changes/st08-browser-history/specs/desktop-collection/spec.md
- 根拠: 取り込み口の更新は `tz_offset_min` / `tz_id` も届いた値で書き換える（`crates/server/src/lib.rs:764-766`、コメント `:753-757`）。
  D5（`design.md:119`）は `tz_id` を**取得したときの** `Zone::current()` にする。旅行中に 1 回目を読み、帰宅後に題名が変わって 2 回目を送ると、
  行のゾーンは帰宅後の値になり、1 回目の値は前の版に移る。spec `spec.md:59` / `:87-90` は「取得したときの PC のもの」で、
  最初の取得か最新の取得かが決まらない。前の版に値は残るので失われるものは無い
- kind: technical
- 提案: 「行のゾーンは、その版を読んだ取得のときのもの」と spec に書くか、2 回目以降も最初に送ったゾーンを送る（帳面に持つ）と D5 に決める
- 処置: fixed D5 — 行のゾーンは新しい版を読んだ取得のもの、前の版のゾーンは前の版に残る、を D5 と spec の SHALL（「その内容を読んだ取得のとき」）に書いた

## R19. FR-13 の本文の「前回取得以降の履歴」が、spec の「毎回すべて読み、古い時刻でも未送信なら送る」と読み違えうる

- 成果物: docs/requirements.md
- 根拠: `docs/requirements.md:123-124` FR-13「前回取得以降の履歴を取得して記録を生成する」は ★ 2026-09-15 の訂正でも残った。
  spec `spec.md:139-141` は「訪問時刻が前回の取得より古いかどうかに関わらず送る」、`deep.md:146` C-8 は
  「位置は訪問時刻でなく重なりを持って読み直す」。要件だけを読むと、訪問時刻で「前回以降」を切る実装が要件どおりに見える
- kind: technical
- 提案: FR-13 の本文を「前回の取得の後に**履歴に入った**訪問と、送った後で値が変わった訪問」のように、訪問時刻でないことが分かる言葉にする（同じ ★ の訂正に含める）
- 処置: fixed deep.md — FR-13 の本文に「前回取得以降は訪問時刻で切らない」を同じ ★ 2026-09-15 の訂正の中で足し、ST08.md を再生成した。deep.md の「要件へ戻すもの」に記録

## R20. INDEX の ST07 の行の `satisfies` が ST07.md と一致していない（この change の外）

- 成果物: docs/stories/INDEX.md
- 根拠: `docs/stories/INDEX.md:29` の ST07 の行は `FR-12` だけ。`docs/stories/ST07.md:4` は `satisfies: [FR-12, FR-81, FR-82, FR-83]`。
  ST08 の spec は FR-83（ST07 の要件）を履歴へ広げる（`spec.md:239`）ので、INDEX だけを見ると「satisfies に無い要件を満たそうとしている」と読める。
  ST08 自身の行（`:30`）と capability 表（`:70` `desktop-collection` = ST07, ST08）は一致している
- kind: defer
- 提案: INDEX の Story 表を `stories.json` から作り直す。`check_chain.py` の観点 8 がこの表を見ていないなら、見る対象に足す
- 処置: fixed deep.md — この change の外だが事実なので、`docs/stories/INDEX.md` の ST07 の行を ST07.md に合わせた（satisfies FR-12, FR-81, FR-82, FR-83 / doors 14, 15）。deep.md に記録

---

## 観点ごとの該当なし

- 観点 1 の「要件へ戻すもの」: 該当なし（確かめた範囲: `docs/requirements.md:129` FR-13 に `★ 2026-09-15 訂正`、`:814` EXT-E の注記に `★ 2026-09-15 訂正`。
  本人の数値 24 時間は `spec.md:34` / `:338`、90 日は `spec.md:207` の Scenario にある）
- 観点 1 の Q2（消えた事実を残す / ashiato からは消さない）: specs への写しは揃っている（`spec.md:179-223`）。食い違いは R5 / R7 だけ
- 観点 3 の「specs に実装の名前」: 該当なし（確かめた範囲: spec 全文。関数名・crate 名・列名は無い。`locking_mode=EXCLUSIVE` は導出元の説明文（`spec.md:17`）にある Chromium 側の名前で、要求の文には無い）
- 観点 3 の「proposal の Capabilities と specs / INDEX」: 該当なし（確かめた範囲: proposal `:50-54` は `desktop-collection` のみ、`specs/` のディレクトリも `desktop-collection` のみ、INDEX `:70` は ST07, ST08。前倒しは無い）
- 観点 4 の依存の順: 該当なし（確かめた範囲: 識別子の形 3.1 → 帳面 5.1 → 取得 5.2 / 振り直し 5.3 → 消えた 6.1 → 除外 7.2 → 生存信号 8.1。登録簿の移行 2.1 は送る側より前）
- 観点 4 の人間の確認待ち: 該当なし（確かめた範囲: `tasks.md:161-168`。機械が再現できない物理的な操作に当たる Scenario は無い）
- 観点 5 の ST08.md の「価値」「完了の判定」と deep の決定の矛盾: 該当なし（確かめた範囲: `docs/stories/ST08.md:12-41`。
  「2 回続けて取得しても行が増えない」は Q3 の「更新して前の版を残す」と両立する。前の版は別の表に入る、`crates/server/src/lib.rs:719-731`）
