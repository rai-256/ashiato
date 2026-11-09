# CH13 実装タスク — 作業箱（コンテナ隔離）の設定を置く

読む順: `deep.md`（**本人の答え Q1〜Q4 と、第 2 回の合意の違和感 (1)〜(5)**）→ このファイル → `design.md`（D1 の JSON が置く中身の正本）→ `docs/changes/CH13.md`（依頼の逐語と完了の判定）→ `enable-before.md` → `match-cases.json` → `premises.md` → `review/deep.md` → `CLAUDE.md`。

仕様を持たない change（`.openspec.yaml` の `skip_specs: true`。design D4）なので、Scenario の印は無い。検証は各タスクの本文に書いてある。
docker と harness2 の作業箱のイメージを要るのは Task 3.1 だけ。エミュレータは要らない。
harness2 #161（2026-11-08 merge）で作業箱は検証の箱ごとに箱の中の dockerd を持ち、`docker` と `prep_files` の鍵は効かなくなった。この版はそれに合わせてある（design の冒頭）。

## Global Constraints

- **`tools/workbox.json` の中身は `design.md` の D1 の JSON の塊のとおり**（依頼の版に Q1 (1) と第 2 回の違和感 (1) の直しを当てたもの。Q1・Q4）。手で打ち直さず、塊を機械で取り出して書く。足りないものに気付いたら、ファイルを直さずに PR 本文に挙げ、`enable-before.md` に 1 行足す
- **`docs/changes/CH13.md` の「依頼（逐語）」を書き換えない**（依頼の版の比べ（Task 1.2・4.1）がそこから取り出す）
- **触ってよいのは `tools/workbox.json` と、この change の中（`openspec/changes/ch13-workbox-config/`）と `docs/changes/CH13.md` と `docs/briefs/CH13*` だけ**（Q4。design D4）。`tools/*.sh`・`docs/testing.md`・`AGENTS.md`・`CLAUDE.md`・`.github/workflows/`・`openspec/specs/` を変えない
- **本人の決定（下流は変えない）**: Q1 JSON で直せるものは CH13 で直し（D1）、根本は harness2 の issue に（#157 / #161 で済んだ。D7。上流で見つけた `PATH` の件だけ D9（仮））、測るものだけを宿題の一覧と issue に残す（D2）/ Q2 後の Story への規約・検査を置かない（D5）/
  Q3 pgrep の確かめと鍵の確かめと `host_only` の当たり方の確かめを tasks に置く（D3・D8）/ Q4 仕様を持たず、触るのはファイル 1 つと change の中の文書だけ（D4）/
  第 2 回の違和感: `docker` と `prep_files` を消し `docker_images` を足す（D1）・当たり方は `host_only` だけで `host` / `box` に分ける（D8）・`needs_docker` を呼ばない（無い）
- **`HARNESS_WORKBOX` を設定しない**（Task 3.1 は箱が off のときだけ成り立つ。design D3）
- **正規表現を写し直さない**。当たり方は `.harness2/scripts/workbox.py` の `host_only` をそのまま呼んで確かめる（harness2 と同じ判定。design D8）
- **下流の PR 本文（`docs/briefs/CH13-pr.md`）の先頭に、`enable-before.md` への案内と、Task 3.1 の pgrep の結果と、Task 5.2・5.3 の issue の番号を載せる**（Q1・Q3。D2・D3・D9（仮））

## Task 1: 箱の設定を置く

