# ST54 spec-review（成果物どうしの整合の独立検査。mode: spec --draft）

対象: `proposal.md` / `specs/distribution/spec.md` / `design.md` / `tasks.md` / `docs/stories/ST54.md`。
照らしたもの: `deep.md`（Q1〜Q10 は答え待ち。推奨で書いた版）・`deep-questions.json`・`premises.md`・`provisional.jsonl`（D5・D7・D8）・`review/deep.md`・`docs/handoff/ST54.md`・
正典 `openspec/specs/{distribution,record-envelope,desktop-collection,device-collection}/spec.md`・`openspec/changes/archive/2026-11-02-st52-release-channels`・実コード（`crates/server/src/migration_ledger.rs` ほか）。

## 機械の検査（2026-11-02 この席で実行）

```
$ openspec validate st54-dev-side-by-side --strict
Change 'st54-dev-side-by-side' is valid                          rc=0

$ python3 scripts/check_chain.py .
  要件 171 件 / Story 58 本 / 扉 35 項
  [ok]   どれかの Story に拾われた要件: 169/171 件
  [ok]   INDEX の「Story の対象外」: NFR-11, NFR-8
chain: OK (0 件 / 未回収 0 件 / warn 0 件)

$ python3 scripts/check_scenarios.py . st54-dev-side-by-side
  [warn] spec に無い Scenario を指す印 … 8 件（どれも ST54 の外: tools/check-shared-appends.sh・tools/check-tmp-hygiene.sh・
         tools/release/build.sh・tools/st12_delta_diff.py・tools/tmp-leak-check.sh・web/src/__tests__/satellites.test.tsx）
  [FAIL] 担保の無い Scenario … 50 件（すべて st54-dev-side-by-side/specs/distribution/spec.md。実装前なので想定どおり）
scenarios: FAIL (担保なし 50 件 / 名無しの確認待ち 0 件)
  → 50 件の名前は全部 tasks.md のどこかが名指ししている（grep -F で 1 件ずつ確かめ、漏れ 0）

$ python3 scripts/review_triage.py . st54-dev-side-by-side
  指摘 12 件 / 仮決め 1 件 / 要件へ戻すもの 0 件
triage: OK

$ python3 scripts/spec_overlap.py candidates . st54-dev-side-by-side
[]
$ python3 scripts/spec_overlap.py check . st54-dev-side-by-side
{"ok": true, "candidates": 0}

$ python3 scripts/evidence.py lint openspec/changes/st54-dev-side-by-side
[OK] 検証コマンド（st54-dev-side-by-side）                        rc=0
```

Scenario の数: delta は 77 本（MODIFIED 10・名前を変えて移した 2 つで 23・新しい 10 の Requirement で 44）。tasks.md の Global Constraints の「77 本 / 新しく印を置く 50 本」と合う。

## 書いた側が気にしていた点への答え（要約。指摘は下の R）

- **REMOVED + 名前を変えた ADDED の写し**: 正典 `distribution/spec.md:625-693`・`:772-834`・`:724-770` と delta を `diff` で突き合わせた。写しの漏れは無い。
  変わったのは Migration に書いた条項・Scenario と、導出元の行だけ。ただし Migration の「2 つの条項」は、実際には 3 行（4 つを並べる・配布物の名前の括弧「開発版に並べるのは ST54 から」を外した・数える規則）が変わっている（R15）
- **D3 と実コード**: 成り立つ。`migration_ledger.applied` は `applied_at`・`build_version` を持ち（`migration_ledger.sql`）、`build_version` に入るのは `BUILD_VERSION = env!("ASHIATO_VERSION")`（`migration_ledger.rs:17`）で、
  これは `crates/server/build.rs:36-47` が `ashiato_build_stamp::version()` の返す**渡した版番号**（`ASHIATO_BUILD_VERSION`。`-dev.` を含みうる）で出す。`Unreadable` が列の集合を固定しているので、最初の行の版番号は読める。
  既存の bin の試験（`st52_bin_*`）が組む版は `-beta.2` で、`-dev.` の行を作る既存の経路は無い。精度の抜けは R10
  （なお `migration_ledger.sql` の列の注記「CARGO_PKG_VERSION」は古いが、振る舞いには効かない）
- **D4・D5（仮）と Q4**: D4 の `renamed` / `removed` は Q4 の推奨の detail（「旧名の対応を main に残し」「直しの移行を足して開発版の DB にも当てる」）に忠実。
  D5（仮）は推奨の「中身の直しは直しの移行を足す」を機械で止めるもので、範囲は越えていない（選択肢 3 の凍結とは違い、A の中身は変えてよい）。P の条件も満たす。ただし抜けがある（R11）
- **D7（仮）の逆向き**: P の条件（失わない側 = 送らず手元に残す・費用は比べ 1 つ・FR-106「取り合わない」の範囲内・要件を変えない）は満たす。
  ただし「正式版の収集アプリの振る舞いを書き換える Requirement は無い」という能力の照合の前提を崩し、正典 3 つと食い違う（R1・R3）
- **能力の照合**: `distribution` だけでは足りない（R1）
- **tasks の検証コマンド**: 道具（`scripts/quiet-run`・`tools/check-openapi.sh`・`check-guards.sh workflows`・`check-migrations.sh --self-test`・`build.sh --self-test`・`draft.sh --self-test`・`sign-apk.sh --self-test`）と crate 名（`ashiato-build-stamp`・`ashiato-server`・`ashiato-collector-windows`）は実在する。
  `check-artifacts.sh --self-test-android` は今は無く（今あるのは `--self-test-gradle-range`）、Task 9.1 が足すもの。書いたまま走らないのは R13（版番号）と、Task 2.1 の `st50_`（R1）

---

## R1. `GET /contract` の答えに `channel` を足すことが、正典 record-envelope の「答えの形」と、desktop-collection / device-collection の送る契機の規則を書き換えるのに、delta は `distribution` にしか無い
- 成果物: openspec/changes/st54-dev-side-by-side/proposal.md・specs/distribution/spec.md・tasks.md
- 根拠: `openspec/specs/record-envelope/spec.md:1055`（「答えは `{"contract_version": <整数>}` の形の 1 つ」。`:1058` の凍結は配った印 `v*` の後なので、印がまだ無い今は効かないが、本文と Scenario の形とは食い違う）・`:1064-1067` の Scenario「問い合わせの口は受け付ける一番新しい契約の版を答える」（THEN「200 で `{"contract_version": 3}` が返る」）/
  その担保 `crates/server/src/contract_version_tests.rs:56-67` `st50_contract_answers_version` は `assert_eq!(body, serde_json::json!({ "contract_version": 3 }))`（本文の完全一致）/
  `openspec/specs/desktop-collection/spec.md:1006-1013`・`openspec/specs/device-collection/spec.md:2363-2376`（送る契機で行を送らずに残す理由を列挙している。「系統が違う」はどちらにも無い）/
  delta `specs/distribution/spec.md:302-305`（答えに系統を載せ、違えばどの行も送らない。正式版の収集アプリも止まる）/
  proposal.md の「capability の照合」（「正式版の振る舞いを書き換える Requirement は無い」の根拠の grep は `ashiato-collector.cmd|ashiato_session|18787` だけで、契約の答えを見ていない）/
  tasks.md 2.1 の検証 `cargo test -p ashiato-server st50_` rc=0 は、`channel` を足した時点で `st50_contract_answers_version` が落ちるので、ST50 の試験を書き換えずには通らない。書き換えれば、正典 record-envelope の Scenario の文と試験が食い違う
- kind: technical
- 提案: どちらかを選ぶ。(a) record-envelope の「S-01 は受け付ける送信契約の版を、問い合わせの口で答える」を MODIFIED にし（答えは `contract_version` と `channel` を持つ。Scenario の THEN は「`contract_version` が 3」）、desktop-collection・device-collection の契約の版の Requirement に「系統が違えば送らない」を MODIFIED で足し、INDEX.md の capability 表に ST54 を足して前倒しの理由を書く。
  (b) 系統を `GET /contract` の外（別の口・見出し）で運び、record-envelope の答えの形に触れない。どちらでも、tasks 2.1 に ST50 の試験をどう扱うかを書く
