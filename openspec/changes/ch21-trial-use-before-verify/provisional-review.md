# CH21 仮で進めた決定の確かめ（provisional_review）

> 2026-11-22。`provisional.jsonl` の生きている D<n>（resolved でないもの）を 1 件ずつ、問いを作った側の「失わない」を信じずに
> design.md・tasks.md・premises.md と既存のコード・harness2 の台本に当てて確かめた。

## D1
- 判定: 確か
- 根拠: `.harness2/scripts/tsukau.py:94-95`（鍵 `out_root` と targets `web`/`pc`/`android` を受ける）・`:245`（`~` を展開する）・`:253-264`（repo の中なら git が無視する場所に限る。`~/.local/state/ashiato2/tsukau` は repo の外）・`:688`（`--label` は `request.json` に残すだけ）。結果は手元にだけ残り、uncaptured / discarded / exported / rewrite-all のどれも起きない。費用は設定 1 行で範囲内。土台の決定は無い

## D3
- 判定: 確か
- 根拠: design.md:55-61。写す側の決定で、閉じる前にログを `$TSUKAU_EVIDENCE/pc-logs/` へ写すので discarded を減らす向き。録画との取り合いは、録画が始める前に動いている Sandbox があれば撮らずに止まり（premises.md「録画は毎回まっさらな…」の行、`.harness2/scripts/record-run:317`）、席も `wsb list` で動いていれば断る（tasks.md:166）ので、録画の「全部止める」（`record-run:285`）が席の Sandbox をログを写す前に止める道は、両方が同時に始める競り合いのときだけ。費用（数十秒・回らない回）は範囲内

## D5
- 判定: 確か
- 根拠: design.md:70-74・tasks.md:30-34（写すときに `.env`・`.env.*`・`collector-android/local.properties` を除き、検証が `test ! -e /mnt/c/dev/ashiato2-tsukau-build/src/.env` を見る。確認バッチの木には `tools/verify-prep.sh:26-29` で本物の `.env` が写されているので、この除きが要るが入っている）。Q7 の答え「準備の中で、確認バッチの木から手元で組む」と同じ向き（`deep-answers-1.txt`）。組めなければ PC の席を回さないだけで失うものは無い

## D7
- 判定: 確か
- 根拠: design.md:91-98・tasks.md:144・152。専用の compose project・乱数の合言葉・`SEED=normal` の偽データで、`<state>/tree`（D9 の使い捨ての木の HEAD）から立てる。`tools/record-env.sh:211-212` と同じ形（`STACK_RESET=1 SEED=normal ./tools/stack.sh up`）で、本人の `.env` と確認バッチの DB に触れない。サーバと画面は `tools/verify-prep.sh:39-46` が組む `dist/verify-<tag>/ashiato-server`・`web/` を絶対パスで受ける（tasks.md:74）。失うものは無い

## D9
- 判定: 確か
- 根拠: design.md:108-113。`.harness2/scripts/init.py:7・437`（`--worktree` は checkout ごとの繋ぎだけで、`.env` を写す処理は無い。grep で `.env` の行が無い）。席の結果は `out_root`（D1）の実行のフォルダに書かれ、消す作業ツリーには残らないので、`git worktree remove --force` で捨てるのは席が木に書いた写しだけ（discarded に当たる記録は無い）

## D12
- 判定: 確か
- 根拠: design.md:131-139。`tools/check-private.sh:5・38-39・84`（`--staged` と `GIT_INDEX_FILE` の一時の index で staged だけを見られる）・`tools/check-secrets.sh` の頭（gitleaks が取れなければ落ちる → D28 で全部伏せる）。伏せた題と本文は手元の `RESULT.md` に残り、公開に出るものが減るだけ。Q4 の答え（題と種類とどの Story だけ）と同じ向き。席が見る記録は D7 の偽データなので、題に本人の記録が入る道は D7 が塞いでいる（D12 の「失わない」は D7 に依らず成り立つ）

## D18
- 判定: 確か
- 根拠: design.md:205-214・tasks.md:263-273。直しの commit・push・issue・手順書は一時の bare repo と偽の `gh` の記録にだけ残る。処置の台本が push 先と `gh` を差し替えられなければ、反転条件（design.md:214「偽の remote と gh では通せない作りになったとき（本人に…聞く。A）」）で本人に返るので、黙って公開に出る道は無い。Q13 の答え（処置まで実地で通す・harness2 側を待つ）と同じ向き。注: 「使い捨ての確認バッチの木の origin を bare repo に」は、本物の repo の `git worktree` では `.git/config` を共有して本物の origin を書き換えるので、clone で作る必要がある（tasks.md:265 は作り方を書いていない。下流の実装で見る点。決定の向きは変わらない）。土台の D15 の 7 が rests_on に無い（下の D15 の否を参照）が、D18 の「失わない」は上の反転条件で D15 に依らず守られる

