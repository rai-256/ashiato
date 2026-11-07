# CH10 — エミュレータが起動できないとき android-emulator.sh が無期限に待たない

> 第 1 回の深掘りの答えを待たずに、推奨の答え（`deep.md` の `（答え待ち・推奨: …）`）で先に書いた（2026-10-20 の流れ）。
> 本人の答えが推奨と違えば、`spec --answers` がその問いだけを写す。`docs/changes/CH10.md` の「価値」「壊してはいけないもの」「完了の判定」も、この段で推奨の答えに合わせて書いた。

## Why

2026-11-05、ST54 の ci_fix で、Codex が sandbox の中から `tools/android-emulator.sh` を走らせた。sandbox では /dev/kvm を開けず、エミュレータは起動直後に落ちたが、
台本の `adb wait-for-device`（`tools/android-emulator.sh:35`）には上限が無く、来ない端末を約 20 分待ち続けた。人間が kill するまで止まらず、エミュレータの出力は捨てていて（`:34`）理由も残らなかった。
並走する Story と change が計測テストを検証に使う今、同じ止まり方が起きるたびに、別のセッションの時間と人間の手が取られる。

## What Changes

- **起こす前に /dev/kvm を開いてみる**。開けなければエミュレータを起こさず、`error: /dev/kvm を開けない（…）` と次の手（verify-run で走らせ直す・Codex なら sandbox の外を申請）を出して rc=1（design D11・D10・D14）
- **待っている間にエミュレータのプロセスが終わったら、すぐ止まる**。端末が adb に現れるのを待つ間と、起動の完了を待つ間の両方（依頼・D6）。出力は毎回 `collector-android/app/build/outputs/emulator.log` に残し、失敗したら末尾 40 行を出す（D12）。
  出力が加速の不足を言うときは /dev/kvm の語を含む行も出す（D17）
- **端末が adb に現れるまでの上限は 120 秒**（Q2 の推奨。D2（仮））。起動の完了の待ちは今の 180 回 × 2 秒のままで、1 回ごとの `adb shell` に 10 秒の上限を掛ける（D15）。上限と調べる /dev/kvm の場所は環境変数で上書きできる（D8）
- **同じ AVD のエミュレータが既に動いていれば、起こさず・その端末を使わず・止めずに理由つきで止まる**（Q3 の推奨。D3（仮））。**正常時の振る舞いが変わる唯一の所**: 今は `KEEP=1` の残りや別の worktree の同じ AVD の端末で計測テストが走っていたが、この後は先に止める手間が 1 回要る
- **失敗で止まるときは、自分の起こしたプロセスだけを止め、`emulator-5554` を名指しで止めない**（D13）。正常に進んだ後の後片付けは今のまま
- **起動と待ちを 1 つの部品（`tools/emulator-start.sh`）にし、利用状況の採寸の台本（`tools/usage-volume.sh`）もそれを使う**（Q1 の推奨。D1（仮））。採寸の台本の rc=2（測れなかった）・動いている端末を使う・`-wipe-data` の引数は変えない
- **正常に立つときの振る舞いを試験で固定する**（Q5 の推奨。D5（仮）・D16）。偽の SDK・偽の adb・偽の計測テストを置いた使い捨ての木で、今と同じ引数・6 段の結果・`KEEP=1` の扱いを見る。本物（kvm あり）でも 1 回通す
- 試験は `tools/test-android-emulator.sh`（新しいファイル）に置き、CI の検査の job で毎回走らせる（D9）。AGENTS.md の「/dev/kvm（Codex のとき）」に、quiet-run で直に走らせて /dev/kvm で落ちたときの手を 1 文足す（D14）
- **やらないこと**: 待つ相手・止める相手の取り違え（どれかの端末で先へ進む・正常時の後片付けが `emulator-5554` を止める）は直さない（Q4 の推奨。D4（仮））。
  st54 が足す `tools/android-side-by-side.sh` の同じ形の起動は直さない（st54 の merge の後に別の change で）。ハーネス側の verify-run の判定と理由の文言（`.harness2/scripts/evidence.py`）は変えない。CI の計測テスト（`android-emulator-runner`）はこの台本を使わないので変わらない

## Capabilities

### New Capabilities

- `emulator-startup`: 手元でエミュレータを起こす台本が、起こせないときに早く・理由つきで止まり、正常に立つときの振る舞いを変えないこと。CH03 の `test-hygiene`・CH06 の `verify-environment` と同じく、製品ではなく開発の道具の約束。
  schema の「tooling は `skip_specs`」は採らない: 止まるまでの時間・終了コード・文言・残す出力は外から確かめられる振る舞いで、偽の SDK の試験が Scenario の印に縛られ、archive の後も正典が固定する（CH09 の `test-timing` と同じ判断）。
  change の capability は `docs/stories/INDEX.md` に足さない

### Modified Capabilities

（無い。正典の spec にエミュレータの台本を扱うものは無い）

## Impact

- 道具: `tools/android-emulator.sh`（起動と待ちの行・`:9` の注記）・`tools/usage-volume.sh`（`emulator_sample` の起動と待ち）を書き換え、新しいファイル `tools/emulator-start.sh`（部品）・`tools/test-android-emulator.sh`（試験）を足す。
  `tools/wait-android-boot.sh` は変えない（呼び出し方 `<serial> <回数> <間隔>` と `ADB` は `tools/test-usage-volume.sh:73` が固定している）。`tools/test-usage-volume.sh` は変えない（通ることを確かめる）
- CI: `.github/workflows/ci.yml` の検査の job（`rust`）に `./tools/test-android-emulator.sh` を 1 行足す（`./tools/docker-tidy.sh --self-test` の後。本物の SDK も /dev/kvm も要らない）
- 文書: `AGENTS.md` の「/dev/kvm（Codex のとき）」に 1 文、`docs/testing.md:177` の行に止まり方を書き足す（行を直すだけ。末尾に足さない）
- 製品のコード・API・移行・画面: 変えない
- 依存: 足さない（`timeout`・`pkill` は coreutils / procps。CI の ubuntu と手元にある）
- **走っている Story・change との重なり**（`origin/main` = 11c0c043 との差分）:
  - st54（`feat/st54-dev-side-by-side`）: `tools/android-side-by-side.sh` を足し、その `:25-26` が事故と同じ形（出力を捨て、上限の無い `adb wait-for-device`）で起こす。**この change の範囲の外**（st54 の merge の後に、部品 `tools/emulator-start.sh` を使う別の change で直す。design の Non-Goals）。
    `ci.yml` は :96 の後に 9 行足す。この change は :124 の後で、重ならない
  - st55（`feat/st55-server-installer`）: `ci.yml` の :49・:111〜114・:383 以降。重ならない
  - st53（`feat/st53-license-bundle`）: `ci.yml` の :212 以降と `docs/testing.md` の 1 行。`tools/android-emulator.sh`・`tools/usage-volume.sh` には触らない。`docs/testing.md` は別の行なら rebase で解ける見込み
  - ch09（archive 前・`main` に入っている）: `tasks.md` が `tools/android-emulator.sh` を検証に使う（Task 10.1）。この change の後、同じ AVD の端末が `KEEP=1` で残っていると、その検証は理由つきで rc=1 になる（止めてから走らせ直す。D3（仮））。正常に立つときの振る舞いは変えない
  - どれも走っている Story へ差し戻さない。merge の順は決めない
