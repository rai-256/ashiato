## ADDED Requirements

### Requirement: 使ってみる席の差し込み口は、画面と PC の相手を持ち、手で呼んでも確認バッチから呼んでも同じ置き場に結果を残す

THE SYSTEM SHALL harness2 の使ってみる席（`scripts/tsukau.py`）が読む差し込み口 `tools/tsukau/tsukau.json` を持たせる。
鍵は `env_hook`（`tools/tsukau/env-hook.sh`。実行できる）・`targets`（`web`（画面）と `pc`（PC）の 2 つ）・`out_root`（repo の外の `~/.local/state/ashiato2/tsukau`）・`ready_timeout_s`（900）の 4 つだけにする。携帯（`android`）は持たない。
THE SYSTEM SHALL 手で呼ぶ `/tsukau` も、確認バッチの準備から呼ぶ席も、この差し込み口と同じ `out_root` を使わせる（どの作業ツリーから呼んでも、実行のフォルダが 1 つの置き場に並ぶ）。
導出元: CH21（依頼の「どこまで」）。深掘り Q5（第 1 回の推奨: 立てる上限 15 分）。review/deep.md R11。

#### Scenario: 差し込み口の形が席の台本の契約に合う
- **WHEN** `tools/tsukau/tsukau.json` を読む
- **THEN** 鍵は `env_hook`・`targets`・`out_root`・`ready_timeout_s` の 4 つだけで、`env_hook` の台本は実行でき、`ready_timeout_s` は 900、`out_root` は `~/.local/state/ashiato2/tsukau` である

#### Scenario: 携帯は相手に持たない
- **WHEN** `tools/tsukau/tsukau.json` の `targets` を読む
- **THEN** `targets` はちょうど `["web", "pc"]` で、`android` を含まない

### Requirement: 確認バッチの範囲と相手は、その回に入った Story と change が変えたファイルと表題から機械で決める

WHEN 確認バッチの準備が席を呼ぶとき、
THE SYSTEM SHALL 確認バッチの作業ツリーの、`origin/main` から HEAD までの merge のうち「確認バッチ <tag> に入れる」merge のそれぞれについて、その merge が入れたファイル（merge の 1 つ目の親との差）で相手を決めさせる。
画面のファイル（`web/src/`・`web/index.html`・`web/public/`）を変えたものは画面の相手に、サーバ・PC の収集アプリ・更新係・インストーラ・サービスの部品・移行・配る形の組み立て（`crates/` の下・`migrations/`・`tools/release/`）を変えたものは PC の相手に入れる。両方を変えたものは両方に入れる。
試験の置き場（`web/src/__tests__/`・`crates/` の下の `tests/`・名前が `_tests.rs` で終わるファイル）は数えない。携帯（`collector-android/`）だけ、文書・道具・試験だけを変えたものは、どの相手にも入れない。
THE SYSTEM SHALL 相手ごとの範囲を、入れたものごとに 1 行の「<ID> <表題>: <何のためか>」を並べたものにさせる。表題は Story の `docs/stories/ST<NN>.md` か change の `docs/changes/CH<NN>.md` の front matter の `title`、
「何のためか」はその文書の「価値」の節の段落のうち、`>`（引用）でも `（`（注記・雛形）でも始まらない最初の段落の 1 行目（120 字まで）。そういう段落が無ければ表題だけにする。
THE SYSTEM SHALL 同じ作業ツリーの同じ HEAD からは、何度求めても同じ範囲と相手を返させる。範囲を AI に要約させない。
導出元: CH21。深掘り Q1（第 1 回の推奨: 変えたファイルと表題から機械で）。review/spec.md R13。

#### Scenario: 画面のファイルだけを変えた Story は、画面の相手の範囲に入る
- **GIVEN** `origin/main` に、`web/src/DayView.tsx` だけを変えた `feat/st12-day-view` を「確認バッチ t1 に入れる」merge を 1 つ重ねた木と、`docs/stories/ST12.md`（表題「1 日の画面」、価値の最初の行「本人として、1 日の動きを 1 枚で見たい。」）
- **WHEN** 範囲を求める
- **THEN** 画面の範囲は「ST12 1 日の画面: 本人として、1 日の動きを 1 枚で見たい。」の 1 行で、PC の範囲は空である

#### Scenario: サーバか PC のアプリか配る形の組み立てを変えたものは、PC の相手の範囲に入る
- **GIVEN** `crates/updater/src/lib.rs` を変えた `feat/ch30-x` の merge と、`migrations/202611200000_x.sql` を変えた `feat/st40-y` の merge と、`tools/release/build.sh` を変えた `feat/ch32-v` の merge を重ねた木
- **WHEN** 範囲を求める
- **THEN** PC の範囲は CH30・ST40・CH32 の 3 行で、画面の範囲は空である

