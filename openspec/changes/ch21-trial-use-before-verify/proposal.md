# CH21 — 確認バッチの前に、その回で足した機能を使ってみる席（/tsukau）で確かめ、違和感を直すか人間に回す

> 通常の路（`lane-verdict.json` の `lane: normal`）。第 3 版（2026-11-20）: 依頼に「見つけた後の処置」が足され、深掘りから立て直した。
> 深掘り第 1 回（Q1〜Q13）は推奨の答えで先に書き、本人の答え（2026-11-20）で推奨と違った Q8（外にはつなぎ、私設網と手元の網を塞ぐ）・Q11（新しく入ったものだけを範囲にする）を写した。
> 仕様への合意で返った違和感「スマホも操作するようにしてください。エミュレータとかで」で、携帯（エミュレータ）の相手を足した（依頼の「携帯は外」を変える）。
> 第 2 回の合意で IMP-13（席はホストの adb で本人の実機に届きうる）が「仕様を直す」で返った（補足: 席を呼ぶ間は adb を席の PATH から外し、phone.sh だけが使えるようにする）。この版（2026-11-21、`spec --until proposal`）は proposal に写した。specs・design・tasks への写しは、先行の CH22 が gate を通った後の spec の段で行う。

## Why

2026-11-19、本人が配る形を自分の PC に入れて普通に使ったら、更新の確認がエラーを出し続けることがすぐ見つかった（#329）。
今の試験はどれも spec の Scenario を 1 つずつ確かめるだけで、「入れて普通に使うとどう見えるか」を見る席が無く、本人が最初の利用者になっている。
席そのもの（AI が利用者として筋書きなしで触り、違和感の候補を根拠つきで残す）は harness2 の `/tsukau`（`scripts/tsukau.py`。harness2 #272 / PR #276）にある。
本人がやりたいのは違和感を見つけて**直すか人間に回す**ことで、手順書に件数を載せるだけでは足りない（2026-11-20 の本人の指摘）。

## What Changes

