# CH23 実装タスク — 毎晩の脆弱性の検査が issue を立てられず、9 晩赤のまま誰にも届かなかったのを直す

読む順: `proposal.md` → `design.md`（D1〜D4）→ `premises.md` → このファイル → `docs/changes/CH23.md`（依頼の逐語・壊してはいけないもの・完了の判定）→ `tools/check-vulns-issues.py` → `tools/check-vulns.sh` の `self_test` の偽の `gh` → `tools/check-vulns-wiring.sh`。

軽い路（`lane.json` の `reason: bug`。名指しの Scenario「落ちた夜に、依存ごとに 1 件の issue が立つ」）。実装の後に、実際の diff が予定のパスの中にあること・
名指しの Scenario の項目（1.1）が、直す前（本番の台本が base のまま）の木で落ち、最新で通った証跡があることを機械が見る（外れると通常の路へ昇格する）。

## Global Constraints

- **触ってよいのは `tools/check-vulns-issues.py`・`tools/check-vulns.sh`・`tools/check-vulns-wiring.sh`・`.github/workflows/vulns-nightly.yml`・`docs/testing.md`・`docs/changes/CH23.md`・`openspec/changes/ch23-vulns-nightly-issue-fix/**` だけ**（`lane.json` の予定のパス。グラフが置く `docs/briefs/CH23*` と `.superpowers/` は数えない）。製品のコード・`osv-scanner.toml`・`nightly.yml`・`ci.yml`・`release.yml`・`openspec/specs/`・`.harness2` に触れない
- **正典の Scenario の約束を変えない**: 既にある自己試験の Scenario（`== Scenario: …` の行と、その判定）は消さず・緩めない。偽の `gh` は `label list` の「ラベルが無いとき」の返し方だけを本物（空の文字。`premises.md`）に合わせ、`issue list`（`[]`）・`issue view`・`label create` の返し方は変えない
- **見つかった当たり（PostgreSQL 17.6 → 17.12・libsqlite3-sys・chrono・rustix）の処置をしない**（版を上げない・例外の表に足さない。#334〜#337 から別に扱う）
- **検査できなかった夜（rc=2）の扱いを変えない**（D3）。`report-unfiled` は `file-issues` の段が落ちたときだけ走る
- 本物の `gh` で issue を立てない（自己試験は偽の `gh` だけを使う）
- **長いコマンドは `scripts/quiet-run`**（`tools/check-vulns.sh --self-test` は osv.dev に問い合わせ、約 2 分かかる。1 回で待つ）

## Task 1: 偽の gh を本物の返し方に合わせ、ラベルが無くても issue を立てる

- [ ] 1.1 Scenario: 落ちた夜に、依存ごとに 1 件の issue が立つ を試験で確かめる。直す前に verify-run で落ちることを記録する。
  手順: (a) `tools/check-vulns.sh` の `self_test` の偽の `gh` の `label list` を、`GH_LABEL_EXISTS=1` のときは今のとおり `[{"name":"脆弱性"}]`、それ以外は**何も出さない**（rc=0。本物と同じ空の文字）に変える。
  `tools/check-vulns-issues.py` にはまだ触らない。(b) `scripts/verify-run 1.1` を走らせ、FAIL を記録する（ログの `== Scenario: 落ちた夜に、依存ごとに 1 件の issue が立つ` の後で止まり、`gh の応答: Expecting value` が出ていること —— osv.dev に届かずに落ちたのではないこと —— を自分で確かめる。
  2026-11-20 に spec-review が捨てる写しで再演し、この形で落ちることを確かめた）。
  `tools/**` は軽い路の分類で本番のコードに数えられないので、機械は「直す前」を木の差分で区別できない。FAIL の記録の `tree` で `tools/check-vulns-issues.py` が base と同じこと
  （`git diff --quiet <base> <tree> -- tools/check-vulns-issues.py`）を、code review の席が証跡で確かめる。
  (c) `tools/check-vulns-issues.py` の `gh_json` を直す（D1。`label list` の呼び出しだけ、rc=0 で空白だけの応答を `[]` として読む。`issue list`・`issue view` は今のまま）。(d) もう一度 `scripts/verify-run 1.1` を走らせて PASS を記録する。
  検証: `scripts/quiet-run vulns-st -- tools/check-vulns.sh --self-test` rc=0
