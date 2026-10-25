# ST49 spec-review（spec --draft。第 1 回の推奨を答えとして読む）

対象: `openspec/changes/st49-record-component-version/` の proposal / specs（7 capability・ADDED 13 Requirement・47 Scenario）/ design / tasks、
`docs/stories/ST49.md`・`stories.json`・`INDEX.md`（訂正の節）。前提は premises.md・review/deep.md・provisional.jsonl（D1〜D4）。
mode が `--draft` なので、`deep.md` の `（答え待ち・推奨: …）` を本人の答えとして読んだ。観点 6（review/answers.md）は書かない。

## 機械の検査（2026-10-25、`/home/yosis/dev/ashiato2-up-st49` で実行）

| コマンド | 結果 |
|---|---|
| `openspec validate st49-record-component-version --strict` | `Change 'st49-record-component-version' is valid`（rc=0） |
| `python3 scripts/check_chain.py .` | `chain: OK (0 件 / 未回収 0 件 / warn 0 件)`。観点 8（再生成との一致）も通る |
| `python3 scripts/check_scenarios.py . st49-record-component-version` | `scenarios: FAIL (担保なし 47 件 / 名無しの確認待ち 0 件)` —— 47 件はすべてこの change の ADDED（実装前なので印がまだ無い。想定どおり）。spec の 47 の名前は全部 tasks.md に `Scenario: <名前>` で逐語に名指しされている（spec から名前を抜いて tasks.md を grep し、欠け 0） |
| `python3 scripts/review_triage.py . st49-record-component-version` | `triage: FAIL (4 件)` —— `deep.md R2/R3/R4/R5: escalated なのに deep.md に R<n> が無い`（→ R1） |
| `python3 scripts/evidence.py lint openspec/changes/st49-record-component-version` | `[OK] 検証コマンド（st49-record-component-version）`（rc=0） |
| `python3 scripts/spec_overlap.py candidates . st49-record-component-version` | `[]`（候補 0 件） |
| `python3 scripts/spec_overlap.py check . st49-record-component-version` | `{"ok": true, "candidates": 0}` |

## R1. deep.md が問いの出所（review/deep.md の R2〜R5）を名指さず、review_triage が落ちる
- 成果物: openspec/changes/st49-record-component-version/deep.md
- 根拠: `python3 scripts/review_triage.py . st49-record-component-version` → `[FAIL] deep.md R2: escalated なのに deep.md に R2 が無い`（R3・R4・R5 も同じ）/ `scripts/review_triage.py:206-208` /
  review/deep.md の処置（R2 → Q3、R3 → Q4、R4・R5 → Q1 の族と推奨）/ 先例 `openspec/changes/st48-db-version-guard/deep.md:8`（`- 出所: review/deep.md R1・R4（問いの段の独立検査で escalated にしたもの）`）。
  ST49 の deep.md（c4a604c1 で作られた 23 行）には `R` の名指しが 1 つも無い
- kind: technical
- 提案: deep.md の Q1 に `- 出所: review/deep.md R4・R5`、Q3 に `R2`、Q4 に `R3` を足し、`review_triage.py` を通してから GO 資料に進む。
- 処置: fixed deep.md —— Q1 に `出所: review/deep.md R4・R5`、Q3 に `R2`、Q4 に `R3` を足した（ST48 の先例の形）

