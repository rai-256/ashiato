# CH01 — ST17 の /code-review の 9 件を直す（tasks）

依頼は `docs/changes/CH01.md`、決定は `deep.md`（Q1〜Q7）と `design.md`（D1〜D10）、確かめた前提は `premises.md`。

## Global Constraints

- **CH01 は ST17（archive 済み）の実装を直す change で、`requires` は無い。** 走っている Story は無い。ST17 の archive 済みの成果物（`openspec/changes/archive/2026-10-17-st17-daily-feeling/`）は読むだけで書き換えない
- **振る舞いを変えない直し（Task 1〜6）は、いまの振る舞いを先に固定してから直す。** 固定は、試験か検査の出力の比較（各 Task の検証に書いた）。固定する前に直さない
- **テストには `Scenario: <名前>` の印を置く。** Rust / TypeScript はコメント（`// Scenario: 滞在の行で断られた値は押された状態に残らない`）、bash は `# Scenario:` か `echo`。
  名前は spec の `#### Scenario:` と一字一句合わせる。この change の delta の Scenario は全部どこかの Task が名指しする（`python3 scripts/check_scenarios.py . ch01-st17-review-fixes`）
- **画面の Scenario（Task 7・8）は `web/e2e`（本物のブラウザ・本物のサーバ）に印を置く。** 新しいファイル `web/e2e/feel-ch01.spec.ts` に置く（ST17 の `feel.spec.ts` に書き足さない）。
  主観の行は DB の錠で消せないので、**書き込む e2e は実行ごとに乱数で選んだ 2100 年以降の日を `page.clock` の今日にして書き、件数は書く前との差で数える**（ST17 の `web/e2e/feel-parts.spec.ts` の `freshDay()` と同じ形）。時刻は `+09:00` つきで置く。
  「人間の確認待ち」に逃がせる Scenario は無い（物理に依るものが無い）
- **件数つき検証**: このファイルで
  **`CT <名前>`** は `bash -o pipefail -c 'set -a; [ -f .env ] && . ./.env; set +a; cargo test -p ashiato-server <名前> 2>&1 | tee /tmp/ct.log' && grep -Eq 'test result: ok\. [1-9][0-9]* passed' /tmp/ct.log`、
  **`VT <ファイル>`** は `bash -o pipefail -c 'cd web && npx vitest run src/__tests__/<ファイル> 2>&1 | tee /tmp/vt.log' && grep -Eq 'Tests +[1-9][0-9]* passed' /tmp/vt.log`、
  **`ET <ファイル>`** は `bash -o pipefail -c 'set -a; [ -f .env ] && . ./.env; set +a; cd web && npx playwright test e2e/<ファイル> 2>&1 | tee /tmp/et.log' && grep -Eq '[1-9][0-9]* passed' /tmp/et.log`
  が rc=0 になることを指す。`cargo test` は repo の根から `scripts/quiet-run` で走らせ、絞り込みは 1 つずつ書く
- **試験の名前の接頭辞は Task ごとに重ならない**: `ch01_authored_`（Task 4）/ `ch01_stay_open_`（Task 5）
- **本人の決定（下流は変えない）**: 読む口を戻しの前へ動かし ST17 の節を自立させて `*.d/` へ（Q1）/ 途中の節の見出しも落とす（Q2）/ 終わりの読めない滞在は直さず申し送る（Q3）/
  滞在の行だけ断られたら戻す・日の欄は spec どおり（Q4）/ 押したときに前後を決め直して知らせる（Q5）/ 30 日の 1 回にまとめる（Q6）/ 規則を 1 つにまとめ種別と順は変えない（Q7）。
  第 1 回は推奨で先に書いている —— 本人の答えが推奨と違った Q は、上流が書き直してから下流に入る
- **D1 / D2 / D5 / D6 / D7 / D8 / D9 は（仮）決め。** 反転条件は `design.md` にある
- **理由の種別・順・応答の形・移行・API の契約（`docs/openapi.json`）を変えない。** `tools/check-openapi.sh` rc=0 で、`docs/openapi.json` が main と同じ（Task 9）
- **主観の値・ひとことをログに出さない**（製造準備 A-2）。`tools/check-log-private.sh` rc=0
- **共有の本体に Story の節を書き足さない。** この change が本体を直すのは、D1 の読む口の移動と ST17 の節を抜くことだけ（`tools/check-shared-appends.sh` rc=0。Task 1 の規則で通る）

