# CH06 spec の検査（spec --draft。第 1 回の推奨 Q1〜Q3 を答えとして読んだ版）

対象: `proposal.md` / `specs/verify-environment/spec.md` / `design.md` / `tasks.md` / `docs/changes/CH06.md`（と根拠として `premises.md`・`review/deep.md`・`deep.md`）。
書いた側の意図は知らないまま、成果物どうしと、成果物が当てにしている道具（`tools/`・harness2）の実物の振る舞いだけを見た。

## 機械の検査

| コマンド | 結果 |
|---|---|
| `openspec validate ch06-reset-verify-db --strict` | `Change 'ch06-reset-verify-db' is valid`（rc=0） |
| `python3 scripts/check_chain.py .` | `chain: OK (0 件 / 未回収 0 件 / warn 0 件)`（rc=0） |
| `python3 scripts/check_scenarios.py . ch06-reset-verify-db` | `scenarios: FAIL (担保なし 18 件)`（rc=1）。18 件とも CH06 の Scenario で、実装の前なので担保が無いのは当然。tasks の Global Constraints（2 + 7 + 9 = 18）と Task 1.2・2.2・3.2・5.1 が 18 本を一字一句名指ししていることは突き合わせた |
| `python3 scripts/review_triage.py . ch06-reset-verify-db` | `triage: FAIL (2 件)`（rc=1）: `deep.md R2: kind=conflict は仮でしか閉じられない` / `deep.md R7: kind=daily は仮でしか閉じられない` → R1 |
| `python3 scripts/spec_overlap.py candidates . ch06-reset-verify-db` | `[]`（0 件）→ 観点 7 は該当なし |

追加で手で確かめたもの:
- 点区切りの私設 IP: `tools/check-private.sh` は rc=0 だが、`git ls-files` だけを見るので**未追跡の change の成果物は見ていない**。同じ 5 つの pattern を `openspec/changes/ch06-reset-verify-db/` と `docs/changes/CH06.md` に `grep -rnEI` で当てて 0 件（`10.20[01].*.0/24`・`10.<200+⌊off/256⌋>…`・`172.17〜172.31` は 4 つ組にならない）
- compose の変数の優先（使い捨ての dir、compose v5.5.0）: `.env` に `S=x` を置き、`env -u S` なら `x`、`S=` を export すると `required variable S is missing a value` で落ち、`S=y` を export すると `y`（shell が `.env` に勝つ。空でも勝つ）→ R5
- 明示の subnet の重なり（使い捨ての network 2 つ、作って消した）: 同じ subnet の 2 つ目は `Pool overlaps with other one on this address space` で作れない → R3

---

## R1. deep-review の R2（conflict）と R7（daily）が `fixed premises.md` で閉じられ、仮の形になっていない（triage が FAIL）
- 成果物: openspec/changes/ch06-reset-verify-db/review/deep.md / openspec/changes/ch06-reset-verify-db/design.md
- 根拠: `python3 scripts/review_triage.py . ch06-reset-verify-db` → `[FAIL] deep.md R2: kind=conflict は仮でしか閉じられない` / `[FAIL] deep.md R7: kind=daily は仮でしか閉じられない`。review/deep.md:32・:67 の処置は `fixed premises.md`。受け皿の design D2（C1 の見分け方。design.md:44）と D1（C2 の消し方。design.md:30）は `（仮）` も反転条件も持たない
- kind: technical
- 提案: D1・D2 に `（仮）` と反転条件（例: D2「CI 以外で `CI=true` が立つ経路が見つかったら見分けを `GITHUB_ACTIONS` に替える」、D1「R2 の手順の実測で volume の扱いが変わったら」）を書き、review/deep.md の処置を `fixed D2 仮` / `fixed D1 仮` に直す
- 処置: fixed D1 仮 —— design D1（仮）・D2（仮）に反転条件を書き、review/deep.md R2 を `fixed D2 仮`、R7 を `fixed D1 仮` に直した

