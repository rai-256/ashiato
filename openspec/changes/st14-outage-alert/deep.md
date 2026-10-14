# ST14 の深掘り — 収集が途切れたら気づける

> 状態: **第 1 回の答えを記録した（2026-10-13。`deep-answers-1.txt`）。9 問すべてに本人が選んだ（推奨のままの問いは無い）。A の未回答は無い。**
> 問いは `deep-questions.json`（9 件。A 1 件 / B 8 件）、渡す形は `docs/briefs/ST14-deep.html`。
> 面は無い（`story_brief.py ST14 --screens` が空。通知は OS の通知で、稼働状況の画面に新しい置き場を作らない —— C13）ので proto は作らない。
> **★ 2026-10-13 訂正**: 答えを戻した後の整合の確認（`docs/audit/2026-10-13-st14-consistency.md` C7）で、ST14 は S-1 稼働状況を変える Story に移った（`ui-direction.md`）。下の「整合の確認から spec の段へ渡すもの」。
> 独立レビューは `review/deep.md`（8 件。A の問いを足した 1（R1）/ 問いの前提を直した 2（R3・R4）/ 問いを C へ移した 2（R4・R5）/ C を足した・直した 4（R2・R6・R7・R8））。

## この Story に宛てられたもの

`docs/handoff/ST14.md` と、`deferred ST14` の処置（st06 R31・st05 R24）。どれも ST14 を宛先にしているので、引き受けを既定にした（C10。R5）。

| 出所 | 何か | 行き先 |
|---|---|---|
| st06 R6 | NFR-13 の仕分けの訂正（アプリ利用は「利用が主語」）が正典と `DEVICE_SUBJECT` に入っていない（`crates/server/src/coverage.rs:28` に `c01-app-usage` が残る） | C10 |
| st06 R5 / R26 | 取得元に無かった期間（`kind: gap`。理由 2 つ）が、その日を①に塗る | Q7・C9 |
| st06 R7 | 集計（`c01-app-usage-rollup`）が格子に出ず、出すと開始日が 1〜2 年前 | Q8 |
| st05 R6 | PC の時計の測定記録（`c02-window` の `kind = clock-skew`）が、PC の動いていた日を①に塗る | C9 |
| st25 Q5 | 稼働状況の画面が OS の明暗に追従しない（NFR-17） | C10 |
| st06 R31 | アプリ利用の未送信の量を実機の直近 10 日で数え、上限の閾値を確定する | C10（確認バッチの手順。物理: device） |
| st05 R24 | `c01-clock` は生存信号を送らない。途切れの判定でどう扱うか | Q6 |
| st17 R4 | 本人が書くソース（`s01-feeling` / `s01-attribute`）を判定から外す | C1 |
| st07 R10（`docs/handoff/ST02.md`。spec の段で拾った —— spec-review R1） | PC の 2 ソースの分母を「生存信号が 1 件でも届いた日」にする（NFR-13 ★ 2026-09-12） | C10（design D17） |
| st07 R38（同上） | `c02-window` の `idle` / `powered-off` / `excluded` / `clock-skew` が①を立てる | C9（`clock-skew` は格子から外す）・design D22（仮）（残り 3 つは格子で数え続ける。R38 の「達成日数が動く」は NFR-13 ★ 2026-09-12 の後は当たらない） |
| ★ 2026-10-12（st17 の整合の確認 C7） | 除外を「本人が書いた（FR-25）記録のソース」と分類で決める（場所 `s01-place` も当たる） | C1（2026-10-13 の整合の確認 C3 で拾い直した。第 1 回の問いを立てたときに読み落とした） |

## 前提として確かめたこと（schema の手順 1〜5）

- **手順 1（要件どうしの衝突）**:
  - FR-35「最後の記録**または生存信号**」× FR-78「取得できる状態か」: 権限が剥がれても生存信号は届くので、逐語どおりなら鳴らない → **Q2**
  - FR-35 の Takeout 系 60 日 × 3 = 180 日 × EXT-E / FR-13（Chrome の履歴は 90 日）× EXT-B（予約は 1 年で止まる）→ **Q9**（R1）
  - FR-35 × FR-82 / NFR-13（PC を閉じた日）: ST07 の深掘り Q8 で本人が「許容する」を選んでいる → **C12**（R4）
  - FR-35 × FR-34（意図的な停止）/ FR-99（衛星）/ FR-36・FR-44（本人が書く）→ **C1・C2**