## Task 1: 共有の本体の途中への節の見出しを落とし、読む口の移動だけを通す（Q2）

`tools/check-shared-appends.sh`（`tail_touched()` と `check()`、`--self-test`）。design D2。

- [ ] 1.1 いまの self-test が通ることを確かめてから直す。検証: `tools/check-shared-appends.sh --self-test` rc=0（直す前）
- [ ] 1.2 足した行が Story の節の見出し（`^# *[=-]{3,}.*ST[0-9]{2}` / `^echo "== (Scenario: )?ST[0-9]{2}`）なら本体のどこでも落とす。
  守る末尾を「読む口の見出しの直前の空でない行から読む口のかたまりの `done` まで」と「ファイルの最後の空でない行より後ろ」にする。読む口のかたまりを一字一句そのまま動かすだけの変更は通す。
  `--self-test` に D2 の 6 件（途中に `# ==== ST99` → 落とす / 途中に `echo "== ST99:` → 落とす / 読む口をそのまま前へ動かす → 通す / 動かした後に読む口の直後へ節 → 落とす /
  動かした後にファイルの最後へ書き足す → 落とす / 動かした後に戻しの並びの途中を直す → 通す）を足す。
  review/spec.md R2・R3 の 4 件（smoke.sh の番号つきの見出しを足す → 落とす / 既存の小見出しの文言を直す → 通す / `done` と最後の判定の行の間に 1 行足す → 落とす を動かす前と後の木で）も足す（D2 の計 10 件）。
  検証: `tools/check-shared-appends.sh --self-test` rc=0 で、出力の `self-test ok:` が 21 行以上（既存 11 件＋10 件）。`shellcheck tools/check-shared-appends.sh` rc=0
- [ ] 1.3 わざと壊して落ちることを見る: 1.2 の見出しの検査を一時的に外すと self-test が rc=1 になる（`sed` で検査の行を消した写しを `/tmp` に置いて `--self-test` を走らせる）。
  検証: 写しの `--self-test` rc=1 かつ出力に `self-test NG` がある。元の台本は変えない（写しを走らせる前と後で `git diff --stat` の出力が同じ）

## Task 2: ST17 の錠の節を `tools/check-immutable.d/` へ移し、読む口を戻しの前へ動かす（Q1・指摘 2）

`tools/check-immutable.sh`（ST17 の節を抜く・読む口を動かす・注記）、`tools/check-immutable.d/<作成時刻>_st17-feeling-locks.sh`（新規）、`tools/check-immutable.d/README.md`。design D1 / D3。

- [ ] 2.1 移す前の出力を控える（固定）。検証: `scripts/quiet-run immutable-before -- bash -c 'tools/check-immutable.sh > /tmp/ch01-imm-before.log 2>&1'` rc=0、
  `grep -c '^  OK' /tmp/ch01-imm-before.log` が 1 以上、`grep -c 'OK feeling' /tmp/ch01-imm-before.log` が 1 以上
- [ ] 2.2 読む口のかたまりを文言を変えずに `# --- 戻し手順が当たること（R117）` の直前へ動かす。ST17 の節（見出しから節の「戻し手順（D16）」の検査まで）を `*.d/` の 1 本へ移し、
  前の節の状態に依らない形にする（利用者の id と本人が書いた行は節の中で置く。主観の版が当たっていなければ NG）。`# Scenario:` の印と `echo` の文言は一字一句同じ。
  「戻しても台帳と登録簿の行と主観の記録が残る」ほか、`|| { NG; }` の次に無条件の OK がある検査を `if …; then OK; else NG; fi` にする（D3）。README と本体の注記の「末尾が読む」を直す。
  検証: `scripts/quiet-run immutable-after -- bash -c 'tools/check-immutable.sh > /tmp/ch01-imm-after.log 2>&1'` rc=0、
  `diff <(grep '^  OK' /tmp/ch01-imm-before.log | sort) <(grep '^  OK' /tmp/ch01-imm-after.log | sort)` が空、`grep -q '== 書き換え禁止の節: .*st17' /tmp/ch01-imm-after.log`、
  `! grep -n 'ST17: 主観の記録の錠' tools/check-immutable.sh`、`shellcheck tools/check-immutable.sh tools/check-immutable.d/*.sh` rc=0、
  `! grep -nE '^(fail|MIGS|ST03_UP|down_fail|AU)=' tools/check-immutable.d/*_st17-*.sh`（本体の変数を上書きしない）
