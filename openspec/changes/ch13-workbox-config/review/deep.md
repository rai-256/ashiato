# CH13 の深掘りの独立検査（deep-review）

対象: `openspec/changes/ch13-workbox-config/deep-questions.json`（Q1・Q2）、`premises.md`、依頼 `docs/changes/CH13.md`。
harness2 の作業箱は `.harness2` の `feat/workbox`（HEAD `9b8d1c27a`）を `git show` で読んだ。

## 手順ごとの記録（問いの一覧を見る前にやったこと）

- 手順 1（衝突）: CH13 は `satisfies: []`。依頼の中で衝突する組が 1 つある —— 完了の判定「`workbox.py run … pgrep` を BLOCKED として残す」と、
  ハーネスの Task gate「`BLOCKED_INFRA` なら Task は完了しない」（`.harness2/docs/flow-gates.md:172`）。→ R1。
  正典 `openspec/specs/verify-environment/spec.md:4`（並走する Story と change が同じ機械で証跡を取り続ける）は、箱を有効にした日に R2 の漏れと食い違うが、既定 off の間は衝突しない
- 手順 2（扉の幅）: doors は無い。「書き換えない。足りないものは PR 本文に挙げる」の幅 → R7。spec を持つか（`verify-environment` の capability がある）→ R8
- 手順 3（一方通行）: 置くのは JSON 1 つで、`HARNESS_WORKBOX` が off の間は読まれない（`evidence.py` の `box_config` が off で None）。
  捨てる・外に出す・凍結した全行の書き直しは無い。`loss` のある決定は見つからなかった。ただし harness2 側の形が凍っていない → R9
- 手順 4（日常）: 箱を有効にした日の待ち（席 2 つを e2e が長く握る）・メモリの合計・イメージの溜まり → R6・R10
- 手順 5（既存コードとの突き合わせ）: 依頼の正規表現を、`tools/*.sh` の中身・`crates/server/tests`・`web/playwright.config.ts`・
  `openspec/changes/**/tasks.md` の検証コマンドと grep で突き合わせ、作業箱のイメージ（`harness2-workbox:f1887cd41fb6`）を実際に起こして
  `docker` が無いことを確かめた → R2・R3・R4・R5。依頼の JSON を一時の worktree に置いて `check-private` `check-tmp-hygiene` `check-spdx`
  `check-shared-appends` `check-secrets` `check-boundaries` を走らせ、すべて rc=0（`check-db-secret` は `.env` が無い環境の rc=1 で、ファイルと無関係）
- 手順 6（過去の取りこぼし）: 当てはまる観点は「検証コマンドは凍結前に verify-run で空走させる」（→ R1: 完了の判定の pgrep は今の harness2 main で
  `BLOCKED_INFRA`）と「同じ型の不成立が一度出たら grep で横展開」（→ R2: Q1 は e2e と actionlint の 2 つで止まっている）。
  「秘密を読む者を表に」は R4（docker の socket を渡すと `.aws` `.config/gh` が見えなくなる前提が崩れる）
- 手順 7（族）: Q1・Q2 の族「箱の中で docker が要る検証」の兄弟 —— (a) docker が要るのに字面が当たらない（Q1 は 2 種だけ。R2 で 4 種を足す）、
  (b) 要らないのに字面が当たる（R4）、(c) ホストが要るのに `host_only` に当たらない（R3）、(d) 完了の判定のコマンド自身が箱に包まれる（R1）、
  (e) 箱のメモリの上限で落ちる（R6）。(b)〜(e) は一覧に無い
- 手順 8（前提の確かさ）: `premises.md` の確かさ 4 未満の行（`enabled()` の読み方 2・正規表現の当て方 2・e2e が docker を使う 3・`.d` の節 3・Android の道具 2）に
  `loss` のある決定は載っていない（Q1・Q2 とも open）。前提が誤りうるとして指摘するものは無い。ただし前提の表に無い前提がある → R2（`cargo test --workspace` が docker を使う）・
  R6（メモリ）・R9（harness2 の形）

