# CH22 design — 配る形を外につながる状態で入れて置き、落ち続けや誤りの続きを PR の CI で捕まえる

## Context

- `install-windows`（`.github/workflows/ci.yml:598` から）は、最初に `tools/release/update-check-install.sh --block-outbound` で
  更新係（`C:\Program Files\ashiato\ashiato-updater.exe`）だけの外向きを塞ぎ（loopback 以外。規則の名前 `ashiato-update-check-ci-outbound`）、
  `install-check.sh`・`update-check-install.sh`・`update-apply-install.sh` を配る zip と試験の zip で走らせる（ST57 D18）。最後の段は試験の zip を入れたまま終わる（premises P9）
- 更新係は起きたときと以後 1 時間ごとに、前回の確かめから 24 時間経っていれば 1 回問い合わせる（届かなかった回も確かめた時刻を書く。P1）。
  `ashiato-setup.exe uninstall` は状態の置き場と更新係の置き場（`status.json`・`check.json`）を残す（P9）
- 本物の配布元（`api.github.com/repos/rai-256/ashiato/releases?per_page=20`）は今、200 で空の一覧を返す（P5）。正常なら記録は「確かめた。新しい版なし」
- PC の収集アプリは、初めて起きたときは案内を出すだけで確かめず、2 回目の起動で起きてすぐ確かめる。通知の差出人を登録できなければ確かめる係を付けない（P10）
- サービスの回復は 10 秒ごとに起こし直す（`reset= 86400`。4 回目以降の規則は確かめていない。P2）。サーバの起動の拒否は終了コード 0 で止まり、起こし直されない（review/deep.md R2）
- 判定が NG のときの観測を出す部品は `tools/release/check-ng-observations.sh`（CH16。`die`・`die_seen`・命令の記録・秘密の伏せ）
- 確かさ 1 の前提が 3 つある（P2 の後半・P8・P11）。どれも判定の閾値と数え方（B）にだけ効くので、最初の走りで見たものを記録して反転条件にする

## Goals / Non-Goals

**Goals**: 普通に入れて置いておけば気づける壊れ方（落ち続け・止まったまま・同じ誤りの続き・配布元に聞けない）を、PR の CI の 1 段で機械が落とす。

**Non-Goals**: 長く置く試験（毎晩 30 分）・本物の答えの形の比べ（CH20）・製品のコードの直し・PC の収集アプリの送る道（送り先の設定）・携帯の収集アプリ。

## Decisions

### D1（仮）. 置く段は新しい台本 `tools/release/soak-install.sh` に置き、組み立ての鍵が変わることを受け入れる

- 口: `soak-install.sh <配るサーバの zip> <配る PC の収集アプリの zip>`（実地）・`--self-test`・`--list`。形は `update-apply-install.sh` に合わせる
  （実地の確かめの前に `echo "== Scenario: <名前>"`、`--list` はそれを 1 行ずつ出す。自己試験は `# Scenario:` のコメントで印を置き、`== Scenario:` を出さない）
- CI の鍵（CH17）は、除く名指しにこの台本が無いので、台本だけを直した push でも Windows の配布物を組み直す。CH17 の Requirement は改めない（範囲を越える）
- `install-check.sh` などに口を足す形は取らない（3 つの台本はどれも置く段と別の場面の Scenario を持ち、`--list` の突き合わせが混ざる）
- 反転条件: 置く段の台本だけを直す push が続き、組み直しの時間が目立つと分かったとき（鍵の除く名指しに足す change を立てる）

### D2（仮）. 消すのは台本の掃除で、対象は `update-apply-install.sh` の `reset_machine` と同じものに PC の収集アプリを足す

- 止めて消すサービス: `ashiato-updater`・`ashiato-server`・`ashiato-db` と開発版の 3 つ。残ったプロセス（`ashiato-updater.exe`・`ashiato-server.exe`・`postgres.exe`・`ashiato-collector-windows.exe`）を終わらせる
- 消す置き場: `C:\Program Files\ashiato`（と `-dev`）・`C:\ProgramData\ashiato`（と `-dev`。所有者を管理者に移してから）・`C:\ProgramData\ashiato-updater`（と `-dev`）・
  アンインストールの登録簿の鍵・PC の収集アプリの置き場（既定の `%LOCALAPPDATA%\ashiato\collector`（`crates/collector-windows/src/config.rs:145-148`）と、zip を開いた置き場）
- 消した後に `ashiato-server.exe`・状態の置き場の `settings`・更新係の置き場の `status.json` が無いことを確かめ、残れば入れずに落とす
- 台本の掃除は `update-apply-install.sh` から写す（`source` しない。あの台本は読み込むと実地の確かめを走らせる形）。共有の部品に切り出すのは、3 つ目の写しが要るとき
- 反転条件: 写した掃除が `update-apply-install.sh` の掃除とずれて、前の段の残りで置く段が落ちたとき（共有の部品に切り出す）