## R2. `.dirty` の印は、add していない作業木の変更では作り直されない（D6 と Risks が逆のことを言い、Risks の方が誤り）
- 成果物: openspec/changes/st49-record-component-version/design.md
- 根拠: design.md:76-78（D6。「作業木のファイルだけを書き換えて add しない変更は、そのファイルを含む crate の作り直しでは拾えない —— Risks」）と
  design.md:187-188（Risks。「`rerun-if-changed` はその crate のソースの変化でも作り直すので、変えた crate 自身の印は新しくなる」）が食い違う。
  cargo は build script が `rerun-if-changed` / `rerun-if-env-changed` を 1 つでも出すと、パッケージのファイルの変化では build script を走らせ直さない（既定の「パッケージ全体を見る」が止まる）。
  `crates/server/build.rs:20-21` は今すでに `migrations/ORDER` と `migrations/` だけを `rerun-if-changed` に出しているので、`src/*.rs` を書き換えても build.rs は走らず、`env!("ASHIATO_BUILD_STAMP")` は前回の値のまま埋め込まれる。
  よって「未 commit の変更があれば `.dirty`」（record-envelope spec:10、desktop spec:10、C10「組み立てのたびに作り直す」「古い印を許さない」）は、stage していない編集の後の差分の組み立てで破れる。
  Risks の「影響は手元の組み立てだけ」も、premises の 4（「配布の経路に切り替える日まで全部が手元の組み立て」）と Q1 の context に照らすと、ST52 までに入る記録の全部が該当する。
  Scenario「S-01 の組み立ての印は版番号・commit・未 commit の印・組み立ての種類を持つ」は**汚れた木から 1 回組み立てる**形で、差分の組み立てでの `.dirty` の付き方を見ていない。
  付記: 使い捨ての git の木で `touch f` の後に `git status --porcelain` を走らせると `.git/index` の mtime が進み（1792889634 → 1792889636）、`GIT_OPTIONAL_LOCKS=0` では進まなかった。D6 が `<git-dir>/index` を `rerun-if-changed` に出すので、build.rs の中の `git status` が index を書くと次の組み立てでもう 1 回走る
- kind: technical
- 提案: 作り直しの入力に workspace の各 crate の `src`（または作業木の追跡対象）を足すか、build.rs を毎回走らせる（費用を D6 に書く）形に D6 を直し、Risks の誤った文を消す。
  Scenario を 1 つ足す:「組み立てた後に、ソースを 1 つ書き換えて add せずにもう一度組み立てると、印は `.dirty` を持つ」。build.rs の `git status` には `GIT_OPTIONAL_LOCKS=0` を付ける。
- 処置: fixed D6 —— `.dirty` を「その部品のソースの未 commit の変更」と定め（D5。S-01・C-02 は Rust の workspace、C-01 は `collector-android/`、V-01 は `web/`）、build.rs の `rerun-if-changed` に部品のソースの道を足した。`git` は `GIT_OPTIONAL_LOCKS=0` で呼ぶ。Risks の誤った文を消した。record-envelope に「組み立ての印は…組み立てのたびに作り直す」の Requirement を立て、add しない変更の後の `.dirty` の Scenario（S-01・C-01・C-02）を足した。検証は 1.2 の `tools/check-build-stamp.sh` が本物の部品を写した木で組み立て直して見る

## R3. Q2 の推奨（衛星の記録にも S-01 の版）は FR-123 の本文と Story の完了の判定に反し、要件へ戻す道が機械に載っていない
- 成果物: openspec/changes/st49-record-component-version/specs/record-envelope/spec.md
- 根拠: record-envelope spec:6（「S-01 の版を、すべての記録（収集アプリ・書庫の取り込み・派生・画面・衛星の記録）に」）/
  `docs/requirements.md:1020-1021`（FR-123 ★ Q13「衛星の記録は対象にしない」）/ `docs/stories/ST49.md:50`（完了の判定「衛星が書いた記録には版を付けない」）/
  design.md:121-122（D10 の「record の段への申し送り」は散文だけ）/ deep.md に「要件へ戻すもの」の節が無く、`review_triage.py` の出力は `要件へ戻すもの 0 件`（`scripts/review_triage.py:233-246` はこの節が無いと ★ の有無を見ない）
- kind: conflict
- 提案: Q2 が推奨で答えられたら record が deep.md に「要件へ戻すもの: FR-123（S-01 の版を持つ記録の範囲を全記録へ、衛星は「衛星自身の版を付けない」）」を置き、
  requirements.md に ★ 2026-10-xx で入れて ST49.md を再生成する（check_chain の観点 8 が完了の判定の書き換えを確かめる）。Q3（破棄の報告・台帳）と Q4（画面の版）も FR-122 / FR-123 の「記録」の外へ広げるので、同じ節で ★ に戻すかを決める。