- [ ] 2.3 NG の後に OK が並ばないことを、わざと壊して見る: 一時 worktree（`git worktree add /tmp/ch01-break HEAD`）で `migrations/202610141000_subjective_log.down.sql` を
  「`DROP TRIGGER feeling_reminder_append_only ON core.feeling_reminder; DELETE FROM core.feeling_reminder;`」に差し替えて `tools/check-immutable.sh` を走らせる
  （錠のトリガを先に外さないと DELETE が拒まれ、期待の NG が出ない。review/spec.md R4）。壊し方が効くことは、2.2 の前の木（`git stash` を使わず、2.1 の commit を worktree に取る）で同じ手順を走らせ、NG と OK が並ぶことで先に見る。検証: rc=1、出力に `NG 戻しで主観の行か台帳か登録簿の行が減った` があり、
  `OK feeling 主観が残っていれば、戻しても台帳と登録簿の行と主観の記録が残る` が無い。走らせた後に worktree を消す（`git worktree remove --force /tmp/ch01-break`）
- [ ] 2.4 ST17 の印が移っていることを見る。検証: `diff <(git show <2.1 の commit>:tools/check-immutable.sh | grep -o '# Scenario: .*' | sort -u) <(cat tools/check-immutable.sh tools/check-immutable.d/*.sh | grep -o '# Scenario: .*' | sort -u)` が空
  （`check_scenarios.py` の rc=0 は、この change の delta の印が揃う Task 9.1 で見る）。
  `tools/check-shared-appends.sh "$(git branch --show-current)"` rc=0（Task 1 の規則で、読む口の移動と ST17 の節を抜いたことが通る）

## Task 3: 録画の環境のトークンを追跡されないファイルから渡す（指摘 3）

`tools/record-env.sh`、`collector-android/app/build.gradle.kts`。design D4。

- [ ] 3.1 `build.gradle.kts` は `ashiato.apiToken` / `ashiato.userId` が property に無ければ `collector-android/local.properties` から読む。`record-env.sh` は `local.properties` に書き、
  `gradle.properties` に書かない・戻さない。検証: `git check-ignore -q collector-android/local.properties`、`! grep -n 'gradle.properties' tools/record-env.sh | grep -v '^\s*#'`（`>>…gradle.properties` と `git checkout -- …gradle.properties` が無い）、
  `bash -n tools/record-env.sh`、`shellcheck tools/record-env.sh` rc=0
- [ ] 3.2 local.properties から読んだ値が APK に入ることを見る。検証: 一時の `local.properties` に `ashiato.apiToken=ch01-probe` を足して
  `scripts/quiet-run gradle-bc -- ./collector-android/gradlew -p collector-android :app:generateDebugBuildConfig` rc=0、
  `grep -rq 'ch01-probe' collector-android/app/build/generated/source/buildConfig/debug`。走らせた後に足した行を消す（`! grep -q ch01-probe collector-android/local.properties`）
- [ ] 3.3 ビルドが KILL で止まってもトークンが追跡ファイルに残らないことを見る: 一時 worktree で `collector-android/gradlew` を「印のファイル（`/tmp/ch01-gradle-started`）を置いてから `sleep 300`」の stub に差し替え、
  `record-env.sh` の Android の段（`REC_PLATFORMS=android`）を後ろで走らせ、**印のファイルが現れてから**そのプロセスグループへ `kill -9` を送る（cargo と npm のビルドの最中に止めない。review/spec.md R6）。
  壊し方が効くことは、直す前の木で同じ手順を走らせ、`collector-android/gradle.properties` にトークンが残る（下の検証が落ちる）ことで先に見る。
  検証: `git -C <一時 worktree> status --porcelain -- collector-android/gradle.properties` が空、`git -C <一時 worktree> ls-files --others --exclude-standard` にトークンを持つファイルが無い

## Task 4: 取り込みの「人が書いた記録」の規則を 1 つにまとめる（Q7）

`crates/server/src/lib.rs`（主張・主観・場所の 3 か所）と、共通の関数の置き場（`crates/server/src/ingest.rs` か新しいモジュール）。試験は `crates/server/src/authored_rule_tests.rs`（新規）。design D7。

