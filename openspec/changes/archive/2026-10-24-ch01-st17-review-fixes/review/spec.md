# CH01 の仕様の独立レビュー（spec --draft。第 1 回の推奨の答えで書いた版）

対象: `openspec/changes/ch01-st17-review-fixes/`（proposal.md / specs/subjective-log/spec.md / specs/personal-entities/spec.md / design.md / tasks.md）と `docs/changes/CH01.md` の「壊してはいけないもの」「完了の判定」。
deep.md の Q1〜Q7 は推奨を本人の答えとみなし、deep-questions.json の note の「問わずに決めた 2・3・9」も決定として扱った。

## 機械の検査（先に写す）

- `openspec validate ch01-st17-review-fixes --strict` → `Change 'ch01-st17-review-fixes' is valid`（rc=0）
- `python3 scripts/check_chain.py .` → `chain: OK (0 件 / 未回収 0 件 / warn 0 件)`（rc=0）
- `python3 scripts/check_scenarios.py . ch01-st17-review-fixes` → `scenarios: FAIL (担保なし 13 件 / 名無しの確認待ち 0 件)`（rc=1）。
  13 件はこの change の delta の Scenario 全部（subjective-log 5・personal-entities 8）で、下流がまだ test を置いていないため（draft では想定どおり。欠陥としては扱わない。ただし R5 の依存順に効く）。
  ほかの `[warn] spec に無い Scenario を指す印` 15 件は CH01 の外（collector-windows・archive_flow_tests・migrate_backup・st12_delta_diff.py）
- `python3 scripts/review_triage.py . ch01-st17-review-fixes` → `triage: OK`（指摘 7 件 / 仮決め 0 件 / 要件へ戻すもの 0 件）（rc=0。このファイルを書く前の値。書いた後は R1〜R20 が「処置が無い」で FAIL になる —— 処置は呼び出し元が付ける）

## 確かめて合っていたもの（指摘なし）

- **MODIFIED が正典の全文を保っているか**: `openspec/specs/subjective-log/spec.md` の 2 つの要件（「主観入力の画面で、押した瞬間に保存する」「主観入力の画面で、その日の滞在ごとの気分を書く」）と delta を difflib で突き合わせた。
  見出しは一字一句同じで、差は SHALL の追加（各 1〜2 行）・導出元の追記・理由の段落の追記・Scenario の追加（2 本 / 3 本）だけ。正典の SHALL と Scenario は 1 行も落ちていない
- **personal-entities の ADDED の 8 本が、いまの取り込みの振る舞いと一致しているか**: `crates/server/src/lib.rs` を辿った。
  主張は `parse_claim` の Malformed（:1016-1022）→ 由来・端末（:1025-1030）→ 外部識別子（:1031-1036）→ 地域のずれ ±1439（:1042-1047）→ 残りの値（:1048-1051）、
  場所は Malformed（:1088-1092）→ 由来・端末（:1093-1095）→ 外部識別子（:1096-1098）→ 地域のずれ（:1100-1102）→ 値（:1103-1106）→ 座標系（:1107-1111）。
  一般の検査 `validate()`（`ingest.rs:130`）は地域のずれも地域名との整合も見ないので、±1,439・0 は通る。8 本とも、いまの振る舞いと同じ結果を言っている（`attributes_tests.rs:1980` `ingest_rejects_out_of_range_tz_offset` も ±1440 断る・±1439/0/540 通すを固定済み）