#### Scenario: 両方を変えたものは、両方の範囲に入る
- **GIVEN** `web/src/App.tsx` と `crates/server/src/lib.rs` を変えた `feat/st20-z` の merge を重ねた木
- **WHEN** 範囲を求める
- **THEN** 画面の範囲と PC の範囲の両方に ST20 の行がある

#### Scenario: 携帯だけ・文書や道具だけ・試験だけを変えたものは、どの範囲にも入らない
- **GIVEN** `collector-android/app/src/main/kotlin/dev/ashiato/collector/A.kt` だけを変えた merge・`docs/testing.md` と `tools/verify-prep.sh` だけを変えた merge・`web/e2e/a.spec.ts` と `web/src/__tests__/a.test.ts` だけを変えた merge・`crates/server/src/stay_tests.rs` と `crates/collector-windows/tests/runtime_windows.rs` だけを変えた merge を重ねた木
- **WHEN** 範囲を求める
- **THEN** 画面の範囲も PC の範囲も空である

#### Scenario: 価値の節の注記と引用は飛ばし、無ければ表題だけを範囲に書く
- **GIVEN** `web/src/App.tsx` を変えた `feat/ch31-w` の merge と、価値の節が「（`hx change` の依頼の逐語。…）」だけの `docs/changes/CH31.md`（表題「並びを直す」）、`web/src/B.tsx` を変えた `feat/ch33-u` の merge と、価値の節が「> 第 1 回の答えで…」の引用の段落の次に「本人として、並びを読みたい。」の段落を持つ `docs/changes/CH33.md`（表題「並び」）
- **WHEN** 範囲を求める
- **THEN** 画面の範囲は「CH31 並びを直す」と「CH33 並び: 本人として、並びを読みたい。」の 2 行である

#### Scenario: 同じ木から 2 度求めると、同じ範囲になる
- **GIVEN** 上の merge を重ねた木
- **WHEN** 範囲を 2 度求める
- **THEN** 2 度の出力はバイト単位で同じである

### Requirement: 確認バッチの準備は、範囲のある相手ごとに 1 回、使い捨ての作業ツリーで、決まった時間と token で席を呼ぶ

WHEN 確認バッチの準備（`tools/verify-prep.sh <tag>`）が、既存の成果物を作り終えたとき、
THE SYSTEM SHALL `tools/tsukau/verify-batch.sh <tag>` を呼ばせ、範囲のある相手ごとに 1 回、`scripts/tsukau.py run --target <相手> --minutes 10 --tokens 200000 --label verify-<tag>` で席を呼ばせる。範囲が空の相手は呼ばない。
THE SYSTEM SHALL 席を、確認バッチの作業ツリーではなく、その HEAD を detached で取り出した使い捨ての作業ツリーで走らせる（`--repo` にその作業ツリーを渡す）。
その作業ツリーには harness2 を繋ぎ（`init.py --worktree`）、`.env` を写さない。繋げなければ、その相手の席を呼ばない。席が終わったら（どの止まり方でも）その作業ツリーを消す。
THE SYSTEM SHALL 画面の相手には、確認バッチの準備が組んだサーバと画面の置き場を絶対パスで渡させる（`ASHIATO_TSUKAU_SERVER_BIN`・`ASHIATO_TSUKAU_WEB_DIST`）。
THE SYSTEM SHALL PC の範囲があれば、席を呼ぶ前に確認バッチの木から配る zip を組ませ（`tools/tsukau/build-pc.sh`。上限 2,400 秒）、その置き場を絶対パスで渡させる（`ASHIATO_TSUKAU_PC_ZIPS`）。組めなければ PC の席を呼ばない（画面の席は呼ぶ）。
THE SYSTEM SHALL 1 回の呼び出しに、立てる上限・席の時間に 1 分を足した 1,560 秒（900 + 600 + 60）で SIGTERM を送り、席の台本の片付けを 360 秒まで待ってから止めさせる（合わせて 1,920 秒）。
打ち切った後、呼ぶ前に無かった Sandbox か `ashiato-tsukau-` で始まる compose project が残っていれば、片付けに残りがあるとみなす。
導出元: CH21（依頼「小さい時間と token で毎回呼ぶ」）。深掘り Q3（第 1 回の推奨: 確認バッチの木から手元で組む。組めなければ回さない）・Q5（10 分・20 万）。review/deep.md R1・R4・R8・R18。review/spec.md R7・R10・R12。

