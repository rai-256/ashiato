## ADDED Requirements

### Requirement: 使ってみる席の差し込み口は、画面と PC の相手を持ち、手で呼んでも確認バッチから呼んでも同じ置き場に結果を残す

THE SYSTEM SHALL harness2 の使ってみる席（`scripts/tsukau.py`）が読む差し込み口 `tools/tsukau/tsukau.json` を持たせる。
その `env_hook` は `tools/tsukau/env-hook.sh`、`targets` は `web`（画面）と `pc`（PC）の 2 つ、`out_root` は repo の外の `~/.local/state/ashiato2/tsukau`、
`ready_timeout_s` は 900 にする。携帯（`android`）は持たない。
THE SYSTEM SHALL 手で呼ぶ `/tsukau` も、確認バッチの準備から呼ぶ席も、この差し込み口と同じ `out_root` を使わせる（どの作業ツリーから呼んでも、実行のフォルダが 1 つの置き場に並ぶ）。
導出元: CH21（依頼の「どこまで」）。深掘り Q5（第 1 回の推奨: 立てる上限 15 分）。review/deep.md R11。

#### Scenario: 席の台本が差し込み口を受け付ける
- **WHEN** repo の根で `python3 scripts/tsukau.py check` を走らせる
- **THEN** rc=0 で、出力の `targets` は `["web", "pc"]`、`out_root` は `~/.local/state/ashiato2/tsukau` を展開したパスである

#### Scenario: 携帯を相手にした呼び出しは、立てる前に断られる
- **WHEN** `python3 scripts/tsukau.py run --scope 試し --target android --minutes 1 --tokens 1000` を走らせる
- **THEN** rc=2 で、`~/.local/state/ashiato2/tsukau` の下に実行のフォルダが増えない

#### Scenario: 別の作業ツリーから確かめても、置き場は同じ
- **GIVEN** 同じ commit を detached で取り出した使い捨ての作業ツリー
- **WHEN** その作業ツリーを `--repo` にして `python3 scripts/tsukau.py check` を走らせる
- **THEN** 出力の `out_root` は repo の根で確かめたときと同じパスである

### Requirement: 確認バッチの範囲と相手は、その回に入った Story と change が変えたファイルと表題から機械で決める

WHEN 確認バッチの準備が席を呼ぶとき、
THE SYSTEM SHALL 確認バッチの作業ツリーの、`origin/main` から HEAD までの merge のうち「確認バッチ <tag> に入れる」merge のそれぞれについて、その merge が入れたファイル（merge の 1 つ目の親との差）で相手を決めさせる。
画面のファイル（`web/src/`・`web/index.html`・`web/public/`）を変えたものは画面の相手に、サーバ・PC の収集アプリ・更新係・インストーラ・サービスの部品・移行（`crates/` の下と `migrations/`）を変えたものは PC の相手に入れる。両方を変えたものは両方に入れる。
携帯（`collector-android/`）だけ、文書・道具・試験だけを変えたものは、どの相手にも入れない。
THE SYSTEM SHALL 相手ごとの範囲を、入れたものごとに 1 行の「<ID> <表題>: <何のためか>」を並べたものにさせる。表題は Story の `docs/stories/ST<NN>.md` か change の `docs/changes/CH<NN>.md` の front matter の `title`、
「何のためか」はその文書の「価値」の節の最初の段落の 1 行目（120 字まで）。価値の節がまだ雛形（「（」で始まる）なら表題だけにする。
THE SYSTEM SHALL 同じ作業ツリーの同じ HEAD からは、何度求めても同じ範囲と相手を返させる。範囲を AI に要約させない。
導出元: CH21。深掘り Q1（第 1 回の推奨: 変えたファイルと表題から機械で）。

#### Scenario: 画面のファイルだけを変えた Story は、画面の相手の範囲に入る
- **GIVEN** `origin/main` に、`web/src/DayView.tsx` だけを変えた `feat/st12-day-view` を「確認バッチ t1 に入れる」merge を 1 つ重ねた木と、`docs/stories/ST12.md`（表題「1 日の画面」、価値の最初の行「本人として、1 日の動きを 1 枚で見たい。」）
- **WHEN** 範囲を求める
- **THEN** 画面の範囲は「ST12 1 日の画面: 本人として、1 日の動きを 1 枚で見たい。」の 1 行で、PC の範囲は空である

