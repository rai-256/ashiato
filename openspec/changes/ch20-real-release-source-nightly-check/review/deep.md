# CH20 の深掘りの問いの独立検査（deep-review）

対象: `openspec/changes/ch20-real-release-source-nightly-check/deep-questions.json`（Q1〜Q9）・`premises.md`（P1〜P8）。
依頼（`docs/changes/CH20.md`）・`lane.json`・`lane-verdict.json`・正典 `openspec/specs/distribution/spec.md`（2488〜3330 行）・コードを読み、
手順 1〜8 を問いの一覧を見る前にやり直してから突き合わせた。指摘は R1〜R15。

## 手順ごとの記録

- 手順 1（要件どうしの衝突）: 3 組あった。
  (a) 依頼の「本物の配布元に毎晩問い合わせる」と、正典の「自動の試験は本物の配布元に問い合わせない」（`spec.md:2523`）。これは正面から食い違っていて、一覧に無い → R1。
  (b) 依頼の「どの状態でもエラーを記録し続けない」と、正典の「404 は配布元が見つからない（確かめられない）」（`spec.md:2897`）・「7 日確かめられなければ知らせる」（`spec.md:3236`）。開発版の道は、正典どおりに動いていても「エラーを記録し続ける」。Q7 は「正典が決めていない」という誤った前提に立っている → R2。
  (c) 検査の側の問い合わせを増やす話と、正典の「問い合わせに資格情報を入れない」（`spec.md:2519`）。これは製品の問い合わせについての規則で、検査の側には掛からない。ただし一覧はこの区別を扱っていない → R12
- 手順 2（扉の幅）: Story ではないので扉は無い（`CH20.md` の `satisfies: []`）。代わりに依頼の語の幅を見た。
  「落ちたら nightly が赤になる」は、開発版を組んで公開する workflow `nightly` とも、毎晩走る検査一般とも読める → R9。
  「決めた振る舞い」は、正典が決めた振る舞いとも、依頼が望む振る舞い（エラーを出し続けない）とも読める。開発版の道ではこの 2 つが分かれる → R2。
  「PR の CI の D18 は変えない」は、正典の 2523 行（同じ規則を正典に書いたもの）を変えることまで許すのかが読めない → R1
- 手順 3（新たに立つ一方通行）: 検査が外に出すのは、runner の IP から配布元への問い合わせだけ（版も資格情報も入らない）。失うものは無い。
  取らずに失うもの（uncaptured）が 1 つある。毎晩の本物の答えを残さなければ、「いつから形が変わったか」を後から引けない（過去の答えは取り直せない）→ R11。
  配布元に試験のリリースを置く案（Q4 の選択肢 3）は exported。一覧はそう書いているが、`loss` の欄が無い → R15。
  列の型・保存形式・鍵の入力は無い（DB にも保存の形にも触れない）
- 手順 4（日常に影響する選択）: 赤を本人がどう知るか、が抜けている。毎晩の脆弱性の検査は issue を立てて知らせる。
  一方 Q7 の推奨は「直しが入るまで毎晩赤」で、この間の知らせ方（重複・期限）が決まっていない → R10。
  Actions の時間: repo は公開なので（P1 `PUBLIC`）、標準の runner の分数は予算を消費しない。日常の負担にはならないので指摘しない
- 手順 5（既存のコードが要件を満たしていない箇所。grep と実行で確かめた）: 本物との食い違いが 3 つ、いまのコードに既にある。
  1. 印 `nightly` の道は、本物ではリリース 1 件のオブジェクトを返す。ところが Rust の読み方は配列しか読まない。試験の受け手は `[]` を返し、共有の材料も配列で書いている → R3。
  2. 版の一覧の読み方に 64 KiB の上限がある。これは正典に無い決まりで、本物の大きさではリリース 8 件ほどで超える → R4。
  3. PC の収集アプリは、5xx と別の宛先への転送を「配布元に届かない」と記録する。正典ではどちらも「読めない答え」 → R5。
  1 と 2 は、`/tmp` に置いた小さな crate で `ashiato_update_check::parse_releases` に通して確かめた。オブジェクトは `Err(InvalidJson("invalid type: map, expected a sequence…"))` になり、本物の 3 件（157,897 バイト）は `Err(TooLarge)` になった。携帯（Kotlin）はオブジェクトを受け、上限も持たない（`UpdateCheck.kt:105-106`）
