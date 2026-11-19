# CH19 spec レビュー（ch19-emulator-only-class・軽い路）

## 機械の検査（先に写す）

- `openspec validate ch19-emulator-only-class --strict` → `Change 'ch19-emulator-only-class' is valid`（rc=0）
- `python3 scripts/check_chain.py .` → `chain: OK (0 件 / 未回収 0 件 / warn 0 件)`（rc=0）
- `python3 scripts/check_scenarios.py . ch19-emulator-only-class` → `scenarios: FAIL (担保なし 9 件 / 名無しの確認待ち 0 件)`（rc=1）。
  FAIL の 9 件は全部この change の Scenario で、試験（`tools/test-android-emulator.sh` の `only-*`）は実装の前なのでまだ無い。spec の段では想定どおりで、これ自体は指摘にしない。
  9 本の名前と tasks.md の `echo 'Scenario: …'` の 9 本は機械で突き合わせて一字一句同じ（sort して diff。違いは雛形の `<名前>` だけ）
- `python3 scripts/review_triage.py . ch19-emulator-only-class` → `triage: OK`（指摘 0 件 / 仮決め 0 件 / 要件へ戻すもの 0 件）
- `python3 scripts/spec_overlap.py candidates . ch19-emulator-only-class` → `[]`（0 件）
- 軽い路の規則: delta は `## ADDED Requirements` だけ（spec:1）。MODIFIED / REMOVED / RENAMED は無い。どの R の提案も、`lane.json` の予定のパス（`tools/android-emulator.sh`・`tools/test-android-emulator.sh`・`AGENTS.md`・`docs/changes/CH19.md`）とこの change の中だけで閉じる。R12 だけは正典を直す提案なので、持ち越し（defer）とした。A（失う側がある問い）は見つからなかったので、escalated は無い
- 補足: `python3 scripts/provisional.py check` は rc=0（台帳は空）。design の D2〜D5 は（仮）だが台帳に載っていない。
  軽い路の前例 CH16（`archive/2026-11-18-ch16-install-check-ng-observations`。D1〜D7 に（仮）があり `provisional.jsonl` は無い）と同じ扱いなので、指摘にはしない

---

## R1. ONLY=St56BackupTest の経路の `keep_stage` は、本物の gradle では結果の置き場が無いときに `set -e` で落ちる。置き場があれば前の実行の XML を今回の段として写す。偽の gradle ではどちらも見えない
- 成果物: openspec/changes/ch19-emulator-only-class/design.md（D2・D3）、tasks.md（3.1・4.1）、specs/emulator-startup/spec.md（バックアップの試験の Requirement）
- 根拠:
  - design.md:27 D2 は「段の置き場は全体と同じ 2 つ（字下げした `keep_stage` 2 回）」。tools/android-emulator.sh:96-100 の `keep_stage` は `find "$OUT/androidTest-results" … -exec cp` を走らせる。台本は `set -euo pipefail`（:14）
  - 全体の経路では、1 段目の `connectedDebugAndroidTest`（:103）が `androidTest-results` を作るので `find` が通る。ONLY=St56 の経路では `connectedDebugAndroidTest` を呼ばない（spec:52）。`:app:installDebug :app:installDebugAndroidTest` は `androidTest-results` を作らないはず（connected の試験の task の出力の置き場なので。ここでは本物の gradle で測っていない）。作らないなら下のとおり落ち、作るとしても次の項の古い XML の写しは起きる
  - 実測: `bash -c 'set -e; f(){ find /nonexistent-xyz -name "*.xml" -exec cp {} /tmp \; ; echo after; }; f; echo reached'` → `find: '/nonexistent-xyz': No such file or directory`、rc=1（`after` も `reached` も出ない）。clean した build の置き場では、`prepareBackup` の直後に止まる
  - D3（design.md:35）は ONLY の段の前に `androidTest-results` を消す。tasks 2.1 はこれを「ふつうのクラスの段」に置き、3.1 は St56 の経路で消すかどうかを書いていない。消せば上のとおり落ちる。消さなければ、前の実行の XML（例: 直前の `ONLY=MediaPremises…` の結果）が `androidTest-results-stage1`・`stage2` に「今回の段」として写る。これは D3 の「古い結果を今回の結果として読ませない」に反する
  - 偽の gradle（tools/test-android-emulator.sh:86-92）は、`installDebug` で呼ばれても `androidTest-results/TEST-fake-_app-.xml` を書く。このため `only-st56` は偽の SDK では必ず通る
  - tasks 4.1 は `MediaPremisesInstrumentedTest` の後に `St56BackupTest` を走らせる。前の実行が置き場を作るので、本物のエミュレータでもこの不具合は順番の運で隠れる
