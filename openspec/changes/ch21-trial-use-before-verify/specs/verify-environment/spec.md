## ADDED Requirements

### Requirement: 使ってみる席の差し込み口は、画面と PC と携帯の相手を持ち、手で呼んでも確認バッチから呼んでも同じ置き場に結果を残す

THE SYSTEM SHALL harness2 の使ってみる席（`scripts/tsukau.py`）が読む差し込み口 `tools/tsukau/tsukau.json` を持たせる。
鍵は `env_hook`（`tools/tsukau/env-hook.sh`。実行できる）・`targets`（`web`（画面）・`pc`（PC）・`android`（携帯のエミュレータ）の 3 つ）・`out_root`（repo の外の `~/.local/state/ashiato2/tsukau`）・`ready_timeout_s`（900）の 4 つだけにする。
THE SYSTEM SHALL 手で呼ぶ `/tsukau` も、確認バッチの準備から呼ぶ席も、この差し込み口と同じ `out_root` を使わせる。手で呼んだ席には処置へ渡す目録を作らせない（結果を手元に残すだけ）。
導出元: CH21（依頼の「どこまで」。携帯は依頼では「外」だったが、仕様への違和感（第 1 回の合意）「スマホも操作するようにしてください。エミュレータとかで」で相手に足した）。深掘り Q5（第 1 回の推奨: 結果を残すだけ）・Q10（立てる上限 15 分）。

#### Scenario: 差し込み口の形が席の台本の契約に合う
- **WHEN** `tools/tsukau/tsukau.json` を読む
- **THEN** 鍵は `env_hook`・`targets`・`out_root`・`ready_timeout_s` の 4 つだけで、`env_hook` の台本は実行でき、`ready_timeout_s` は 900、`out_root` は `~/.local/state/ashiato2/tsukau` である

#### Scenario: 相手は画面と PC と携帯の 3 つである
- **WHEN** `tools/tsukau/tsukau.json` の `targets` を読む
- **THEN** `targets` はちょうど `["web", "pc", "android"]` である

#### Scenario: 差し込み口だけで立てて片付けても、処置へ渡す目録は作らない
- **GIVEN** 偽の起動の台本と偽の `docker`
- **WHEN** 手で呼ぶ席と同じく `tools/tsukau/env-hook.sh up <repo> <state> web` と `down` だけを走らせる
- **THEN** `<repo>/dist` の下に `tsukau.json` は無い

### Requirement: 確認バッチの範囲と相手は、その回に入った Story と change が変えたファイルと表題から機械で決める

WHEN 確認バッチの準備が席を呼ぶとき、
THE SYSTEM SHALL 確認バッチの作業ツリーの、`origin/main` から HEAD までの merge のうち「確認バッチ <tag> に入れる」merge のそれぞれについて、その merge が入れたファイル（merge の 1 つ目の親との差）で相手を決めさせる。
画面のファイル（`web/src/`・`web/index.html`・`web/public/`）を変えたものは画面の相手に、サーバ・PC の収集アプリ・更新係・インストーラ・サービスの部品・移行・配る形の組み立て（`crates/` の下・`migrations/`・`tools/release/`）を変えたものは PC の相手に、携帯の収集アプリ（`collector-android/` の下）を変えたものは携帯の相手に入れる。2 つ以上を変えたものは、それぞれの相手に入れる。
試験の置き場（`web/src/__tests__/`・`crates/` の下の `tests/`・名前が `_tests.rs` で終わるファイル・`collector-android/app/src/test/`・`collector-android/app/src/androidTest/`）は数えない。文書・道具・試験だけを変えたものは、どの相手にも入れない。
THE SYSTEM SHALL 相手ごとの範囲を、入れたものごとに 1 行の「<ID> <表題>: <何のためか>」を並べたものにさせる。表題は Story の `docs/stories/ST<NN>.md` か change の `docs/changes/CH<NN>.md` の front matter の `title`、
「何のためか」はその文書の「価値」の節の段落のうち、`>`（引用）でも `（`（注記・雛形）でも始まらない最初の段落の 1 行目（120 字まで）。そういう段落が無ければ表題だけにする。
THE SYSTEM SHALL 同じ作業ツリーの同じ HEAD からは、何度求めても同じ範囲と相手を返させる。範囲を AI に要約させない。
導出元: CH21。深掘り Q6（第 1 回の推奨: 変えたファイルと表題から機械で）。携帯の相手は仕様への違和感（第 1 回の合意）で足した。

#### Scenario: 画面のファイルだけを変えた Story は、画面の相手の範囲に入る
- **GIVEN** `origin/main` に、`web/src/DayView.tsx` だけを変えた `feat/st12-day-view` を「確認バッチ t1 に入れる」merge を 1 つ重ねた木と、`docs/stories/ST12.md`（表題「1 日の画面」、価値の最初の行「本人として、1 日の動きを 1 枚で見たい。」）
- **WHEN** 範囲を求める
- **THEN** 画面の範囲は「ST12 1 日の画面: 本人として、1 日の動きを 1 枚で見たい。」の 1 行で、PC と携帯の範囲は空である

#### Scenario: 画面の入口と公開のファイルを変えたものも、画面の相手の範囲に入る
- **GIVEN** `web/index.html` だけを変えた `feat/ch34-a` の merge と、`web/public/icon.svg` だけを変えた `feat/ch35-b` の merge を重ねた木
- **WHEN** 範囲を求める
- **THEN** 画面の範囲は CH34・CH35 の 2 行で、PC の範囲は空である

#### Scenario: サーバか PC のアプリか配る形の組み立てを変えたものは、PC の相手の範囲に入る
- **GIVEN** `crates/updater/src/lib.rs` を変えた `feat/ch30-x` の merge と、`migrations/202611200000_x.sql` を変えた `feat/st40-y` の merge と、`tools/release/build.sh` を変えた `feat/ch32-v` の merge を重ねた木
- **WHEN** 範囲を求める
- **THEN** PC の範囲は CH30・ST40・CH32 の 3 行で、画面と携帯の範囲は空である

#### Scenario: 携帯の収集アプリを変えたものは、携帯の相手の範囲に入る
- **GIVEN** `collector-android/app/src/main/kotlin/dev/ashiato/collector/A.kt` だけを変えた `feat/st41-p` の merge と、`collector-android/app/build.gradle.kts` だけを変えた `feat/ch36-q` の merge を重ねた木
- **WHEN** 範囲を求める
- **THEN** 携帯の範囲は ST41・CH36 の 2 行で、画面と PC の範囲は空である

#### Scenario: 両方を変えたものは、両方の範囲に入る
- **GIVEN** `web/src/App.tsx` と `crates/server/src/lib.rs` を変えた `feat/st20-z` の merge を重ねた木
- **WHEN** 範囲を求める
- **THEN** 画面の範囲と PC の範囲の両方に ST20 の行がある

#### Scenario: 文書や道具だけ・試験だけを変えたものは、どの範囲にも入らない
- **GIVEN** `docs/testing.md` と `tools/verify-prep.sh` だけを変えた merge・`web/e2e/a.spec.ts` と `web/src/__tests__/a.test.ts` だけを変えた merge・`crates/server/src/stay_tests.rs` と `crates/collector-windows/tests/runtime_windows.rs` だけを変えた merge・`collector-android/app/src/test/kotlin/A.kt` と `collector-android/app/src/androidTest/kotlin/B.kt` だけを変えた merge を重ねた木
- **WHEN** 範囲を求める
- **THEN** 画面の範囲も PC の範囲も携帯の範囲も空である

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
THE SYSTEM SHALL 画面の相手と携帯の相手には、確認バッチの準備が組んだサーバと画面の置き場を絶対パスで渡させる（`ASHIATO_TSUKAU_SERVER_BIN`・`ASHIATO_TSUKAU_WEB_DIST`）。
THE SYSTEM SHALL PC の範囲があれば、席を呼ぶ前に確認バッチの木から配る zip を組ませ（`tools/tsukau/build-pc.sh`。上限 2,400 秒）、その置き場を絶対パスで渡させる（`ASHIATO_TSUKAU_PC_ZIPS`）。組めなければ PC の席を呼ばない（画面の席は呼ぶ）。
THE SYSTEM SHALL 1 回の呼び出しに、立てる上限・席の時間・席の台本の片付けの上限に 1 分を足した 1,860 秒（900 + 600 + 300 + 60）で SIGTERM を送り、席の台本の片付けを 360 秒まで待ってから止めさせる（合わせて 2,220 秒）。打ち切った後でも、最後の行が読めて `stop` が `done`・`time`・`tokens` なら、席を呼んだ回として件数と題を写させる。
打ち切った後、呼ぶ前に無かった Sandbox・専用の AVD（`ashiato-tsukau-api35`）のエミュレータ・`ashiato-tsukau-` で始まる compose project のどれかが残っていれば、片付けに残りがあるとみなす。
THE SYSTEM SHALL どの相手の席も、`PATH` から `$ANDROID_HOME` と `$ANDROID_SDK_ROOT` の下の置き場を外した環境で呼ばせ（ほかの置き場の並びは変えない）、その `PATH` で `adb` と `emulator` が名前で見つからないことを確かめさせる。見つかれば、その回はどの相手の席も呼ばず、段落を「席を呼べなかった」、目録の `state` を `not_run` にさせる（理由は `dist/verify-<tag>/tsukau.log` にだけ書く）。
導出元: CH21（依頼「小さい時間と token で毎回呼ぶ」）。深掘り Q7（第 1 回の推奨: 準備の中で、確認バッチの木から手元で組む）・Q10（10 分・20 万）。review/deep.md R2・R11。携帯の相手は仕様への違和感（第 1 回の合意）で足した。席の `PATH` から Android の道具を外すのは IMP-13 の本人の答え（第 2 回の合意「席を呼ぶ間は adb を席の PATH から外し、phone.sh だけが使えるようにする」。design D32・D33（仮））。

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