- 処置: fixed specs/distribution/spec.md —— 案 (b) を採った。系統を `/contract` の本文の欄でなく応答の見出し `Ashiato-Channel` で運び、本文 `{"contract_version": <整数>}` と ST50 の試験は変えない（tasks 2.1 の `st50_` は書き換えずに通る）。収集アプリ側の「送らずに残す」理由は、正典の列挙が閉じていないので distribution の Requirement が足しても食い違わないことを proposal の capability の照合に書いた。design D7・tasks 7.1・9.2 を見出しに合わせた

## R2. 「系統が同じサーバには、今どおり送る」の開発版の側（開発版の収集アプリ → 開発版のサーバ）に Scenario が無い。開発版が一切送らない実装でも全部の Scenario が通る
- 成果物: openspec/changes/st54-dev-side-by-side/specs/distribution/spec.md
- 根拠: `specs/distribution/spec.md:305`（Requirement の本文「系統が同じサーバには、今どおり送る」）/ 同じ Requirement の 7 本（`:308-341`）のうち、開発版の収集アプリが出てくる 4 本はどれも「送らない」側で、送る側は `:328-331` の正式版 → 古いサーバだけ /
  tasks.md 7.1・9.2 の試験の一覧にも、開発版 → 開発版で送る試験が無い
- kind: technical
- 提案: 「開発版の PC の収集アプリは、開発版のサーバには今どおり送る」（`channel: dev` を答える偽のサーバで、未送信 3 件が送られ 0 件になる）と、携帯の同じ形の Scenario を足し、tasks 7.1・9.2 に名指しする
- 処置: fixed specs/distribution/spec.md —— 「開発版の PC の収集アプリは、開発版のサーバには今どおり送る」と携帯の同じ形を足し、tasks 7.1・9.2 で名指しした

## R3. D7 の「違う系統の答えは契約の版の待ち（ST50 の『サーバを更新してください』の通知）に数えない」は観測できる振る舞いなのに、design と tasks にしか無い
- 成果物: openspec/changes/st54-dev-side-by-side/design.md
- 根拠: `design.md:103`（D7「違う系統の答えは、契約の版の待ち … に数えない」）・tasks.md 7.1・9.2（同じ）/ `openspec/specs/desktop-collection/spec.md:1061-1064`（「S-01 の更新を待っている」の定義と通知）/ delta に「通知」「更新を待っている」の語は無い（`grep -n "通知\|更新を待" specs/distribution/spec.md` は自動起動・通知の差出人の行だけ）
- kind: technical
- 提案: 系統が違う間は「サーバを更新してください」を出さないことを Scenario にする（R1 で desktop-collection を MODIFIED にするなら、そちらの「更新を待っている」の定義に入れる）。`openspec archive` は design を正典に写さない
- 処置: fixed specs/distribution/spec.md —— 本文に「系統の違いを契約の版で止めていることに数えない」を足し、Scenario「系統が違って送らない間は、サーバを更新してくださいの通知を出さない」を足した（tasks 7.1）

## R4. 送らないものの一覧が spec と design / tasks で違う。携帯の「写真の欠け（原本の欠け）」が spec に無く、携帯の Scenario はログを見ない
- 成果物: openspec/changes/st54-dev-side-by-side/specs/distribution/spec.md
- 根拠: `specs/distribution/spec.md:303`（「記録・生存信号・破棄の報告・写真の原本のどれも送らず」）・`:336`・`:341`（同じ 4 つ）/ `design.md:104` と tasks.md 9.2（5 つの送り手: 記録・生存信号・破棄の報告・写真の原本・**写真の欠け**）/
  正典 `device-collection/spec.md:2360`（C-01 が積む行は「記録・生存信号・破棄の報告・原本の欠け・原本の送り待ち」）/ Requirement の本文 `:303` は「送らなかった理由をログに残す」が、携帯の 2 本（`:333-341`）の THEN にログが無い（PC の逆向き `:323-326` も無い）
- kind: technical
- 提案: Requirement と携帯の 2 本の THEN に「原本の欠け」を足す（正典 device-collection の語に合わせる）。ログの主張は、携帯と逆向きの PC の Scenario にも THEN で入れるか、本文から外す
- 処置: fixed specs/distribution/spec.md —— 本文と携帯の Scenario を「記録・生存信号・破棄の報告・原本の欠け・原本」にそろえ、携帯の 2 本と PC の逆向きの THEN にログを入れた。design D7・tasks 9.2 も同じ語にした

## R5. Q2 の推奨「除外の登録は**正式版の置き場**のものを読む」が、spec では「**渡された置き場**のものを読む」になり、別の置き場を渡したときは正式版の除外が効かない
- 成果物: openspec/changes/st54-dev-side-by-side/specs/distribution/spec.md・design.md
- 根拠: deep-questions.json Q2 の選択肢 1（推奨）「除外の登録は正式版の置き場のものを読む」・detail「正式版で登録した除外が開発版にもそのまま効く」/
  `specs/distribution/spec.md:269`（「渡された置き場の `exclusions.json` から読む（正式版と同じ置き場を渡せば、正式版で登録した除外がそのまま効く）」）・`design.md:41` / premises P5（置き場は既定を持たず本人が渡す）/
  review/deep.md R2（除外が効かないと除外した題名・URL が開発版の D-01 に入る。loss: exported）
- kind: conflict
- 提案: 開発版は正式版の置き場を自分で知る手段が無い（P5）ので、読み替え自体は避けられない。Q2 はまだ答え待ちなので、まず deep-questions.json の Q2 の選択肢 1 の label と detail を「除外の登録は渡された置き場のものを読む（正式版と同じ値を渡せば正式版のもの）」に直し、本人が承ける文と spec を同じにする。あわせて D2 にこの読み替え（「正式版の置き場」=「渡された置き場」）と、別の値を渡すと除外が効かないことを明記し、GO の資料で本人が見えるようにする。
  失う側を塞ぐなら、開発版が `<渡された置き場>/exclusions.json` を持たない置き場で起動したときに通知かログで知らせる（仮で足せる）
- 処置: escalated —— Q2 の context に「推奨の『正式版の置き場』は開発版に渡した置き場を指す。別の値を渡すと正式版の除外は効かない」を足し、Q2 として本人に渡す（選択肢の文面は推奨の答えの印とずれないよう変えない）。design D2 に読み替えと理由（P5）を書いた。deep.md の「問いの出所」に R5 を足した

## R6. D2 の「自動起動の `.cmd` には、`dev` を掘る前の置き場と `-dev` を付ける前の端末識別子を書く」は観測できる振る舞いなのに spec に無い。間違えると次のログオンごとに `dev/dev`・`-dev`（付けない規則で止まる）になる
- 成果物: openspec/changes/st54-dev-side-by-side/design.md
- 根拠: `design.md:43`（D2 の 1 つ目の箇条）・tasks.md 6.1（同じ文。試験の Scenario の一覧に、この振る舞いの名は無い）/ delta の PC の置き場の Requirement（`specs/distribution/spec.md:266-298`）は `.cmd` の中身に触れない
- kind: technical
- 提案: Scenario を足す:「開発版の自動起動の項目は、渡された置き場と端末識別子をそのまま書く」（`<使い捨ての置き場>` と `pc-01` を渡して仕込み、`ashiato-collector-dev.cmd` の中の値が `<使い捨ての置き場>` と `pc-01`）。tasks 6.1 で名指しする
- 処置: fixed specs/distribution/spec.md —— 本文に `.cmd` へ渡された値のまま書く条項と、Scenario「開発版の自動起動の項目は、渡された置き場と端末識別子をそのまま書く」を足した（tasks 6.1）

