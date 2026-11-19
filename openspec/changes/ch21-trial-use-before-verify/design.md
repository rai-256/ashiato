# CH21 design — 確認バッチの前に Claude が配る形を入れて普通に使ってみる

動機は `proposal.md` の Why、振る舞いは `specs/trial-use/spec.md`。ここは「どう作るか」と、下流が実装中に決めそうな判断の前払い（`D<n>（仮）` と反転条件）。
第 1 回の深掘りの答えはまだ無い。推奨の答えで書いてある（`deep.md`）。

## Context

- 確認バッチ（harness2 の `scripts/verify_batch.sh`）は、`tools/verify-prep.sh <tag>` を呼び（落ちたら `warn:` で成果物なしに続く）、次に録画（`record-run`）、最後に手順書（`verify_checklist.py --manifest`）を作る。
  manifest は段落ごとに 1 行の文字の note になり、`docs/verify/<tag>.json` として commit され公開の origin に push され、PR の本文にも写る（`premises.md`）
- `verify-prep.sh` は `set -euo pipefail` で、manifest を末尾の heredoc で書く。ここで非 0 になると、manifest ごと手順書から消える（deep-review R7）
- 録画は毎回まっさらな Windows Sandbox を起こし、別の Sandbox が動いていれば止まる。Sandbox は手元で同時に 1 つ
- 配る Windows の zip は `tools/release/package.sh windows`（→ `build.sh server-windows` / `collector-windows`）で組む。中の版の文字列と zip の時刻は回ごとに変わるので、zip のバイト列は同じソースからでも一致しない
- 確かめていない前提が 3 つある（`premises.md` の確かさ 1〜2）: 手元の Windows で配る zip 一式を組めるか・配るインストーラが Sandbox の中で最後まで通るか・外の Claude が Sandbox の中の Edge を操れるか。
  どれも `loss` を持つ決定は依らない。最初の Task で実測し、外れたら下の倒し先へ

## Goals / Non-Goals

**Goals**: 確認バッチの準備の 1 段として、配る形を入れて使う席を、落ちても準備を止めない形で足す。候補は根拠を機械で確かめてから数える。外に出るのは手順書の件数・題・場所だけ。

**Non-Goals**:
- harness2 の変更（手順書の問いの形・録画・/iwakan・agent の起こし方。Q12）
- 携帯の収集アプリを入れて使うこと（依頼で範囲外。ただし材料と報告の形は端末を問わないものにし、後で `android` の席を同じ入口に足せるようにする —— D5）
- 候補の issue 化（Q8）。候補が正しいかの判定（人間が報告を読んで選ぶ）
- 11-19 の違和感を必ず拾えること（Q13。結果は記録するが、完了の判定に使わない）

## Decisions

### D1（仮）. capability は新しい `trial-use` にする

`verify-environment`（検証の準備の DB の作り直し・APK の入れ先）に ADDED で積む案もある。あちらは「検証のたびの環境」の約束で、読む人は Task agent と verify-run。
こちらは「確認バッチの 1 段」の約束で、読む人は確認バッチを読む本人。変える理由も別（Sandbox・Claude・候補の基準）。
反転条件: 確認バッチの準備に関わる約束を 1 つの capability にまとめる別の change が来たら、そちらへ移す（名前は変えられないので、移すときは REMOVED + ADDED）。

### D2（仮）. 配る形は、準備の中で手元の Windows で組む（Q2 の推奨）

席は確認バッチの木を `git clone --local --no-hardlinks` で `C:\dev\ashiato2-trial-build\src\` に写し（`.git` の要る組み立てがある —— `build.sh:48` の `git show HEAD:LICENSE`。`git archive` では落ちる。spec-review R24。`\\wsl$` 越しのビルドは遅い。録画の `C:\dev\ashiato2-rec-build` と同じ型）、
Windows の Git の bash で `tools/release/package.sh windows <置き場>` を走らせる。target は同じフォルダに道具のキャッシュとして残す（2 回目からは差分のビルド）。
組んだ 2 つの zip（`ashiato-server-*-windows-x86_64.zip`・`ashiato-collector-windows-*-x86_64.zip`）だけを Sandbox の読みだけの置き場へ写す。
spec は組み立ての台本の名前を持たない（HEAD から組んだ 2 つの zip であることだけを約束する）ので、倒しても spec は変わらない。
反転条件: 最初の Task で手元の Windows で組めなければ（道具・PostgreSQL の同梱・MSVC の CRT が足りない）、確認バッチの PR の CI の artifact（`package-windows-release`）を待って落とす案に倒す。
そのときは、手順書が渡るのが CI の時間だけ遅れることを、この D に追記する。どちらにも倒せなければ「組めなかった」の警告で終える（席は止まるが準備は止まらない）。

### D3（仮）. Claude は外（WSL）で動き、Sandbox の中を 2 つの口で触る（Q3 の推奨）

- **画面**: Sandbox の LogonCommand（`runner.ps1`）が中の Edge を遠隔のデバッグの口つきで起こし、その口を Sandbox の網の口へ転送する（`netsh interface portproxy`）。
  外の Claude はその口へ、ブラウザを操る道具（Playwright の MCP を CDP の接続先で起こす）で繋ぐ。中の `http://127.0.0.1:18700` の画面は、本人と同じ Edge で開く
