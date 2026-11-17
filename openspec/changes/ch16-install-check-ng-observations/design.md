# CH16 design — 判定が落ちたら観測を必ずログに出す

## Context

- `tools/release/install-check.sh`（1,743 行・`die` の呼び出し 259 か所）と `tools/release/update-check-install.sh`（386 行・54 か所）の
  `die` は `echo "NG: $*" >&2; exit 1` だけ。判定の多くは `sc.exe qc "$service" | grep -Eq … || die …` のように命令の出力をその場で捨てる
- 観測を出しているのは一部だけ: `wait_state` と `wait_db_restarted_by_server` の時間切れが `service_diagnostics`（`sc.exe queryex`・
  `qfailure`・プロセス・DB のログ・イベントログ）を出す。`install_with` の時間切れは自分で `NG:` を出し、出力の末尾とプロセスを fd 9 へ書いて `exit 1`
  （呼び手が標準エラーを `$out` へ向けているので、元の標準エラー fd 9 へ書く）。`common_teardown` は `NG:` を出した後も片付けを続け、元の終了コードを返す
- 秘密は `secret_values`（API の合言葉・ログインの印の鍵・照らすための値・2 つの DB の合言葉）が読める。画面の合言葉は `install-check.sh` が `PW1` など、
  `update-check-install.sh` が字面の `'update-check-test-password-0123'`・`'update-check-release-password-0123'` を持つ。
  実地の確かめは、作り直した後の前の値も命令に渡す（`-H "Authorization: Bearer $old_token"`・`-H "Cookie: ashiato_session=$cookie_old"`）
- `update-check-install.sh` は `STATE`・`SETTINGS` を持たず（設定は `C:/ProgramData/ashiato/settings/server.json` を直に書く）、`URL`（18701）は
  偽の配布元の受け手で、サーバの healthz ではない
- 2 つの `--self-test` は Linux の CI の `chain` の job（`.github/workflows/ci.yml` の 609〜610 行）で走る。Windows の実地は `install-windows` の job
- ST58 は自分の新しい台本 `update-apply-install.sh` で、`die` が Scenario の置き場のファイルの末尾を出す形と、判定が観測を `die` に添える形
  （`die $'理由\n'"$observed"`）を先に作った（`origin/feat/st58-update-apply-rollback`）。この change はそれを 2 つの台本へ一般にする
- `set -e` は `self_test` の本体でも効く。効かないのは `$(…)` の中・`if ( … )`（`expect_fail`）の中・`||` / `&&` の左

## Goals / Non-Goals

- Goals: `NG:` を出して終わる道の全部が観測を出す（判定ごとの書き忘れが起きない形）。出さない道を機械で落とす。秘密は出さない
- Non-Goals: 判定の条件を直す・判定を足す（D7 の確かめを除く。D7 は道具自身の確かめで、製品の判定の条件は変えない）・CI の job を変える・製品のコードを変える・ST58 の新しい台本を検査の対象に入れる・
  `set -e` で判定の外の命令が落ちて止まる道に観測を出す（D8）

## Decisions

### D1（仮）. 観測は口が出す。判定ごとに書かせない

観測を出す口を 3 つ置く:

- `ng_report "<理由>" [<観測>] [fd]`: `NG: <理由>` の行と観測の塊を、指定の fd（既定は標準エラー）へ出して**戻る**（exit しない）
- `die "<理由>"`: `ng_report` を呼んでから `exit 1`
- `die_seen "<理由>" "<観測>"`: 判定が見た値を渡して `die` と同じ

259 + 54 か所の `die` の呼び出しは書き換えない。`common_teardown` は `ng_report` で出して片付けを続け、`install_with` の時間切れは
`ng_report … 9` で fd 9 へ出して `exit 1`（この行に `# ng-observations: 呼び手が標準エラーを向け替えるので fd 9 へ出す` の印）。
`console_probe` の `exit 2`（PowerShell が無い）は印 `# ng-observations: 窓の試験を走らせられない環境` で残す。
塊は `== 観測（ここから）` と `== 観測（ここまで）` の行で挟む。

- 反転条件: 観測の塊が CI のログを読めなくするほど長い（1 回の NG で 400 行を超える）と分かったら、共通の様子を時間切れの判定だけに絞る

### D2（仮）. 直近の観測の命令の出力を記録して、口が末尾を出す

`sc.exe`・`icacls.exe`・`netstat.exe`・`reg.exe`・`curl` を同じ名前の bash の関数で包み（`sc.exe() { ng_observed sc.exe "$@"; }` の形）、
`update-check-install.sh` の `windows_sc`・`windows_icacls`・`windows_netsh` も同じ記録を通す。記録する口は:

1. 命令の標準出力と標準エラーを別々の一時ファイルに受け、命令の終了コードを取る（命令はパイプに書かない）
2. 標準エラーを呼び手の標準エラーへ、標準出力を呼び手の標準出力へ、そのまま（CR も）書く。書き先のパイプが閉じていても失敗にしない
3. 記録 `$NG_OBSERVED_DIR/observed.log` に名前・引数・終了コード・2 つの出力を書き足す
4. 命令の終了コードを返す