- kind: technical
- 提案: St56 の経路では `keep_stage` を呼ばない（`am instrument` は XML を書かず、判定は `OK (1 test)` の照合だけ。D3 も「XML の確かめはしない」と言っている）。そう決めたら D2 の「段の置き場 2 つ」を消し、spec に「St56 を渡したときは段の結果の置き場を作らない（または前の実行の XML を写さない）」を 1 本足す。
  試験は、`androidTest-results` の無い木で `only-st56` が rc=0 になることを見る。そのため、偽の gradle は `connectedDebugAndroidTest` のときだけ XML を書くようにする。これで依頼の「1 段」と D2 の「2 段」の字面の食い違いも消える
- 処置: fixed D2 仮 — St56 の経路では keep_stage を呼ばず段の置き場を作らないと D2 に書き、spec の Requirement と Scenario（段の置き場が 1 つも無い・androidTest-results の無い木で）に写した。偽の gradle は connectedDebugAndroidTest のときだけ XML を書く（tasks 1.1）。4.1 は置き場を消してから St56 を先に走らせる

## R2. gradle と adb の間の順番が、偽の SDK では観測できない（Scenario「ONLY の段の前に…」と、Requirement 1 の「同じ順に」）
- 成果物: openspec/changes/ch19-emulator-only-class/specs/emulator-startup/spec.md、tasks.md（1.1・2.2）
- 根拠:
  - spec:35 の THEN は「…`shell pm reset-permissions` がこの順で、**偽の gradle の計測テストより前に**届いている」。spec:6 の Requirement 1 は「同じ命令を同じ順に送り」
  - 偽の gradle の記録は `FAKE_LOG`（`run.log`。tools/test-android-emulator.sh:91・:181）、偽の adb は `FAKE_ADB_LOG`（`adb.log`。:115）で、別々のファイルに書いている。tasks 1.1 も gradle を `FAKE_LOG` に残す形のままで、比べ方は「`FAKE_LOG` の gradle の行と、`FAKE_ADB_LOG` から…を除いたもの」（tasks.md:30）。2 つのファイルをまたいだ前後は比べていない
  - このため、全体の経路で `adb shell pm reset-permissions` を gradle の段の前後へ動かしても `only-unset` は緑のまま。`only-order` の「gradle より前」は確かめる手段が無い
  - 除く命令の一覧も spec:14（`getprop`・`devices`・`get-state`・`wait-for-device`）と tasks.md:30（さらに `emu avd name`）で違う
- kind: technical
- 提案: 偽の gradle と偽の adb（と `st58-replace`・`media-fixtures`）が、同じ 1 本の記録（例: `FAKE_SEQ_LOG`）に 1 行ずつ足すようにし、命令の列はその 1 本で比べることを tasks 1.1 に書く。spec と tasks の除く一覧は同じにする
- 処置: fixed 1.1 — 偽の gradle・adb・写真と動画・入れ替えの台本が同じ 1 本の記録（seq.log）に足し、命令の列と順はその 1 本で比べる。除く一覧は spec と tasks で同じ 5 つにそろえた。1.2 の self-test に「行を gradle の後ろへ動かすと落ちる」を足した