- deep の推奨と note の 2・3・9 の写り先: Q1→D1・Task 2 / Q2→D2・Task 1 / Q3→D9・Task 5.1・5.3 / Q4→D5・spec（SHALL 2 行と Scenario 3 本）・Task 7 / Q5→D6・spec（SHALL 1 行と Scenario 2 本）・Task 8 / Q6→D8・Task 6 / Q7→D7・personal-entities の ADDED・Task 4 / 2→D3・Task 2.2-2.3 / 3→D4・Task 3 / 9→D10・Task 5.2。抜けは無い（中身の食い違いは下の R）
- Q7 の「主観の東京の条件は主観の側に残す」は D7 と Task 4.2 にあり、`lib.rs:1060-1068` のいまの形と合う
- proposal の Capabilities（subjective-log・personal-entities）と `specs/` のディレクトリが一致。`CH01.md` の `capability` にある `browsing-views` に delta が無い理由は proposal.md:44 にある
- delta の Scenario 13 本は全部どれかの Task が名指ししている（Task 4.1 に 8、7.1 に 3、8.1 に 2）
- `CT` / `VT` / `ET` の定義は `[1-9][0-9]* passed` を grep するので 0 本では通らない。`cargo test -p ashiato-server`（`crates/server/Cargo.toml` の name と一致）、`web/src/__tests__/FeelView.test.tsx`・`web/e2e/feel*.spec.ts`・`npm run typecheck|lint|build`・`tools/check-openapi.sh`・`tools/check-log-private.sh` は実在する
- `tools/check-immutable.d/*.sh` の `# Scenario:` の印は `scripts/check_scenarios.py` が数える（`TEST_EXT` に `.sh`。:60）

---

## R1. D1 の「本体の注記を直す」と「読む口を文言を変えずに動かす」が矛盾し、注記を直すと D2 の例外が外れて CH01 の PR 自身が落ちる
- 成果物: openspec/changes/ch01-st17-review-fixes/design.md（D1 / D2）
- 根拠: design.md:33（読む口のかたまりを「文言を変えずに」動かす）/ design.md:40（「README と本体の注記の『末尾が読む』を…直す」）/
  design.md:51（D2 が通すのは「基準の木と HEAD でかたまりの中身が同じ」移動だけ）/
  本体で「末尾」「読む」を言っている注記は読む口のかたまりの中にしか無い: tools/check-immutable.sh:1502-1506（「このファイルの末尾を並走する Story が取り合い」「feat/st* がこの見出しより前の最後の節の後ろに書き足すと…落とす（途中の既存の節を直すのは通す）」。後者は D2 の後は事実でもなくなる）/
  `grep -n 'check-immutable.d' tools/check-immutable.sh` は 1502-1507 だけ / tools/check-immutable.d/README.md も「`tools/check-immutable.sh` の末尾が…読む」「末尾の注記のとおり」と言う /
  CI は PR の HEAD の台本で `tools/check-shared-appends.sh "$HEAD_REF"` を走らせる（.github/workflows/ci.yml:295-298）
- kind: conflict
- 提案: 直す注記がどれかを D1 に名指しする。かたまりの中の注記を直すなら、D2 の例外を「かたまりの `for … done` の行が同じ」に緩めるか、注記の直しを読む口の移動と別の chore の PR に分ける。
  かたまりの外（README と R117 の注記 :1345-1348）だけを直すなら、かたまりの中の古い 2 文（「末尾」「途中は通す」）が残ることを D1 に書く
- 処置: fixed D1 仮 — 見出しの行と `for … done` の行だけを一字一句保ち、かたまりの中の注記は直すと D1 に名指しした。D2 の例外も見出しと `for … done` だけを比べる形にした

## R2. D2 の守る範囲は、読む口の `done` と最後の判定の行の間を外す。今の規則が落としている「末尾への書き足し」が通るようになる
- 成果物: openspec/changes/ch01-st17-review-fixes/design.md（D2）/ tasks.md（Task 1.2）
- 根拠: design.md:49-50（守る末尾は「読む口の見出しの直前の空でない行から `done` まで」と「ファイルの最後の空でない行より後ろ」。「読む口が末尾にある木では今までと同じ範囲になり」）/
  tools/smoke.sh:1129 が `done`、:1130 が空行、:1131 が最後の判定の `echo`。tools/check-immutable.sh:1512 が `done`、:1513 が空行、:1514 が `[ "$fail" -eq 0 ] && …`。
  :1130 の後ろ（判定の行の手前）への挿入は `@@ -1130,0 …` になり、どちらの範囲にも入らない。今の `tail_touched()` は `last >= body_end`（tools/check-shared-appends.sh:36-39）で、ここも落としている /
  既存の self-test「ファイルの最後に書き足すと落とす」は `echo more >>`（:115-116）なので、この穴を見ない。D2 の新しい self-test 6 件（design.md:52-53）にも無い。seed.sh は :610 の `done` で終わるので当たらない
