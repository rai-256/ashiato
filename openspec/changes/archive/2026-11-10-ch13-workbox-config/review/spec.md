# CH13 spec-review（spec --draft。第 1 回の先読み）

対象: `openspec/changes/ch13-workbox-config/`（proposal・design・tasks・enable-before・.openspec.yaml）と `docs/changes/CH13.md`。
deep.md の答えは全部 `（答え待ち・推奨: …）` なので、推奨を答えとして突き合わせた。

## 機械の検査（先に写す）

| コマンド | 結果 |
|---|---|
| `openspec validate ch13-workbox-config --strict` | rc=0（`skip_specs is set … zero deltas accepted`） |
| `python3 scripts/check_chain.py .` | rc=0（chain: OK） |
| `python3 scripts/check_scenarios.py . ch13-workbox-config` | rc=0（scenarios: OK。specs/ が無く、正典の Scenario だけを見る） |
| `python3 scripts/review_triage.py . ch13-workbox-config` | **rc=1（triage: FAIL 10 件）** —— review/deep.md の R1〜R8・R10 の処置の形（→ R4） |
| `python3 scripts/spec_overlap.py candidates . ch13-workbox-config` | `[]`（0 件） |
| `python3 scripts/spec_overlap.py check . ch13-workbox-config` | rc=0（`{"ok": true, "candidates": 0}`） |
| `python3 scripts/evidence.py lint openspec/changes/ch13-workbox-config` | rc=0（ただし 1.2 の抽出が壊れている。→ R1） |
| `python3 scripts/provisional.py check openspec/changes/ch13-workbox-config` | rc=0（台帳は空） |

追加で確かめたこと:
- Task 1.1・1.2 を使い捨てのフォルダ（`docs/changes/CH13.md` の写しと、依頼の塊を機械で取り出して書いた `tools/workbox.json`）で走らせた: 1.1 rc=0、1.2 rc=0、`"6g"` を `"7g"` にすると 1.2 は AssertionError で rc=1。**手で貼れば正しく働く**。repo の `tools/workbox.json` は置いていない（最後に無いことを `ls` で確かめた）
- Task 2.1・2.2・2.4 をこの worktree で書いたまま走らせた: どれも rc=0。Task 2.3 の 6 本も rc=0（63 秒）
- `openspec archive ch13-workbox-config --yes` を `openspec/` の使い捨ての写しで走らせた: rc=0、`2026-11-08-ch13-workbox-config` に移った（proposal.md:36 の主張どおり）
- `spec_merge.py apply` は `specs/*/spec.md` を glob するだけで、無ければ何もしない（`.harness2/scripts/spec_merge.py:87-91,137`）。`archive.sh` が skip_specs で詰まるところは見つからなかった
- `go_brief.py build` は go.json がまだ無いので `[FAIL] … go.json が無い`（この段では正常）。Requirement 0 本で組めるか（design.md:75 が reading の段に回した点）は、change を使い捨ての木に写し、`go_brief.py --example light` の go.json の refs を `requirements` から `decisions`（Q1〜Q4・D2〜D5）に差し替え、ch10 との組み合わせを `impact.coverage.not_examined` に 1 行書いて `go_brief.py check` を走らせた: `go_brief: OK` rc=0。refs は決定だけでも通る（`basis: spec` の場面・`uses` も決定の refs で足りる）。go.json を書く段は、走っている ch10 との組み合わせ（`tools/android-emulator.sh`）を dependencies か coverage に書く必要がある（書かないと FAIL）

## R1. Task 1.2 の検証コマンドは、ハーネスが取り出すと最初の ``` で切れ、構文エラーで必ず落ちる
- 成果物: openspec/changes/ch13-workbox-config/tasks.md
- 根拠: tasks.md:22（コマンドの中に `r"```json\n(.*?)\n```"` がある）/ `.harness2/scripts/evidence.py:142`（`CODE_SPAN = re.compile(r"`((?:\\`|[^`])+)`")` —— code span の中の backtick は `\`` しか許さない）/ `evidence.declared()` で 1.2 を取り出すと `python3 -c 'import json,re;…b=re.findall(r"` で切れる。その文字列を `bash -c` に渡すと `rc=2`（`unexpected EOF while looking for matching ''`）/ `evidence.py lint` はこれを拾わない（rc=0）
- kind: technical
- 提案: コマンドの中に backtick の字を置かない。`chr(96)*3` で ``` を組む（例 `f=chr(96)*3; re.findall(f+"json\n(.*?)\n"+f, t, re.S)`）か、比べる台本を change の中のファイル（`openspec/changes/ch13-workbox-config/check-workbox.py`。ch09 の `check-test-names.sh` と同じ置き方）にして検証はそれを呼ぶ。直した後に `evidence.declared` で取り出した文字列を走らせて確かめる
- 処置: fixed 1.2 — ``` を `\x60{3}` の正規表現にした。`evidence.declared` で取り出した文字列を走らせて、依頼の塊を書いた木で rc=0 を確かめた

## R2. Q1 の推奨「issue にも残す」が D2 で「issue は立てない」に変わっている。推奨のまま答えると、答えが写らないまま流れる
- 成果物: openspec/changes/ch13-workbox-config/design.md
- 根拠: deep.md Q1「答え: （答え待ち・推奨: PR 本文に挙げ、有効にする前の宿題として issue にも残す）」/ design.md:42「**issue は立てない**（推奨の「issue にも残す」をこの形にした）」/ design.md:45 の反転条件は「本人が issue を求める（Q1 の補足）」だけで、推奨の選択肢そのものが issue を求めていることを反転の引き金にしていない / docs/changes/CH13.md の「価値」・tasks.md:11 も「issue は立てない」/ 第 1 回の `spec --answers` は推奨と違った答えだけを名指すので、本人が推奨を選ぶと Q1 は見直されず、選んだ「issue にも残す」は実現しない。なお D2 の理由「`publish.sh` の issue は CH13 の merge で閉じる」は自動ではない: `publish.sh:64` は PR 本文に `refs #N` を足すだけで、閉じるのは PR 本文が `closes #N` を書いたとき（`.harness2/docs/flow-gates.md:212`「`closes #N` は残タスク 0 のときだけ。それ以外は `refs #N`。issue が残タスクの入口」・`.harness2/scripts/issue_body.py:142`「残タスクが 0 になるまで閉じない」）。宿題を残タスクとして書けば、既存の CH13 の issue が置き場になりうる
- kind: conflict
- 提案: どちらかに揃える。(a) 推奨どおり issue に残す形にする —— agent に gh を持たせずに済む置き場（publish.sh が作る CH13 の issue を、下流の PR 本文で `closes` にせず `refs` のまま残し、宿題を残タスクとして載せる。または archive の後に人間が立てる 1 行を PR 本文に置く）を D2 に書く。(b) issue を立てない形を採るなら、推奨の文と違うことを GO 資料の highlights に出して本人に見せる（推奨のまま答えても見落とさない形）。どちらでも D2 の「merge で閉じる」は「下流の PR 本文が `closes` を書けば閉じる」に直す
- 処置: escalated — Q1 で本人に問い、Q1 (3) の答え（issue にも残す）を D2 に写した（3 回目の R12 で直した。前は fixed D2 仮）。推奨どおり issue にも残す形にした: 下流の Task 3.2 が同じ題が無いときだけ 1 つ立てる。CH13 の issue を置き場にしない理由を「`closes` を書けば閉じる」に直した。CH13.md の価値・完了の判定と proposal も揃えた