- [x] 1.1 `design.md` の `### D1.` の節の ```` ```json ```` の塊を、そのまま `tools/workbox.json` に書く（末尾に改行 1 つ）。
  手で打ち直さず、塊を機械で取り出して書く（写し間違いを避ける）。
  検証: `python3 -c 'import json; json.load(open("tools/workbox.json", encoding="utf-8"))'` rc=0
- [x] 1.2 置いたファイルが D1 の塊と同じ中身であること（JSON として読んで等しい。D1 の節に塊がちょうど 1 つ）と、依頼の版から変えたのが 4 つの欄だけであることを確かめる（design D1・違和感 (5)）:
  `docker` と `prep_files` が無い・`docker_images` が `["postgres:17-alpine"]`・`host_only` は依頼の版の後ろに 3 本を足しただけ・ほかの欄は依頼の版と同じ。
  検証: `python3 -c 'import json,re;g=lambda p,h:open(p,encoding="utf-8").read().split(h,1)[1];d=re.split(r"\n### D",g("openspec/changes/ch13-workbox-config/design.md","### D1."),1)[0];b=re.findall(r"\x60{3}json\n(.*?)\n\x60{3}",d,re.S);assert len(b)==1,"D1 の JSON の塊が %d 個" % len(b);w=json.loads(b[0]);assert w==json.load(open("tools/workbox.json",encoding="utf-8")),"tools/workbox.json が D1 と違う";t=g("docs/changes/CH13.md","## 依頼（逐語）").split("\n## ",1)[0];t="\n".join(re.sub(r"^> ?","",l) for l in t.splitlines());r=re.findall(r"\x60{3}json\n(.*?)\n\x60{3}",t,re.S);assert len(r)==1,"依頼の JSON の塊が %d 個" % len(r);r=json.loads(r[0]);k={"docker","prep_files","docker_images","host_only"};assert {a:v for a,v in w.items() if a not in k}=={a:v for a,v in r.items() if a not in k},"変えてよい 4 つの欄の外が依頼の版と違う";assert "docker" not in w and "prep_files" not in w,"docker か prep_files が残っている";assert w["docker_images"]==["postgres:17-alpine"],"docker_images が違う";assert w["host_only"]==r["host_only"]+["tools/release/(build|install-check)\\.sh","tools/android-side-by-side\\.sh","tools/usage-volume\\.sh"],"host_only が依頼の版の足し算でない"'` rc=0

## Task 2: off の間に何も変わらないことを確かめる

- [ ] 2.1 ashiato2 の台本・コード・CI・文書が `workbox` を読まないことを確かめる（`HARNESS_WORKBOX` が off の間、このファイルを読むのは harness2 だけ。壊してはいけないもの 1）。
  検証: `bash -c '! git grep -n -i workbox -- . ":!tools/workbox.json" ":!openspec/changes" ":!docs/changes/CH13.md" ":!docs/briefs"'` rc=0
- [ ] 2.2 この change が触ったファイルが、許した範囲（Global Constraints の 3 つ目）の中だけであることを確かめる（壊してはいけないもの 2）。
  作業ツリーの変更と未追跡のファイルも見る。`git` が落ちたら落ちる。
  検証: `bash -c 'set -o pipefail; base=$(git merge-base origin/main HEAD) || exit 1; files=$( { git diff --name-only "$base" -- . && git ls-files -o --exclude-standard; } ) || exit 1; ! printf "%s\n" "$files" | grep -vE "^(tools/workbox\.json|openspec/changes/ch13-workbox-config/|docs/changes/CH13\.md|docs/briefs/CH13|$)"'` rc=0
- [ ] 2.3 ファイルを置いた木で、リポジトリの検査 6 本が通ることを確かめる（壊してはいけないもの 3）。
  検証: `bash -c 'set -e; for s in check-private check-secrets check-spdx check-shared-appends check-tmp-hygiene check-boundaries; do tools/$s.sh; done'` rc=0
- [ ] 2.4 有効にする前の宿題の一覧が change の中にあり、測るもの 3 つと、harness2 #157 / #161 で済んだものの節と、harness2 の issue の節と、分類の一覧への案内を持つことを確かめる（Q1 (3)・違和感 (3)。design D2）。下流は一覧の測るもの・「2. 上流で見つけたこと」・「ホストで走らせる台本」の節を消さない（#161 で消えた項目は上流が消した）。
  検証: `bash -c 'f=openspec/changes/ch13-workbox-config/enable-before.md; test -s "$f" && grep -qF "画面の e2e 1 本が検証の箱の 6g に収まるか" "$f" && grep -qF "で 5 本のときの空きメモリ" "$f" && grep -qF "cargo のビルドが 4g に収まるか" "$f" && grep -q "^## harness2 #157 / #161 で済んだもの" "$f" && grep -q "^## harness2 の issue" "$f" && grep -q "^## 2\. 上流で見つけたこと" "$f" && grep -qF "iptables not found" "$f" && grep -q "^## ホストで走らせる台本" "$f" && grep -q "command-classes.md" "$f"'` rc=0

## Task 3: 箱の中で pgrep が箱の中しか見ないことと、鍵が今の harness2 で読めることを確かめる

- [ ] 3.1 依頼の完了の判定の pgrep の確かめ（Q3・違和感 (4)。design D3）。`HARNESS_WORKBOX` を外したシェルから、置いた `tools/workbox.json` で箱を 1 つ起こし、`pgrep -fc /` の数が 10 未満であること。
  箱が off のときだけ成り立つ（`HARNESS_WORKBOX=tests` の下では検証の側が先に箱に包む）。docker と作業箱のイメージが要る（初回はイメージを作り、`docker_images` の `postgres:17-alpine` をホストで pull・save するので数分かかりうる）。`docker` を呼ばないので箱の中の dockerd は立たない。出力の値を PR 本文に書く（5.1）。
  検証: `bash -c 'n=$(env -u HARNESS_WORKBOX python3 .harness2/scripts/workbox.py run . -- bash -c "pgrep -fc /") && echo "pgrep: $n" && test "$n" -lt 10'` rc=0
- [ ] 3.2 鍵の確かめ（Q3 の補足・review/deep.md R9。design D3）。harness2 の main の `workbox.load_config` で `tools/workbox.json` を読めて（知らない鍵があれば止まる）、もう効かない鍵（`workbox.RETIRED`。`docker` `prep_files`）を 1 つも持たないこと。
  検証: `python3 -c 'import sys;from pathlib import Path;sys.path.insert(0,".harness2/scripts");import workbox;cfg=workbox.load_config(Path("."),text=open("tools/workbox.json",encoding="utf-8").read());bad=sorted(set(cfg)&set(workbox.RETIRED));assert not bad,"もう効かない鍵: %s" % bad;assert set(cfg)<=workbox.KEYS'` rc=0

## Task 4: `host_only` が harness2 と同じ判定で当たる・当たらないことを確かめる

- [ ] 4.1 `openspec/changes/ch13-workbox-config/check-workbox-match.py` を書く（design D8・違和感 (2)）。`.harness2/scripts` を `sys.path` に足して `workbox` を import し、`match-cases.json` の各行を `workbox.host_only(cfg, cmd)` だけで `host` / `box` に分け、`want` と違う行を全部出して **rc=3**（表との不一致だけを 3 にし、読めない・例外などほかの失敗と分ける。review/spec.md R6）。`needs_docker` は呼ばない（harness2 #161 で消えた）。正規表現を台本に写さない。
  `cfg` は既定で `tools/workbox.json`、`--request` を付けると `docs/changes/CH13.md` の「依頼（逐語）」の JSON の塊（行頭の `> ` を外す）。依頼の版では、足した `host_only` の行が落ちること（台本が表を見分けること）も確かめる。
  検証: `bash -c 'd=openspec/changes/ch13-workbox-config; python3 "$d/check-workbox-match.py" && { python3 "$d/check-workbox-match.py" --request; test $? -eq 3; } && test "$(python3 -c "import json; print(len(json.load(open(\"$d/match-cases.json\"))[\"cases\"]))")" -ge 35'` rc=0
- [ ] 4.2 同じ台本に `--list <出力>` を足し、既存の `openspec/**/tasks.md`（archive を含む）の検証コマンドを、`.harness2/scripts/evidence.py` の `items` → `declared` → `unwrap_quiet` で取り出して、同じ判定で `## ホストで走る（host_only）` / `## 箱で走る` の 2 つの節に分けて書き出す（違和感 (2)）。
  各行は `- <tasks.md のパス> <項目の番号>: <コマンド>`（コマンドは 1 行に。長ければ切らずにそのまま）。冒頭に節ごとの件数と、書き出したときの HEAD を書く。
  `openspec/changes/ch13-workbox-config/command-classes.md` に書き出して commit し、`enable-before.md` の「分類の一覧」の節に件数を写す（2026-11-08 の上流の走査では 1,786 本のうちホスト 35・箱 1,751。下流のときの数が違ってよい）。
  検証: `bash -c 'f=openspec/changes/ch13-workbox-config/command-classes.md; grep -q "^## ホストで走る（host_only）" "$f" && grep -q "^## 箱で走る" "$f" && ! grep -q "^## 箱で docker を渡" "$f" && test "$(grep -c "^- openspec/" "$f")" -ge 1700'` rc=0