- kind: technical
- 提案: 2 つ目の範囲を「最後の空でない行の 1 つ前の空でない行より後ろ」にする。動かす前の木では読む口の `done`（smoke.sh:1129・check-immutable.sh:1512）より後ろで今と同じになり、動かした後の check-immutable.sh では app-role の `done`（いまの :1500）より後ろになるので、戻しの並びの途中を直すのは通したまま判定の行の手前への挿入を落とせる（「読む口の `done` より後ろ全部」にすると、動かした後の木で戻しの並び全体を守ってしまい D2 の「戻しの並びの途中を直す → 通す」と衝突する）。self-test に「`done` と最後の行の間に 1 行足す → 落とす」を動かす前と後の両方の木で足す
- 処置: fixed D2 仮 — 2 つ目の守る範囲を「最後の空でない行の 1 つ前の空でない行より後ろ」にし、`done` と最後の判定の行の間への挿入の self-test を動かす前と後の木で足した（Task 1.2）

## R3. D2 の見出しの型は smoke.sh の節の見出しの形（`echo "== 45. …（ST99 …）"`）を拾わない。smoke.sh の途中への新しい節は、コメントの見出しを付けなければ通る
- 成果物: openspec/changes/ch01-st17-review-fixes/design.md（D2）/ tasks.md（Task 1.2）
- 根拠: design.md:47（見出しは `^# *[=-]{3,}.*ST[0-9]{2}` と `^echo "== (Scenario: )?ST[0-9]{2}`）/
  smoke.sh の節の echo は番号で始まる: tools/smoke.sh:886 `echo "== 41. 位置を送るとその日の滞在が一覧に出る（ST16 / …）"`・:920・:959・:1047・:1111（`== ST` で始まる echo は 1 本も無い）/
  コメントの見出しを持たない節は既にある（smoke.sh:194 `echo "== Scenario: 決まり文句の…"`、:838）/
  反対側の誤検出: 既存の小見出し（check-immutable.sh:834 `# --- ST22: …`・:854 `# --- ST33: …`）の文言を直すと `-U0` の `+` 行になり、見出しを「足した」と数えて落とす（design.md:54 の反転条件は別の場合を言っている）
- kind: technical
- 提案: smoke.sh の echo の型（`^echo "== [0-9]+[a-z]?\. .*ST[0-9]{2}`）を足すか、型を足した行の数でなく「`-` の行に同じ見出しが無い `+` の見出し」に絞る。self-test に「smoke.sh の途中に `echo "== 45. …（ST99）"` を足す → 落とす」「既存の小見出しの文言を直す → 通す」を足す
- 処置: fixed D2 仮 — smoke.sh の `echo "== <番号>. …ST<NN>"` の型を足し、同じ文字列の `-` 行がある見出しの言い直しは通す形にした。self-test 2 件を足した（Task 1.2）

## R4. Task 2.3 の壊し方（戻しを「`core.feeling_reminder` の行を消す」に差し替える）では、期待している NG が出ない
- 成果物: openspec/changes/ch01-st17-review-fixes/tasks.md（Task 2.3）
- 根拠: tasks.md:53-55（出力に `NG 戻しで主観の行か台帳か登録簿の行が減った` があり、`OK feeling 主観が残っていれば…` が無い、を期待）/
  migrations/202610141000_subjective_log.sql:61-64（`feeling_reminder_append_only` は `BEFORE UPDATE OR DELETE … FOR EACH ROW` で拒む）。行が 1 つある状態の `DELETE FROM core.feeling_reminder` は拒まれ、
  `psql` は `ON_ERROR_STOP=1`（tools/check-immutable.sh:13）なので出るのは `NG 202610141000_subjective_log.down.sql が当たらない`（:1329-1330）。行は減らないので :1331-1335 の数の検査は通り、D3 の if/else の後は OK が出る。
  期待どおりの出力にはならず、直しが正しくても検証は通らない