## R3. PR 本文に載せる宿題が「1〜6 の見出し」で、Q3 で回した pgrep の確かめ（§0）が PR 本文から落ちる。PR 本文を確かめる Task も無いのに CH13.md は「Task 3.1」を指す
- 成果物: openspec/changes/ch13-workbox-config/tasks.md
- 根拠: deep.md Q3 推奨「tasks には置かず、**PR 本文と** Q1 の宿題に回す」/ enable-before.md:8 `## 0. 先に確かめること`（pgrep と鍵の名前の突き合わせはここ）/ tasks.md:14「下流の PR 本文の先頭に、`enable-before.md` への案内と、その **1〜6** の見出しを載せる」/ docs/changes/CH13.md:115「PR 本文に同じ一覧への案内がある（**Task 3.1**）」—— tasks.md に Task 3 は無い（Task 1・2 だけ）。PR 本文への記載は Global Constraints にあるだけで、検証コマンドが無い / design.md:54 は Q3 の反転のときの pgrep の Task を「Task 3」と呼び、番号がぶつかる
- kind: technical
- 提案: Global Constraints を「0〜6 の見出し」に直す。PR 本文の記載を Task 3.1 として置き、検証を付ける（例 `bash -c 'f=docs/briefs/CH13-pr.md; grep -q "enable-before.md" "$f" && grep -q "pgrep -fc /" "$f"'` rc=0。`publish.sh:46` は両方の段で `docs/briefs/$ST-pr.md` を PR 本文にする）。design D3 の反転の Task は別の番号（Task 4）にする
- 処置: fixed 3.1 — Global Constraints を 0〜6 の見出しと pgrep に直し、PR 本文を確かめる Task 3.1 と issue の Task 3.2 を足した。D3 の反転の Task は Task 4 にした

## R4. review/deep.md の処置が review_triage の形に合わず、上流の merge_gate が落ちる（R8 の処置は proposal と食い違ったまま）
- 成果物: openspec/changes/ch13-workbox-config/review/deep.md
- 根拠: `python3 scripts/review_triage.py . ch13-workbox-config` → FAIL 10 件（R1〜R5・R7・R8 は `fixed: Q<n> …` で、指す先が D<n> / <x.y> / ファイル名でない。R1 kind=conflict と R6 kind=daily は `fixed D<n> 仮` でしか閉じられない。R10 は rejected なのに根拠が無い）/ `.harness2/scripts/merge_gate.sh:197`（`review_triage.py` が rc≠0 なら note）・`archive.sh` の通過条件も同じ / review/deep.md:83 R8 の処置「skip_specs はこのハーネスに無い」と proposal.md:36-37「`skip_specs: true` で差分 0 を受け付け… R8 の処置は見落としていた」が食い違う / R10 は rejected だが enable-before.md:49 が宿題として載せている
- kind: technical
- 提案: 処置を design の D に向け直す: R1 → `fixed D3 仮`、R6 → `fixed D2 仮`（一覧の置き場）か premises.md / enable-before.md を指したうえで D2 仮、R2〜R5・R7・R9 → `fixed enable-before.md`（または D2）、R8 → `fixed D4 仮`（skip_specs を採った旨に書き直す）、R10 → enable-before.md に載せたので `fixed enable-before.md`（rejected をやめる）。直した後 `review_triage.py` が rc=0 であることを確かめる
- 処置: fixed proposal.md — review/deep.md の処置を D2〜D4 仮・enable-before.md・premises.md に向け直し、R8 を skip_specs を採った旨に、R10 を宿題に載せた旨に直した

## R5. Task 2.2 は commit 済みの差分しか見ず、作業ツリーの変更・未追跡のファイルを見逃す。git diff が落ちても通る
- 成果物: openspec/changes/ch13-workbox-config/tasks.md
- 根拠: tasks.md:29（`git diff --name-only "$base" HEAD -- . | grep -vE …; test $? -eq 1`）/ 証跡は「いまのコード（作業ツリー）」に対して取る（`.harness2/scripts/evidence.py:46`・`:112`）ので、commit 前に走らせると、作業ツリーで書き換えた `tools/*.sh` も未追跡の新しいファイルも差分に出ずに PASS する / パイプの終了コードは grep のもので、`git diff` が落ちて入力が空でも grep は rc=1 を返し `test $? -eq 1` が通る
- kind: technical
- 提案: `{ git diff --name-only "$base" -- . && git ls-files -o --exclude-standard; }` を `set -o pipefail` 付きで grep に渡し、`git diff` の失敗で落ちる形にする（作業ツリーと未追跡を両方見る）
- 処置: fixed 2.2 — 作業ツリーの差分と未追跡を pipefail で見る形にした。未追跡の tools/zz-test.sh を置くと rc=1 になることを確かめた

## 観点 1: deep の決定が正典に写っているか
- Q1 → R2（推奨と違う形で書かれている）
- Q2 → design D5（仮）・tasks.md:11・enable-before.md:48。置かない、で一致
- Q3 → design D3（仮）・tasks.md:13・CH13.md「完了の判定」4 つ目・enable-before.md:12-13。外すことは 4 か所で一致（PR 本文の側は R3）
- Q4 → design D4（仮）・`.openspec.yaml` の `skip_specs: true`・tasks.md:10 と Task 2.2・CH13.md「壊してはいけないもの」2。一致
- 数値: 決定に数値は無い（依頼の JSON の数値は Task 1.2 が依頼の塊と等しいことで固定する。ただし R1）
- 要件へ戻すもの: 無い（`satisfies: []`）

## 観点 2: Scenario が検証可能か
該当なし（確かめた範囲: `specs/` が無い。`skip_specs: true` で Requirement・Scenario を持たない。代わりの確かめは tasks の検証コマンドで、観点 4 で見た）

## 観点 3: 置き場の誤り
該当なし（確かめた範囲: design D1〜D5 はどれも ashiato2 の観測できる振る舞いを述べていない —— D1 は比べ方、D2〜D5 は置き場と範囲。proposal の Capabilities「無い」と `specs/` が無いことが一致。CH13.md の front matter `capability: []`。INDEX.md の表に change は載らない）。
skip_specs を採ったこと自体（依頼 (a)）: `HARNESS_WORKBOX` が off の間 ashiato2 の側で変わる振る舞いは無く、箱の振る舞いの正本は harness2 の未 merge の `scripts/workbox.py` で ashiato2 の試験に印を置けない。妥当。下流の関門は check_scenarios（rc=0）・spec_merge（no-op）・`openspec archive --yes`（rc=0）・spec_overlap（rc=0）で詰まらない。詰まるのは review_triage（R4）だけ。go_brief も Requirement 0 本・決定の refs だけで `check` が rc=0（機械の検査の節）

## 観点 4: tasks が検証を持つか
R1・R3・R5。そのほかの項目（1.1・2.1・2.3・2.4）はコマンドと終了条件があり、書いたまま走って rc=0。依存順（1 → 2）に問題は無い。人間の確認待ちは無く、CH13.md の「人間の確認待ちは無い」と一致