## R7. THEN の一部（`/healthz`・「収集を始め」・「別の置き場の未送信」・除外の読み込み口）を、tasks が観測する手段を書いていない
- 成果物: openspec/changes/st54-dev-side-by-side/tasks.md
- 根拠: Scenario「空の DB に開発版の migrate を当てると、開発版の DB になる」（`spec.md:369-372`。THEN に `/healthz` が 200）は tasks 3.1 で「`st54_tests.rs`。関数の段」/
  「名前を変えた移行は、旧名の対応があれば…」「消した移行は…」（`spec.md:399-407`。THEN に `/healthz` 200）は tasks 4.1 で「`st54_tests.rs`。LINEAGE は試験が差し替えた表を渡す」（待ち受けを開かない段）/
  「正式版と同じ置き場を渡しても、開発版と正式版は両方起動する」（`spec.md:295-298`。THEN「収集を始め、2 つの未送信は別の置き場にある」）は tasks 6.1 で「二重起動の印を…両方取れる」だけ /
  「開発版は、渡された置き場の除外の登録を読む」（`spec.md:290-293`）は tasks 6.1 で「`Engine` に題名…を通し」—— 設定の `exclusions_path` から読む口（`main.rs:81` の置き換え）を通るかが書かれていない
- kind: technical
- 提案: `/healthz` を含む 3 本は、`server_startup.rs` の `st54_bin_` に置くか、`st54_tests.rs` の中で起動の検査（`startup_refusal`）を通した後に router で `/healthz` を引く（`contract_version_tests.rs` の `tee_app` の形）と tasks 3.1・4.1 に書く。THEN を「起動の検査が止めない」に直してもよい。二重起動の 1 本は THEN を試験が見るもの（印と未送信の置き場の道）に合わせる。除外の 1 本は、置き場から読み込む関数を通すと tasks 6.1 に書く
- 処置: fixed tasks.md —— 3.1・4.1 に `/healthz` を起動の検査の後に router で引く見方を書いた。二重起動の Scenario の THEN を印と未送信の道に合わせ、6.1 に除外を設定の置き場から読み込む関数を通すと書いた

## R8. 「開発版のサーバの画面のログインは、正式版のクッキーを受け付けない」は、クッキーの名前を分けなくても通る
- 成果物: openspec/changes/st54-dev-side-by-side/specs/distribution/spec.md
- 根拠: `spec.md:486-489`（WHEN `ashiato_session=<正式版のサーバで得た値>`、THEN 401）/ ログインの印は DB の表に置く（`crates/server/src/web_session.rs:347` `INSERT INTO core.web_session`）。正式版と開発版は別の D-01（D3 が同じ DB を拒む）なので、正式版で得た値は名前に依らず開発版の DB に無く 401 になる
- kind: technical
- 提案: 名前の分離を見る形にする:「開発版のサーバで得た値を `ashiato_session` の名前で付けると 401、`ashiato_session_dev` で付けると 200」。P17 が守りたいのは同じ host での上書きなので、「開発版のログインの応答の Set-Cookie は `ashiato_session` を書き換えない」でもよい
- 処置: fixed specs/distribution/spec.md —— Scenario を「開発版のサーバの画面のログインは、開発版の名前のクッキーだけを使う」（名前で 200 / 401 が分かれ、Set-Cookie は `ashiato_session` を書かない）に替えた

## R9. Requirement の本文だけが言い、Scenario が言っていないこと
- 成果物: openspec/changes/st54-dev-side-by-side/specs/distribution/spec.md
- 根拠:
  - `spec.md:474`「`BIND`・`MIGRATE_BACKUP_DIR` を明示で渡したときは、系統に依らずその値を使う」—— 3 本（`:478-489`）のどれも明示の値を渡さない
  - `spec.md:248`「開発版の自動起動を仕込む操作で、正式版の…を書き換えない（逆も同じ）」—— `:257-260` は正式版 → 開発版の順だけ
  - `spec.md:493`「決まった文の 1 行目は…で始まり、開発版は正式版と別のアプリとして並べて入ること・開発版のサーバは正式版と別の DB に繋ぐことを書く」—— Scenario は 1 行目だけを見て、`design.md:130` は「2 行目以降は文書として直してよい」と言う（本文の SHALL と design が食い違う）
  - `spec.md:209`「系統を実行時の設定で変える口を持たない」—— Scenario なし
- kind: technical
- 提案: 明示の `BIND` / `MIGRATE_BACKUP_DIR` を渡した開発版の Scenario を 1 本足す。自動起動は逆順（開発版 → 正式版）も THEN に入れる。説明文は、本文の SHALL を 1 行目に絞るか、2 行目以降の 2 つの主張を Scenario に入れるかを決める。実行時の口の不在は、`ASHIATO_CHANNEL` などの名前を渡しても系統が変わらない Scenario で書けるなら書き、書けないなら本文から外す
- 処置: fixed specs/distribution/spec.md —— 明示の `BIND`・`MIGRATE_BACKUP_DIR` の Scenario、自動起動の逆順の THEN、実行時の環境変数で系統が変わらない Scenario を足した。説明文の SHALL は 1 行目に絞り、2 行目以降は文書として直してよいと本文に書いた（design D9 と一致）

## R10. D3 の「ashiato の表を持つ / 1 つも持たない」は、実コードの判定（`core` の schema の有無）と言葉が違う。行が 0 の場合と、更新の履歴を書かないことの置き場が tasks に無い
- 成果物: openspec/changes/st54-dev-side-by-side/specs/distribution/spec.md・tasks.md
- 根拠: `specs/distribution/spec.md:346`（「ashiato の表を持つもの」「表を 1 つも持たない空の D-01」）/ `crates/server/src/migration_ledger.rs:250-256`（`BeforeLedger` と `Empty` の分かれ目は `schema_exists(pool, "core")`）・`:163-172`（`Ledger::Recorded` は名前の集合だけを持ち、0 行でも `Recorded`）/
  `:421`（`record_result` は `Ahead | Unreadable` のときだけ更新の履歴を書かずに返す。新しい `ChannelMismatch` はここに足さないと結果の行を書く）/ `spec.md:347`（「更新の履歴の行も書かずに」）/ tasks.md 3.1 は「更新の履歴より前」とだけ書き、`record_result` に触れない /
  「版の記録はあるが行が 0 で ashiato の表を持つ」場合の Scenario は無い（`:374-377` は schema が無い場合だけ）
- kind: technical
- 提案: 本文の判定の言葉を観測できるもの（「`core` の schema がある」など、試験が作れる条件）にそろえる。tasks 3.1 に `record_result` の早い返りへ `ChannelMismatch` を足すことを書く（Scenario `:354-357` の「更新の履歴の行の数は同じ」がこれを見る）。行が 0 の場合の扱いを Scenario にするか、本文から外す
- 処置: fixed specs/distribution/spec.md —— 判定の言葉を `core` の schema の有無にそろえ、版の記録の行が 0 の DB を系統なしにして Scenario を足した。tasks 3.1 に `record_result` の早い返りへ `ChannelMismatch` を足すことを書いた

## R11. D5（仮）の検査は「名前を変え、中身も変えた」移行を `renamed` 1 行だけで通しうる。サーバも指紋の違いを警告するだけなので、開発版の DB は黙ってずれる
- 成果物: openspec/changes/st54-dev-side-by-side/specs/distribution/spec.md・design.md
- 根拠: `spec.md:421`（名前の変更は `renamed` があれば通す）・`:422`（中身の変更は「印 `nightly` の版にある移行の中身」で、名前を変えた後の比べ先が決まっていない）/ `design.md:64`（D4: `renamed` の行の指紋は今の名前の中身と比べ、違えば今どおり**警告**）/ `design.md:79`（D5 の落とす条件は 2 つを別々に見る）/
  Scenario「開発版に載った移行の名前を変えても、旧名の対応があれば通る」（`spec.md:435-438`）は中身を変えない場合だけ / Q4 の推奨の detail「中身の直しは直しの移行を足して開発版の DB にも当てる」/
  ほか: tasks.md 5.1 の「浅い木では印を見る前に rc≠0（履歴が浅い）」は spec に無い（design D5 にも無い）
- kind: technical
- 提案: 本文に「`renamed A B` のとき、`B` の中身を印 `nightly` の版の `A` の中身と比べ、違えば `rewritten` を求める」を足し、Scenario「名前を変え中身も変えた移行は、`renamed` だけでは落ちる」を足す。浅い木の止めは Scenario にするか、tasks から外す。
  （注: この検査はコメントや空白の違いでも `rewritten` を求める。配った印の凍結（ST48）と同じ粒度なので矛盾はしないが、D5 の費用に書いておくとよい）