#### Scenario: 範囲のある相手だけを、決まった時間と token で呼ぶ
- **GIVEN** 画面の範囲だけがある確認バッチの木 `t1` と、受けた引数を記録し、本物と同じ 2 行（1 行目「使ってみた結果: <絶対パス>/RESULT.md」、最後の行に `stop`・`count`・`titles`・`run`・`cleanup` の JSON）を出して rc=0 で終わる偽の席の台本
- **WHEN** `tools/tsukau/verify-batch.sh t1` を走らせる
- **THEN** 偽の席の台本はちょうど 1 回呼ばれ、引数に `--target web`・`--minutes 10`・`--tokens 200000`・`--label verify-t1` があり、`--target pc` の呼び出しは無い

#### Scenario: 席を呼ぶ作業ツリーは確認バッチの作業ツリーと別で、harness2 が繋がり、.env が無い
- **GIVEN** 呼ばれた作業ツリー（`--repo`）のパスと、そこに `.harness2` と `.env` があるかを記録する偽の席の台本
- **WHEN** `tools/tsukau/verify-batch.sh t1` を走らせる
- **THEN** `--repo` は確認バッチの作業ツリーと違うパスで、そこに `.harness2` があり、`.env` は無かった

#### Scenario: 席が作業ツリーを書き換えても、確認バッチの作業ツリーは変わらず、使い捨ての作業ツリーは消える
- **GIVEN** 呼ばれた作業ツリーに `scribble.txt` を書く偽の席の台本
- **WHEN** `tools/tsukau/verify-batch.sh t1` を走らせる
- **THEN** 確認バッチの作業ツリーの `git status --porcelain` は呼ぶ前と同じで、`--repo` のパスは無く、`git worktree list` にも無い

#### Scenario: 画面の相手には、準備が組んだ置き場を絶対パスで渡す
- **GIVEN** 受けた環境変数を記録する偽の席の台本
- **WHEN** 確認バッチの作業ツリーで `tools/tsukau/verify-batch.sh t1` を走らせる
- **THEN** `ASHIATO_TSUKAU_SERVER_BIN` は `/` で始まり確認バッチの作業ツリーの `dist/verify-t1/ashiato-server` を指し、`ASHIATO_TSUKAU_WEB_DIST` は `/` で始まり `dist/verify-t1/web` を指す

#### Scenario: 配る zip を組めなければ、PC の席は呼ばず、画面の席は呼ぶ
- **GIVEN** 画面と PC の両方の範囲がある確認バッチの木と、rc=1 で終わる偽の組み立ての台本
- **WHEN** `tools/tsukau/verify-batch.sh t1` を走らせる
- **THEN** 偽の席の台本は `--target web` で 1 回だけ呼ばれ、PC の段落は「配る zip を組めず、回さなかった」である

#### Scenario: 時間の上限で打ち切り、片付けの間は待つ
- **GIVEN** SIGTERM を受けると 1 秒後に `"stop":"interrupted"` の最後の行を出して rc=3 で終わり、それまでは終わらない偽の席の台本と、SIGTERM までの秒数を 2 秒に縮めた自己試験
- **WHEN** `tools/tsukau/verify-batch.sh t1` を走らせる
- **THEN** 偽の席の台本は SIGTERM を受けて最後の行を出してから終わり、`verify-batch.sh` は rc=0 で、画面の段落は「時間の上限で打ち切った」である

#### Scenario: 打ち切った後に立てたものが残っていれば、片付けに残りがあると書く
- **GIVEN** 呼ぶ間に `ashiato-tsukau-x` の compose project を残したと偽の `docker` に答えさせ、SIGTERM を受けても終わらない偽の席の台本と、秒数を縮めた自己試験
- **WHEN** `tools/tsukau/verify-batch.sh t1` を走らせる
- **THEN** 画面の段落に「時間の上限で打ち切った」と「片付けに残りがある」がある

### Requirement: 席の結果は、件数・題・止まり方だけを決まった文言で手順書に写す

