# CH23 —— 毎晩の脆弱性の検査が issue を立てられず、9 晩赤のまま誰にも届かなかったのを直す

change: `openspec/changes/ch23-vulns-nightly-issue-fix/`

## deep.md に積んだ未決（A）

なし

## 仮決め（B）と Ruling

design の（仮）:
- D1（仮）. 空の文字を空の一覧とみなすのは、実測した `label list` だけ
- D2（仮）. 知らせの段が落ちた夜は、ラベル無しの決まった題の issue 1 件で知らせる
- D3（仮）. 検査できなかった夜（rc=2）は、正典のとおり issue を立てない
- D4（仮）. job の赤・緑の付き方は変えない
- D5（仮）. 毎晩の検査が立てる issue には、リポジトリの持ち主を担当者として付ける

全 Task の ledger に `Ruling:` の行は無い。

## Task

3 Task のうち、埋めた Task は 3 本。

残した項目:
- 4.1（人間の確認待ち（確認バッチが持つ））: 4.1 確認バッチで本人に聞く（3 つ）:

## 通した検証（`evidence.jsonl` の項目とコマンドごとの最新）

- 1.1 `scripts/quiet-run vulns-st -- tools/check-vulns.sh --self-test` → rc=0（PASS・head b548fa5）
- 1.2 `bash -c 'for s in "ラベルが無いと空の文字で答える gh でも、ラベルを作ってから issue を立てる" "ラベルがあれば、作り直さない" "開いた issue のコメントの問い合わせが空の文字なら、rc=2 で落ちる"; do grep -qF "== Scenario: $s" tools/check-vulns.sh || { echo "無い: $s"; exit 1; }; done'` → rc=0（PASS・head 756cd8b）
- 1.2 `scripts/quiet-run vulns-st -- tools/check-vulns.sh --self-test` → rc=0（PASS・head 756cd8b）
- 2.1 `bash -c 'for s in "知らせの段が落ちた夜は、決まった題の 1 件の issue が立つ" "知らせの issue はラベルに触れない" "知らせの issue の本文は、日付と run の 2 行だけ" "知らせの issue が開いていれば、立て直さずに追記する" "知らせの issue を立てられなければ、rc=2 で落ちる" "report-unfiled の引数が違えば、rc=2 で落ちる"; do grep -qF "== Scenario: $s" tools/check-vulns.sh || { echo "無い: $s"; exit 1; }; done'` → rc=0（PASS・head ea577a7）
- 2.1 `scripts/quiet-run vulns-st -- tools/check-vulns.sh --self-test` → rc=0（PASS・head ea577a7）
- 2.2 `tools/check-vulns-wiring.sh --self-test` → rc=0（PASS・head 8ebce8b）
- 2.2 `bash -c 'out=$(tools/check-vulns-wiring.sh) || exit 1; for s in "知らせの段は、材料を issue にする段の後にあり、run の URL を渡す" "知らせの段は、材料を issue にする段が落ちたときだけ走る" "材料が無い夜は、file-issues を呼ばない" "毎晩の検査は公開の流れから独立し、issue の権限は 1 つの job だけ"; do grep -qF "== Scenario: $s" <<<"$out" || { echo "無い: $s"; exit 1; }; done'` → rc=0（PASS・head 8ebce8b）
- 2.2 `python3 -c 'import re,subprocess as s;b=s.run(["git","merge-base","origin/main","HEAD"],capture_output=True,text=True,check=True).stdout.strip();o=s.run(["git","show",b+":.github/workflows/vulns-nightly.yml"],capture_output=True,text=True,check=True).stdout;n=open(".github/workflows/vulns-nightly.yml",encoding="utf-8").read();cut=lambda t:t[:t.index("\n  issues:\n")];assert cut(o)==cut(n),"issues の job より前（vulns・pg-minor・起動の条件・権限）が変わった";assert cut(n).count("exit \"$rc\"")==2'` → rc=0（PASS・head 8ebce8b）
- 2.3 `bash -c 'for s in "毎晩の検査が立てる issue は、持ち主が担当者になる" "持ち主の名前が無ければ、担当者を付けない" "開いた issue への追記では、担当者を足さない" "担当者を付けられなければ、担当者無しで立て直す"; do grep -qF "== Scenario: $s" tools/check-vulns.sh || { echo "無い: $s"; exit 1; }; done'` → rc=0（PASS・head ea577a7）
- 2.3 `scripts/quiet-run vulns-st -- tools/check-vulns.sh --self-test` → rc=0（PASS・head ea577a7）
- 3.1 `bash -c 'grep -E "^\| \`vulns-nightly.yml\`" docs/testing.md | grep -qF "report-unfiled"'` → rc=0（PASS・head 82d10e5）
- 3.2 `bash -c 'set -o pipefail; base=$(git merge-base origin/main HEAD) || exit 1; files=$( { git diff --name-only "$base" -- . && git ls-files -o --exclude-standard; } ) || exit 1; ! printf "%s\n" "$files" | grep -vE "^(tools/check-vulns-issues\.py|tools/check-vulns\.sh|tools/check-vulns-wiring\.sh|\.github/workflows/vulns-nightly\.yml|docs/testing\.md|docs/changes/CH23\.md|openspec/changes/ch23-vulns-nightly-issue-fix/|docs/briefs/CH23|\.superpowers/|$)"'` → rc=0（PASS・head 82d10e5）

## レビュー（`review/code.md`）

- 指摘 10 件（fixed 8・rejected 2）
- 処置の re-review: 2 回。最後の判定が NOT ADDRESSED のもの: なし

## 未処置の申し送り

なし