#### Scenario: 席を呼ぶ PATH には Android の道具の置き場が無く、adb が名前で見つからない
- **GIVEN** 偽の `$ANDROID_HOME` の `platform-tools`・`emulator`・`cmdline-tools/latest/bin` に偽の `adb`・`emulator`・`avdmanager` を置き、その 3 つの置き場を `PATH` の先に持つ準備の環境と、受けた `PATH` と `command -v adb`・`command -v emulator` の結果を記録する偽の席の台本
- **WHEN** 画面・PC・携帯の範囲がある確認バッチの木 `t1` で `tools/tsukau/verify-batch.sh t1` を走らせる
- **THEN** 偽の席の台本は 3 回呼ばれ、どの回も記録した `PATH` に `$ANDROID_HOME` の下の置き場は無く、`command -v adb` と `command -v emulator` は空で、準備の環境の `PATH` のほかの置き場は同じ順で残っている

#### Scenario: Android の道具の置き場の外にも adb があれば、どの相手の席も呼ばない
- **GIVEN** 上の準備の環境に、`$ANDROID_HOME` の外の置き場に置いた偽の `adb` を `PATH` に足したものと、受けた引数を記録する偽の席の台本
- **WHEN** 画面と携帯の範囲がある確認バッチの木 `t1` で `tools/tsukau/verify-batch.sh t1` を走らせる
- **THEN** rc=0 で、偽の席の台本は呼ばれず、「画面: 」と「携帯: 」の段落は「席を呼べなかった」で始まり、目録の `web` と `android` の `state` は `not_run` である

#### Scenario: 画面の相手には、準備が組んだ置き場を絶対パスで渡す
- **GIVEN** 受けた環境変数を記録する偽の席の台本
- **WHEN** 確認バッチの作業ツリーで `tools/tsukau/verify-batch.sh t1` を走らせる
- **THEN** `ASHIATO_TSUKAU_SERVER_BIN` は `/` で始まり確認バッチの作業ツリーの `dist/verify-t1/ashiato-server` を指し、`ASHIATO_TSUKAU_WEB_DIST` は `/` で始まり `dist/verify-t1/web` を指す

#### Scenario: 携帯の相手にも、準備が組んだサーバと画面の置き場を絶対パスで渡す
- **GIVEN** 携帯の範囲だけがある確認バッチの木と、受けた環境変数を記録する偽の席の台本
- **WHEN** 確認バッチの作業ツリーで `tools/tsukau/verify-batch.sh t1` を走らせる
- **THEN** 偽の席の台本は `--target android` で 1 回だけ呼ばれ、渡った `ASHIATO_TSUKAU_SERVER_BIN` は `/` で始まり `dist/verify-t1/ashiato-server` を、`ASHIATO_TSUKAU_WEB_DIST` は `/` で始まり `dist/verify-t1/web` を指す

#### Scenario: PC の相手には、組んだ配る zip の置き場を絶対パスで渡す
- **GIVEN** PC の範囲がある確認バッチの木と、置き場に zip を 2 つ置いて rc=0 で終わる偽の組み立ての台本と、受けた環境変数を記録する偽の席の台本
- **WHEN** 確認バッチの作業ツリーで `tools/tsukau/verify-batch.sh t1` を走らせる
- **THEN** `--target pc` の呼び出しに渡った `ASHIATO_TSUKAU_PC_ZIPS` は `/` で始まり、確認バッチの作業ツリーの `dist/verify-t1/tsukau-pc` を指す

#### Scenario: harness2 を繋げなければ、席を呼ばずに「席を呼べなかった」と書く
- **GIVEN** rc=1 で終わる偽の `init.py` と、受けた引数を記録する偽の席の台本
- **WHEN** 画面の範囲がある確認バッチの木で `tools/tsukau/verify-batch.sh t1` を走らせる
- **THEN** 偽の席の台本は呼ばれず、画面の段落は「画面: 席を呼べなかった」で始まり、目録の `web` の `state` は `not_run` である

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

#### Scenario: 打ち切った後に専用の AVD のエミュレータが残っていれば、片付けに残りがあると書く
- **GIVEN** 呼ぶ間に `-avd ashiato-tsukau-api35` のエミュレータのプロセスを残したと偽の `ps` に答えさせ、SIGTERM を受けても終わらない偽の席の台本と、秒数を縮めた自己試験と、携帯の範囲がある確認バッチの木
- **WHEN** `tools/tsukau/verify-batch.sh t1` を走らせる
- **THEN** 携帯の段落に「時間の上限で打ち切った」と「片付けに残りがある」がある

#### Scenario: 打ち切りの秒数は、立てる上限・席の時間・片付けの和で決まる
- **WHEN** `tools/tsukau/verify-batch.sh --print-limits` を走らせ、`tools/verify-prep.sh` の席を呼ぶ部分を読む
- **THEN** SIGTERM までは 1860・待ちは 360・組み立ては 2400・全体は 9360 である

#### Scenario: 打ち切った後でも、最後の行が読めて席が終えていれば結果を写す
- **GIVEN** SIGTERM を受けると `"stop":"done"`・`"count":1` の最後の行を出して rc=0 で終わる偽の席の台本と、秒数を縮めた自己試験
- **WHEN** `tools/tsukau/verify-batch.sh t1` を走らせる
- **THEN** 画面の段落は「席が自分で終えた。違和感の候補 1 件」で始まり、目録の `web` の `state` は `ran` である

### Requirement: 確認バッチの準備は、処置へ渡す目録を手元にだけ書き、自分では直さず issue も作らない

WHEN `tools/tsukau/verify-batch.sh <tag>` が相手ごとの処理を終えたとき、
THE SYSTEM SHALL 処置（harness2 の確認バッチ）が読む目録 `dist/verify-<tag>/tsukau.json` を書かせる。形は `{"v": 1, "tag", "targets": [{"target", "state", "scope", "run", "earlier", "recheck"}]}` で、
`targets` は `web`・`pc`・`android` の 3 つをこの順に持ち、`state` は `ran`（席を呼んだ）・`skipped`（範囲が空か、新しく入ったものが無い）・`not_run`（組めない・呼べない・打ち切って最後の行が読めないか席が終えていない）のどれか、
`scope` はこの回に席へ渡した範囲のものの `{"id", "title"}` の配列、`run` は結果のある実行のフォルダの絶対パス（無ければ `null`）、`earlier` は前の回で試したので範囲から外したものの `{"id", "tag", "run"}` の配列（無ければ空）、`recheck` は確かめ直した候補の配列（確かめ直しでなければ空）。
THE SYSTEM SHALL 候補の本文・根拠・止まり方は目録に写させない（実行のフォルダの `result.json`・`candidates.jsonl` が持つ）。目録は git が無視する場所にだけ置く。
THE SYSTEM SHALL 席を呼ぶ台本に、確認バッチの作業ツリーのファイルを直させず、`gh` を呼ばせない（直す・本人に聞く・issue にするのは処置の側）。
導出元: CH21（依頼「見つけた後の処置」「どこまでを ashiato2 に置き、どこからを harness2 の依頼にするか」）。深掘り Q1（第 1 回の推奨: 呼ぶまで ashiato2、処置は harness2 の確認バッチ）・Q11（本人の答え: 新しく入ったものだけを範囲にする）。

#### Scenario: 目録は相手ごとの状態と範囲と実行のフォルダを持つ
- **GIVEN** 画面の範囲に ST12 だけがある確認バッチの木 `t1` と、`"run":"/home/someone/.local/state/ashiato2/tsukau/20261120-010203-web"` を出して rc=0 で終わる偽の席の台本
- **WHEN** `tools/tsukau/verify-batch.sh t1` を走らせる
- **THEN** `dist/verify-t1/tsukau.json` の `targets` は `web`・`pc`・`android` の順で、`web` は `state` が `ran`・`scope` に `{"id":"ST12", …}`・`run` が上の絶対パス・`earlier` が空、`pc` と `android` は `state` が `skipped`・`run` が `null` である

#### Scenario: 目録は git が無視する場所にあり、候補の本文を持たない
- **GIVEN** 候補 1 件（題「並びがおかしい」、根拠「画面の写し a.png」）を `candidates.jsonl` に書き、最後の行を出す偽の席の台本
- **WHEN** `tools/tsukau/verify-batch.sh t1` を走らせる
- **THEN** `git check-ignore dist/verify-t1/tsukau.json` は rc=0 で、目録に「画面の写し a.png」は無い

#### Scenario: 席を呼ぶ台本は、作業ツリーを直さず issue も作らない
- **GIVEN** 受けた引数を記録する偽の `gh` を `PATH` の先に置き、候補 1 件を出す偽の席の台本
- **WHEN** `tools/tsukau/verify-batch.sh t1` を走らせる
- **THEN** 偽の `gh` は 1 回も呼ばれず、確認バッチの作業ツリーの `git status --porcelain` と HEAD は呼ぶ前と同じである（`dist/` は無視される）

### Requirement: 席の結果は、止まり方と件数と伏せた題だけを決まった文言で手順書に写す