- **差し込み口を置く**: `tools/tsukau/tsukau.json` と env_hook（`tools/tsukau/env-hook.sh` の `up` / `down`）。相手は画面（`web`）・PC（`pc`）・携帯（`android`。仕様への違和感で足した）。手で呼ぶ `/tsukau` も同じ差し込み口と置き場を使い、結果を残すだけ（Q5・design D1・D17）
- **画面の相手**: 専用の compose project・空いている port・乱数の合言葉・偽データで立てる（本人の `.env` と確認バッチの DB を使わない。design D7）
- **PC の相手**: まっさらな Windows Sandbox を網につないで起こし、何かを入れる前に中のファイアウォールで私設網と手元の網を塞いで確かめ（届けば外も塞ぎ、それでも届けば立てない）、配る形のサーバの zip を入れ、窓を開いてから配る形の PC の収集アプリを利用者のセッションで起こす（Q7・Q8 の本人の答え・Q9・design D3〜D6）
- **携帯の相手**: 専用の AVD のエミュレータを画面なし・まっさらで起こし、網を切って確かめ、その回の専用の環境に `adb reverse` で送る debug の APK を入れる。自分の serial だけを操作し、実機・本人や計測テストのエミュレータには触らない（仕様への違和感・design D22（仮）〜D25）
- **席から adb を隠す**（IMP-13 の本人の答え）: 席（claude）を起こすときの `PATH` から Android の道具（`platform-tools` と `emulator`）の置き場を外し、席が `adb` を名前で呼べないようにする。携帯を触る口は `tools/tsukau/phone.sh` だけにし、`phone.sh` と差し込み口（`env-hook.sh`）は `adb` を `$ANDROID_HOME` からの絶対パスで呼んで、自分の serial だけを名指す。確認バッチの準備からの呼び方（`verify-batch.sh`）はこの repo で外す。手で呼ぶ `/tsukau` は呼ぶ人の環境をそのまま席に渡すので、席の環境から外す口を harness2 への依頼に足す（どちらの形かは spec の段で design に決める）。`PATH` から外しても、席が絶対パスで `adb` を呼ぶことは止められない（名前で見つからなくするだけ）。その残りは GO 資料の IMP-13 に書き直す
- **確認バッチの準備から呼ぶ**: `tools/verify-prep.sh` の最後に `tools/tsukau/verify-batch.sh <tag>` を呼び、その回に入った Story / change が変えたファイルで相手を決め、表題と「何のためか」を範囲にして（Q6・design D2。携帯の収集アプリを変えたものは携帯の相手）、相手ごとに 10 分・20 万 token で 1 回呼ぶ（Q10・design D8）。席は使い捨ての作業ツリーで走らせる（design D9）
- **作り直したときは、新しく入ったものだけを範囲にする**（Q11 の本人の答え・design D10・D21（仮））: 相手・ID・PR のいちばん新しいコミットの組で、席が終えた記録のあるものは外し、段落に「前の回で試した」と並べる。前の回の結果は写さない
- **処置へ渡す目録**: `dist/verify-<tag>/tsukau.json`（手元だけ）に、相手ごとの状態・範囲・実行のフォルダを書く（Q1・design D14）。直した後の確かめ直しの口 `TSUKAU_RECHECK`（直した候補だけ・5 分・10 万。Q10・design D27）
- **手順書と PR 本文に写すのは止まり方・件数・番号つきの伏せた題だけ**（Q4・Q12・design D11）。公開に出す題は `tools/tsukau/public-title.sh` 1 か所で伏せ、処置（harness2）も同じものを通す（design D12）
- **見つけた後の処置は harness2 の確認バッチへの依頼にする**（Q1〜Q4・design D15）: 別の席の選別 → この回が原因なら本人が見る前に Story のブランチで直して確かめ直す（3 件まで・1 回まで）／判断が要れば手順書の問い／前からなら違和感の issue。合意の後に依頼を置く
- **席が落ちても、時間切れでも、準備は止めない**。今の準備の成果物と準備の成否を変えない
- 台本の自己試験を CI の `rust` の job に足す（design D13）
- **完了**は、harness2 の処置が入った後に、使い捨ての確認バッチの木で処置まで本物で通す（Q13・design D18。push 先は一時の bare repo、`gh` は偽物で、公開には何も出さない）
- 席の中身（触り方・候補の書き方）は harness2 側で、ここでは変えない（依頼）。以前の CH21（ashiato2 だけで席を作る形）と第 2 版（差し込み口と手順書への記載だけ）は捨てた

## Capabilities

### New Capabilities

なし

### Modified Capabilities

- `verify-environment`: Requirement を 12 ADDED（差し込み口・範囲と相手・準備からの呼び方・処置へ渡す目録・手順書へ写すもの・公開の題・止めないこと・新しく入ったものだけの範囲・確かめ直し・PC の相手・携帯の相手・画面の相手）と、処置まで通ることの受け入れの Requirement を 1 つ。
  確認バッチの準備の約束（「確認バッチの準備は、エミュレータにだけ携帯の APK を入れる」）が既にこの capability にあるので、同じ所に置く。既存の Requirement は変えない
- `distribution`: CH22 が書き直す「更新を確かめる問い合わせは、版の一覧を取る決まった形だけを配布元へ送る」を MODIFIED で、自動の試験が本物の配布元へ繋ぐ名指しに 3 つ目（使ってみる席の PC の相手）を足す（Q8 の本人の答え。design D6）。CH22 の delta の要件を写すので requires に CH22

## Impact

- 触れる予定のパス: `tools/tsukau/`（新しい。`tsukau.json`・`env-hook.sh`・`verify-batch.sh`・`public-title.sh`・`build-pc.sh`・`pc.sh`・`phone.sh`・`real-run.sh` と Sandbox の中で走らせる PowerShell）・
  `tools/verify-prep.sh`（最後に 1 か所呼ぶだけ）・`.github/workflows/ci.yml`（`rust` の job の `check-private.sh --self-test` の後に自己試験を 3 行）・`docs/testing.md`・`docs/changes/CH21.md`・この change
