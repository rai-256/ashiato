# CH18 実装タスク — 作業箱の設定に圧縮ライブラリと CPU の上限を足す

読む順: `proposal.md` → このファイル → `docs/changes/CH18.md`（依頼の逐語・壊してはいけないもの・完了の判定）→ `.harness2/scripts/workbox.py` の冒頭の説明 → `CLAUDE.md`。

軽い路（`lane.json` の `reason: same`）。仕様を持たない change（`.openspec.yaml` の `skip_specs: true`）なので、Scenario の印は無い。検証は各項目の本文に書いてある。
実装の後に、実際の diff が予定のパスの中にあることを機械が見る（外に触れると通常の路へ昇格する）。

## Global Constraints

- **触ってよいのは `tools/workbox.json`・`docs/changes/CH18.md`・`openspec/changes/ch18-workbox-libs-cpus/**` だけ**（`lane.json` の予定のパス。グラフが置く `docs/briefs/CH18*` と `.superpowers/` は `lanes.py` と同じく数えない）。製品のコード・試験・CI・`docs/testing.md`・`AGENTS.md`・`CLAUDE.md`・`openspec/specs/`・`.harness2` には触れない
- **`tools/workbox.json` で変えるのは 2 つだけ**: `apt` の末尾に `liblzma-dev`・`libbz2-dev` を足す（既存の 16 個とその順は変えない）・`"cpus": 6` を置く。`parallel_verify`（2）・`memory`・`tmp_size`・`host_only`・`env`・`agent_memory`・`docker_images` は変えない。ほかに足りないものに気付いても、ファイルを直さずに PR 本文に挙げる
- **`HARNESS_WORKBOX` を設定しない・hx-dev.service とその drop-in（`~/.config/systemd/user/hx-dev.service.d/`）に触れない**（有効にするのは merge の後に人間が行う）。repo の外なので検証にはしない（merge の後に人間が有効にすると、走らせ直しで必ず落ちる）。最後のレビューの席が `systemctl --user cat hx-dev.service` に `HARNESS_WORKBOX` が無いことを見る。PR 本文にもその出力の有無を 1 行載せる
- **`docs/changes/CH18.md` の「依頼（逐語）」を書き換えない**
- 検証のうち 1.3・2.1 は docker と作業箱のイメージを要る（`apt` が変わるのでイメージを作り直す。初回は数分）。ホストで走る `scripts/verify-run` から `workbox.py run` で箱を 1 つ起こす形で、今の hx-dev.service（作業箱が off）で走る
- **箱の中の試験が落ちても、コードも試験も直さない・`tools/workbox.json` に 3 つ目の包みや別の欄を足さない**（どちらも範囲の外。完了の判定 1 は「2 つだけ」）。落ちた試験の名前・ログの場所・同じ試験がホストの base で通るか（`scripts/quiet-run` で 1 本だけ）を PR 本文に挙げ、A として人間に返す
- **長いコマンドは `scripts/quiet-run`**（2.1 は全体テストで 15〜20 分かかる。1 回で待つ）

## Task 1: 箱の設定に 2 つの包みと CPU の上限を置く

- [ ] 1.1 `tools/workbox.json` の `apt` の末尾に `"liblzma-dev"`・`"libbz2-dev"` を足し、`"cpus": 6` を置く（JSON の整形は今のファイルに合わせ、末尾に改行 1 つ）。
  置いたファイルが、base（`git merge-base origin/main HEAD`）の版から、`apt` の末尾の 2 つと `cpus` だけを変えたものであることを確かめる（壊してはいけないもの 1）。
  検証: `python3 -c 'import json,subprocess as s;b=s.run(["git","merge-base","origin/main","HEAD"],capture_output=True,text=True,check=True).stdout.strip();o=json.loads(s.run(["git","show",b+":tools/workbox.json"],capture_output=True,text=True,check=True).stdout);n=json.load(open("tools/workbox.json",encoding="utf-8"));k=len(o["apt"]);assert n["apt"][:k]==o["apt"],"既存の apt が変わった";assert sorted(n["apt"][k:])==["libbz2-dev","liblzma-dev"],"足した apt が 2 つでない: %s" % n["apt"][k:];r=lambda d:{a:v for a,v in d.items() if a!="apt"};assert r(n)=={**r(o),"cpus":6},"apt と cpus の外が変わった";assert n.get("parallel_verify")==2;assert open("tools/workbox.json",encoding="utf-8").read()==json.dumps(n,indent=2,ensure_ascii=False)+"\n","整形が今のファイルと違う"'` rc=0