- 処置: fixed specs/distribution/spec.md —— `renamed` の版は今の名前の中身を `nightly` の旧名の中身と比べる条項と Scenario「名前を変えて中身も変えた移行は、旧名の対応だけでは落ちる」、浅い木の Scenario を足した。D5（仮）の費用に粒度を書いた

## R12. D10 / tasks 10.4 の `draft.sh --channel nightly` は、正典「手で起こした確かめの組み立ては、下書きのリリースにだけ置き、印を作らない」の振る舞いを広げるのに、Scenario が無い
- 成果物: openspec/changes/st54-dev-side-by-side/design.md・tasks.md
- 根拠: `openspec/specs/distribution/spec.md:836-852`（下書きは「PR の組み立ての配布物を使う」・「同じリリースの種類（正式版・先行公開）」）/ `design.md:140`（D10: 手で起こした `nightly.yml` の run の配布物を落とし、使い捨ての開発版の鍵で署名し、`--notes-file` 付きの下書きにする）/
  tasks.md 10.4 の `--self-test` に足す開発版の場合には Scenario の名が無い（`check_scenarios.py` が守れない）/ この下書きは Q9 の推奨の「完了の判定に足す 3 つ」の唯一の確かめ（tasks 12.1）
- kind: technical
- 提案: 正典の下書きの Requirement を MODIFIED にして「開発版の下書きは、手で起こした開発版の作り直しの配布物を開発版の鍵で署名して置き、印を作らず、説明文に決まった文を使う」を足し、Scenario「開発版の確かめの組み立ては、印を作らずに下書きのリリースになる」を足す。tasks 10.4 で名指しする
- 処置: fixed specs/distribution/spec.md —— 「手で起こした確かめの組み立ては、下書きのリリースにだけ置き、印を作らない」を MODIFIED にして開発版の下書きの条項と Scenario を足し、tasks 10.4 で名指しした

## R13. tasks 9.1・9.3 の版番号 `0.4.0` / `0.4.0-dev.20261102` では、今の出所の版（`0.1.0`）で gradle が「版番号の出所が一致しない」で落ちる
- 成果物: openspec/changes/st54-dev-side-by-side/tasks.md
- 根拠: `Cargo.toml:6` `version = "0.1.0"`（出所の版）/ `collector-android/app/build.gradle.kts:50-58`（渡した版番号の MAJOR.MINOR.PATCH が出所と違えば `GradleException("版番号の出所が一致しない…")`）/
  tasks.md 9.1「`--self-test-android`（gradle で `0.4.0-dev.20261102` と `0.4.0` の debug を組む）」・9.3「版番号 `0.4.0` の debug（正式版）を入れて」（同じ 9.3 の開発版は `<出所>-dev.20261102` と書き分けている）/ 比べ: tasks 共通の bin の試験は `<出所>-dev.20261102`、既存の `range_self_test`（`check-artifacts.sh:112-126`）は clone の `Cargo.toml` を書き換えてから組む
- kind: technical
- 提案: 9.1・9.3 の版番号を `<出所>` と `<出所>-dev.20261102`（出所は `Cargo.toml` から読む）に直すか、`range_self_test` と同じく使い捨ての clone の版を `0.4.0` に書き換えてから組むと書く。Scenario の `0.4.0` は例の値なので、試験がどちらで組むかを tasks に書けば足りる
- 処置: fixed tasks.md —— 9.1・9.3 の版番号を `<出所>` と `<出所>-dev.20261102`（`version.sh source`）にした

## R14. 人間の確認待ち 12.1 が、Q9 の推奨で完了の判定に足す 3 つのうち「開発版の鍵で署名されている」を見る終了条件を持たない
- 成果物: openspec/changes/st54-dev-side-by-side/tasks.md
- 根拠: deep-questions.json Q9 の question（「携帯の APK が並ぶこと・開発版の鍵で署名されていること・『サポートしない』の文」）/ `design.md:140`（D10 の 4 つ目の箇条）/ tasks.md 12.1 が見るのは「`git ls-remote --tags origin` の行が同じ」「4 つの配布物」「説明文の 1 行目」の 3 つで、署名の指紋を見る手順が無い
- kind: technical
- 提案: 12.1 に「下書きの `ashiato-collector-dev-<版番号>.apk` を落とし、`apksigner verify --print-certs` の SHA-256 が使い捨ての開発版の鍵の指紋と一致し、識別子が `dev.ashiato.collector.dev`」を足す
- 処置: fixed tasks.md —— 12.1 に `apksigner verify --print-certs` の指紋と識別子を見る終了条件を足した

## R15. REMOVED の Migration が変えた所を言い尽くしていない。移した先の導出元が、外した例外の本人の答えを根拠に挙げたまま
- 成果物: openspec/changes/st54-dev-side-by-side/specs/distribution/spec.md
- 根拠: 正典 `distribution/spec.md:625-693` と delta `:64-139` の `diff`（変わった本文は 3 行: 4 つを並べる条項・配布物の名前の括弧から「開発版に並べるのは ST54 から」を外した・数える規則）/ Migration `spec.md:518`「変えたのは…の 2 つの条項と」/
  delta の導出元 `spec.md:78`「ST52 の深掘り…Q4（本人の答え: ST54 まで、開発版に携帯の APK を入れない）」は、移した先の Requirement（開発版も 4 つ）の根拠として読むと逆向き
- kind: technical
- 提案: Migration に配布物の名前の括弧の変更を足す（「3 つの条項」）。導出元の Q4 は「ST52 の Q4（ST54 までの例外。ST54 で期限が来た）」のように、期限が来たことが分かる書き方にする
- 処置: fixed specs/distribution/spec.md —— Migration を 3 つの条項にし、導出元の ST52 Q4 に「ST54 で期限が来た」を書いた

---

## 観点 1: deep の決定が正典に写っているか（draft。推奨の答えで照合）

R5（Q2）・R11（Q4 の中身の直し）のほかは、推奨が写っている。確かめた範囲:
- Q1 → `spec.md:205-220`（版番号の `-dev.` で決まる・手で組んだ版は正式版）
- Q3 → `spec.md:343-387`（印の無い今の DB は正式版・違えば止まる・開発版の DB を正式版のサーバに繋いでも止まる）
- Q4 → `spec.md:389-468`（旧名の対応・直しの移行・止まって原因を示す `kind=db_ahead`）
- Q5 → tasks 11.1（`docs/release.md` に `git fetch --tags --force`。手順書だけで振る舞いを変えないので spec に無くてよい）
- Q6 → `spec.md:300-341`（R1〜R4 の抜けは別）
- Q7 → `spec.md:225`・`:245`（「あしあと。開発版」「ashiato 開発版」。アイコンは `design.md:148` で同じ）
- Q8 → `spec.md:491-507`
- Q9 → D10・tasks 12.1（R12・R14）。完了の判定への 3 つの追記は record の段（`design.md:141`）で、今は ST54.md に無いのが想定どおり
- Q10 → `spec.md:226`・`:239-241`
- 数値: 待ち受け `18788`・DB のポート `55433` は `tools/ports.sh` の割り当て（18787・19000〜・55432・55499・55500〜・55600〜）の外。`18788` は Scenario `spec.md:480` にある。`55433` は手順書（tasks 11.1）だけで Scenario に無いが、サーバの振る舞いではないので置き場は正しい
- 要件へ戻すもの: FR-104 の ★ 2026-11-01（`docs/requirements.md:969`）は「ST54 まで」と期限を自分で持つので、改訂の印を足さなくても矛盾しない。proposal の「record の段が判断する」の扱いでよい

## 観点 3: 置き場（R1・R3・R6・R12 以外）