- 処置: escalated —— Q2（A の問い）はすでに人間に渡る問いで、推奨が採られると FR-123 と完了の判定が広がる。deep.md の Q2 に「要件との関係」を書き、末尾に「record の段で要件に戻すかを決めるもの」の節（Q2 → FR-123 と完了の判定、Q3 → FR-122 / FR-123 の「記録」の範囲、Q4 → FR-123）を置いた。答えを受けた record が「要件へ戻すもの」に移して ★ を入れる（`review_triage.py` の検査 4 がそこから効く）

## R4. Q3 の族（画面の操作が作る追記のみの行）に、滞在の基準と個人属性の種類の名前が抜けていて、design はそれを「中身を持たない器」と呼んでいる
- 成果物: openspec/changes/st49-record-component-version/deep-questions.json
- 根拠: deep-questions.json の Q3 の family（`core.deletion_ledger`・`core.feeling_reminder` だけを「画面の操作の台帳」に挙げる）/
  design.md:33-34（Non-Goals:「中身を持たない器・S-01 の内部の台帳（`core.place`・`core.attribute_kind(_name)`・`core.stay_criteria`…）」）/
  `migrations/202609142125_stays.sql:18-26`・`:59-63`（`core.stay_criteria` は半径・最短・間隔・ソースを持ち、追記のみのトリガ）と `crates/server/src/lib.rs:2032`（`stays.set_criteria` —— 画面の作り直しの要求で本人が選んだ基準を書く）/
  `migrations/202609160220_personal_attributes.sql:40-47`（`core.attribute_kind_name.name` —— 本人が付けた名前。追記のみ。`crates/server/src/attributes_store.rs:109`・`:188`・`:230` が書く）。
  どちらも `core.feeling_reminder`（`feelings.rs:791`。本人が選んだ時刻を 1 行書く）と同じ形（画面の操作 → S-01 が追記のみの表に値を 1 行）で、後から列を埋められない
- kind: premise
- 提案: 本人に Q3 を見せる前に、family に `core.stay_criteria`・`core.attribute_kind_name` を足し、推奨の側に入れるか、外す理由を Non-Goals に書き直す（「中身を持たない」は `core.place`・`core.attribute_kind` にしか当たらない）。
- 処置: escalated —— deep-questions.json の Q3 の族に `core.stay_criteria`・`core.attribute_kind_name`（未決 Q3）と、中身を持たない器・S-01 の内部の台帳（聞かない）を足し、推奨の選択肢の「画面の操作の台帳」を「削除・気分の通知・滞在の基準・個人属性の種類の名前」に広げた（deep.md の推奨も同じ文に）。推奨の側で specs（derived-records・personal-entities・data-durability）・design（Non-Goals・D7・D10・D17）・tasks を書き、INDEX の `personal-entities` に ST49 を足した。Q3 は A の問いのまま人間に渡る

## R5. ST23 が足す削除の台帳の行と新しい操作の台帳に、ST49 の「削除の台帳の行は S-01 の版を持つ」が届かない
- 成果物: openspec/changes/st49-record-component-version/specs/record-deletion/spec.md
- 根拠: record-deletion spec:5（「削除の台帳（消した・戻した操作）の 1 行ごとに」）/ design.md:119（D10 が名指す INSERT は `deletion.rs`・`stay_store.rs` の 2 か所）/
  `openspec/changes/st23-record-erasure/design.md:96`（本文ごと消すときに `core.deletion_ledger` へ `erase`・`mark = 'user:purged'` の行を足す）・`:179`（遅れて届いた記録で `core.erasure_operation` を積む）・`:114`（新しい追記のみの 2 表）。
  D7 の列は `text NULL`・既定値なし・CHECK なし（design.md:88-100）なので、ST23 の INSERT が `server_version` を書かなくても何も落ちない。ST49 の試験（tasks 6.1）は消す・戻すの 2 経路だけ