THE SYSTEM SHALL 確認バッチの準備に、席の結果を `dist/verify-<tag>/manifest.md` の最後に、頭の 1 段落と相手ごとに 1 段落ずつ足させる（手順書の冒頭と verify PR の本文に載る。手順書は段落ごとに 1 つの note になる）。
頭の段落は「使ってみる席（AI が利用者として使ってみた結果。人間の確認ではない）」、相手の段落は「画面: 」か「PC: 」で始まる。
写すのは、席の台本の最後の行の JSON の止まり方（`stop`）・件数（`count`）・題（`titles`）と、片付けに残りがあるか（`cleanup` が「済み」か）だけで、
1 行目のパス・`run`・`cleanup` の文・候補の本文・根拠・画像・ログは写さない。最後の行が JSON として読めなければ「席を呼べなかった」にする。
THE SYSTEM SHALL 止まり方ごとに次の決まった文言を書かせる: `done` は「席が自分で終えた」、`time` は「時間を使い切った」、`tokens` は「token を使い切った」に続けて「。違和感の候補 <件数> 件」と題の一覧、
`crashed` は「席が途中で落ちた」、`env_failed` は「相手を立てられなかった」、`interrupted` は「中断した」、席の台本が rc=2 で終わった・最後の行が読めない・台本が無い・harness2 を繋げなかったときは「席を呼べなかった」、
打ち切ったときは（最後の行の JSON に依らず）「時間の上限で打ち切った」、範囲が空の相手は「回さなかった（この回に<相手>を変えたものが無い）」、PC の配る zip を組めなかったときは「配る zip を組めず、回さなかった」。
片付けに残りがあれば、その段落に「片付けに残りがある」を足す。
THE SYSTEM SHALL どの相手をどの範囲で呼び、どの実行のフォルダに結果があるかは、手元の `dist/verify-<tag>/tsukau.log`（git が無視する）にだけ書かせる。
THE SYSTEM SHALL issue を作らせない。
導出元: CH21（依頼「件数・題・止まり方だけ」「issue は作らない」）。深掘り Q7（第 1 回の推奨: 止まり方ごとの決まった文言だけ）。review/deep.md R10。review/spec.md R6・R15。

#### Scenario: 席が終えた相手は、件数と題を手順書に載せる
- **GIVEN** 1 行目に「使ってみた結果: /home/someone/.local/state/ashiato2/tsukau/20261120-010203-web/RESULT.md」、最後の行に `"stop":"done"`・`"count":2`・`"titles":["更新の確認が赤いまま","設定の言葉が分からない"]`・`"run":"/home/someone/.local/state/ashiato2/tsukau/20261120-010203-web"`・`"cleanup":"済み"` を出す偽の席の台本
- **WHEN** `tools/tsukau/verify-batch.sh t1` を走らせる
- **THEN** 出力は空行で分かれた段落で、頭の段落は「使ってみる席（AI が利用者として使ってみた結果。人間の確認ではない）」で始まり、「画面: 」で始まる段落に「席が自分で終えた。違和感の候補 2 件」と 2 つの題がある
- **AND** 出力に `/home/`・`RESULT.md`・`.local/state` は無い

#### Scenario: 止まり方ごとに決まった文言だけを書く
- **GIVEN** 最後の行の `stop` が `time`・`tokens`・`crashed`・`env_failed`・`interrupted` のそれぞれで、`cleanup` に「残りあり（down rc=1。logs/env-down.log）」を出す偽の席の台本と、rc=2 で終わり何も出さない偽の席の台本と、最後の行が JSON でない偽の席の台本
- **WHEN** それぞれで `tools/tsukau/verify-batch.sh t1` を走らせる
- **THEN** 画面の段落はそれぞれ「画面: 」に続けて「時間を使い切った」「token を使い切った」「席が途中で落ちた」「相手を立てられなかった」「中断した」「席を呼べなかった」「席を呼べなかった」で始まり、`cleanup` を出した 5 つには「片付けに残りがある」がある
- **AND** どの出力にも `logs/env-down.log` と `down rc=1` は無い

#### Scenario: 範囲が空の相手は、回さなかったと書く
- **GIVEN** 文書だけを変えた merge だけを重ねた確認バッチの木
- **WHEN** `tools/tsukau/verify-batch.sh t1` を走らせる
- **THEN** 席の台本は呼ばれず、段落に「回さなかった（この回に画面を変えたものが無い）」と「回さなかった（この回に PC を変えたものが無い）」がある

#### Scenario: issue を作らない
- **GIVEN** 受けた引数を記録する偽の `gh` を `PATH` の先に置き、候補 1 件を出す偽の席の台本
- **WHEN** `tools/tsukau/verify-batch.sh t1` を走らせる
- **THEN** 偽の `gh` は 1 回も呼ばれない

### Requirement: 題は写す前に、私的な形と確認バッチの commit を止める形を伏せ、1 行に整える