## D15
- 判定: 否
- 根拠: 条件 4（rests_on に無い土台）。依頼の中身は、項目 3 に D27（`TSUKAU_RECHECK` で 5 分・10 万）、項目 6 に D12（`tools/tsukau/public-title.sh`）と D20（試験の名前と中の文字列）、項目 7 に D18（差し替えて単独で走らせる）、項目 8 に D21（相手・ID・head の組）をそのまま書いている（design.md:172-186）が、`rests_on` は Q1〜Q4・Q11 だけ。`keeps` は「本人の答え（Q1〜Q4・Q11）が推奨と違えば spec --answers が依頼の中身を直す」と言うが、これらの仮の決定が確認バッチで覆ったときに、すでに下流の Task 11 が `intake.py save` で harness2 に置いた依頼（design.md:190）を直す道が台帳から辿れない。rests_on に D12・D18・D20・D21・D27 を足すか、覆ったときに依頼をどう直すかを本人に見せる

## D20
- 判定: 確か
- 根拠: design.md:148-152。根拠の文を試験に写さない側で、根拠は手元の `RESULT.md` に残る。Q4 の答え（題・種類・どの Story だけ）と同じ向き。D12 の台本を通すことが前提だが（rests_on に D12 が無い）、「失わない」（根拠は手元に残る）は D12 の伏せ方に依らず成り立つ

## D21
- 判定: 確か
- 根拠: design.md:216-222・tasks.md:66-90（4.1 の試した記録の突き合わせ）。試した記録は手元に追記のみで消さない。`time`・`tokens` で終えた回も「試した」に数えるので、時間切れで触れなかった範囲のものは次の回に試し直さないが、PR の head は git に残り、手で `/tsukau` を呼べば同じ head を試せるので uncaptured（後から取り直せないもの）には当たらない。D8 の反転条件（design.md:106「席が範囲を一通り触れずに time で終わることが続く」）が受ける。Q11 の答え（新しく入ったものだけ）と同じ向き

## D22
- 判定: 確か
- 根拠: design.md:224-235。`tools/emulator-start.sh:58-66`（`emu_start` は `$ANDROID_HOME/emulator/emulator` を引数そのままで起こす）・`:31-38・69-83`（`emu_stop_own` は `EMU_PID` だけを止める。`down` は別のプロセスなので `<state>` に残した PID を読み直す形で使える）。`-wipe-data` で消えるのは専用の AVD `ashiato-tsukau-api35` の中身だけ。確認バッチの準備の `install_on_emulators`（`tools/verify-prep-device.sh:4-22`）は `tools/verify-prep.sh:60-62` で走り、席の差し込み（tasks.md:131。manifest を書いた後）より前なので、席のエミュレータには入らない。起動から網を切るまでの間の外への接続は D31 の否を参照（D22 の keeps はそれを主張していない）

## D27
- 判定: 確か
- 根拠: design.md:192-197。確かめ直しの結果も実行のフォルダに残り、試した記録を読まず足さないので、直しで進んだ head は次のふだんの回で試し直す（D21）。Q10 の答え（10 分・20 万）はふだんの回の大きさで、確かめ直しは別の呼び出し。費用（1 回 5 分）は範囲内

## D28
- 判定: 確か
- 根拠: design.md:141-146。検査できない回は全部伏せる側で、伏せた題も本文も手元に残る。`tools/check-secrets.sh` の頭は gitleaks が取れなければ落ちる（「黙って飛ばさない」）ので、その落ちを「全部伏せる」に倒せる。D12 の台本の一部だが（rests_on に D12 が無い）、「失わない」は D12 に依らず成り立つ

## D26
- 判定: 確か
- 根拠: design.md:260-269。当てる先はその回だけ立てた一時の口とその回のサーバで、本人のサーバや記録に触れない。確かめられなければ何も入れない。費用（許可の窓が出うる。premises.md の確かさ 1 の行）は Task 8 で確かめ、反転条件がある。注: `rests_on` の D24 は resolved（D31 に立て直し）。携帯の部分は D31 と同じ中身で、D31 が D26 を rests_on に持つ

## D29
- 判定: 確か
- 根拠: design.md:271-277。範囲は bash の数から組み、自己試験が見本の宛先で私設網・私用・リンクローカル・マルチキャストを含まないことを固定する。字面の範囲を書かない（`tools/check-private.sh` が止める）とも合う。Q8 の答え（外にはつなぎ、私設網と手元の網を塞ぐ）と同じ向き。失うものは無い

## D30
- 判定: 確か
- 根拠: design.md:237-247。`collector-android/app/build.gradle.kts:27-28`（`credential()` は gradle の property が先、無ければ `local.properties`）・`:192-200`（`ashiato.baseUrl` は property だけ）・`:252`（debug の更新の宛先は `updateCheckUrl` か空）。`~/.gradle/gradle.properties` の鍵は `ashiato.baseUrl` だけで、`-P` で上書きされる（`updateCheckUrl` は無い）。配る APK の署名の鍵と本人の接続先を使わない。その回の乱数が組み立ての間だけ `ps` に見えるのは、127.0.0.1 の使い捨ての環境にしか効かない