THE SYSTEM SHALL 確認バッチの準備に、席の結果を `dist/verify-<tag>/manifest.md` の最後に、頭の 1 段落と相手ごとに 1 段落ずつ足させる（手順書の冒頭と verify PR の本文に載る。手順書は段落ごとに 1 つの note になる）。
頭の段落は「使ってみる席（AI が利用者として使ってみた結果。人間の確認ではない）」、相手の段落は「画面: 」か「PC: 」か「携帯: 」で始まる。
写すのは、席の台本の最後の行の JSON の止まり方（`stop`）・件数（`count`）・片付けに残りがあるか（`cleanup` が「済み」か）と、その実行のフォルダ（最後の行の `run`）の `result.json` の `candidates` の候補ごとの番号（席の台本が振る `id`。`C1` から）と題（公開の題の Requirement の台本を通したもの）と、前の回で試したものの ID と tag だけで、
1 行目のパス・`run`・`cleanup` の文・候補の本文・根拠・画像・ログは写さない。最後の行が JSON として読めなければ「席を呼べなかった」にする。
THE SYSTEM SHALL 止まり方ごとに次の決まった文言を書かせる: `done` は「席が自分で終えた」、`time` は「時間を使い切った」、`tokens` は「token を使い切った」に続けて「。違和感の候補 <件数> 件」と候補の番号つきの題の一覧、
`crashed` は「席が途中で落ちた」、`env_failed` は「相手を立てられなかった」、`interrupted` は「中断した」、席の台本が rc=2 で終わった・最後の行が読めない・台本が無い・harness2 を繋げなかったときは「席を呼べなかった」、
打ち切ったときは、最後の行が読めて `stop` が `done`・`time`・`tokens` ならその文言、そうでなければ「時間の上限で打ち切った」、範囲が空の相手は「回さなかった（この回に<相手>を変えたものが無い）」、前の回で試したものを外して範囲が空になった相手は「回さなかった（この回に新しく入った<相手>のものが無い）」、確かめ直しで項目の無い相手は「回さなかった（直した候補が無い）」、PC の配る zip を組めなかったときは「配る zip を組めず、回さなかった」。
片付けに残りがあれば、その段落に「片付けに残りがある」を足す。確かめ直しの回は「直した候補を確かめ直した」を足す。前の回で試したので範囲から外したものがあれば、「前の回で試した: <ID>（<前の tag>）」を並べる（ID と tag だけ）。
THE SYSTEM SHALL どの相手をどの範囲で呼び、どの実行のフォルダに結果があるかは、手元の目録と `dist/verify-<tag>/tsukau.log`（どちらも git が無視する）にだけ書かせる。
導出元: CH21（依頼「落ちても準備は止めず、警告として手順書に書く」「候補の本文を公開の repo に出さない」）。深掘り Q4（第 1 回の推奨: 題と種類とどの Story だけ）・Q12（止まり方ごとの決まった文言だけ）。

#### Scenario: 席が終えた相手は、件数と番号つきの題を手順書に載せる
- **GIVEN** 1 行目に「使ってみた結果: /home/someone/.local/state/ashiato2/tsukau/20261120-010203-web/RESULT.md」、最後の行に `"stop":"done"`・`"count":2`・`"titles":["更新の確認が赤いまま","設定の言葉が分からない"]`・`"run":"/home/someone/.local/state/ashiato2/tsukau/20261120-010203-web"`・`"cleanup":"済み"` を出し、その `run` の `result.json` の `candidates` に `id` が `C1`・`C2` の 2 つ（題は上の 2 つ）を書く偽の席の台本
- **WHEN** `tools/tsukau/verify-batch.sh t1` を走らせる
- **THEN** 出力は空行で分かれた段落で、頭の段落は「使ってみる席（AI が利用者として使ってみた結果。人間の確認ではない）」で始まり、「画面: 」で始まる段落に「席が自分で終えた。違和感の候補 2 件」と「C1」「C2」に続く 2 つの題がある

#### Scenario: 席が終えた相手の段落に、手元のパスは出ない
- **GIVEN** 上と同じ偽の席の台本
- **WHEN** `tools/tsukau/verify-batch.sh t1` を走らせる
- **THEN** 出力に `/home/`・`RESULT.md`・`.local/state` は無い

#### Scenario: 止まり方ごとに決まった文言だけを書く
- **GIVEN** 最後の行の `stop` が `time`・`tokens`・`crashed`・`env_failed`・`interrupted` のそれぞれで、`cleanup` に「残りあり（down rc=1。logs/env-down.log）」を出す偽の席の台本と、rc=2 で終わり何も出さない偽の席の台本と、最後の行が JSON でない偽の席の台本
- **WHEN** それぞれで `tools/tsukau/verify-batch.sh t1` を走らせる
- **THEN** 画面の段落はそれぞれ「画面: 」に続けて「時間を使い切った」「token を使い切った」「席が途中で落ちた」「相手を立てられなかった」「中断した」「席を呼べなかった」「席を呼べなかった」で始まり、`cleanup` を出した 5 つには「片付けに残りがある」がある
- **AND** どの出力にも `logs/env-down.log` と `down rc=1` は無い

#### Scenario: 携帯の段落は「携帯: 」で始まり、画面・PC・携帯の順に並ぶ
- **GIVEN** 画面と携帯の範囲がある確認バッチの木と、どの相手でも `"stop":"done"`・`"count":0` の最後の行を出す偽の席の台本
- **WHEN** `tools/tsukau/verify-batch.sh t1` を走らせる
- **THEN** 頭の段落の後に「画面: 」「PC: 」「携帯: 」で始まる段落がこの順にあり、「携帯: 」の段落は「席が自分で終えた。違和感の候補 0 件」で始まる

#### Scenario: 範囲が空の相手は、回さなかったと書く
- **GIVEN** 文書だけを変えた merge だけを重ねた確認バッチの木
- **WHEN** `tools/tsukau/verify-batch.sh t1` を走らせる
- **THEN** 席の台本は呼ばれず、段落に「回さなかった（この回に画面を変えたものが無い）」と「回さなかった（この回に PC を変えたものが無い）」と「回さなかった（この回に携帯を変えたものが無い）」がある

### Requirement: 公開の repo に出す題は、1 つの台本で私的な形と commit を止める形を伏せ、1 行に整える

THE SYSTEM SHALL 題を公開の repo に出す前に通す台本 `tools/tsukau/public-title.sh` を持たせる。標準入力の題 1 行ずつに、標準出力へ同じ数の行を返す。手順書の段落も、処置（harness2）が公開の repo に書く題も、これを通す。
THE SYSTEM SHALL 題に次のどれかがあれば、その題を「（題は手元の結果に）」に置き換えさせる:
この PC の利用者名・ホスト名・私設網の名前、確認バッチの作業ツリーの `.env` の 8 字以上の値、ホームや絶対パスの形（`/home/`・`/mnt/`・`/Users/`・`C:\`・`\\` で始まる語）、メールの形、IPv4 の形、
確認バッチの手順書の commit の検査（`tools/check-private.sh`・`tools/check-secrets.sh`）が止める形。
検査は台本を持つ使い捨ての作業ツリーに題を置き、一時の index で stage して当てる（確認バッチの作業ツリーには何も置かない）。検査を走らせられない（`tools/check-secrets.sh` が gitleaks を用意できない など）ときは、全部の題を伏せる。
THE SYSTEM SHALL 伏せなかった題は、改行を空白にし、80 字で切り、`@` を「＠」に、`#` に続く数字の `#` を「＃」に替えてから返させる（公開の repo で人への通知や issue への繋がりにしない）。
導出元: CH21（依頼「候補の本文を公開の repo に出さない決まりは残す。issue にするときは私的な値を伏せる」）。深掘り Q4（第 1 回の推奨）。review/deep.md R1・R14・R15。

#### Scenario: 私的な形を含む題は伏せる
- **GIVEN** 題が「/home/someone/x が開けない」「/mnt/c/x で止まる」「/Users/a/x が無い」「\\srv\share が見えない」「C:\Users\a で落ちる」「a@example.com に送れない」「並びがおかしい」の 7 つと、この PC のホスト名を含む題 1 つ
- **WHEN** `tools/tsukau/public-title.sh` に通す
- **THEN** 出力は 8 行で、「（題は手元の結果に）」が 7 つと「並びがおかしい」で、ホスト名・`/home/someone`・`/mnt/c`・`/Users/a`・`\\srv`・`C:\Users\a`・`a@example.com` は出力に無い

#### Scenario: 利用者名と .env の値を含む題は伏せる
- **GIVEN** cwd の木の `.env` に `WEB_PASSWORD=honnin-no-aikotoba` があり、題が「honnin-no-aikotoba で入れない」と、この PC の利用者名を含む題と、「並びがおかしい」の 3 つ
- **WHEN** `tools/tsukau/public-title.sh` に通す
- **THEN** 出力は 3 行で、「（題は手元の結果に）」が 2 つと「並びがおかしい」で、`honnin-no-aikotoba` と利用者名は出力に無い

#### Scenario: 検査を走らせられなければ、全部の題を伏せる
- **GIVEN** gitleaks の置き場を無い場所に向け、取りに行けなくした自己試験と、題「並びがおかしい」「設定の言葉が分からない」
- **WHEN** `tools/tsukau/public-title.sh` に通す
- **THEN** 出力は 2 行とも「（題は手元の結果に）」である

#### Scenario: 伏せた後の題は、確認バッチの commit の検査を通る
- **GIVEN** 私設網の IPv4 の形（自己試験が数から組んだもの）を含む題、私設網のホスト名の形を含む題、秘密の形を含む題、ふつうの題 1 つ
- **WHEN** `tools/tsukau/public-title.sh` の出力を、台本を持つ使い捨ての作業ツリーのファイルに置いて一時の index で stage し、`tools/check-private.sh --staged` と `tools/check-secrets.sh --staged` を走らせる
- **THEN** どちらも rc=0 で、出力は 4 行、「（題は手元の結果に）」は 3 つである

#### Scenario: 題は 1 行に整え、通知の形を替える
- **GIVEN** 題「@media の指定が 効かない #12 と同じ」と、100 字の題 1 つ
- **WHEN** `tools/tsukau/public-title.sh` に通す
- **THEN** 出力は「＠media の指定が 効かない ＃12 と同じ」と、元の題の頭 80 字の 2 行で、出力に `@` と `#12` は無い

#### Scenario: 席の段落の題は、公開の題の台本を通したものである
- **GIVEN** 題「/home/someone/x が開けない」と「並びがおかしい」を出す偽の席の台本
- **WHEN** `tools/tsukau/verify-batch.sh t1` を走らせる
- **THEN** 画面の段落は「違和感の候補 2 件」で、題は「（題は手元の結果に）」と「並びがおかしい」で、`/home/someone` は出力に無い

### Requirement: 席が落ちても時間切れでも、確認バッチの準備は止まらず、今の成果物を変えない