- 手順 6（過去の取りこぼし。`python3 scripts/upstream_misses.py report`）: 当てはまったのは 4 つ。
  「送らずに止める決定には、止めたことを本人がどう知るかを問う」→ R10。
  「解消するまで待つ規則には、解消しないときの上限を問う」→ R10（直しが入るまで赤のままでよいのか）。
  「構造の検査を新しく作るなら、どの CI の job が毎回走らせるかを書く」→ R9。
  「決定の根拠になる実測は本人の実物から取る」→ premises は実物を見ている。ただし読み落としがある → R13。
  当てはまらなかったもの:
  - 時計の族: 判定は runner の時計と試験の差し込み（`ASHIATO_UPDATER_TEST_NOW`）で決まる。Q8 の「3 晩」は実行の並びで数える
  - git の履歴の深さ: 一覧に、印（タグ）を読んで期待値を出す決定が無い
- 手順 7（族の網羅）:
  - Q7（開発版の道の 404）の兄弟のうち、「印 `nightly` がある日の 200（オブジェクト）」が無い → R3
  - Q8（届かない夜: 403・429・時間切れ・接続できない）の兄弟のうち、「配布元の 5xx（障害の 502・503）」と「別の宛先への転送」が無い。正典ではどちらも「読めない答え」で、「届かない」の族に入らない → R5
  - Q4（作る状態: 同じ版・新しい版）の兄弟のうち、「一覧が 20 件で満ちたときの大きさ」と「印 `nightly` のオブジェクト」が無い → R3・R4
  - Q2（部品）の兄弟の携帯は一覧にある
- 手順 8（前提の確かさ）:
  - P4（確かさ 2）は誤り。正典は 404 を決めている → R2
  - P6（確かさ 2）は、根拠のファイル名が成り立たない。11-09 の zip には、まだ更新係が入っていない → R13
  - P5 は「決められない」と書くが、コードを辿ると記録の無さから言えることがある → R13
  - P7（確かさ 3。コードを読んだだけ）は、PC の収集アプリの版を組み立てで変えられる範囲を確かめていない。Q4 の推奨がそこに乗っている → R7
  - P8（確かさ 2）に乗る Q8 には `loss` が無いので、指摘しない
  - 外の形式に依る決定で、premises に行が無いもの: 配る形の PC の収集アプリが runner でトーストを出せるか（Q3 の推奨が乗る）→ R8
  - 確かさの段と確かめ方の食い違い（走らせていないのに 4 以上）は無かった。P1・P2 と同じ見出しで公開の repo に `curl` し、`200`・`x-ratelimit-remaining: 53`・配布物に `digest` があることを見た
- 構成の記号: `grep -noE '\b(S|C|V|D)-0[0-9]\b' deep-questions.json` は 0 件。該当なし
- 既定の無い仮の問い: 全 9 問に `recommended` が 1 つずつある。該当なし
- 文字で決めさせている画面の問い: 画面の構造を問う問いは無い。該当なし

---

## R1. 依頼が、正典の「自動の試験は本物の配布元に問い合わせない」を変える。要件の本文を変えるので仮にできないのに、一覧に問いが無い
- 成果物: openspec/changes/ch20-real-release-source-nightly-check/deep-questions.json
- 根拠: `openspec/specs/distribution/spec.md:2523`「試験の組み立ては差し替えの宛先が無ければ確かめない（**自動の試験は本物の配布元に問い合わせない**。配る組み立てを動かす CI は、外への接続を塞いで走らせる）」 / 依頼 `docs/changes/CH20.md`「本物の配布元…と話したときに正しく振る舞うことを、nightly で 1 日 1 回確かめる」 / Q9 の推奨の detail「この change の中では製品のコードと正典の振る舞いを変えない」と context の「壊してはいけないもの」の候補 / `lane.json` の paths に `openspec/specs/**` が無い / `.harness2/docs/flow-gates.md:275`（仮で進めるのは「要件を変えない」ときだけ）
- kind: conflict
- 提案: 第 1 回に問いを 1 つ足す（kind `conflict`。推奨を付ける）。中身は「正典 2523 行の括弧書きを、『PR の CI と配る組み立てを動かす CI は本物に問い合わせない。毎晩の検査だけは、試験の組み立てで本物に問い合わせてよい』のように改める（MODIFIED）か」。選択肢には、変えるのは試験の決まりの文で、製品の振る舞いは変わらないことを書く。Q9 の「正典の振る舞いを変えない」は「正典の製品の振る舞いは変えない。試験の決まりの文だけ変える」に直す。
- 処置: escalated （fixed Q10（premise で第 1 回に足した。要件の本文を変えるので A）。Q9 の文も「製品の振る舞いは変えない」に寄せた）