- [ ] 1.2 harness2 の `workbox.load_config` で読めて（知らない鍵があれば止まる）、鍵が `workbox.KEYS` の中で、もう効かない鍵（`workbox.RETIRED`）を持たず、`cpus` が 6 であることを確かめる。
  検証: `python3 -c 'import sys;from pathlib import Path;sys.path.insert(0,".harness2/scripts");import workbox;cfg=workbox.load_config(Path("."),text=open("tools/workbox.json",encoding="utf-8").read());assert set(cfg)<=workbox.KEYS;assert not set(cfg)&set(workbox.RETIRED);assert cfg["cpus"]==6;assert {"liblzma-dev","libbz2-dev"}<=set(cfg["apt"])'` rc=0
- [ ] 1.3 置いた設定で箱を 1 つ起こし、箱の中に 2 つの包みが入っていて、CPU の上限が 6 本（cgroup v2 の `/sys/fs/cgroup/cpu.max` が `600000 100000`）であることを確かめる。
  **1.1 の前に** `scripts/verify-run 1.3` を走らせて FAIL を記録する（必須。base の設定の箱では包みが無く `cpu.max` が `max 100000` で落ちる。上流と spec-review が 2026-11-17 に確かめた）。FAIL のログに `max 100000` と `no packages found` が出ていること（箱が起きずに落ちたのではないこと）を自分で確かめ、1.1 の後にもう一度走らせて PASS を記録する。
  ホストと箱は worktree の `target/` を共有するので、2.1 はホストの組み立てで温まっていると包みが無くても通りうる。直す前と後を分けるのはこの項目（値はログに出る。確認バッチはそれを見る）。
  検証: `scripts/quiet-run cg -- env -u HARNESS_WORKBOX_CONFIG python3 .harness2/scripts/workbox.py run . -- bash -c 'cat /sys/fs/cgroup/cpu.max; dpkg-query -W -f="\${Package} \${Status}\n" liblzma-dev libbz2-dev; test "$(cat /sys/fs/cgroup/cpu.max)" = "600000 100000" && test "$(dpkg-query -W -f="\${Status}\n" liblzma-dev libbz2-dev | grep -c "install ok installed")" -eq 2'` rc=0

## Task 2: 箱の中で全体テストが通り、触ったのが予定のパスだけであることを確かめる

- [ ] 2.1 依頼の「確かめ方」のコマンド（検証の準備・`cargo test --workspace`・`tools/smoke.sh`・画面の e2e・携帯の単体）を、直した木で箱の中で走らせる（完了の判定 3。2026-11-17 に本人が同じ 2 つの包みを足した写しで通した。workspace 約 5 分・e2e 約 6 分・携帯の単体 約 2 分）。
  依頼の逐語から変えたのは 3 つだけ: `scripts/quiet-run` で包む・1.3 と同じ設定を見るよう `-u HARNESS_WORKBOX_CONFIG` を足す・`mktemp -d` で作ったフォルダを終わりに消す（verify-run が渡す一時フォルダの下に作られ、残ると `tmp_left` になる）。
  落ちたら、落ちたのが組み立てのリンク（`-llzma` / `-lbz2` などの `unable to find library`）か、試験の中身かをログで分ける。どちらも Global Constraints のとおり直さずに、PR 本文に挙げて A として人間に返す（`cpus: 6` を置いた状態で全体が通るかは、まだ実測されていない）。
  検証: `scripts/quiet-run wb -- env -u HARNESS_WORKBOX_CONFIG HARNESS_WORKBOX=tests python3 .harness2/scripts/workbox.py run . bash -c 'export TMPDIR=$(mktemp -d); trap "rm -rf \"\$TMPDIR\"" EXIT; . ./tools/agent-env.sh; tools/verify-env.sh && cargo test --workspace --no-fail-fast && tools/smoke.sh && (cd web && npm run test:e2e) && ./collector-android/gradlew -p collector-android :app:testDebugUnitTest'` rc=0
- [ ] 2.2 この change が触ったファイルが予定のパスの中だけであることを確かめる（作業ツリーの変更と未追跡のファイルも見る。`git` が落ちたら落ちる。完了の判定 4）。
  検証: `bash -c 'set -o pipefail; base=$(git merge-base origin/main HEAD) || exit 1; files=$( { git diff --name-only "$base" -- . && git ls-files -o --exclude-standard; } ) || exit 1; ! printf "%s\n" "$files" | grep -vE "^(tools/workbox\.json|docs/changes/CH18\.md|openspec/changes/ch18-workbox-libs-cpus/|docs/briefs/CH18|\.superpowers/|$)"'` rc=0