### D3. 順序は「消す → 塞ぎを外す → 入れる → PC の収集アプリを起こす → 置く」（深掘り Q5）

- 塞ぎの規則は `netsh advfirewall firewall delete rule name=ashiato-update-check-ci-outbound` で消し、`show rule` で無いことを確かめてから入れる（残れば入れずに落とす）
- 入れるのは配る形のサーバの zip（`package/package-windows-release/ashiato-server-*.zip`）。`install-check.sh` と同じインストールの口と合言葉の渡し方を使う
- 更新係は入れた直後に本物の配布元へ 1 回聞く（置き場を消したので前回の時刻が無い。P1）
- 置く段の始まり（確かめた記録の時刻と比べる）= 塞ぎの規則を消す直前の時刻（UTC）

### D4（仮）. 起こし直しは、5 秒ごとの状態とプロセスの番号と、システムの記録の 2 つから数える

- 置く間、5 秒ごとに `sc.exe queryex <サービス>` で状態と PID を読む。実行中でない読みがある・PID が前の読みと違う、をそのサービスの「起こし直し・止まり」として数える
  （回復は 10 秒待ってから起こすので、5 秒ごとの読みは止まっている間を必ず 1 回は見る）
- 加えて、置く段の始まりより後のシステムの記録の Service Control Manager の 7031・7034（サービスが予期せず終わった）を `wevtutil` で読み、サービスの名前ごとに数える
- 2 つの数のどちらかが 1 以上なら落とし、サービスごとに両方の数を出す
- システムの記録を読めたか（読めたなら置く間の件数）を、落ちない回にも 1 行出す。読めなければ `::warning::` を出し、状態と PID の読みだけで判定する（spec に書いた。P11 を緑の run からも読めるように。review/spec.md R6）
- 反転条件: 最初の走りで、runner で `wevtutil` が読めても 7031/7034 が起こし直しのときに出ないと分かったとき（P11）。システムの記録の読みを外し、spec の Requirement「置く間にサービスが…」の本文からも外す（D15 の扱い）。
  P2（4 回目以降は止まる）は置き終わりの状態の判定が捕まえるので、どちらでも判定は変わらない

### D5（仮）. 置く段の始まりは「塞ぎの規則を消す直前」、置く間は「入れ終わって healthz が 200 になってから 5 分」

- 5 分は本人の決定（「5 分ほど」）。入れ終わり = インストーラが 0 で戻り、`http://127.0.0.1:18700/healthz` が 200 を返した時刻（150 秒まで待つ。`install-check.sh` の `wait_healthz 150` と同じ）
- 起こし直しと誤りの行を数える窓はこの 5 分（入れる最中の DB の起動の行を数えない）。確かめた記録は置く段の始まり（D3）より後を数える（更新係は入れた直後に確かめるので、窓の前に書かれる）
- 置き終わりの判定は 5 分の後に 1 回: 3 つのサービスの状態が実行中・healthz が 200・PC の収集アプリのプロセスがある
- 反転条件: 本人が「長く置かないと出ない問題が要る」と決めたとき（別の change で毎晩の形を足す）

### D6（仮）. 同じ誤りは、日時を除いた中身が同じ行が 1 つのログに 3 回以上で落とす。サーバの panic の行は 1 行で落とす

- 3 回は深掘り Q3 の推奨の答え（本人が答えたら、その数を spec に写す。spec の Scenario は 3 行で落ち・2 行で通る）。仮なのは、下の行の切り方と数える段・窓
- サーバの panic の行（`kind="panic"`。`crates/server/src/lib.rs:4583-4586`）は 1 行で落とす。サーバは作業を分けて動かし（`tokio::spawn`）、分けた作業の panic はプロセスを止めないので、
  起こし直しとしては見えない。依頼は panic を判定に挙げている（review/spec.md R8）。更新係と PC の収集アプリには panic の hook が無いので、panic はプロセスの終わり（D4・D7）で見る

- 「日時を除く」: 行の頭の日時（`2026-11-19T13:42:02.123Z` の形と、DB のログの `2026-11-19 13:42:02.123 UTC` の形）と、DB のログのプロセスの番号 `[1234]` を除いて比べる
- 「続く」は 1 つのログの中の回数で数える（間に別の行を挟んでもよい）。0.1.0 型の落ち続けは、起きるたびに同じ行を書く
- 数える段: 更新係の `C:\ProgramData\ashiato-updater\logs\updater.log` は全部の行（誤りしか書かない）、サーバの `logs\server\server-<日付>.log` は `ERROR` の段の行と panic の行、
  DB の `logs\db\postgresql-<曜日>.log` は `ERROR:`・`FATAL:`・`PANIC:` の行、PC の収集アプリの `collector.log` は `ERROR` の段の行。`WARN` は数えない