- kind: technical
- 提案: 差し替えた戻しの中で錠を先に外す（`DROP TRIGGER feeling_reminder_append_only …; DELETE FROM core.feeling_reminder;`）か、錠の無い行（登録簿の `s01-feeling` の行。外部キーを先に確かめる）を消す形にする。直す前の木で走らせて、NG と OK が並ぶことも見る（壊し方が効いていることの確かめ）
- 処置: fixed 2.3 — 差し替える戻しで錠のトリガを先に外す形にし、直す前の木で NG と OK が並ぶことを先に見る手順を足した

## R5. Task 2.4 の `python3 scripts/check_scenarios.py .` rc=0 は、Task 4・7・8 が印を置くまで通らない（依存順）
- 成果物: openspec/changes/ch01-st17-review-fixes/tasks.md（Task 2.4）
- 根拠: tasks.md:56 / `python3 scripts/check_scenarios.py .`（引数なし）を今の木で走らせると rc=1。FAIL はこの change の delta の 13 本（Task 4.1・7.1・8.1 が印を置くもの）
- kind: technical
- 提案: 2.4 の検証を「ST17 の錠の節が持っていた `# Scenario:` の名前が `tools/check-immutable.d/` の 1 本に全部ある」（移す前の `grep '# Scenario:'` の集合と diff）に絞り、`check_scenarios.py` の rc=0 は Task 9.1 に任せる
- 処置: fixed 2.4 — 検証を `# Scenario:` の印の集合の diff に絞り、`check_scenarios.py` の rc=0 は Task 9.1 に任せた

## R6. Task 3.3 の「5 秒後に kill -9」は gradle の段に届く前に止める。直す前の木でも検証が通り、直したことを見分けない
- 成果物: openspec/changes/ch01-st17-review-fixes/tasks.md（Task 3.3）
- 根拠: tasks.md:69-71 / tools/record-env.sh で Android の段（:166-172 でトークンを書く）より前に `cargo build -q --release`（:148）と `npm ci && npm run build`（:150）が走る。5 秒ではそこで止まり、トークンはまだどこにも書かれていない。
  「Android の段だけを後ろで走らせる」入口が record-env.sh にあるとも書かれていない
- kind: technical
- 提案: stub の `gradlew` が印のファイルを置いてから `sleep` し、印が現れてから `kill -9` を送る形にする。直す前の木で同じ手順を走らせ、`gradle.properties` にトークンが残る（検証が落ちる）ことを先に見る
- 処置: fixed 3.3 — stub の gradlew が印のファイルを置いてから眠り、印が現れてから kill -9 を送る形にした。直す前の木で検証が落ちることも先に見る

## R7. Task 1.2 の self-test の下限「13 行以上（既存 7 件＋6 件）」は既存の件数を取り違えている。新しい 6 件のうち 2 件しか無くても通る
- 成果物: openspec/changes/ch01-st17-review-fixes/tasks.md（Task 1.2）/ design.md（Risks）
- 根拠: tasks.md:37 / design.md:121（「既存の 7 件」）/ tools/check-shared-appends.sh の `expect` は :108・111・114・116・118・121・123・126・129・132・134 の 11 件
- kind: technical
- 提案: 下限を 17 にするか、D2 の 6 件の説明文をそれぞれ `grep -F` で名指しする
- 処置: fixed 1.2 — 既存は 11 件。D2 の 10 件と合わせて下限を 21 にした

## R8. `docs/handoff/ST23.md` は既にある。「新規」と書いているので、書き手が上書きすると ST22・ST16・ST19・ST21・ST12 からの申し送り 10 節を消す
- 成果物: openspec/changes/ch01-st17-review-fixes/proposal.md / design.md（D9）/ tasks.md（Task 5）
- 根拠: proposal.md:55「`docs/handoff/ST23.md`（新規…）」/ design.md:107-108「`docs/handoff/ST23.md`（新規）に書く」/ tasks.md:89「`docs/handoff/ST23.md`（新規）」/
  `ls docs/handoff/` に ST23.md があり、`grep -n '^## ' docs/handoff/ST23.md` は 10 節（:5 から :98）