- kind: technical
- 提案: `docs/handoff/ST23.md` に「`core.deletion_ledger` の INSERT は `server_version` を書く。`core.erasure_operation` に版を持たせるかは Q3 の答えに従う」を申し送る（tasks 11.1 に足す）。
  機械の止めとして、S-01 だけが書く表（削除の台帳・気分の通知の台帳）には `CHECK (server_version IS NOT NULL) NOT VALID`（既存の行を見ず、新しい行だけに効く）を置くかを D7 で決める。
- 処置: followup ST23 —— `docs/handoff/ST23.md` に「削除の台帳の INSERT に `server_version`」を申し送った。機械の止めとして、`server_version` を持つ表への INSERT が列を名指すことを文字列で見る試験 `st49_ledger_inserts_name_server_version`（tasks 6.1・design D10）を置いた。CHECK `NOT VALID` は、表へ直に INSERT する既存の試験（`feelings_tests.rs:116` など）を全部書き換えることになるので置かない

## R6. 外部サービス由来の更新の Scenario が、本表の S-01 の版が動くことを確かめていない
- 成果物: openspec/changes/st49-record-component-version/specs/record-envelope/spec.md
- 根拠: record-envelope spec:97（「3 つの版を、届いた要求の部品の版と、その更新を書いた S-01 の版に替える」）と spec:104-107（THEN は本表の収集アプリの版と、前の版の行の 2 つの版だけ）/
  tasks.md:101-102（4.1 は同じ試験バイナリで送るので `BUILD_STAMP` は 1 つ。滞在の 5.1 のように `server_version` を先に書き換えて「別の S-01」を作る手順が無い）。
  本表の `server_version` を UPDATE し忘れても、試験の中では最初の値と同じ `BUILD_STAMP` が残るので落ちない
- kind: technical
- 提案: Scenario を「印 A の S-01 が入れた記録を、印 D の S-01 が更新すると、本表の S-01 の版は D、前の版の行の S-01 の版は A」に分け、4.1 に「入れた後に `server_version` を直に `0.1.0+<commit A>` へ書き換える（D16 の門の外）」を書く。
- 処置: fixed 4.1 —— Scenario を「印 A の S-01 が入れた記録を印 D の S-01 が更新すると、本表の収集アプリの版と S-01 の版が D、前の版の行が A」に直し、4.1 に最初の記録を試験から直に INSERT して `server_version` を `0.1.0+<commit A>` にする手順を書いた

## R7. 「収集した記録の版だけを直すことはできる」が、C8 / D16 の「版を書き換えるのは前の版を残す経路だけ」と食い違う
- 成果物: openspec/changes/st49-record-component-version/specs/record-envelope/spec.md
- 根拠: record-envelope spec:126（「版を書き換えるのは中身が入れ替わる更新の経路だけで、そのとき前の値は前の版に残る」）と spec:128-131（Scenario:「前の版の履歴を書かずに書き換える」→「通る」）/
  premises.md:48-49（C8「代わりに、列を書き換えるのは C5 の経路（前の版を残す）だけにする」）/ design.md:155（D16）/ provisional.jsonl の D1 の keeps（「前の値は前の版の表に残るので、どちらの読み方にも後から戻せる」）。
  Scenario が認める直の書き換えでは前の値がどこにも残らず、D1（仮）の「戻せる」が成り立たない。「だけにする」を強制する仕組みも design に無い
- kind: conflict
- 提案: どちらかに寄せる。(a) 直の書き換えも認めるなら、C8・D16・spec:126 の「経路だけ」を消し、直した前の値が残らないことを代償に書く。
  (b) 前の値を残すなら、門の外のまま「版だけを書き換えるときは前の版の行を積む」トリガ（`device_id` の先例にはない）を置き、Scenario を「直すと、前の値が前の版に残る」にする。