- 実装の名前: 該当なし（確かめた範囲: spec が使うのは環境変数 `ASHIATO_STATE_DIR` `ASHIATO_DEVICE_ID` `BIND` `MIGRATE_BACKUP_DIR`・ファイル名 `migrations/LINEAGE` `exclusions.json`・API の道 `GET /contract` と欄 `channel`・命令 `ashiato-server migrate`・終了コードと標準エラーの種別で、関数名・crate 名・列名は無い）
- proposal の Capabilities と `specs/` のディレクトリ: `distribution` だけで一致。INDEX.md の capability 表（`docs/stories/INDEX.md:105`）とも一致するが、R1 のとおり足りない

## 観点 5: Story と要件

該当なし（確かめた範囲: `check_chain.py` OK（観点 8 を含む）。ST54.md の「価値」「壊してはいけないもの」「完了の判定」は推奨の答えと矛盾しない。satisfies は FR-106 だけで、
specs の導出元に出る FR-104・FR-105・FR-107 は ST52 の Requirement を写したもの、FR-117・FR-83・FR-30 は FR-106 の振る舞いが触れる既存の要件として挙げただけで、満たす要件を前倒ししていない）

## 観点 6: 答えが写ったか

該当なし（mode が `spec --draft`。`--answers` ではない）

## 観点 7: 走っている change との前提関係

観点 7: 該当なし（spec_overlap.py candidates が 0 件）

---

# 第 2 回（合意の差し戻しの直し。IMP-01・IMP-02）

範囲: 第 1 回の合意で「仕様を直す」と答えた IMP-01・IMP-02 の直し（`git diff HEAD -- openspec/changes/st54-dev-side-by-side` の deep.md・design.md D2/D3・spec.md の PC の開発版の Requirement・tasks.md 冒頭/3.2/6.1）と、同じ型の漏れ。R1〜R15 の処置済みの部分は見ていない。

## 機械の検査（2026-11-02）

- `openspec validate st54-dev-side-by-side --strict` → `Change 'st54-dev-side-by-side' is valid`
- `python3 scripts/check_chain.py .` → `chain: OK (0 件 / 未回収 0 件 / warn 0 件)`（観点 8 を含む）
- `python3 scripts/check_scenarios.py . st54-dev-side-by-side` → `scenarios: FAIL (担保なし 61 件 / 名無しの確認待ち 0 件)`（spec 段では印がまだ無いので想定どおり。件数 61 は tasks 冒頭の「ST54 が新しく印を置くのは 61 本」と一致）
- `python3 scripts/review_triage.py . st54-dev-side-by-side` → `triage: OK`（指摘 27 件 / 仮決め 2 件）
- `python3 scripts/spec_overlap.py candidates . st54-dev-side-by-side` → `[]`
- 勘定: `grep -c '^#### Scenario:' specs/distribution/spec.md` = 90。MODIFIED 2 Requirement に 13 本・ADDED 12 Requirement に 77 本（= 移した 2 つの 23 + 新しい 10 の 54）。90 − 61 = 29（ST52 の写し）。tasks 冒頭の 90 / 61 / 29 は合う。6.1 の名指しは 10 本で、`-ge 10` と合う

## R16. tasks 6.1 の新しい試験は、直していないコードでも緑になりうる。`history_exclusion_added_later` と「同じ組み方」で `write_rules` を使うと、除外は `<置き場>/dev` に書かれる
- 成果物: openspec/changes/st54-dev-side-by-side/tasks.md
- 根拠: tasks.md:98（「既存の `history_exclusion_added_later` と同じ組み方」）/ `crates/collector-windows/src/runtime.rs:3925-3931`（`write_rules` は `cfg.state_dir.join("exclusions.json")` に書く）・`runtime.rs:4156-4165`（`history_exclusion_added_later` は `write_rules(&cfg, …)` で除外を置く）/
  開発版の設定では `state_dir = <置き場>/dev`（spec.md:298・design.md:41）。そこへ書けば、直していない `history/collect.rs:315`（`state_dir.join("exclusions.json")`）がそれを読み、`example.test/1` は送られず試験は通る —— IMP-01 を直さないまま緑になる。Scenario の GIVEN（spec.md:331「`<置き場>/dev` には除外の登録が無く」）を試験が守る手段を tasks が書いていない
- kind: technical
- 提案: 6.1 の試験の記述に「除外は `write_rules` を使わず、渡された置き場（`cfg.state_dir` の親）の `exclusions.json` に直接書く」「取り込みの前に `<置き場>/dev/exclusions.json` が無いことを assert する」を足す。組み方の手本は `history_exclusion_url_matches_history`（`runtime.rs:4086`。初回の取り込みで除外を見る）の方が近い
- 処置: fixed tasks.md —— 6.1 の試験の記述を「`exclusions.json` は渡された置き場に直接書き、`write_rules` は使わない」「取り込みの前に `<置き場>/dev/exclusions.json` が無いことを assert する」「直していない `collect.rs:315` では落ちる」にした

## R17. tasks 6.1 が `HistoryCollector::new` の引数を変えると書くが、他の呼び出し口と `Runtime` が置き場を持つことを挙げていない。Windows だけで組む試験の壊れは Linux の検証に出ない
- 成果物: openspec/changes/st54-dev-side-by-side/tasks.md
- 根拠: tasks.md:93（「`HistoryCollector::new` が除外の登録の置き場を引数で受け、`Runtime::with_history` が設定の `exclusions_path` を渡す」）/
  呼び出し口は `runtime.rs:310`（`with_history`）のほかに `history/collect.rs:844`（試験の土台）と `tests/runtime_windows.rs:1443`（`#![cfg(windows)]`。`tests/runtime_windows.rs:19`）/
  `with_history(mut self, reader)` は `cfg` を受けず、`Runtime` は `state_dir` だけを持つ（`runtime.rs:284`）ので、`Runtime::new` で `cfg.exclusions_path` も持たせる必要がある /
  6.1 の検証（tasks.md:99）は Linux の `cargo test -p ashiato-collector-windows` で、`runtime_windows.rs` を組まない。壊れは CI の Windows の job（`.github/workflows/ci.yml:191`）と 8.1 で初めて出る。
  なお `Config` に `exclusions_path` を足すこと自体（前の版から 6.1 にある）でも、`tests/runtime_windows.rs:585`・`:654` の構造体の直書きが組めなくなる（`examples/browser_history_smoke.rs:172`・`runtime.rs:955` は Linux の `cargo test` が組むので出る）
- kind: technical
- 提案: 6.1 に「`Runtime::new` が `cfg.exclusions_path` を `state_dir` と同じく持つ」「`HistoryCollector::new` の呼び出し口 3 つ（`runtime.rs` の `with_history`・`collect.rs` の試験の土台・`tests/runtime_windows.rs` の `fetch_history_until`）と `Config` の直書き 4 つを直す」を書く。Windows 側の組みは 8.1（CI の Windows の job）で確かめる、と 6.1 の検証に添える
- 処置: fixed tasks.md —— 6.1 に、`Runtime::new` が設定の `exclusions_path` を持ち `with_history` が渡すこと、`HistoryCollector::new` のほかの呼び出し口（`collect.rs:844`・`tests/runtime_windows.rs:1443`）と `Config` の直書き（`runtime.rs` の `cfg()`・`examples/browser_history_smoke.rs:172`・`tests/runtime_windows.rs:585`・`:654`）を合わせること、Windows だけで組む試験は Task 8 の Windows の検証で組めることを見る、を書いた

## R18. ST58 の requires の鎖に ST54 が無い。ST58 の上流が ST54 の merge より先に始まると、3.2 の申し送りは ST58 に届かず、IMP-02 は塞がらない
- 成果物: openspec/changes/st54-dev-side-by-side/tasks.md（3.2）・docs/stories/ST58.md（requires）
- 根拠: tasks.md:65-67（申し送りは下流の 3.2 が `docs/handoff/ST58.md` に書く = ST54 の feat の PR が merge されて初めて main に載る）/
  `docs/stories/ST58.md:5` `requires: [ST57, ST48, ST51]`・`ST57.md:5` `requires: [ST52, ST55]`・INDEX.md:75-81（ST55 → ST52・ST28、ST51 → ST02・ST49。どこからも ST54 に届かない）/
  `docs/handoff/README.md` の規則 4・5（上流は開始時に `story_brief.py` で handoff を読み、途中では入れない）/ go.json IMP-02 の `if_accepted.residual`（ST58 が終了コード 4 を移行の失敗と扱って作ると、古い退避を戻して以後の記録を失う）