- **Windows の操作**（zip を展開・インストーラを起こす・PC の収集アプリを動かす・アンインストール・画面の写し）: 書き込める置き場の中の「頼みの箱」（`C:\trial\out\box\`）に、
  外の Claude が決まった形の頼み（`<n>.ps1` と、終わりを待つ秒数）を書き、中の `runner.ps1` が順に実行して、標準出力・rc・その時点のデスクトップの写しを `<n>.out` に書く。
  外の Claude には「頼みを書いて結果を待つ」小さな道具（`tools/trial-use/box.sh`）だけを許す。`box.sh` は時計を変える命令（`Set-Date`・`w32tm`・`date`・`time`）を含む頼みを断る（spec）
- Claude の資格情報は Sandbox に持ち込まない。Sandbox の中には Claude も Node も入れない
- 倒し先（最初の Task で外から Edge に繋げなければ）: Q3 の第 2 案 —— 中に Node と Playwright を入れ、外の Claude が「頼みの箱」で 1 手ずつ命令を渡す。資格情報は外のまま。
  倒したら、この D を書き換え、`provisional.py` の台帳の反転に記録する（仕様は変わらない —— spec は Claude の動かし方を書いていない）
- 昇格の確認（UAC）が出るなら、それは中のデスクトップに出て Edge からは押せない。Sandbox の利用者は管理者なので、`runner.ps1` が昇格したまま動けば出ない見込み（最初の Task で確かめる）

### D4（仮）. Claude は席の台本が `claude -p` で起こす（グラフの外。Q12 の推奨に従う）

`verify-prep.sh` はグラフの `verify_batch.sh` の中で走るので、グラフの `agent()`（利用上限の扱い・記録）を通らない。harness2 に手を入れないので、席の台本が直に `claude -p` を起こす。
- 作業場所・引数・許す道具の約束は spec（「Claude はリポジトリの外で、限った道具だけで起こされる」）。作り方: `box.sh` をリポジトリの外の道具の置き場（`$TRIAL_USE_HOME/<tag>/tools/box.sh`。材料の置き場とも作業場所とも別）に写し、`--allowedTools` にブラウザを操る MCP の道具・`Bash(<写した box.sh の絶対パス>:*)` の 1 つ・候補の置き場への `Write` を並べる。作業場所は `mktemp -d` の下に材料を写したもの。リポジトリのパスはどこにも渡さない
- Claude が動けなかったときに起こし直さないのは、利用上限が数時間戻らないから（約束は spec）
- 反転条件: harness2 に「確認バッチの準備から agent を起こす口」ができたら、そちらに乗り換える（Q12 で範囲を広げると答えたときも）

### D5. 席の入口と、準備との境目

- 入口は `tools/trial-use/seat.sh <tag> <manifest のパス>`。**どう終わっても rc=0** で、manifest の末尾に空行を 1 つ挟んでから「使ってみた」で始まる 1 段落を足す（`>>`。挟まないと既存の最後の段落に繋がる）。段落の中に空行を入れない
  中で本体（`tools/trial-use/run.sh`）を時間の上限つきで呼び、本体の rc・止まった理由を段落にする。本体を差し替える環境変数（`TRIAL_USE_BODY`）は自己検査だけが使う
- `verify-prep.sh` には、manifest の heredoc の後・`== 出来た` の前に 1 行だけ足す: `./tools/trial-use/seat.sh "$tag" "$out/manifest.md" || echo "warn: 使ってみる席の入口が落ちた"`
  （`set -e` の下でも準備を止めない二重の守り。既存の段の順・成果物は変えない）
- 段落の見出しの語を「使ってみた」に固定する。手順書では 1 つの note になり、人間は冒頭の manifest の最後に読む
- 端末を問わない形: 本体は `--platform windows` を受け、材料・報告・段落の作り方は platform に依らない。携帯は後で `--platform android` を足す（この change では作らない）

### D6（仮）. Claude に渡す材料と指示（Q4・Q5 の推奨）

- 材料の置き場（Sandbox に読みだけで渡す `C:\trial\in\` と、Claude の作業場所の写し）: 目的の一覧 `goals.md`・2 つの zip・合成の書き出し（Takeout の形の zip）だけ
- 目的の一覧は `tools/trial-use/goals.md`（追跡する。手順・順・部品の名前を書かない）。例: 「入れて、合言葉を決め、昨日までの自分の行動を見る」「Google の書き出しを入れて、前の週を振り返る」
  「設定を見て、自分に合うように変える」「新しい版が出ていないか確かめる」「いったん消して、入れ直す」「PC の収集アプリも入れて、しばらく PC を使う」
- 合成の書き出しは `tools/trial-use/make-export.py` が毎回組む（7 日分の YouTube の視聴・検索。日付は席を走らせた日の前の 7 日。録画の `archive_prepare` の形と同じ）。偽データの台本（`tools/seed.sh`）は渡さない
- 指示（`tools/trial-use/prompt.md`）: 「普通の利用者として、目的を自分の順でこなす。気になったことは候補の置き場に書く」。候補に挙げるもの（画面に出たエラー・同じ失敗の繰り返し・行き止まり・
  説明なしに 10 秒を超える待ち・表示のはみ出し）と、添える根拠（D7 の 4 つ）、もう 1 回同じ操作をして確かめること、Sandbox に固有の違い（ダウンロードの印・Tailscale・遅さ・書き出しの小ささ）を書く。
  好みの話（色・言い回し）は挙げない。時計を変える頼みは書かない
- 反転条件: 回を重ねて、目的の一覧に無い操作（毎日はしない操作）に一度も届かないなら、目的の書き方を変える（Q4 の第 3 案には倒さない）

### D7（仮）. 候補の形と機械の確かめ（Q6 の推奨）

- Claude は候補を 1 件 1 ファイル（`candidates/<n>.json`）で書く: `title`・`steps`（直前の操作の列）・`expected`・`seen`・`screenshot`（手元の写しのファイル名）・`again`（`happened` / `not_happened`）・
  `sandbox_note`（任意）・`waited_sec`（待ち時間の候補だけ）
- `tools/trial-use/report.py` が確かめる: 4 つの根拠（写しが実在・`steps` が 1 つ以上・`expected` と `seen` が空でない・`again` がどちらか）が揃わない候補は「根拠が欠けたもの」へ
- 既知の照合（D8）の後、並べる: `again: happened` → `not_happened`（「1 回だけ」）。既知は数えない。手順書には上から 10 件、手元の報告には全件
- 待ち時間の候補の題には「（Sandbox で測った）」を付ける
- 反転条件: 確認バッチで人間が「候補のうち違和感でなかったもの」を続けて多く返すなら、上限か基準を下げる（Q6 の第 3 案に寄せる）

### D8（仮）. 既知の照合は、使い終えた後に別の短い呼び出しで行う

使っている間の Claude には開いている issue を見せない（Q4 の「前の回の結果は見せない」と同じく、知っている所を探しに行かないため）。
使い終えた後、席が `gh issue list --label 違和感 --state open --json number,title,body`（読むだけ）で一覧を取り、候補と一覧だけを渡した 2 回目の `claude -p`（道具なし・出力は JSON）に
「同じものの番号」を返させる。読めなかったときの振る舞いは spec。
反転条件: 照合の誤り（違うものを既知とした）が確認バッチで返ってきたら、照合を人間に任せ、候補には候補らしい issue の番号を並べるだけにする。

### D9（仮）. 出所の比べ（Q11 の推奨。zip のバイト列では比べない）

zip は同じソースでも版の文字列と時刻が変わる（Context）。比べるのは、組み立てに入るものの木の hash: `git rev-parse HEAD:<path>` を
`crates` `web` `migrations` `Cargo.toml` `Cargo.lock` `.cargo` `LICENSE` `tools/db-roles.sql` `tools/release` `tools/trial-use` について並べた文字列の sha256
（spec-review R13。一覧は `tools/trial-use/sources.txt` の 1 か所に置き、自己検査が `tools/release/build.sh`・`package.sh` が `$root/` から読む・`git show HEAD:` するパスが一覧のどれかに含まれることを突き合わせる）。
前の回の値・tag・段落・終わり方は手元の置き場の `last.json` に置く。「走れた」に数える終わり方は `一通り終えた` と `時間の上限で打ち切った` だけで、それ以外（`Claude が動けなかった` を含む）の回の後は同じ出所でも走る（利用上限の回が写され続けないように）。tag が `probe-` で始まる回は読みも書きもしない（spec。手で試した 1 回が確認バッチの回を写しにしないように。spec-review R18）。
反転条件: 出所が同じなのに見え方が変わる原因（同梱の PostgreSQL の版の取り直し・配布元の更新）が見つかったら、その入力も比べる値に入れる。

### D10（仮）. 時間の上限は 45 分（Q9 の推奨）

`timeout --kill-after=30s 2700 claude -p …`。上限は `TRIAL_USE_LIMIT`（秒）で変えられる（自己検査は 2 秒）。打ち切った後も、それまでに書かれた候補と写しで報告を作る。
組み立て（D2）・Sandbox の起動・片付け・照合（D8）は 45 分の外。組み立てには別に 60 分の上限（約束は spec）。手順書が遅れるのは、初回で最大およそ 45 + 60 + 数分、2 回目からは 45 分と差分のビルドと数分（Q9 の detail もこう直した。spec-review R4）。
反転条件: 一通り（インストールから入れ直しまで）に届かない回が続くなら延ばす。手順書が遅れることへの不満が出たら縮める。

### D11. Sandbox の起こし方と片付け

- `.wsb` は `Networking` を既定（開ける。Q10 の推奨）、MappedFolders は `C:\trial\in`（読みだけ: Claude に渡す材料だけ —— 目的・2 つの zip・合成の書き出し）・`C:\trial\bin`（読みだけ: 席の道具 `runner.ps1`。材料と分ける。spec-review R25）・
  `C:\trial\out`（書ける: 写し・ログ・頼みの箱 `box\`）、LogonCommand は `C:\trial\bin\runner.ps1`
- 錠: 手元の置き場の `.lock` に `flock -n` を掛ける。取れなければ起こさない（Q12 の推奨の「ashiato2 の側の錠」。spec-review R5）。録画は harness2 の側なのでこの錠を見ないが、席は準備の中で同期に走り、録画は準備の後なので重ならない
- 起こす前に、動いている Sandbox のプロセス（`WindowsSandbox*`）を問う（録画の `record-run` の `sandbox_pids` と同じ問いを ashiato2 の側に写す。harness2 は変えない）
- 起こした後に増えたプロセスを控え、終わり（`trap` で、正常・失敗・打ち切りのどれでも）に `wsb stop`（あれば）か控えたプロセスの停止で閉じ、残っていないかを問い直す
- 閉じる前に、`runner.ps1` に頼んでログ（サーバ・同梱の DB・更新係・インストーラ）を `C:\trial\out\logs\` に写させ、回の間に `runner.ps1` が 30 秒ごとに `http://127.0.0.1:18700/healthz` を問い、一度でも応えたかを `C:\trial\out\healthz.txt` に書く（インストールまで届いたかの機械の確かめ。使い終えた後の 1 回だけでは、アンインストールの後に終わった回を誤って「届かなかった」にする。約束は spec）
- 時計は動かさない