- kind: technical
- 提案: 「新規」を「末尾に 1 節足す」に直す。Task 5.3 の検証に「既存の節の見出しの数が減っていない」（`grep -c '^## '` が 11）を足す
- 処置: fixed D9 仮 — proposal・D9・Task 5 の「新規」を「末尾に 1 節足す」に直し、5.3 の検証に `grep -c '^## '` が 11 を足した

## R9. personal-entities の ADDED は「地域のずれが範囲の外 → `malformed_claim` / `malformed_place_record`」を言うが、正典の表の同じ種別の行には無い。同じ理由の種別の条件が 2 つの要件に分かれる
- 成果物: openspec/changes/ch01-st17-review-fixes/specs/personal-entities/spec.md
- 根拠: delta :6 / 正典 openspec/specs/personal-entities/spec.md:92-96（「次の表のいずれかに当たる主張を…」。`malformed_claim` の行は JSON・必須の欄・識別子・乱数だけ）/ :732-736（`malformed_place_record` の行にも地域のずれが無い）。
  archive の後、表だけを読むと地域のずれで断ることが分からない
- kind: technical
- 提案: ADDED の代わりに「形の合わない主張は受け付けない」「形の合わない場所の記録は受け付けない」を MODIFIED にし、表の `malformed_*` の行に「地域のずれが -1,439〜+1,439 分の外」を足し、確かめる順の 1 文と 8 本の Scenario をそこへ置く（MODIFIED は全文を写す）
- 処置: fixed specs/personal-entities/spec.md — ADDED をやめ、「形の合わない主張は受け付けない」「形の合わない場所の記録は受け付けない」を MODIFIED にして表の `malformed_*` の行に地域のずれを足し、順の SHALL と Scenario をそこへ置いた

## R10. personal-entities の要件の本文が言う順のうち、場所の「形 → 由来」と、主張・場所の「地域のずれ → それより後の値の検査」を、どの Scenario も固定していない
- 成果物: openspec/changes/ch01-st17-review-fixes/specs/personal-entities/spec.md
- 根拠: delta :5（順: 原文の形 → 由来と端末識別子 → 外部識別子 → 地域のずれの範囲 → それより後の値の検査）/
  Scenario は主張の「形 vs 由来」（:19-22）・「由来 vs 外部」（:14-17）・「外部 vs 地域」（:34-37）と、場所の「由来 vs 外部」（:39-42）・「外部 vs 地域」（:49-52）だけ。
  場所の「形 vs 由来」、主張・場所の「地域 vs 値」（例: 地域のずれ +1,440 で値が空の主張 → いまは `malformed_claim`。lib.rs:1042-1051）は無い。
  D7 で 3 つの写しを 1 つの関数に寄せたとき、呼ぶ位置が値の検査の後ろへずれても spec は落ちない
- kind: technical
- 提案: 「形が崩れて端末識別子も持つ場所の記録は形の理由で断る」「地域のずれが範囲の外で値も空の主張は形の理由で断る」「地域のずれが範囲の外で名前も空の場所の記録は形の理由で断る」の 3 本を足し、Task 4.1 の名指しと下限（11 本）を増やす
- 処置: fixed specs/personal-entities/spec.md — 「形が崩れて端末識別子も持つ場所の記録は形の理由で断る」「地域のずれが範囲の外で値も空の主張は地域の理由で断る」「地域のずれが範囲の外で名前も空の場所の記録は地域の理由で断る」を足し、Task 4.1 の名指しと下限（14 本）を直した

## R11. Scenario「由来も外部識別子も違う場所の記録は由来の理由で断る」の WHEN は由来でなく端末識別子を変えている
- 成果物: openspec/changes/ch01-st17-review-fixes/specs/personal-entities/spec.md
- 根拠: delta :39-42（WHEN「端末識別子と外部サービス上の識別子を持つ場所の記録」）。結果の種別 `place_record_not_authored` は表どおり（正典 :735）だが、名前と WHEN が違うことを言っている。
  主張の側の同名の Scenario（:14-17）は由来（「収集した」）を変えている