- **手順 2（扉 #14 の幅）**: 「最後の記録」に何を数えるか（観測でない記録 → **C9**。R2）、どの時刻で測るか → **Q3**（R3）
- **手順 3（新たに立つ一方通行）**:
  - 書庫の通知が取得元の保持より遅いと、取り逃した期間が取れなくなる（`uncaptured`）→ **Q9**
  - 外への push は文面を外に出す（`exported`）→ **Q1** の選択肢
  - 登録簿の「判定の対象」の列の既定が「対象外」だと、新しい収集のソースが黙って外れる → **C1**（R7）
- **手順 4（日常に影響する選択）**: 通知の経路（Q1）・回数（Q4）・まとめ方（Q5）・問い合わせの間隔（C11。R8）。
  電池: 問い合わせは生存信号（6 時間）と時計の測定（1 時間）に相乗りし、新しく起きる仕組みを足さない
- **手順 5（既存コードが要件を満たしていない箇所）**:
  - 途切れの判定と通知はどこにも無い。サーバの定期処理は書庫の取り込み器の loop（`crates/server/src/archive/worker.rs:1803-1876`）だけ
  - PC の収集アプリは OS の通知の口を持たない。携帯は常駐の通知と未送信の警告だけ（`LocationService.kt`）
  - `coverage.rs` は `event_time` / `emitted_at` で日を数え、`ingest_time` / `received_at` を使わない。重複した記録は行を作らない（届いた時刻も進まない）
  - `DEVICE_SUBJECT` が NFR-13 ★ 2026-09-18 と食い違う（C10）
  - `c02-window` に観測でない記録（`clock-skew` 毎時・`idle`・`powered-off`・`excluded`）が混ざる（`crates/collector-windows/src/contract.rs:30-46`）→ C9
  - 意図的な停止は表（`core.coverage_span`）だけがあり、書き込む口は無い（ST15）

## 聞かないで決めること（C）

design.md の D 番号に写す。**異論があれば、答えを貼り戻すときに番号で書いてもらう**（問いの HTML の冒頭に同じ一覧を載せた）。