## R2. Q7 の前提が誤っている。正典は開発版の道の 404 を「確かめられない（配布元が見つからない）」と決めていて、依頼の「エラーを記録し続けない」と衝突する。推奨の「赤にする」は、正典どおりの振る舞いを赤とみなす
- 成果物: openspec/changes/ch20-real-release-source-nightly-check/deep-questions.json / premises.md
- 根拠:
  - Q7 の why「正典はこの場合の振る舞いを決めていない（`premises.md` P4）」と P4「決めていない」（確かさ 2）
  - 正典はこう決めている:
    - `spec.md:2897`: 理由の 6 つに「配布元が見つからない（`404`）」
    - `spec.md:2916`: Scenario「確かめられない理由を状態の番号で分ける」の 404
    - `spec.md:2585`: 開発版は印 `nightly` のリリースだけを数える
    - `spec.md:3236`: 7 日確かめられなければ「更新を 7 日確かめられていない」を知らせる
  - よって正典が決めた振る舞いは、「印が無い間、開発版の部品は毎日確かめられないと記録し、7 日後に知らせる」になる
  - コードはそのとおり: `crates/updater/src/lib.rs:173`、`crates/collector-windows/src/update_check.rs:482`
- kind: conflict
- 提案:
  - Q7 の kind を `conflict` にする。context を「正典は 404 を確かめられないと決めていて、いまの製品は正典どおり。依頼の『エラーを記録し続けない』とぶつかる」に直す
  - 「赤にする」は、正典の意味を変える（いまの 404 の扱いを誤りとみなす）ことを選択肢に書く。仮（P）にはしない
  - premises.md の P4 は「決めている（`spec.md:2897`・`:3236`）」に直す
- 処置: escalated （fixed Q7（context と選択肢を直し、推奨を「正典どおりなら緑・食い違いを要約に出す」にした）・fixed premises.md P4）

## R3. 抜け: 本物の印 `nightly` の答え（リリース 1 件のオブジェクト）を、更新係と PC の収集アプリは読めない。試験の受け手と共有の材料が配列を返しているので、試験は通る。依頼が捕まえたい「偽の配布元が本物とずれる」が既に起きている
- 成果物: openspec/changes/ch20-real-release-source-nightly-check/deep-questions.json
- 根拠:
  - 読み方は配列だけを読む: `crates/update-check/src/lib.rs:172-178`（`parse_releases` は `Vec<Release>` として読む）
  - 呼び出し側: 更新係 `crates/updater/src/lib.rs:233-235`、PC の収集アプリ `crates/collector-windows/src/update_check.rs:213-214`・`:409-410`
  - 試験の受け手は印 `nightly` の道に `[]` を返す: `crates/updater/tests/st57_once.rs:89`
  - 共有の材料も、印 `nightly` の答えを配列で書いている: `docs/contracts/update-check-vectors.json:8-10`
  - 携帯は、オブジェクトを 1 件の配列に包んで読む: `collector-android/.../UpdateCheck.kt:105-106`
  - 実行して確かめた: `parse_releases(br#"{"tag_name":"nightly",...}"#)` は `Err(InvalidJson("invalid type: map, expected a sequence at line 1 column 0"))` を返す
  - 帰結: 開発版の公開の設定が入った日から、開発版の更新係と PC の収集アプリは毎日「読めない答え」を記録する
  - Q7 は「今 404」だけを見ている。Q4 の推奨（その晩の本物の答えを流し直す）は、404 の間はこの形を一度も流さない
- kind: technical
- 提案:
  - Q7 の context に、この食い違いを事実として書く
  - Q4 の「作る状態」の族に「印 `nightly` がある日の 200（リリース 1 件のオブジェクト。形は本物の 1 件から取る）」を足す。失わない側なので仮（P）でよい
  - この状態を入れると、Q9 の推奨（直さない）のもとでは初日から赤になる。そのことを Q9 の context に書く