## R3. 「この change の前の台本と同じ命令」の比べは渡したときだけ走り、CI ではいつも素通りする
- 成果物: openspec/changes/ch19-emulator-only-class/specs/emulator-startup/spec.md（Scenario「ONLY を渡さないか空なら…」）、tasks.md 1.1
- 根拠:
  - Requirement 1 の主張は「この change の前の台本と同じ命令を同じ順に」（spec:6）。Scenario のうち、それを確かめるのは spec:15 の「`ASHIATO_TEST_BASE_REV` を渡したときは」の AND だけ。既定の実行では、同じ台本の `ONLY` 無しと `ONLY=` を比べるだけになる。全体の経路を変えてもこの比べは通るので、前の版と同じことの証拠にならない
  - CI は `./tools/test-android-emulator.sh` と `--self-test` を BASE_REV なしで走らせる（.github/workflows/ci.yml:183-184）。merge の後は `git merge-base origin/main HEAD` が HEAD そのものなので、比べても必ず同じになる
  - 1 つの Scenario に、rc=0・段の数・入れ替え 1 回・空と未設定が同じ・前の版と同じ、の 5 つの主張が束ねてある
- kind: technical
- 提案: Scenario を 2 本に分ける。(a)「空と未設定は同じ命令の列」は常に走る。(b)「前の版と同じ命令の列」は、PR の証跡（tasks 1.1・3.1 の BASE_REV つきの検証）で確かめるものだと Scenario に書く。
  または、試験が `origin/main` を解決できるときは BASE_REV を既定で merge-base にして、CI でも PR の間は比べるようにする
- 処置: fixed 1.1 — Scenario を 2 本に分けた（空と未設定の比べは常に走る `only-empty`／前の版との比べ `only-base` は、元の版を明示して渡したときだけ比べ、渡されなければ飛ばしたことを出す（merge の後に段を足す Story で落ちないよう、origin/main を既定にしない）。PR の証跡では飛ばしていないことを検証で見る）

## R4. `only-stale` は、別の理由で落ちても緑になる。置く XML の名前の決め方によっては、消す処理が無くても通る
- 成果物: openspec/changes/ch19-emulator-only-class/specs/emulator-startup/spec.md:45-47、tasks.md 2.2
- 根拠:
  - spec:47 の THEN は「rc≠0 で終わる」だけで、どんな失敗でも満たす（たとえば R1 の `find` の失敗でも）
  - tasks.md:47 は、前の実行の XML を `androidTest-results/connected/` に「置き」とだけ書き、ファイル名を決めていない。`keep_stage` は `TEST-*.xml` しか拾わない（tools/android-emulator.sh:99）。名前が `TEST-*.xml` でなければ、消す処理が無くても写らずに rc≠0 になり、試験が何も見ないまま通る。偽の gradle と同じ名前（`TEST-fake-_app-.xml`）なら、平らな段の置き場への cp でどちらが残るかが `find` の順番で決まる
  - tasks 2.2 の `--self-test` が足すのは「`class=` を外す」と「XML の確かめを外す」の 2 つで、「前の結果を消す行を外すと `only-stale` が落ちる」は無い
- kind: technical
- 提案: THEN に `only-zero` と同じ `error:` の行（渡したクラスの名前つき）を足す。tasks は、置く XML の名前を `TEST-*.xml` の形で、偽の gradle と違う名前（例: `TEST-stale-_app-.xml`）に決める。`--self-test` に「消す行を外すと `only-stale` が落ちる」を足す
- 処置: fixed 2.2 — only-stale の THEN に rc=1 と error の行を置き、置く XML を `TEST-stale-_app-.xml` と決め、self-test に「前の結果を消す行を外すと落ちる」を足した

## R5. Requirement の本文だけが言っていて、Scenario が言っていないことがある（いちばん多く通る NG の道を含む）
- 成果物: openspec/changes/ch19-emulator-only-class/specs/emulator-startup/spec.md
- 根拠:
  - Requirement 2（spec:19-25）: 「全体のときと同じにエミュレータを起こし、アニメーションを切り、写真と動画を入れ」に当たる Scenario が無い。依頼の回し方（「落ちる → 直す → 走らせる」）でいちばん多く通るのは、「渡したクラスが走って落ちたら rc≠0 になり、KEEP でなければエミュレータを止める」の道だが、これにも Scenario が無い（偽の gradle はいつも rc=0。tools/test-android-emulator.sh:86-92）
  - Requirement 3（spec:52-55）: 「確かめが `OK (1 test)` でなければ、全体のときと同じ文言で rc≠0」と「バックアップを有効にし（`bmgr enable`）」に当たる Scenario が無い。偽の adb の `am instrument` はいつも `OK (1 test)` を返す（:135）