## D31
- 判定: 否
- 根拠: 条件 1（exported が起きる）。`keeps` は「外へも私設網へも手元の網へも何も出ない」と言うが、design.md:252 と tasks.md:214 の手順は、網を持ったまま起動し（D22 の引数に網を断つものは無い。design.md:228）、起動を待ち、切る前の確かめをしてから `svc wifi disable`・`svc data disable` する。起動から切るまでの間、端末は網につながっている（premises.md の `svc` の行「切る前は届き…Active default network は切る前 `101`」）。使う image は `system-images;android-35;google_apis;x86_64`（`tools/android-emulator.sh:39`。D22 は同じ image で AVD を作る）で、`-wipe-data` のまっさらな初回の起動では OS が外（Google の接続確認・時刻合わせなど）へ問い合わせるので、この PC の網の IP と時刻が外に残りうる。premises.md・design・tasks・spec のどこにも、この起動の間の外への接続は書かれていない（grep `gstatic|connectivitycheck|Google|切るまで` は 0 件）。APK を入れる前なので本人の記録は出ないが、「何も出ない」は誤り。起動の時から網を持たせない道を確かめるか、起動から切るまでの間に OS が外へ出すことを本人に見せる（Q8 は PC の Sandbox への答えで、携帯には聞いていない —— design.md:251）

## D33
- 判定: 確か
- 根拠: design.md:305-311。見つかれば席を呼ばない側で、本人の実機に届く席を起こさない。`which -a adb emulator` は `$ANDROID_HOME/platform-tools/adb` の 1 つだけ（`/usr/bin/adb` は無い。2026-11-22 に実行）。土台の D32 は rests_on にある。注: 判定は名前 `adb`・`emulator` だけを見るので、WSL が `PATH` に足す Windows 側の `adb.exe` は数えない（premises.md の最後の行で今は無いと確かめてある）

## D34
- 判定: 確か
- 根拠: design.md:313-322・`harness2-request-seat-path.json`（`seat_hide` は席の環境の `PATH` だけを変え、差し込み口の環境は変えない）・`.harness2/scripts/tsukau.py:94・231-232`（今は知らない鍵で落とすので、入る前に足さない理由が合う）。手で呼ぶ道も閉じる側で、断られても確認バッチの道は D32 で閉じている。CH21 が harness2 を待つ費用は、D18（Q13 の答え）ですでに待つのと同じ（rests_on に D18 がある）。本人は第 3 回の合意で IMP-13 の直し（D32〜D34）に「直した・確かめた結果に同意する」と答えている（`deep-answers-3.txt`・`deep-answers-4.txt`）

## D35
- 判定: 確か
- 根拠: design.md:264-278・tasks.md:212-218・256-259・spec.md（verify-environment）:485-492。条件 1: 箱は `unshare --user --map-root-user --net` で中は loopback だけ（premises.md:48-49 の確かさ 5。`--pid` は分けないので、箱の外の片付けが `<state>` の `EMU_PID` で止められる。`tools/emulator-start.sh:69-83` の `emu_stop_own` は PID だけを止める）。エミュレータの本体のプロセスも箱の中なので、起動の間に端末からも本体からも外へ出る道が無い（exported は起きない）。橋は箱の外で `<state>` の Unix のソケットから `127.0.0.1:<その回のサーバの port>` へだけ渡す（premises.md:53）。`emu_wait_device`・`emu_wait_boot`（`tools/emulator-start.sh:101-140`）は `PATH` の `adb` を呼ぶが、`nsenter … bash` の中では箱の loopback の 5037 番、つまり箱の中の adb のサーバにしか届かないので、この PC の adb のサーバ・実機・ほかのエミュレータには触れない。箱に入れないときに外の adb のサーバへ落とさないことは spec.md:490・492 が決めている。条件 2: 費用は `tools/tsukau` の中の数十行と自己試験（偽の `unshare`・`nsenter`・`socat`。tasks.md:221）。`unshare`・`nsenter`・`socat`・`ip` は `/usr/bin` にある（`which`。2026-11-22）。`max_user_namespaces` は 128345 で、範囲を越えず、要件の本文も変えない。条件 3: `follows` は無い。source の Q-G2 の答え（`deep-answers-5.txt`「起動の時から網を持たせない道を確かめて使う（道が無ければ携帯の相手を回さず本人に返す）」）とは同じ向きで、Task 10 が `up`・`down` を 3 回続けて本物で確かめ、通らなければ A として返す（tasks.md:259）。条件 4: 土台の D31（Q-G2 で本人の決定になった）と D22 は rests_on にある。D25・D32 は台帳に無い本人の決定。D26 は D35 を rests_on に持つ側。注: 箱の中の `adb reverse` の道（premises.md:52 の確かさ 2）は、未確定のまま Task 10 の反転条件に載っている。失うものは無く、通らなければ携帯の相手が回らないだけ。design.md:233 の「この PC の adb のサーバから席のエミュレータは見えない」（premises.md:50 の確かさ 1）が誤っていても、本人のサーバ向けの APK が網の無い席のエミュレータに入るだけで、失うものは無い