- 触れないもの: 製品のコード（`crates/`・`collector-android/`・`web/`・`migrations/`）・harness2 のファイル（`scripts/`・`.claude/skills/`・`.harness2/`。処置は依頼として harness2 の受け皿に置く）・
  `tools/record-env.sh`・`tools/recording/`（読んで立て方を写すだけ）・`tools/release/`（`package.sh windows` を呼ぶだけ）・`tools/check-private.sh`・`tools/check-secrets.sh`・`tools/emulator-start.sh`・`tools/android-env.sh`（呼ぶだけ）
- **harness2 への依頼**: 処置（選別・直し・問い・違和感の issue・確かめ直しを呼ぶこと）は harness2 の確認バッチに足す（design D15）。手で呼ぶ `/tsukau` で席の環境から `adb` を外す口（IMP-13）も、`scripts/tsukau.py` が席を起こす所の変更なので同じ依頼に足す見込み。CH21 はそれが入るまで終わらない（Q13 の推奨）。CH21 の実装（下流のブランチ）は、最後の Task（処置まで本物で通す）が harness2 の処置が入るまで通らないので、それまで main に入らない。**その間、確認バッチで席は回らない**（review/spec.md R7。Q1・Q13 の代償の文もこれに直した）
- **確認バッチの準備が延びる**: 相手ごとに、PC の zip の組み立て（未測。Task 1 で測る）＋立てる（上限 15 分。携帯は APK の組み立てとエミュレータの起動を含む）＋席（10 分）＋片付け（上限 5 分）。全体の上限は 9,360 秒。その間、確認バッチの作業ツリーの錠を握り続け、記録と fix の撮り直しが待たされる
- **外へ出るもの**: PC を変えた回ごとに、Sandbox の中の更新係と PC の収集アプリの問い合わせ（この PC の網の IP と時刻）と、席が範囲の中で開いた外のサイトへの接続（Q8 の本人の答えの代償）。携帯のエミュレータは網を切るので何も出ない。公開の repo に出るのは、手順書と PR 本文の段落（止まり方・件数・伏せた題・前の回で試した ID）と、harness2 の処置が書く題・種類・どの Story・回・番号（Q4）
- **本人の画面**: PC を変えた回ごとに、Sandbox の窓が本人のデスクトップに出る（Q9。1 日 1〜5 回まで）
- **走っている change との重なり**: CH22 は `.github/workflows/ci.yml` の `rust` の job に `soak-install.sh --self-test` を足し、`docs/testing.md` を書き足すので、CH21 は ci.yml の別の位置（`check-private.sh --self-test` の後）に足し、testing.md には別の 1 節を足す。
  CH23 は `docs/testing.md` の毎晩の行に書き足し、CH21 は同じファイルの別の場所。CH19（エミュレータだけの試験の分類）とは重ならない。
  CH22 は正典 distribution の「自動の試験は本物の配布元に問い合わせない」を名指しの 2 つに書き直しており、Q8 の本人の答え（外につなぐ）で CH21 はその上に 3 つ目を積む（requires に CH22。CH22 の下流（PR #349）が merge gate を通るまで CH21 は待つ）。録画（harness2 `record-run`）とは Sandbox を取り合う（design D3）。
  計測テスト（`tools/android-emulator.sh`）と確認バッチの準備の APK の入れ先とはエミュレータを取り合いうる（別の AVD と serial の名指しで分ける。design D22（仮））

## 持ち越し

- 手で呼んだ席の処置（Q5 の推奨。本人が `RESULT.md` を読んで `/iwakan` か `/hanasu` に回す）
- 確認バッチに後から足す、すでに main の Story（`--story`）は木の差に出ないので範囲に入らない（Q6 の context。要るようになったら別の change）