## R2. D1 の手順は、普通の経路（作り直しを経ていないコンテナ）で `docker volume rm` が落ち、作り直しが毎回 BLOCKED_INFRA になる
- 成果物: openspec/changes/ch06-reset-verify-db/design.md / openspec/changes/ch06-reset-verify-db/tasks.md
- 根拠: design.md:37（`docker compose rm -s -f -v db` でコンテナを消し、控えた volume を `docker volume rm` で 1 つずつ消す）・design.md:71（4. で docker が失敗したら rc=1）・tasks.md:65-66（「控えた volume を `docker volume rm`（落ちたら `exit 1`）」）。一方、P11 の根拠の review/deep.md:64 は「作り直しを経ていないコンテナは `rm -s -f -v db` で消えた」と実測している。CH06 の作り直しで立つコンテナは毎回新しく作られる（作り直しを経ていない）ので、2 回目以降は `rm -v` が匿名 volume を消し、その後の名指しの `volume rm` は no such volume で非 0 → `exit 1`
- kind: technical
- 提案: どちらかに決めて design D1・D4 と tasks 4.1 に書く: (a) `rm -s -f db`（`-v` なし）で消し、volume は名指しだけで消す / (b) 名指しの `volume rm` は「もう無い」を成功として扱う。Scenario「作り直しの後、前の DB の volume は残らない」はどちらでも `volume ls` で見られる。本体（D8）の 5. と 6. は 2 回続けて作り直すので、(a)(b) が入っていないと 6. で落ちることを tasks 5.1 の終了条件で捕まえられる、と書き添える
- 処置: fixed D1 仮 —— (a) を採った。`rm -s -f db`（`-v` なし）で消し、volume は名指しだけで消す。spec の Scenario「作り直しの後、前の DB の volume は残らない」を 2 回続けて作り直す形にし、tasks 4.2 の検証に 2 回続けての作り直しを足した

## R3. subnet は「生きている木の port」から決まるが、消えた worktree の network は残る。台帳の番号を使い回した次の木は `Pool overlaps` で DB を立てられない
- 成果物: openspec/changes/ch06-reset-verify-db/design.md / openspec/changes/ch06-reset-verify-db/specs/verify-environment/spec.md
- 根拠: design.md:76（port が被らないので subnet も被らない。台帳を別に持たない）/ `tools/ports.sh:27`（消えた worktree の行は捨てて番号を空ける）・`tools/check-ports.sh:40-42`（「消えた worktree の番号を使い回す」を自己検査で固定）/ premises.md P10（`ashiato2*_default` 25 個のうち 15 個が消えた worktree のもの。ch01・ch02 は台帳の木）/ design.md:23（消えた worktree の network は片付けない）/ 使い捨ての network 2 つで実測: 同じ subnet の 2 つ目は `Pool overlaps with other one on this address space`。`-ch<NN>`・`-up-st<NN>`・`-rec-st<NN>` は名前が毎回違う（compose project も違う）ので、同じ番号の次の木は同じ subnet の**新しい** network を作ろうとして落ちる。spec の Scenario「どの 2 つの木も、DB の subnet が被らない」は生きている 7 つの木しか見ないので通る
- kind: technical
- 提案: design D5 で次のどれかを決め、Scenario に「消えた木の network が残っていても、番号を継いだ木の DB が立つ」を足す: (a) subnet の番号を使い回さない（単調に増える別の割り当て。D5 の「台帳を別に持たない」を改める）/ (b) `verify-env.sh` が `up` の前に、同じ subnet を持ち**コンテナの繋がっていない**他の project の network だけを消す。(b) は Non-Goals「片付けない」（Q3 の推奨）の一部を変えるので、そうするなら deep の Q3 の答えの範囲かを確かめる
- 処置: fixed D5 仮 —— (b) を狭く採った。自分の network が無いとき、同じ subnet でコンテナの繋がっていない network だけを消し、繋がっていれば何も消さずに非 0。Requirement「worktree ごとの DB の network は…」に Scenario 2 本、tasks 4.2・5.1 の 8. に足した。Q3 の推奨の「片付けない」の例外として design D5 と proposal に書き、本人の答えが例外を認めないときの反転条件を置いた