| # | 何を | 決め方 | 理由 |
|---|---|---|---|
| C1 | 判定の対象 | 「収集した」ソース。登録簿に列を持ち、**既定は「対象」**。外すもの（本人が書いた FR-25 のソース —— `s01-feeling`・`s01-attribute`・`s01-place` など、登録簿の `origin = authored` で絞る / 導出 `s01-stay` / 衛星 FR-99 / `c01-clock`（Q6））に印を付ける（★ 2026-10-13 整合の確認 C3 で `s01-place` と分類を足した） | 既定は厳しい側。印を付け忘れた新しい収集のソースが黙って外れない（R7） |
| C2 | 鳴らさない期間 | 退役した日以降・意図的な停止の間・収集開始日の前。一度も届かないまま壊れている収集は鳴らない（格子の⑦で見える） | FR-35 / FR-80 と同じ除外 |
| C3 | 書庫と取り込み器 | 書庫の各ソース（`c03-*`）は記録で、取り込み器（`s01-archive-inbox`）は生存信号で測る（3 日で鳴る）。`c03-legacy-*` は取り込み器が退役させる | FR-78 ★ 2026-09-15 |
| C4 | 判定の置き場 | サーバが 1 か所で判定し、通知を出す側は結果を読むだけ | 判定が端末ごとに分かれない |
| C5 | 通知の文面 | ソースの表示名と最後に届いた日時だけ。記録の中身を載せない | ロック画面に出る |
| C6 | 通知の台帳 | 鳴らしたこと・戻ったことを追記のみの表に残す | 後から「いつ途切れ、いつ端末に返したか」を数えられる（★ 2026-10-15 言い直し。`delivered` は「サーバが端末に返した」で、「端末で出せた・気づけた」とは読まない。Q-R16） |
| C7 | サーバの起動直後 | 端末が溜めた分を送り直すまで判定を待つ | 起動のたびに鳴らない |
| C8 | 破棄の報告 | `/drops` では鳴らさない | 端末が上限の 7 日前に 1 回知らせる（FR-9） |
| C9 | 観測でない記録 | `c02-window` の `clock-skew` / `idle` / `powered-off` / `excluded` と、`c01-app-usage` の `gap` は「最後の記録」に数えない。格子の日の状態の数えからも `clock-skew` と `gap` を除く | 毎時の時計の測定が、窓が読めなくなった PC を「生きている」にし続ける（R2）。申し送り st05 R6・R5 |
| C10 | 申し送りの引き受け | 上の表の全部をこの Story で持つ。R31 は実機を USB で 1 回繋ぐ物理の作業なので確認バッチの手順に入れ、上限を超えていれば種別を絞るかを改めて聞く | 宛先が既に ST14（R5）。R31 は測るまでの長い圏外で上限に捨てられうる（discarded）ので先延ばしにしない |
| C11 | 問い合わせの間隔 | 携帯は生存信号と同じ 6 時間、PC は時計の測定と同じ 1 時間に相乗り | 新しく起きる仕組みを足さない。携帯で鳴るのは止まってから 18〜24 時間後（R8） |
| C12 | PC を閉じていた時間 | 経過時間に数える（要件は変えない） | ST07 の深掘り Q8 で本人が「許容する」を選んだ（R4）。サーバが同じ PC なら閉じている間は判定が動かず、開いた後は C7 の間に記録が届く |
| C13 | 画面 | 稼働状況の画面に新しい置き場（帯・箱）を作らない。途切れは格子の⑥で見える | 画面の構造を変えない（R6） |

## 問うた項目

第 1 回（`deep-questions.json`）。答えは下の「第 1 回の答え」。

| # | 段 | 論点 | 推奨 |
|---|---|---|---|
| Q1 | B | 通知をどこに出すか | 携帯と PC の両方の収集アプリ |
| Q2 | B | 取れない状態の生存信号を生きていると数えるか | 数えない（FR-35 に ★） |
| Q3 | B | 経過時間をどの時刻から測るか（R3） | 端末で作られた時刻。書庫はそのソースを運んだ最後の書庫 |
| Q4 | B | 何回鳴らすか・戻ったら | 音は 1 回、続く間は音の無い通知、戻ったら消す |
| Q5 | B | 同時に途切れた複数のソース | 1 通にまとめる |
| Q6 | B | `c01-clock` | 入れない |
| Q7 | B | 取得元に無かった期間の格子での見え方 | ③にする。詳細に理由を分けて出す |
| Q8 | B | 集計を格子に出すか | 出す。開始日は `c01-app-usage` にそろえる（FR-79 に但し書き） |
| Q9 | **A**（uncaptured。R1） | 書庫のソースを何日で知らせるか | 最後の書庫から 75 日（FR-35 に ★） |

## 第 1 回の答え（2026-10-13。`deep-answers-1.txt`）

9 問とも、本人が選択肢を選んだ（`(推奨のまま: …)` の行は無い）。選んだものはどれも推奨と同じ。補足は無い。
C1〜C13 への異論は書かれていない（C は既定のまま design.md の D 番号に写す）。