- kind: technical
- 分類の直し: spec の段で daily から technical に直した。Scenario の名前と WHEN の食い違いで、日常の選択ではない
- 提案: 名前を「端末識別子と外部識別子を持つ場所の記録は由来の理由で断る」にするか、WHEN を由来「収集した」に揃える
- 処置: fixed specs/personal-entities/spec.md — 名前を「端末識別子と外部識別子を持つ場所の記録は由来の理由で断る」にした

## R12. Q5 の 2 本の Scenario の時刻に地域が無い。e2e が UTC で読むと、日の終わりをまたがない検査になる
- 成果物: openspec/changes/ch01-st17-review-fixes/specs/subjective-log/spec.md
- 根拠: delta :122・:127（「2026-10-07 23:58 に…」「2026-10-08 00:01 に…」）。同じ要件の「日付の無いアドレスは夜中も暦の今日を開く」（:37）は「`Asia/Tokyo` で 2026-10-08 00:30」と地域を書いている。
  日の区切りは `Asia/Tokyo` の 24 時（正典 :255 の「対象の終わり」）。`page.clock.install` は絶対時刻を取るので、23:58 を UTC で置くと JST の 08:58 になり、どちらの Scenario も前/後が変わらない
- kind: technical
- 提案: 2 本とも「`Asia/Tokyo` で 2026-10-07 23:58 に…」と書く。Task 8.1 の e2e の時刻も `+09:00` つきで置くと書く
- 処置: fixed specs/subjective-log/spec.md — Q5 の Scenario の時刻に `Asia/Tokyo` を書き、Task 8.1 の e2e は `+09:00` つきで置くと書いた

## R13. Q5 の Scenario「開いたまま日の終わりをまたいで押すと振り返りとして残る」は THEN に 4 つの主張を束ねている。いまの実装でも知らせ以外の 3 つは既に成り立つ
- 成果物: openspec/changes/ch01-st17-review-fixes/specs/subjective-log/spec.md
- 根拠: delta :122-123（THEN: 「後」として格納 / 保存したことと振り返りの知らせ / -1 が押された状態 / 「前に書いた」+1 の 1 行）/
  記録の前/後はサーバが記入の日時で決める（正典 :255。直す前から「後」で入る）。受理の後は `tick` で読み直し、読めたら `setNow(new Date())` する（web/src/FeelView.tsx:112-124）ので、押した後の表示も既に「後」の側に移る。
  この Scenario で直す前に落ちるのは「振り返りとして残したことが出る」だけで、ほかが崩れても知らせが出ていれば 1 本として緑になる
- kind: technical
- 提案: 格納（「後」になる）と表示（知らせ・押された状態・1 行）を 2 本に分ける。知らせの文言（proposal.md:13 の「振り返りとして残しました」）を THEN に書く
- 処置: fixed specs/subjective-log/spec.md — 格納（後として格納される）と表示（「振り返りとして残しました」・押された状態・1 行）を 2 本に分け、文言を THEN に書いた

## R14. D6 の「届かなかった記録は、届いたときに同じ判定をする」は Scenario が無く、日をまたいだ後に「前」の記録が届くと「振り返りとして残した」と誤って出しうる
- 成果物: openspec/changes/ch01-st17-review-fixes/design.md（D6）/ specs/subjective-log/spec.md
- 根拠: design.md:82 / delta :20（SHALL「画面が出している前/後と違えば…振り返りとして残したことを…出す」）。
  例: 23:59 に押して届かず（記入の日時は 23:59 → 前）、00:01 に別の値を押して届いて画面が「後」に移った後に、23:59 の原文の送り直しが届く。記録は「前」なのに画面は「後」を出しているので「違えば」に当たり、「振り返りとして残しました」が出る。
  送り直しの原文は同じ記入の日時を持つ（正典 :743 の SHALL「届かずに送り直すときは同じ原文を送る」）
