# CH10 — エミュレータが起動できないとき android-emulator.sh が無期限に待たない

## 人間に返す未決（A）

なし。

**人間の手が要るもの 1 つ**: 実装中の本物の検証が残したエミュレータが 2 本、CPU を使い続けている（code-verify R15）。
PID 1811997（14:50 開始・CPU 約 100%）と PID 2718742（17:25 開始・`-port 5668`）で、どちらも HOME がこの worktree の消えた一時木。
今の `real-proof.sh` は残りを出さない（HEAD で走らせ直して確かめた）ので、残っているのは直す前の版の残骸。finish の席から止めようとしたが権限で拒まれた。
`ps -o pid,lstart,args -p 1811997,2718742` で確かめてから `kill 1811997 2718742` で止めてほしい。

## 何をしたか

2026-11-05、ST54 の ci_fix で `tools/android-emulator.sh` が、/dev/kvm を開けずに落ちたエミュレータを `adb wait-for-device` で約 20 分待ち続けた。
起動と待ちを共有の部品 `tools/emulator-start.sh` にまとめ、計測テストの台本と採寸の台本（`tools/usage-volume.sh`）の両方で使う。この部品で次のように止まる:

- 起こす前に /dev/kvm を読み書きで開いてみて、開けなければ `error: /dev/kvm を開けない` で止まる（起こさない）
- 起動の途中でプロセスが終わったら、すぐ止まる。エミュレータの出力は `collector-android/app/build/outputs/emulator.log` に残し、末尾 40 行を出す
- 端末が adb に現れるのは最大 120 秒まで待つ。起動の完了を問い合わせる `adb shell` は、1 回ごとに 10 秒で打ち切る
- 同じ AVD のエミュレータが既に動いていたら、計測テストの台本は起こさずに止まる
- 失敗で止まるときは、自分の起こしたプロセスだけを止める

正常に立つときの振る舞いは変えていない（6 段の計測テスト・引数・`KEEP=1`）。
試験は `tools/test-android-emulator.sh`（偽の SDK で 23 Scenario）。CI の検査の job で毎回走る。

## 仮決め（B）と Ruling

- D1（仮）〜D5（仮）: 深掘りの Q1〜Q5。本人の答えはどれも推奨と同じだった（Q4「取り違えは直さない」も含む）。反転条件は design.md の各 D に書いてある
- D4（仮）に final R6 で 1 項足した: 他の端末（実機など）が繋がっていると、端末の待ちは上限まで待ってから「端末が adb に現れない」で落ちる。偽の adb はこの振る舞いを再現しない
- Task の ledger（`progress.md`）は作られておらず、`Ruling:` の行は 0 件。park した判断は次の 2 件で、review/code.md に処置として書いてある:
  - final R5（rejected）: 同じ AVD の確かめの待ちが D2 の上限の外に足される。ただし端末 1 台ごとに有界なので直さない
  - final R10（rejected）: 試験の判定の分け方・数え方・self-test の網羅の残り。どれも Scenario の振る舞いを外していないので直さない

## Task

6 Task・11 項目がすべて `[x]`。残した Task は無い。

## 検証（evidence.jsonl の最後の PASS。どれも exit_code 0）

| 項目 | コマンド | head |
|---|---|---|
| 1.1 / 2.2 / 3.1 / 4.1 | `tools/test-android-emulator.sh` | 0ed4fb180 ほか |
| 1.2 / 6.2 | `tools/test-android-emulator.sh --self-test` | 86b5bcb75 |
| 2.1 | `bash -n tools/emulator-start.sh` / `bash -n tools/android-emulator.sh` | 3f95fd47f |
| 2.3 | `ASHIATO_TEST_ONLY=dies …`（直す前の台本では見張りで打ち切られ、直した台本では通る） | 3f95fd47f |
| 4.1 | `tools/test-usage-volume.sh` | dfd1bbc65 |
| 5.1 | `./openspec/changes/ch10-emulator-start-timeout/check-5.1.sh` | 658d93e53 |
| 5.2 | ci.yml の grep / `tools/check-shared-appends.sh` | d813459e2 |
| 6.1 | `real-proof.sh dies / timeout / same-avd / ok`（本物のエミュレータ・kvm あり） | 86b5bcb75（code-verify が 9df54ee09 で走らせ直し、4 つとも rc=0） |
| 6.2 | `python3 scripts/check_scenarios.py . ch10-emulator-start-timeout` | 86b5bcb75 |

finish の直し（15ae74ae7）の後に走らせたもの:
- `scripts/quiet-run unit -- ./tools/test-android-emulator.sh`: rc=0
- `--self-test`: rc=0
- `tools/test-usage-volume.sh`: rc=0
- `check-5.1.sh`: rc=0

新しく足した判定は、写しの木で台本を 7 通りに壊すと、どれでも rc=1 で落ちる。

## レビュー

- final review: R1〜R10。fixed 7・rejected 3（R5・R9・R10）。scoped re-review で ADDRESSED
- code-verify: 申告どおりの rc を確かめた。Scenario の主張を観測していない試験が 4 件あった（R11〜R14）ので、4 件とも直した。R15 は上の「人間の手が要るもの」
- security-review: 指摘なし
- `/code-review`: 10 件
  - 直したもの（4 件）:
    - R18: 止めた後に `EMU_PID` を空にする
    - R19: CR を落とす
    - R24: アニメーションの設定 3 行をループにした
    - R25: 余計な書き直しを消した
  - 退けたもの（6 件）:
    - R16・R17: Q4 で「直さない」と決めた範囲。直す前の台本と同じ振る舞い
    - R20: 直す前の台本と同じ形
    - R21: spec の定めのとおり。直す前は上限が無かった
    - R22: Global Constraints と Q5 が変更を禁じている
    - R23: D3 で採寸の台本には当てないと決めた
- finish の直しの scoped re-review: 8 件とも ADDRESSED。新しい破損は無い

## 申し送り（未処置）

- `tools/wait-android-boot.sh` は使い手が無くなった（採寸の台本は部品の `emu_wait_boot` を使う）。この change では Q5 が変えることを禁じているので残した。消すなら別の change で、`tools/test-usage-volume.sh` の検査も一緒に外す
- st54 の `tools/android-side-by-side.sh` が、同じ形で無期限に待つ。st54 の merge の後に別の change で直す
- ハーネス: plan_fix.py が、tasks.md 5.1 の旧コマンドの前半だけを置き換えた（final R9）。直すのは harness2 側

🤖 Generated with [Claude Code](https://claude.com/claude-code)