| # | 段 | 論点 | 本人の答え（逐語） | 効く先 |
|---|---|---|---|---|
| Q1 | B | 通知をどこに出すか | 携帯と PC の両方の収集アプリが、サーバに問い合わせて OS の通知を出す | FR-35 ★。PC の収集アプリに Windows の通知の口を作る。外の push は作らない（exported を開けない） |
| Q2 | B | 「取れない状態」の生存信号を生きていると数えるか | 知らせる —— 取得できる状態の生存信号だけを「生きている」と数える | FR-35 ★。判定式 |
| Q3 | B | 経過時間をどの時刻から測るか | 携帯・PC のソースは端末で作られた時刻（格子と同じ）。書庫のソースは、そのソースを運んだ最後の書庫の時刻（台帳） | FR-35 ★。書庫は `archive_ledger_source` を読む |
| Q4 | B | 何回鳴らすか・戻ったら | 途切れ 1 回につき音は 1 回。続く間は音の無い通知を残し、戻ったら黙って消す | FR-35 ★。通知を出す側の振る舞い |
| Q5 | B | 同時に途切れた複数のソース | 1 通にまとめ、途切れたソースの名前を並べる（新しく途切れたソースが増えたときだけ鳴る） | FR-35 ★。通知を出す側の振る舞い |
| Q6 | B | 携帯の時計の測定（c01-clock） | 入れない（携帯の稼働は位置で見る） | 登録簿で `c01-clock` を判定の対象から外す印（C1 の列）。要件は変えない |
| Q7 | B | 「取得元に残っていなかった期間」の格子での見え方 | その期間の日を③「動いていたが取れない状態だった」にする。日の詳細に理由（2 つを分けて）と期間を出す | FR-54 ★。`coverage.rs` の日の状態の数え（C9 と併せて） |
| Q8 | B | アプリ利用の集計を格子に出すか | 出す。開始日は同じ携帯のアプリ利用（c01-app-usage）の開始日にそろえる | FR-79 ★。重複の到着での当て直し（`touch_started_on`）も同じ規則に |
| Q9 | **A**（uncaptured） | 書庫のソースを何日で知らせるか | 最後の書庫から 75 日で鳴らす（書庫のソースだけ。登録簿に通知までの日数を持つ） | FR-35 ★。登録簿に列を足す。Story の完了の判定に足す |

## 要件へ戻すもの

| 要件 | 何を戻すか | 元になった答え |
|---|---|---|
| **FR-35** | 生存信号は「取得できる状態」のものだけを数える。経過時間は端末のソースは端末で作られた時刻、書庫のソースは「そのソースを運んだ最後の書庫」の時刻で測る。書庫のソースは最後の書庫から 75 日で通知する（登録簿にソースごとの日数）。通知は携帯と PC の両方の収集アプリが、サーバに問い合わせて OS の通知で出す。音は途切れ 1 回につき 1 回、戻ったら黙って消す、同時の途切れは 1 通にまとめる | Q1 / Q2 / Q3 / Q4 / Q5 / Q9 |
| **FR-54** | 取得元に残っていなかった期間（FR-85）の日は③「動いていたが取れない状態だった」。週の文字に理由を分けて出す。観測ではない記録（期間の主張・時計の測定）は①に数えない | Q7 / C9 |
| **FR-79** | アプリ利用の集計の収集開始日は、同じ携帯のアプリ利用の収集開始日にそろえる（当て直しも同じ規則） | Q8 |

Story（`stories.json` の ST14 の `done`）に 2 つ足した: 「書庫のソースは、最後の書庫から 75 日を過ぎると通知が来る」（Q9）と
「生存信号が『取得できない状態』のまま想定間隔の 3 倍を過ぎると通知が来る」（Q2）。表題・価値・`satisfies` は変えない。
既存の「Takeout 系のソースは、正常な 2 か月周期では通知が来ない」は 75 日でも成り立つ（60 日 < 75 日）。

## 当初案を覆したもの

無い（9 問とも推奨を選んだ）。要件の逐語（FR-35 の「最後の生存信号」「想定間隔の 3 倍」、FR-79 の「いちばん古い記録の日」）から
変わった 3 点は、問いを立てた時点で推奨にしていたもので、上の「要件へ戻すもの」が受ける。

## 整合の確認から spec の段へ渡すもの（2026-10-13。`docs/audit/2026-10-13-st14-consistency.md`）

答えを要件へ戻した後に consistency-review を回した（16 件）。要件で直したもの（C3 / C4 / C5 / C8 / C9 / C10 / C11 / C12 / C13 / C15）は
FR-35 / FR-9 / FR-54 の ★ と EXT-E の影響に入れた。次の `spec` が受けるもの:

- **C1（ST44）**: 判定の口は収集アプリの資格情報で読める口にする。収集アプリの資格情報で呼んで 200 を見る結合テストを ST14 にも置く。申し送りは `docs/handoff/ST44.md`
- **C2（capability）**: spec は `collection-coverage` に閉じる（INDEX:84 の担当に「通知」がある）。収集アプリの通知のコードは ST08（`crates/collector-windows`）・ST17（`collector-android`）と同じ crate / module に当たりうるので、proposal の Impact に書く
- **C5（書庫の 60 日と 75 日）**: 正典 `external-ingestion` の「60 日を過ぎたことを知らせるのは ST14」（:697）と :588 は、通知の日数が 75 日になったことと食い違う。ST47 が `external-ingestion` を走らせている間は MODIFIED にできないので、`external-ingestion` を MODIFIED にするか、`collection-coverage` の側で「通知は 75 日・格子は 60 日」と書いて正典の文を後の Story に申し送るかを spec で決める
- **C6（正典の評価の順）**: `collection-coverage` の日の状態の (5)「記録が 1 件でもあれば①」は (7) より先に評価される。Q7 / C9 のために、観測ではない記録を除いた数えと評価の順を MODIFIED にする。開始日の遡りの Requirement にも集計の例外（Q8）を MODIFIED で入れる
- **C7（画面）**: ST14 は S-1 を変える（Q7 の週の文字・Q8 の集計の行・NFR-17）。**構造は変えない**（行が 1 本増え、文字が変わり、明暗が追従するだけ）ので問いにせず、集計の行の置き場（`c01-app-usage` の隣か、並びの末尾か）と週の文字の文言は `design.md` の D（仮）にする。
  画面の Scenario は `web/e2e` に置き、`reading` の段で `proto.html`（GO view）を作る
- **C14（取得可否が通知の引き金になる）**: Q2 の後、ST08 のブラウザ履歴（プロファイルが 1 つ読めないと「取得できない」）や
  ウィンドウの URL の経路が応答しないだけで鳴る。通知に使う「取得できない」を blockers の種類で絞るかを `design.md` の D（仮）にし、反転条件を書く（ST08 には差し戻さない）
- **未確認（spec / tasks で確かめる）**: 形の確認を待つ書庫（`pending_shape`）が `archive_ledger_source` に行を持つか（持たないなら「最後の書庫」が進まず 75 日で鳴る）/
  Takeout の Chrome の履歴の実際の保持（本物の書庫で測る。FR-35 ★ の C8）/ ST17 の通知と同じ Android のファイルに当たるか / C-02 が未パッケージの exe のまま Windows の通知を出せるか（AUMID）

## 下流 Task 1 で積んだ A（premise: 検証コマンドが成立しない。2026-10-14）

### Q-T1 [Task 1 の検証コマンド 3 本が書かれたままでは原理的に通らない] -> 承認する

- **答え（2026-10-14。`deep-answers-3.txt`。本人の選択。推奨と同じ）**: 承認する
  - 補足（逐語）: quiet-run の件（Q-T1 の Task 2〜7 と Q-T11）は harness2 #73 で解消済み
- **当てたもの**: 1.1 だけを直した（`outage_target` の否定を外す。`plan_fix.py` は新しい入口が `bash -c` で通らないので、9.3 と同じく手で直し `plan-corrections.md` に記録）。
  直した後の `scripts/verify-run 1.1` は PASS。
- **直さないもの**: 1.2 / 1.3 と、下の Task 2〜7 の `CT` コマンド。harness2 #73（`verify-run` が検証コマンドの `quiet-run` の包みを外して走らせる）で、書かれたままのコマンドが成立するようになった（本人の補足）。tasks.md は変えない。