## 観点 5: Story（change）が依頼と一致しているか
- `docs/changes/CH13.md` の「完了の判定」は書かれている。「壊してはいけないもの」1〜3 はそれぞれ Task 2.1・2.2・2.3 が確かめる
- 依頼の完了の判定から pgrep を外したこと（依頼 (c)）: CH13.md の完了の判定 4 つ目・design D3・tasks.md:13・enable-before.md §0・proposal.md:18-19 は同じことを言っていて食い違いは無い。依頼の範囲を狭める変更だが deep の Q3 に決定（推奨）がある。食い違いは PR 本文の側（R3）と、CH13.md が存在しない Task 3.1 を指すこと（R3）だけ
- 依頼「足りないものに気付いたら PR 本文に挙げる」を越えて `enable-before.md` を置くことは Q1 に依る（R2 の issue の扱いを除いて一致）

## 観点 6: 答えが写ったか
該当なし（`spec --draft` の段。`--answers` ではない）

## 観点 7: 走っている change との前提関係
観点 7: 該当なし（spec_overlap.py candidates が 0 件）。依頼どおり、走っている 3 本を個別にも見た（CH13 は Requirement を持たないので、名前の欄は「重なる Requirement 無し」）:
- st53-license-bundle / 重なる Requirement 無し: 独立（CH13 は `tools/workbox.json` 1 つを足すだけで、st53 の振る舞いを当てにしない。st53 が変える `tools/check-spdx.sh` は表示の無いファイルを見ない（st53 proposal.md:21）ので JSON は Task 2.3 で落ちない。actionlint の docker への退きは enable-before の宿題で、どちらの順でも意味は変わらない）
- st56-collector-connection-setup / 重なる Requirement 無し: 独立（触るファイルが重ならない。画面の e2e が箱で落ちるのは enable-before の宿題で、st56 の合否にも CH13 の合否にも効かない）
- ch10-emulator-start-timeout / 重なる Requirement 無し: 独立（ch10 は `tools/docker-tidy.sh`（`prep_files` の 1 つ）と `tools/check-shared-appends.sh`（Task 2.3 の 1 本）を変えるが、CH13 はその中身を当てにせず、名前が残る限りどちらの順でも同じ。`tools/test-android-emulator.sh` は `host_only` の `tools/android-emulator\.sh` に当たらない）

## spec --answers の後（2026-11-08）

対象: `spec --answers Q1,Q2,Q3,Q4` で書き直した proposal・design・tasks・enable-before・match-cases.json・provisional.jsonl と `docs/changes/CH13.md`・`docs/briefs/CH13-pr.md`。推奨で書いた版（HEAD `a0a0644e5`）からの差分を読んだ。観点 6 は `review/answers.md`。

### 機械の検査（先に写す）

| コマンド | 結果 |
|---|---|
| `openspec validate ch13-workbox-config --strict` | rc=0（`skip_specs is set … zero deltas accepted`） |
| `python3 scripts/check_chain.py .` | rc=0（chain: OK） |
| `python3 scripts/check_scenarios.py . ch13-workbox-config` | rc=0（scenarios: OK） |
| `python3 scripts/review_triage.py . ch13-workbox-config` | rc=0（指摘 15 件 / 仮決め 3 件。triage: OK） |
| `python3 scripts/spec_overlap.py candidates . ch13-workbox-config` | `[]`（0 件）。`check` も rc=0 |
| `python3 scripts/evidence.py lint openspec/changes/ch13-workbox-config` | rc=0 |
| `python3 scripts/provisional.py check openspec/changes/ch13-workbox-config` | rc=1（D6・D7・D8 に `provisional-review.md` の確かめが無い）—— 別の席（グラフの provisional_review）が書く前の状態で、この段では正常 |

追加で確かめたこと:
- **(a) D1 の正規表現**: `.harness2/scripts/workbox.py`（main `1df37e05b`）の `host_only` → `needs_docker` を import し、`match-cases.json` の 31 行に当てた。D1 は 31 行とも `want` どおり、依頼の版は 18 行が外れる（e2e・`--workspace`・`server_startup`・`-p ashiato-server`・actionlint・中で docker を呼ぶ 3 本・`docker` 直が box、grep・`bash -n` の 4 行が docker、host_only の 4 行が box）。
  答えの (1) の名指し（錨・e2e・`--workspace`・`--test server_startup`・actionlint・load-repeat / st25-seed-check / check-st23-derived-erasure・host_only の 4 本・prep_files の 2 本）は全部 D1 の JSON にある。
  既存の `openspec/**/tasks.md` を `evidence.items` → `declared` → `unwrap_quiet` で 1,786 本集めて当てた: 依頼の版 ホスト 18・docker 209・箱 1,559、D1 ホスト 35・docker 317・箱 1,434（文書の 1,781 本との 5 本の差は CH13 自身の tasks.md が増えた分。5.2 は数の違いを許している）。
  D1 で箱に残り、docker の台本・e2e・`--workspace` などの名前を含む 61 本を読んだ: grep・`bash -n`・引数の位置・写した先の台本・`cargo clippy --workspace`・`server_startup_*` の絞り込み（st51 の試験に当たらない）で、裸の `cargo test`（→ R9）を除いて docker を要るものは無い
- 錨の頭に `!` `if` / `bash -lc` が無く、`bash -c '! tools/check-exposure.sh'`・`bash -c 'if tools/smoke.sh; then :; fi'`・`bash -lc 'tools/smoke.sh'`・`npm --prefix web run test:e2e` は D1 で box になる。既存の 1,786 本に実例は無いので指摘にしない（D6（仮）の反転条件と D7（仮）の issue の範囲。Q2 のとおり後の Story には何も置かない）
- **(b) 検証コマンド**: `evidence.declared` で取り出した字面をそのまま走らせた。D1 の塊を一時に `tools/workbox.json` に置いて 1.1・1.2・2.1・2.4・3.1・4.2 が rc=0、`"6g"` を `"7g"` にすると 1.2 は `AssertionError: tools/workbox.json が D1 と違う` で rc=1。
  4.1 は `pgrep: 1`、rc=0（イメージは作ってあった）。置いた `tools/workbox.json` は消し、`git status` で消えたことを確かめた。
  5.1 は使い捨ての木（`/tmp` の下。match-cases.json・CH13.md の写しと、5.1 の本文どおりに書いた台本）で rc=0。引用（`\"$d/match-cases.json\"`）は書いたまま通る。ただし `--request` の否定が何の失敗でも通る（→ R6）。5.2 は grep だけで、書いたまま走る形
- **(c) Q3 の補足**: 「harness2 と同じ判定」は D8（仮）・Task 5.1 が `workbox.host_only` / `needs_docker` を import して写し直さない形で満たす（main の `workbox.py:122` が `re.search(p, cmd)`、`evidence.py` は `unwrap_quiet` の後の字面に当てる。5.2 も同じ取り出し）。
  「既存の検証コマンドを全部分類した一覧を enable-before.md に添える」は、一覧そのものは下流の 5.2 が `command-classes.md` に書き出し、enable-before.md の「分類の一覧」の節が案内と上流の件数を持つ形（置き方は D8（仮））。issue の本文に入る時期は R8
