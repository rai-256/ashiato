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
- 処置: fixed D2 仮 — 推奨どおり issue にも残す形にした: 下流の Task 3.2 が同じ題が無いときだけ 1 つ立てる。CH13 の issue を置き場にしない理由を「`closes` を書けば閉じる」に直した。CH13.md の価値・完了の判定と proposal も揃えた

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