- kind: technical
- 提案: 偽の gradle の終わり方（例: `FAKE_GRADLE_RC`）と、偽の adb が `verifyRestored` に返す出力を差し替えられるようにする。そのうえで次の 2 本を足す。
  (a) ONLY のクラスが落ちたら rc≠0 で、`-s emulator-5554 emu kill` が届く。
  (b) St56 の確かめが通らなければ、rc≠0 で `error: St56BackupTest の復元の確かめが通らない` を出す。
  アニメーションと写真・動画は、`only-class` の THEN に AND で足すより、別の 1 本にする
- 処置: fixed 2.2 — Scenario を 3 本足した（ONLY のクラスが落ちたら rc≠0 で止めてエミュレータを止める／ONLY でもアニメーションと写真・動画／St56 の確かめが通らなければ全体と同じ文言で rc=1）。偽物に FAKE_GRADLE_RC・FAKE_VERIFY_OUT を足す（tasks 1.1・3.1）。bmgr enable は St56 の Scenario の順に入れた

## R6. 決めた値が design にだけあり、Scenario は「rc≠0」「旨」で受けている。rc=2 は同じ台本の既存の約束と食い違う
- 成果物: openspec/changes/ch19-emulator-only-class/specs/emulator-startup/spec.md:24・:43・:47、design.md D3・D4・D5
- 根拠:
  - design.md:36 D3 は `error: ONLY=<クラス> のテストが 1 本も走っていない` で rc=1 と決めている。spec:24・:43 は「0 本である旨」「rc≠0」で、文言も rc も決まっていない。実装が文言や rc を変えても spec は落ちない
  - D4・D5（design.md:45・:51）は、ONLY の値が誤りのとき rc=2 にする（rc=2 は spec:67・:72・:76 にも写っている。写っていないのは D4・D5 の文言のほう）。正典の同じ台本では、設定の値が誤りなら rc=1 で、rc=2 は採寸の台本の「測れなかった」（openspec/specs/emulator-startup/spec.md:61）。D4・D5 は、なぜこの台本で rc=2 を使い分けるか（verify-run や harness2 #271 が rc=2 をどう読むか）を書いていない
- kind: technical
- 提案: Scenario の THEN に `rc=1` と `error: ONLY=<クラス> のテストが 1 本も走っていない` の字面を置く。ONLY の値が誤りのときの rc は、D4・D5 に使い分けの理由を書くか、正典に合わせて rc=1 にする（どちらでも、Scenario と完了の判定 1 の「rc=2」を同じにそろえる）
- 処置: fixed D4 仮 — D3 の文言と rc=1 を spec の Scenario に写した。ONLY の値の誤りと専用の台本のクラスは、同じ台本の既存の設定の誤りに合わせて rc=1 にし（D4・D5 に理由）、spec と完了の判定 1 をそろえた

## R7. tasks 2.2 の「直す前の台本では落ちる」は、どんな失敗でも通る
- 成果物: openspec/changes/ch19-emulator-only-class/tasks.md:52
- 根拠: `! ASHIATO_TEST_ONLY=only-class ASHIATO_TEST_SCRIPT_REV=$base tools/test-android-emulator.sh` は、終了コードが 0 でなければ何でも満たす。知らない鍵の rc=2（tools/test-android-emulator.sh:480）でも、試験の構文の誤りでも、偽の木の作りの失敗でも通り、「`only-class` の確かめが前の台本を見分けた」ことを示さない
- kind: technical
- 提案: rc≠0 に加えて、標準出力に `Scenario: ONLY にクラスを渡すと、そのクラスだけに絞った計測テストを 1 回だけ走らせる` が出ていること（知らない鍵や構文の誤りではなく、その場合の確かめまで走って落ちたこと）を見る形にする
- 処置: fixed 2.2 — 直す前の台本での否定の検証を、rc≠0 に加えて only-class の Scenario の行が出たことを見る形にした