## Task 5: 宿題の一覧を PR 本文と issue に残す

- [ ] 5.1 `docs/briefs/CH13-pr.md`（上流が書いた PR 本文。下流の finish も同じファイルを PR 本文にする）に、`enable-before.md` への案内と、Task 3.1 の pgrep の確かめがあることを確かめる（Q1・Q3）。無ければ足す。下流は Task 3.1 の出力を `pgrep: <数>` の形で書き足す（review/spec.md R8）。
  検証: `bash -c 'f=docs/briefs/CH13-pr.md; test -s "$f" && grep -q "enable-before.md" "$f" && grep -q "pgrep -fc /" "$f" && grep -qE "pgrep: [0-9]+" "$f"'` rc=0
- [ ] 5.2 harness2 に、上流で見つけた `PATH` の件を issue にする（Q1 (2)。design D9（仮））。題は `作業箱: ホストの PATH に /usr/sbin が無いと箱の中の dockerd が iptables を見つけられず立たない`。
  **同じ題の issue が既にあれば立てない**（`gh issue list -R rai-256/harness2 --state all --search "<題> in:title"` で確かめてから `gh issue create -R rai-256/harness2 --title … --body …`）。
  本文は design D9（仮）の再現（`PATH` に `/usr/sbin` が無いシェルから `workbox.py run . -- bash -c 'docker images'` が `iptables not found` で落ち、足すと立つ・`hx-dev.service` の `PATH` にも無い）と、この change への案内。立てた番号を `enable-before.md` の `## harness2 の issue` の節に `- rai-256/harness2#<番号> <題>` の形で書く（同じ題が既にあれば、その番号を書く）。
  gh が網に出られないときは、別の手で立てようとせず、design D2 の最後の行（PR 本文に「merge の後に人間が立てる」1 行）を A として人間に返す。
  検証: `bash -c 'f=openspec/changes/ch13-workbox-config/enable-before.md; test "$(grep -cE "^- rai-256/harness2#[0-9]+ 作業箱: " "$f")" -eq 1 && for n in $(grep -oE "rai-256/harness2#[0-9]+" "$f" | cut -d"#" -f2); do gh issue view "$n" -R rai-256/harness2 --json title --jq .title | grep -qF "作業箱: ホストの PATH に /usr/sbin が無いと" || exit 1; done'` rc=0