THE SYSTEM SHALL 題を写す前に、題に次のどれかがあれば、その題を「（伏せた）」に置き換えさせる（件数には数える）:
この PC の利用者名・ホスト名・私設網の名前、確認バッチの作業ツリーの `.env` の 8 字以上の値、絶対パスの形（`/home/`・`/mnt/`・`/Users/`・`C:\`・`\\` で始まる語）、
確認バッチの手順書の commit の検査（`tools/check-private.sh`・`tools/check-secrets.sh`）が止める形（私設網と手元の網の IPv4・私設網のホスト名・秘密の形）。
検査は台本を持つ使い捨ての作業ツリーに題を置き、一時の index で stage して当てる（確認バッチの作業ツリーには何も置かない）。検査を走らせられない（`tools/check-secrets.sh` が gitleaks を用意できない など）ときは、全部の題を伏せる。
THE SYSTEM SHALL 伏せなかった題は、改行を空白にし、80 字で切り、`@` を「＠」に、`#` に続く数字の `#` を「＃」に替えてから写させる（verify PR の本文で人への通知や issue への繋がりにしない）。
導出元: CH21（依頼「題を写す」）。review/deep.md R5。review/spec.md R9。

#### Scenario: 私的な形を含む題は伏せて、件数には数える
- **GIVEN** 題が「/home/someone/x が開けない」「C:\Users\a で落ちる」「並びがおかしい」の 3 つと、この PC のホスト名を含む題 1 つの最後の行を出す偽の席の台本
- **WHEN** `tools/tsukau/verify-batch.sh t1` を走らせる
- **THEN** 画面の段落は「違和感の候補 4 件」で、題は「（伏せた）」が 3 つと「並びがおかしい」で、ホスト名と `/home/someone` と `C:\Users\a` は出力に無い

#### Scenario: 伏せた後の段落は、確認バッチの commit の検査を通る
- **GIVEN** 私設網の IPv4 の形（自己試験が数から組んだもの）を含む題、私設網のホスト名の形を含む題、秘密の形を含む題、ふつうの題 1 つの最後の行を出す偽の席の台本
- **WHEN** `tools/tsukau/verify-batch.sh t1` の出力を、台本を持つ使い捨ての作業ツリーのファイルに置いて一時の index で stage し、`tools/check-private.sh --staged` と `tools/check-secrets.sh --staged` を走らせる
- **THEN** どちらも rc=0 で、段落の件数は 4 件、「（伏せた）」は 3 つである

#### Scenario: 題は 1 行に整え、通知の形を替える
- **GIVEN** 題が「@media の指定が」改行「効かない #12 と同じ」の 1 つと、100 字の題 1 つの最後の行を出す偽の席の台本
- **WHEN** `tools/tsukau/verify-batch.sh t1` を走らせる
- **THEN** 段落の題は「＠media の指定が 効かない ＃12 と同じ」と、元の題の頭 80 字で、出力に `@` と `#12` は無く、画面の段落は空行で割れていない

### Requirement: 席が落ちても時間切れでも、確認バッチの準備は止まらず、今の成果物を変えない

THE SYSTEM SHALL 席の呼び出し（`tools/tsukau/verify-batch.sh`）がどんな終わり方をしても（非 0・何も出さない・台本が無い・終わらない）、確認バッチの準備（`tools/verify-prep.sh`）を rc=0 のまま続けさせ、そのときは手順書に「使ってみる席: 呼べなかった」で始まる 1 段落を足させる。
THE SYSTEM SHALL 確認バッチの準備に、席の呼び出し全体を 6,540 秒（組み立ての 2,400 秒 + 2 つの相手の 1,920 秒ずつ + 300 秒）で打ち切らせる。
THE SYSTEM SHALL 席を呼んでも、確認バッチの準備が作る成果物（`ashiato-server`・`web/`・`app-debug.apk`・`run.sh`）と、手順書の既存の段落を変えさせない。席の段落は既存の段落の後に足すだけにする。
導出元: CH21（依頼「席が落ちても・時間切れでも、確認バッチの準備は止めない」）。深掘り Q8（第 1 回の推奨: 守るもの）。review/deep.md R10。review/spec.md R8。

#### Scenario: 席の呼び出しが落ちても、準備の rc は 0 のまま
- **GIVEN** rc=1 で終わり何も出さない偽の `tools/tsukau/verify-batch.sh` と、中身のある仮の `manifest.md`
- **WHEN** 確認バッチの準備の、席を呼ぶ部分を `set -euo pipefail` の下で走らせる
- **THEN** rc=0 で、`manifest.md` の頭は元の中身のままで、その後に「使ってみる席: 呼べなかった」で始まる 1 段落がある

#### Scenario: 席の呼び出しが終わらなくても、上限で打ち切って準備を続ける
- **GIVEN** 終わらない偽の `tools/tsukau/verify-batch.sh` と、上限を 2 秒に縮めた自己試験
- **WHEN** 確認バッチの準備の、席を呼ぶ部分を `set -euo pipefail` の下で走らせる
- **THEN** 10 秒以内に rc=0 で終わり、`manifest.md` に「使ってみる席: 呼べなかった」で始まる段落がある