THE SYSTEM SHALL 席の呼び出し（`tools/tsukau/verify-batch.sh`）がどんな終わり方をしても（非 0・何も出さない・台本が無い・終わらない）、確認バッチの準備（`tools/verify-prep.sh`）を rc=0 のまま続けさせ、そのときは手順書に「使ってみる席: 呼べなかった」で始まる 1 段落を足させる。
THE SYSTEM SHALL 確認バッチの準備に、席の呼び出し全体を 9,360 秒（組み立ての 2,400 秒 + 3 つの相手の 2,220 秒ずつ + 300 秒）で打ち切らせる。
THE SYSTEM SHALL 席を呼んでも、確認バッチの準備が作る成果物（`ashiato-server`・`web/`・`app-debug.apk`・`run.sh`）と、手順書の既存の段落を変えさせない。席の段落は既存の段落の後に足すだけにする。
導出元: CH21（依頼「席が落ちても・時間切れでも、確認バッチの準備は止めない」）。深掘り Q13（第 1 回の推奨: 守るもの）。

#### Scenario: 席の呼び出しが落ちても、準備の rc は 0 のまま
- **GIVEN** rc=1 で終わり何も出さない偽の `tools/tsukau/verify-batch.sh` と、中身のある仮の `manifest.md`
- **WHEN** 確認バッチの準備の、席を呼ぶ部分を `set -euo pipefail` の下で走らせる
- **THEN** rc=0 で、`manifest.md` の頭は元の中身のままで、その後に「使ってみる席: 呼べなかった」で始まる 1 段落がある

#### Scenario: 席の呼び出しが終わらなくても、上限で打ち切って準備を続ける
- **GIVEN** 終わらない偽の `tools/tsukau/verify-batch.sh` と、上限を 2 秒に縮めた自己試験
- **WHEN** 確認バッチの準備の、席を呼ぶ部分を `set -euo pipefail` の下で走らせる
- **THEN** 10 秒以内に rc=0 で終わり、`manifest.md` に「使ってみる席: 呼べなかった」で始まる段落がある

#### Scenario: 準備の席の部分の上限は、席の呼び出しの全体の上限と同じ
- **WHEN** `tools/verify-prep.sh` の席を呼ぶ部分と `tools/tsukau/verify-batch.sh --print-limits` を読む
- **THEN** 席の部分の既定の上限は 9360 で、`--print-limits` の全体の値と同じである

#### Scenario: 席の段落は、既存の段落の後に足すだけ
- **WHEN** 確認バッチの準備の台本を読む
- **THEN** 席を呼ぶ部分は `manifest.md` を書く部分より後にあり、`manifest.md` へは追記（`>>`）だけで書き、`run.sh`・`ashiato-server`・`web/`・`app-debug.apk` を書く行を持たない

### Requirement: 確認バッチを作り直したときは、前の回で試していない新しく入ったものだけを範囲にする

WHEN 確認バッチの準備が相手の範囲を決めたとき（確かめ直しのときを除く）、
THE SYSTEM SHALL 範囲に入ったものごとに、相手・ID・その PR のいちばん新しいコミット（その merge の 2 つ目の親）の組を、`out_root` の下の試した記録と突き合わせ、同じ組で席が終えた（止まり方が `done`・`time`・`tokens`）記録があるものを範囲から外させる。外した後の範囲が空なら、その相手の席を呼ばない。
`origin/main` が進んだだけでは、試したものを範囲に戻さない。PR のいちばん新しいコミットが変わったものは、新しく入ったものとして範囲に入れる。
THE SYSTEM SHALL 席が終えたら（最後の行の `stop` が `done`・`time`・`tokens`）、席に渡した範囲のものの組を、回の tag と実行のフォルダとともに試した記録に足させる。それ以外の止まり方・最後の行が読めない打ち切りでは足さない（次の回に試し直す）。
THE SYSTEM SHALL 範囲から外したものを、その相手の段落に「前の回で試した: <ID>（<前の tag>）」の形で並べさせ、目録のその相手の `earlier` に `{"id", "tag", "run"}` を書かせる。外した後の範囲が空の相手は、段落を「回さなかった（この回に新しく入った<相手>のものが無い）」で始め、目録の `state` を `skipped` にする。
導出元: CH21（依頼「毎回」）。深掘り Q11（本人の答え: 新しく入ったものだけを範囲にする）。design D10・D21（仮）。

#### Scenario: 前の回で試したものは外し、新しく入ったものだけを範囲にする
- **GIVEN** 確認バッチ `t1` で画面の範囲 ST12 の席が `done` で終わった後、同じ PR のコミットの ST12 と、新しい `web/src/B.tsx` を変えた `feat/st13-b` を重ねた確認バッチ `t2`
- **WHEN** `tools/tsukau/verify-batch.sh t2` を走らせる
- **THEN** 偽の席の台本は `--target web` で 1 回だけ呼ばれ、`--scope` は ST13 の行だけで ST12 の行が無く、画面の段落に「前の回で試した: ST12（t1）」があり、目録の `web` の `earlier` に `"id": "ST12"`・`"tag": "t1"` がある

#### Scenario: 目録の earlier の run は、前の回で席を呼んだ実行のフォルダである
- **GIVEN** 確認バッチ `t1` で画面の範囲 ST12 の席が `done` で終わり、`t1` の目録の `web` の `run` が `/home/someone/.local/state/ashiato2/tsukau/20261120-010203-web` だった後、同じ PR のコミットの ST12 を重ねた確認バッチ `t2`
- **WHEN** `tools/tsukau/verify-batch.sh t2` を走らせる
- **THEN** `t2` の目録の `web` の `earlier` の ST12 の `run` は、`t1` の目録の `web` の `run` と同じ絶対パスである

#### Scenario: 新しく入ったものが無ければ、呼ばずに回さなかったと書く
- **GIVEN** 確認バッチ `t1` で画面の範囲 ST12 の席が `done`・候補 1 件で終わった後、同じ `origin/main` と同じ PR のコミットで merge を作り直した確認バッチ `t2`
- **WHEN** `tools/tsukau/verify-batch.sh t2` を走らせる
- **THEN** 席の台本は呼ばれず、画面の段落は「回さなかった（この回に新しく入った画面のものが無い）」で始まって「前の回で試した: ST12（t1）」があり、目録の `web` は `state` が `skipped`・`earlier` に `"tag": "t1"` がある

#### Scenario: main が進んだだけでは、呼び直さない
- **GIVEN** `t1` の後に、`origin/main` だけを進め、画面の範囲は同じ PR のコミットの ST12 だけの確認バッチ `t3`
- **WHEN** `tools/tsukau/verify-batch.sh t3` を走らせる
- **THEN** 席の台本は呼ばれない

#### Scenario: PR のコミットが変われば、新しく入ったものとして呼び直す
- **GIVEN** `t1` の後に、ST12 の PR のコミットだけを進めた確認バッチ `t4`
- **WHEN** `tools/tsukau/verify-batch.sh t4` を走らせる
- **THEN** 画面の席の台本が 1 回呼ばれ、`--scope` に ST12 の行がある

#### Scenario: 前の回が落ちていたら、試していないものとして呼び直す
- **GIVEN** `t1` で画面の席が `env_failed` で終わった後、同じ中身の確認バッチ `t2`
- **WHEN** `tools/tsukau/verify-batch.sh t2` を走らせる
- **THEN** 画面の席の台本が 1 回呼ばれ、`--scope` に ST12 の行がある

### Requirement: 直した後の確かめ直しは、直した候補だけを範囲に、半分の大きさで呼ぶ

WHEN 処置が `TSUKAU_RECHECK=<ファイル>`（`{"items": [{"target", "run", "id"}]}`。`id` は席の台本が振る `C<n>`）を渡して確認バッチの準備を走らせたとき、
THE SYSTEM SHALL 相手ごとに、その相手の項目の候補の題（`run` の下の `result.json` の `candidates` から `id` で引き、公開の題の台本は通さない —— 範囲は手元にだけある）を「直した違和感が直ったか: <題>」の行で並べたものだけを範囲にして、`--minutes 5 --tokens 100000` で席を呼ばせる。項目の無い相手は呼ばず、段落を「回さなかった（直した候補が無い）」、目録の `state` を `skipped` にする。
THE SYSTEM SHALL 確かめ直しでは試した記録を読ませず（範囲から外さない）、足させもせず（直しで進んだ PR のコミットは、次のふだんの回に新しく入ったものとして試す）、確かめ直しの範囲のほかは呼ばせない。目録のその相手の `recheck` に項目（前の `run` と番号）を書かせ、段落に「直した候補を確かめ直した」を足させる。
導出元: CH21（依頼「直した後に席でもう一度確かめる」）。深掘り Q10（第 1 回の推奨: 直した候補だけを範囲に上限の半分で）・Q11（本人の答え: 新しく入ったものだけを範囲にする）。review/deep.md R5。design D21（仮）。

#### Scenario: 確かめ直しは、直した候補の題だけを範囲に、5 分と 10 万で呼ぶ
- **GIVEN** 前の実行のフォルダの `candidates.jsonl` が、壊れた 1 行の後に `id` が `C1`（題「設定の言葉が分からない」）と `C2`（題「並びがおかしい」）の 2 行を持ち、`result.json` の `candidates` もその 2 つで、`{"items":[{"target":"web","run":<そのフォルダ>,"id":"C2"}]}` を書いた `TSUKAU_RECHECK` のファイルと、受けた引数を記録する偽の席の台本
- **WHEN** `TSUKAU_RECHECK` を渡して `tools/tsukau/verify-batch.sh t5` を走らせる
- **THEN** 偽の席の台本は `--target web` で 1 回だけ呼ばれ、引数に `--minutes 5`・`--tokens 100000` があり、`--scope` は「直した違和感が直ったか: 並びがおかしい」だけである

#### Scenario: 確かめ直しは、試した記録で外さず足しもせず、目録に直した候補を書く
- **GIVEN** 上と同じ `TSUKAU_RECHECK` と、画面の範囲の ST12 を同じ PR のコミットで試した記録（`done`）が `out_root` にある木
- **WHEN** `TSUKAU_RECHECK` を渡して `tools/tsukau/verify-batch.sh t5` を走らせる
- **THEN** 偽の席の台本は 1 回だけ呼ばれ、目録の `web` の `recheck` に `"id": "C2"` があり、画面の段落に「直した候補を確かめ直した」があり、試した記録の行の数は走らせる前と同じである