- 処置: fixed D21 仮 —— (b) の側。収集した記録の版の列を本表の門（`core.require_version_or_ledger`）の対象にし、版を直すときも前の版を同じまとまりに積む（直す前の値が前の版に残る）。Scenario を「前の版なしの書き換えは拒まれる」「前の版を積めば直せて前の値が残る」に直し、provisional.jsonl に D21 で載せた（rests-on D1）

## R8. 観測できる振る舞いが design にだけあり、Scenario に無い（archive で正典から落ちる）
- 成果物: openspec/changes/st49-record-component-version/design.md
- 根拠:
  - design.md:106（D8:「前後の空白を除いて空なら NULL」「U+0000 を取り除いてから持つ」「長さは切り詰めない」）—— spec は「文字列のまま持ち」「空の文字列は不明」（record-envelope spec:8）だけ。前後の空白を除くかは「文字列のまま」と矛盾しうる。tasks.md:84 は U+0000 の確かめを別の Scenario の試験に混ぜている
  - design.md:151（D15:「保存済みの下書きに `clientVersion` が無ければ…閉じ、閉じた報告は版の欄の無いまま送る」）—— tasks.md:174 の試験は「印なし」
  - design.md:41（D1:「削除済み（`SkippedDeleted`）では版を変えない」）—— spec のどこにも無い
- kind: technical
- 提案: 3 つを Scenario にする（「前後に空白だけの版は不明・U+0000 を含む版は取り除いて残る」「版を持たない保存済みの下書きは閉じられ、報告は版の欄なしで届く」「削除済みの記録への更新では版は変わらない」）。前後の空白を版の中から除くかは「文字列のまま」と揃えて決める。
- 処置: fixed D8 —— 「空の文字列だけを NULL、前後の空白は除かない（文字列のまま）、U+0000 は取り除く」に決め、Scenario「部品の版が空か形の違う文字列でも…」の値を前後に空白のある文字列にし、「格納できない文字を含む部品の版は、その文字を取り除いて残る」を足した。D15 の版の無い保存済みの下書きと、D1（仮）の削除済みへの更新も Scenario にした（device-collection・record-envelope）

## R9. Requirement の本文だけが言い、Scenario が言っていないこと
- 成果物: openspec/changes/st49-record-component-version/specs/
- 根拠:
  - desktop-collection spec:9（ブラウザ履歴の読み直しには読み直した時点の版）—— Scenario 無し。tasks.md:151 の `st49_c02_history_reread_gets_current_stamp` は「印なし」
  - record-envelope spec:99（古い到着として退けた更新では版を変えない）—— Scenario 無し。tasks.md:103 が「内容の変わらない再到着」の試験に混ぜている（片方だけ通っても緑）
  - device-collection spec:10・desktop-collection spec:10（未 commit なら `.dirty`、release でなければ `.debug`）—— C-01・C-02 の Scenario は commit の作り直しだけ（device spec:40-43、desktop spec:35-38）。
    同じ commit の debug / release を分けることは Q1 の推奨が名指した族で、C-01 は今 debug の鍵で入っている（扉 34）のに、C-01 の release の variant が `.debug` を持たないことを見る Scenario も試験も無い（tasks.md:158-161 は commit と正規表現だけ）
  - record-deletion spec:6・subjective-log spec:6（版を残し始める前の台帳の行は不明のまま）—— Scenario は `core.event` だけ（record-envelope spec:37-40）
  - record-envelope spec:147（画面の表示と操作を変えない）—— Scenario 無し
- kind: technical
- 提案: 上の各項を 1 主張 1 Scenario で足す。少なくとも「C-01 の release の組み立ての印は `.debug` を持たない」「ブラウザ履歴を読み直した要求は読み直した時点の印を持つ」「古い更新時刻の到着では版は変わらない」を別の Scenario にする。
- 処置: fixed 9.1 —— 古い更新時刻の到着・ブラウザ履歴の読み直し・C-01 の release に `.debug` が付かないこと・C-01 / C-02 の `.dirty`・台帳の既存の行が「不明」のまま（削除・気分の通知・滞在の基準・個人属性の種類の名前）を 1 主張 1 Scenario で足し、tasks（2.2・4.1・8.1・9.1）が名指す。「画面の表示と操作を変えない」は観測の形を持たないので Requirement の本文から外し、proposal の Impact に置いた