- 処置: fixed deep-questions.json （fixed Q9（見つかった食い違い (1) として問いに載せた）・fixed premises.md P9）

## R4. 抜け: 版の一覧に 64 KiB の上限がある。正典に無い決まりで、リリース 8 件ほどで本物の答えが上限を超え、更新係と PC の収集アプリは毎日「読めない答え」になる。Q5 の context は、この上限を製品の決まりとして書いている
- 成果物: openspec/changes/ch20-real-release-source-nightly-check/deep-questions.json
- 根拠:
  - 上限を置いている箇所: `crates/update-check/src/lib.rs:8`（`MAX_RELEASES_JSON_BYTES = 64 * 1024`）・`:173-176`（版の一覧に掛ける）
  - 正典の 64 KB は結果のファイルについての決まりで、版の一覧には掛からない: `spec.md:2983`
  - 正典は一覧を `per_page=20` で取る: `spec.md:2517`
  - 公開の repo のリリースを製品と同じ見出しで取って測った:
    - リリース 1 件の骨組み（本文を除く）が約 2.1 KB、配布物 1 つが約 1.7 KB（`digest` などを含む）
    - 配布物 4 つのリリースは約 9 KB と本文の分になり、8 件ほどで 64 KiB を超える
  - 実行して確かめた: 本物の 3 件（157,897 バイト）で `parse_releases` → `Err(TooLarge)`
  - 携帯には上限が無い（`UpdateCheck.kt:217`）
  - Q5 の context「製品が読むのは…本文の大きさ（64 KiB まで）」
  - Q4 の推奨「本物の 1 件の形に版の名前だけを差し込んだ一覧」は 1 件しか流さないので、この食い違いに当たらない
- kind: premise
- 提案:
  - Q5 の context を「64 KiB の上限はコードの定数で、正典に無い。本物はリリース 8 件ほどで超える」に直す
  - Q4 の「作る状態」に「本物の形で 20 件・各 4 配布物の一覧（上限いっぱいの大きさ）」を足す（P でよい）
  - R3 と同じく、Q9 の推奨のもとでは初日から赤になることを Q9 の context に書く
- 処置: fixed deep-questions.json （fixed Q9（(2) として載せた）・fixed Q5 の context（正典に無い決まりと明記）・fixed premises.md P10）

## R5. Q8 の context の前提が誤っている。PC の収集アプリは 5xx と別の宛先への転送を「配布元に届かない」と記録し、正典の「読めない答え」と合わない。届かない夜の族にも 5xx が無い
- 成果物: openspec/changes/ch20-real-release-source-nightly-check/deep-questions.json
- 根拠:
  - PC の収集アプリ:
    - `crates/collector-windows/src/update_check.rs:479-485`（`map_request_error`: 403・429・404 以外の状態の番号は `Unreachable`）
    - `:394-397`（別の宛先への転送も `Unreachable`）
  - 正典: `spec.md:2897`「読めない答え（`200` でも読めない形・別の宛先への転送・転送が 3 回を超えた・ほかの状態の番号）」
  - 更新係は正典どおり: `crates/updater/src/lib.rs:174`・`:189-193`
  - Q8 の context「届かなかったときも、製品がその理由…を正典どおりに記録したかは判定できる」
  - Q8 の問いが挙げるのは `403`・`429`・時間切れ・接続できない で、配布元の 5xx（障害の日の 502・503）が無い
- kind: premise
- 提案:
  - Q8 の context に「PC の収集アプリは 5xx と別の宛先への転送を『配布元に届かない』と記録していて、正典と合わない。配布元に障害がある夜は、この判定で赤になる」を書く
  - Q8 の族に「配布元が 5xx を返した夜」を足し、その夜を「届かない夜」として扱うかを推奨とともに書く
  - Q9 の「直さない」に、この食い違いも含めて挙げる
- 処置: fixed deep-questions.json （fixed Q8 の context・fixed Q9（(3) として載せた）・fixed premises.md P11）

