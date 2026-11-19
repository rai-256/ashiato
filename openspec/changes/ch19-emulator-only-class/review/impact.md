# impact-review — ch19-emulator-only-class

seen: IMP-01 medium, IMP-02 low

## R1 — IMP-01 の「PASS の証跡が残りうる」が、証跡の組み方で変わることを書いていない
- 対象: impact.findings[IMP-01].text / derived / unknowns、dependencies[D2]
- 種類: 根拠
- 指摘: E2（scripts/evidence.py:419）は環境を引き継ぐことしか支えない。「絞られて PASS」になるかは、全体の項目の検証コマンドの形で決まる。過去の全体の項目は、`android-emulator.sh` の rc=0 に、結果の XML のクラス数を数える照合を重ねている（archive/2026-11-14-st57-update-notice/tasks.md:120 は `connected/*/TEST-*.xml` の `St57UpdateQueryTest` が 2 本以上、2026-10-17-st17 の tasks.md:29 も同じ型）。`ONLY` の経路は `androidTest-results` を消して stage1 の 1 クラスだけを残し、`connected/st57-update` も消えるので、こうした項目は黙って PASS にならず落ちる（`ONLY` が照合のクラスと同じ場合を除く）。一方、rc だけの項目（archive/2026-10-04-st28-private-network-only/tasks.md:176 の「検証: `tools/android-emulator.sh`」）は絞られても PASS になる。D2 の `who` と IMP-01 の `unknowns` は、どちらの型が実際に多いかを挙げていない。IMP-01 の危険は「rc だけの項目」に限られる。
- 直し方の案: IMP-01 の derived に「rc のみの検証は絞られても PASS・XML のクラス数を数える検証は落ちる」を書き、unknowns に「今後の全体の項目がどちらの型か」を足す。
- 処置: fixed: IMP-01 の題・derived を「rc だけを見る検証は絞られても PASS、XML のクラス数を数える検証は落ちる」に直し、unknowns に今後の項目の型を足し、ST28 の rc だけの項目を E7 として根拠に置いた

## R2 — 「`ONLY` の実行は前の全体の結果を消す」が、St56BackupTest の経路では起きない
- 対象: impact.lost[0]、targets[T2].change、rules[R3]、scenarios[S1]、overview の「できなくなることは 1 つ」
- 種類: 言い切り
- 指摘: spec は結果の置き場を消す規則を「ONLY にクラスを 1 つ渡すと…」の Requirement にだけ置く（spec.md:22-34）。バックアップの試験の Requirement（spec.md:64-72）には消す規則が無い。tasks.md:40 も消す行を「ふつうのクラスの段」に置き、tasks.md:71 は St56 の試験の前に消す作業を試験側で行う。つまり `ONLY=…St56BackupTest` では前の全体の XML と `connected/st57-update` は残る。ところが `lost` / T2 / R3 / S1 は `ONLY` 全般に消えると書いている。逆に残る側の帰結（`rm -rf "$OUT"/androidTest-results-stage*` は走って stage 側だけ消え、`androidTest-results` は古いまま残る）は書いていない。ST57 型の検証が `connected/` の古い写しを読む組み合わせは、impact に出ていない。
- 直し方の案: lost / T2 / R3 / S1 に「ふつうのクラスの経路のとき」の限定を付け、St56 の経路では古い結果が残ることと、それを読む側の扱いを別の行（D3 の読み手）で書く。
- 処置: fixed: lost・T2・R3・S1・overview に「ふつうのクラスの経路のとき」の限定を付け、St56 の経路では古い XML が残ることを lost に書き、それを読む側を D8（holds: unknown・確かめ方つき）として足した

## R3 — one_way_door が、まだ無い呼び手を「使う」と断定している
- 対象: impact.memory[1].text、dependencies[D7]
- 種類: 言い切り
- 指摘: memory[1] は「`ONLY` はハーネスの絞った検証（harness2 #271）が使うので」と現在形で書く。一方 D7 は同じ呼び手を「まだ作られていない・basis: unconfirmed」とし、coverage.not_examined も「まだ設計されていない」とする。docs/changes/CH19.md の「#271 が使う口になる」は依頼者の意図であって、呼び手の実物ではない（.github と tools・scripts を `ONLY` で探しても、呼ぶ側は無い）。名前を変えるときに一緒に直す相手は、現時点では存在しない。形 A の扉の説明が、形 B の未確認の依存を確定事項として書いている。
- 直し方の案: 「使う予定（未設計）。確認できた呼び手は無い」に直し、refs に D7 を足す。
- 処置: fixed: memory の one_way_door を「使う予定（未設計。確かめた呼び手は無い）」に直した

## R4 — IMP-02 の「必ず落ちる」は確かめていない断定で、evidence が空
- 対象: impact.dependencies[D6].affects、findings[IMP-02].title / text
- 種類: 言い切り
- 指摘: D6 は「gradle 1 回で走って必ず落ちる」と書く。St56 は実物で落ちると分かっている（design.md の Context）が、将来のクラスは存在しない。往復の手順が必要でも、前提が整っていれば通ることがありうる。IMP-02 の `evidence` は空で `basis: derived`。「理由を出さずに落ちる」の「落ちる」は仮定。ただし、落ちる側に寄るので low のままでよい（rc≠0 で見える）。指摘は重大度でなく言い切りの限定。
- 直し方の案: 「落ちうる（St56 では実物で落ちた）」に直す。
- 処置: fixed: D6 と IMP-02 の「必ず落ちる」を「落ちうる（St56 は実物で落ちた）」に直した。重大度は low のまま

## R5 — 場合選びの変数 `ONLY`（試験側）との衝突が T3 の searched に出るが、依存にも IMP にも載っていない
- 対象: impact.targets[T3].searched、dependencies
- 種類: 依存の見落とし
- 指摘: T3 は `tools/test-android-emulator.sh:477` の `ONLY=${ASHIATO_TEST_ONLY:-}` を見つけて「使っているのは場合選びの変数だけ」で終わる。bash では export 済みの変数に代入しても export は残るので、呼んだ人が `ONLY` を export していると、`test-android-emulator.sh` が台本（line 187 の `env … android-emulator.sh`）へ場合名（`stages` など）を漏らし、台本が形の誤りで rc=1 になる。対処は tasks.md の Global Constraints（`CASE` に改名・`env -u ONLY`）と spec-review R8 にあるが、go.json の `dependencies` には読み手 / 書き手として載らず、IMP-01 の「export された `ONLY`」と同じ根を持つ旨も書かれていない。
- 直し方の案: D8 として `tools/test-android-emulator.sh`（writes、`CASE` への改名で解消）を足し、IMP-01 の deps から参照する。
- 処置: fixed: D9 として `tools/test-android-emulator.sh` の場合選びの変数を足し（tasks の Global Constraints が CASE に改めるので holds: yes）、IMP-01 の deps に入れた