- kind: technical
- 提案: 呼び出し元が `docs/stories/ST58.md` の requires に ST54 を足す（順の付け方で、要件は変わらない）。足さないなら、3.2 に「ST58 の上流がすでに始まっていれば、申し送りの代わりに ST58 の change の戻しの判定が終了コード 4 で戻さないことを確かめ、無ければ deep の問いとして立てる」を添える
- 処置: fixed proposal.md —— Impact に申し送りと requires の追加を書き、`stories.json` の ST58 の requires に ST54 を足して `make_story.py` で再生成し、`INDEX.md` の ST58 の行と訂正の記録（2026-11-02）を足した（layer は 7 のまま。要件は変わらない）

## 観点 1（IMP-01・IMP-02 の差し戻しが写ったか）

- IMP-01: 写った。Requirement の条項（spec.md:299「前景の窓の記録でもブラウザの履歴の取り込みでも」）・新しい Scenario（spec.md:330-333）・design D2 の 2 つの読み込み口（design.md:41。`main.rs:81` と `history/collect.rs:315` は実コードの行と一致）・tasks 6.1（tasks.md:93・98）・導出元（spec.md:303）。go.json IMP-01 の options の 1 つ目と verify（`runtime.rs` の試験・`<置き場>/dev` に置かない）と同じことを言う。試験が直しを落とせるかは R16
- IMP-02: 写った。design D3 の申し送りの箇条（design.md:60）と tasks 3.2（tasks.md:65-67）。3.2 の検証の grep 4 つは、今の `docs/handoff/ST58.md` に「終了コード 4」「channel_mismatch」「`## st54-dev-side-by-side D3`」が無く、`## st48-db-version-guard R21` がある（3 行目）ことを確かめた —— 書き足す前は落ち、書き足せば通り、既存の節を消すと落ちる。`kind=channel_mismatch` と終了コード 4 は spec.md:404・414 と一致。3.2 は 3.1 の後で依存順も合う。届くかは R18
- 数値: 新しい Scenario に置く値（`example.test/1`・`example.test/2`）は試験の偽の読み手の URL（`runtime.rs:2307` `https://example.test/{id}`）で作れる。除外の記録（`history/contract.rs:237-252` `Visit::excluded`）は URL を持たないので、「本文に `example.test/1` が無い」は除外の件数の記録と矛盾しない

## 観点 2: 該当なし（確かめた範囲: 新しい Scenario 1 本。WHEN「初めて取り込んで送る」・THEN「送る本文に 1 の訪問が無く 2 の訪問がある」は送った本文で決まる。THEN の後半は「何も送らない」実装を落とす対照で、主張は「選んで除く」1 つ。Requirement の書き直した条項は 2 つの読み込み口を言い、Scenario が 1 本ずつ持つ）

## 観点 3: 該当なし（確かめた範囲: 足した文の置き場。spec に入った名前はファイル名 `exclusions.json` と環境変数だけで、`HistoryCollector`・`collect.rs` などの実装の名前は design D2 と tasks にだけある。D3 の申し送りの箇条は他の Story への手順で、観測できる振る舞いではない）

## 観点 4: R16・R17。ほかは該当なし（確かめた範囲: 6.1 の `-ge 10` は 6.1 が名指す 10 本と一致。ただし同じ crate の `st54_` の絞りは 7.1 の `st54_channel*`（`-ge 5`）も数えるので、7.1 の後は 6.1 の下限だけでは 6.1 の試験の欠けを落とせない —— 前の版の `-ge 9` から同じで、印の欠けは 11.2 の `check_scenarios.py` が落とすので指摘にはしない。3.2 は検証のコマンドと終了条件を持つ）

## 観点 5: R18。ほかは該当なし（確かめた範囲: `check_chain.py` OK。直しは ST54 の satisfies（FR-106）と除外の FR-83 の範囲で、要件を足していない）

## 観点 6: 該当なし（mode なし。`--answers` ではない）

## 観点 7: 走っている change との前提関係

観点 7: 該当なし（spec_overlap.py candidates が 0 件）

## 同じ型の漏れ（除外の登録を state_dir から読むほかの口）

該当なし（確かめた範囲: `crates/collector-windows` の `exclusions.json` / `Exclusions::load` の呼び出しは `main.rs:81` と `history/collect.rs:315`（`:598` で読む）の 2 つだけで、design D2 の 2 つと一致。`state_dir` の下のほかのファイル（`last-seen.txt`・`counters*.json`・`outage-notice.json`・`contract-wait.json`・`engine.json`・`collector.log`・`browser-history/`）は収集アプリが自分で書くもので、本人の登録ではない。携帯の収集アプリ（`collector-android/app/src/main`）に除外の登録は無い。`exclusions.json` を書く道具・手順書は repo に無い（`crates/collector-windows/README.md:37` が置き場の表に挙げるだけ）

---

# 第 3 回（spec --answers Q11,Q12,Q13,Q14。深掘り第 2 回の答えの写し）

範囲: `git diff HEAD -- openspec/changes/st54-dev-side-by-side docs/briefs/ST54-pr.md`（design D5・D7・D8・D12（仮）・D13（仮）・Risks・Migration Plan、proposal、provisional.jsonl、spec の「収集アプリは、自分と同じ系統のサーバにだけ送る」と導出元 3 つ、tasks 冒頭・2.1・Task 5〜7・9.2、PR 本文）。整合の確認 `docs/audit/2026-11-03-st54-q12-consistency.md` の C1・C4〜C8 の写しを含む。R1〜R18 の処置済みの部分は見ていない。

## 機械の検査（2026-11-03）

- `openspec validate st54-dev-side-by-side --strict` → `Change 'st54-dev-side-by-side' is valid`
- `python3 scripts/check_chain.py .` → `chain: OK (0 件 / 未回収 0 件 / warn 0 件)`（観点 8 を含む）
- `python3 scripts/check_scenarios.py . st54-dev-side-by-side` → `scenarios: FAIL (担保なし 70 件 / 名無しの確認待ち 0 件)`（spec 段では印がまだ無いので想定どおり。70 は tasks 冒頭の「ST54 が新しく印を置くのは 70 本」と一致）
- `python3 scripts/review_triage.py . st54-dev-side-by-side` → **`triage: FAIL (1 件)`**「deep.md R10: design.md の D8 に「仮」の印が無い」（R19）
- `python3 scripts/spec_overlap.py candidates . st54-dev-side-by-side` → `[]`
- 勘定: `grep -c '^#### Scenario:' specs/distribution/spec.md` = 99。MODIFIED 2 Requirement に 13 本、移した 2 つに 23 本（12 + 11）、新しい 10 の Requirement に 63 本（3+3+3+7+19+8+4+10+4+2）。tasks に名前が一字一句ある Scenario は 70 本、無い 29 本は全部 ST52 の写し（署名・下書き・印・作り直し）。tasks 冒頭の 99 / 70 / 29 は合う。7.1 の名指しは 10 本で `-ge 10`、9.2 の名指しは 7 本で `-ge 7` と合う

## R19. review_triage が FAIL する: review/deep.md R10 の処置「fixed D8 仮」が、Q14 で本人が決めて（仮）の外れた D8 を指したまま
- 成果物: openspec/changes/st54-dev-side-by-side/review/deep.md（R10 の処置）・design.md（D8）
- 根拠: `python3 scripts/review_triage.py . st54-dev-side-by-side` → `[FAIL] deep.md R10: design.md の D8 に「仮」の印が無い` / review/deep.md:96「処置: fixed D8 仮」・:94「kind: daily」/ scripts/review_triage.py:167-181（kind=daily は `fixed D<n> 仮` か escalated でしか閉じられず、`fixed D<n> 仮` は design の D<n> を含む行に「仮」を求める）/ design.md の D8 を含む行に「仮」は 0 行（D5・D7 は `design.md:193` などの「D7・D12（仮）」の行でたまたま通っている）/ provisional.jsonl の D8 は `resolved: true`（深掘り第 2 回 Q14 で人間に返した）
- kind: technical
- 提案: R10 の処置を「escalated —— 推奨で D8（仮）に書いたが、別の席が否とし深掘り第 2 回 Q14 で本人が決めた（系統で選び、値はこのまま）」の形に直し、`review_triage.py` が OK に戻ることを確かめる。D8 に（仮）を戻すのは誤り（本人の決定）
- 処置: fixed review/deep.md —— R10 の処置を「escalated —— D8（仮）にまとめたが別の席が否とし、深掘り第 2 回 Q14 で本人が決めた。D8 は仮でない」に直した。D8 に（仮）は戻さない