- [ ] 4.1 まとめる前に、いまの拒み方を試験で固定する（直す前に通る）。主張と場所は `personal-entities` の delta の Scenario、主観は順の組み合わせ（形が崩れて由来も違う → `malformed_feeling` /
  地域が +9:00 でなく由来も違う → `malformed_feeling` / 由来も外部識別子も違う → `feeling_not_authored`）。
  Scenario: `由来も外部識別子も違う主張は由来の理由で断る` / `形が崩れて由来も違う主張は形の理由で断る` / `地域のずれが範囲の外の主張は断る` / `地域のずれが範囲の端の主張は受け付ける` /
  `外部識別子を持ち地域のずれも範囲の外の主張は外部識別子の理由で断る` / `地域のずれが範囲の外で値も空の主張は地域の理由で断る` /
  `端末識別子と外部識別子を持つ場所の記録は由来の理由で断る` / `形が崩れて端末識別子も持つ場所の記録は形の理由で断る` / `地域のずれが範囲の外の場所の記録は断る` /
  `外部識別子を持ち地域のずれも範囲の外の場所の記録は外部識別子の理由で断る` / `地域のずれが範囲の外で名前も空の場所の記録は地域の理由で断る`。
  検証: 直す前の木で `CT ch01_authored_`（`cargo test -p ashiato-server ch01_authored_ -- --list` で 14 本以上 = Scenario 11 本＋主観の組み合わせ 3 本）
- [ ] 4.2 共通の関数にまとめ、3 か所から呼ぶ。主観の地域（`Asia/Tokyo` の +9:00）と、それを形と一緒に先に見る順は主観の側に残し、その理由（spec の決定・前提 7）をコメントに書く。
  検証: `CT ch01_authored_`、`CT st17_reject_`、`CT ingest_rejects_out_of_range_tz_offset`、`CT place_`（場所の既存の試験。`-- --list` で 1 本以上）、
  `cargo clippy -p ashiato-server --all-targets -- -D warnings` rc=0、`grep -c 'origin != "authored"' crates/server/src/lib.rs` が 0

## Task 5: 直さない 2 件の理由を残す（Q3・指摘 9）

`crates/server/src/feelings.rs`（紐づけ先の検査のコメントと、始まりの日の `min(event_time)` のコメント）、試験は `crates/server/src/feelings_reject_tests.rs` に足す、`docs/handoff/ST23.md`（既にある。末尾に 1 節足す）。design D9 / D10。

- [ ] 5.1 終わりの読めない滞在のいまの振る舞いを試験で固定する: 本文に終わりを持たない `s01-stay` の行を DB に直接置き、記録の区間が始まりの一点を含めば受け付け、
  始まりより後から始まる記録は `invalid_feeling_target` で断る。コメントを D9 の文に直す（「消した滞在も通す」を正確にする）。
  検証: `CT ch01_stay_open_`（`-- --list` で 2 本以上）、`grep -n 'span_of' crates/server/src/feelings.rs` が 1 行以上
- [ ] 5.2 `min(event_time)` の上に、索引 `event_by_source_time` に乗ることと測った値（`premises.md` 前提 9）をコメントで残す。
  検証: `grep -n 'event_by_source_time' crates/server/src/feelings.rs` が 1 行以上
- [ ] 5.3 `docs/handoff/ST23.md` の末尾に 1 節足す（既存の 10 節は消さない。`docs/handoff/README.md` の書式。見出しは `## ch01-st17-review-fixes D9 — 本文を消した滞在を紐づけ先にする気分`。
  いまの読み方・断られる例・Q3 の「直すなら」の候補の規則）。検証: `python3 scripts/story_brief.py ST23 | grep -q 'ch01-st17-review-fixes'`、`grep -c '^## ' docs/handoff/ST23.md` が 11

## Task 6: 主観の画面の主観の読み出しを 1 回にまとめる（Q6）

`web/src/FeelView.tsx`（日の読み出しと 30 日の読み出し）、試験は `web/src/__tests__/FeelView.test.tsx`。design D8。