## R1. 完了の判定の pgrep の確かめは、今のままでは Task gate を通らず、change が harness2 の merge まで止まる。範囲の終わりを問うていない
- 成果物: openspec/changes/ch13-workbox-config/deep-questions.json
- 根拠: docs/changes/CH13.md の「完了の判定」（「feat/workbox が入っていなければ…BLOCKED として残す」）/ .harness2/docs/flow-gates.md:172（Task gate: `BLOCKED_INFRA` なら Task は完了しない）/ `ls scripts/workbox.py` → No such file（`.harness2` は harness2 の main）→ `feat/workbox:scripts/evidence.py:387` の `missing_executable` で `BLOCKED_INFRA` / さらに `HARNESS_WORKBOX=tests` の下で走らせると、このコマンド自身が `docker` の欄にも `host_only` にも当たらず箱に包まれ（`evidence.py:398`）、箱のイメージには docker が無い（`docker run --rm harness2-workbox:f1887cd41fb6 bash -c 'command -v docker'` → 無し）ので `workbox.py:162` の `Unavailable` で落ちる
- kind: conflict
- 提案: 問いを 1 つ足す（B・推奨つき）。「pgrep の確かめを tasks の検証項目に置くか（置けば harness2 の merge まで CH13 は下流で止まる）／置かずに PR 本文と確認バッチに回すか／harness2 の merge を CH13 の着手の前提にするか」。どれでも、確かめるときは `HARNESS_WORKBOX` が off であること（または `host_only` に `scripts/workbox\.py`）を前提に書く
- 処置: escalated — Q3 として本人に問うた（推奨は tasks に置かず宿題に回す）。第 1 回の答え（tasks に置く。harness2 は merge 済み）を spec --answers で D3・tasks 4.1 に写した

## R2. Q1 の挙げる漏れは e2e と actionlint だけで、いちばん多い `cargo test --workspace` と、中で docker を呼ぶ台本 3 本が抜けている
- 成果物: openspec/changes/ch13-workbox-config/deep-questions.json
- 根拠: crates/server/tests/server_startup.rs:1904（`st51_bin_migrate_attempt_survives_restore` が `Command::new("docker")…output().unwrap()`。`#[ignore]` なし）・同 1949（本物の `tools/pg_dump-docker.sh`）/ archive の tasks で `cargo test … --workspace` が 39 行・`server_startup` が 15 行（`grep -h 'cargo test' openspec/changes/archive/*/tasks.md | grep -c -- --workspace`）/ tools/load-repeat.sh:31（`tools/db.sh exec`）/ tools/st25-seed-check.sh:12（`tools/stack.sh up`）/ tools/check-st23-derived-erasure.sh:8（`tools/check-immutable.sh`）。どれも字面に `docker` の欄の正規表現が当たらない（`feat/workbox:scripts/workbox.py:121-129` の `re.search`）。依頼の「確かめたこと」は `--lib access_log_tests` だけで、`server_startup` を含まない
- kind: technical
- 提案: Q1 の `context` と `why` の列挙を、e2e・actionlint に `cargo test --workspace` / `-p ashiato-server --test server_startup`・`load-repeat.sh`・`st25-seed-check.sh`・`check-st23-derived-erasure.sh` を足した一覧に直す。一覧は手で拾わず、tasks の検証コマンドと台本の中身を突き合わせる機械の走査の結果として載せる
- 処置: fixed enable-before.md — Q1 の context と宿題の一覧に cargo test --workspace と 3 本の台本を足した

## R3. `host_only` に当たらないのに Windows かエミュレータが要る検証がある（族の兄弟が一覧に無い）
- 成果物: openspec/changes/ch13-workbox-config/deep-questions.json
- 根拠: tools/release/install-check.sh:374 ほか（`powershell.exe`。検証の字面は `tools/release/install-check.sh` で `powershell\.exe` が当たらない。archive の tasks に `--self-test` でない形が 3 行）/ tools/release/build.sh:226・258（`server-windows` などで `powershell.exe`）/ tools/android-side-by-side.sh:25（`emulator -avd` を自分で起こす。tasks に 2 行）・tools/usage-volume.sh（tasks に 4 行）。箱の中では `/mnt/c` が見えず、エミュレータは検証の箱のメモリ 6g の中で起きる（`workbox.py:245`）
- kind: technical
- 提案: Q1 の族に「ホストが要るのに `host_only` に当たらない」を兄弟として足す（Q1 の選択肢に含めるか、別の問いにする）。扱いは Q1 と同じく B
- 処置: fixed enable-before.md — Q1 の context と宿題の一覧に「ホストが要るのに host_only に当たらない」を足した

