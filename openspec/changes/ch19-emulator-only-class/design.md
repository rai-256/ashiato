# CH19 design — ONLY の口の形

## Context

`tools/android-emulator.sh` は、エミュレータを起こした後に計測テストを 8 段で走らせる（1 段目: ふつうのクラス / 2〜3 段目: `St56BackupTest` の準備と確かめ（テストの外で `tools/android-backup-roundtrip.sh` が往復させる）/
4 段目: 権限を外した状態から始めるクラス / 5〜8 段目: 1 クラスずつの段 / 最後に `tools/android-st58-replace.sh`）。
gradle の `-Pandroid.testInstrumentationRunnerArguments.class` は 5〜8 段目がもう使っている（実物。本物のエミュレータで毎回通っている）。
段の結果の XML のクラス名は `classname="<完全修飾名>"` で出る（実物。台本の末尾の ST57 の写しがこの形で探している）。

`tools/test-usage-volume.sh` は台本の行の形を grep する（採寸の試験を `notClass` で除く行・**コメントでない最初の `connectedDebugAndroidTest` の行が `adb uninstall dev.ashiato.collector ` の行より後**）。
`tools/test-android-emulator.sh` の `assert_normal` は、行頭の `keep_stage` の行の数と段の置き場の数が等しいことを見る。

## Decisions

### D1. 全体の経路の行は変えず、ONLY の経路を前に足して `exit` する

ONLY の値の検査はエミュレータを起こす前（`emu_start` より前）に置く。ONLY の段は、全体と同じ準備（起動・アニメーション・写真と動画・アプリを外す・段の置き場の定義と古い段の置き場の削除）の**後**、全体の 1 段目の gradle の**前**に
`if [ -n "${ONLY:-}" ]; then … exit 0; fi` の塊として置く。塊の中の `keep_stage` は字下げする（行頭の `keep_stage` の数 = 全体の段の数、を保つ）。
全体の経路の既存の行は、字下げも含めて変えない。
理由: 全体の経路を変えないことを、行の差分が無いことと命令の列が同じことの 2 つで見られる。`test-usage-volume.sh` の「最初の gradle の行が uninstall より後」も、塊を uninstall の後に置けば保たれる。

### D2（仮）. `St56BackupTest` を渡したときは、全体のバックアップの段と同じ手順だけを走らせる

依頼の決定は「指定したクラスだけを 1 段で走らせる」で、例が `St56BackupTest`。このクラスは gradle の 1 回の計測テストでは通らない
（`prepareBackup` と `verifyRestored` の間に、テストのプロセスの外でデータを消してバックアップから戻す必要がある。gradle は間で APK を入れ直し、戻したデータを消す）。
そこで、このクラスに限り、全体の 2〜3 段目と同じ命令（`installDebug installDebugAndroidTest` → `bmgr enable` → `am instrument …#prepareBackup` → 往復 → `am instrument …#verifyRestored` と `OK (1 test)` の照合）を同じ順で走らせる。
**この経路では `keep_stage` を呼ばない**（段の置き場を作らない）。`am instrument` は XML を書かず、`keep_stage` の `find "$OUT/androidTest-results"` は置き場が無ければ `set -e` で台本ごと落ち、在れば前の実行の XML を今回の段として写す（spec-review R1）。判定は `OK (1 test)` の照合だけ。
全体の経路の行を関数に括り出すことはしない（D1。行頭の `keep_stage` の数が変わる）。ONLY の経路の中の重複は受け入れる。

- 反転条件: バックアップの試験が gradle の 1 回の計測テストで通る形に直った（往復をテストの中に持てた）とき、または特別扱いのクラスが 2 つ以上に増えて重複が保守を妨げるとき → 特別扱いを外す / 段を関数にまとめる（その時は全体の経路の行も変わるので、別の change で）
- 失わない側: どちらを選んでもデータも記録も失わない（台本の口の形だけ）。依頼の「1 段」を「そのクラスが全体で走る手順」と読む。依頼の目的（ST56 Task 5 の 1 クラスの繰り返し）を満たすのはこちらだけ

### D3（仮）. ONLY の段の前に前の実行の結果を消し、段の後に渡したクラスの結果が XML にあることを確かめる

gradle の計測テストは、存在しないクラスや 0 本でも終わり方が分かりにくい（「名前で絞った実行が 0 本でも rc=0」の型）。段の前に `collector-android/app/build/outputs/androidTest-results` を消し、
段の後に `androidTest-results-stage1` の XML に `classname="<ONLY>"` が 1 つも無ければ、`error: ONLY=<クラス> のテストが 1 本も走っていない` を標準エラーに出して rc=1 で止める（文言と rc は spec の Scenario に写した）。
前の実行の結果を消すので、全体の実行が残した `connected/st57-update` の写しも消える（古い写しを今回の結果として読ませない。全体を走らせ直せば作り直される）。
`St56BackupTest`（D2）は全体と同じく `OK (1 test)` の照合で見る（XML の確かめはしない）。

- 反転条件: gradle の計測テストが 0 本のとき確実に rc≠0 になると実物で確かめられた → XML の確かめを外してよい

### D4（仮）. 専用の台本で走るクラスは、起こさずに rc=1 で止めて台本の名前を出す

`St58PackageReplacedInstrumentedTest`（入れ替えの台本が APK を 2 つ組んで入れ替える。依頼が対象外とした）と `UsageVolumeInstrumentedTest`（採寸の台本が利用状況の許可とイベントを用意する）は、
gradle の 1 段では前提が無く必ず落ちる。起こしてから落ちるより先に止める方が速い。文言: `error: ONLY=<クラス> は <台本> で走らせる`、rc=1（同じ台本の既存の設定の誤り —— emulator が無い・上限の値が正の整数でない —— と同じ。rc=2 は採寸の台本の「測れなかった」に使われているので使わない。spec-review R6）。

- 反転条件: どちらかの台本が `ONLY` の口から呼べるようになった（別の change）

### D5（仮）. ONLY の値は完全修飾のクラス名 1 つだけ

形: `^[A-Za-z_][A-Za-z0-9_]*(\.[A-Za-z_][A-Za-z0-9_]*)+$`（パッケージを持つ・`#` のメソッドも `,` の複数も空白も無い）。外れたら起こさずに `error: ONLY=<値> は完全修飾のクラス名 1 つの形でない（例: dev.ashiato.collector.St56BackupTest）` で rc=1（D4 と同じ）。
空（`ONLY=`）は渡していないのと同じ（全体）。
理由: 依頼の決定は「指定したクラス」。メソッドや複数を許すと D2・D3 の確かめ（クラス名の照合）が曖昧になる。

- 反転条件: 絞った検証（harness2 #271）がメソッド単位か複数のクラスを要ると分かった → 形を広げる（広げても既存の呼び方は壊れない）

### D6. AGENTS.md の書き方

「長いコマンド」の節（`<!-- harness2:long-commands -->` の印は残す）の既存の項目の後に 1 項目を足す:
直している途中は落ちたところだけに絞って走らせ（エミュレータ: `ONLY=dev.ashiato.collector.<クラス> scripts/quiet-run emu -- tools/android-emulator.sh`、`cargo test`: `cargo test -p <crate> <名前>` に `-- --list` で 1 本以上を確かめる）、
通ったら最後に全体を `scripts/verify-run <項目>` で走らせる。tasks.md の検証コマンドは全体のまま書き換えない（絞った実行は Task の完了の証跡にしない）。