## R6. Q2 の前提が誤っている。携帯は選び方のコードを共有していない（Kotlin で自前に持ち、共有しているのは材料だけ）。R3・R4 の食い違いは Rust の 2 部品だけにある
- 成果物: openspec/changes/ch20-real-release-source-nightly-check/deep-questions.json
- 根拠:
  - Q2 の context「3 つの部品は選び方（`crates/update-check` の `choose`）を共有し」
  - 携帯の選び方は `collector-android/app/src/main/kotlin/dev/ashiato/collector/UpdateCheck.kt:91-106` の `choose` で、Rust の crate に依らない
  - 本物との違い: 携帯は印 `nightly` のオブジェクトを受け（`:106`）、版の一覧に大きさの上限を持たない（`:217`）
- kind: premise
- 提案: Q2 の context を「3 つの部品は共有の材料（`update-check-vectors.json`）で同じ答えを出すことだけを試している。選び方のコードは、サーバの更新係と PC の収集アプリが共有し、携帯は Kotlin で別に持つ。本物の答えの読み方も携帯だけ違う」に直す。推奨（依頼どおり 2 部品）はこのままでよいが、携帯を外すと「携帯の読み方が本物とずれたら気づかない」ことを選択肢 1 の detail に書く。
- 処置: fixed deep-questions.json （fixed Q2 の context）

## R7. Q4 の推奨は、PC の収集アプリの版を組み立てで変えて「本物の最新より 1 つ古い版」を作る前提に立つ。組み立ての規則では、出所の版と土台が違う版は組めない
- 成果物: openspec/changes/ch20-real-release-source-nightly-check/deep-questions.json / premises.md
- 根拠:
  - Q4 の推奨の detail「本物にリリースがあれば、その最新と同じ版・1 つ古い版で回す。…PC の収集アプリは版を変えた試験の組み立てを 2 つ組む」
  - 組み立ての規則: `crates/build-stamp/src/lib.rs:41-58`（`ASHIATO_BUILD_VERSION` の土台が出所の版（`Cargo.toml:6` の `0.1.0`）と違えば組み立てが落ちる）。PC の収集アプリの組み立ては `crates/collector-windows/build.rs:19-30` でこれを使う
  - このため、PC の試験の組み立てで作れる版は `0.1.0` と `0.1.0-<先行の印>` だけになる
  - 出所の版が次の版に上がった後（正典 `spec.md:943`。正式版を出したら出所の版を上げる）は、「本物の最新より古い PC の収集アプリ」を組めない
  - P7 は、版の上書きを更新係（`ASHIATO_UPDATER_VERSION`）でしか確かめていない
- kind: premise
- 提案: Q4 の推奨を「入っている版は組み立てのまま変えず、流し直す一覧の版の名前を書き換えて版の関係（同じ・新しい・古い）を作る」に直す。少なくとも context に、PC の収集アプリの版は土台を変えられないことを書く。premises.md の P7 にもこの制約を足す。
- 処置: fixed deep-questions.json （fixed Q4（推奨を、一覧の側の版の名前を書き換える形に直した））

## R8. Q3 の推奨（配る形を Windows に入れて PC の収集アプリを回す）が、確かめていない前提に乗っている。前提は 2 つ: runner でトーストが成功すること、配る zip がその晩にあること
- 成果物: openspec/changes/ch20-real-release-source-nightly-check/deep-questions.json / premises.md
- 根拠:
  - 配る形の PC の収集アプリは、切り方のトーストが成功するまで問い合わせない:
    - `crates/collector-windows/src/update_check.rs:154-161`（起動時。示した回は確かめない）
    - `:176-188`（定期の時は、ロックされておらず直前 300 秒以内に入力があった時だけ示し直す）
    - 入れたてでも状態のファイルが無ければ「まだ示していない」から始まる（`:108-111`）
  - 正典 `spec.md:2682`「どれでも示せない間は確認が止まったままになる」（入れ替えた収集アプリの文。入れたての PC の収集アプリも、コード上は同じ道を通る）
  - runner のセッションでトーストが成功するかは premises に行が無い
  - 予定の実行は、開発版の公開の設定が無い間は配る形を組まない（`spec.md:942`、`.github/workflows/nightly.yml` の `package-windows` は `build == 'yes'` のときだけ。P3）
  - いまの Windows のインストールの台本は、PC の収集アプリを入れない（`tools/release/update-check-install.sh:21-32` の Scenario の一覧は更新係とインストーラだけ）