## R4. 確認バッチの作業ツリー（`-verify`）では subnet が入らないか、main の subnet を写して main と被る
- 成果物: openspec/changes/ch06-reset-verify-db/design.md / openspec/changes/ch06-reset-verify-db/proposal.md
- 根拠: subnet の鍵を `.env` に書くのは `tools/env-init.sh` だけで、それを呼ぶのは `tools/verify-env.sh`（design.md:34）。`-verify` には `verify-run` を呼ぶ係が無い（design.md:58・premises.md C1）。`tools/verify-prep.sh:23-31` は `-verify` に `.env` が無ければ **main の checkout の `.env` を写し**、env-init は main にも無いときだけ呼ぶ。`tools/stack.sh:27-28` は env-init を呼ばない。よって (1) main の `.env` に鍵が入っていれば（design.md:150 の Risks が起きると書いている）、`-verify` は main と同じ `ASHIATO_DB_SUBNET` で `up` し、main の network と重なって `Pool overlaps`（上の実測）、(2) 入っていなければ既定の空きのまま（P4 の BLOCKED と同じ）。spec の Requirement「worktree ごとの DB の network は、木ごとに被らない subnet で立つ」（spec.md:87-92）と design の Goals「アドレスの空きが尽きた機械でも、worktree の DB を立てられる」（design.md:20）が `-verify` で成り立たない。Risks にも Non-Goals にも書かれていない
- kind: technical
- 提案: `tools/verify-prep.sh`（Global Constraints の「変えないもの」に入っていない）で `.env` を写した後・作った後に `tools/env-init.sh` を呼ぶ（D7 の 4. が鍵を今の木の値に直す）と design に書き、Scenario「ほかの木の `.env` を写した木で env-init を呼ぶと、`compose config` の subnet がその木の値になる」を足す。R5 の優先の規則を直さないと、この直しも効かない
- 処置: fixed D7 仮 —— `tools/verify-prep.sh` が `.env` を写した・作った後に `env-init.sh` を呼ぶ（tasks 2.1）。subnet は環境変数からも `.env` からも読まず木の port から決めるので、写した `.env` でもその木の値になる（Scenario「別の木から写した .env でも…」）

## R5. `ASHIATO_DB_SUBNET` を `.env` に書きながら「渡された値を優先する」規則にしたため、`.env` を読んでから `ports.sh` を読む呼び出し口では `.env` の値が木の値に勝ち、古い値が直らない
- 成果物: openspec/changes/ch06-reset-verify-db/specs/verify-environment/spec.md / openspec/changes/ch06-reset-verify-db/design.md
- 根拠: spec.md:90（`ASHIATO_DB_SUBNET` が渡されていればそれを使う）/ `tools/stack.sh:27-28`・`tools/smoke.sh:7-8`・`tools/st25-seed-check.sh:11` は `set -a; . ./.env` の**後に** `. tools/ports.sh` / ハーネスは agent と `verify-run` に `tools/agent-env.sh` の export を渡し（`harness2/graph/harness_graph/effects.py:162`・`:290-296`）、`tools/agent-env.sh:47-50` は `.env` を `set -a` で読む → agent の起動時の `.env` の `ASHIATO_DB_SUBNET` が環境変数として渡る。すると `verify-env.sh` → `ports.sh` はその値を「渡された値」として使い、`env-init.sh` はそれを `.env` に書き戻す。`.env` が別の木から写された（R4・`tools/env-init.sh:7` の「他の worktree の `.env` は手で置かれていた」）ときは、Scenario「鍵が古い値なら今の木の値に直し、何度呼んでも同じである」（spec.md:117-119）がハーネスの中では成り立たない（自己検査は環境を落として呼べば通るので気付かない）。`tools/env-init.sh` は `ASHIATO_DB_PORT` を `.env` に書かない（URL の port は `ashiato_db_url` が差し替える。`tools/ports.sh:65-67`）ので同じことが起きない —— CH06 の鍵だけがこの対称を崩す。加えて、範囲の外で `ASHIATO_DB_SUBNET` を**空で export** する（tasks.md:30）と、`.env` に `COMPOSE_FILE` がある木では compose が `required variable … is missing a value` で落ちる（上の実測: 空の shell 変数は `.env` に勝つ）
- kind: technical
- 提案: (a) `ports.sh` は木の値を計算し、上書きは別の名前（例: 明示の上書き用の変数）でだけ受ける、か (b) `env-init.sh` は渡された `ASHIATO_DB_SUBNET` を見ずに木の値を書く、のどちらかを design D5・D7 に決め、Scenario「`ASHIATO_DB_SUBNET` に古い値が export された環境でも、env-init は `.env` を今の木の値にする」を足す。範囲の外は「空を export する」でなく「unset する（export しない）」にする
- 処置: fixed D6 仮 —— subnet の値を `.env` にも環境変数にも載せない形に替えた。`env-init.sh` が値を書いた `docker-compose.subnet.yml`（生成物・gitignore）を毎回作り、`.env` は木によらず同じ形の `COMPOSE_FILE` だけを持つ。`ports.sh` は export しない関数 `ashiato_db_subnet` だけを足す（範囲の外は何も出さない）。上書きの Scenario は外した

