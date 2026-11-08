# Task 10 レポート

## 実装内容

- `ci.yml` の `changes` に `deps` 出力と比較不能時の `all()` 出力を追加。
- PR の依存変更時だけ走る `vulns` job を追加し、base commit の取得、base 比較、例外表由来の許可一覧、SHA 固定の dependency-review を配線。
- `rust` job に脆弱性配線・PostgreSQL minor の検査を追加し、`android` job の detekt 後に Gradle lock 検査を追加。
- `release.yml` に脆弱性検査 job を追加し、Linux/Windows の組み立てがそれを待つよう変更。
- `tools/check-vulns-wiring.sh` を追加。標準 Python の字下げベース解析で CI/release/nightly の配線を検査し、5 本の Scenario 印を出す。`--self-test` は条件・入力・権限・コメント設定の欠落を検出する。

## テスト結果

- `tools/check-vulns-wiring.sh`: PASS
- `tools/check-vulns-wiring.sh --self-test`: PASS
- `tools/check-vulns.sh --self-test`: PASS
- `bash -c 'grep -q "changes.sh deps" .github/workflows/ci.yml'`: PASS（直接実行）
- `bash -c 'grep -q "tools/check-vulns.sh" .github/workflows/release.yml'`: PASS（直接実行）
- `git diff --check`: PASS
- `scripts/verify-run 10.2`: PASS
- `scripts/verify-run 10.3` の self-test: PASS
- `scripts/verify-run 10.1` と通常の 10.3: `BLOCKED_INFRA`。検証準備の `tools/verify-env.sh` が rc=1 で終了し、コマンド本体は未実行。直接実行は PASS。

## TDD Evidence

### RED

`tools/check-vulns-wiring.sh --self-test` を実装前に実行し、`No such file or directory` / rc=127 を確認した。対象の配線検査が存在しないための期待された失敗。

### GREEN

実装後に `tools/check-vulns-wiring.sh --self-test` を実行し、5 つの Scenario 印と `OK: 脆弱性 CI 配線の自己テストに成功` を確認した。通常検査も 5 つの印を出して rc=0 で終了した。

## 変更ファイル

- `.github/workflows/ci.yml`
- `.github/workflows/release.yml`
- `tools/check-vulns-wiring.sh`
- `openspec/changes/ch11-dependency-vulnerability-scan/evidence.jsonl`（`verify-run` の証跡）
- `.superpowers/sdd/ch11-task-10/task-10-report.md`

## Self-review

- PR の workflow 変更だけでは `deps` が真にならず、脆弱性 job が走らない条件を維持している。
- `nightly.yml` は変更しておらず、開発版の組み立て・公開は脆弱性 job を待たない。
- `issues: write` の nightly job や dependency-review の PR コメント設定を誤って追加していないことを self-test で確認した。

## 懸念

- `scripts/check_scenarios.py . ch11-dependency-vulnerability-scan` は、Task 9 が担保する「設定を API で読み返す」の印が既存木に無いため 1 件 FAIL した。この Task の対象外であり、Task 10 の 5 本の印は追加済み。
- 10.1 と通常の 10.3 は `verify-run` の準備失敗で `BLOCKED_INFRA` の証跡が残っている。直接コマンドは成功しているため、証跡の状態は置き換えていない。

## Fix report: review 1 F1/F2

- F1 を修正し、`.github/workflows/ci.yml` の base 取得手順は base commit の取得・存在確認だけにした。base 側の `Cargo.lock`、`web/package-lock.json`、`osv-scanner.toml` の存在は決め打ちせず、`tools/check-vulns.sh --base` に委ねる。
- 配線検査もこの契約に合わせ、`tools/check-vulns-wiring.sh` は個別ファイルの `git show` ではなく base commit の確認を検査する。
- F2 を修正し、`allow-ghsas` の出力を変数へ代入してから `GITHUB_OUTPUT` に書くことで、コマンド置換の失敗を隠さないようにした。
- F3〜F7 は今回の open findings ではないため変更していない。

### Fix verification

- `tools/check-vulns-wiring.sh --self-test`: PASS
- `tools/check-vulns-wiring.sh`: PASS（5 Scenario 印）
- `scripts/verify-run 10.1`: PASS
- `scripts/verify-run 10.2`: PASS
- `scripts/verify-run 10.3`: PASS（通常検査・self-test）
- `scripts/quiet-run task10-suite -- bash -c 'tools/check-vulns.sh --self-test && tools/check-vulns-wiring.sh --self-test && tools/check-vulns-wiring.sh'`: PASS、rc=0
- `bash -n tools/check-vulns-wiring.sh`, `bash -n tools/check-vulns.sh`, `git diff --check`: PASS