- **(d) Q1 (3) の補足**: enable-before.md の宿題（チェックボックス）は測るもの 3 つだけになり、直した項目は「この change で済ませたこと（一覧から消したもの）」へ移った。harness2 の issue の節に 3 つの題があり、番号は下流の 3.2 が書く。pgrep（Q1 (3) が宿題に挙げたもの）は Q3 の補足（merge 済み）で tasks 4.1 に移り、「済ませたこと」に上流の結果つきで残る —— Q3 の補足が後の前提なので矛盾と読まない
- design.md の Risks から消えた 2 文（「上流の検査が仕様 0 本の change を想定していない → go_brief は reading の段で確かめる」と、gh の項の「上流で避ける手は無い」）は、決定・task・検証のどれも変えない字句の整理（go_brief は第 1 回の先読みで `check` が rc=0、go.json も HEAD より前にある）として確かめた

## R6. Task 5.1 の `! python3 … --request` は、台本がどんな理由で落ちても通る
- 成果物: openspec/changes/ch13-workbox-config/tasks.md
- 根拠: tasks.md:65 の検証 `… && ! python3 "$d/check-workbox-match.py" --request && …` / 使い捨ての木で、台本の `--request` が読む先を `docs/changes/CH13-typo.md` に変えると `FileNotFoundError` で落ち、5.1 の検証は rc=0。Python の捕まえない例外は exit 1 なので、「不一致で rc=1」と区別できない（`test $? -eq 1` に直しても塞がらない）
- kind: technical
- 提案: 不一致の終了コードを 1 以外の固有の値（例 3）にし、検証を `python3 "$d/check-workbox-match.py" --request; test $? -eq 3` にする（本文の「rc=1」も合わせる）。または不一致の件数を 1 行で出させ、依頼の版で `-ge 18` を確かめる
- 処置: fixed 4.1 — 台本は表との不一致だけを rc=3 にし、検証を `{ …--request; test $? -eq 3; }` にした（Task の番号は R8 の並べ替えで 5.1 → 4.1）

## R7. Task 2.4 の grep は、測るもの 3 つのうち 2 つを消しても通る
- 成果物: openspec/changes/ch13-workbox-config/tasks.md
- 根拠: tasks.md:37 の検証は `6g` `4g` `5 本` を本文のどこかに grep するだけ / enable-before.md の「1. 測っていない」から「画面の e2e 1 本が」と「agent の箱の中で agent 自身が」の 2 行を消した写しで rc=0（`6g` と `4g` は 2 つ目の項目の「agent 4g × 5 本 + 検証 6g × 2」、`5 本` は節の見出しにも出る）。本文の「下流は一覧の測るものを消さない」を検証が担保しない
- kind: technical
- 提案: 項目ごとの固有の句を個別に grep する（例 `grep -q "e2e 1 本が検証の箱の 6g" && grep -q "5 本のときの空きメモリ" && grep -q "cargo のビルドが 4g"`）か、「## 1. 測っていない」の節の `^- \[ \]` が 3 本あることを数える
- 処置: fixed 2.4 — 測るもの 3 つを項目ごとの固有の句で grep する形にした

## R8. Task 3.3 が enable-before.md を本文に issue を立てた後で、4.1・5.2 が本文を書き換える（依存順）
- 成果物: openspec/changes/ch13-workbox-config/tasks.md
- 根拠: tasks.md:48-51（3.3 は `--body-file openspec/changes/ch13-workbox-config/enable-before.md` で立てる）/ tasks.md:68（5.2 は「`enable-before.md` の「分類の一覧」の節に件数を写す」）/ design.md:113（D2「一覧には結果だけを残す」）/ tasks.md:41（3.1 は「下流は Task 4.1 の出力の値を書き足す」が、検証は `pgrep -fc /` の字面の grep だけで、上流が書いた行で既に通る）。グラフは Task を順に閉じるので、issue の本文は上流の件数・上流の pgrep の値で凍り、`command-classes.md` も書き出される前の案内になる
- kind: technical
- 提案: Task 3 を Task 5 の後ろへ移す（番号を付け直す）か、3.3 を「5.2 の後に立てる」と書いて順を入れ替える。3.1 の検証は下流の値を見る形（例 `grep -qE "pgrep: [0-9]+"`。4.1 の検証が出す行）にする
- 処置: fixed 5.1 — issue と PR 本文の Task を最後（Task 5）へ移し（旧 Task 4・5 → 3・4）、5.1 の検証に `pgrep: [0-9]+` を足した

## R9. 裸の `cargo test`（と、絞り込みなしの `-p ashiato-server` の後に `2>&1` などが続く形）は D1 の docker の欄に当たらない
- 成果物: openspec/changes/ch13-workbox-config/design.md
- 根拠: design.md:78（`cargo\s+test\b([^;&|'"]*\s--(workspace|test\s+server_startup)\b|\s+-p\s+ashiato-server\s*($|['";&|)]))`）/ `Cargo.toml` は virtual manifest で `default-members` が無く、根で裸の `cargo test` を走らせると全 crate の試験（`server_startup.rs:1901` の `st51_bin_migrate_attempt_survives_restore` を含む）が走る = `--workspace` と同じ。design.md:147 の D6（仮）の根拠がそのまま当てはまる / 実在: `openspec/changes/archive/2026-09-14-st03-idempotent-ingest/tasks.md` の 3.1・12.2 の `cargo test` は、D1 で box（5.2 の `command-classes.md` に誤って「docker を渡さない」で載る）/ `bash -c 'cargo test -p ashiato-server 2>&1 | tee x'` も D1 で box（実例は 0 本）。design.md:150 の「残るもの」はこの 2 つを挙げていない
- kind: technical
- 提案: 4 本目の正規表現に、裸の `cargo test`（例 `cargo\s+test\s*($|['";&|)]|\d?>)`。後ろに絞り込みの名前もフラグも無いもの）と、`-p ashiato-server` の後が `\d?>` で始まる形を足し、`match-cases.json` に `cargo test` と `bash -c 'cargo test -p ashiato-server 2>&1 | tee "$TMPDIR/x"'` を `docker` で 1 行ずつ足す。直さないなら D6（仮）の「残るもの」にこの 2 つを書く
- 処置: fixed D6 — 第 2 回の合意の違和感 (1) で docker の欄ごと消えた（台帳で resolve。3 回目の R12 で処置の形を直した）。当時は裸の `cargo test` と `-p ashiato-server` の後に `2>&1` などが続く形を docker の欄の cargo の型に足し、match-cases.json に 3 行足した（34 行）。D6（仮）に書いた