#### Scenario: 確かめ直しで項目の無い相手は、回さなかったと書く
- **GIVEN** `web` の項目だけを書いた `TSUKAU_RECHECK` と、画面と PC の両方の範囲がある確認バッチの木
- **WHEN** `TSUKAU_RECHECK` を渡して `tools/tsukau/verify-batch.sh t5` を走らせる
- **THEN** 偽の席の台本は `--target pc` で呼ばれず、PC の段落は「回さなかった（直した候補が無い）」、目録の `pc` の `state` は `skipped` である

### Requirement: PC の相手は、私設網と手元の網を塞いで外にはつないだまっさらな Windows Sandbox に配る形を入れて立て、自分が起こした Sandbox だけを、中のログを写してから片付ける

WHEN 差し込み口が PC の相手を立てるとき（`env-hook.sh up <repo> <state> pc`）、
THE SYSTEM SHALL 起こす前に別の Windows Sandbox が動いていれば立てずに `UNAVAILABLE=` を置かせる（待たない）。
THE SYSTEM SHALL 配る形のサーバの zip と PC の収集アプリの zip が、`ASHIATO_TSUKAU_PC_ZIPS` の置き場にも `dist/tsukau-pc/<HEAD のコミット>/` にも無ければ、立てずに `UNAVAILABLE=` を置かせる（理由に `tools/tsukau/build-pc.sh` の名前）。
THE SYSTEM SHALL そうでなければ、網をつないだ（`.wsb` に `<Networking>Disable</Networking>` を書かない）Sandbox を 1 つ起こし、その id を `<state>` に残し、何かを入れる前に、Sandbox の中のファイアウォールで外向きの既定を「塞ぐ」にし、外の宛先と、Sandbox の既定のゲートウェイへの名前の問い合わせ（53 番）だけを通させる。
通す外の宛先は `env-hook.sh` が数から組み（`env-hook.sh --print-net-rules` で出せる）、私設網（tailscale の IPv4 の範囲と IPv6 の範囲）・3 つの私用の範囲・リンクローカル（IPv4 と IPv6）・IPv6 の私用の範囲・マルチキャストを含めない。
THE SYSTEM SHALL 確かめは対照つきで行わせる: その回だけ待ち受ける一時の口を、この PC の私設網の宛先（IPv4 と、あれば IPv6）と手元の網の宛先（Windows 側のこの PC のアドレス）に立て、塞ぐ前に Sandbox の中からその口に届く（つながる）ことを、塞いだ後に同じ口が時間切れになることを見る。外は `api.github.com` を引いて HTTPS で届くかを見る。
一時の口を立てられない（この PC の私設網の宛先を求められない・待ち受けを立てられない）か、塞ぐ前に一時の口に届かなければ、確かめられないとして何も入れずに `UNAVAILABLE=` を置かせる（理由に口の名前）。塞いだ後に私設網か手元の網の口に届いたら、外の宛先を通す規則も外して外向きを全部塞ぎ、同じ口で確かめ直す。それでも届けば、何も入れずに `UNAVAILABLE=` を置かせる。外にだけ届かなかったときは続け、案内と `INFO_NET=` にそう書く。一時の口は確かめの後に閉じる。
THE SYSTEM SHALL 確かめた後に、配る形のサーバの zip を、その場で作った乱数の合言葉でインストーラで入れ（`--repo` と確認バッチの作業ツリーの `.env` は読まない）、サーバが応えたら Sandbox の窓を本人のデスクトップに開き（`wsb connect`）、その後に配る形の PC の収集アプリを利用者のセッション（`ExistingLogin`）で起こしてから、案内（`GUIDE=`）を置いた `ready` を置かせる。
案内には、Sandbox の中でコマンドを走らせて出力を受け取る口と画面の写しを撮る口（`tools/tsukau/pc.sh`）、サービスとログの場所、網の様子（外にはつながり、私設網と手元の網は塞いである。全部塞いだ回・外に届かなかった回はそう書く）、
触らないもの（本人のサーバ・本人の PC の配布版 `C:\ProgramData\ashiato`・確認バッチの DB・main の checkout の `.env`・本人が起こした Sandbox・ファイアウォールの規則）を書く。
THE SYSTEM SHALL 片付け（`env-hook.sh down <repo> <state> pc`）では、Sandbox の中のサービスのログ・更新係の置き場の `checks.jsonl` とログ・PC の収集アプリのログ・サービスに関わるシステムのイベントの抜き書きを `$TSUKAU_EVIDENCE/pc-logs/` へ写してから、`<state>` に残した id の Sandbox だけを止めさせる。
止めた後もその id が動いていれば rc≠0 で終わらせる。ほかの Sandbox には触らせない。
導出元: CH21（依頼「PC（Windows Sandbox に配る zip を入れる）」）。深掘り Q7（第 1 回の推奨: 確認バッチの木から手元で組む）・Q8（本人の答え: 外にはつなぎ、私設網と手元の網を塞ぐ。選んだ案の「塞げたことを確かめてから渡す」と、塞げなければ網を切る側に倒す）・Q9（窓を開いて画面も触る）。review/deep.md R4。review/spec.md R22・R23。design D6・D26（仮）・D29（仮）。

#### Scenario: 別の Sandbox が動いていれば、立てずに断る
- **GIVEN** 動いている Sandbox の id を 1 つ返す偽の `wsb`
- **WHEN** `tools/tsukau/env-hook.sh up <repo> <state> pc` を走らせる
- **THEN** `<state>/ready` に `UNAVAILABLE=` の行があり、`wsb start` は呼ばれない

#### Scenario: 配る zip が無ければ、立てずに断る
- **GIVEN** 動いている Sandbox が無い偽の `wsb` と、空の配る zip の置き場
- **WHEN** `tools/tsukau/env-hook.sh up <repo> <state> pc` を走らせる
- **THEN** `<state>/ready` に `UNAVAILABLE=` の行があり、その理由に `tools/tsukau/build-pc.sh` の名前がある。`wsb start` は呼ばれない

#### Scenario: Sandbox は網をつないで起こす
- **GIVEN** 動いている Sandbox が無く、`start` で id `sb-1` を返し、受けた設定を記録し、塞ぐ前の確かめに「一時の口に届く」、塞いだ後の確かめに「私設網に届かない・手元の網に届かない・外に届く」と答える偽の `wsb` と、配る zip の置き場
- **WHEN** `tools/tsukau/env-hook.sh up <repo> <state> pc` を走らせる
- **THEN** `wsb start` は 1 回だけ呼ばれ、渡した設定に `<Networking>Disable</Networking>` は無い

#### Scenario: 何かを入れる前に、塞いで確かめる
- **GIVEN** 上と同じ偽の `wsb`（`exec` で走らせた台本の名前を記録する）と配る zip の置き場
- **WHEN** `tools/tsukau/env-hook.sh up <repo> <state> pc` を走らせる
- **THEN** 塞ぐ前の確かめ・塞ぐ台本・塞いだ後の確かめの `exec` がこの順に、サーバのインストールの `exec` より先に呼ばれる

#### Scenario: 通す外の宛先は、私設網と手元の網の範囲を含まない
- **GIVEN** 自己試験が数から組んだ見本の宛先: tailscale の IPv4 の範囲の 1 つ・IPv6 の範囲の 1 つ・3 つの私用の範囲の 1 つずつ・IPv4 のリンクローカル・IPv6 の私用の範囲の 1 つ・IPv6 のリンクローカル・マルチキャスト・外の宛先 1 つ
- **WHEN** `tools/tsukau/env-hook.sh --print-net-rules` の出力の範囲に、見本を 1 つずつ当てる
- **THEN** 外の宛先だけがどれかの範囲に入り、ほかの見本はどの範囲にも入らない

#### Scenario: 一時の口を立てられなければ、何も入れずに断る
- **GIVEN** この PC の私設網の宛先を返さない偽の `tailscale` と、配る zip の置き場と、受けた引数を記録する偽の `wsb`
- **WHEN** `tools/tsukau/env-hook.sh up <repo> <state> pc` を走らせる
- **THEN** `<state>/ready` に `UNAVAILABLE=` の行があり、その理由に一時の口の名前があり、サーバのインストールの `exec` も `connect` も呼ばれない

#### Scenario: 塞ぐ前に一時の口に届かなければ、確かめられないとして何も入れずに断る
- **GIVEN** 塞ぐ前の確かめに「一時の口に届かない」と答える偽の `wsb` と配る zip の置き場
- **WHEN** `tools/tsukau/env-hook.sh up <repo> <state> pc` を走らせる
- **THEN** `<state>/ready` に `UNAVAILABLE=` の行があり、サーバのインストールの `exec` も `connect` も呼ばれない

#### Scenario: 私設網か手元の網に届いたら、外向きを全部塞いで確かめ直す
- **GIVEN** 塞いだ後の 1 回目の確かめに「私設網に届く」、全部塞いだ後の確かめに「どこにも届かない」と答える偽の `wsb` と配る zip の置き場
- **WHEN** `tools/tsukau/env-hook.sh up <repo> <state> pc` を走らせる
- **THEN** 全部塞ぐ台本の `exec` がサーバのインストールの `exec` より先に呼ばれ、`ready` に `UNAVAILABLE=` は無く、案内に「外向きを全部塞いだ」がある

#### Scenario: 全部塞いでも届くなら、何も入れずに断る
- **GIVEN** 塞いだ後のどの確かめにも「私設網に届く」と答える偽の `wsb` と配る zip の置き場
- **WHEN** `tools/tsukau/env-hook.sh up <repo> <state> pc` を走らせる
- **THEN** `<state>/ready` に `UNAVAILABLE=` の行があり、サーバのインストールの `exec` も `connect` も呼ばれない

#### Scenario: 外にだけ届かなければ、続けて案内に書く
- **GIVEN** 塞いだ後の確かめに「私設網に届かない・手元の網に届かない・外に届かない」と答える偽の `wsb` と配る zip の置き場
- **WHEN** `tools/tsukau/env-hook.sh up <repo> <state> pc` を走らせる
- **THEN** サーバのインストールの `exec` が呼ばれ、`ready` に `UNAVAILABLE=` は無く、`INFO_NET=` と案内に「外に届かなかった」がある