- kind: conflict
- 分類の直し: spec の段で premise から conflict に直した。外の形式や事実の前提ではなく、D6 の知らせの条件と送り直しの SHALL（同じ原文を送る）の食い違い。失われるものは無く、前/後は記入の日時から計算し直せる
- 提案: 知らせの条件を「記入の日時の前/後が『後』で、押したときに画面が『前』を出していた」に絞るか、送り直しでは知らせないと決める。どちらかを SHALL にし、送り直しが日をまたいで届く Scenario を 1 本足す
- 処置: fixed D6 仮 — 知らせの条件を「押したときに画面が『前』で、記入の日時が『後』」に絞り、送り直しでは知らせない SHALL と Scenario「日をまたいでから届いた送り直しは振り返りの知らせを出さない」を足した

## R15. D8 の「主観の読み出しは開いて 1 回・失敗は日の欄と暦の両方に出る」は観測できる振る舞いで、完了の判定にもあるのに、spec に無い
- 成果物: openspec/changes/ch01-st17-review-fixes/design.md（D8）/ docs/changes/CH01.md（完了の判定）
- 根拠: design.md:97-99（「利用者に見える差は…spec に書かない。勘定は jsdom の試験で固定する」）/ docs/changes/CH01.md:83（「主観の読み出しは開いて 1 回」）/
  `openspec archive` は main specs しか更新しないので、archive の後に 2 回へ戻しても正典の Scenario は落ちない。正典の「読み出しの失敗と未記入を区別する」（正典 :1013-1016）は暦の「未」しか見ていない
- kind: daily
- 提案: subjective-log の「主観入力の画面の読み出しの失敗と下限」を MODIFIED にし、「読み出しが失敗したら日の欄も読み出しの失敗を出す」Scenario を足す（1 回の勘定は jsdom のままでよい）。spec に置かないなら完了の判定の文を D8 と tasks 6.1 の検証の言葉に揃える
- 処置: fixed D8 仮 — 「主観入力の画面の読み出しの失敗と下限」を MODIFIED にし、30 日の 1 回で読む SHALL と Scenario「読み出しの失敗は日の欄にも出る」を足した（Task 6.1）

## R16. 「日の欄は断られても消さない」の SHALL は滞在の行と区別する言葉を持たず、区別は理由の段落の「CH01 の Q4 の推奨」だけにある
- 成果物: openspec/changes/ch01-st17-review-fixes/specs/subjective-log/spec.md
- 根拠: delta :16（「送れなかったとき、選んだ値と入れたひとことを消さずに…」。要件 1 の中にあるので日の欄の話と読めるが、SHALL 自身は欄を名指ししない）/ :151（滞在の行は断られたら戻す）/
  :162（「CH01 の Q4 の推奨」と、推奨であることを正典に書いている。本人の答えを受けた後も残ると古くなる）
- kind: daily
- 提案: :16 の頭に「日の欄で」を足す（要件 1 の MODIFIED の中で直せる）。:162 の「Q4 の推奨」を「Q4」にする（record が答えで書き換えるときに一緒に直す）
- 処置: fixed D5 仮 — SHALL の頭に「日の欄で」を足し、理由の段落の「Q4 の推奨」を「Q4」にした

## R17. *.d/ の節が本体の後ろの検査より先に走るようになるのに、節が本体の変数や関数を上書きしてはいけないことがどこにも書かれていない
- 成果物: openspec/changes/ch01-st17-review-fixes/design.md（D1 / Risks）
- 根拠: design.md:119-120（Risks は「本体の後ろの検査は…`*.d/` の節が残す行に依らない」とだけ言う）/
  読む口は同じ shell の中で `.` で読む（tools/check-immutable.sh:1507-1512）。動かした後は、節の後ろに R117 の `MIGS` `ST03_UP` `down_fail`（:1350-1447）、`psql`（:13）、`fail` を使う検査が続く。
  節が `fail=0` や `MIGS=…` を書くと、後ろの検査が黙って変わる（今は節が最後なので後ろに検査が無い）
- kind: technical
- 提案: README と読む口の注記に「本体の変数（`fail` `MIGS` `ST03_UP` `AU` …）と関数を上書きしない」を足す。D1 の自立した節の変数は節の接頭辞（`ST17_…`）にすると書く
- 処置: fixed D1 仮 — README と読む口の注記に本体の変数・関数を上書きしないことを書き、節の変数に `ST<NN>_` を付けると D1 に書いた。Risks と Task 2.2 の検証にも足した