## 観点 1〜5・7（spec --answers の後）
- 観点 1: deep の Q1〜Q4 の答えと補足は proposal・design（D1〜D8）・tasks・CH13.md の完了の判定と壊してはいけないものに写っている（Q ごとは `review/answers.md`）。数値（10 未満・6g・4g・5 本・31 行・1,700 本）は tasks の検証にある。要件へ戻すものは無い（`satisfies: []`）
- 観点 2: 該当なし（確かめた範囲: `specs/` が無い。`skip_specs: true`）
- 観点 3: 該当なし（確かめた範囲: D1〜D8 は置き場・比べ方・範囲で、ashiato2 の観測できる振る舞いを述べていない。proposal の Capabilities「無い」と `specs/` が無いこと、CH13.md の `capability: []` が一致）
- 観点 4: R6・R7・R8。ほかの項目（1.1・1.2・2.1〜2.3・3.2・4.1・4.2・5.2）はコマンドと終了条件があり、走らせたものは書いたまま rc=0（3.2・3.3 は GitHub に出るので走らせていない）。人間の確認待ちは無く、CH13.md の「人間の確認待ちは無い」と一致
- 観点 5: `docs/changes/CH13.md` の完了の判定は Q3・Q4 の答えで書き直されており、tasks の 1.1〜5.2 と 1 対 1 で対応する。壊してはいけないもの 1〜4 は Task 2.1・2.2・2.3・1.2 / 5.1 が確かめる。依頼の範囲を越える足し算（D6〜D8（仮）の細部）は Q1・Q3 の答えに従うものとして deep に決定がある
- 観点 7: 該当なし（spec_overlap.py candidates が 0 件）。proposal の Impact の st53・st56・ch10 の読みは、D1 での当たり方（actionlint・e2e が docker、`tools/test-android-emulator.sh` が box）を match-cases.json と走査で確かめ、どれも独立のまま

## R6〜R9 の処置の後（2026-11-08。2 回目）

対象: commit `9014a3b3f`（R6〜R9 の処置を含む）と作業ツリー（tasks.md の 5.2・5.3 に「同じ題が既にあれば、その番号を書く」）。`git diff a0a0644e5` で推奨の版からの差分を読み直した。

### 機械の検査
| コマンド | 結果 |
|---|---|
| `openspec validate ch13-workbox-config --strict` | rc=0 |
| `python3 scripts/check_chain.py .` / `check_scenarios.py . ch13-workbox-config` | rc=0 / rc=0 |
| `python3 scripts/review_triage.py . ch13-workbox-config` | triage: OK（R6〜R9 の処置が付いた） |
| `python3 scripts/spec_overlap.py candidates . ch13-workbox-config` | `[]`（0 件） |
| `python3 scripts/evidence.py lint openspec/changes/ch13-workbox-config` | rc=0 |
| `python3 scripts/provisional.py check openspec/changes/ch13-workbox-config` | rc=1（D6〜D8 の provisional-review.md が無い。グラフの席が書く前の状態） |

追加で確かめたこと:
- 処置の確かめ: `match-cases.json` 34 行は D1 で全部一致、依頼の版で 20 行が外れる。裸の `cargo test`・`cargo test -p ashiato-server 2>&1 | tee x`・`bash -c 'cargo test 2>&1 | tail'` は D1 で docker（R9 は閉じた）。`cargo test foo_bar`・`cargo test --lib`・`cargo test -- --nocapture` は box のまま
- 4.1 の検証（`{ … --request; test $? -eq 3; }`）を使い捨ての木で、不一致で 3 を返す台本では rc=0、`--request` が `FileNotFoundError` で落ちる台本では rc=1（R6 は閉じた）
- 2.4 は今の enable-before.md で rc=0。項目ごとの固有の句で grep するので、測るものを消すと落ちる（R7 は閉じた）
- D1 の塊を一時に置いて 1.1・1.2・3.2 が rc=0。置いたファイルは消した
- 並べ替えの番号は tasks・design・CH13.md・enable-before.md で揃っている。PR 本文の 1 か所だけ古い（→ R11）
- 1,786 本の走査（D1）: ホスト 35・docker 319・箱 1,432（文書の 1,781 本との差は CH13 自身の tasks.md）
- R7（2.4 の grep）と R8（Task の並べ替え・5.1 の `pgrep: [0-9]+`）の直しは、検証の強さと Task の順で、D2 の中身（何を一覧に残し、何を issue にするか）を変えない。answers.md の「答え以外の変更」には入れなかった。R6 の終了コード（4.1 の rc=3）も同じく検証の形で、D8（仮）の節には書かれていないので入れなかった。R9（裸の `cargo test`）は D6（仮）の節に書かれているので入れた

## R10. D1 の `-p ashiato-server` と裸の `cargo test` の終わりの判定が、引用符で囲んだ絞り込み（`"$1"`）を「絞り込みなし」と読み、`-- --list` で数えるだけの検証に docker の socket を渡す
- 成果物: openspec/changes/ch13-workbox-config/design.md
- 根拠: design.md:78 の 4 本目の正規表現は、`-p\s+ashiato-server\s*` と裸の `cargo\s+test\b\s*` の後に `($|['";&|)]|[0-9]?>)` を求める。`"` も終わりとして数えるので、`cargo test -p ashiato-server "$1" -- --list` の `"$1"` の開きの引用符で当たる。
  走査（`evidence.items` → `declared` → `unwrap_quiet` の 1,786 本）で、依頼の版では docker が渡らず D1 で渡るもののうち、`-- --list` で試験の本数を数えるだけのものが 20 本（archive の ch03・ch04。例 `bash -c 'set -- ch04_testdb_ 3; [ "$(cargo test -p ashiato-server "$1" -- --list 2>/dev/null | grep -c ": test$")" -ge "$2" ]'`）。
  `cargo test -p ashiato-server "st99_name"` も docker。この形は AGENTS.md:24（「`cargo test <名前>` 型は `-- --list` で 1 本以上あることも確かめる」）が勧める型で、後の Story が書き続ける。答えの (1)「台本の名前を grep するだけの検証に socket が渡らないようにする（当たりすぎ）」と同じ種類の当たりすぎで、design.md:149 の D6（仮）の当たり方の変化はこれを挙げていない。match-cases.json の `--list` の行（170 行目）は引用符の無い形だけ
- kind: technical
- 提案: 終わりの判定の引用符を「閉じの引用符」に限る（例 `(['"](\s|$|[;&|)])|$|[;&|)]|[0-9]?>)`。`bash -c 'cargo test -p ashiato-server'` の閉じは当たり、`"$1"` の開きは当たらない）。match-cases.json に上の `set -- …; cargo test -p ashiato-server "$1" -- --list` の形を `box` で 1 行足す。直さないなら D6（仮）の「残るもの」に、この形が socket を受け取ることを書く
- 処置: fixed D6 — 第 2 回の合意の違和感 (1) で docker の欄ごと消えた（台帳で resolve。3 回目の R12 で処置の形を直した）。当時は引用符は閉じのとき（後が空白・行末・`;&|)`）だけ終わりとみなす形にし、match-cases.json に `"$1" -- --list` の形を box で 1 行足した（35 行）。1,781 本の分かれ方はホスト 35・docker 299・箱 1,447

## R11. PR 本文の Q3 の行が、当たり方の確かめを並べ替える前の「tasks 5」と呼んでいる
- 成果物: docs/briefs/CH13-pr.md
- 根拠: docs/briefs/CH13-pr.md:30「直した正規表現の当たり方の確かめ（D8（仮）・tasks 5）」/ 並べ替えの後、当たり方の確かめは Task 4（tasks.md:47-55）で、Task 5 は PR 本文と issue（tasks.md:57-69）。同じ本文の 47 行目は「当たり方の確かめと分類の一覧（4.1・4.2）」で、食い違う
- kind: technical
- 提案: 30 行目の「tasks 5」を「tasks 4」に直す
- 処置: fixed 5.1 — PR 本文（Task 5.1 が見る docs/briefs/CH13-pr.md）の Q3 の行を tasks 4 に直した