### D12（仮）. 置き場と手順書の段落（Q7 の推奨。A）

- 手元の置き場: `C:\dev\ashiato2-trial-use\<tag>\`（WSL からは `/mnt/c/dev/ashiato2-trial-use/<tag>/`。自己検査は `TRIAL_USE_HOME` で一時の場所）。
  `report.html`（候補の全件・根拠・写し・違和感メモ・根拠が欠けたもの・巡った目的・取り出せなかったログ）・`shots/`・`logs/`・`transcript.jsonl`（Claude の操作の記録）・`candidates/`
- 段落の形（手順書は段落の中の改行を空白に潰す —— `.harness2/scripts/verify_checklist.py:583-586`。1 行で読める形にする）: `使ってみた（Windows Sandbox・<所要> 分・<終わり方>）: 違和感の候補 <N> 件（手順書には上位 <M> 件）。` に続けて題を `①… ②…` の番号つきで 1 行に並べ（印つき）、`巡った目的: install・login・…`、
  `報告: C:\dev\ashiato2-trial-use\<tag>\report.html`。本文・根拠・画像名・ログの中身・Claude の自由な文（題を除く）は書かない（約束は spec）
- 終わり方の決まった語: `一通り終えた` / `時間の上限で打ち切った` / `Claude が動けなかった` / `組めなかった` / `別の Sandbox が動いていた` / `別の席が走っていた` / `Windows Sandbox が無い` / `本体が落ちた` / `前の回（<tag>）の結果`
- Q7 の推奨は「件数・題 1 行・場所だけ」。巡った目的は Q13 の推奨「0 件でも見たもの」を満たすために足すが、Claude の文ではなく目的の一覧の札（追跡する `goals.md` の固定の語）だけにし、
  所要と終わり方も機械の値だけにする（spec-review R1。Q7 の context と推奨の irreversible にも書いた）。題は 1 行 60 文字・`tools/check-private.sh` の型は伏せる（spec-review R2）。
  反転条件: Q7 で本人が「題も出さない」側を選んだら、題を外して件数・札・場所だけにする。札も出したくないと言われたら札を外し、0 件の見分けは報告の場所だけに任せる
- 置き場は消さない（寿命は手で消すまで）。リポジトリの外なので commit されない

### D13. 違和感メモ（Q8 の推奨。A）

報告の各候補に、`iwakan_widget.py` と同じ形のメモを付ける: `=== 違和感メモ ===` / `ページ: <report.html の場所>` / `表題: CH21 使ってみた <tag>` / `場所: 候補 <n>「<題>」（写し: <shots の場所>）` /
`画面: Sandbox` / `日時: <見た時刻>` / `何が引っかかったか:`（`seen` と `expected` から）/ `こうなってほしい:`（`expected`）。報告の HTML にはコピーのボタンを付ける。席は GitHub に書き込まない。

### D14. 自己検査と CI

`tools/trial-use/check-trial-use.sh --self-test` が、偽の Windows（PowerShell・Sandbox のプロセスの問い・`wsb`）・偽の組み立て・偽の Claude・偽の `gh` を `PATH` の先頭に置き、
spec の Scenario を 1 本ずつ `echo "Scenario: …"` の印で確かめる。Windows も Claude も要らないので、出し分けの無い CI の `chain` job で走る（md だけの変更でも走る。spec-review R20）。
本物の Sandbox と Claude で 1 回通すのは、手元の `tools/trial-use/seat.sh probe-<日付> <一時の manifest>`（tasks の最後の Task。証跡は verify-run）と、確認バッチの実地（人間の確認待ち）。

### D15. ADR

「確認バッチの前に、配る形を入れて筋書きなしで使う席を置く。候補は根拠を機械で確かめ、外には件数・題・場所だけを出す」は Story をまたぐ道具の規則なので、
`docs/adr/` に 1 つ書く（`docs/adr/README.md` の「いつ書くか」）。中身は D3・D7・D12 の要約と、退けた案（Q7・Q8 の第 2・第 3 案）と反転の条件。

## Risks / Trade-offs

- [Sandbox の窓が本人の画面に出て、最大 45 分＋組み立ての間、Sandbox を占める] → 出所が変わったときだけ走る（D9）。本人が自分の Sandbox を起こしていれば席は走らない（D11）
- [遠隔のデバッグの口を Sandbox の網の口に開ける] → 口は Sandbox の仮想の網（手元の機械と Sandbox の間）にだけ出る。Sandbox を閉じると消える。中の画面に映るのは合成の記録だけで、本人の記録は無い
- [網を開けるので、中の Windows・更新係が外に出る] → 本人の PC と同じ状態を作るため（Q10）。中に資格情報は無い（D3）
- [Claude の利用枠を使い、上限で止まる] → 警告で終える（D4）。再試行しない
- [筋書きなしなので、回ごとに巡る所が違い、拾える違和感が揺れる] → 巡った目的を段落に出し、0 件でも何を見たかが分かる（spec）。完了の判定に「拾えること」を入れない（Q13）
- [組み立ての初回が長い（同梱の PostgreSQL の取得・release のビルド）] → 組み立てに別の上限（D10）。target を道具のキャッシュとして残す（D2）
- [インストーラが Sandbox で通らない（UAC・仮想アカウント）] → 最初の Task で実測。通らなくても席は走り、インストールまで届かなかったことを候補とは別の警告として段落に書く（spec）。報告は警告の回も作る
- [手順書に出る題 1 行は、公開の repo と PR の本文に出る] → Q7 で本人が決める（A）。材料に本人のものは無い（spec の「材料の置き場には…何も無い」が機械で縛る）。題の長さと私的な値の形は機械が縛る（D12）。
  それでも Claude の文なので、言い回しは機械では縛れない

## Migration Plan

足すだけ。戻すときは `verify-prep.sh` の 1 行を消す（`tools/trial-use/` は残っても呼ばれない）。手元の置き場は手で消す。

## Open Questions

（無し。確かめていない前提は最初の Task が実測し、外れたら D2・D3 の倒し先へ）