## R18. Task 1.3 と Task 3.2 の終了条件が、機械で真偽の決まらない形か、何も見ていない形になっている
- 成果物: openspec/changes/ch01-st17-review-fixes/tasks.md（1.3 / 3.2）
- 根拠: tasks.md:39（「`git diff --quiet tools/check-shared-appends.sh` が 1.2 の差分だけ」。`--quiet` は差があるか無いかだけを返す）/
  tasks.md:68（「`git status --porcelain` が空のまま」。`collector-android/local.properties` は .gitignore:29 で無視されるので、足した行を消し忘れても空のまま）
- kind: technical
- 提案: 1.3 は「写しを走らせた後 `git diff --stat` が走らせる前と同じ」。3.2 は「走らせた後 `! grep -q ch01-probe collector-android/local.properties`」にする
- 処置: fixed 1.3 — `git diff --stat` が走らせる前と同じ、に直した。3.2 は `! grep -q ch01-probe collector-android/local.properties` にした（fixed 3.2）

## R19. e2e の日を「2030 年以降」にしている。ST17 の e2e は理由つきで「2100 年以降」にしている
- 成果物: openspec/changes/ch01-st17-review-fixes/tasks.md（Global Constraints）
- 根拠: tasks.md:12 / web/e2e/feel.spec.ts:7、web/e2e/feel-parts.spec.ts:26-28（「2100-01-01 から約 8 年の中の…実際の今日がいつになっても『これから』の側に残る十分先」）。
  2030 年代は数年で実際の今日が追いつき、暦の「これから」の扱いが変わる
- kind: technical
- 分類の直し: spec の段で daily から technical に直した。e2e の日の置き方で、ST17 の既存の形に揃えるだけ
- 提案: ST17 と同じ 2100 年以降にする（`freshDay()` の形を写す）
- 処置: fixed tasks.md — 2100 年以降（ST17 の `freshDay()` と同じ形）にした

## R20. CH01.md の「価値」が依頼の写しのままで、「上流の深掘りの後に…1〜3 文に整える」の注が残っている
- 成果物: docs/changes/CH01.md
- 根拠: docs/changes/CH01.md:12-38（「## 価値」の下が依頼の逐語と同じ 9 項目と、:38 の注）
- kind: defer
- 提案: 答えを受けた record の段で、何のために変えるか（道具の置き場の取り違いを機械で止める・トークンを追跡ファイルに書かない・画面と記録の前/後を合わせる）の 1〜3 文に直す

---

観点 1: R12〜R14・R16（Q4・Q5 の写り方）。数値の決定（地域のずれ ±1,439）は Scenario にある。ほかは該当なし（確かめた範囲: deep.md の Q1〜Q7 と deep-questions.json の note の 2・3・9 を、proposal・D1〜D10・Task 1〜9・delta の Scenario に 1 件ずつ当てた）
観点 2: R10〜R14
観点 3: R15（D8）・R14（D6 の送り直し）。specs に実装の名前（関数名・crate 名・列名）は無い（`grep -nE 'fn |crates/|core\.|\.rs\b|\.tsx\b|lib\.rs|FeelView' specs/*/spec.md` が 0 行。理由の種別 `malformed_claim` などは API の契約の値）
観点 4: R4〜R7・R18。人間の確認待ちの要る検証は無い（物理に依るものが無い。tasks.md:13 と合う）
観点 5: CH01.md の「完了の判定」は書かれている。「壊してはいけないもの」は Task 1.1/1.2（self-test）・2.2（OK の行の集合）・6.2/7.2（ST17 の e2e。「使い始める前の日は未記入と出ない」が始まりの日の求め方を担保）・9.1（openapi）で固定されている。依頼の範囲を越える specs は無い（personal-entities は Q7 の推奨の帰結で、CH01.md の capability に足してある）。R15・R20
観点 6: 該当なし（`spec --draft`。`--answers` ではない）
- 処置: fixed proposal.md — proposal の Why と同じ中身を docs/changes/CH01.md の「価値」に 1〜3 文で写した。 「価値」を 1〜3 文に整えた（推奨の答えで書いた。record が本人の答えで直す）