#### Scenario: 席の段落は、既存の段落の後に足すだけ
- **WHEN** 確認バッチの準備の台本を読む
- **THEN** 席を呼ぶ部分は `manifest.md` を書く部分より後にあり、`manifest.md` へは追記（`>>`）だけで書き、`run.sh`・`ashiato-server`・`web/`・`app-debug.apk` を書く行を持たない

### Requirement: 確認バッチの中身が前の回と同じなら、席を呼ばずに前の結果を写す

WHEN 確認バッチの準備が相手の範囲を決めたとき、
THE SYSTEM SHALL `origin/main` のコミット・相手・その相手の範囲に入ったものごとの ID とその PR のいちばん新しいコミットから鍵を作らせ、同じ鍵で前に席を呼んだ結果が `out_root` の下にあれば、席を呼ばずにその結果（件数・題・止まり方・片付けに残りがあるか）を写させる。
そのときの段落には「前の回 <前の tag> と同じ中身なので、その結果を写した」を足す。前の結果の止まり方が `done`・`time`・`tokens` でないなら写さずに呼ぶ。
導出元: CH21（依頼「毎回」）。深掘り Q6（第 1 回の推奨: 同じ中身なら前の結果を写す）。review/deep.md R19。

#### Scenario: 同じ中身で作り直した回は、席を呼ばずに前の結果を写す
- **GIVEN** 確認バッチ `t1` で画面の席が `done`・候補 1 件「並びがおかしい」で終わった後、同じ `origin/main` と同じ PR のコミットで作り直した確認バッチ `t2`
- **WHEN** `tools/tsukau/verify-batch.sh t2` を走らせる
- **THEN** 席の台本は呼ばれず、画面の段落に「違和感の候補 1 件」「並びがおかしい」と「前の回 t1 と同じ中身なので、その結果を写した」がある

#### Scenario: main が進んだか、PR のコミットが変われば、呼び直す
- **GIVEN** `t1` の後に、`origin/main` だけを進めた確認バッチ `t3` と、画面の範囲の PR のコミットだけを進めた確認バッチ `t4`
- **WHEN** それぞれで `tools/tsukau/verify-batch.sh` を走らせる
- **THEN** どちらでも画面の席の台本が 1 回呼ばれる

#### Scenario: 前の回が落ちていたら、写さずに呼び直す
- **GIVEN** `t1` で画面の席が `env_failed` で終わった後、同じ中身の確認バッチ `t2`
- **WHEN** `tools/tsukau/verify-batch.sh t2` を走らせる
- **THEN** 画面の席の台本が 1 回呼ばれる

### Requirement: PC の相手は、網を切ったまっさらな Windows Sandbox に配る形を入れて立て、自分が起こした Sandbox だけを、中のログを写してから片付ける

WHEN 差し込み口が PC の相手を立てるとき（`env-hook.sh up <repo> <state> pc`）、
THE SYSTEM SHALL 起こす前に別の Windows Sandbox が動いていれば立てずに `UNAVAILABLE=` を置かせる（待たない）。
THE SYSTEM SHALL 配る形のサーバの zip と PC の収集アプリの zip が、`ASHIATO_TSUKAU_PC_ZIPS` の置き場にも `dist/tsukau-pc/<HEAD のコミット>/` にも無ければ、立てずに `UNAVAILABLE=` を置かせる（理由に `tools/tsukau/build-pc.sh` の名前）。
THE SYSTEM SHALL そうでなければ、網を切った（`<Networking>Disable</Networking>`）Sandbox を 1 つ起こし、その id を `<state>` に残し、
配る形のサーバの zip をインストーラで入れ、サーバが応えたら Sandbox の窓を本人のデスクトップに開き（`wsb connect`）、その後に配る形の PC の収集アプリを利用者のセッション（`ExistingLogin`）で起こしてから、案内（`GUIDE=`）を置いた `ready` を置かせる。
案内には、Sandbox の中でコマンドを走らせて出力を受け取る口と画面の写しを撮る口（`tools/tsukau/pc.sh`）、サービスとログの場所、網が切ってあること（更新の確認が「つながらない」になるのは網を切ったため）、
触らないもの（本人のサーバ・本人の PC の配布版 `C:\ProgramData\ashiato`・確認バッチの DB・main の checkout の `.env`・本人が起こした Sandbox）を書く。
THE SYSTEM SHALL 片付け（`env-hook.sh down <repo> <state> pc`）では、Sandbox の中のサービスのログ・更新係の置き場の `checks.jsonl` とログ・PC の収集アプリのログ・サービスに関わるシステムのイベントの抜き書きを `$TSUKAU_EVIDENCE/pc-logs/` へ写してから、`<state>` に残した id の Sandbox だけを止めさせる。
止めた後もその id が動いていれば rc≠0 で終わらせる。ほかの Sandbox には触らせない。
導出元: CH21（依頼「PC 側（Windows Sandbox に配る zip を入れる）」）。深掘り Q2（第 1 回の推奨: 網を切る）・Q3（確認バッチの木から手元で組む）・Q4（窓を開いて画面も触る）。review/deep.md R6・R7。review/spec.md R1・R5。