## R4. 正規表現が錨なしで字面に当たるので、台本の名前を grep するだけの検証にも docker の socket が渡る
- 成果物: openspec/changes/ch13-workbox-config/deep-questions.json
- 根拠: `feat/workbox:scripts/workbox.py:121`（`re.search`）・同 203（docker を渡すと `/var/run/docker.sock` を rw）/ 走査の結果、tasks の検証コマンドのうち `grep`・`test` で始まり `docker` の欄に当たるものが 9 件（例: openspec/changes/archive/2026-10-24-ch01-st17-review-fixes/tasks.md:70 `! grep -n 'gradle.properties' tools/record-env.sh`）/ `all` のときは同じコマンドが `evidence.py:406` で「docker を要する検証」として断られ、ホストでの取り直しに回る / docker の socket があれば箱から `docker run -v $HOME:…` でホームが見え、依頼の「`.aws` `.config/gh` は見えない」は docker を渡した箱では成り立たない
- kind: technical
- 提案: 族の兄弟として「当たりすぎ」を Q1 に並べる（B）。直し方の候補は「正規表現を `(^|[;&|( ]|bash )(\./)?tools/…` のように呼び出しの位置に錨を置く」「harness2 側で字面でなく実行する台本で判定する（Q1 の 3 番目の選択肢）」
- 処置: fixed enable-before.md — Q1 の context と宿題の一覧に「当たりすぎ」を足した

## R5. `prep_files` に、検証の準備がホストで読む `tools/db-roles.sql` と `docker-compose.override.yml` が無い
- 成果物: openspec/changes/ch13-workbox-config/deep-questions.json
- 根拠: tools/db-roles.sh:45（`< tools/db-roles.sql` を DB の管理者で流す）/ tools/env-init.sh:24（木に `docker-compose.override.yml` があれば `COMPOSE_FILE` に足し、`verify-env.sh` の `docker compose up` がそれを使う。正典 openspec/specs/verify-environment/spec.md:133 も同じ）/ `feat/workbox:scripts/evidence.py:408`（中継から来た依頼は `prep_files` の中身が main と違えば断る。無いファイルは `changed_from_main` が「違う」と読むので、override を挙げれば置いた木で断れる）/ 準備はホストで走る（同 434）
- kind: technical
- 提案: `all` のときだけ効く不足として、Q1 の「PR 本文に挙げる不足」の一覧に載せる（問いは足さなくてよい）。作業箱は「事故の隔離」（`workbox.py` の `serve_relay` の注記）なので A にはならない
- 処置: fixed enable-before.md — Q1 の context と宿題の一覧に足した（問いは足さない）

## R6. メモリと席の前提が `tests` の cargo test でしか測られておらず、e2e と `all` の合計が前提の表に無い
- 成果物: openspec/changes/ch13-workbox-config/premises.md
- 根拠: 依頼の「確かめたこと」は `HARNESS_WORKBOX=tests` の cargo test 5 本だけ / `agent_memory` 4g × 5 + `memory` 6g × `parallel_verify` 2 = 32g（`feat/workbox:scripts/workbox.py:330` が書く WSL の VM は 32GB。ホストの langgraph・DB の容器はこの外）/ 箱の `/tmp` は tmpfs（同 248）で、tmpfs の中身は箱のメモリの上限に数えられる / e2e は `tools/stack.sh up` が release のビルドをしてから Chromium を起こし（tools/stack.sh:64、web/playwright.config.ts:57 の待ちは 10 分）、6g と席 1 つをその間握る / agent の箱の中で agent 自身が走らせる cargo のビルド（`verify-run` を通らない）も 4g に収まるかは測っていない
- kind: daily
- 提案: `premises.md` に「e2e 1 本が 6g の箱で通るか」「`all` で 5 本のときの空きメモリの最低」「agent の箱の中の `cargo build` が 4g で落ちないか」を確かさ 1〜2 の行として足す。どれも `loss` は無いので問いにはしない（有効にする前に測る項目として、R7 の行き先に載せる）
- 処置: escalated — premises.md に確かさ 1 の行として足し、Q1 の宿題として本人に問うた。第 1 回の答え（Q1 (3) 測るものだけを宿題に残す）で enable-before.md の 1 節に残した（D2）