## R10. S-01 と C-02 の組み立ての印の Scenario を、S-01・C-02 でない最小の crate で担保している
- 成果物: openspec/changes/st49-record-component-version/tasks.md
- 根拠: tasks.md:49-56（1.2: `$TMPDIR` の最小の crate を組み立てて印を比べ、その台本に S-01 の 3 つと C-02 の 1 つの Scenario の印を置く。`crates/server/build.rs` と `crates/collector-windows/build.rs` は `ashiato_build_stamp` を呼ぶかを grep するだけ）/
  record-envelope spec:42-45（THEN は「記録の S-01 の版は…の形」—— S-01 が格納した記録を見る）・spec:47-50 / desktop spec:35-38。
  本物の `build.rs` が `emit_rerun` を呼ばない・別の版番号を渡すなどの誤りは、grep が通る限り落ちない（R2 の誤りもこの形では見えない）
- kind: technical
- 提案: 印の作り方の 3 つの Scenario は WHEN を「印を組む道具が、未 commit の変更がある木で…」のように `crates/build-stamp`（最小の crate で制御できる側）の話に書き直す。
  そのうえで「S-01 が格納した記録の S-01 の版は、D5 の正規表現に合い、S-01 の `BUILD_STAMP` と同じ」を 1 つ足して 3.1 の試験に置き、本物の `build.rs` が印を出していることは S-01・C-02 の crate を実際に組み立てる形（R2 の差分の組み立ても含む）で 1.3 / 8.1 に置く。
- 処置: fixed 1.2 —— 印の作り方の Scenario は WHEN を「印を組む道具が…」に書き直して 1.1 の `crates/build-stamp` の単体試験に置き、本物の部品の組み立て直し（commit を足す・add しない変更）は 1.2 の `tools/check-build-stamp.sh` の `server` / `windows` / `android` の節が repo を写した木で本物の部品を組み立てて見る。「記録の S-01 の版は、その S-01 の組み立ての印と同じで、印の形を持つ」を足し、3.1 に置いた

## R11. Task 1.2 の検証は 1.2 の時点で通らない形で、1.3 の検証は終了条件が機械で決まらない
- 成果物: openspec/changes/st49-record-component-version/tasks.md
- 根拠: tasks.md:56（1.2:「Task 8 の前は C-02 の grep の行で落ちる。Task 8 の後に取り直す」）/
  tasks.md:60（1.3:「`check-build-stamp.sh rust` の S-01 の 3 つの印の節が通る（出力に `FAIL` が S-01 の節に無い）」—— rc で決まらず、出力を人が読む）
- kind: technical
- 提案: `check-build-stamp.sh` に節を選ぶ引数（`rust server` / `rust windows`）を持たせ、1.2・1.3 は `rust server` の rc=0、C-02 の grep と節は 8.1 の `rust windows` の rc=0 に移す。
- 処置: fixed 1.2 —— `tools/check-build-stamp.sh` に節（`tool` / `server` / `windows` / `android`）を持たせ、1.2 は `tool`、1.3 は `server`、8.1 は `windows`、9.1 は `android` の rc=0 で判定する

## R12. 6.2 が破棄の報告と原本の欠けの要求の形を変えるのに、契約の再生成は 3.3 にしかない
- 成果物: openspec/changes/st49-record-component-version/tasks.md
- 根拠: `docs/openapi.json` の `components.schemas` に `DropReportRequest`・`MediaMissingRequest` がある（`python3 -c` で確かめた）/ tasks.md:95-97（3.3 で再生成・`IngestRequest` だけを `jq` で見る）/
  tasks.md:127-129（6.2 は `drops.rs`・`media_missing.rs` の要求に `client_version` を足すが、検証に `check-openapi.sh` も再生成も無い）。11.2 の `./tools/check-openapi.sh` で初めて落ち、直すタスクが無い