#### Scenario: 立てるときは、塞いで確かめ、サーバを入れ、窓を開き、利用者のセッションで PC の収集アプリを起こしてから案内を置く
- **GIVEN** 塞ぐ前の確かめに「一時の口に届く」、塞いだ後の確かめに「私設網に届かない・手元の網に届かない・外に届く」と答える偽の `wsb` と配る zip の置き場
- **WHEN** `tools/tsukau/env-hook.sh up <repo> <state> pc` を走らせる
- **THEN** 偽の `wsb` は `start`・塞ぐ前の確かめの `exec`・塞ぐ台本の `exec`・塞いだ後の確かめの `exec`・サーバのインストールの `exec`・`connect --id sb-1`・`-r ExistingLogin` で PC の収集アプリを起こす `exec` の順に呼ばれ、その後に `<state>/ready` が置かれる

#### Scenario: PC の相手は、本人の合言葉でなく、その場の合言葉で入れる
- **GIVEN** `--repo` の作業ツリーに `WEB_PASSWORD=honnin-no-aikotoba` を持つ `.env` と、`exec` が受けた引数と標準入力を記録する偽の `wsb` と配る zip の置き場
- **WHEN** `tools/tsukau/env-hook.sh up <repo> <state> pc` を走らせる
- **THEN** 偽の `wsb` の記録と案内のどこにも `honnin-no-aikotoba` は無い

#### Scenario: Sandbox の中のコマンドは、利用者のセッションで走り、出力が返る
- **GIVEN** `<state>` に id `sb-1` が残り、`exec` で共有のフォルダに「hello」を書く偽の `wsb`
- **WHEN** `tools/tsukau/pc.sh exec 'Write-Output hello'` を走らせる
- **THEN** 偽の `wsb` は `exec --id sb-1 -r ExistingLogin` で呼ばれ、`pc.sh` の標準出力は「hello」である

#### Scenario: PC の案内は、口と網と触らないものを書く
- **GIVEN** 塞ぐ前の確かめに「一時の口に届く」、塞いだ後の確かめに「私設網に届かない・手元の網に届かない・外に届く」と答える偽の `wsb` と配る zip の置き場
- **WHEN** `tools/tsukau/env-hook.sh up <repo> <state> pc` を走らせる
- **THEN** `ready` の `GUIDE=` の案内に `tools/tsukau/pc.sh exec`・`tools/tsukau/pc.sh shot`・画面の操作の仕方（`exec` で UI Automation を使う）・「私設網と手元の網は塞いで」・`C:\ProgramData\ashiato`・`.env`・「触らない」がある

#### Scenario: 片付けは、中のログを写してから自分の Sandbox だけを止める
- **GIVEN** `<state>` に id `sb-1` が残り、動いている Sandbox として `sb-1` と `sb-other` を返す偽の `wsb`
- **WHEN** `tools/tsukau/env-hook.sh down <repo> <state> pc` を走らせる
- **THEN** ログを共有のフォルダへ写す `exec --id sb-1` が `stop --id sb-1` より先に呼ばれ、`$TSUKAU_EVIDENCE/pc-logs/` に写したファイルがある

#### Scenario: 片付けは、自分の Sandbox だけを止める
- **GIVEN** 上と同じ `<state>` と偽の `wsb`
- **WHEN** `tools/tsukau/env-hook.sh down <repo> <state> pc` を走らせる
- **THEN** `stop` は `sb-1` にだけ呼ばれ、`sb-other` への呼び出しは 1 つも無い

#### Scenario: 止めた後も自分の Sandbox が動いていれば、片付けは非 0 で終わる
- **GIVEN** `stop` の後も `sb-1` を動いていると返す偽の `wsb`
- **WHEN** `tools/tsukau/env-hook.sh down <repo> <state> pc` を走らせる
- **THEN** rc≠0 で終わる

#### Scenario: 本物の Sandbox で、私設網と手元の網を塞いだまま配る形が入り、片付けで残らない
- **WHEN** この PC で、組んだ配る zip を置いて `tools/tsukau/env-hook.sh up` と `down` を PC の相手で本物の Sandbox に対して走らせる
- **THEN** `ready` に `UNAVAILABLE=` が無く、`INFO_NET=` は「塞いだ」で、塞ぐ前には届いたこの PC の私設網と手元の網の一時の口に、塞いだ後の Sandbox の中から届かず、`api.github.com` の名前が引けて HTTPS で届き、Sandbox の中のサーバが loopback で応え、PC の収集アプリのプロセスがあり、`down` は rc=0 で、終わった後に動いている Sandbox は無い

### Requirement: 携帯の相手は、専用のエミュレータを網を切って起こし、その回のサーバ向けに組んだ APK を入れて立て、自分が起こしたエミュレータだけを、中のログを写してから片付ける

WHEN 差し込み口が携帯の相手を立てるとき（`env-hook.sh up <repo> <state> android`）、
THE SYSTEM SHALL `/dev/kvm` を読み書きで開けない・エミュレータか SDK か JDK が無い・専用の AVD（`ashiato-tsukau-api35`）のエミュレータがもう動いている（エミュレータのプロセスの引数で見る。ほかの端末に `adb` で問わない）、のどれかなら、何も起こさずに `UNAVAILABLE=` を置かせる（待たない）。
THE SYSTEM SHALL そうでなければ、画面の相手と同じ形で `<state>/tree` にその回の専用の環境（`ashiato-tsukau-` で始まる compose project・空いている port・乱数の合言葉・偽データ）を立て、その木で、接続先を端末の中の `http://127.0.0.1:18787`、資格情報（`ashiato.apiToken`・`ashiato.userId`）をその回の環境のものにした debug の APK を組ませる。
接続先と資格情報は組み立ての引数（`-P`）で渡し、`~/.gradle/gradle.properties` の値に負けないようにする。`--repo` の作業ツリーの `.env`・`~/.gradle/gradle.properties` の接続先と資格情報・配る APK の署名の鍵は使わない。
THE SYSTEM SHALL 専用の AVD のエミュレータを、空いている port で、画面を出さず、前のデータを消して 1 つ起こし、その serial を `<state>` に残させる。起動を待ってから、端末の中から、エミュレータがホストの loopback に付ける別名の宛先のその回のサーバの port に届くことを見て（対照。外へは出ない）、端末の網（Wi-Fi と携帯の回線）を切り、端末に既定のネットワークが無いこと（`dumpsys connectivity` の「Active default network」が `none`。外へは問わない）と、同じ別名の宛先に届かないことを確かめ、`adb reverse` で端末の中の 18787 をその回のサーバの API の port へ渡し、APK を入れて起こしてから、案内を置いた `ready` を置かせる。切る前に届かなかった・切った後も届いた、のどちらかなら、確かめられないとして APK を入れずに `UNAVAILABLE=` を置かせる。
THE SYSTEM SHALL `adb` の操作を、端末の一覧（`adb devices`）のほかはどれも残した serial の名指し（`-s`）で行わせ、実機と、ほかのエミュレータ（本人のもの・計測テストのもの）には問いも入れも止めもさせない。
THE SYSTEM SHALL 差し込み口と `tools/tsukau/phone.sh` に、`adb`・`emulator`・`avdmanager` を `PATH` の名前でなく `$ANDROID_HOME` の下の置き場から見つけさせ、受けた `PATH` に `adb` が無くても端末を操作させる（席の `PATH` には `adb` が無い）。`phone.sh` には `<state>` に残した serial のほかを名指させない。
案内には、端末を操作する口（`tools/tsukau/phone.sh` の `shell`・`shot`・`ui`）と、席の `PATH` に `adb` は無く端末は `phone.sh` だけで触ること、その回の画面の URL と合言葉、サーバのログの場所、網の様子（端末の網は切ってあり、サーバへは `adb reverse` の道だけで届く）、触らないもの（本人の携帯・本人のサーバ・確認バッチの DB・main の checkout の `.env`・ほかのエミュレータ・端末の網の設定）を書く。
THE SYSTEM SHALL 片付け（`env-hook.sh down <repo> <state> android`）では、端末のログ（`logcat`）を `$TSUKAU_EVIDENCE/phone-logs/` へ写してから、残した serial のエミュレータだけを止め、その回の compose project と `<state>/tree`（APK を組むときに書いた合言葉を含む）を消させる。止めた後もその serial のエミュレータが残っていれば rc≠0 で終わらせる。
導出元: CH21（仕様への違和感（第 1 回の合意）「スマホも操作するようにしてください。エミュレータとかで」）。design D22（仮）・D30（仮）・D31（仮）・D25・D26（仮）。review/spec.md R20〜R27。`$ANDROID_HOME` から道具を見つけるのと案内の「`phone.sh` だけ」は IMP-13 の本人の答え（第 2 回の合意。design D32）。

#### Scenario: KVM を開けなければ、何も起こさずに断る
- **GIVEN** 読み書きで開けない置き場を `/dev/kvm` の代わりに向けた自己試験と、受けた引数を記録する偽の `emulator`・`adb`・起動の台本
- **WHEN** `tools/tsukau/env-hook.sh up <repo> <state> android` を走らせる
- **THEN** `<state>/ready` に `UNAVAILABLE=` の行があり、偽の `emulator` と起動の台本は呼ばれない

#### Scenario: エミュレータか SDK か JDK が無ければ、何も起こさずに断る
- **GIVEN** `emulator`・`adb`・`java` のどれか 1 つを無い場所に向けた自己試験（3 通り）と、受けた引数を記録する起動の台本
- **WHEN** それぞれで `tools/tsukau/env-hook.sh up <repo> <state> android` を走らせる
- **THEN** 3 通りとも `<state>/ready` に `UNAVAILABLE=` の行があり、起動の台本は呼ばれない