- 置く間に書かれた行だけを数える（ログの行の日時で切る。日時の読めない行は、置く間の始まりのログの大きさより後に書かれた分を数える）
- 見つけた誤りの行は全部、ログの名前を付けて出す（落とさない 1〜2 回の行も）
- 反転条件: 最初の走りで、何もしないで置いた正常な状態で同じ誤りが 3 回以上出る行が見つかったとき（P8）。段を緩めず、その行が不具合かを D12 のとおり扱う。
  不具合でない（例: 外の揺れ）と分かったら、CH22 の中で数え方（窓・段・行の切り方）を直す（D15）

### D7（仮）. PC の収集アプリは、入れ終わりに起こし、1 分後に 1 回止めて起こし直す

- zip を runner の一時の置き場に開き、`ashiato-collector-windows.exe` を送り先の設定なしで起こす（runner の対話のセッションの利用者。窓のある形。P7）
- 1 分後（案内の通知を出し終える時間。台本の定数）に `taskkill` で止め、同じ実行ファイルを起こし直す。2 回目の起動は起きてすぐ配布元へ確かめる（P10）
- 確かめた記録は既定の置き場（`%LOCALAPPDATA%\ashiato\collector`）の `update-checks.jsonl`、誤りの行は同じ置き場の `collector.log`（`crates/collector-windows/src/main.rs:105`）。`ASHIATO_STATE_DIR` は渡さない（配る形のまま）
- 台本が止めた 1 回のほかに PID が消えた・変わったら落とす（5 秒ごとに `tasklist` で読む）
- 登録できたかは `collector.log` の警告の行 `outage_notice_unavailable` で読む（登録できなければこの行を書き、確かめる係を付けない。`crates/collector-windows/src/main.rs:222-253`）。
  置く段はこの行の有無を落ちない回にも 1 行出す（最初の走りの P7 の観測が CI のログに残る）
- 反転条件: runner で通知の差出人を登録できず、PC の収集アプリが確かめる係を付けないと分かったとき（review/deep.md R3・P7 は通知の登録を確かめていない）。
  これは製品の不具合ではないので、CH22 の中で PC の収集アプリの「確かめた」の判定だけを外し（spec の Requirement「置く間に更新係と PC の収集アプリが…」の本文と、
  Scenario「配る形の PC の収集アプリは、設定せずに起こし直すと本物の配布元に確かめ、動き続ける」を直す）、落ちない・止まらない・誤りの続きの判定は残す（D15）。
  案内の通知が 1 分で済まないと分かったときは、待つ時間（台本の定数）を延ばす

### D8（仮）. 時間の上限は、置く段の手順に 15 分、`install-windows` の job に 100 分

- 置く段: 消す（〜2 分）＋入れる（〜2 分）＋置く 5 分＋判定。15 分で打ち切る
- job: 今の約 61 分（run 44455083173）に置く段を足し、余裕を見て 100 分。上限が無いと固まったときに既定の 360 分を 2 倍で数える（review/deep.md R12）
- 反転条件: 正常な run が上限の 8 割を超えるようになったとき

### D9. 置く段は観測の部品を読み込み、NG と出力の秘密を部品で伏せる

- `soak-install.sh` は頭で `check-ng-observations.sh` を `source` し、`die`・`die_seen`・`ng_report` で落とす（`echo 'NG:` や裸の `exit 1` を書かない）。
  `ng_secret_values` に、状態の置き場の設定の秘密（`ng_settings_secrets`）と台本の画面の合言葉を出させる
- 誤りの行・確かめた記録の行・起こし直しの回数を出すときも、部品の伏せる関数を通す（落ちない回にも出すので、`die` の観測だけでは足りない）
- `--self-test` は `check-ng-observations.sh "$0"` を自分に当てる（CH16 の 2 つの台本と同じ）
- 根拠: リポジトリは公開で、CI のログも公開（review/deep.md R13）。伏せるのは捨てる runner の上の一時の秘密だが、伏せる部品は既にある

### D10. 正典は、本物の配布元へ繋ぐ自動の試験を 2 つ名指しして改める（聞かない既定。review/deep.md R9）

- 本人の決定の字義（「この段と #329 の握手の試験に合わせて改める」）どおりに、例外を 2 つ名指しする。一般の「外に繋いでよい試験」の口は作らない
- 塞ぐ対象も、正典の「外への接続」から「更新係の外への接続」に揃えた。今の D18 の塞ぎ（`.github/workflows/ci.yml:606-610`・`update-check-install.sh` の `block_outbound`）が更新係だけを塞いでいるのに合わせたもので、
  本人の決定の「正典の…を、この段と握手の試験に合わせて改める」の範囲（同じ SHALL の文）に入る（review/spec.md R12）
