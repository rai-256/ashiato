# CH21 確認バッチの前に、その回で足した機能だけを使ってみる席（/tsukau）で軽く確かめる

change: `openspec/changes/ch21-trial-use-before-verify/`（capability `verify-environment`。ADDED の Requirement 9 つ・Scenario 41 本）。
依頼と完了の判定は `docs/changes/CH21.md`。

harness2 の使ってみる席（`/tsukau`・`scripts/tsukau.py`）が本物を立てる差し込み口（`tools/tsukau/tsukau.json` と `env-hook.sh`）を ashiato2 に置く。
相手は画面（専用の compose project・乱数の合言葉・偽データ）と PC（網を切ったまっさらな Windows Sandbox に配る形のサーバの zip を入れ、窓を開いて PC の収集アプリを起こす）。
確認バッチの準備（`tools/verify-prep.sh`）の最後から、その回に入った Story / change が変えたファイルで相手を決め、表題と「何のためか」を範囲にして、相手ごとに 10 分・20 万 token で 1 回呼ぶ。
手順書と verify PR の本文に写すのは件数・題・止まり方だけで、題は私的な形と commit の検査が止める形を伏せる。席が落ちても準備は止めない。製品のコードと harness2 は変えない。

## 深掘りで本人が決めること（第 1 回は答え待ち）

第 1 回（Q1〜Q8）は推奨の答えで先に書いた（`deep.md` の「答え待ち」）。答えが推奨と違えば `spec --answers` で写す。

- Q1 範囲と相手の決め方 —— 推奨: 変えたファイルと表題から機械で
- **Q2 Sandbox の網（A。失うもの: 外に出る。答えを省けない）** —— 推奨: 網を切る。
  spec-review（review/spec.md R1）を受けて推奨を移した: 外につなぐ案は、CH22 が書き直す正典 distribution の「自動の試験が本物の配布元へ繋ぐのを名指しの 2 つに限る」に 3 つ目を足すことになり、CH22 の後に積む必要がある。#329 の型は CH22 の CI が見る
- Q3 配る zip の出所 —— 推奨: 確認バッチの木から手元で組む
- Q4 Sandbox の窓 —— 推奨: 窓を開いて画面も触る
- Q5 毎回の大きさ —— 推奨: 10 分・20 万（立てる上限 15 分）
- Q6 作り直しのとき —— 推奨: 同じ中身なら前の結果を写す
- Q7 警告の書き方 —— 推奨: 止まり方ごとの決まった文言だけ
- Q8 どこまでで終わりか —— 推奨: 実地の 1 回まで（CH21 だけの確認バッチでは席が回らないので、同じ台本を使い捨ての確認バッチの木で本物に通す）

## 仮決め（B）

台帳（`provisional.jsonl`）に載せたもの: D1・D3・D5・D7・D9・D12・D14・D15。

- D1（仮）置き場を `~/.local/state/ashiato2/tsukau` に明示し、`--label verify-<tag>` で走らせる
- D3（仮）PC は Sandbox を 1 つだけ起こし（動いていれば待たずに断る）、自分の id だけを止め、止める前に中のログを写す
- D5（仮）配る zip は席を呼ぶ前に `C:\dev\ashiato2-tsukau-build` で組み（上限 2,400 秒）、ready の待ちに数えない。反転条件: 温まった組み立てが 15 分を超えるか組めない
- D7（仮）画面の相手は録画の専用環境と同じ立て方で、`.env` の無い木から起動する
- D9（仮）席は確認バッチの HEAD を取り出した使い捨ての作業ツリーで走らせ、harness2 を繋ぎ `.env` を写さない
- D12（仮）・D15（仮）題は利用者名・ホスト名・私設網の名前・`.env` の値・絶対パスと、commit の検査（`check-private`・`check-secrets`）が止める形を伏せ、1 行に整える
- D14（仮）実地の 1 回は、画面と PC を変えた偽の merge を重ねた使い捨ての確認バッチの木で `verify-batch.sh` を本物で通す
- 第 1 回の推奨の写し（本人の答えで変わる）: D2（Q1）・D4（Q4）・D6（Q2。網を切る）・D8（Q5）・D10（Q6）・D11（Q7）
- 聞かずに決めたこと（C）: D13（CI の自己試験）

## capability

前倒しなし。確認バッチの準備の約束（エミュレータにだけ APK を入れる）が既にある `verify-environment` に置いた。

## tasks

9 Task・13 項目（zip を組む → 範囲 → 呼んで段落にする・題を伏せる → verify-prep から呼ぶ → 差し込み口と画面 → PC → 本物の Sandbox → 本物で 1 回通す・CI・文書 → 全体）。
本物の Sandbox と claude を使う確かめ（Task 1・7・8.1）はこの PC で走る。網を切った Sandbox でインストールと loopback が通るかは確かめていない（Task 7 で確かめ、通らなければ人間に返す）。

## レビュー

- `review/deep.md`: 21 件。fixed 14（うち仮 6）・escalated 7（第 1 回の問いにした）
- `review/spec.md`: 20 件。fixed 17（うち仮 5。結果の読み元を最後の行の JSON に・打ち切りは SIGTERM で片付けを待つ・準備の全体の上限・PC の収集アプリは窓の後に利用者のセッションで・試験の置き場を範囲から除く・段落を相手ごとに・Scenario の分割）・escalated 3（R1・R2 を Q2 に、R14 を Q8 に）

## 走っている change との重なり

- CH22: `.github/workflows/ci.yml` の `rust` の job に自己試験を足す位置を分けた。Q2 で外につなぐ案が選ばれたときだけ、CH22 が書き直す distribution の約束の上に積む
- CH23: `docs/testing.md` の別の場所に書き足す
- CH19: 重ならない

🤖 Generated with [Claude Code](https://claude.com/claude-code)
