# CH18 —— 作業箱の設定に圧縮ライブラリと CPU の上限を足す

change: `openspec/changes/ch18-workbox-libs-cpus/`

## deep.md に積んだ未決（A）

なし

## 仮決め（B）と Ruling

design の（仮）: なし

全 Task の ledger に `Ruling:` の行は無い。

## Task

2 Task のうち、埋めた Task は 2 本。
残した項目は無い。

## 通した検証（`evidence.jsonl` の項目とコマンドごとの最新）

- 1.1 `python3 -c 'import json,subprocess as s;b=s.run(["git","merge-base","origin/main","HEAD"],capture_output=True,text=True,check=True).stdout.strip();o=json.loads(s.run(["git","show",b+":tools/workbox.json"],capture_output=True,text=True,check=True).stdout);n=json.load(open("tools/workbox.json",encoding="utf-8"));k=len(o["apt"]);assert n["apt"][:k]==o["apt"],"既存の apt が変わった";assert sorted(n["apt"][k:])==["libbz2-dev","liblzma-dev"],"足した apt が 2 つでない: %s" % n["apt"][k:];r=lambda d:{a:v for a,v in d.items() if a!="apt"};assert r(n)=={**r(o),"cpus":6},"apt と cpus の外が変わった";assert n.get("parallel_verify")==2;assert open("tools/workbox.json",encoding="utf-8").read()==json.dumps(n,indent=2,ensure_ascii=False)+"\n","整形が今のファイルと違う"'` → rc=0（PASS・head 83cb773）
- 1.2 `python3 -c 'import sys;from pathlib import Path;sys.path.insert(0,".harness2/scripts");import workbox;cfg=workbox.load_config(Path("."),text=open("tools/workbox.json",encoding="utf-8").read());assert set(cfg)<=workbox.KEYS;assert not set(cfg)&set(workbox.RETIRED);assert cfg["cpus"]==6;assert {"liblzma-dev","libbz2-dev"}<=set(cfg["apt"])'` → rc=0（PASS・head 83cb773）
- 1.3 `scripts/quiet-run cg -- env -u HARNESS_WORKBOX_CONFIG python3 .harness2/scripts/workbox.py run . -- bash -c 'cat /sys/fs/cgroup/cpu.max; dpkg-query -W -f="\${Package} \${Status}\n" liblzma-dev libbz2-dev; test "$(cat /sys/fs/cgroup/cpu.max)" = "600000 100000" && test "$(dpkg-query -W -f="\${Status}\n" liblzma-dev libbz2-dev | grep -c "install ok installed")" -eq 2'` → rc=0（PASS・head 83cb773）
- 2.1 `scripts/quiet-run wb -- env -u HARNESS_WORKBOX_CONFIG HARNESS_WORKBOX=tests python3 .harness2/scripts/workbox.py run . bash -c 'export TMPDIR=$(mktemp -d); trap "rm -rf \"\$TMPDIR\"" EXIT; . ./tools/agent-env.sh; tools/verify-env.sh && cargo test --workspace --no-fail-fast && tools/smoke.sh && (cd web && npm run test:e2e) && ./collector-android/gradlew -p collector-android :app:testDebugUnitTest'` → rc=0（PASS・head 1110490）
- 2.2 `bash -c 'set -o pipefail; base=$(git merge-base origin/main HEAD) || exit 1; files=$( { git diff --name-only "$base" -- . && git ls-files -o --exclude-standard; } ) || exit 1; ! printf "%s\n" "$files" | grep -vE "^(tools/workbox\.json|docs/changes/CH18\.md|openspec/changes/ch18-workbox-libs-cpus/|docs/briefs/CH18|\.superpowers/|$)"'` → rc=0（PASS・head 1110490）

## レビュー（`review/code.md`）

- 指摘 6 件（rejected 6）
- 処置の re-review: 1 回。最後の判定が NOT ADDRESSED のもの: なし

## 未処置の申し送り

なし