## R8. 試験の中の変数 `ONLY` が、台本の新しい口と同じ名前。`env -u ONLY` は `run_emulator` しか覆わない
- 成果物: openspec/changes/ch19-emulator-only-class/tasks.md:18（Global Constraints）・1.1
- 根拠:
  - tools/test-android-emulator.sh:477 は `ONLY=${ASHIATO_TEST_ONLY:-}` と、場合の選びを `ONLY` という名前で持つ。呼んだ人の環境に `ONLY` が export されていると、この代入は export されたまま子へ渡る（bash の仕様）
  - tasks は `run_emulator` だけを `env -u ONLY` で守る。`assert_term_cleanup`（:426-431）は `run_emulator` を通らずに台本を直接起こす。`ONLY` が export された環境で `ASHIATO_TEST_ONLY=term-cleanup` を指定したときだけ、台本は `ONLY=term-cleanup` を受け取り、D5 の rc=2 で起こさずに止まる。その結果、試験は「偽のエミュレータが起きない」で落ちる。`ASHIATO_TEST_ONLY` を渡さない普通の実行では、:477 が `ONLY` を空にするので全体の経路になり、落ちない（起きる場面は狭い）
- kind: technical
- 提案: 試験の中の変数の名前を変える（例: `CASE`。環境変数 `ASHIATO_TEST_ONLY` の名前は変えない）か、:477 の前で `export -n ONLY` する。tasks 1.1 にどちらかを書く
- 処置: fixed 1.1 — 試験の中の場合選びの変数を CASE に改め、台本を直に起こす場合も env -u ONLY で外すと Global Constraints に書いた

## R9. spec に実装の名前（関数名・試験の環境変数）が入っている
- 成果物: openspec/changes/ch19-emulator-only-class/specs/emulator-startup/spec.md:13・:15
- 根拠: spec:13 の「台本の `keep_stage` の行の数」は台本の中の関数の名前、spec:15 の `ASHIATO_TEST_BASE_REV` は試験の道具の変数。どちらも台本を使う人から観測できる振る舞いではない（`ONLY`・`KEEP`・`androidTest-results-stage<N>` は口と出力なので構わない）
- kind: technical
- 提案: spec:13 は「全体の経路が残す段の数（この change の時点で 8）」、spec:15 は「この change の前の版の台本」と書き、実装の名前は tasks に寄せる
- 処置: fixed specs/emulator-startup/spec.md — keep_stage を「全体の経路が残す段の数」、ASHIATO_TEST_BASE_REV を「比べる元の版（この change の前の版）」に書き直し、実装の名前は tasks に寄せた

## R10. ONLY を渡したときは、正典の「正常にエミュレータが立つとき…変わらない」の字面に対する例外になる。ADDED の本文はそれを言っていない
- 成果物: openspec/changes/ch19-emulator-only-class/specs/emulator-startup/spec.md
- 根拠: 正典 openspec/specs/emulator-startup/spec.md:117-120 の WHEN（/dev/kvm を開け、同じ AVD が動いておらず、端末が現れて起動を終えるとき）には ONLY の条件が無い。SHALL は「計測テストを 6 段で走らせて…stage1〜stage6 に残し」。この change の Requirement 2（spec:17-25）は、同じ WHEN に `ONLY` を足した場面で段を 1 つにする。archive の後、正典の中で 2 つの Requirement が ONLY を渡した場面について違うことを言う。軽い路は MODIFIED を書けない（正典の WHEN に「ONLY が無いとき」を足せない）
- kind: technical
- 提案: ADDED のまま閉じられる。Requirement 1 と 2 の本文に「`ONLY` を渡したときは、正典の Requirement『正常にエミュレータが立つときの計測テストの台本の振る舞いは変わらない』の段の数の約束の例外で、この Requirement が優先する」と 1 文書く。正典の WHEN そのものを直すのは R12 と一緒に持ち越す
- 処置: fixed specs/emulator-startup/spec.md — Requirement 1 と 2 の本文に、正典の Requirement の段の数の約束は ONLY が無いか空のときの約束で、ONLY にクラスを渡したときは ONLY の Requirement が優先する、と書いた