- [ ] 6.1 30 日の 1 回を読み、日の欄・暦・滞在の行をその応答から出す。押して受理された後の読み直しも 1 回。読み出しが失敗したら日の欄と暦の両方が失敗を出す。
  Scenario: `読み出しの失敗は日の欄にも出る`。
  検証: `VT FeelView.test.tsx`（`/api/feelings` への fetch が、開いて 1 回・快–不快を押して受理された後に 1 回だけ増えることを数える試験と、失敗の試験を含む）、`ET feel-ch01.spec.ts`（`読み出しの失敗は日の欄にも出る` の印。`page.route` で `/api/feelings` を落とす）、
  `cd web && npm run typecheck && npm run lint` rc=0
- [ ] 6.2 ST17 の画面の e2e がそのまま通ることを確かめる（振る舞いを変えない）。検証: `ET feel.spec.ts`、`ET feel-parts.spec.ts`、`ET feel-limits.spec.ts`

## Task 7: 滞在の行は断られたら押された印を戻し、届かなかったら残す（Q4）

`web/src/FeelView.tsx`（`pressStay` と `post` の断られた・届かなかったの枝）、jsdom は `web/src/__tests__/FeelView.test.tsx`、e2e は `web/e2e/feel-ch01.spec.ts`（新規）。design D5。

- [ ] 7.1 断られたら、その行の押された印を画面が最後に読んだその滞在の最新の記録の値（無ければ押されていない）に戻す。届かなかったら残す。日の欄（`pick`）の振る舞いは変えない。
  Scenario: `滞在の行で断られた値は押された状態に残らない` / `記録の無い滞在の行で断られるとどれも押されていない` / `滞在の行で届かなかった値は押された状態に残る`。
  検証: `VT FeelView.test.tsx`（3 本の印を含む）、`ET feel-ch01.spec.ts`（3 本の印。断られる応答は `page.route` で `/api/ingest` に 1 件ごとの結果つきの拒否を返し、届かないのは `route.abort`）
- [ ] 7.2 日の欄が今までどおり断られても値を残すことを確かめる。検証: `ET feel.spec.ts`（`届かなかったとき選んだ値とひとことが残る` と `断られたとき理由の文が出る` を含む）

## Task 8: 日の欄は送るたびに前後を決め直し、日をまたいでいたら知らせる（Q5）

`web/src/FeelView.tsx`（`submit` と受理の枝、`now`）、`web/src/feelings.ts` か画面の文言（「振り返りとして残しました」）、jsdom は `web/src/__tests__/FeelView.test.tsx`、e2e は `web/e2e/feel-ch01.spec.ts`。design D6。

- [ ] 8.1 押したときに記入の日時で前/後を決め直し、画面が「前」を出していて記入の日時が「後」なら、受理の後に画面の「いま」を記入の日時に進めて読み直し、保存の文に「振り返りとして残しました」を添える。
  知らせは「押したときに画面が『前』を出していて、記入の日時の前/後が『後』」のときだけ。送り直しが届いたときは出さない（D6）。
  Scenario: `開いたまま日の終わりをまたいで押した記録は後として格納される` / `開いたまま日の終わりをまたいで押すと振り返りとして残したと出る` /
  `日の終わりをまたがずに押すと振り返りの知らせは出ない` / `日をまたいでから届いた送り直しは振り返りの知らせを出さない`。
  検証: `VT FeelView.test.tsx`（4 本の印）、`ET feel-ch01.spec.ts`（4 本の印。`page.clock.install` で対象の日の `23:58+09:00` に開き、`page.clock.setSystemTime` で翌日 `00:01+09:00` / 同じ日 `23:59+09:00` に進めて押す。送り直しは `page.route` で 1 回目を `route.abort` にする。
  格納された記録の前/後は `/api/feelings` の応答で見る）

## Task 9: 全体の検査と文書

- [ ] 9.1 文書と検査。`docs/changes/CH01.md` の「完了の判定」の各行が満たされていることを見る。
  検証: `openspec validate ch01-st17-review-fixes --strict` rc=0、`python3 scripts/check_scenarios.py . ch01-st17-review-fixes` rc=0、`tools/check-log-private.sh` rc=0、
  `cargo fmt --all --check` rc=0、`tools/check-openapi.sh` rc=0 かつ `git diff --quiet origin/main -- docs/openapi.json`（API の契約が変わらない）、
  `tools/check-shared-appends.sh "$(git branch --show-current)"` rc=0、`cd web && npm run build` rc=0
