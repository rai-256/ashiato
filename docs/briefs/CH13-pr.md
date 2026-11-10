# CH13 —— 作業箱（コンテナ隔離）の設定を置く

change: `openspec/changes/ch13-workbox-config/`

## deep.md に積んだ未決（A）

なし

## 仮決め（B）と Ruling

design の（仮）:
- D9（仮）. 上流で見つけた `PATH` の件を harness2 の issue に 1 つ立てる（Q1 (2) に従う）

全 Task の ledger の `Ruling:`（逐語）:
- 全 Task の ledger の `Ruling:`（逐語）:
- Ruling: settle（review/code.md R18・R19）—— 作業箱を有効にする前の宿題は終わっていない。正本は `openspec/changes/ch13-workbox-config/enable-before.md`（同じ本文で ashiato2 の issue #275 作業箱を有効にする前の宿題（CH13）を立てた）。merge の後、`HARNESS_WORKBOX` を有効にする操作の前にこれを読む
- Ruling: settle（R19）—— 宿題のうち測っていないもの 3 つ: 画面の e2e 1 本が検証の箱の 6g に収まるか（箱の中の DB の容器も数える）・`all` で 5 本のときの空きメモリ・agent の箱の中の cargo のビルドが 4g に収まるか
- Ruling: 本人の答え（deep.md Q-D10）—— `host_only` の漏れ（`tools/release/sandbox-probe.sh`）は CH13 の中で `host_only` と `match-cases.json` に足した（design D10）
- Ruling: settle（R18・R19）—— 上流で見つけた `PATH` の件（ホストの `PATH` に `/usr/sbin` が無いと箱の中の dockerd が iptables を見つけられず立たない）は rai-256/harness2#165（CLOSED。#164 で直った）。直すものとしては残っていない
- Ruling: settle（R19）—— 依頼の完了の判定の pgrep の確かめ（Task 3.1）: `HARNESS_WORKBOX` を外したシェルから `python3 .harness2/scripts/workbox.py run . -- bash -c 'pgrep -fc /'` → `pgrep: 1`（10 未満）

## Task

5 Task のうち、埋めた Task は 4 本（Task 1, Task 2, Task 3, Task 4）。

残した項目:
- Task 5 の 5.2（PR ができてから走る検証（gate が持つ））: 5.2 harness2 に、上流で見つけた `PATH` の件を issue にする（Q1 (2)。design D9（仮））。題は `作業箱: ホストの PATH に /usr/sbin が無いと箱の中の dockerd が iptables を見つけられず立たない`。
- Task 5 の 5.3（PR ができてから走る検証（gate が持つ））: 5.3 `enable-before.md` を本文にした ashiato2 の issue を 1 つ立てる（Q1 (3)。design D2）。5.2 の番号を一覧に書いた後に立てる。題は `作業箱を有効にする前の宿題（CH13）`。

## 通した検証（`evidence.jsonl` の項目とコマンドごとの最新）