- kind: technical
- 提案: 6.2 に `cargo run -q -p ashiato-server --bin openapi > docs/openapi.json` と `./tools/check-openapi.sh` rc=0、`jq -e '.components.schemas.DropReportRequest.properties | has("client_version")'`（`MediaMissingRequest` も）を足す。`docs/collector-contract.md` の破棄の報告と原本の欠けの節にも欄を足す。
- 処置: fixed 6.2 —— 6.2 に契約の再生成・`./tools/check-openapi.sh` rc=0・`DropReportRequest` / `MediaMissingRequest` の `client_version` の `jq` と、`docs/collector-contract.md` の破棄の報告と原本の欠けの節を足した

## R13. 画面の 2 つの Scenario を vitest（jsdom）で担保している
- 成果物: openspec/changes/st49-record-component-version/tasks.md
- 根拠: `AGENTS.md`（「画面の Scenario は `web/e2e`（本物のブラウザ）で担保する。jsdom へも『人間の確認待ち』へも逃がさない」）/ `CLAUDE.md` の構成の表（`web/src/__tests__` は「指定と勘定まで」）/
  tasks.md:181-185（10.1: 2 つの Scenario を `web/src/__tests__/buildStamp.test.ts` に置き、組み立ての関数の戻り値を見る）。
  Scenario「画面が送る…1 件は、画面の組み立ての印を持つ」（record-envelope spec:152-155）は画面が取り込み口へ送る要求の話で、`vitest.config.ts` の `define` ではなく `vite build` した画面が送る本文が観測の対象
- kind: technical
- 提案: 前者の Scenario は `web/e2e` で `vite build` した画面から気分・場所・個人属性の主張を 1 件ずつ送り、`page.route('/api/ingest')` で本文の `client_version` が画面の印（D5 の正規表現）であることを見る。後者（印の形）は vitest のままでよいかを `docs/testing.md` の区分で決めて書く。
- 処置: fixed 10.2 —— 画面の Scenario を `web/e2e/st49-client-version.spec.ts`（本物の画面から 3 種の記録を送り、届いた本文の `client_version` を見る）に移した。印の形は「組み立ての印は…」の Requirement に移して `crates/build-stamp` の試験が持ち、vitest は `web/build-stamp.ts` の関数の確かめ（印なし）だけにした

## R14. proposal の Requirement の数と、specs に入った実装の名前
- 成果物: openspec/changes/st49-record-component-version/proposal.md
- 根拠: proposal.md:44（`record-envelope` は「ADDED 5 つ」）と `grep -c '^### Requirement' specs/record-envelope/spec.md` = 6（「画面は、本人が書く記録の要求に、読み込まれた画面の組み立ての印を載せる」が数に入っていない）/
  specs に実装の名前: desktop-collection spec:10（`Cargo.toml`）・device-collection spec:10（`versionName`）・record-envelope spec:146・:160（`web/package.json`）・record-envelope spec:45（`Cargo.toml`）
- kind: technical
- 提案: proposal の数と並びに画面の Requirement を足す。版番号の出所は spec では「その部品の宣言した版番号」とし、ファイル名と gradle の名前は design D5 に置く（ST52 が出所を 1 つにすると spec だけが古くなる）。
- 処置: fixed proposal.md —— record-envelope の Requirement の数と並び（7 つ）を直した。spec の版番号の出所は「その部品の宣言した版番号」とし、`Cargo.toml`・`versionName`・`web/package.json` は design D5 に移した

---

## 観点 1: deep の決定が正典に写っているか