だから、パイプが途中で閉じても終了コードは命令のものになる（直に呼んだときに Windows の命令が閉じたパイプへの書き込みで落ちていた場合とは違いうるが、
対象の命令の出力は 64 KiB より十分小さく、今の判定でパイプが閉じる前に書き終わる）。2 つの出力を別々に返すので、呼び手が `2>&1` で 1 本に受けたときの
行の交ざり方は直に呼んだときと違いうる（`2>&1` で受けるのは表示だけの `service_diagnostics` と、`>/dev/null 2>&1` で終了コードだけを見る判定で、判定の入力は変わらない）。
`NG_OBSERVED_DIR` が無ければ記録はせず、命令をそのまま呼ぶ。口は記録の直近 12 件（1 件 40 行まで）を出す。
待ちのループ（`wait_state`・`wait_healthz` など）は 1 秒ごとに同じ命令を呼ぶので、**直前と名前・引数・終了コード・出力が同じ記録は書き足さず、回数だけ数える**
（`（同じ記録 ×<n>）`）。そうしないと、待ちの後の NG で 12 件が同じ行で埋まり、判定が見た出力が押し出される。
記録は書くときにも D4 の伏せ方で伏せる（`curl --data` の合言葉などを素のまま残さない）。記録の置き場は `NG_OBSERVED_DIR`（`$WORK` か自己試験の置き場）だけで、
状態の置き場・プロファイル・登録簿には書かない（実地の確かめの秘密の判定 —— `$STATE`・登録簿・インストーラの出力を探す —— に記録が引っかからない）。`$WORK` は `common_teardown` が消す。

- なぜ: ST58 の誤り（`sc query` の 4 行目の状態を読めていない）は、判定が読んだ生の出力があれば 1 回のログで分かった。
  共通の様子（D3）だけでは、ACL・待ち受け・登録簿のように判定ごとに違う命令の出力が残らない
- 反転条件: 包んだ関数が判定の入力を変える（自己試験か実地で、直に呼んだときと違う結果）と分かったら、その命令を包むのをやめ、その判定は `die_seen` で値を渡す

### D3（仮）. 実地の確かめの中の NG は、共通の様子を添える。入力は台本が名前で渡す

台本の変数の名前を推して読まず、台本が次の名前で渡したときだけ出す:
`NG_STATE_DIR`（状態の置き場。サーバのログは `logs/server` の最新）・`NG_SETTINGS_FILES`（設定のファイル。キーの名前だけ）・
`NG_HEALTHZ_URL`（サーバの healthz）・`NG_OBSERVED_DIR`（命令の記録と、台本が判定のために書く出力のファイルの置き場）。
出すもの: `ashiato-db`・`ashiato-server`・`ashiato-updater` の `sc.exe queryex` と `sc.exe qc`・healthz の HTTP の番号・設定のキーの名前・
`NG_OBSERVED_DIR` の中で最後に書かれた出力のファイル 2 つの末尾 40 行（判定が grep する実名に合わせる: `out`・`*.out`（`install.out`・`repair.out`・`*.ps.out`）・`trace`・
`*.window`・`redirect.*.txt`。合言葉を渡す `*.in`・設定の写し `settings.*`・記録 `observed.log` 自身・展開した `unpacked/` は出さない）・サーバのログの最新の末尾 40 行。

- `install-check.sh` は `common_setup` で `NG_STATE_DIR="$STATE"`・`NG_SETTINGS_FILES="$SETTINGS $OWNER_SETTINGS"`・`NG_HEALTHZ_URL="$URL/healthz"`・`NG_OBSERVED_DIR="$WORK"`
- `update-check-install.sh` は実地の確かめの始めで `NG_STATE_DIR='C:/ProgramData/ashiato'`・`NG_SETTINGS_FILES='C:/ProgramData/ashiato/settings/server.json'`・
  `NG_HEALTHZ_URL='http://127.0.0.1:18700/healthz'`・`NG_OBSERVED_DIR="$work"`（偽の配布元の `URL` は渡さない）
- 観測の命令は 1 つずつ `NG_OBSERVE_TIMEOUT` 秒（既定 30）で打ち切り（`timeout`）、落ちたら `（取れない: <命令> rc=<n>）` と 1 行書いて次へ進む
- 今の `service_diagnostics` はそのまま残す
- 反転条件: 打ち切りの 30 秒が runner で足りない・長すぎる（CI の 1 周が目に見えて延びる）と分かったら変える

### D4（仮）. 秘密は、出す前に決まった値と見出しの形を伏せる