## R20. Requirement が携帯にも課す「24 時間ごとの出し直し」と「404・読めない 200・401 は系統の判定に使わない」に、携帯の Scenario が無い
- 成果物: openspec/changes/st54-dev-side-by-side/specs/distribution/spec.md
- 根拠: spec.md:346「携帯は画面の行と通知、PC は通知で、**どちらも**状態に入った契機で 1 回、続く間は前に出してから 24 時間ごとに出し直す」・spec.md:344「404・200 で読めない本文・401 などの答えは系統の判定に使わず」（主語は収集アプリ全体）/ 出し直しの Scenario は PC の 1 本だけ（spec.md:407、GIVEN「PC の収集アプリ」）、404・読めない答えの Scenario も PC の 2 本だけ（spec.md:417・422、GIVEN「開発版の PC の収集アプリ」）/ 携帯は `ContractGate.kt`・`ContractNotifier` の別実装（tasks 9.2、design D12「`ContractNotifier.update` を…もう 1 つ持つ」）で、PC の試験は携帯の振る舞いを落とせない / design D12 が出し直しを選んだ理由（design.md:172「取り違えに気づく手がかりが 83 日目の 1 回だけにならない」）は携帯の 90 日の上限に効くもので、担保が無いのは効く側の方
- kind: technical
- 提案: 携帯の Scenario を 2 本足す:「系統の違いの知らせは、携帯でも続く間 24 時間ごとに出し直す」（23 時間後は出さず 24 時間後に 1 回）・「契約の問い合わせが 404 か読めない 200 なら、携帯の収集アプリは系統の違いの知らせを出さない」。9.2 の名指しに足し、`-ge 7` を `-ge 9`、tasks 冒頭の 99 / 70 / 63 を 101 / 72 / 65 に直す
- 処置: fixed 9.2 —— 携帯の Scenario「系統の違いの知らせは、携帯でも続く間 24 時間ごとに出し直す」「契約の問い合わせが 404 か読めない 200 なら、携帯の収集アプリは系統の違いの知らせを出さない」を足し、tasks 9.2 に名指しして下限を `-ge 9` に、冒頭の勘定を直した

## R21. 「契約の版を読めない答えでは、系統の違いの知らせを出さない」の THEN が「ST50 の今の扱いのまま」で観測の言葉でなく、2 つの答え × 3 つの主張を 1 本に束ねている
- 成果物: openspec/changes/st54-dev-side-by-side/specs/distribution/spec.md
- 根拠: spec.md:422-425 GIVEN「200 で HTML の本文…を返す偽のサーバと、401 を返す偽のサーバ」・THEN「どちらも札 `channel` の通知は出ず、ログに `channel_mismatch` は無く、**ST50 の今の扱い（読めない 200 は 24 時間の待ち・401 は一時的な失敗）のまま**」—— 「今の扱いのまま」は何を測るかを言っていない（24 時間の待ちは 1 回の契機では観測できず、WHEN は「送る契機が 1 回来る」）。401 の側と読めない 200 の側の片方だけを試験しても、名前の印は 1 本で緑になる
  同じ型: spec.md:437-440「携帯の上限の 7 日前の知らせに、系統の違いの案内を添える」は GIVEN に自分と相手の系統が無く、THEN が「（相手が `official` のとき）」の条件つき —— 相手が `dev` の側を試験しても印は通る
- kind: technical
- 提案: 読めない 200 と 401 を 2 本に分け、THEN を観測できる形（「札 `channel` の通知は出ず、`channel_mismatch` のログも無く、契約の版の待ちの状態（ST50 の `contract-wait.json`）に入った／入らない」など）にする。7 日前の Scenario は GIVEN に「開発版の携帯の収集アプリと、見出し `Ashiato-Channel: official` で答えるサーバ」を書き、THEN の条件を外す。本数が変われば tasks 7.1 の `-ge 10` と冒頭の勘定も直す
- 処置: fixed 7.1 —— 読めない 200 と 401 を 2 本に分け、THEN を札 `channel`・`contract` の通知の有無・`channel_mismatch` のログ・契約の版の待ちの理由 / 状態で書いた。7 日前の Scenario は GIVEN に自分と相手の系統を書き、THEN の条件を外した。tasks 7.1 の下限と勘定を直した

## R22. D12（仮）の観測できる振る舞い（再起動を跨いで 24 時間を数える・札を分けて片方だけ消す）が design にだけあり、Scenario が無い
- 成果物: openspec/changes/st54-dev-side-by-side/design.md（D12）・specs/distribution/spec.md
- 根拠: design.md:170「前に出した時刻は ST50 と同じ手元の置き場に持つ（**再起動しても 24 時間経つまで出し直さない**）」・design.md:169「ST50 の札・id と分けるのは…**片方を消すときにもう片方を消さない**ため」/ spec の出し直しの Scenario（spec.md:407-410）は再起動を跨がず、404 の Scenario（spec.md:417-420）は 2 つの札が並ぶ状態も片方を消す契機も持たない / `openspec archive` は main specs しか更新しないので、design にだけある振る舞いは正典から落ち、実装が時刻を記憶に持っても（再起動のたびに通知が出ても）spec は落ちない
- kind: technical
- 提案: 「系統の違いの知らせの時刻は、収集アプリを起こし直しても引き継ぐ」（出した 1 時間後に起こし直して契機が来ても出し直さない）を Requirement の条項と Scenario に足す。札の分離は「系統の違いの知らせを消しても、契約の版の待ちの知らせは消えない」の形で足すか、足さないなら D12 から観測の言い方を外す。D12 が仮なので Scenario も仮のまま書いてよい
- 処置: fixed D12 仮 —— Requirement に「起こし直しても 24 時間経つまで出し直さない」「片方を消してももう片方は消さない」の条項を足し、Scenario「系統の違いの知らせの時刻は、収集アプリを起こし直しても引き継ぐ」「系統の違いの知らせを消しても、サーバを更新してくださいの知らせは消えない」を足して tasks 7.1 に名指しした

## R23. 「系統の違うサーバからも、携帯の収集アプリは途絶の判定を今どおり引く」の GIVEN が系統の違いの状態に入っておらず、今のコードのままで緑になり、読む口を止めた実装も落とせない
- 成果物: openspec/changes/st54-dev-side-by-side/specs/distribution/spec.md
- 根拠: spec.md:442-445 GIVEN「開発版の携帯の収集アプリと、契約の問い合わせに見出し `Ashiato-Channel: official` を付けて答える偽のサーバ」・WHEN「途絶の判定を問い合わせる契機が来る」・THEN「`GET /outages` が今どおり引かれる」—— 送る契機（契約の問い合わせ）が先に来ることを GIVEN も WHEN も言わないので、収集アプリが系統の違いを知る前の状態で試験できる。その形では、送る契機で得た `ChannelMismatch` を読む口のゲートにも使う実装（D13 の反対側）でも通る / design.md:180（D13 が選んだのは「系統の違いで止めない」）
- kind: technical
- 提案: GIVEN を「送る契機で系統の違いを知り、送らずにいる（系統の違いの知らせを出した）開発版の携帯の収集アプリ」にし、THEN に「`GET /outages` が引かれる」を残す
- 処置: fixed 9.2 —— GIVEN を「送る契機で系統の違いを知り、送らずにいる（知らせを出した）」にし、THEN を「同じ偽のサーバに `GET /outages` が引かれる」にした