- PC の収集アプリの `tls_online_handshake_verifies_with_os_roots` は版の一覧でなく根 `/` を叩く（`crates/collector-windows/src/tls.rs:95`）ので、正典の文にそう書く
- MODIFIED に足す 2 つの Scenario（CI の定義を読む）は `soak-install.sh --self-test` が見る。読むのは `.github/workflows/*.yml` の全部で、手順の `run:` の命令だけを数える（注記の行は除く。`ci.yml:255` の注記にも `--ignored` の字面がある。review/spec.md R13）:
  `--ignored`・`--include-ignored` を持つ手順が `tls_` に絞った 1 つだけ、塞ぎを外す（`soak-install.sh <zip>` を走らせる・`delete rule`）手順が `install-windows` の置く段の 1 つだけ

### D11. 深掘りの答えの数は正典に書き、それ以外の閾値は台本の定数と design に置く（聞かない既定。review/deep.md R11・review/spec.md R4）

- 深掘りで本人が答える数（起こし直し 1 回で落とす・同じ誤り 3 回で落とす・確かめた 1 行）は Requirement と Scenario に書く。本人の答えなので、変えるときは合意を聞き直す
- 答えでない閾値（PC の収集アプリの起こし直しまでの 1 分・読む間隔 5 秒・待ちの 150 秒・時間の上限）は台本の頭の定数で、spec は「決まった時間」と書き、design の D 番号を本文から指さない
  （archive の後に正典に残らない）。直すときに正典の MODIFIED が要らない

### D12. 最初の走りで置く段が赤になったら、段を緩めず、直しは別の change にする（深掘り Q7）

- 台本の作りの決まり: 外れを許す一覧（読み込むファイル・環境変数）を持たない。配布元の答えの形を見本と比べない（依頼の「やらないこと」）。PC の収集アプリに送り先を設定しない（D7）。
  観測できる振る舞いではなく作りの決まりなので spec に置かず、code-verify と review が台本を読んで見る
- 赤の原因が製品の不具合なら: 段の判定と閾値を変えない・許す一覧に載せない・製品のコードを直さない（本人の決定）。Task を `BLOCKED` で終え、
  段の出力の抜き書き（どのサービス・どの行・どの記録）を報告と ledger に書く。人間が直しの change を登録し、CH22 の `requires` に足す。CH22 はその直しの後に merge する
- 赤の原因が段の側（台本の誤り・runner の環境）なら: CH22 の中で台本を直す。判定を変えるなら D15 のとおり
- 配布元の揺れ（5xx・時間切れ）で赤なら: 走らせ直す（判定は変えない。Q4 の推奨の答え）

### D13（仮）. 問い合わせの上限だけのときの警告は、GitHub の注記（`::warning::`）で出す

- 上限は共有の runner の IP の都合（P6）なので、job を落とさず、run の要約に残る注記にする
- 反転条件: 警告が続いて置く段が配布元に聞けない run が常になったとき（runner の外の IP から聞く形か、認証つきの問い合わせを考える change を立てる）

### D15. 最初の走りで反転条件に当たったら、CH22 の中で spec・design・tasks を直す

- CH22 は置く段を含めて緑になってから merge する（Q7・完了の判定）。merge の前の CH22 の Requirement は正典に無く、別の change からは直せないので、
  反転（D4・D6・D7）の直しは CH22 の枝の中で行う（review/spec.md R3）
- 直すのが仮の D と台本の定数だけなら、design の D と反転条件の下に何が分かったかを書き、`Ruling:` を ledger に置く（合意は引き継がれる。仮は確認バッチの「合意したときから変わった仕様」で見る）
- Requirement の本文か Scenario を書き直す（判定を外す・数え方を変える）なら、変えた理由と観測を design に書く。仮でない仕様の変わりなので、グラフが合意を聞き直す
- 製品の不具合のときは直さない（D12）

## Risks / Trade-offs

- **配布元の揺れで PR の CI が赤になる**: 外を開けると、配布元が落ちている日・時間切れも赤になる（Q4 の推奨の答えで受け入れた）。走らせ直しで済ませる
- **CI の時間が延びる**: 1 run あたり 7〜8 分（Windows は 2 倍で数える）
- **確かさ 1 の前提（P2 の後半・P8・P11）**: 最初の走りで見るまで、正常な状態でも段が赤になる恐れがある。D4・D6・D7 の反転条件で扱い、見たものは `premises.md` の確かさを上げて記録する
- **PC の収集アプリの通知の登録**（P7・R3）: runner で登録できなければ、製品は正しくても PC の収集アプリの「確かめた」の判定が赤になる（D7 の反転条件）
- **組み立ての鍵**: 置く段の台本の直しで Windows の配布物を組み直す（D1）