## R6. `--self-test` が呼び出し側の環境に左右される: CI では env-init が `.env` を作らず、Scenario「env-init の .env で…」が落ちる
- 成果物: openspec/changes/ch06-reset-verify-db/design.md / openspec/changes/ch06-reset-verify-db/tasks.md
- 根拠: CI の rust job は `tools/ci-db-env.sh:6-11` で `POSTGRES_PASSWORD`・`OWNER_DB_PASSWORD`・`APP_DB_PASSWORD` を `$GITHUB_ENV` に入れ、以後の段すべての環境に渡る / design D7 の 1.（design.md:100: `.env` が無く 3 つの合言葉が環境にあれば何もしない）/ design D8（design.md:113）と tasks 2.2（tasks.md:43）は「env-init が書いた `.env` だけで `docker compose config`」を見るが、env-init を呼ぶときに 3 つの合言葉を落とすことはどこにも書かれていない（`env -i` は `compose config` を呼ぶときだけ）。CI の `--self-test`（tasks 6.1 で rust job に足す）では一時の木に `.env` ができず、この Scenario は落ちる。手元でも R5 の `agent-env.sh` が `COMPOSE_FILE`・`ASHIATO_DB_SUBNET` を渡すので、一時の木の `ports.sh` と env-init は本物の木の値を使う
- kind: technical
- 提案: design D8 と tasks 2.2・3.2 に「一時の木の道具は、`POSTGRES_PASSWORD` `OWNER_DB_PASSWORD` `APP_DB_PASSWORD` `ASHIATO_DB_SUBNET` `ASHIATO_DB_PORT` `ASHIATO_HTTP_PORT` `COMPOSE_FILE` `CI` `HARNESS_VERIFY` を落として呼ぶ（場合ごとに要るものだけ足す）」と書く。tasks 2.2 の検証に、3 つの合言葉を export した環境で `--self-test` を走らせる 1 行を足すと CI と同じ条件を手元で見られる
- 処置: fixed D8 仮 —— 一時の木の道具は 9 つの変数を `env -u` で落として呼ぶと design D8・tasks 2.2 に書き、tasks 2.2 の検証に CI と同じ環境（3 つの合言葉・`ASHIATO_DB_SUBNET`・`CI=true`）での `--self-test` を足した

## R7. `docs/changes/CH06.md` に `## 完了の判定` が 2 つあり、機械が読むのは古い方（雛形の残り）
- 成果物: docs/changes/CH06.md
- 根拠: docs/changes/CH06.md:33-40（`hx change` の雛形の `## 依頼` `## なぜ` `## 完了の判定` と「上流の深掘りの後に…整える」の注記が残っている）と :74-81（この段で書いた完了の判定）/ `scripts/issue_body.py:45-53` の `section()` は最初の一致を返し、`:128` で `done = section(story, r"完了の判定")`（`SID.source` で `docs/changes/CH06.md` を読む）/ `scripts/stories_json.py:29` も最初の一致。古い方には subnet・切った接続・非 0・main の checkout を作り直さない、が無い / CH03.md・CH04.md は `## 価値` `## 依頼（逐語）` `## 壊してはいけないもの` `## 完了の判定` の 1 つずつ（`grep -n "^## " docs/changes/CH0[34].md`）
- kind: technical
- 提案: :18-40 の雛形の節（`## 依頼` `## なぜ` 最初の `## 完了の判定` と注記）を消し、逐語は `## 依頼（逐語）` だけに残す（CH03・CH04 と同じ形）
- 処置: fixed D11 —— 雛形の `## 依頼` `## なぜ` 最初の `## 完了の判定` と注記を消し、`## 依頼（逐語）` だけを残した