- 1.1 `python3 -c 'import json; json.load(open("tools/workbox.json", encoding="utf-8"))'` → rc=0（PASS・head e587f1a）
- 1.2 `python3 -c 'import json,re;g=lambda p,h:open(p,encoding="utf-8").read().split(h,1)[1];d=re.split(r"\n### D",g("openspec/changes/ch13-workbox-config/design.md","### D1."),1)[0];b=re.findall(r"\x60{3}json\n(.*?)\n\x60{3}",d,re.S);assert len(b)==1,"D1 の JSON の塊が %d 個" % len(b);w=json.loads(b[0]);assert w==json.load(open("tools/workbox.json",encoding="utf-8")),"tools/workbox.json が D1 と違う";t=g("docs/changes/CH13.md","## 依頼（逐語）").split("\n## ",1)[0];t="\n".join(re.sub(r"^> ?","",l) for l in t.splitlines());r=re.findall(r"\x60{3}json\n(.*?)\n\x60{3}",t,re.S);assert len(r)==1,"依頼の JSON の塊が %d 個" % len(r);r=json.loads(r[0]);k={"docker","prep_files","docker_images","host_only"};assert {a:v for a,v in w.items() if a not in k}=={a:v for a,v in r.items() if a not in k},"変えてよい 4 つの欄の外が依頼の版と違う";assert "docker" not in w and "prep_files" not in w,"docker か prep_files が残っている";assert w["docker_images"]==["postgres:17-alpine"],"docker_images が違う";assert w["host_only"]==r["host_only"]+["tools/release/(build|install-check)\\.sh","tools/android-side-by-side\\.sh","tools/usage-volume\\.sh"],"host_only が依頼の版の足し算でない"'` → rc=0（PASS・head e587f1a）
- 2.1 `bash -c '! git grep -n -i workbox -- . ":!tools/workbox.json" ":!openspec/changes" ":!docs/changes/CH13.md" ":!docs/briefs"'` → rc=0（PASS・head 0fd3c60）
- 2.2 `bash -c 'set -o pipefail; base=$(git merge-base origin/main HEAD) || exit 1; files=$( { git diff --name-only "$base" -- . && git ls-files -o --exclude-standard; } ) || exit 1; ! printf "%s\n" "$files" | grep -vE "^(tools/workbox\.json|openspec/changes/ch13-workbox-config/|docs/changes/CH13\.md|docs/briefs/CH13|$)"'` → rc=0（PASS・head 0fd3c60）
- 2.3 `bash -c 'set -e; for s in check-private check-secrets check-spdx check-shared-appends check-tmp-hygiene check-boundaries; do tools/$s.sh; done'` → rc=0（PASS・head 0fd3c60）
- 2.4 `bash -c 'f=openspec/changes/ch13-workbox-config/enable-before.md; test -s "$f" && grep -qF "画面の e2e 1 本が検証の箱の 6g に収まるか" "$f" && grep -qF "で 5 本のときの空きメモリ" "$f" && grep -qF "cargo のビルドが 4g に収まるか" "$f" && grep -q "^## harness2 #157 / #161 で済んだもの" "$f" && grep -q "^## harness2 の issue" "$f" && grep -q "^## 2\. 上流で見つけたこと" "$f" && grep -qF "iptables not found" "$f" && grep -q "^## ホストで走らせる台本" "$f" && grep -q "command-classes.md" "$f"'` → rc=0（PASS・head 2af2cdb）
- 3.1 `bash -c 'n=$(env -u HARNESS_WORKBOX python3 .harness2/scripts/workbox.py run . -- bash -c "pgrep -fc /") && echo "pgrep: $n" && test "$n" -lt 10'` → rc=0（PASS・head 3947c08）
- 3.2 `python3 -c 'import sys;from pathlib import Path;sys.path.insert(0,".harness2/scripts");import workbox;cfg=workbox.load_config(Path("."),text=open("tools/workbox.json",encoding="utf-8").read());bad=sorted(set(cfg)&set(workbox.RETIRED));assert not bad,"もう効かない鍵: %s" % bad;assert set(cfg)<=workbox.KEYS'` → rc=0（PASS・head 3947c08）
- 4.1 `bash -c 'd=openspec/changes/ch13-workbox-config; python3 "$d/check-workbox-match.py" && { python3 "$d/check-workbox-match.py" --request; test $? -eq 3; } && test "$(python3 -c "import json; print(len(json.load(open(\"$d/match-cases.json\"))[\"cases\"]))")" -ge 35'` → rc=0（PASS・head dbf27ff）
- 4.2 `bash -c 'f=openspec/changes/ch13-workbox-config/command-classes.md; grep -q "^## ホストで走る（host_only）" "$f" && grep -q "^## 箱で走る" "$f" && ! grep -q "^## 箱で docker を渡" "$f" && test "$(grep -c "^- openspec/" "$f")" -ge 1700'` → rc=0（PASS・head dbf27ff）
- 5.1 `bash -c 'f=docs/briefs/CH13-pr.md; test -s "$f" && grep -q "enable-before.md" "$f" && grep -q "pgrep -fc /" "$f" && grep -qE "pgrep: [0-9]+" "$f"'` → rc=0（PASS・head 9861754）
- 5.2 `bash -c 'f=openspec/changes/ch13-workbox-config/enable-before.md; test "$(grep -cE "^- rai-256/harness2#[0-9]+ 作業箱: " "$f")" -eq 1 && for n in $(grep -oE "rai-256/harness2#[0-9]+" "$f" | cut -d"#" -f2); do gh issue view "$n" -R rai-256/harness2 --json title --jq .title | grep -qF "作業箱: ホストの PATH に /usr/sbin が無いと" || exit 1; done'` → rc=0（PASS・head 2af2cdb）
- 5.3 `bash -c 'n=$(gh issue list --state open --search "作業箱を有効にする前の宿題 in:title" --json number --jq length) && test "$n" -ge 1 && grep -qE "^issue: #[0-9]+" openspec/changes/ch13-workbox-config/enable-before.md'` → rc=0（PASS・head dbf27ff）
- 5.4 `bash -c 'f=docs/briefs/CH13-pr.md; grep -qE "rai-256/harness2#[0-9]+" "$f" && grep -qE "#[0-9]+.*作業箱を有効にする前の宿題" "$f"'` → rc=0（PASS・head 9861754）

## レビュー（`review/code.md`）

- 指摘 23 件（escalated 4・fixed 15・rejected 4）
- 処置の re-review: 11 回。最後の判定が NOT ADDRESSED のもの: R20, R21, R22

## 未処置の申し送り

なし
