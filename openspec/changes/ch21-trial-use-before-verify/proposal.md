# CH21 — 確認バッチの前に、その回で足した機能だけを使ってみる席（/tsukau）で軽く確かめる

> 通常の路（`lane-verdict.json` の `lane: normal`。外へ出るもの（Sandbox の網）と範囲の決め方で、決めることが残っていた）。
> 深掘り第 1 回（Q1〜Q8）は推奨の答えで先に書いている（`deep.md` の「答え待ち」）。Q2（Sandbox の網）は本人が答えを省けない。
> spec-review（review/spec.md R1）を受けて、Q2 の推奨を「網を切る」に移した（外につなぐ案は、CH22 が書き直す正典 distribution の約束に 3 つ目の例外を足すことになるため）。

## Why

2026-11-19、本人が配る形を自分の PC に入れて普通に使ったら、更新の確認がエラーを出し続けることがすぐ見つかった（#329）。
今の試験はどれも spec の Scenario を 1 つずつ確かめるだけで、「入れて普通に使うとどう見えるか」を見る席が無く、本人が最初の利用者になっている。
席そのもの（AI が利用者として筋書きなしで触り、違和感の候補を根拠つきで残す）は harness2 の `/tsukau`（`scripts/tsukau.py`。harness2 #272 / PR #276）に置いた。
ashiato2 には、その席が本物を立てる**差し込み口**が無く、確認バッチの準備からも呼ばれていない。

## What Changes

- **差し込み口を置く**: `tools/tsukau/tsukau.json` と env_hook（`tools/tsukau/env-hook.sh` の `up` / `down`）。相手は画面（`web`）と PC（`pc`）。
  携帯は持たない（後で `targets` に足せる形。依頼の「どこまで」）。手で呼ぶ `/tsukau` も同じ差し込み口と同じ置き場を使う（design D1）
- **画面の相手**: 専用の compose project・空いている port・乱数の合言葉・偽データで立てる（録画の専用環境と同じ。本人の `.env` と確認バッチの DB を使わない。design D7）
- **PC の相手**: 網を切ったまっさらな Windows Sandbox に配る形のサーバの zip を入れ、窓を開いてから配る形の PC の収集アプリを利用者のセッションで起こす（深掘り Q2・Q4・design D3・D4・D6）。起こす前に別の Sandbox が動いていれば立てない。片付けは自分が起こした Sandbox だけを止め、止める前に中のログを実行のフォルダへ写す（design D3）
- **確認バッチの準備から呼ぶ**: `tools/verify-prep.sh` の最後に `tools/tsukau/verify-batch.sh <tag>` を呼び、その回に入った Story / change が変えたファイルで相手を決め、
  表題と「何のためか」の 1 行を範囲にして（Q1・design D2）、相手ごとに 10 分・20 万 token で 1 回呼ぶ（Q5・design D8）。席は確認バッチの作業ツリーでなく、
  使い捨ての作業ツリーで走らせる（design D9）。PC の配る zip は、席を呼ぶ前に確認バッチの木から手元で組む（Q3・design D5）
- **同じ中身なら前の結果を写す**（Q6・design D10）
- **手順書と PR 本文に写すのは件数・題・止まり方だけ**（依頼）。回らなかったときは止まり方ごとの決まった文言だけを書く（Q7・design D11）。
  題は写す前に、利用者名・ホスト名・私設網の名前・`.env` の値・絶対パスの形と、確認バッチの commit の検査が止める形を機械で伏せ、1 行に整える（design D12（仮）・D15（仮））
- **席が落ちても、時間切れでも、準備は止めない**。今の準備の成果物（サーバ・画面・APK・`run.sh`・手順書の既存の段落）と準備の成否を変えない（依頼・Q8）
- 台本の自己試験を CI の `rust` の job に足す（design D13）
- CH21 だけの確認バッチでは席が回らないので、実地の 1 回は同じ台本を使い捨ての確認バッチの木で本物に通す（Q8・design D14（仮））
- 席の中身（触り方・違和感の候補の書き方・止まり方）は harness2 側。ここでは変えない（依頼）。以前の CH21（ashiato2 だけで席を作る形・Scenario 39 本）は捨てた

## Capabilities

### New Capabilities

なし

### Modified Capabilities

- `verify-environment`: Requirement を 9 つ ADDED（差し込み口・範囲と相手の決め方・準備からの呼び方・手順書へ写すもの・題の伏せ方・止めないこと・同じ中身の写し・PC の相手の立て方・画面の相手の立て方）。
  確認バッチの準備の約束（「確認バッチの準備は、エミュレータにだけ携帯の APK を入れる」）が既にこの capability にあるので、同じ所に置く。既存の Requirement は変えない

## Impact

- 触れる予定のパス: `tools/tsukau/`（新しい。`tsukau.json`・`env-hook.sh`・`verify-batch.sh`・`build-pc.sh`・`pc.sh`・`real-run.sh` と Sandbox の中で走らせる PowerShell）・
  `tools/verify-prep.sh`（最後に 1 か所呼ぶだけ）・`.github/workflows/ci.yml`（`rust` の job の `check-private.sh --self-test` の後に自己試験を 2 行）・`docs/testing.md`・`docs/changes/CH21.md`・この change
- 触れないもの: 製品のコード（`crates/`・`collector-android/`・`web/`・`migrations/`）・harness2（席の中身・`scripts/tsukau.py`・`verify_batch.sh`）・
  `tools/record-env.sh`・`tools/recording/`（読んで立て方を写すだけ）・`tools/release/`（`package.sh windows` を呼ぶだけ）
- **確認バッチの準備が延びる**: 相手ごとに、PC の zip の組み立て（未測。Task 1 で測る）＋立てる（上限 15 分）＋席（10 分）＋片付け（上限 5 分）。
  その間、確認バッチの作業ツリーの錠を握り続け、記録と fix の撮り直しが待たされる（harness2 `graph/harness_graph/verify.py:108`）
- **外へ出るもの**（Q2 の答えに依る）: 推奨の形（網を切る）では何も出ない。本人が外につなぐ案を選んだら、PC の相手を立てるたびに配布元への問い合わせと席が開く外のサイトへの接続が出て、正典 distribution の約束を改める（CH22 の後に積む）
- **本人の画面**: PC を変えた回ごとに、Sandbox の窓が本人のデスクトップに出る（Q4。1 日 1〜5 回まで）
- **走っている change との重なり**: CH22 は `.github/workflows/ci.yml` の `rust` の job の `update-apply-install.sh --self-test` の直後に行を足すので、CH21 は別の位置（`check-private.sh --self-test` の後）に足す（review/spec.md R20）。
  CH23 は `docs/testing.md` の毎晩の行に書き足し、CH21 は同じファイルの別の場所に 1 節を足す。CH19（`tools/android-emulator.sh`）とは重ならない。
  CH22 は正典 distribution の「自動の試験は本物の配布元に問い合わせない」を書き直しており、Q2 で外につなぐ案が選ばれたときだけ CH21 がその上に積む（requires に CH22）。録画（harness2 `record-run`）とは Sandbox を取り合う（design D3。席は録画より先に終わる並び）

## 持ち越し

- 携帯（`android`）の相手は足さない（依頼。`targets` に足せる形にしておく）
- 席が見つけた違和感の候補を直すこと・issue にすることはしない（依頼。候補をどうするかは本人が決める）
- 確認バッチに後から足す、すでに main の Story（`--story`）は木の差に出ないので範囲に入らない（Q1 の context。要るようになったら別の change）