## 観点 1（第 2 回の答えが写ったか）

- Q11: 写った。design D5 の頭書き（design.md:80）・「Q5 に乗る」（基準の印が 1 つで `--force` が要るのは Q5 による。日付つきの印に変わったら見直す）・反転条件・選ばなかった案、spec の導出元（spec.md の D5 の Requirement）、tasks 冒頭・Task 5 の見出し、proposal。Scenario の言うこと（印 `nightly` 1 つの版と比べる）は前の版のままで、答えと同じ
- Q12: 写った。spec.md:343 の WHEN 条項（自分の系統と違えば送らない。向きを限らない）と、両向きの Scenario（PC・携帯それぞれ「正式版の…は、開発版のサーバに何も送らず手元に残す」）、design D7 の逆向きの箇条・残るもの（携帯の 90 日の上限）・反転条件（Q12 を変えるなら聞き直す）、FR-106 ★ と一致
- Q13: 写った。spec.md:346 の WHILE 条項の文「送り先が開発版のサーバです。送り先を確かめてください」/「送り先が正式版のサーバです。…」は答えの文と FR-106 ★ の文と同じ。PC・携帯の知らせの Scenario（spec.md:402・427）。選択肢の detail の「1 回」と表題の「ST50 と同じ形」（24 時間ごと）の読みの分かれは D12（仮）が出し直す側に倒した（整合の確認 C5 の行き先どおり）。携帯の担保の不足は R20
- Q14: 写った。design D8 の頭書き・「D1 に乗る」・反転条件・選ばなかった案、spec の導出元 2 か所、tasks 2.1・6.2・Task 6 の見出し。値（18788・`ashiato_session_dev`・`backups/pre-migrate-dev`・55433・`ashiato-collector-windows-dev.exe`）は変わっていない
- 数値: 24 時間（spec.md:346・Scenario spec.md:407 の 23 時間 / 24 時間）、7 日前（Scenario の 83 日）は Scenario にある
- 古い側の残り: `D5（仮）`・`D7（仮）`・`D8（仮）`・「未決」・「答えが入るまで」・「欄 `channel`」・「ログだけ」は change の design / tasks / proposal / spec / premises と PR 本文に 0 件（grep）。ただし R19

## 観点 2: R20・R21・R23。ほかは該当なし（確かめた範囲: 足した 9 本の Scenario。PC の知らせ・出し直し・送り先を直す・404、携帯の知らせ・送り先を直すは、札・文・件数・要求の有無で真偽が決まる）

## 観点 3: R22。ほかは該当なし（確かめた範囲: D7 の足した箇条（読めた答えからだけ読む・系統は 2 つ・知らせの文・直したら送る）は spec.md:342-347 に条項がある。D13 は spec.md:348 の条項と Scenario がある。spec に入った名前は札 `channel`・`Ashiato-Channel`・`GET /outages` で、`read_contract_answer`・`ContractNotifier`・`OutagePoller` などの実装の名前は design と tasks にだけある。proposal の Capabilities と specs/ は distribution 1 つで変わらない）

## 観点 4: 該当なし（確かめた範囲: 7.1 は `st54_channel` の `--list` で `-ge 10`（名指し 10 本）、9.2 は `ChannelGateTest` の `<testcase` で `-ge 7`（名指し 7 本）、どちらも ST50 の試験を別に走らせる。依存順は 2.1（見出しを出す）→ 7.1 / 9.2（読む）で合う。新しい Scenario 9 本の名前は tasks に一字一句ある。人間の確認待ちは足していない）

## 観点 5: 該当なし（確かめた範囲: `check_chain.py` OK。ST54.md の完了の判定 5 行目「正式版の携帯の収集アプリを開発版のサーバへ向けても（逆も）記録は送られず、『送り先が開発版のサーバです』の知らせが出て、送り先を直すとそのまま送られる」は spec.md の「正式版の携帯の収集アプリは、開発版のサーバに何も送らず手元に残す」・「系統が違って送らない間、携帯の収集アプリは…知らせる」・「送り先を直すと、携帯の収集アプリは…」の 3 本と、逆向きは PC の 3 本と合う。壊してはいけないもの 3 行目（ST50 の知らせと契約の版を確かめる送り手）は「系統が違って送らない間は、サーバを更新してくださいの通知を出さない」・404 の Scenario と、7.1 / 9.2 の ST50 の試験で受ける。satisfies は FR-106 だけで、spec は FR-106 ★ 2026-11-03 の範囲を越えていない。D12 の「携帯は画面の行も」は ★ の「携帯は通知」より広いが、ST50 の口（画面の行と通知）と同じで仮の決定として置いてある）

## 観点 6: `review/answers.md`（Q11・Q12・Q13・Q14 とも写った。答え以外の変更はあり —— 整合の確認 C1・C8 を仮でない D7 の箇条と spec の条項に写した。決定の行は書いていない）

## 観点 7: 走っている change との前提関係

観点 7: 該当なし（spec_overlap.py candidates が 0 件）

---

# 第 3 回の直しの確かめ（2026-11-03。C1・C8 を D14（仮）へ移し、R19〜R23 を直した後）

- `openspec validate --strict` valid / `check_chain.py` OK / `check_scenarios.py` 担保なし 75 件（tasks 冒頭の「新しく印を置く 75 本」と一致）/ `review_triage.py` OK / `spec_overlap.py candidates` `[]`
- 勘定: Scenario 104 本、tasks に名前が無い 29 本は ST52 の写し（前と同じ 29 本）。7.1 の名指し 13 本 = `-ge 13`、9.2 の名指し 9 本 = `-ge 9`。足した 14 本の名前は tasks に一字一句ある
- 「読めない 200」の PC の Scenario の THEN「契約の版の待ちの理由に『読めない答え』が記録される」は、`contract_gate.rs:24` の `unreadable_since` と `:44` の `unreadable_long`（ST50 の待ちの理由）で観測できる

## R24. 携帯の「契約の問い合わせが 404 か読めない 200 なら…」が、R21 で PC の側を分けたのと同じ型で 2 つの答えを 1 本に束ねている
- 成果物: openspec/changes/st54-dev-side-by-side/specs/distribution/spec.md
- 根拠: spec.md:457 の Scenario「契約の問い合わせが 404 か読めない 200 なら、携帯の収集アプリは系統の違いの知らせを出さない」—— GIVEN が「404 を返す偽のサーバと、200 で HTML の本文…を返す偽のサーバ」、THEN が「どちらも…出ず…、404 では『サーバを更新してください』の通知が出る」。印は 1 本なので、片方の答えだけを試験しても緑になる。PC の側は R21 の処置で「404 なら」「読めない 200 なら」「401 なら」の 3 本に分けた（tasks 7.1）
- kind: technical
- 提案: 携帯も「404 なら、携帯の収集アプリは系統の違いの知らせでなく更新を促す」と「読めない 200 なら、携帯の収集アプリは系統の違いの知らせを出さない」の 2 本に分け、9.2 の名指しと `-ge 9` → `-ge 10`、tasks 冒頭の勘定（104 / 75 / 68 → 105 / 76 / 69）を直す
- 処置: fixed 9.2 —— 携帯の Scenario を「404 なら…系統の違いの知らせでなく更新を促す」「読めない 200 なら…系統の違いの知らせを出さない」の 2 本に分け、9.2 に名指しして下限を `-ge 10` に、冒頭の勘定を 105 / 76 / 69 に直した

## R24 の直しの確かめ（2026-11-03）

- 携帯の Scenario は「404 なら…更新を促す」（spec.md:457）と「読めない 200 なら…出さない」（spec.md:462）の 2 本に分かれ、9.2 の名指し 10 本 = `-ge 10`、勘定 105 / 76 / 69 は実物（Scenario 105 本・tasks に名前の無い 29 本・`check_scenarios.py` 担保なし 76 件）と合う。読めない 200 の THEN の待ちの理由は `ContractGate.kt:80` の `unreadableSinceMs` で観測できる。validate valid・check_chain OK・triage OK。新しい指摘なし