## R8. Scenario「ほかの compose project と compose の外のものには触らない」は、compose の外のものしか置いていない。依頼の完了の判定「ほかの worktree の DB には触らない」を確かめる Scenario が無い
- 成果物: openspec/changes/ch06-reset-verify-db/specs/verify-environment/spec.md
- 根拠: spec.md:34-36（WHEN は compose の外の `ch06-sentinel-` のコンテナと volume だけ）/ spec.md:13（Requirement 本文は「ほかの compose project のコンテナ・volume と、compose の外の…には触らない」）/ docs/changes/CH06.md:36・:70（完了の判定と壊してはいけないもの）/ design D8 の 5.（design.md:121）も compose の外だけ。compose の label（`com.docker.compose.service=db` など）で引く誤りはほかの worktree の DB を消すが、label を持たない sentinel は残るので緑になる
- kind: technical
- 提案: 本体の 5. に、別の compose project（`docker compose -p ch06-sentinel-<乱数>` で、service 名 `db` の小さなコンテナと匿名 volume）を置く段を足し、作り直しの後も同じ id で残ることを見る。Scenario を「ほかの compose project の db は…」と「compose の外のものは…」の 2 本に分けるか、WHEN に両方を書く
- 処置: fixed D8 仮 —— 本体の 5. に別の compose project（`ch06-sentinel-<乱数>`、service `db`、`network_mode: none`）を置き、Scenario の WHEN に両方を書いた

## R9. 「CI でない main の checkout では作り直さない」は `--plan` の出力しか確かめない。印つきの実行経路で中身が残ることを、どの検査も通らない
- 成果物: openspec/changes/ch06-reset-verify-db/specs/verify-environment/spec.md / openspec/changes/ch06-reset-verify-db/design.md
- 根拠: spec.md:40-43（Requirement 本文: DB の中身に触らない・作り直さなかったことと理由を 1 行出す）に対し、Scenario は spec.md:50-52 の `--plan` だけ / design D8 の本体は worktree でない木で rc=2（design.md:115）なので、main の checkout の印つきの実行は一度も走らない / premises.md C4（:37）と review/deep.md R2 の提案（:31）は「CI でない main の checkout で行が残る（作り直さない）ことも同じ台本で見る」と言っていた —— `--plan` と実行の経路が分かれる実装の誤りは、失う側（main の開発用 DB。premises.md P7）に倒れても緑になる
- kind: technical
- 提案: 実行の経路を見る Scenario を足す。例: CI の本体の中で `env -u CI HARNESS_VERIFY=1 tools/verify-env.sh` を呼ぶ（CI の checkout は worktree でない木なので main の checkout と同じ判定になる）と、行が残り、判定の 1 行が標準エラーに出る。あるいは `--self-test` の一時の main checkout で PATH の先頭に偽の `docker` を置き、`rm`・`volume rm` が呼ばれないことを見る
- 処置: fixed D8 仮 —— Scenario「CI でない main の checkout では、印つきで呼んでも行が残る」を足し、本体の 3. で worktree でない木（CI）のとき `env -u CI HARNESS_VERIFY=1` で実行の経路を通す。worktree では SKIP と出し、CI の証跡で通す

## R10. D6 の「override を書くのは録画の環境だけ」はこの機械で成り立たない。`COMPOSE_FILE` を書くと、手で置いた override が黙って読まれなくなる
- 成果物: openspec/changes/ch06-reset-verify-db/design.md
- 根拠: design.md:91（`COMPOSE_FILE` を置くと override を自動では読まない。override を書くのは録画の環境だけで、その `.env` には触らない）/ `/home/yosis/dev/ashiato2-integ/docker-compose.override.yml`（db の port を `55434` に替える）があり、その `.env` に `COMPOSE_PROJECT_NAME` の行は無い（`.env` は手で書かれ、`ASHIATO_DB_PORT` を持つ）/ `/home/yosis/dev/ashiato2-rec-st22/docker-compose.override.yml` もあり、その `.env` にも `COMPOSE_PROJECT_NAME` は無い（D7 の 2. の印が当たらない）。どちらも env-init が走れば D7 の 4. で `COMPOSE_FILE` が入り、override が外れる。D6 自身の反転条件（design.md:94「worktree で override のファイルが要るようになったら」）に当たっている
- kind: technical
- 分類の直し（spec の段。元は premise）: 前提「override を書くのは録画の環境だけ」は実物（`ashiato2-integ`・`ashiato2-rec-st22` の override）で外れが確かめられ、人間に聞く事実が残らない。直しは設計の漏れ
- 提案: D7 に「木に `docker-compose.override.yml` があれば、`COMPOSE_FILE` の 3 枚目に足す（か、その `.env` に触らない）」を足し、Requirement「env-init は…」に Scenario を 1 本足す
- 処置: fixed D6 仮 —— 木に `docker-compose.override.yml` があれば `COMPOSE_FILE` の 3 つ目に並べる（D7）。Scenario「木に override のファイルがあれば、COMPOSE_FILE に並べる」を足した