## R11. 依頼の前提（「台本から渡すだけ」）は、依頼が挙げた例そのもので成り立たない。D2 はその読み替えで、確認バッチの問いに出ていない
- 成果物: openspec/changes/ch19-emulator-only-class/design.md D2、docs/changes/CH19.md（完了の判定 6）
- 根拠:
  - docs/changes/CH19.md:31（そのとき考えたこと）は「`-P…class` がすでにあるので、台本から渡すだけ」。:25 の例は `ONLY=dev.ashiato.collector.St56BackupTest`
  - St56BackupTest は `prepareBackup` と `verifyRestored` の間に、テストの外での往復（tools/android-backup-roundtrip.sh）が要る（St56BackupTest.kt の 2 つの `@Test`、tools/android-emulator.sh:107-123）。gradle の 1 段では通らない
  - D2 は「1 段」を「そのクラスが全体で走る手順」と読み替えた。どちらを選んでもデータも記録も失わず、依頼の目的（1 クラスの繰り返し）を満たすのはこちらだけなので、A ではなく仮（P）で進めてよい
  - ただし、完了の判定 6 の確認バッチの問い（CH19.md:82）は時間と rc だけを問い、この読み替えを人間に見せていない
- kind: premise
- 提案: 完了の判定 6 の問いに「St56BackupTest は gradle の 1 段ではなく、全体と同じ準備・往復・確かめの手順で走る（D2（仮））」を添えて、確認バッチで本人が見られるようにする。spec と tasks はそのままでよい
- 処置: escalated

## R12. archive の後も、正典の「6 段」は古いまま残る（実物は 8 段）
- 成果物: openspec/changes/ch19-emulator-only-class/proposal.md
- 根拠: 正典 openspec/specs/emulator-startup/spec.md:119・:129・:131 は「6 段」「stage1〜stage6」と書いている。今の台本の行頭の `keep_stage` は 8 本（tools/android-emulator.sh:106・115・124・130・137・145・150・155）なので、ST56・ST58 の後、正典の 6 段はもう古い。この change の ADDED は「この change の時点で 8 段」（spec:7）と書くので、archive の後は同じ正典に 6 と 8 が並ぶ。軽い路は MODIFIED を書けないので、この change では直せない
- kind: defer
- 提案: proposal の Impact に「正典の 6 段の Requirement・Scenario は古い（実物は 8 段）。R10 の WHEN と一緒に、別の change で MODIFIED で直す」と書いて持ち越す。ADDED の 8 は消さない（決定の数値として Scenario に残す）
- 処置: fixed proposal.md — Impact に、正典の 6 段の古さ（実物は 8 段）を持ち越しとして書いた（軽い路は MODIFIED を書けない。R10 の読み方を ADDED の本文に置いた）。ADDED の Scenario は 8 を決め打たず、台本から数える形にした

---

## 観点ごとの確かめ

- **観点 1（deep の決定が正典に写っているか）**: deep.md は機械の申告の写しで「本人の答え」は無い。本人の決定として扱う CH19.md の「議論で決めたこと」を 1 件ずつ当たった。
  - 「指定したクラスを 1 段で」→ Requirement 2
  - 「指定が無ければ今の 8 段のまま」→ Requirement 1（数の書き方は R9・R12。正典との例外の関係は R10）
  - 「AGENTS.md の長いコマンドに絞り方と最後の verify-run」→ D6・tasks 3.2（spec ではなく文書なので、置き場は妥当）
  - 「どこまで: 全体の段の構成と既存の tasks.md の検証コマンドは変えない」→ D1・CH19.md 壊してはいけないもの 1・4・tasks 4.3
  - 「st58-replace は対象外」→ D4

  D2〜D5 が依頼の決定と食い違うかを個別に見た。
  - D1 は全体の経路の行を変えないので、依頼の「どこまで」と合う
  - D4 は依頼の「対象外」と同じ向き。採寸の試験を断るのは範囲内の安全側
  - D5 は「指定したクラス」を 1 つに絞る形で、依頼と合う
  - D2 だけが「1 段」の読み替え（R1・R11）

  数値は 8（段）が spec にある。それ以外の数値の決定は無い