- [ ] 5.3 `enable-before.md` を本文にした ashiato2 の issue を 1 つ立てる（Q1 (3)。design D2）。5.2 の番号を一覧に書いた後に立てる。題は `作業箱を有効にする前の宿題（CH13）`。
  **同じ題の issue が既にあれば立てない**（`gh issue list --state all --search "作業箱を有効にする前の宿題 in:title"` で確かめてから `gh issue create --title … --body-file openspec/changes/ch13-workbox-config/enable-before.md`）。
  立てた issue の番号を `enable-before.md` の冒頭に 1 行で書く（`issue: #<番号>`。同じ題が既にあれば、その番号を書く）。gh が網に出られないときは 5.2 と同じく人間に返す。
  検証: `bash -c 'n=$(gh issue list --state open --search "作業箱を有効にする前の宿題 in:title" --json number --jq length) && test "$n" -ge 1 && grep -qE "^issue: #[0-9]+" openspec/changes/ch13-workbox-config/enable-before.md'` rc=0
- [ ] 5.4 PR 本文（`docs/briefs/CH13-pr.md`）に、5.2 の harness2 の issue と 5.3 の ashiato2 の issue の番号を書く（Global Constraints の最後。review/spec.md R16）。5.2・5.3 の後に閉じる。
  検証: `bash -c 'f=docs/briefs/CH13-pr.md; grep -qE "rai-256/harness2#[0-9]+" "$f" && grep -qE "#[0-9]+.*作業箱を有効にする前の宿題" "$f"'` rc=0