## R11. 観測できる振る舞いが design にだけ書かれ、spec に無い（archive で正典から落ちる）
- 成果物: openspec/changes/ch06-reset-verify-db/design.md / openspec/changes/ch06-reset-verify-db/specs/verify-environment/spec.md
- 根拠:
  - D3 の 1 行の書式 `verify-env: DB を作り直す（切った接続: <数>。<application_name>@<接続元>, …）`・同じ出所の `×<数>`・10 種の上限・`切った接続: 0（DB は止まっていた）`（design.md:63-64）。tasks 4.1 の検証（tasks.md:67）は design にしか無い文字列 `切った接続: ` を grep している
  - D1 の 2.「コンテナが無ければ 5. へ」（design.md:35）では切った接続の行が出ない。spec の「繋いでいるものが無ければ、数が 0 であることを 1 行で出す」（spec.md:63）と食い違う（新しい木で tasks 4.1 の検証を走らせると grep が落ちる）
  - D7 の「書き換えは一時ファイルと `mv`、権限（`umask 077` で作ったもの）を保つ」（design.md:105）。`.env` は合言葉を持つので、`mv` で 0644 になると誰でも読める。Scenario「既にある .env に…」（spec.md:113-115）は行しか見ない
  - D5 の「範囲の外の port では `ASHIATO_DB_SUBNET` を空にし、env-init は鍵を書かない」（design.md:77）
  - D2 の「印があって `keep` のときだけ判定の 1 行を標準エラーに出す」（design.md:57）—— spec は「1 行出す」だけで、出す先を言わない（R9 の Scenario を書くときに要る）
- kind: technical
- 提案: 少なくとも権限（Scenario「env-init が書き換えた `.env` の権限は書き換える前と同じ」）と「DB のコンテナが無いときも `切った接続: 0` の 1 行を出す」を spec に移す。行の書式は Requirement の本文に `切った接続: <数>` の形だけを書けば、tasks 4.1 の grep の根拠が正典に残る
- 処置: fixed specs/verify-environment/spec.md —— `切った接続: <数>` の形と、0 でも（DB が止まっていた・コンテナが無い）1 行出すことを Requirement に移し、Scenario「DB のコンテナが無くても…」を足した。権限の Scenario を足した。keep の 1 行は標準エラーと書いた。範囲の外の扱いは design D7 の 5.（観測は `COMPOSE_FILE` の行の有無）に残す

## R12. 本人に見せる Q3 の推奨「台帳の番号から割り当てる」と、design D5「DB の port から決める・台帳を別に持たない」が言葉の上で違う
- 成果物: openspec/changes/ch06-reset-verify-db/deep.md / openspec/changes/ch06-reset-verify-db/design.md
- 根拠: deep.md:18（推奨: worktree ごとの subnet を台帳の番号から割り当てる）/ design.md:75-76（`off = ASHIATO_DB_PORT − 55400`。`<main>-st<NN>`・main・`-verify` は台帳の番号を持たない）/ proposal.md:27（port から決める）
- kind: technical
- 提案: 本人が「推奨のまま」と答えたときに spec がその答えを言っていると読めるよう、D5 に「台帳の番号から」は port（台帳の木は 55600+n、ほかは固定の規則）を通して実現する、と 1 行書く。R3 の (a) を採るなら、Q3 の推奨の文がそのまま当たる形になる
- 処置: fixed D5 仮 —— 「台帳の番号から」は port を通して実現する（台帳の木は台帳の番号 n から port、ほかは固定の規則）と D5 に書いた

## R13. 1 つの Scenario に主張を 2 つ束ねているもの
- 成果物: openspec/changes/ch06-reset-verify-db/specs/verify-environment/spec.md
- 根拠: spec.md:54-57（THEN `reset` と AND「印が無ければ `keep:`」。後者は「印が無ければ作り直さず…」と別の主張で、`--plan` だけの別の場合）/ spec.md:25-28（前の volume が無い・新しい volume が 1 つで名前が違う）/ spec.md:66-69（1 行に数と名前・psql が非 0）/ spec.md:82-85（rc と名前・コンテナの id と行数）
- kind: technical
- 提案: 少なくとも spec.md:57 の AND は Scenario「印が無ければ、どの木でも `--plan` は `keep:` で始まる」に分ける。ほかは同じ 1 回の呼び出しの観測なので束ねたままでもよいが、片方だけ通る実装（例: 新しい volume が 2 つ）を落とせるかを tasks 5.1 の書き方で確かめる
- 処置: fixed specs/verify-environment/spec.md —— spec.md:57 の AND を Scenario「印が無ければ、どの木でも作り直さない判定になる」に分けた。ほかは同じ 1 回の呼び出しの観測として束ねたまま。volume は「ちょうど 1 つ」と書いた