- **1.2 / 1.3**: `scripts/quiet-run ct -- cargo test … | tee /tmp/ct.log` は成功時に末尾 5 行しか返さず、残るのは doc-test の `0 passed; N filtered out` だけ。`grep 'test result: ok. [1-9]… passed'` が通らない。試験は 8 本（1.3 は 1 本）通っている（rc=0・ログ `.superpowers/sdd/logs/*-ct.log`）。FAIL の証跡は evidence.jsonl。提案: `quiet-run ct --` を外し、ST17 と同じ `cargo test -p ashiato-server <名前> 2>&1 | tee /tmp/ct.log`。
- **1.1**: 着手前の確認（`! grep -rq outage_target migrations/`）で、移行を書いた後は必ず FAIL。提案: 検証を「書く前に成り立っていた」前提の確認ではなく、`bash -c 'grep -q "DEVICE_SUBJECT" crates/server/src/coverage.rs && grep -q "archive_ledger_source" migrations/202609181600_archive_ingestion.sql'` に直す（`outage_target` の否定を外す）。
- **Task 2 の 2.1 / 2.2 / 2.3 の `CT` コマンドも同じ欠陥**（`quiet-run ct --` を `tee` に通す形。試験は `st14_grid_` 13 本・`coverage::tests::` 81 本が通っている）。提案は Q-T1 と同じ: `quiet-run ct --` を外した形を正式な入口にする（`plan_fix.py` で 2.1 / 2.2 / 2.3 の旧コマンドを直す）。FAIL の実証は evidence.jsonl（head f23f611d）。
- **Task 3 の 3.1 / 3.2 / 3.3 の `CT` コマンドも同じ欠陥**（試験は `st14_rollup_started` 4 本・`st14_rollup_order` 2 本・`coverage::tests::` 87 本・`api_tests` 26 本が `quiet-run` を外した `cargo test` で通っている）。提案は Q-T1 と同じ。FAIL の実証は evidence.jsonl。
- **Task 4 の 4.1 / 4.2 の `CT` コマンドも同じ欠陥**（試験は `st14_ach_` 5 本・`coverage::tests::` 92 本・`api_tests` 26 本が `quiet-run` を外した `cargo test` で通っている。vitest は `npm ci` の後 342 本通過）。提案は Q-T1 と同じ。FAIL の実証は evidence.jsonl。
- **Task 5 の 5.1 / 5.2 の `CT` コマンドも同じ欠陥**（試験は `st14_judge_` 18 本・`st14_inputs_device` 13 本・`st14_inputs_nonobs_registry` 1 本が `quiet-run` を外した `cargo test` で通っている）。提案は Q-T1 と同じ。FAIL の実証は evidence.jsonl。
- **Task 6 の 6.1 / 6.2 の `CT` コマンドも同じ欠陥**（試験は `st14_judge_archive` 4 本・`st14_inputs_archive` 10 本（`st14_inputs_archive_grid` 1 本を含む）が `quiet-run` を外した `cargo test` で通っている）。提案は Q-T1 と同じ。FAIL の実証は evidence.jsonl。
- **Task 7 の 7.1 / 7.2 の `CT` コマンドも同じ欠陥**（試験は `st14_api_outages_` 10 本・`st14_ledger_` 8 本が `quiet-run` を外した `cargo test` で通っている。`tools/check-openapi.sh` は PASS）。提案は Q-T1 と同じ。FAIL の実証は evidence.jsonl。

## 下流 Task 10 で積んだ A（premise: 10.1 の検証コマンドが成立しない。2026-10-14）

### Q-T10 [10.1 の検証コマンドは書かれたままでは終わらない] -> 1 コマンドの入口に直す

- **答え（2026-10-14。`deep-answers-4.txt`。本人の選択。推奨と同じ。補足なし）**: 1 コマンドの入口に直す
- **再回答（2026-10-15。`deep-answers-5.txt`。本人の選択。補足なし）**: 1 コマンドの入口に直す —— `deep-answers-4.txt` と同じ答え。当て済みなので、新たに当てたものは無い。
  この回で聞いていたのは Q-R16（`deep-questions-r3.json`）だったが、返ってきたのは Q-T10（r2）の答えだけだった。Q-R16 はその時点では未回答のまま残した（答えは答え 6）