#### Scenario: 専用の AVD のエミュレータが動いていれば、立てずに断る
- **GIVEN** `-avd ashiato-tsukau-api35` を引数に持つエミュレータのプロセスを返す偽の `ps` と、受けた引数を記録する偽の `adb`・`emulator`
- **WHEN** `tools/tsukau/env-hook.sh up <repo> <state> android` を走らせる
- **THEN** `<state>/ready` に `UNAVAILABLE=` の行があり、偽の `emulator` は呼ばれず、偽の `adb` に `-s` の呼び出しは無い

#### Scenario: 携帯の相手の APK は、<state>/tree で組む
- **GIVEN** 組み立ての場所と引数を記録する偽の `gradlew`
- **WHEN** `tools/tsukau/env-hook.sh up <repo> <state> android` を走らせる
- **THEN** 偽の `gradlew` は `<state>/tree` の中で `:app:assembleDebug` を組み、`--repo` の `git status --porcelain` は呼ぶ前と同じである

#### Scenario: 携帯の相手の APK の接続先は、端末の中のその回のサーバである
- **GIVEN** 上と同じ偽の `gradlew`
- **WHEN** `tools/tsukau/env-hook.sh up <repo> <state> android` を走らせる
- **THEN** 組み立ての引数に `-Pashiato.baseUrl=http://127.0.0.1:18787` がある

#### Scenario: 携帯の相手の APK に、本人の資格情報は入らない
- **GIVEN** `--repo` の作業ツリーに `WEB_PASSWORD=honnin-no-aikotoba`・`API_TOKEN=honnin-no-token`・`ASHIATO_USER_ID=honnin-no-id` を持つ `.env` と、組み立ての引数と `collector-android/local.properties` の中身を記録する偽の `gradlew`
- **WHEN** `tools/tsukau/env-hook.sh up <repo> <state> android` を走らせる
- **THEN** 組み立ての引数・`local.properties`・案内のどこにも `honnin-no-aikotoba`・`honnin-no-token`・`honnin-no-id` は無く、`-Pashiato.apiToken=` の値はその回の環境の `.env` の `API_TOKEN` と同じである

#### Scenario: エミュレータは画面を出さず、前のデータを消して起こす
- **GIVEN** 受けた引数を記録する偽の `emulator` と、網の確かめに試験が決めた答えを返す偽の `adb`
- **WHEN** `tools/tsukau/env-hook.sh up <repo> <state> android` を走らせる
- **THEN** 偽の `emulator` の引数に `-avd ashiato-tsukau-api35`・`-no-window`・`-wipe-data` がある

#### Scenario: 携帯の相手は、自分の serial だけを名指す
- **GIVEN** 実機 `R58N12ABCDE` と別のエミュレータ `emulator-5554` を一覧に返し、受けた引数を記録する偽の `adb` と偽の `emulator`
- **WHEN** `tools/tsukau/env-hook.sh up <repo> <state> android` と `down` を走らせる
- **THEN** 偽の `adb` の `devices` 以外の呼び出しはどれも `-s` に `<state>` に残した serial を持ち、`R58N12ABCDE` と `emulator-5554` を名指す呼び出しは無い

#### Scenario: 網を切って確かめてから、サーバへの道を渡し、APK を入れて案内を置く
- **GIVEN** 切る前の確かめに「ホストに届く」、切った後の確かめに「既定のネットワークが無い・ホストに届かない」と答える偽の `adb` と偽の `emulator`
- **WHEN** `tools/tsukau/env-hook.sh up <repo> <state> android` を走らせる
- **THEN** 偽の `adb` は、切る前の確かめ・`svc wifi disable` と `svc data disable`・切った後の確かめ・`reverse tcp:18787`・`install`・アプリを起こす `am start` の順に呼ばれ、その後に `<state>/ready` が置かれる

#### Scenario: 切った後も既定のネットワークがあれば、APK を入れずに断る
- **GIVEN** 切る前の確かめに「ホストに届く」、切った後の確かめに「既定のネットワークがある・ホストに届かない」と答える偽の `adb` と偽の `emulator`
- **WHEN** `tools/tsukau/env-hook.sh up <repo> <state> android` を走らせる
- **THEN** `<state>/ready` に `UNAVAILABLE=` の行があり、`install` は呼ばれない

#### Scenario: 切った後もホストに届けば、APK を入れずに断る
- **GIVEN** 切る前の確かめに「ホストに届く」、切った後の確かめに「既定のネットワークが無い・ホストに届く」と答える偽の `adb` と偽の `emulator`
- **WHEN** `tools/tsukau/env-hook.sh up <repo> <state> android` を走らせる
- **THEN** `<state>/ready` に `UNAVAILABLE=` の行があり、`install` は呼ばれない

#### Scenario: 切る前に届かなければ、確かめられないとして APK を入れずに断る
- **GIVEN** 切る前の確かめに「ホストに届かない」と答える偽の `adb` と偽の `emulator`
- **WHEN** `tools/tsukau/env-hook.sh up <repo> <state> android` を走らせる
- **THEN** `<state>/ready` に `UNAVAILABLE=` の行があり、`install` は呼ばれない

#### Scenario: 端末の中のコマンドは、自分の serial で走り、出力が返る
- **GIVEN** `<state>` に serial `emulator-5600` が残り、偽の `$ANDROID_HOME` の `platform-tools` に置いた、`shell` で「hello」を返す偽の `adb` と、`adb` の置き場を持たない `PATH`
- **WHEN** `tools/tsukau/phone.sh shell 'echo hello'` を走らせる
- **THEN** 偽の `adb` は `-s emulator-5600 shell` で呼ばれ、`phone.sh` の標準出力は「hello」である

#### Scenario: 受けた PATH に adb が無くても、差し込み口は SDK の置き場の道具で携帯の相手を立てる
- **GIVEN** 偽の `$ANDROID_HOME` に置いた偽の `adb`・`emulator`・`avdmanager`（切る前の確かめに「ホストに届く」、切った後の確かめに「既定のネットワークが無い・ホストに届かない」と答える）と、`adb`・`emulator`・`avdmanager` の置き場を持たない `PATH`
- **WHEN** `tools/tsukau/env-hook.sh up <repo> <state> android` を走らせる
- **THEN** `<state>/ready` に `UNAVAILABLE=` の行は無く、偽の `adb` に `install` の呼び出しがある

#### Scenario: phone.sh は serial を引数で受けず、ほかの端末を名指さない
- **GIVEN** `<state>` に serial `emulator-5600` が残り、受けた引数を記録する偽の `adb`
- **WHEN** `ANDROID_SERIAL=emulator-5554 tools/tsukau/phone.sh shell 'echo hello'` と `tools/tsukau/phone.sh shot <置き場>` と `tools/tsukau/phone.sh ui` を走らせる
- **THEN** 偽の `adb` の呼び出しはどれも最初の 2 つの引数が `-s emulator-5600` である

#### Scenario: 携帯の案内は、口と網と触らないものを書く
- **GIVEN** 切る前の確かめに「ホストに届く」、切った後の確かめに「既定のネットワークが無い・ホストに届かない」と答える偽の `adb` と偽の `emulator`
- **WHEN** `tools/tsukau/env-hook.sh up <repo> <state> android` を走らせる
- **THEN** `ready` の `GUIDE=` の案内に `tools/tsukau/phone.sh shell`・`tools/tsukau/phone.sh shot`・`tools/tsukau/phone.sh ui`・「`phone.sh` だけ」・`http://127.0.0.1:` で始まる画面の URL・「網は切って」・`.env`・「触らない」がある

#### Scenario: 片付けは、端末のログを写してから自分のエミュレータを止める
- **GIVEN** `<state>` に serial `emulator-5600` が残り、動いている端末として `emulator-5600` と `emulator-5554` を返す偽の `adb`
- **WHEN** `tools/tsukau/env-hook.sh down <repo> <state> android` を走らせる
- **THEN** `-s emulator-5600 logcat -d` が `-s emulator-5600 emu kill` より先に呼ばれ、`$TSUKAU_EVIDENCE/phone-logs/` に写したファイルがある

#### Scenario: 携帯の片付けは、ほかのエミュレータに触らない
- **GIVEN** 上と同じ `<state>` と偽の `adb`
- **WHEN** `tools/tsukau/env-hook.sh down <repo> <state> android` を走らせる
- **THEN** `emulator-5554` を名指す呼び出しは 1 つも無い

#### Scenario: 携帯の片付けは、その回の project と木を消す
- **GIVEN** 上と同じ `<state>` と偽の `adb` と、立てた回の compose project と別の `ashiato-verify-t1` の project を返す偽の `docker`
- **WHEN** `tools/tsukau/env-hook.sh down <repo> <state> android` を走らせる
- **THEN** `docker compose` の `down -v` は立てた回の project にだけ呼ばれ、`<state>/tree` は無い

#### Scenario: 止めた後も自分のエミュレータが残っていれば、片付けは非 0 で終わる
- **GIVEN** `emu kill` の後も `emulator-5600` を返す偽の `adb`
- **WHEN** `tools/tsukau/env-hook.sh down <repo> <state> android` を走らせる
- **THEN** rc≠0 で終わる

#### Scenario: 本物のエミュレータで、網を切ったまま APK が入ってサーバに届き、片付けで残らない
- **WHEN** この PC で `tools/tsukau/env-hook.sh up` と `down` を携帯の相手で本物のエミュレータに対して走らせる
- **THEN** `ready` に `UNAVAILABLE=` が無く、端末に既定のネットワークが無く、切る前には届いたホストの loopback の別名の宛先に端末から届かず、端末の中の `http://127.0.0.1:18787` でその回のサーバが応え、携帯の収集アプリのプロセスがあり、`down` は rc=0 で、終わった後に専用の AVD のエミュレータは無い

#### Scenario: 本物のエミュレータで、席の口で端末を操作できる
- **WHEN** この PC で携帯の相手を本物のエミュレータに立て、`tools/tsukau/phone.sh shot`・`phone.sh ui`・`phone.sh shell 'input keyevent KEYCODE_HOME'` を走らせてから片付ける
- **THEN** `shot` は PNG を 1 枚置き、`ui` は携帯の収集アプリのパッケージ名を含む XML を返し、`input` の呼び出しは rc=0 で終わる