## R14. tasks の検証が書いたままでは誤って落ちる・誤って通るもの
- 成果物: openspec/changes/ch06-reset-verify-db/tasks.md
- 根拠:
  - tasks.md:67 `! grep -nE "down -v|up .*-V|force-recreate" tools/verify-env.sh` は注記の行も見る。D11（design.md:142）は冒頭の注記を直すよう求め、D1（design.md:40）は「`down -v` は使わない」を理由つきで書いている —— 注記に理由を書くと検証が落ちる
  - tasks.md:43 の 2.2 の検証は CI と同じ条件（3 つの合言葉が環境にある）で走らないので、R6 を手元で捕まえない
- kind: technical
- 提案: grep を注記の外に絞る（`grep -vE '^\s*#' tools/verify-env.sh | grep -nE …`）。2.2 の検証に `POSTGRES_PASSWORD=a OWNER_DB_PASSWORD=b APP_DB_PASSWORD=c tools/check-verify-env-reset.sh --self-test` rc=0 を足す

---

## 観点ごとのまとめ

- 観点 1（deep の決定が写っているか）: Q1（volume ごと毎回作り直す）→ spec.md:7-36、Q2（構わず切り、数と出所を 1 行）→ spec.md:59-73、Q3（木ごとの subnet・消えた木の残りは片付けない）→ spec.md:87-127 と design Non-Goals（design.md:23）。答えが言っていない数値は無い。言葉のずれは R12。当初案を覆したもの・要件へ戻すものは deep.md に無い
- 観点 2（検証可能か）: R8・R9・R11（Requirement の本文だけが言っていること）・R13
- 観点 3（置き場）: R11。specs の道具の名前（`tools/verify-env.sh`・`COMPOSE_FILE`・`docker-compose.subnet.yml`）は、この capability の主題が道具の約束そのもので、CH03・CH04 と同じ扱いなので挙げない。`proposal.md` の Capabilities（`verify-environment`）と `specs/` のディレクトリは一致。`docs/changes/CH06.md` の `capability: []` は design D10（仮）に理由がある（CH04 の D12 と同じ）
- 観点 4（tasks）: 全タスクにコマンドと rc の終了条件がある。依存順は Task 1（subnet。DB が立たない間は検証が BLOCKED）→ 2 → 3（判定）→ 4（作り直し）→ 5（本体）で、鍵（subnet）を変えるタスクが先にある。人間の確認待ちは無く、CH06.md:81 がそう書いている。誤りは R2（4.1 の終了条件が普通の経路で落ちる）・R6・R14
- 観点 5（change の文書）: 「完了の判定」はあるが 2 つある（R7）。「壊してはいけないもの」のうち、ほかの worktree の DB（R8）と main の checkout の DB（R9）が Scenario で固定されていない。印が無いときの振る舞い（spec.md:46-48）・既にある `.env` の合言葉（spec.md:113-115）・CI の合言葉（spec.md:121-123）は Scenario がある。依頼の逐語を越える Q3（subnet）は deep の Q3 に決定がある
- 観点 6: 該当なし（`spec --draft` で、`--answers` ではない）

## 観点 7: 走っている change との前提関係

観点 7: 該当なし（spec_overlap.py candidates が 0 件）

参考に確かめた範囲: CH04 の delta（`origin/main:openspec/changes/ch04-test-db-growth/specs/test-database/spec.md`）は回ごとの database と片付けで、CH06 の Scenario はそのどれも当てにしていない（作り直しの後も `ashiato` database は `POSTGRES_DB` で立つので、CH04 の「ashiato database が消えていれば落ちる」とも食い違わない）。`ci.yml` は CH04 の実装（`ashiato2-ch04` の `git diff origin/main...HEAD`）が `cargo test` の直後に 4 行、CH06 は rust job の末尾（`check-ports.sh --self-test` の後）で、同じ行を取らない
- 処置: fixed tasks.md —— 4.2 の grep を注記の外に絞り、2.2 の検証に CI と同じ環境の 1 行を足した