## 第 2 回の合意の違和感の後（2026-11-08。3 回目）

対象: commit `8caf47e94`（`git diff 730decb9d HEAD`。proposal・design・tasks・enable-before.md・match-cases.json・premises.md・provisional.jsonl・deep.md の `→ 対応:`・`docs/changes/CH13.md`・`docs/briefs/CH13-pr.md`）。
harness2 は `/home/yosis/dev/harness2` の main `d65ff3b4a`（#161 の merge）の `scripts/workbox.py`・`scripts/evidence.py`・`workbox/`（`docker-shim` `dockerd-up` `Dockerfile`）を読んだ。mode は `spec`（`--answers` / `--iwakan` ではない）。

### 機械の検査

| コマンド | 結果 |
|---|---|
| `openspec validate ch13-workbox-config --strict` | rc=0（`skip_specs is set … zero deltas accepted`） |
| `python3 scripts/check_chain.py .` | rc=0（chain: OK） |
| `python3 scripts/check_scenarios.py . ch13-workbox-config` | rc=0（scenarios: OK。warn は他の Story の印だけ） |
| `python3 scripts/review_triage.py . ch13-workbox-config` | **rc=1（triage: FAIL 1 件）`spec.md R2: design.md の D2 に「仮」の印が無い`**（→ R12） |
| `python3 scripts/spec_overlap.py candidates . ch13-workbox-config` | `[]`（0 件） |
| `python3 scripts/spec_overlap.py check . ch13-workbox-config` | rc=0（`{"ok": true, "candidates": 0}`） |
| `python3 scripts/evidence.py lint openspec/changes/ch13-workbox-config` | rc=0 |
| `python3 scripts/provisional.py check openspec/changes/ch13-workbox-config` | rc=1（D9 の `provisional-review.md` が無い）—— グラフの provisional_review が書く前の状態で、この段では正常 |

### 追加で確かめたこと

- **違和感 (1)〜(5) の写り**（見てほしいもの 1）:
  - (1)(5) design D1 の JSON は `docker` `prep_files` が無く、`docker_images: ["postgres:17-alpine"]`、`host_only` は依頼の 4 本 + 3 本、ほかの 6 欄は依頼の版と同じ。Task 1.2 がこの 4 つの欄だけを機械で見る形になった。`docs/changes/CH13.md` の「依頼（逐語）」の節は `730decb9d` と HEAD で sha1 が同じ（書き換えていない）
  - (2) tasks 4.1・4.2 は `host_only` だけで `host` / `box`、`needs_docker` を呼ばない。`match-cases.json` は前の版の `docker` 17 行を `box` にしただけ（`cmd` の差分 0。35 行 = host 6・box 29）
  - (3) enable-before.md から #161 で消えた項目が「harness2 #157 / #161 で済んだもの」の案内に移り、測るもの 3 つ（DB の容器も数える旨つき）と「ホストで走らせる台本（`host_only` に足した分）」が残る。deep.md (3) の `→ 対応:` と一致
  - (4) Task 3.1 は残り、3.2 は `load_config` と `RETIRED` で見る形。deep.md (4) の `→ 対応:` と一致
  - deep.md の `→ 対応:` 5 行は、どれも実際の変更と合っている（食い違いは見つからなかった）
- **harness2 の実物との突き合わせ**（見てほしいもの 2）: Context の 5 項目・D1・D3・D7・D8 の主張は実物と合う —— `KEYS`（`workbox.py:61`）に `docker_images` があり `docker` `prep_files` は無い / `RETIRED`（:64）は止めずに stderr に出す（:124-125）/ 知らない鍵は `ValueError`（:121-123）/ `host_only` は `re.search`（:137-142）/ `evidence.py:398` の `in_box = box_cfg is not None and not workbox.host_only(box_cfg, cmd)` / `needs_docker` は無い / `image_tars` は `~/.cache/harness2-workbox/images` に save（:59・:203-229）/ `prune_images` は `until=168h`（:66・:174-178）/ `main` の `run` は `mode()` を見ずに `load_config` → `wrap`（:566-）/ `docker-shim` が最初の `docker` で `dockerd-up` を呼ぶ / 箱はホストの `PATH` を受け継ぐ（`env_flags` が `PATH` を落とさない。`ENV_DENY` に無い）。harness2 #157 は CLOSED（`gh issue view 157`）
- **D9 の再現**: D1 の中身を一時に `tools/workbox.json` に置き、`/usr/sbin` の無いこのシェルから `env -u HARNESS_WORKBOX python3 .harness2/scripts/workbox.py run . -- bash -c 'docker images'` → `failed to create NAT chain DOCKER: iptables not found`、rc=75（`dockerd-up` の終わり）。`PATH="$PATH:/usr/sbin:/sbin"` を足すと `postgres:17-alpine` が出て rc=0。`hx-dev.service` は `~/.config/systemd/user/` の利用者のファイルで、`Environment=PATH=…:/usr/local/bin:/usr/bin:/bin`（`/usr/sbin` 無し）。harness2 の `systemd/hx-tick@.service:16` の `PATH` にも `/usr/sbin` は無い
- **検証コマンド**（見てほしいもの 3。`evidence.items` → `declared` → `unwrap_quiet` で取り出した字面をそのまま走らせた）: D1 の塊を一時に置いて 1.1・1.2・2.1・2.2・2.4・3.2 が rc=0、2.3 が rc=0（67 秒）、3.1 が `pgrep: 1` rc=0。5.1 は rc=1（`pgrep: [0-9]+` は下流が書き足す。設計どおり）。
  壊した写し: 1.2 は `docker` を残す・`prep_files` を残す・知らない鍵を足す・`host_only` の順を変える・`env` を変える・`docker_images` を空にする、のどれも AssertionError で rc=1。3.2 は `docker` / `prep_files` を残すと `もう効かない鍵`、知らない鍵で `ValueError` の rc=1。
  design と依頼の写しを置いた使い捨ての木で、D1 の `memory` を `7g` にすると 1.2 は「変えてよい 4 つの欄の外が依頼の版と違う」、`host_only` に 1 本足すと「依頼の版の足し算でない」で rc=1。
  4.1 は、本文どおりに書いた使い捨ての台本（`workbox.host_only` を import、不一致で rc=3）を使い捨ての木に置いて rc=0。D1 では 35 行一致、`--request` では足した 4 行（install-check・build・side-by-side・usage-volume）だけが外れて rc=3。
  1,786 本の走査は D1 でホスト 35・箱 1,751、依頼の版でホスト 18（enable-before.md・premises.md の数と一致）。
  置いた `tools/workbox.json` は消し、`git status --short` が空であることを確かめた。5.2・5.3 は走らせていない（5.2 の検証の字面だけを、番号を差し替えた使い捨ての写しで走らせた。→ R13）