#### Scenario: サーバか PC のアプリを変えたものは、PC の相手の範囲に入る
- **GIVEN** `crates/updater/src/lib.rs` を変えた `feat/ch30-x` の merge と、`migrations/202611200000_x.sql` を変えた `feat/st40-y` の merge を重ねた木
- **WHEN** 範囲を求める
- **THEN** PC の範囲は CH30 と ST40 の 2 行で、画面の範囲は空である

#### Scenario: 両方を変えたものは、両方の範囲に入る
- **GIVEN** `web/src/App.tsx` と `crates/server/src/lib.rs` を変えた `feat/st20-z` の merge を重ねた木
- **WHEN** 範囲を求める
- **THEN** 画面の範囲と PC の範囲の両方に ST20 の行がある

#### Scenario: 携帯だけ・文書や道具だけを変えたものは、どの範囲にも入らない
- **GIVEN** `collector-android/app/src/main/kotlin/dev/ashiato/collector/A.kt` だけを変えた merge と、`docs/testing.md` と `tools/verify-prep.sh` だけを変えた merge と、`web/e2e/a.spec.ts` だけを変えた merge を重ねた木
- **WHEN** 範囲を求める
- **THEN** 画面の範囲も PC の範囲も空である

#### Scenario: 価値の節が雛形の change は、表題だけを範囲に書く
- **GIVEN** `web/src/App.tsx` を変えた `feat/ch31-w` の merge と、価値の節が「（`hx change` の依頼の逐語。…）」のままの `docs/changes/CH31.md`（表題「並びを直す」）
- **WHEN** 範囲を求める
- **THEN** 画面の範囲は「CH31 並びを直す」の 1 行である

#### Scenario: 同じ木から 2 度求めると、同じ範囲になる
- **GIVEN** 上の 3 つの merge を重ねた木
- **WHEN** 範囲を 2 度求める
- **THEN** 2 度の出力はバイト単位で同じである

### Requirement: 確認バッチの準備は、範囲のある相手ごとに 1 回、使い捨ての作業ツリーで、決まった時間と token で席を呼ぶ

WHEN 確認バッチの準備（`tools/verify-prep.sh <tag>`）が、既存の成果物を作り終えたとき、
THE SYSTEM SHALL `tools/tsukau/verify-batch.sh <tag>` を呼ばせ、範囲のある相手ごとに 1 回、`scripts/tsukau.py run --target <相手> --minutes 10 --tokens 200000 --label verify-<tag>` で席を呼ばせる。範囲が空の相手は呼ばない。
THE SYSTEM SHALL 席を、確認バッチの作業ツリーではなく、その HEAD を detached で取り出した使い捨ての作業ツリーで走らせる（`--repo` にその作業ツリーを渡す）。
その作業ツリーには harness2 を繋ぎ（`init.py --worktree`）、`.env` を写さない。席が終わったら（どの止まり方でも）その作業ツリーを消す。
THE SYSTEM SHALL 1 回の呼び出しを、立てる上限・席の時間・片付けの上限に 2 分を足した時間（900 + 600 + 300 + 120 = 1,920 秒。PC の配る zip を組む時間は含まない）で打ち切らせる。
導出元: CH21（依頼「小さい時間と token で毎回呼ぶ」）。深掘り Q5（第 1 回の推奨: 10 分・20 万）。review/deep.md R1・R4・R8・R18。

#### Scenario: 範囲のある相手だけを、決まった時間と token で呼ぶ
- **GIVEN** 画面の範囲だけがある確認バッチの木 `t1` と、受けた引数を記録して rc=0 で `{"v":1,"stop":"done","count":0,"titles":[]}` を出す偽の席の台本
- **WHEN** `tools/tsukau/verify-batch.sh t1` を走らせる
- **THEN** 偽の席の台本はちょうど 1 回呼ばれ、引数に `--target web`・`--minutes 10`・`--tokens 200000`・`--label verify-t1` がある
- **AND** `--target pc` の呼び出しは無い

