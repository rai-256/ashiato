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