- **前回の R の直しが壊れていないか**（見てほしいもの 5）: R6 —— 4.1 の検証の字面は `730decb9d` から変わっていない（`{ … --request; test $? -eq 3; }`）。上の使い捨ての木で rc=0。R7 —— 2.4 は測るもの 3 つを項目ごとの句で grep したまま、`^## harness2 #157 / #161 で済んだもの` が足された。R10 —— D6 の正規表現は欄ごと消えたので前提が無くなった。`"$1" -- --list` の行は `box` で残り、D1 で一致。R1（`\x60{3}`）・R5（2.2 の未追跡）・R8（Task 5 が最後）・R11 も壊れていない。ただし triage が R2 で落ちる（→ R12）
- `host_only` の当たりすぎ（情報のみ。指摘にしない）: 1,786 本のホスト 35 本に、`bash -n tools/android-emulator.sh`（ch10）と `grep -c HealthPermission tools/android-emulator.sh`（st11 の archive）が入る。依頼の版の `tools/android-emulator\.sh` が字面に当たるためで、違和感 (5) が依頼の版のままとした欄。ホストで走るだけで落ちない（`all` で箱の中の agent から来たものは Task gate がホストで取り直す。`evidence.py:399-405`）

## R12. review_triage が R2 の処置 `fixed D2 仮` で落ちる（この commit で design の冒頭の行が変わったため）。merge_gate を通らない
- 成果物: openspec/changes/ch13-workbox-config/review/spec.md（R2 の処置）/ openspec/changes/ch13-workbox-config/design.md
- 根拠: `python3 scripts/review_triage.py . ch13-workbox-config` → `[FAIL] spec.md R2: design.md の D2 に「仮」の印が無い`（rc=1）。`review_triage.py:531` は「D2 と 仮 を同じ行に持つ行」を探す。`730decb9d` の design.md:4「本人の答えで決まった D2〜D5 は（仮）を外した」がたまたまそれに当たっていた。HEAD の design.md:6 は「D6〜D8 は（仮）を外した」で D2 の行が無くなった。
  D2 は Q1 (3) で本人が決めた決定なので、design に（仮）を戻すのは事実と違う。なお R9・R10 の `fixed D6 仮` が通っているのも、design.md:6・137・139 の「前の D6（仮）」の字面に当たっているだけ（D6 は台帳で resolve 済み）
- kind: technical
- 提案: R2 の処置を、実際の経路どおり `escalated — Q1 で本人に問い、Q1 (3) の答え（issue にも残す）で D2 に写した` に直す（kind=conflict は `escalated` で閉じられる。`review_triage.py:26`）。R9・R10 は字面の偶然に頼らないよう、処置を `fixed D6 — 違和感 (1) で docker の欄ごと消えた（台帳で resolve）` のような仮でない形に直すかを呼び出し元で決める
- 処置: fixed review/spec.md — R2 の処置を `escalated`（Q1 で本人に問い、Q1 (3) の答えを D2 に写した）に、R9・R10 の処置を仮でない `fixed D6`（違和感 (1) で欄ごと消えた）に直した。design に（仮）は戻していない

## R13. Task 5.2 の検証は、harness2 の issue の番号が存在するかだけを見て、題を見ない。無関係の #157 を書いても通る
- 成果物: openspec/changes/ch13-workbox-config/tasks.md
- 根拠: tasks.md の 5.2 の検証（`grep -cE "^- rai-256/harness2#[0-9]+ 作業箱: "` が 1 本 → `gh issue view "$n" -R rai-256/harness2 --json number`）/ enable-before.md の写しで `## harness2 の issue` の行を `- rai-256/harness2#157 作業箱: ホストの PATH に …` にして検証の字面を走らせると rc=0（#157 は CLOSED の別件「作業箱: docker を渡すかを字面で決めない・…」）。本文の「同じ題が既にあれば、その番号を書く」の誤読（似た題の #157 を「同じ題」と読む）を検証が止めない
- kind: technical
- 提案: 番号ごとに題を比べる（例 `gh issue view "$n" -R rai-256/harness2 --json title --jq .title | grep -qF "作業箱: ホストの PATH に /usr/sbin が無いと"`）。行の題の字面と一致させるだけでなく、GitHub 側の題で見る
- 処置: fixed 5.2 — 番号ごとに GitHub 側の題を `gh issue view --json title` で取り、`作業箱: ホストの PATH に /usr/sbin が無いと` を含むかを見る形にした

## R14. Task 2.4 は、一覧に残すと決めた 4 つのうち「上流で見つけた PATH の件」と「`host_only` に足した台本」の節を消しても通る
- 成果物: openspec/changes/ch13-workbox-config/tasks.md
- 根拠: 違和感 (3)「残すのは、測っていないもの…と、2 のホストが要る台本のうち host_only に足した分の確かめ」/ design D2（載せるものに `host_only` に足した台本の確かめ・D9（仮）の `PATH` の件）/ docs/changes/CH13.md:128 の完了の判定は「測るもの 3 つ・上流で見つけた `PATH` の件・…があり（Task 2.4）」と 2.4 を指す。
  enable-before.md の写しから `## 2. 上流で見つけたこと` と `## ホストで走らせる台本（host_only に足した分）` の 2 節を消して 2.4 の検証の字面を走らせると rc=0（`## harness2 の issue` の題の 1 行は残るので、5.2 も PATH の再現を消されたまま通る）。R7 と同じ形
- kind: technical
- 提案: 2.4 の検証に `grep -q "^## 2\. 上流で見つけたこと" "$f" && grep -qF "iptables not found" "$f" && grep -q "^## ホストで走らせる台本" "$f"` を足し、本文の「下流は一覧の測るものを消さない」を 2 つの節にも広げる
- 処置: fixed 2.4 — `^## 2\. 上流で見つけたこと`・`iptables not found`・`^## ホストで走らせる台本` の grep を足し、本文の「消さない」を 2 つの節にも広げた

## R15. D9（仮）と enable-before.md・PR 本文は、PATH の件の影響を「docker を使う検証と準備」と書くが、実際は箱に入る検証の全部（1,751 本）が BLOCKED になる
- 成果物: openspec/changes/ch13-workbox-config/design.md（D9（仮））/ openspec/changes/ch13-workbox-config/enable-before.md / docs/briefs/CH13-pr.md / openspec/changes/ch13-workbox-config/premises.md
- 根拠: design.md:163・enable-before.md:21・CH13-pr.md:10「docker を使う検証と検証の準備が箱の中で落ちる見込み」/ `evidence.py:422-450` は箱に入る検証のたびに先に `tools/verify-env.sh` を同じ箱で走らせ、rc≠0 なら検証を走らせずに BLOCKED_INFRA にする / `tools/verify-env.sh:128-129` は毎回 `clear_blocking_network`（`docker network ls`）と `docker compose up -d --wait db` を呼ぶ / 再現で `dockerd-up` は rc=75。
  したがって `/usr/sbin` が無いまま `tests` にすると、grep だけの検証も含めて箱に入る全部が BLOCKED_INFRA になる。下流の Task 5.2 は D9 の再現をそのまま harness2 の issue の本文にするので、重さが小さく伝わる。
  あわせて、issue の本文の材料として: 直す場所の候補は `workbox/dockerd-up:25` の `PATH="$bin:$PATH" setsid rootlesskit …`（ここで `/usr/sbin:/sbin` を足せば利用者の unit に依らない）、harness2 自身の `systemd/hx-tick@.service:16` の `PATH` にも `/usr/sbin` が無い（`hx-dev.service` は harness2 の repo に無い利用者のファイル）