- **観点 2（検証可能か）**: R2（順番が観測できない）・R3（主張の束ねと、条件つきの THEN）・R4（rc≠0 だけ）・R5（本文だけ）・R6（「旨」）。ほかの Scenario は、偽の SDK の既存の記録（`FAKE_ARGS`・`FAKE_AVD_LOG`・`adb.log`・`run.log`）で真偽が決まる
- **観点 3（置き場）**: R9（spec に実装の名前）。
  - design の D のうち観測できる振る舞いで spec に写っていないのは、D3 の文言と rc=1、D4・D5 の文言（R6）。D4・D5 の rc=2 は spec:67・:72・:76 に写っている
  - proposal の Modified Capabilities `emulator-startup` と `specs/emulator-startup/` は一致する
  - CH19.md の capability を `device-collection` から `emulator-startup` に直したことは、「行き先」に理由が書いてある
  - `grep -c emulator-startup docs/stories/INDEX.md` は 0。INDEX の capability 表に `emulator-startup` の行は無い（CH10 が作った capability。change は INDEX の表に載っていない）。この change で表と食い違うものは無い
- **観点 4（tasks の検証）**: どの `- [ ]` にもコマンドと rc の終了条件がある。ただし、意味のある判定にならないものが R4（`only-stale`）と R7（2.2 の否定）にある。
  - 3.2 の grep は、変更の前の節では `ONLY=dev.ashiato.collector`・`scripts/verify-run`・`-- --list` が当たらないことを確かめた（`harness2:long-commands` と `cargo test` は今でも当たる）ので、判定として効いている
  - 依存の順（1 で試験の道具 → 2・3 で台本 → 4 で本物）は正しい。1.1 の BASE_REV の比べは、台本を変える前に走るので必ず同じになるが、3.1 で同じコマンドを走らせ直す
  - 4.1・4.2 は /dev/kvm が要ることと、BLOCKED_INFRA の扱いが書いてある。人間の確認待ちは無い（完了の判定 6 に、人間が触るものは無いと明記してある）
- **観点 5（change）**:
  - 完了の判定は空でなく、spec と合っている（rc=2 は R6、St56 の読み替えの見せ方は R11）
  - 壊してはいけないもの 1〜5 は、それぞれ Requirement 1、既存の試験と `--self-test` の実行（tasks 1.1・2.2）、正典の Scenario「計測テストの台本の行の形の検査は、今のまま通る」と tasks 1.1・2.1 の `tools/test-usage-volume.sh`、tasks 4.3 で受けている
  - specs は依頼の範囲を越えていない（採寸の試験を断るのは安全側）
  - `tools/test-usage-volume.sh` との衝突は無い。:88 は、行頭が `#` の行だけを除いた最初の `connectedDebugAndroidTest` が、`^adb uninstall dev.ashiato.collector `（:89）より後かを見る。ONLY の段の塊は uninstall（tools/android-emulator.sh:89-90）の後に置かれ、値の検査（`avdmanager` の前）に `connectedDebugAndroidTest` の字面は出ない（D4・D5 の文言・冒頭の `#` の使い方）。`notClass` の行は変えない
  - `assert_normal` の行頭の `keep_stage` の数え方（tools/test-android-emulator.sh:302）も、D1 の字下げで保たれる
- **観点 6**: 該当なし（`spec --answers` ではない軽い路）
観点 7: 該当なし（spec_overlap.py candidates が 0 件）