#### Scenario: 別の Sandbox が動いていれば、立てずに断る
- **GIVEN** 動いている Sandbox の id を 1 つ返す偽の `wsb`
- **WHEN** `tools/tsukau/env-hook.sh up <repo> <state> pc` を走らせる
- **THEN** `<state>/ready` に `UNAVAILABLE=` の行があり、`wsb start` は呼ばれない

#### Scenario: 配る zip が無ければ、立てずに断る
- **GIVEN** 動いている Sandbox が無い偽の `wsb` と、空の配る zip の置き場
- **WHEN** `tools/tsukau/env-hook.sh up <repo> <state> pc` を走らせる
- **THEN** `<state>/ready` に `UNAVAILABLE=` の行があり、その理由に `tools/tsukau/build-pc.sh` の名前がある。`wsb start` は呼ばれない

#### Scenario: Sandbox は網を切って起こす
- **GIVEN** 動いている Sandbox が無く、`start` で id `sb-1` を返し、受けた設定を記録する偽の `wsb` と、配る zip の置き場
- **WHEN** `tools/tsukau/env-hook.sh up <repo> <state> pc` を走らせる
- **THEN** `wsb start` に渡した設定に `<Networking>Disable</Networking>` があり、`wsb start` は 1 回だけ呼ばれる

#### Scenario: 立てるときは、サーバを入れ、窓を開き、利用者のセッションで PC の収集アプリを起こしてから案内を置く
- **GIVEN** 上と同じ偽の `wsb` と配る zip の置き場
- **WHEN** `tools/tsukau/env-hook.sh up <repo> <state> pc` を走らせる
- **THEN** 偽の `wsb` は `start`・サーバのインストールの `exec`・`connect --id sb-1`・`-r ExistingLogin` で PC の収集アプリを起こす `exec` の順に呼ばれ、その後に `<state>/ready` が置かれる

#### Scenario: PC の案内は、口と網と触らないものを書く
- **GIVEN** 上と同じ偽の `wsb` と配る zip の置き場
- **WHEN** `tools/tsukau/env-hook.sh up <repo> <state> pc` を走らせる
- **THEN** `ready` の `GUIDE=` の案内に `tools/tsukau/pc.sh`・「網を切って」・`C:\ProgramData\ashiato`・`.env`・「触らない」がある

#### Scenario: 片付けは、中のログを写してから自分の Sandbox だけを止める
- **GIVEN** `<state>` に id `sb-1` が残り、動いている Sandbox として `sb-1` と `sb-other` を返す偽の `wsb`
- **WHEN** `tools/tsukau/env-hook.sh down <repo> <state> pc` を走らせる
- **THEN** ログを共有のフォルダへ写す `exec --id sb-1` が `stop --id sb-1` より先に呼ばれ、`$TSUKAU_EVIDENCE/pc-logs/` に写したファイルがある
- **AND** `stop` は `sb-1` にだけ呼ばれ、`sb-other` への呼び出しは 1 つも無い

#### Scenario: 止めた後も自分の Sandbox が動いていれば、片付けは非 0 で終わる
- **GIVEN** `stop` の後も `sb-1` を動いていると返す偽の `wsb`
- **WHEN** `tools/tsukau/env-hook.sh down <repo> <state> pc` を走らせる
- **THEN** rc≠0 で終わる

#### Scenario: 本物の Sandbox で、網を切ったまま配る形が入り、片付けで残らない
- **WHEN** この PC で、組んだ配る zip を置いて `tools/tsukau/env-hook.sh up` と `down` を PC の相手で本物の Sandbox に対して走らせる
- **THEN** `ready` に `UNAVAILABLE=` が無く、Sandbox の中のサーバが loopback で応え、PC の収集アプリのプロセスがあり、`down` は rc=0 で、終わった後に動いている Sandbox は無い

### Requirement: 画面の相手は専用の環境で立て、本人の合言葉と確認バッチの DB を使わない