- kind: technical
- 提案: 3 か所の文を「準備（`tools/verify-env.sh`）が毎回 docker を呼ぶので、箱に入る検証は全部 BLOCKED_INFRA になる見込み（2026-11-08 の走査で 1,786 本のうち 1,751 本）」に直し、D9（仮）の本文の材料に `dockerd-up:25` と `hx-tick@.service:16` を足す。D9 の決定（issue を 1 つ・`env` で回避しない）は変えない
- 処置: fixed D9 仮 — design D9（仮）・enable-before.md・PR 本文の影響を「準備が毎回 docker を呼ぶので、箱に入る検証は全部（1,751 本）BLOCKED_INFRA の見込み」に直し、issue の本文の材料に `dockerd-up` の PATH の行と `hx-tick@.service` を足した。D9 の決定は変えていない

## R16. Global Constraints は PR 本文に Task 5.2・5.3 の issue の番号を載せると言うが、確かめる Task が無く、5.1 は 5.2・5.3 より前に閉じる
- 成果物: openspec/changes/ch13-workbox-config/tasks.md
- 根拠: tasks.md:19「下流の PR 本文…の先頭に、…Task 5.2・5.3 の issue の番号を載せる」/ tasks.md:62 の 5.1 の検証は `enable-before.md`・`pgrep -fc /`・`pgrep: [0-9]+` だけを grep し、番号を見ない / グラフは Task を順に閉じるので、5.1 の時点で 5.2・5.3 の番号は無い。`730decb9d` の tasks.md:17 から同じ（今回の退行ではない）。R8 と同じ形
- kind: technical
- 提案: 5.3 の後に PR 本文の番号を確かめる項目を置く（例 5.4: `grep -qE "rai-256/harness2#[0-9]+" docs/briefs/CH13-pr.md && grep -qE "#[0-9]+.*作業箱を有効にする前の宿題" docs/briefs/CH13-pr.md`）か、Global Constraints から番号の句を外す
- 処置: fixed 5.4 — 5.3 の後に、PR 本文に 2 つの issue の番号があることを確かめる 5.4 を足した

## R17. 字句の古さ: match-cases.json の `why` が消えた D6 を指し、PR 本文の項目数が 14 になっている
- 成果物: openspec/changes/ch13-workbox-config/match-cases.json / docs/briefs/CH13-pr.md
- 根拠: match-cases.json:12・27・87 の `why` が「（D6）」を出所に挙げる —— HEAD の design D6 は「docker の正規表現は持たない」で、この行の出所（前の D6（仮）の族の足し算）は消えた / CH13-pr.md:48「5 Task・14 項目」だが tasks.md の `- [ ]` は 13 本（`730decb9d` から同じ）
- kind: technical
- 提案: `why` の「D6」を「前の D6（仮）。#161 で docker の欄ごと消えた」か削除に、PR 本文を「13 項目」に直す
- 処置: fixed match-cases.json — `why` の「D6」を「前の D6（仮）。#161 で docker の欄ごと消えた」に直した。PR 本文の項目数は 5.4 を足して 14 になった

### 見てほしいもの 4: D9（仮）の範囲と、P でよいか

- 範囲: 違和感 (5) は `tools/workbox.json` の変えてよい欄を 4 つに限り、`env` を依頼の版のままとしている。D9（仮）は `env` に `PATH` を足す回避を採らず（design.md:164）、`tools/workbox.json` も D1 から変えない —— (5) を越えていない。
  違和感 (3) は「残すのは」で一覧に残すものを 2 種に挙げたが、PATH の件はその後に上流が見つけた新しい事実で、(3) が消せと言った項目（#161 で消えたもの）には当たらない。宿題の一覧に 1 節足すのは (3) の列挙の外だが、Q1 (2)「根本は harness2 の issue」が拠り所で、design の Goals / Non-Goals（harness2 を直さない）とも矛盾しない
- P でよいか: 失う側は見当たらない。issue は閉じれば戻り、`HARNESS_WORKBOX` が off の間 ashiato2 の振る舞いは変わらない。逆の選択（一覧に置くだけ・issue を立てない）も失うものは無く、反転条件（design.md:168）にその場合が書いてある。台帳（`provisional.jsonl` の D9）と design の（仮）の印が揃い、PR 本文の「仮決め」にも出ている。本人は次の合意でこれを見る。外向きの作用（harness2 への issue）は、第 1 回の Q1 (2) で本人が harness2 に issue を立てる形を選んでおり、前の版も 3 つ立てる予定だった
- 影響の書き方だけは R15

### 観点 1〜7（この回）

- 観点 1: deep の決定（Q1〜Q4・違和感 (1)〜(5)）は proposal・design（D1〜D9）・tasks・enable-before.md・match-cases.json・CH13.md の価値と完了の判定に写っている。数値（10 未満・6g・4g・5 本・35 行・1,700 本・`postgres:17-alpine`）は tasks の検証か D1 の JSON にある。要件へ戻すものは無い（`satisfies: []`）。写りの穴は検証の側（R13・R14・R16）
- 観点 2: 該当なし（確かめた範囲: `specs/` が無い。`.openspec.yaml` の `skip_specs: true`）
- 観点 3: 該当なし（確かめた範囲: D1〜D9 は置き場・中身・確かめ方・範囲で、ashiato2 の観測できる振る舞いを述べていない。proposal の Capabilities「無い」・`specs/` が無いこと・CH13.md の `capability: []` が一致）
- 観点 4: R13・R14・R16。ほかの項目はコマンドと終了条件があり、走らせたものは書いたまま通り、壊すと落ちた。人間の確認待ちは無く、CH13.md の「人間の確認待ちは無い」と一致。依存順は 5.2 → 5.3（5.3 は 5.2 の番号を書いた後に立てる）で正しい
- 観点 5: `docs/changes/CH13.md` の「完了の判定」は空でなく、違和感 (1)〜(5) に合わせて書き直されている。「壊してはいけないもの」1〜4 は Task 2.1・2.2・2.3・1.2 / 4.1 が確かめる。「依頼（逐語）」は書き換えていない（節の sha1 が `730decb9d` と同じ）。依頼の範囲を越える足し算（`host_only` の 3 本・`docker_images`・当たり方の確かめ・issue）は Q1・Q3・違和感 (1)(5) に決定がある。D9（仮）は上の「見てほしいもの 4」
- 観点 6: 該当なし（mode は `spec`。`--answers` / `--iwakan` ではない）。グラフへの注意として: `review/answers.md` の `spec-digest`（`fc83512a…`）は今の `agreement.py spec-digest`（`646b8179…`）と違い、「答え以外の変更の決定: D6 D7 D8」は台帳で resolve 済みの決定を名指している
- 観点 7: 該当なし（spec_overlap.py candidates が 0 件）。proposal の Impact の st53（actionlint）・st56（e2e）・ch10（`tools/test-android-emulator.sh` は `box`、`tools/android-emulator.sh` は `host`）の読みは match-cases.json と走査で確かめ、どれも独立のまま