- [ ] 1.2 specs の Requirement「gh がラベルの一覧に空の文字で答えたら、ラベルが無いと読む」の 3 つの Scenario を自己試験に足す（`self_test` の中、1.1 の節の後に、`echo "== Scenario: <名前>"` の印を付けて）:
  Scenario: ラベルが無いと空の文字で答える gh でも、ラベルを作ってから issue を立てる（1.1 の節の材料を使い、`label create 脆弱性` の行が最初の `issue create` の行より前にあることを `gh.log` の行番号で見る）・
  Scenario: ラベルがあれば、作り直さない（`GH_LABEL_EXISTS=1` で `file-issues` を走らせ、`gh.log` に `label create` が無いことを見る）・
  Scenario: 開いた issue のコメントの問い合わせが空の文字なら、rc=2 で落ちる（偽の `gh` に `issue view` を空の文字で答えさせる切り替えを新しい環境変数で足し、`GH_OPEN_ISSUE=1` と合わせて走らせる）。
  検証: `bash -c 'for s in "ラベルが無いと空の文字で答える gh でも、ラベルを作ってから issue を立てる" "ラベルがあれば、作り直さない" "開いた issue のコメントの問い合わせが空の文字なら、rc=2 で落ちる"; do grep -qF "== Scenario: $s" tools/check-vulns.sh || { echo "無い: $s"; exit 1; }; done'` rc=0
  検証: `scripts/quiet-run vulns-st -- tools/check-vulns.sh --self-test` rc=0

## Task 2: 知らせの段が落ちた夜に、決まった題の issue で知らせる

- [ ] 2.1 `tools/check-vulns.sh report-unfiled <run の URL>` を足す（`main` の分岐から `tools/check-vulns-issues.py report-unfiled <URL>` を呼ぶ）。振る舞いは specs の Requirement「毎晩の知らせの段が落ちた夜は、決まった題の 1 件の issue で知らせる」と design D2 のとおり
  （開いた issue は `gh issue list --state open --limit 1000 --json number,title` で、`--label` を付けずに読む・無ければ `--label` 無しの `issue create`・あれば `issue comment`・本文は 2 行・`label list`/`label create` を呼ばない・使い方と `gh` の失敗は rc=2）。
  偽の `gh` に、同じ題の開いた issue を返す切り替えと、`issue create` を 0 でない rc で落とす切り替えを足す（既存の切り替えの返し方は変えない）。
  自己試験（PostgreSQL の節の後）に、次の 6 つを `echo "== Scenario: <名前>"` の印つきで足す: 知らせの段が落ちた夜は、決まった題の 1 件の issue が立つ・知らせの issue はラベルに触れない・知らせの issue の本文は、日付と run の 2 行だけ・
  知らせの issue が開いていれば、立て直さずに追記する・知らせの issue を立てられなければ、rc=2 で落ちる・report-unfiled の引数が違えば、rc=2 で落ちる。本文は `GH_BODY_DIR` の写しを `diff` で見る（日付の行は `printf '夜: %s' "$(date -u +%F)"` と比べる）。
  検証: `bash -c 'for s in "知らせの段が落ちた夜は、決まった題の 1 件の issue が立つ" "知らせの issue はラベルに触れない" "知らせの issue の本文は、日付と run の 2 行だけ" "知らせの issue が開いていれば、立て直さずに追記する" "知らせの issue を立てられなければ、rc=2 で落ちる" "report-unfiled の引数が違えば、rc=2 で落ちる"; do grep -qF "== Scenario: $s" tools/check-vulns.sh || { echo "無い: $s"; exit 1; }; done'` rc=0
  検証: `scripts/quiet-run vulns-st -- tools/check-vulns.sh --self-test` rc=0