推奨を答えとして読んだ。Q1 → record-envelope spec:10・:42-55、device / desktop の印の Scenario（40 桁は Scenario にある）。Q2 → record-envelope spec:6 と各 Scenario の「S-01 の版はその S-01 の組み立ての印」。
Q3 → device-collection の 2 つ目の Requirement・record-deletion・subjective-log・data-durability。Q4 → record-envelope の 6 つ目の Requirement。仮の D1〜D4 はそれぞれ Scenario がある。
外れたもの: R2（Q1 の `.dirty` が差分の組み立てで成り立たない）・R3（Q2 が要件と Story に反し、戻す道が無い）・R4（Q3 の族の欠け）・R9（Q1 の `.debug` が C-01 / C-02 の Scenario に無い）。
「当初案を覆したもの」（review/deep.md で C5 の凍結 → D1（仮）、C8 の「開き直せば揃う」→ Q4、旧 Q3 → C12）は、覆した後の内容だけが specs に残っている（凍結の文・旧 C8 は spec に無い）。

## 観点 2: Scenario が検証可能か

R6・R7・R9。それ以外の Scenario は WHEN / THEN が観測できる値（印の形・`null`・件数・拒否）で書かれている。

## 観点 3: 置き場の誤り

R8・R14。proposal の Capabilities（7 つ）・`specs/` のディレクトリ（7 つ）・INDEX.md の capability 表（ST49 を 7 つに載せる）は一致し、前倒しの 4 つ（derived-records・data-durability・record-deletion・subjective-log）は INDEX.md:463-470 の訂正の節に理由がある。

## 観点 4: tasks が検証を持つか

R10・R11・R12・R13。それ以外のタスクはコマンドと rc と本数の下限を持つ。下限の数は今の木と合っていた（`grep -cE '#\[(tokio::)?test'` で dedup_tests 35・stay_tests 65・drops_tests 15・media_missing_tests 6・deletion_tests 26・feelings_remind_tests 8、`export_` を名に持つ試験は 145 + 10）。
移行の試験を使い捨ての DB で走らせる形（2.2）は `media_migration_tests.rs:27-30` の先例がある。実機・実データの確認待ちが要るタスクは無い（C-01 の更新をまたぐ未送信は Robolectric の模擬で見る）。

## 観点 5: Story が要件の現在の本文と一致しているか

`check_chain.py` の観点 8 は通る（ST49.md は今の requirements.md の FR-122 / FR-123 から再生成した形で、requires `[ST01, ST48]`・layer 2 は stories.json・INDEX.md と一致）。
完了の判定と deep の推奨の矛盾は R3（衛星）。satisfies の外へ広げる Q3・Q4 も R3 の提案に含めた。

## 観点 6

`--draft` なので書かない。

## 観点 7: 走っている change との前提関係

観点 7: 該当なし（spec_overlap.py candidates が 0 件）

機械の候補の外で、依頼にあった 4 本を読んだ（判定の行ではない。spec_overlap は候補だけを見る）:
ST48 はこの change の印と移行の規則の土台で、requires に入っている（ST48.md の requires との関係は INDEX.md の訂正の節）。
ST41 は record-envelope の 3 つを MODIFIED し、ST49 はそれらに触らず ADDED だけ。同じ `apply_external_update`・前の版の INSERT・`core.event_version_live`・`core.reject_version_rewrite`・書き出しの一覧に列を足すが、
書き出しの一覧は ST41 が `tz_id` の次（`git show feat/st41-event-end-time:crates/server/src/export.rs` の 189-194 行）、ST49 は `schema_version` の次で、行が重ならない。後から入る側が移行の本体を写し直す（D19）ことは、ST49 の `st49_cols_`・`st49_gate_history_versions_are_frozen` が落ちて知らせる。
ST29 は data-durability の 1 つを MODIFIED する。その「書き出しに入れる表のどの列も分類する」は main の正典にすでにある（`openspec/specs/data-durability/spec.md:371`）ので、ST49 の列の足し方（D17）はどちらの順でも同じ検査に掛かる。
ST23 は R5（削除の台帳の行と新しい操作の台帳）。ST23 が `ingest_request` に引数を足すことは D9 で畳み方が決まっている。