- kind: premise
- 提案: premises.md に 2 行を足す（確かさ付き）。1 行目は「runner の Windows でトーストが成功するか」、2 行目は「毎晩の検査が使う配る zip をどこから得るか（その晩に組む／前の組み立てを取る）」。Q3 の推奨の detail にも、この 2 つと、トーストが出なければ PC の収集アプリは問い合わせずに終わることを書く。
- 処置: fixed deep-questions.json （fixed Q3（推奨を、サーバは配る形・PC の収集アプリは試験の組み立てに直した。配る zip は検査の中で組む））

## R9. 抜け: 検査をどの workflow に置くかが、Q7 の推奨の detail の中にしか無い。既存の決まりは、毎晩の検査を公開の流れから分けている
- 成果物: openspec/changes/ch20-real-release-source-nightly-check/deep-questions.json
- 根拠:
  - Q7 の選択肢 1 の detail「検査は、組み立てと公開を受け持つ今の `nightly` の実行とは分けて置く」。Q7 で選択肢 2・3 を選ぶと、置き場が決まらない
  - `lane.json` の paths は `.github/workflows/nightly.yml` だけを名指す
  - 既存の決まり: `tools/check-vulns-wiring.sh:149-150`（`nightly.yml` に脆弱性の検査が混ざれば落とす）・`:154`（Scenario「毎晩の検査は公開の流れから独立し…」）
  - P3 は、`nightly` の実行が 12〜19 秒で success になること（組まずに終わる）を前提の確かめに使っている
  - 依頼の「落ちたら nightly が赤になる」は、どの workflow かを決めていない
  - 過去の取りこぼし「構造の検査を新しく作るなら、どの CI の job が毎回走らせるかを書く」
- kind: technical
- 提案: 置き場を独立の問いにする（B。推奨は「開発版の組み立てと公開の workflow とは別の毎晩の workflow に置き、配線の検査（`check-vulns-wiring.sh` と同じ形）で『公開の流れから独立』『予定と手動の起動を持つ』『時間の上限がある』を見る」）。`lane.json` の paths に新しい workflow を足すことも書く。
- 処置: fixed D1 （fixed Q7 の context（別の実行に置くことを書いた）。置き場の細部は C の既定として design の D に残す）

## R10. 抜け（日常）: 赤になったことを本人がどう知るか。Q7 の推奨では直しが入るまで毎晩赤が続くのに、その間の知らせ方も上限も決まっていない
- 成果物: openspec/changes/ch20-real-release-source-nightly-check/deep-questions.json
- 根拠:
  - 毎晩の脆弱性の検査は、見つけたものを issue にして知らせる（`.github/workflows/vulns-nightly.yml:47-70`）。workflow が赤になるだけでは、通知のメールのほかに届く先が無い
  - Q7 の推奨「赤にする（毎晩、別の実行として赤が続く。直しの change を立てる合図にする）」は、誰かが赤を見ることを前提にしている
  - R3・R4・R5 を入れると、初日から赤になる理由が 3 つ増える
  - 過去の取りこぼし「送らずに止める決定には、止めたことを本人がどう知るかを問う」「解消するまで待つ規則には上限を問う」
- kind: daily
- 提案: B の問いを足す。推奨は「赤の理由ごとに issue を 1 本だけ開いたまま更新し、通った夜に閉じる（毎晩新しい issue を立てない）」。対案に「赤だけ（issue なし）」と「issue を毎晩立てる」を並べる。Q7 の「赤が続く」には「理由ごとに 1 本の issue が開いたままになる」を書く。
- 処置: fixed D9 仮 （rejected: 問いにしない（C）。予定の workflow の失敗は GitHub が起こした人へ通知する既定の仕組みで届く。追加の知らせ方は design の D（仮）に残す）

## R11. 抜け（一方通行）: 毎晩の本物の答えを残すか、比べる基準をどこに置くか。残さなければ、いつから形が変わったかを後から引けない。lane-verdict は人間と決めるとしたのに、一覧に無い
- 成果物: openspec/changes/ch20-real-release-source-nightly-check/deep-questions.json
- 根拠:
  - `lane-verdict.json` の why「配布元の応答の形の変化を何で判定するか（基準の置き場・どの項目の変化を赤にするか）」。Q5 は項目を問うが、基準の置き場と答えの保存を問わない
  - 過去の答えは配布元から取り直せない
  - 本物の答えは公開の情報（リリースの名前・配布物の URL・`digest`・公開した利用者の名前）だけで、本人の記録は入らない