- **再々回答（2026-10-15。`deep-answers-6.txt`。Q-R16 と一緒に返ってきた）**: 1 コマンドの入口に直す —— 答え 4・5 と同じ。当て済み
- **当てたもの**: 10.1 の検証コマンドだけを直した。スタックを裏で起こし（setsid）→ 「画面: http」の行を待つ → `tools/ports.sh` の `ASHIATO_HTTP_PORT` を curl → プロセスグループごと止める、までを 1 本の `bash -c` にし、その後に同じ `jq -e`。受け入れ条件（jq の判定）は変えない。提案の形に、`tools/ports.sh` の source（`ASHIATO_HTTP_PORT` は stack.sh の中でしか決まらない）と、スタックが落ちたら待たずに抜ける `kill -0` を足した。`plan_fix.py` は新しい入口が `bash -c` で通らないので、1.1 / 9.3 と同じく手で直し `plan-corrections.md` に記録。
- **併せて外したもの**: 10.1 の本文の `（… ポートは \`tools/stack.sh\` が出す値に読み替える。spec-review R13）` の backtick。`verify-run` は「検証:」の後ろの backtick をすべて検証コマンドとして拾うので、`tools/stack.sh`（= `up`、前景で待つ）も走らせていた。9 分以上返らなかった原因の 1 つ。
- **確かめたこと**: 直した後の `scripts/verify-run 10.1` は PASS（rc=0・28 秒。スタックを作り直して起動し、jq の判定は `true`、止めた後に 19028 の待ち受けは残らない）。evidence.jsonl にはその前に **3 秒の PASS が 1 行ある（無効）**: 1 回目の verify-run が R13 の `tools/stack.sh` を拾って起こしたスタックが残っていて、新しいスタックは「19028 は使用中」で落ち、curl は残っていたサーバを読んだ。残りを止めてから取り直したのが 28 秒の PASS。


- **原因**: `tools/stack.sh up >/dev/null && curl …` の `stack.sh up` は前景で待つ（`wait`。Ctrl-C で止まる）ので終了せず、`&&` の後ろの `curl` に着かない。`scripts/verify-run 10.1` は 9 分以上返らず、私が止めた（`evidence.py` に timeout は無い）ので FAIL の証跡は残せていない。
  もう 1 つ: 読み先の既定が `${ASHIATO_PORT:-18787}`。`ASHIATO_PORT` という変数は無く（`tools/ports.sh` が出すのは `ASHIATO_HTTP_PORT`。この worktree は 19028）、既定の 18787 は main の本番のサーバ。
- **成り立つことの確認（手で）**: `.env` を読んで `STACK_RESET=1 tools/stack.sh up` を別のプロセスグループで起こし、`curl http://127.0.0.1:19028/coverage?from=2026-08-01&to=2026-09-30` → 検証コマンドの `jq -e …` は `true`（rc=0）。アプリ利用 `2026-09-01〜04` の 4 日が `alive_not_capturable` で gaps を持ち、集計は 1 本（開始日 2026-08-31）。
- **提案する正式な入口**: スタックを起こし・読み・止めるまでを 1 コマンドにする（例: `bash -c 'set -a; . ./.env; set +a; (STACK_RESET=1 setsid tools/stack.sh up >/tmp/stack.log 2>&1 & echo $! >/tmp/stack.pid); for i in $(seq 1 180); do grep -q "^画面: http" /tmp/stack.log && break; sleep 2; done; curl -sf -H "authorization: Bearer ${API_TOKEN}" "http://127.0.0.1:${ASHIATO_HTTP_PORT}/coverage?from=2026-08-01&to=2026-09-30" > /tmp/cov.json; rc=$?; kill -- -$(cat /tmp/stack.pid); exit $rc'` の後に同じ `jq -e …`）。または e2e が偽データを見る形（Task 10.2〜10.4 の `ET`）で代える。


## 下流 Task 11 で積んだ A（premise: 11.3 の `cargo test --workspace` を `quiet-run` に包んだ検証コマンドが PASS にならない。2026-10-14）

### Q-T11 [11.3 の検証コマンドは試験が通っても FAIL になる] -> 解消（訂正しない）