台本が `ng_secret_values` を定義し、伏せる値を 1 行ずつ出す。設定のファイルが**無い**（最初のインストールの前・消した後・設定を退けた Scenario）ときは
字面の値だけを出して rc=0、**あるのに読めない・形が読めない**ときだけ rc≠0:
`install-check.sh` は `secret_values` と `PW1` などの画面の合言葉、`update-check-install.sh` は状態の置き場の設定の秘密と 2 つの字面の合言葉。
口は、塊を出す前にそれらの値と、`Authorization:` の見出しの値・`ashiato_session=` の値（形で見つけるので、前の値も伏せる）を `***` に置き換える。
伏せる値は記録を書くたびと塊を出すときに集め、一度集めた値は走りの終わりまで伏せ続ける（入れ直し・repair・決め直しの前の値も伏せる）。
集めた値は `NG_OBSERVED_DIR` の中の権限を絞ったファイルに置く（`$WORK` には設定の写しが既にあり、`common_teardown` が消す）。
`ng_secret_values` が rc≠0 なら、命令の記録とファイルとログの末尾を出さず、`（伏せる値を読めないので出さない）` と書く（厳しい側）。
ただし台本が字面で持つ合言葉は、設定を読めなくても伏せる値に入れる。設定はキーの名前だけで、値は伏せる前から出さない。

- なぜ: CI のログは外から読めて、一度出た値は消せない。「インストーラの出力とログに秘密が出ない」の判定が落ちたときこそ、出力の末尾に秘密がある
- 反転条件: 伏せる値の集め方で漏れる秘密が見つかったら、その値を集める側へ足す（出す側を広げない）

### D5（仮）. 部品と検査を 1 つの台本に置き、2 つの台本の `--self-test` から当てる

`tools/release/check-ng-observations.sh` は、`source` されると観測を出す部品（D1〜D4 の口と包む関数）を定義し、直に走らせると検査をする
（同じ部品を 2 つの台本に写さない）。検査の決まりは spec の 4 つ。決まり 4 は「`check-ng-observations.sh` を `source` する行が無い」か
「`die()` / `ng_report()` / `die_seen()` の定義が台本にある」で外れる。
`check-ng-observations.sh --self-test` は決まりごとに 1 つの決まりにだけ外れる写しを作って落ちることを見る。
2 つの台本の `--self-test` は自分に検査を当て、`install-check.sh --self-test` は検査の `--self-test` も走らせる。
CI は既に 2 つの `--self-test` を `chain` の job で走らせているので、`ci.yml` は変えない。

- 対象は名指しの 2 つ（`tools/release/*install*.sh` の glob にしない）。ST58 の `update-apply-install.sh` は main に無く、この change の予定のパスの外
- 反転条件: ST58 が main に入り、その台本も同じ部品に寄せると決めたら、別の change で名指しに足す（proposal の持ち越し）

### D6. 判定の条件と標準出力は変えない

`die` の 1 行目（`NG: <理由>`）・終了コード 1・標準出力（`== Scenario:` の行）・`--list` は今のまま。観測の塊は標準エラーにだけ出す。
CI の `install-windows` は標準出力を `tee` して `== Scenario:` を数えるので、標準エラーに足したものは数えに入らない。

### D7（仮）. 観測の塊を本物の Windows で一度出してから実地の確かめに入る

緑の CI は NG の道を通らないので、観測の塊の Windows の部分（本物の `sc.exe`・`timeout`・`curl` を Git Bash で呼ぶ）が壊れていても、
次に判定が落ちた回 —— まさに観測が要る回 —— まで分からない。`install-check.sh` の `common_setup` の末尾（2 つの実地の確かめ `check_release_zip`・`check_test_zip` の始めで呼ばれる）に、
部分殻で `ng_report` を本物の命令に向けて一度走らせ、見出しの 2 行と 3 つのサービスの `sc.exe` の答えの行があることを見る（無ければ `die`）。
出力は標準エラーへ（標準出力を変えない）。`ci.yml` は変えない。

- 反転条件: 実地の確かめの時間が目に見えて延びる（1 分を超える）と分かったら、`--probe-console` の側へ移す

### D8. `set -e` で止まる道は対象の外

判定の外の命令（`unzip` や設定を読む単独の代入）が落ちて `set -e` で止まる道は、`NG:` も観測も出さずに終わる。`trap … ERR` で拾うと、
`$(…)` の中や期待して落とす試しでの振る舞いが読みにくくなるので、この change では足さない。本人の決定の「判定が NG になったとき」は、`NG:` を出す道で満たす。

- 反転条件: CI のログで `set -e` の黙った止まりが観測の不足として実際に起きたら、ERR の trap を別の change で足す

## Risks / Trade-offs

- 同じ名前の関数で命令を包むと、呼び手の見る出力が変わる恐れがある → 自己試験で包む前と後の 2 つの出力と終了コードを比べる（spec の Scenario）
- 期待して落とす判定（`if install_with … ; then die …; fi` や `expect_fail`）の中で包んだ命令が走っても、記録が増えるだけで判定は変わらない
- 観測の塊が長いと、NG の 1 行が埋もれる → `NG:` の行を先に出し、塊を見出しで挟む（D1）
- 伏せる値の外にある秘密（台本が知らない値で、見出しの形も持たないもの）は伏せられない → 設定は値を出さず、出力の末尾は伏せる値を読めなければ出さない（D4）