### Requirement: 画面の相手は専用の環境で立て、本人の合言葉と確認バッチの DB を使わない

WHEN 差し込み口が画面の相手を立てるとき（`env-hook.sh up <repo> <state> web`）、
THE SYSTEM SHALL `<state>/tree` に `--repo` の HEAD を detached で取り出した木に、実行ごとの compose project（`ashiato-tsukau-` で始まる名前）・空いている port・その場で作った乱数の合言葉を持つ `.env` と port の上書きを置き（録画の専用環境と同じ形）、その木で作り直した偽データとともに `tools/stack.sh up` を起こさせる。
`--repo` の作業ツリーの `.env` は読まず、`--repo` には何も書かない。本人の合言葉・確認バッチの DB（`ashiato-verify-` の compose project）・既定の compose project を使わない。
THE SYSTEM SHALL サーバと画面の置き場が `ASHIATO_TSUKAU_SERVER_BIN`・`ASHIATO_TSUKAU_WEB_DIST` で渡されたらそれを使い、渡された置き場が無ければ組まずに `UNAVAILABLE=` を置かせる。渡されていなければ（手で呼んだとき）`<state>/tree` で組む。
THE SYSTEM SHALL 案内に、画面の URL・その回の画面の合言葉・本物のブラウザで画面を開いて写しを撮るコマンド・サーバのログの場所・触らないもの（本人のサーバ・確認バッチの DB・main の checkout の `.env`）を書かせ、`INFO_COMMIT=` に立てたコミットを残させる。
THE SYSTEM SHALL 片付けでは、その compose project のコンテナと volume だけを消し、`<state>/tree` を消させ、残れば rc≠0 で終わらせる。
導出元: CH21。review/deep.md R2（本物の合言葉と本人の記録の入った DB を席に渡さない）。深掘り Q8 の context（どの案でも共通の既定）。

#### Scenario: 画面の相手は、本人の .env を持たない別の木から起動する
- **GIVEN** `--repo` の作業ツリーに `WEB_PASSWORD=honnin-no-aikotoba` を持つ `.env` と、起こされた場所とその木の根の `.env` の中身を記録する偽の起動の台本（`<state>/tree/tools/stack.sh` の位置に置く）
- **WHEN** `tools/tsukau/env-hook.sh up <repo> <state> web` を走らせる
- **THEN** 起動の台本は `<state>/tree` の中で起こされ、その木の `.env` に `honnin-no-aikotoba` は無く `COMPOSE_PROJECT_NAME=ashiato-tsukau-` の行がある

#### Scenario: 画面の相手を立てても、--repo は変わらない
- **GIVEN** 上と同じ `.env` と偽の起動の台本
- **WHEN** `tools/tsukau/env-hook.sh up <repo> <state> web` と `down` を走らせる
- **THEN** `--repo` の `git status --porcelain` は呼ぶ前と同じである

#### Scenario: 画面の相手は、専用の project と本人のものでない合言葉で立つ
- **GIVEN** 上と同じ `.env` と、受けた環境変数を記録する偽の起動の台本
- **WHEN** `tools/tsukau/env-hook.sh up <repo> <state> web` を走らせる
- **THEN** 起動に渡った `COMPOSE_PROJECT_NAME` は `ashiato-tsukau-` で始まり、`WEB_PASSWORD` は `honnin-no-aikotoba` でなく、案内にも `honnin-no-aikotoba` は無い

#### Scenario: 渡されたサーバと画面の置き場がそのまま起動に届く
- **GIVEN** 受けた環境変数を記録する偽の起動の台本と、`ASHIATO_TSUKAU_SERVER_BIN`・`ASHIATO_TSUKAU_WEB_DIST` に、ある一時の置き場の絶対パス
- **WHEN** `tools/tsukau/env-hook.sh up <repo> <state> web` を走らせる
- **THEN** 起動に渡った `SERVER_BIN`・`WEB_DIST` は渡した絶対パスと同じである

#### Scenario: 渡された置き場が無ければ、立てずに断る
- **GIVEN** 受けた環境変数を記録する偽の起動の台本と、`ASHIATO_TSUKAU_SERVER_BIN`・`ASHIATO_TSUKAU_WEB_DIST` に無い置き場の絶対パス
- **WHEN** `tools/tsukau/env-hook.sh up <repo> <state> web` を走らせる
- **THEN** `ready` に `UNAVAILABLE=` があり、起動の台本は呼ばれない

#### Scenario: 手で呼んだときは、置き場が渡されなければ別の木で組む
- **GIVEN** `ASHIATO_TSUKAU_SERVER_BIN`・`ASHIATO_TSUKAU_WEB_DIST` を渡さず、組む命令を記録する偽の `cargo` と偽の `npm`
- **WHEN** `tools/tsukau/env-hook.sh up <repo> <state> web` を走らせる
- **THEN** 偽の `cargo` と偽の `npm` は `<state>/tree` の中で呼ばれ、`--repo` では呼ばれない

#### Scenario: 画面の案内は、URL と触らないものを書き、立てたコミットを残す
- **GIVEN** 受けた環境変数を記録する偽の起動の台本
- **WHEN** `tools/tsukau/env-hook.sh up <repo> <state> web` を走らせる
- **THEN** 案内に `http://127.0.0.1:` で始まる画面の URL・その回の画面の合言葉・写しを撮るコマンド（`playwright`）・サーバのログの場所・「触らない」があり、`ready` に `INFO_COMMIT=` と `--repo` の HEAD のコミットがある

#### Scenario: 片付けは、その回の project だけを消す
- **GIVEN** 立てた回の compose project と、別の `ashiato-verify-t1` の project を返す偽の `docker`
- **WHEN** `tools/tsukau/env-hook.sh down <repo> <state> web` を走らせる
- **THEN** `docker compose` の `down -v` は立てた回の project にだけ呼ばれ、`ashiato-verify-t1` への呼び出しは無く、`<state>/tree` は無い

### Requirement: 処置との契約を守り、処置まで通ることを、公開の repo に何も出さない実地の 1 回で受け入れる

THE SYSTEM SHALL 処置（harness2 の確認バッチ。入口は design D15 の 1 で依頼した台本）に対して、ashiato2 が持つ契約 —— 目録 `dist/verify-<tag>/tsukau.json`・公開の題の台本 `tools/tsukau/public-title.sh`・確かめ直しの口 `TSUKAU_RECHECK` —— を守らせる。
選別・直す数の上限（3 件まで・1 回まで）・本人に聞く問い・違和感の issue の振る舞いは harness2 への依頼（design D15）が約束し、ashiato2 の正典は約束しない。
THE SYSTEM SHALL CH21 の受け入れを、実地の台本 `tools/tsukau/real-run.sh` の 1 回で確かめさせる: 使い捨ての確認バッチの木（push 先は一時の bare repo、`gh` は呼ばれた引数と本文を記録する偽物、偽の Story の作業ツリーと緑を返す CI を持つ）に、
画面と PC を変えた偽の `feat/ch99-tsukau-real` を重ね、席の候補に、用意した候補を 2 つ足す（その回の画面の文言を崩した「この回が原因」のものと、main の版でも起きる「前から」のもの。根拠の文は自己試験が決めた字面）。
そのうえで処置を走らせ、用意した 2 つの候補が、どちらも処置の結果（直した・本人に聞く・違和感の issue・誤検知のどれか）を持ち、公開の repo に向けて出たもの（bare repo の commit の本文・手順書の JSON・偽の `gh` の本文）に実行のフォルダのパスと用意した根拠の文が無いことを見る。
本物の公開の repo には何も出させない。処置の入口が無ければ、確かめは「確認バッチの処置（harness2）が無い」と出して rc=127 で落ちる（飛ばさない。rc=127 は「要る環境が無い」として証跡に残り、AI の直しの輪に流れない）。
導出元: CH21（依頼「見つけた後の処置」）。深掘り Q1（第 1 回の推奨）・Q2・Q3（数と「迷えば本人へ」は harness2 への依頼が持つ）・Q4（題と種類とどの Story だけ）・Q13（処置まで実地で通す）。review/deep.md R1。review/spec.md R2・R4・R5・R6。

#### Scenario: 本物で、用意した 2 つの候補はどちらも処置の結果を持つ
- **GIVEN** harness2 の処置が入った木と、上の使い捨ての確認バッチの木と、用意した 2 つの候補
- **WHEN** `tools/tsukau/real-run.sh` を走らせる
- **THEN** 処置の結果に、用意した 2 つの候補の `id` がどちらもあり、それぞれが「直した」「本人に聞く」「違和感の issue」「誤検知」のどれかである

#### Scenario: 本物で、直した候補は Story のブランチに直しが載り、席で確かめ直される
- **GIVEN** 上と同じ実地の 1 回で、「直した」になった候補
- **WHEN** 一時の bare repo の `feat/ch99-tsukau-real` と、作り直した回の目録を読む
- **THEN** `feat/ch99-tsukau-real` に直しの commit があり、作り直した回の目録の `recheck` にその候補の `id` がある（「直した」になった候補が無ければ、この Scenario は「直した 0 件」と 1 行出して落ちる）

#### Scenario: 本物の処置で公開の repo に向けて出たものは、題と番号だけ
- **GIVEN** 上と同じ実地の 1 回の結果
- **WHEN** 一時の bare repo に push された commit の本文・足された試験の名前と中の文字列・手順書の JSON・記録する偽の `gh` に渡った本文（issue とコメント）を読む
- **THEN** どれにも実行のフォルダのパス（`.local/state`）・`RESULT.md`・`/home/`・用意した根拠の文は無く、本物の公開の repo への push と本物の `gh` の呼び出しは 1 つも無い

#### Scenario: harness2 の処置が無ければ、実地の確かめは落ちる
- **GIVEN** 処置の入口が無い木
- **WHEN** `tools/tsukau/real-run.sh` を走らせる
- **THEN** rc=127 で、「確認バッチの処置（harness2）が無い」と 1 行出す