WHEN 差し込み口が画面の相手を立てるとき（`env-hook.sh up <repo> <state> web`）、
THE SYSTEM SHALL `<state>/tree` に `--repo` の HEAD を detached で取り出した木に、実行ごとの compose project（`ashiato-tsukau-` で始まる名前）・空いている port・その場で作った乱数の合言葉を持つ `.env` と port の上書きを置き（録画の専用環境と同じ形）、その木で作り直した偽データとともに `tools/stack.sh up` を起こさせる。
`--repo` の作業ツリーの `.env` は読まず、`--repo` には何も書かない。本人の合言葉・確認バッチの DB（`ashiato-verify-` の compose project）・既定の compose project を使わない。
THE SYSTEM SHALL サーバと画面の置き場が `ASHIATO_TSUKAU_SERVER_BIN`・`ASHIATO_TSUKAU_WEB_DIST` で渡されたらそれを使い、渡された置き場が無ければ組まずに `UNAVAILABLE=` を置かせる。渡されていなければ（手で呼んだとき）`<state>/tree` で組む。
THE SYSTEM SHALL 案内に、画面の URL・その回の画面の合言葉・本物のブラウザで画面を開いて写しを撮るコマンド・サーバのログの場所・触らないもの（本人のサーバ・確認バッチの DB・main の checkout の `.env`）を書かせ、`INFO_COMMIT=` に立てたコミットを残させる。
THE SYSTEM SHALL 片付けでは、その compose project のコンテナと volume だけを消し、`<state>/tree` を消させ、残れば rc≠0 で終わらせる。
導出元: CH21。review/deep.md R4・R11（録画の専用環境と同じ立て方）。review/spec.md R11・R12。

#### Scenario: 画面の相手は、本人の .env を持たない別の木から起動する
- **GIVEN** `--repo` の作業ツリーに `WEB_PASSWORD=honnin-no-aikotoba` を持つ `.env` と、起こされた場所とその木の根の `.env` の中身を記録する偽の起動の台本（`<state>/tree/tools/stack.sh` の位置に置く）
- **WHEN** `tools/tsukau/env-hook.sh up <repo> <state> web` を走らせる
- **THEN** 起動の台本は `<state>/tree` の中で起こされ、その木の `.env` に `honnin-no-aikotoba` は無く `COMPOSE_PROJECT_NAME=ashiato-tsukau-` の行があり、`--repo` の `git status --porcelain` は呼ぶ前と同じである

#### Scenario: 画面の相手は、専用の project と本人のものでない合言葉で立つ
- **GIVEN** 上と同じ `.env` と、受けた環境変数を記録する偽の起動の台本
- **WHEN** `tools/tsukau/env-hook.sh up <repo> <state> web` を走らせる
- **THEN** 起動に渡った `COMPOSE_PROJECT_NAME` は `ashiato-tsukau-` で始まり、`WEB_PASSWORD` は `honnin-no-aikotoba` でなく、案内にも `honnin-no-aikotoba` は無い

#### Scenario: 渡されたサーバと画面の置き場がそのまま起動に届き、無ければ立てずに断る
- **GIVEN** 受けた環境変数を記録する偽の起動の台本と、`ASHIATO_TSUKAU_SERVER_BIN`・`ASHIATO_TSUKAU_WEB_DIST` に一時の置き場の絶対パス
- **WHEN** その置き場がある回と無い回のそれぞれで `tools/tsukau/env-hook.sh up <repo> <state> web` を走らせる
- **THEN** ある回は起動に渡った `SERVER_BIN`・`WEB_DIST` が渡した絶対パスと同じで、無い回は `ready` に `UNAVAILABLE=` があり、起動の台本は呼ばれない

#### Scenario: 画面の案内は、URL と触らないものを書き、立てたコミットを残す
- **GIVEN** 受けた環境変数を記録する偽の起動の台本
- **WHEN** `tools/tsukau/env-hook.sh up <repo> <state> web` を走らせる
- **THEN** 案内に `http://127.0.0.1:` で始まる画面の URL と「触らない」があり、`ready` に `INFO_COMMIT=` と `--repo` の HEAD のコミットがある

#### Scenario: 片付けは、その回の project だけを消す
- **GIVEN** 立てた回の compose project と、別の `ashiato-verify-t1` の project を返す偽の `docker`
- **WHEN** `tools/tsukau/env-hook.sh down <repo> <state> web` を走らせる
- **THEN** `docker compose` の `down -v` は立てた回の project にだけ呼ばれ、`ashiato-verify-t1` への呼び出しは無く、`<state>/tree` は無い