- kind: technical
- loss: uncaptured
- 提案: Q5 の兄弟として問いを足す。失わない側（その晩の本物の答えと判定の要約を run の成果物に残し、基準は repo の中の固定のファイルに置いて変えるときは PR で変える）は安く、範囲内で、要件を変えないので、推奨を付けて仮（P）で進めてよい。選択肢には「残さない側を選ぶと、形が変わった日を後から特定できない（uncaptured）」を書く。
- 処置: fixed D13 仮 （rejected: 問いにしない（C、扉を開けたままにする既定）。毎晩の本物の答えは artifact に残し、比べる基準は repo の材料に置く。design の D に残す）

## R12. 抜け: 検査の側の問い合わせ（Q5 の本物の形・Q6 の別の repo・Q8 の前の実行の読み出し）に、workflow の資格情報を使うか
- 成果物: openspec/changes/ch20-real-release-source-nightly-check/deep-questions.json
- 根拠:
  - 正典が資格情報を禁じるのは製品の問い合わせだけ（`spec.md:2519`「問い合わせのどこにも…資格情報…を入れない」）
  - 依頼の「1 回の実行で問い合わせは数回まで」は、共有の IP の上限 60 回（P8）を理由にしている
  - 検査の側の問い合わせに資格情報を付ければ別の上限で数えられ、共有の上限は製品の問い合わせだけで使う
  - Q6 の推奨（別の公開の repo を読む）と Q8 の推奨（前の実行の結果から数える）は、どちらも問い合わせを足す
- kind: technical
- 提案: B の問いか、Q8 の context に書く。推奨は「製品の問い合わせは資格情報なしのまま。検査の側の問い合わせだけ workflow の読み取りの資格情報で行い、共有の上限を製品の問い合わせだけで使う」。依頼の「数回まで」が製品の問い合わせの数を指すと書く。
- 処置: fixed D4 （rejected: 問いにしない（C）。検査の側の問い合わせだけ workflow の資格情報を使い、製品の問い合わせは資格情報なしのまま。design の D に残す）

## R13. 前提が足りない（P5・P6・Q1 の context）。記録の形から、更新係も PC の収集アプリも一度も確かめを終えていないと言える。P6 の根拠の zip には更新係が入っていない
- 成果物: openspec/changes/ch20-real-release-source-nightly-check/premises.md / deep-questions.json
- 根拠:
  - `enabled: false` を書くのは `update-check` の口だけ:
    - `crates/installer/src/main.rs:211-237`・`:253-254`・`:3141-3144`
    - 引数が `on` でなければ切る。引数が無くても切る
    - インストールは `enabled: true` を書き、ファイルが既にあれば何も書かない（`main.rs:2699`・`:216-218`）
    - よって `since 2026-11-18T22:58:04Z` は、22:58:04Z（日本時間 07:58）に `update-check` の口が走った時刻
  - 更新係は、確かめた回を成否に依らず `checks.jsonl` に追記する（`crates/updater/src/lib.rs:243-253`）。`once` が失敗すれば `logs/updater.log` に書く（`:286-289`）。どちらも無いので、有効だった 07:50〜07:58 にも更新係は一度も確かめを終えていない
  - PC の収集アプリの `checked_at: null` は、`check()` を一度も通っていないことを示す（`crates/collector-windows/src/update_check.rs:203-`。送り先が無くても「宛先なし」を記録して `checked_at` を埋める）
  - 結論: 配布元の答えに由来する記録は、どちらの部品にも 1 行も無い
  - サーバの画面は、この記録のもとでは `pending` → `off` を出す（`crates/server/src/update_status.rs:211-219`・`:205-206`）。設定を読めなければ `unreadable`
  - P6 の「`ashiato-server-0.1.0-windows-x86_64.zip`（2026-11-09）」は、更新係のコードより前のもの（`crates/updater/src/lib.rs` を足した最初の commit `3bf2abed8` は 2026-11-12）。それなのに本人の Windows には更新係の置き場がある。入っているのは 11-09 の zip ではない（版の出所は `Cargo.toml:6` の `0.1.0` のままなので、版番号の読みは変わらない見込み）
