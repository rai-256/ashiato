# Task 1 report

## 実装

- `tools/release/soak-install.sh` を追加した。
- サービスの状態・PID と Service Control Manager の 7031/7034、healthz、4 種類のログ、更新確認 JSONL、PC 収集アプリの PID を置く段の時刻窓で判定する。
- 失敗は `check-ng-observations.sh` の `die` / `die_seen` 経由で観測を出し、通常の観測行も伏せ字を通す。問い合わせの上限だけは `::warning::` とする。
- `--self-test`、`--list`、内部判定口を追加し、20 個以上の Scenario 印を自己試験に置いた。

## TDD Evidence

### RED

`tools/release/soak-install.sh --self-test` を実行し、初回は自己試験の実行時に未設定変数参照で終了した。その後、自己試験が実際に失敗入力を失敗として捕まえられるよう、判定をサブシェルで実行する形へ修正した。

### GREEN

`tools/release/soak-install.sh --self-test` は `OK: soak-install self-test` で終了した。自己試験には、起こし直し、停止、healthz、イベント記録の窓、ログ閾値、panic、更新確認理由、collector PID、観測の伏せ字の検査を含めた。

## 検証

- `scripts/verify-run 1.1`: 3 検証すべて PASS、rc=0
- `bash -n tools/release/soak-install.sh`: PASS
- `git diff --check`: PASS

ハーネスの一時置き場には `check-ng-observations.sh` が作る `.ng-secrets` だけが残ったが、自己試験が作る評価用ディレクトリは掃除されている。

## Files changed

- `tools/release/soak-install.sh`
- `openspec/changes/ch22-soak-install-online-ci/evidence.jsonl`
- `.superpowers/sdd/ch22-task-1/task-1-report.md`

## Self-review

Task 1 の実地 Windows 起動手順は brief の指定どおり Task 2 の範囲として接続していない。判定入力は自己試験で使う明示的なファイル形式に分離しており、実地口から時刻を注入する環境変数は追加していない。

## Fix round 1 report

- F1/F2/F3/F6: 自己試験を spec の 20 Scenario に限定し、`--list` は実地 Scenario の固定一覧を出す形にし、失敗理由・サービス名・システム記録件数・警告内容を確認するようにした。
- F4: JSON の `CheckReason` と同じ snake_case の `rate_limited` を判定し、上限だけの警告と混在時の失敗を検査するようにした。
- F5/F8/F9/F10: ISO/DB 日時と DB PID の除去、置く間の窓、システム記録件数の集約、誤り行の重複抑制を修正した。
- F7/F11: 起こし直し前の collector 記録を無視し、起こし直し後の PID 変化を 0 回だけ許可する自己試験にした。
- 秘密の値の伏せ字と観測口の静的検査も自己試験で実体を確認するようにした。自己試験用の `.ng-secrets` は明示的に掃除する。

### Verification

- RED: review findings が示した既存の自己試験は、実質 17 本でダミー印を含み、`RateLimited` を使っていたため `rate_limited` の誤判定を検出できなかった。
- GREEN: `scripts/verify-run 1.1` — 3/3 PASS、各 rc=0。`tools/release/soak-install.sh --self-test` は `OK: soak-install self-test`。
- 追加確認: `bash -n tools/release/soak-install.sh`、`git diff --check`、`--list` 20 行。

## Fix round 2 report

- F5/F15: DB ログの `YYYY-MM-DD HH:MM:SS.mmm UTC` を置く間の ISO 窓へ正規化し、UTC と DB PID をキーから除外した。読めない日時の行は、ログ読込前に記録した `.soak-log-baselines/<basename>.lines` より後の行だけを数えるようにした。
- F7: 自己試験を、サービス PID だけが変わる入力、7031/7034 の記録だけがある入力、collector PID が変わる入力と消える入力に分離した。DB の実際の日時形式と、基準後に追加された同一の日時不明行 3/2 行も試験する。

### Verification

- GREEN: `tools/release/soak-install.sh --self-test` — `OK: soak-install self-test`、rc=0。
- GREEN: `scripts/verify-run 1.1` — 3/3 検証 PASS、各 rc=0。`check-ng-observations.sh` と Scenario/LIMIT 検査も PASS。
- 追加確認: `git diff --check` — PASS。
