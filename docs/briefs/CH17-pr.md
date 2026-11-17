# CH17 —— Windows のインストールの試験の 1 周を縮める（台本だけの変更ではビルドし直さない）

change: `openspec/changes/ch17-windows-install-ci-reuse/`

## deep.md に積んだ未決（A）

なし

## 仮決め（B）と Ruling

design の（仮）:
- D1（仮）. 組み立ての鍵は「名指しして除くもの以外の全部」から作る
- D2（仮）. 保存物は actions/cache の完全一致で引き、PR の run でだけ使う
- D4（仮）. 使い回したときは `ch14_` の試験も飛ばす
- D5（仮）. Rust のビルドのキャッシュ

全 Task の ledger に `Ruling:` の行は無い。

## Task

3 Task のうち、埋めた Task は 3 本。
残した項目は無い。

## 通した検証（`evidence.jsonl` の項目とコマンドごとの最新）

- 1.1 `tools/release/windows-build-key.sh --self-test` → rc=0（PASS・head 6e7afd5）
- 1.1 `bash -c 'out=$(tools/release/windows-build-key.sh --self-test); for s in "試験の台本と文書だけが違う 2 つの木は、同じ組み立ての鍵になる" "組み立てに入るファイルが 1 つでも違えば、鍵が変わる" "名指ししていないファイルが増えれば、鍵が変わる" "道具の版が違えば、鍵が変わる" "鍵を出せない木では、鍵を出さずに落ちる"; do grep -qF "== Scenario: $s" <<<"$out" || exit 1; done'` → rc=0（PASS・head 6e7afd5）
- 1.1 `bash -c 'k=$(tools/release/windows-build-key.sh HEAD); [[ "$k" =~ ^[0-9a-f]{64}$ ]]'` → rc=0（PASS・head 6e7afd5）
- 1.1 `bash -c 's="$PWD/tools/release/windows-build-key.sh"; d=$(mktemp -d); cd "$d" && ! out=$(GIT_CEILING_DIRECTORIES="$(dirname "$d")" "$s" HEAD 2>/dev/null) && [ -z "$out" ]; rc=$?; cd / && rm -rf "$d"; exit $rc'` → rc=0（PASS・head 6e7afd5）
- 2.2 `tools/release/windows-build-key.sh --check-workflow` → rc=0（PASS・head 6e7afd5）
- 2.2 `bash -c 'out=$(tools/release/windows-build-key.sh --check-workflow); for s in "前の組み立ては、鍵が空でなく完全に一致した PR の run でだけ引く" "引けたときだけ組み立てを飛ばし、今と同じ名前で上げる" "上げる配布物は全部、保存と引きの置き場に入る" "使い回した Windows の配布物は、組んだ commit のソースの場所と印のまま上げる" "組み立てた配布物は、鍵が空でなく組み立てが成功したときだけ保存する" "鍵を出す手順は、道具の版と走っている CI の定義を鍵に入れる" "ビルドするときは、Rust の依存のビルドを戻す" "使い回した run でも、インストールと更新の Scenario が 1 つ欠ければ CI が落ちる"; do grep -qF "== Scenario: $s" <<<"$out" || exit 1; done'` → rc=0（PASS・head 6e7afd5）
- 2.2 `bash -c 'out=$(tools/release/windows-build-key.sh --self-test); ! grep -q "飛ばした" <<<"$out"'` → rc=0（PASS・head 6e7afd5）
- 2.3 `tools/release/check-guards.sh workflows` → rc=0（PASS・head 6e7afd5）
- 2.3 `tools/release/check-guards.sh --self-test` → rc=0（PASS・head 6e7afd5）
- 2.3 `tools/check-ci-ch14-step.sh` → rc=0（PASS・head 6e7afd5）
- 2.3 `python3 -c 'import subprocess as sp, yaml; base=sp.run(["git","merge-base","HEAD","origin/main"],capture_output=True,text=True,check=True).stdout.strip(); o=yaml.safe_load(sp.run(["git","show",base+":.github/workflows/ci.yml"],capture_output=True,text=True,check=True).stdout); n=yaml.safe_load(open(".github/workflows/ci.yml",encoding="utf-8")); oj=o.pop("jobs"); nj=n.pop("jobs"); assert o==n and oj.keys()==nj.keys(); assert all(oj[k]==nj[k] for k in oj if k!="package-windows"); a=dict(oj["package-windows"]); b=dict(nj["package-windows"]); a.pop("steps"); b.pop("steps"); assert a==b'` → rc=1（FAIL・head c5d1f7f）
- 2.3 `bash -c 'printf "%s\n" tools/release/install-check.sh | tools/release/changes.sh code && ! printf "%s\n" docs/x.md | tools/release/changes.sh code'` → rc=0（PASS・head 6e7afd5）
- 2.3 `./openspec/changes/ch17-windows-install-ci-reuse/check-ci-scope.sh` → rc=0（PASS・head 6e7afd5）
- 3.1 `bash -c 'grep -q "windows-build-key.sh" docs/testing.md && grep -q "release-package" docs/testing.md'` → rc=0（PASS・head 6e7afd5）
- 3.2 `bash -c 'sed -n "/^### 11.1/,/^### 11.2/p" docs/release.md | grep -q "使い回した"'` → rc=0（PASS・head 6aff1c4）

## レビュー（`review/code.md`）

- 指摘 26 件（escalated 2・fixed 22・rejected 2）
- 処置の re-review: 3 回。最後の判定が NOT ADDRESSED のもの: なし

## 未処置の申し送り

なし