## R7. Q1 は依頼で既に決まっている。開いているのは「不足の一覧をどこに残し、有効にする前に誰が見るか」の方
- 成果物: openspec/changes/ch13-workbox-config/deep-questions.json
- 根拠: docs/changes/CH13.md「書き換えない。足りないものに気付いたら PR 本文に挙げる」「完了の判定: `tools/workbox.json` が上の中身であり」/ Q1 の 2 番目の選択肢（欄に足す）は完了の判定の本文を変えるので、仮（P）で選べる側ではない / 見終えた PR は機械が merge する運用のため、PR 本文に書いた不足は人間の目に触れずに main に入りうる
- kind: defer
- 提案: Q1 を「不足の一覧（R2〜R6）をどこに残すか —— PR 本文だけ／harness2 の issue／`docs/handoff` か新しい change の依頼」に問い直す（B・推奨つき）。2 番目の選択肢は「依頼の本文を変える」と明記して、人間に返す側（A でなく依頼の差し戻し）として分ける
- 処置: escalated — Q1 を「不足の一覧をどこに残すか」に問い直して本人に問うた。第 1 回の答えで本人が依頼を差し戻し（「書き換えない」を外す）、D1・D2・D7（仮）に写した

## R8. spec を持つか（`skip_specs`）と、触らないもの（「ついでに直す」の境界）を問うていない
- 成果物: openspec/changes/ch13-workbox-config/deep-questions.json
- 根拠: openspec/changes/ch13-workbox-config/.openspec.yaml に `skip_specs` が無い / 正典に `verify-environment`（openspec/specs/verify-environment/spec.md:4「並走する Story と change が同じ機械の上で証跡を取り続けるための、開発の道具の約束」）があり、spec の段が Requirement を足すと Scenario の印のある試験か台本が要る（`check_scenarios.py`）= ファイル 1 つの範囲を越える / CLAUDE.md の「構成」の `tools` の行は「ハーネスの差し込み口（`agent-env.sh` `verify-prep.sh` `pre-commit.sh`）」を列挙しており、下流が `workbox.json` を書き足す動機がある / docs/changes/CH13.md の「壊してはいけないもの」は空
- kind: technical
- 提案: 問いを 1 つ足す（B・推奨つき）。「この change は spec を持たない（`skip_specs: true`）か、`verify-environment` に Requirement を足すか」。あわせて「変えないもの」（`tools/*.sh`・`docs/testing.md`・`AGENTS.md`・`CLAUDE.md` の構成表・`verify-environment` の正典）を `context` に列挙し、Q2 の 2・3 番目の選択肢がその境界を越えることを書く
- 処置: fixed D4 — Q4 を足した（範囲と変えないもの。第 1 回の答えで本人が決めた）。spec の段で、openspec の `.openspec.yaml` の `skip_specs: true` を採った。questions の段の「skip_specs はこのハーネスに無い」は誤りで、proposal の Capabilities に訂正を書いた

## R9. 設定の形は harness2 の未 merge のブランチのもので、未知の鍵を黙って無視する。確かめた版が記録されていない
- 成果物: openspec/changes/ch13-workbox-config/premises.md
- 根拠: `feat/workbox:scripts/workbox.py:100`（`load_config` は object かどうかだけを見て、鍵の名前を検査しない）/ harness2 の main に未 merge（premises.md の最後の行）。merge までに鍵の名前が変わると、このファイルの欄は黙って効かなくなり、完了の判定（中身の一致と JSON の parse）では気付けない
- kind: premise
- 提案: `premises.md` に「確かめた harness2 の版（`feat/workbox` `9b8d1c27a`）」の行を足し、harness2 の merge の後に鍵の一覧を突き合わせ直す項目を R7 の行き先に載せる
- 処置: fixed premises.md — 確かめた版（b4797fbbb。レビューの 9b8d1c27a はこの worktree の先頭と違ったので読んだ先頭を書いた）の行を足した

## R10. 作業箱のイメージが溜まる（`apt` を変えるたびに 1 GB 級の新しいイメージ）
- 成果物: openspec/changes/ch13-workbox-config/deep-questions.json
- 根拠: `feat/workbox:scripts/workbox.py:159`（`image_tag` は Dockerfile と `apt` の一覧の hash。古いタグを消す処理は無い）/ `docker images` に既に `harness2-workbox:5188825443bf`（783MB）と `:f1887cd41fb6`（1.25GB）
- kind: daily
- 提案: CH13 の範囲外（harness2 の片付け）。R7 の不足の一覧に 1 行載せる。問いは足さない
- 処置: fixed D7 仮 — harness2 側の話で問いは足さず、宿題の一覧に載せた。第 1 回の答え（Q1 (2)）で harness2 の issue にする（D7（仮））

