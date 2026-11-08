# CH13 実装タスク — 作業箱（コンテナ隔離）の設定を置く

読む順: `deep.md`（**本人の答え Q1〜Q4**。第 1 回は推奨で先に書いてある）→ このファイル → `design.md` → `docs/changes/CH13.md`（依頼の逐語と完了の判定）→ `enable-before.md` → `premises.md` → `review/deep.md` → `CLAUDE.md`。

仕様を持たない change（`.openspec.yaml` の `skip_specs: true`。design D4（仮））なので、Scenario の印は無い。検証は各タスクの本文に書いてある。どれも docker・エミュレータ・harness2 の作業箱を要らない。

## Global Constraints

- **`tools/workbox.json` の中身は `docs/changes/CH13.md` の「依頼（逐語）」の JSON の塊のとおり。書き換えない・足さない・並べ替えない**（依頼の指定。design D1）。足りないものに気付いたら、ファイルを直さずに PR 本文に挙げ、`enable-before.md` に 1 行足す
- **触ってよいのは `tools/workbox.json` と、この change の中（`openspec/changes/ch13-workbox-config/`）と `docs/changes/CH13.md` と `docs/briefs/CH13*` だけ**（Q4。design D4（仮））。`tools/*.sh`・`docs/testing.md`・`AGENTS.md`・`CLAUDE.md`・`.github/workflows/`・`openspec/specs/` を変えない
- **本人の決定（下流は変えない）**: Q1 宿題の一覧は `enable-before.md` と PR 本文に置き、同じ一覧を issue にも残す（D2）/ Q2 後の Story への規約・検査を置かない（D5）/ Q3 pgrep の確かめを検証に置かない（D3）/
  Q4 仕様を持たず、ファイル 1 つだけ（D4）（第 1 回は推奨で先に書いた。推奨と違った答えは `spec --answers` が写す）
- **harness2 の作業箱を走らせない**（`scripts/workbox.py` は harness2 の main に無い。D3（仮））。`HARNESS_WORKBOX` を設定しない
- **下流の PR 本文（`docs/briefs/CH13-pr.md`）の先頭に、`enable-before.md` への案内と、その 0〜6 の見出し（0 の pgrep の確かめを含む）と、Task 3.2 の issue の番号を載せる**（Q1・Q3。D2（仮）・D3（仮））

## Task 1: 箱の設定を置く

- [ ] 1.1 `docs/changes/CH13.md` の「依頼（逐語）」の節の ```` ```json ```` の塊（行頭の `> ` を外したもの）を、そのまま `tools/workbox.json` に書く（末尾に改行 1 つ）。
  手で打ち直さず、塊を機械で取り出して書く（写し間違いを避ける）。
  検証: `python3 -c 'import json; json.load(open("tools/workbox.json", encoding="utf-8"))'` rc=0
- [ ] 1.2 置いたファイルが依頼の JSON と同じ中身であることを確かめる（JSON として読んで等しい。「依頼（逐語）」の節に塊がちょうど 1 つあることも見る。design D1）。
  検証: `python3 -c 'import json,re;t=open("docs/changes/CH13.md",encoding="utf-8").read().split("## 依頼（逐語）",1)[1].split("\n## ",1)[0];t="\n".join(re.sub(r"^> ?","",l) for l in t.splitlines());b=re.findall(r"\x60{3}json\n(.*?)\n\x60{3}",t,re.S);assert len(b)==1,"依頼の JSON の塊が %d 個" % len(b);assert json.loads(b[0])==json.load(open("tools/workbox.json",encoding="utf-8")),"tools/workbox.json が依頼の中身と違う"'` rc=0

## Task 2: off の間に何も変わらないことを確かめる

- [ ] 2.1 ashiato2 の台本・コード・CI・文書が `workbox` を読まないことを確かめる（`HARNESS_WORKBOX` が off の間、このファイルを読むのは harness2 だけ。壊してはいけないもの 1）。
  検証: `bash -c '! git grep -n -i workbox -- . ":!tools/workbox.json" ":!openspec/changes" ":!docs/changes/CH13.md" ":!docs/briefs"'` rc=0
- [ ] 2.2 この change が触ったファイルが、許した範囲（Global Constraints の 2 つ目）の中だけであることを確かめる（壊してはいけないもの 2）。
  作業ツリーの変更と未追跡のファイルも見る。`git` が落ちたら落ちる。
  検証: `bash -c 'set -o pipefail; base=$(git merge-base origin/main HEAD) || exit 1; files=$( { git diff --name-only "$base" -- . && git ls-files -o --exclude-standard; } ) || exit 1; ! printf "%s\n" "$files" | grep -vE "^(tools/workbox\.json|openspec/changes/ch13-workbox-config/|docs/changes/CH13\.md|docs/briefs/CH13|$)"'` rc=0
- [ ] 2.3 ファイルを置いた木で、リポジトリの検査 6 本が通ることを確かめる（壊してはいけないもの 3）。
  検証: `bash -c 'set -e; for s in check-private check-secrets check-spdx check-shared-appends check-tmp-hygiene check-boundaries; do tools/$s.sh; done'` rc=0
- [ ] 2.4 有効にする前の宿題の一覧が change の中にあり、依頼の完了の判定から外した pgrep の確かめを持つことを確かめる（Q1・Q3。D2・D3）。下流は一覧を書き直さない（足すだけ）。
  検証: `bash -c 'f=openspec/changes/ch13-workbox-config/enable-before.md; test -s "$f" && grep -q "pgrep -fc /" "$f" && grep -q "HARNESS_WORKBOX" "$f"'` rc=0

## Task 3: 宿題の一覧を PR 本文と issue に残す

- [ ] 3.1 `docs/briefs/CH13-pr.md`（上流が書いた PR 本文。下流の finish も同じファイルを PR 本文にする）に、`enable-before.md` への案内と、依頼の完了の判定から外した pgrep の確かめがあることを確かめる（Q1・Q3）。無ければ足す。
  検証: `bash -c 'f=docs/briefs/CH13-pr.md; test -s "$f" && grep -q "enable-before.md" "$f" && grep -q "pgrep -fc /" "$f"'` rc=0
- [ ] 3.2 `enable-before.md` を本文にした issue を 1 つ立てる（Q1 の推奨「issue にも残す」。design D2（仮））。題は `作業箱を有効にする前の宿題（CH13）`。
  **同じ題の issue が既にあれば立てない**（`gh issue list --state all --search "作業箱を有効にする前の宿題 in:title"` で確かめてから `gh issue create --title … --body-file openspec/changes/ch13-workbox-config/enable-before.md`）。
  立てた issue の番号を `enable-before.md` の冒頭に 1 行で書く（`issue: #<番号>`）。gh が網に出られないときは、別の手で立てようとせず、design D2 の反転条件（PR 本文に「merge の後に人間が立てる」1 行）を A として人間に返す。
  検証: `bash -c 'n=$(gh issue list --state open --search "作業箱を有効にする前の宿題 in:title" --json number --jq length) && test "$n" -ge 1 && grep -qE "^issue: #[0-9]+" openspec/changes/ch13-workbox-config/enable-before.md'` rc=0