- kind: premise
- 提案:
  - P5 に、上の推論の連なり（file:line つき）を足す
  - P6 の確かめ方を「アプリと機能」の版の表示（`crates/installer/src/main.rs` の `uninstall_registration_commands` が書く）に替え、確かさを付け直す
  - Q1 の context に「どちらの部品にも、配布元の答えに由来する記録は無い（更新係は有効だった 8 分にも確かめを終えていない）。07:58 に `update-check` の口で切られている」を書く。本人が何を見たかの推測は書かない
  - Q1 の推奨（覚えていない → この change は検査だけ）の irreversible に、「記録の上では、本人が見たものはこの検査が捕まえる種類ではない見込みが高い」を足す
- 処置: fixed premises.md （fixed premises.md P5・P6・fixed Q1 の context と推奨の irreversible）

## R14. 範囲の食い違い: lane の paths に製品のコード `crates/update-check/**` がある。Q9 の推奨は「製品のコードを変えない」。壊してはいけないものの候補からも 4 つが漏れている
- 成果物: openspec/changes/ch20-real-release-source-nightly-check/deep-questions.json
- 根拠:
  - `lane.json` の paths と `lane-verdict.json` の `"code": ["crates/update-check/**"]`。この crate は 3 つの部品のうち 2 つが配る形に組み込む選び方そのもの
  - Q9 の context の「壊してはいけないもの」の候補は、製品の振る舞い・D18・問い合わせに資格情報と版を入れないこと、の 3 つ。次の 4 つが無い:
    - 配る組み立てに宛先を変える口を足さない（`spec.md:2522`。Q3・Q4 の推奨が配る形を本物に当てるときに触れうる）
    - 試験の組み立ては差し替えの宛先が無ければ確かめない（`spec.md:2523` の前半）
    - 開発版の組み立てと公開の workflow の振る舞い（`spec.md:942`。予定の実行は組まずに終わる）
    - 毎晩の検査を公開の流れから分ける決まり（`tools/check-vulns-wiring.sh:149-150`）
- kind: technical
- 提案:
  - Q9 の context の候補に、上の 4 つを足す
  - Q9 の推奨を「直さない」にするなら、`crates/update-check/**` に触れてよいのは試験（`#[cfg(test)]`）と材料だけ、と書く。または lane の paths から外す
  - 製品のコードに触れる余地を残すなら、Q9 の選択肢 2 と同じく正典の振る舞いの変更になることを書く
- 処置: fixed deep-questions.json （fixed Q9 の context（壊してはいけないものの候補を足した）。lane.json は昇格済みの履歴なので書き換えない（触れる予定のパスは spec の段で proposal に書く））

## R15. Q4 の選択肢 3 は外に出る（exported）のに、`loss` が無い。Q4 の推奨は、Q6 の推奨に乗っているのに、それが書かれていない
- 成果物: openspec/changes/ch20-real-release-source-nightly-check/deep-questions.json
- 根拠:
  - Q4 の why「配布元に置いたリリースを見た部品の知らせは取り消せない」と、選択肢 3 の irreversible「先行公開を置くと、ベータ版・開発版の利用者の部品がそれを見て知らせる（外に出る）」
  - それなのに Q4 は `kind: open` で、`loss` の欄が無い。失わない側（選択肢 1）が推奨で、安く、範囲内なので、仮（P）で進めるのは規則どおり（`.harness2/docs/flow-gates.md:275`）。ただし台帳の `provisional.py add` に載せるには `loss` が要る
  - Q4 の推奨の「リリースが 0 件の間は、本物の 1 件の形に…（形は Q5 で取る）」は、0 件の間の 1 件の形を Q6 の推奨（別の公開の repo）からしか得られない。Q6 で選択肢 2・3 を選ぶと、Q4 の推奨は材料を失う
- kind: technical
- loss: exported
- 提案: Q4 に `loss: exported` を付ける（推奨は選択肢 1 のままで、仮で進める側）。Q4 の推奨の detail に「Q6 で別の repo を読まない答えなら、0 件の間は共有の材料の形で代える」と書く。または Q4 を Q6 に従う問いとして結ぶ。
- 処置: escalated （fixed Q4 の選択肢 3 の irreversible に「外に出る（exported）」を明記し、推奨が Q6 に乗ることを書いた。問いの loss は付けない —— 推奨の側は失わず、付けると A になって第 1 回の答えを必須にしてしまう（選択肢 3 を選ばれたら、そのとき A として扱う））