#### Scenario: 席は使い捨ての作業ツリーで走り、確認バッチの作業ツリーは書き換わらない
- **GIVEN** 呼ばれた作業ツリー（`--repo`）に `scribble.txt` を書き、そこに `.harness2` と `.env` があるかを記録する偽の席の台本
- **WHEN** `tools/tsukau/verify-batch.sh t1` を走らせる
- **THEN** `--repo` は確認バッチの作業ツリーと違うパスで、そこに `.harness2` があり `.env` は無かった
- **AND** 終わった後、確認バッチの作業ツリーの `git status --porcelain` は呼ぶ前と同じで、`--repo` のパスは無く、`git worktree list` にも無い

#### Scenario: 時間の上限を越えた呼び出しは打ち切られる
- **GIVEN** 終わらない偽の席の台本と、上限の秒数を 2 秒に縮めた自己試験
- **WHEN** `tools/tsukau/verify-batch.sh t1` を走らせる
- **THEN** 偽の席の台本は打ち切られ、`verify-batch.sh` は rc=0 で、画面の行は「時間の上限で打ち切った」である

### Requirement: 席の結果は、件数・題・止まり方だけを決まった文言で手順書に写し、題は私的な形を伏せてから写す

THE SYSTEM SHALL 確認バッチの準備に、席の結果を `dist/verify-<tag>/manifest.md` の最後の段落として 1 段落だけ足させる（手順書の冒頭と verify PR の本文に載る）。
段落は「使ってみる席（AI が利用者として使ってみた結果。人間の確認ではない）」で始まり、相手ごとに 1 行を持つ。写すのは席の台本の `public.json` の件数・題・止まり方だけで、
候補の本文・根拠・画像・ログ・実行のフォルダのパス・席の台本の `detail` は写さない。
THE SYSTEM SHALL 止まり方ごとに次の決まった文言を書かせる: 終えた・時間を使い切った・token を使い切った（`done`・`time`・`tokens`）は「<止まり方の文言>。違和感の候補 <件数> 件」と題の一覧、
`crashed` は「席が途中で落ちた」、`env_failed` は「相手を立てられなかった」、`interrupted` は「中断した」、席の台本が rc=2 で終わった・`public.json` を読めない・台本が無いときは「席を呼べなかった」、
打ち切ったときは「時間の上限で打ち切った」、範囲が空の相手は「回さなかった（この回に<相手>を変えたものが無い）」、PC の配る zip を組めなかったときは「配る zip を組めず、回さなかった」。
片付けに残りがあれば、その行に「片付けに残りがある」を足す。
THE SYSTEM SHALL 題を写す前に、題に、この PC の利用者名・ホスト名・私設網の名前・確認バッチの作業ツリーの `.env` の値のどれかがあるか、絶対パスの形（`/home/`・`/mnt/`・`C:\`・`\\` で始まる語）があれば、その題を「（伏せた）」に置き換えさせる（件数には数える）。
THE SYSTEM SHALL issue を作らせない。
導出元: CH21（依頼「件数・題・止まり方だけ」「issue は作らない」）。深掘り Q7（第 1 回の推奨: 止まり方ごとの決まった文言だけ）。review/deep.md R5・R10。

#### Scenario: 席が終えた相手は、件数と題を手順書に載せる
- **GIVEN** `{"v":1,"stop":"done","count":2,"titles":["更新の確認が赤いまま","設定の言葉が分からない"]}` を出し、実行のフォルダに候補の根拠「logs の 12 行目」を書く偽の席の台本
- **WHEN** `tools/tsukau/verify-batch.sh t1` を走らせる
- **THEN** 出力の段落は「使ってみる席（AI が利用者として使ってみた結果。人間の確認ではない）」で始まり、画面の行に「席が自分で終えた。違和感の候補 2 件」と 2 つの題がある
- **AND** 段落に「logs の 12 行目」・実行のフォルダのパス・`~/.local/state` の文字列は無い

#### Scenario: 止まり方ごとに決まった文言だけを書き、席の台本の理由は写さない
- **GIVEN** `crashed`・`env_failed`・`interrupted` のそれぞれを `detail` に「/home/u/x の行」を持たせて出し rc=3 で終わる偽の席の台本と、rc=2 で終わる偽の席の台本
- **WHEN** それぞれで `tools/tsukau/verify-batch.sh t1` を走らせる
- **THEN** 画面の行はそれぞれ「席が途中で落ちた」「相手を立てられなかった」「中断した」「席を呼べなかった」で、どの段落にも「/home/u/x」は無い

#### Scenario: 範囲が空の相手は、回さなかったと 1 行書く
- **GIVEN** 文書だけを変えた merge だけを重ねた確認バッチの木
- **WHEN** `tools/tsukau/verify-batch.sh t1` を走らせる
- **THEN** 席の台本は呼ばれず、段落に「回さなかった（この回に画面を変えたものが無い）」と「回さなかった（この回に PC を変えたものが無い）」がある

#### Scenario: 私的な形を含む題は伏せて、件数には数える
- **GIVEN** 題が「/home/someone/x が開けない」「C:\Users\a で落ちる」「並びがおかしい」の 3 つと、この PC のホスト名を含む題 1 つの `public.json` を出す偽の席の台本
- **WHEN** `tools/tsukau/verify-batch.sh t1` を走らせる
- **THEN** 画面の行は「違和感の候補 4 件」で、題は「（伏せた）」が 3 つと「並びがおかしい」で、ホスト名と `/home/someone` と `C:\Users\a` は段落に無い

#### Scenario: 片付けに残りがあれば、その行に書く
- **GIVEN** `stop` が `done` で、片付けが「残りあり」だったことを最後の行の JSON の `cleanup` に出す偽の席の台本
- **WHEN** `tools/tsukau/verify-batch.sh t1` を走らせる
- **THEN** 画面の行に「片付けに残りがある」がある

### Requirement: 席が落ちても時間切れでも、確認バッチの準備は止まらず、今の成果物を変えない

THE SYSTEM SHALL 席の呼び出し（`tools/tsukau/verify-batch.sh`）がどんな終わり方をしても（非 0・打ち切り・台本が無い）、確認バッチの準備（`tools/verify-prep.sh`）を rc=0 のまま続けさせ、そのときは手順書に「使ってみる席: 呼べなかった」の 1 段落を足させる。
THE SYSTEM SHALL 席を呼んでも、確認バッチの準備が作る成果物（`ashiato-server`・`web/`・`app-debug.apk`・`run.sh`）と、手順書の既存の段落を変えさせない。席の段落は既存の段落の後に足すだけにする。
導出元: CH21（依頼「席が落ちても・時間切れでも、確認バッチの準備は止めない」）。深掘り Q8（第 1 回の推奨: 守るもの）。review/deep.md R10。

#### Scenario: 席の呼び出しが落ちても、準備の rc は 0 のまま
- **GIVEN** rc=1 で終わり何も出さない偽の `tools/tsukau/verify-batch.sh`
- **WHEN** 確認バッチの準備の、席を呼ぶ部分を `set -euo pipefail` の下で走らせる
- **THEN** rc=0 で、足す段落は「使ってみる席: 呼べなかった」で始まる 1 段落である

#### Scenario: 席の段落は、既存の段落の後に足すだけ
- **WHEN** 確認バッチの準備の台本を読む
- **THEN** 席を呼ぶ部分は `manifest.md` を書く部分より後にあり、`manifest.md` へは追記（`>>`）だけで書き、`run.sh`・`ashiato-server`・`web/`・`app-debug.apk` を書く行を持たない

### Requirement: 確認バッチの中身が前の回と同じなら、席を呼ばずに前の結果を写す

WHEN 確認バッチの準備が相手の範囲を決めたとき、
THE SYSTEM SHALL `origin/main` のコミット・相手・その相手の範囲に入ったものごとの ID とその PR のいちばん新しいコミットから鍵を作らせ、同じ鍵で前に席を呼んだ結果が `out_root` の下にあれば、席を呼ばずにその結果（件数・題・止まり方）を写させる。
そのときの行には「前の回 <前の tag> と同じ中身なので、その結果を写した」を足す。前の結果の止まり方が `done`・`time`・`tokens` でないなら写さずに呼ぶ。
導出元: CH21（依頼「毎回」）。深掘り Q6（第 1 回の推奨: 同じ中身なら前の結果を写す）。review/deep.md R19。

#### Scenario: 同じ中身で作り直した回は、席を呼ばずに前の結果を写す
- **GIVEN** 確認バッチ `t1` で画面の席が `done`・候補 1 件「並びがおかしい」で終わった後、同じ `origin/main` と同じ PR のコミットで作り直した確認バッチ `t2`
- **WHEN** `tools/tsukau/verify-batch.sh t2` を走らせる
- **THEN** 席の台本は呼ばれず、画面の行に「違和感の候補 1 件」「並びがおかしい」と「前の回 t1 と同じ中身なので、その結果を写した」がある

#### Scenario: main が進んだか、PR のコミットが変われば、呼び直す
- **GIVEN** `t1` の後に、`origin/main` だけを進めた確認バッチ `t3` と、画面の範囲の PR のコミットだけを進めた確認バッチ `t4`
- **WHEN** それぞれで `tools/tsukau/verify-batch.sh` を走らせる
- **THEN** どちらでも画面の席の台本が 1 回呼ばれる

#### Scenario: 前の回が落ちていたら、写さずに呼び直す
- **GIVEN** `t1` で画面の席が `env_failed` で終わった後、同じ中身の確認バッチ `t2`
- **WHEN** `tools/tsukau/verify-batch.sh t2` を走らせる
- **THEN** 画面の席の台本が 1 回呼ばれる

### Requirement: PC の相手は、まっさらな Windows Sandbox に配る形を入れて立て、自分が起こした Sandbox だけを、中のログを写してから片付ける

WHEN 差し込み口が PC の相手を立てるとき（`env-hook.sh up <repo> <state> pc`）、
THE SYSTEM SHALL 起こす前に別の Windows Sandbox が動いていれば立てずに `UNAVAILABLE=` を置かせる（待たない）。
THE SYSTEM SHALL 配る形のサーバの zip と PC の収集アプリの zip（`tools/tsukau/build-pc.sh` が組んだもの）が無ければ、立てずに `UNAVAILABLE=` を置かせる。
THE SYSTEM SHALL そうでなければ Sandbox を 1 つ起こし、その id を `<state>` に残し、配る形のサーバの zip をインストーラで入れ、配る形の PC の収集アプリを起こし、Sandbox の窓を本人のデスクトップに開いてから、案内（`GUIDE=`）を置いた `ready` を置かせる。
案内には、Sandbox の中でコマンドを走らせて出力を受け取る口と画面の写しを撮る口（`tools/tsukau/pc.sh`）、サービスとログの場所、触らないもの（本人のサーバ・本人の PC の配布版 `C:\ProgramData\ashiato`・確認バッチの DB・main の checkout の `.env`・本人が起こした Sandbox）を書く。
THE SYSTEM SHALL 片付け（`env-hook.sh down <repo> <state> pc`）では、Sandbox の中のサービスのログ・更新係の置き場の `checks.jsonl` とログ・PC の収集アプリのログ・サービスに関わるシステムのイベントの抜き書きを `$TSUKAU_EVIDENCE/pc-logs/` へ写してから、`<state>` に残した id の Sandbox だけを止めさせる。
止めた後もその id が動いていれば rc≠0 で終わらせる。ほかの Sandbox には触らせない。
導出元: CH21（依頼「PC 側（Windows Sandbox に配る zip を入れる）」）。深掘り Q3（第 1 回の推奨: 確認バッチの木から手元で組む）・Q4（窓を開いて画面も触る）。review/deep.md R6・R7。

#### Scenario: 別の Sandbox が動いていれば、立てずに断る
- **GIVEN** 動いている Sandbox の id を 1 つ返す偽の `wsb`
- **WHEN** `tools/tsukau/env-hook.sh up <repo> <state> pc` を走らせる
- **THEN** `<state>/ready` に `UNAVAILABLE=` の行があり、`wsb start` は呼ばれない

#### Scenario: 配る zip が無ければ、立てずに断る
- **GIVEN** 動いている Sandbox が無い偽の `wsb` と、配る zip の置き場が空
- **WHEN** `tools/tsukau/env-hook.sh up <repo> <state> pc` を走らせる
- **THEN** `<state>/ready` に `UNAVAILABLE=` の行があり、その理由に `tools/tsukau/build-pc.sh` の名前がある。`wsb start` は呼ばれない

#### Scenario: 立てるときは、入れて起こして窓を開いてから案内を置く
- **GIVEN** 動いている Sandbox が無く、`start` で id `sb-1` を返し、網の確かめで外にだけ届いたと返す偽の `wsb` と、配る zip の置き場
- **WHEN** `tools/tsukau/env-hook.sh up <repo> <state> pc` を走らせる
- **THEN** 偽の `wsb` は `start`・サーバのインストールの `exec`・PC の収集アプリを起こす `exec`・`connect --id sb-1` の順に呼ばれ、その後に `<state>/ready` が置かれる
- **AND** `ready` の `GUIDE=` の案内に `tools/tsukau/pc.sh`・`C:\ProgramData\ashiato`・`.env`・「触らない」がある

#### Scenario: 片付けは、中のログを写してから自分の Sandbox だけを止める
- **GIVEN** `<state>` に id `sb-1` が残り、動いている Sandbox として `sb-1` と `sb-other` を返す偽の `wsb`
- **WHEN** `tools/tsukau/env-hook.sh down <repo> <state> pc` を走らせる
- **THEN** ログを共有のフォルダへ写す `exec --id sb-1` が `stop --id sb-1` より先に呼ばれ、`$TSUKAU_EVIDENCE/pc-logs/` に写したファイルがある
- **AND** `stop` は `sb-1` にだけ呼ばれ、`sb-other` への呼び出しは 1 つも無い

#### Scenario: 止めた後も自分の Sandbox が動いていれば、片付けは非 0 で終わる
- **GIVEN** `stop` の後も `sb-1` を動いていると返す偽の `wsb`
- **WHEN** `tools/tsukau/env-hook.sh down <repo> <state> pc` を走らせる
- **THEN** rc≠0 で終わる

### Requirement: PC の Sandbox は、私設網と手元の網に届かず外に届くことを中から確かめてから席に渡し、確かめられなければ網を切って立て直す

WHEN 差し込み口が PC の相手の Sandbox を網ありで起こしたとき、
THE SYSTEM SHALL 配る形を入れる前に、Sandbox の中のファイアウォールで、私設網と手元の網の宛先（RFC 1918 の私用の 3 つの範囲・私設網（tailscale）が使う CGNAT の範囲（RFC 6598）・IPv4 のリンクローカル・IPv6 の私用（ULA）とリンクローカルの、7 つの範囲）への外向きを塞がせる。
THE SYSTEM SHALL 塞いだ後に Sandbox の中から、この PC の私設網の宛先と手元の網の宛先のそれぞれに TCP でつなぎに行き、どれにも届かないことを確かめさせる。1 つでも届けば、その Sandbox を止めて、網を切った（`<Networking>Disable</Networking>`）Sandbox で立て直させる。
THE SYSTEM SHALL どの網で立てたかを `INFO_NETWORK=` に残させる: 外にだけ届く（`outside-only`）・網を切った（`disabled`）・塞いだ後に外にも届かない（`no-outside`）。
THE SYSTEM SHALL 案内に、Sandbox が外へつながることと、席が範囲の外の外のサイトを開かないことを書かせる。
導出元: CH21。深掘り Q2（第 1 回の推奨: 外にはつなぎ、私設網と手元の網を塞ぐ。塞げなければ網を切る側に倒す）。premises.md（Sandbox から私設網に届いた。確かさ 5）。review/deep.md R13・R14。

#### Scenario: 塞いだ後に私設網に届かず外に届けば、網ありで渡す
- **GIVEN** 網の確かめで、私設網と手元の網の宛先には届かず、外の宛先には届いたと返す偽の `wsb`
- **WHEN** `tools/tsukau/env-hook.sh up <repo> <state> pc` を走らせる
- **THEN** `ready` に `INFO_NETWORK=outside-only` があり、`wsb start` は 1 回だけ呼ばれ、ファイアウォールを塞ぐ `exec` が網の確かめと配る形のインストールより先に呼ばれている

#### Scenario: 塞いだ後も私設網に届けば、網を切って立て直す
- **GIVEN** 1 度目の Sandbox の網の確かめで私設網の宛先に届いたと返す偽の `wsb`
- **WHEN** `tools/tsukau/env-hook.sh up <repo> <state> pc` を走らせる
- **THEN** 1 度目の Sandbox の id に `stop` が呼ばれ、2 度目の `wsb start` に渡した設定に `<Networking>Disable</Networking>` があり、`ready` に `INFO_NETWORK=disabled` がある

#### Scenario: 本物の Sandbox で、塞いだ後に私設網に届かない
- **WHEN** この PC で `tools/tsukau/env-hook.sh probe-network` を走らせる（本物の Sandbox を網ありで起こし、塞ぎ、確かめ、止める）
- **THEN** rc=0 で、出力は私設網の宛先と手元の網の宛先のそれぞれに「届かない」、結果の行は `outside-only` か `no-outside` であり、終わった後に動いている Sandbox は無い

### Requirement: 画面の相手は専用の環境で立て、本人の合言葉と確認バッチの DB を使わない

WHEN 差し込み口が画面の相手を立てるとき（`env-hook.sh up <repo> <state> web`）、
THE SYSTEM SHALL 実行ごとの compose project（`ashiato-tsukau-` で始まる名前）・空いている port・その場で作った乱数の合言葉・作り直した偽データで、DB・サーバ・画面を立てさせる。
作業ツリーの `.env` があっても読まず、本人の合言葉・確認バッチの DB（`ashiato-verify-` の compose project）・既定の compose project を使わない。
THE SYSTEM SHALL 案内に、画面の URL・その回の画面の合言葉・本物のブラウザで画面を開いて写しを撮るコマンド・サーバのログの場所・触らないもの（本人のサーバ・確認バッチの DB・main の checkout の `.env`）を書かせ、`INFO_COMMIT=` に立てたコミットを残させる。
THE SYSTEM SHALL 片付けでは、その compose project のコンテナと volume だけを消させ、残れば rc≠0 で終わらせる。
導出元: CH21。review/deep.md R4・R11（録画の専用環境と同じ立て方）。

#### Scenario: 画面の相手は、本人の .env を読まずに専用の project で立つ
- **GIVEN** 作業ツリーに `WEB_PASSWORD=honnin-no-aikotoba` を持つ `.env` と、受けた環境変数を記録して起動したことにする偽の起動の台本
- **WHEN** `tools/tsukau/env-hook.sh up <repo> <state> web` を走らせる
- **THEN** 起動に渡った `COMPOSE_PROJECT_NAME` は `ashiato-tsukau-` で始まり、`WEB_PASSWORD` は `honnin-no-aikotoba` でなく、案内にも `honnin-no-aikotoba` は無い
- **AND** 案内に画面の URL と「触らない」があり、`ready` に `INFO_COMMIT=` がある

#### Scenario: 片付けは、その回の project だけを消す
- **GIVEN** 立てた回の compose project と、別の `ashiato-verify-t1` の project を返す偽の `docker`
- **WHEN** `tools/tsukau/env-hook.sh down <repo> <state> web` を走らせる
- **THEN** `docker compose` の `down -v` は立てた回の project にだけ呼ばれ、`ashiato-verify-t1` への呼び出しは無い