- **答え（2026-10-14。`deep-answers-3.txt` の Q-T1 の補足、逐語）**: quiet-run の件（Q-T1 の Task 2〜7 と Q-T11）は harness2 #73 で解消済み
- 11.3 の検証コマンドは変えない。`verify-run` が `quiet-run` の包みを外して走らせるので、書かれたままで成立する。


- **原因**: Q-T1 と同じ型。`scripts/quiet-run all -- cargo test --workspace` は末尾しかログに出さず、`evidence.py` の `ran_no_tests` が見る「passed の和」が末尾の doc-test の `0 passed` だけになり、rc=0 でも `FAIL（試験が 1 本も走っていない）`になる。
  実際の全文（`.superpowers/sdd/logs/20261014-190528-all.log`）では `151 + 891 + 7 passed; 0 failed`、fmt・clippy も rc=0。FAIL の実証は evidence.jsonl の 11.3（`status: FAIL, rc=0`）。
- **提案する正式な入口**: `quiet-run` を外した `cargo test --workspace`（Q-T1 の提案と同じ）。
- 11.1（smoke）・11.2（check_scenarios）・11.3 の `check-openapi` / `check-migrations` / `check-immutable` / `check-log-private` は PASS。

## 下流 finish で積んだ A（premise: C6 の理由と台帳の `delivered` が合っていない。2026-10-15。review/code.md R16）

### Q-R16 [台帳の delivered は何を残すか（C6 の理由）] -> C6 の理由を言い直す（返したことだけ残す）

- **答え（2026-10-15。`deep-answers-6.txt`。本人の選択。推奨と同じ。補足なし）**: C6 の理由を言い直す（返したことだけ残す）
- **当てたもの**: 上の C の表の C6 の理由を言い直した（「いつ気づけたか」→「いつ途切れ、いつ端末に返したか」）。`delivered` は「サーバが端末に返した」と読み、「気づけた」とは読まない。**コードは変えない**（spec の Scenario は既に「返した」と書いていて、実装はそれと合っている）
- **受け入れたこと（不可逆）**: 端末で通知を出せたかどうかは、これから先も台帳に残らない（`uncaptured`）。要るなら `shown` を返す口を別の Story か `fix/` で足す（台帳は追記のみなので、足した時点から先の分だけ）
- **次の段へ渡すもの**: 同じ理由の言い回しが残っている 2 か所 —— `specs/collection-coverage/spec.md:362` の本文（「後から『いつ途切れ、いつ気づけたか』を数えるため（C6）」）と `go.json:469`（「後から『いつ途切れ、いつ気づけたか』を数えるため」）。この段は仕様を書き換えないので、spec / reading の段が「いつ端末に返したか」へ言い直す。Requirement の SHALL と Scenario は変わらない
- 要件へ戻すもの: 無し（`docs/requirements.md` に「気づけた」の言い回しは無い。FR-35 の本文は台帳の理由を書いていない）


- **loss**: `uncaptured` —— 端末が通知を出せたかどうか。台帳は追記のみなので、残すと決めるまでの分は後から作れない
- **事実**: サーバは応答を返す前の取引で `delivered` を書く（`crates/server/src/outage_store.rs` の `judge_and_record`。`st14_ledger_delivered_per_device`）。
  携帯は問い合わせの後に通知を出し、権限が無ければ出さない（`OutageNotifierTest` の「通知を出せなくても収集は止まらない」）。端末が 10 秒で打ち切ったときも同じ。
  このとき台帳は「端末に届けた」、端末には何も出ていない
- **崩れた前提**: C6「鳴らしたこと・戻ったことを追記のみの表に残す —— 後から『いつ気づけたか』を数えられる」。`delivered` は「返した」で、「気づけた」の材料にならない。
  spec の Scenario は「返した」と書いているので、実装は spec とは合っている
- **選択肢**: (1) C6 の理由を言い直す（`delivered` は「返した」。コードは変えない。推奨）/ (2) 端末が「出せた」を返す口（`shown`）を足す（API と 2 つの収集アプリ。別の Story か fix/）
- 問いの HTML: `docs/briefs/ST14-deep-r3.html`（`deep-questions-r3.json`）