- [ ] 2.2 `.github/workflows/vulns-nightly.yml` の `issues` の job で、`file-issues` を呼ぶ段に `id: file` を付け、その後に D2 の段（`if: failure() && steps.file.outcome == 'failure'`・`run: tools/check-vulns.sh report-unfiled "$GITHUB_SERVER_URL/$GITHUB_REPOSITORY/actions/runs/$GITHUB_RUN_ID"`・`env: GH_TOKEN: ${{ github.token }}`）を足す。
  `issues` の job のほかの段・ほかの job・権限・`timeout-minutes`・材料が無ければ `file-issues` を呼ばない分岐は変えない。
  `tools/check-vulns-wiring.sh` の本体に、specs の 3 つの配線の Scenario（知らせの段は、材料を issue にする段の後にあり、run の URL を渡す・知らせの段は、材料を issue にする段が落ちたときだけ走る・材料が無い夜は、file-issues を呼ばない）の検査と `print("== Scenario: …")` の印を足す。
  自己試験に `expect_fail` を 4 つ足す（`sed` で写しの `vulns-nightly.yml` を書き換える）: 知らせの段を消す・その段の `if:` を `if: always()` にする・その段の `if:` から `failure() && ` を外す・材料の有無の分岐を外して `file-issues` を毎晩呼ぶ。
  検証: `tools/check-vulns-wiring.sh --self-test` rc=0
  検証: `bash -c 'out=$(tools/check-vulns-wiring.sh) || exit 1; for s in "知らせの段は、材料を issue にする段の後にあり、run の URL を渡す" "知らせの段は、材料を issue にする段が落ちたときだけ走る" "材料が無い夜は、file-issues を呼ばない" "毎晩の検査は公開の流れから独立し、issue の権限は 1 つの job だけ"; do grep -qF "== Scenario: $s" <<<"$out" || { echo "無い: $s"; exit 1; }; done'` rc=0
  検証（壊してはいけないもの: `issues` の外の job と、検査の段の `exit "$rc"` を変えない）: `python3 -c 'import re,subprocess as s;b=s.run(["git","merge-base","origin/main","HEAD"],capture_output=True,text=True,check=True).stdout.strip();o=s.run(["git","show",b+":.github/workflows/vulns-nightly.yml"],capture_output=True,text=True,check=True).stdout;n=open(".github/workflows/vulns-nightly.yml",encoding="utf-8").read();cut=lambda t:t[:t.index("\n  issues:\n")];assert cut(o)==cut(n),"issues の job より前（vulns・pg-minor・起動の条件・権限）が変わった";assert cut(n).count("exit \"$rc\"")==2'` rc=0

## Task 3: 文書を合わせ、触ったのが予定のパスだけであることを確かめる

- [ ] 3.1 `docs/testing.md` の `vulns-nightly.yml` の行に、`file-issues` が落ちた夜は同じ job の `tools/check-vulns.sh report-unfiled` が題 `脆弱性: 毎晩の知らせを issue にできなかった` の issue（ラベル無し）で知らせることを書き足す（その行の中だけ。ほかの行は変えない）。
  検証: `bash -c 'grep -E "^\| \`vulns-nightly.yml\`" docs/testing.md | grep -qF "report-unfiled"'` rc=0
- [ ] 3.2 この change が触ったファイルが予定のパスの中だけであることを確かめる（作業ツリーの変更と未追跡のファイルも見る。完了の判定 4）。
  検証: `bash -c 'set -o pipefail; base=$(git merge-base origin/main HEAD) || exit 1; files=$( { git diff --name-only "$base" -- . && git ls-files -o --exclude-standard; } ) || exit 1; ! printf "%s\n" "$files" | grep -vE "^(tools/check-vulns-issues\.py|tools/check-vulns\.sh|tools/check-vulns-wiring\.sh|\.github/workflows/vulns-nightly\.yml|docs/testing\.md|docs/changes/CH23\.md|openspec/changes/ch23-vulns-nightly-issue-fix/|docs/briefs/CH23|\.superpowers/|$)"'` rc=0

## 人間の確認待ち

- [ ] 4.1 確認バッチで本人に聞く（3 つ）:
  (1) 2026-11-19 に毎晩の検査が立てた #334〜#337（`脆弱性: crates.io chrono` など）の知らせが、本人に届いていたか（メールか GitHub の通知）。届いていなければ D2 の反転条件に当たる（issue では足りない）。
  (2) 2026-11-10〜11-18 の毎晩の検査の失敗の知らせ（予定した workflow の知らせは cron を最後に変えた人に送られる。`premises.md`）は届いていたか。届いていたなら、なぜ気づかなかったか（埋もれた・失敗が毎晩なので読まなくなった など）。
  (3) 検査できなかった夜（osv.dev や PostgreSQL の版の一覧に届かない）も issue で知りたいか（design D3。知りたければ、正典を変える change を通常の路で立てる）。
  > 物理: realdata（本人の受信箱と通知の設定は、機械から見えない）
