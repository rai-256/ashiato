# ST19 実装タスク — 個人属性を上書きせず履歴で残す

読む順: `deep.md`（**最優先。本人が決めたこと**）→ このファイル → `specs/personal-entities/spec.md` → `design.md`
→ `proto.html`（Q2 の画面。`#p=1v` で本人が選んだものに近い形）→ `docs/stories/ST19.md` → `docs/handoff/`（開始時と PR 前の 2 回）→ `CLAUDE.md`。

**移行は 1 本だけ足す**（design D12）。**名前は作成時刻 `YYYYMMDDHHMM_personal_attributes.sql`**（連番にしない）で、
`crates/server/src/lib.rs` の `MIGRATIONS` 配列の末尾に足す。
**`record-envelope` の要件・取り込みの契約の欄・収集側・ST03 の錠と門の関数・ST16 の行き先には触らない**（design D10）。

検証は各タスクの本文に書いてある。「動いた」ではなくコマンドと終了コードで判定する。
DB を使う検査は `docker compose up -d db` が前提。**章は依存の順**（種類と読み出しの関数 → 取り込み → 画面）。

## 0. 規律（**最初に読む**）

- **テストには `Scenario: <名前>` の印を置く。** Rust / TypeScript はコメント（`// Scenario: 住所を 2 回変えると 3 つの主張が残る`）、bash は `echo`。
  `scripts/check_scenarios.py` が spec の全 Scenario と突き合わせ、**印の無い Scenario を FAIL にする**。印の名前は spec の `#### Scenario:` と**一字一句合わせる**
- **この change が足す Scenario は 106 本**（すべて `personal-entities`）
- **件数つき検証**: `cargo test <絞り込み>` は一致するテストが 0 本でも rc=0 になる。このファイルで **`CT <絞り込み>`** と書いたものは、
  `bash -o pipefail -c 'cargo test -p ashiato-server <絞り込み> 2>&1 | tee /tmp/ct.log' && grep -Eq 'test result: ok\. [1-9][0-9]* passed' /tmp/ct.log` が rc=0 になることを指す。
  絞り込みはテストのパスの部分一致なので、**単体テストは `attributes.rs` の `mod tests` に置き、`attributes::tests::<接頭辞>` で絞る**（`stay.rs` の置き方）。
  結合テストは `crates/server/src/attributes_tests.rs`（`stay_tests.rs` の置き方）に置き、`attributes_tests::<接頭辞>` で絞る。
  **`VT <ファイル>`** は `bash -o pipefail -c 'cd web && npx vitest run src/__tests__/<ファイル> 2>&1 | tee /tmp/vt.log' && grep -Eq 'Tests +[1-9][0-9]* passed' /tmp/vt.log` が rc=0
- **本人の決定（下流は変えない）**: 主張も記録で、削除の印と台帳つきの消去は通す（Q1）/ 画面は種類ごとのカードに**積んだ主張を常に全部**・「いつから」の新しい順・
  **書いた日時は押したときだけ**・訂正で取り消した主張は畳む・カードごとに「書く」1 つ・精度を先に選ぶ（Q2）/ 既定の感度はローカル AI まで（Q3）/ 1 種類 1 値（Q4）。
  **画面が長い（10 年後で 4.3 画面）のは本人が承知で選んだ。畳む形に戻さない**
- **D3 / D6 / D7 / D13 は（仮）決め。** 反転条件は `design.md`。変えたらその D 番号を書き直す
- **ログに出すのは件数・ソース名・所要時間・エラーの種別だけ**（製造準備 A-2）。主張の値・補足・種類の名前は出さない
- **主張の原文の `nonce` は `payload` にも他の列にも写さない**（design D4）

## 1. 移行（design D2 / D7 / D12）

- [x] 1.1 移行 `migrations/YYYYMMDDHHMM_personal_attributes.sql` と `.down.sql` を足す ——
  `core.attribute_kind`（`UNIQUE (id, user_id)`）/ `core.attribute_kind_name`（`(kind_id, user_id)` → `attribute_kind (id, user_id)` の外部キー）と 2 表の UPDATE / DELETE / TRUNCATE を拒むトリガ、
  `core.reject_claim_rewrite()`（`BEFORE UPDATE`）/ `core.require_claim_erasure_ledger()`（`CONSTRAINT TRIGGER … DEFERRABLE INITIALLY DEFERRED`。台帳は `event_id = NEW.id` まで照合）/
  `core.reject_claim_delete()`（`BEFORE DELETE`）と各トリガ、登録簿に `('s01-attribute', '個人属性', 86400, 'none')`。
  **当て直せる形**。`MIGRATIONS` 配列の末尾に足す。ST03 の関数（`core.reject_collected_rewrite` など）は変えない。
  `.down.sql` は主張の行が残っていれば登録簿の行と 2 表を残す（design D12）。
  検証: `tools/check-migrations.sh` rc=0、`CT attributes_tests::migration_applies_twice`、
  `git diff --stat origin/main -- migrations/202609120944_gates.sql` が空、
  主張を 1 件入れた DB で `.down.sql` を `psql -v ON_ERROR_STOP=1` で当てて rc=0 かつ `SELECT count(*) FROM core.attribute_kind_name` が当てる前と同じ
- [x] 1.2 `tools/check-immutable.sh` に主張の段を足す。**既存の「本人が書いた記録は書き換えられる」の段の `WHERE` を `logical_source = 'immutable-check'` に絞り、主張の行を入れた後に置く**（design D2 の最後の段落）。
  主張の段は `psql` で主張の行を 1 件入れてから、次を 1 つずつ確かめる:
  値の書き換えは失敗 / 「いつから」の書き換えは失敗 / 取り消し先の書き換えは失敗 / `event_time` の書き換えは失敗 / 行の削除は失敗 / 別の本人が書いた記録を `s01-attribute` へ付け替えるのは失敗 /
  削除の印は成功 / 感度は成功 / 同じまとまりで**その主張の** `core.erasure_ledger` の行を書いてからの消去は成功 / 台帳なしの消去は失敗 /
  **別の記録の**台帳の行を書いたまとまりでの消去は失敗 / その主張の台帳の行があっても `raw=''` と `payload='{"forged":true}'` にするのは失敗 /
  種類の名前の台帳の UPDATE は失敗 / 種類の表と名前の台帳の DELETE と TRUNCATE はそれぞれ失敗。
  Scenario: `主張の値を書き換える文は拒まれる` / `主張のいつからは書き換えられない` / `主張の取り消し先は書き換えられない` / `主張した日時は書き換えられない` /
  `主張の行は削除できない` / `他の記録を主張へ付け替えられない` / `主張に削除の印を付けられる` / `主張の感度を変えられる` / `台帳のある主張の消去は通る` /
  `台帳の無い主張の消去は拒まれる` / `別の記録の台帳の行では主張の消去は通らない` / `台帳の行があっても消去の形でない書き換えは拒まれる` /
  `主張以外の本人が書いた記録は従来どおり書き換えられる` / `種類の名前の台帳は書き換えられない` / `種類の台帳は削除も切り詰めもできない`。
  検証: `tools/check-immutable.sh` rc=0。**わざと壊して落ちるか**: 移行の門で台帳の照合から `l.event_id = NEW.id` を外して当て直すと rc≠0（「別の記録の台帳の行では」の段が落ちる。確かめたら戻す）

## 2. 主張の解釈・導き方・種類・読み出し（design D5 / D6 / D7 / D8）

- [x] 2.1 `crates/server/src/attributes.rs` に原文の解釈と形の検査（`parse_claim(raw, id) -> Result<Claim, ClaimInvalid>`）。精度と日付の組・暦に無い日付・値の空（前後の空白を除く）・
  `nonce` の長さ（22 文字以上の base64url）・`claim` と `id` の一致。`payload` は原文から `nonce` を除いて組み直す（NFC）。単体テスト。
  Scenario: `値が空の主張は受け付けない` / `精度と日付が合わないいつからは受け付けない` / `暦に無いいつからは受け付けない` / `乱数が短い主張は受け付けない`（ここでは種別の写像まで。応答は 3.1 でも確かめる）。
  検証: `CT attributes::tests::parse`
- [x] 2.2 `attributes.rs` に `view(kinds, claims, today)` を置き、単体テストで導き方を固定する（削除の印と消去の行は呼び出し側が除いて渡す。消去の行を渡されても落とす）。
  Scenario: `いつからが最も新しい主張がいまの値になる` / `同じいつからなら主張した日時が後の主張がいまの値になる` / `未来のいつからは予定に出ていまの値にならない` /
  `積んだ主張はいまの値と予定を含む` / `いつからを直す訂正で古い開始が残らない` / `年だけの主張はその年の初めから有効とみなす` / `いつからが分からない主張は最も古い側に置く` /
  `なしの主張がいまの値になる` / `取り消された主張がした取り消しも効く`。
  検証: `CT attributes::tests::view`
- [x] 2.3 種類の口 `POST /attributes/kinds` と `POST /attributes/kinds/{id}/names`、初期化 `ensure_initial_kinds`（利用者ごとの錠 → 0 件なら v5 の 2 つ。名前の行は種類の `INSERT … RETURNING` が返したときだけ）。
  名前は NFC、同じ錠の中でいまの名前との重なりと種類の利用者を見る。`Cargo.toml` の `uuid` に `v5` を足す。OpenAPI に載せる。
  Scenario: `種類を足せる` / `名前を変えても識別子と主張が変わらない`（主張は DB に直接入れる）/ `名前を変えても前の名前が台帳に残る` / `初めて種類を足す前に住所と職業が置かれる` /
  `空の名前の種類は足せない` / `いまある名前と同じ種類は足せない`（NFD の名前を含む）/ `いまある名前へは変えられない` / `別の利用者の種類の名前は変えられない`。
  検証: `CT attributes_tests::kinds`、`tools/check-openapi.sh` rc=0、`grep -c '"/attributes/kinds"' docs/openapi.json` が 1 以上
- [x] 2.4 `GET /attributes`（design D8）。先頭で `ensure_initial_kinds`。`core.event_live` から主張のソースの行を読み、`raw = ''` の行を除いて `view` に渡す。今日は `Asia/Tokyo`（試験は時刻を差し込めるようにする）。
  OpenAPI に載せる。**この章のテストは主張を DB に直接入れる**（取り込みの分岐は 3 章）。
  Scenario: `最初に住所と職業がある` / `同時に初めて読み出しても住所と職業は 1 つずつ`（2 本を `tokio::join!` で走らせ、種類 2・名前の台帳 2 を数える）/
  `今日は Asia/Tokyo の日付で決まる` / `消したことにした主張は出ない` / `消したことにした主張の次がいまの値になる` / `消した主張が取り消していた主張は戻る` /
  `本文を消去した主張は出ず、その取り消しも効かない`（台帳つきで消去してから読む）/ `読み出しは 2 つの時刻を別々に返す` / `種類は作った順に返る` /
  `感度で主張を絞らない` / `別の利用者の主張は読み出せない`。
  検証: `CT attributes_tests::read`、`tools/check-openapi.sh` rc=0、`grep -c '"/attributes"' docs/openapi.json` が 1 以上

## 3. 取り込み口の分岐（design D1 / D3 / D4 / D5）

- [x] 3.1 `ingest_one` に `logical_source = 's01-attribute'` の分岐を足す —— 2.1 の検査、由来が `authored` でない・端末識別子を持てば `claim_not_authored`、外部識別子を持てば `claim_has_external_id`、
  同じトランザクションで種類がその利用者にあるか・取り消す主張が主張のソース・同じ利用者・同じ種類・自分以外か、`payload` を組み直した値に差し替え、`sensitivity = 2`。
  `IngestError` に spec の表の 7 種別を足す（`docs/openapi.json` を再生成）。
  Scenario: `無い種類の主張は受け付けない` / `別の種類の主張は取り消せない` / `無い主張は取り消せない` / `別の利用者の主張は取り消せない` / `自分自身は取り消せない` /
  `主張でない記録は取り消せない` / `消した主張を取り消し先に指せる` / `1 つの主張を 2 つの主張が取り消せる` / `本人が書いたでない主張は受け付けない` /
  `端末識別子を持つ主張は受け付けない` / `外部識別子を持つ主張は受け付けない` / `乱数が短い主張は受け付けない` /
  `主張の拒否の応答に値が含まれない`（応答の本文に値と補足の文字列が無いことを `contains` で見る）/ `主張はローカル AI までで格納される` / `主張以外の既定は変わらない`。
  各 Scenario のテストは**期待する種別の値**を `assert_eq!` で見る（種別を取り違えたら落ちる）。
  検証: `CT attributes_tests::ingest`、`tools/check-openapi.sh` rc=0
- [x] 3.2 格納の結合テスト（`/ingest` 経由で入れ、DB と `GET /attributes` で読み戻す）。
  Scenario: `住所を 2 回変えると 3 つの主張が残る`（A → B → A と書いて 3 件）/ `主張した日時といつからが別々に入る` / `D-01 に入った時刻も別に残る` / `年だけ分かるいつからは年のまま残る` /
  `いつからが分からない主張を受け付ける` / `未来のいつからを受け付ける` / `なしの主張を受け付ける` / `同じ値といつからを書き直しても 1 件増える` /
  `同じ主張の再送は増えない` / `補足が残る` / `主張の原文が 1 バイトも変わらずに残る`（バイト単位で比べる）/ `合成済みでない値は合成済みで読み出される` /
  `原文と食い違う解析済みを送っても原文の値で格納される`。
  検証: `CT attributes_tests::store`
- [x] 3.3 乱数の結合テスト —— (a) 主張を入れ、`core.event.payload` と `GET /attributes` の応答に原文の `nonce` の文字列が無いこと。
  (b) 主張を入れ、その主張の `core.erasure_ledger` の行と同じまとまりで消去し、残った `id` / `event_time` と正しい種類・値・「いつから」から `nonce` を持たない原文を組んで
  `ingest::content_hash_of` を計算し、残った `content_hash` と一致しないこと。
  Scenario: `乱数は解析済みに写らない` / `消去後に残る列と正しい値から鍵を作り直せない`。
  検証: `CT attributes_tests::erasure`。**わざと壊して落ちるか**: 3.1 で `payload` を組み直すときに `nonce` を除かないと (a) が落ちる（確かめたら戻す）

## 4. 画面（design D9 / D13）

- [x] 4.1 `web/src/attributes.ts` —— 応答の型と形の検査（形が違えば失敗として出す）、「いつから」の書き方、原文の組み立て（`id` / `nonce`（`crypto.getRandomValues` で 16 バイトを base64url）/ 主張した日時と地域）、
  `/ingest` の応答の読み方（200 でも 400 でも本文の 1 件ごとの結果を読む。読めない・5xx・401・`fetch` が投げたら「届かなかった」）、種別ごとの文（design D9 の表）。
  入力が同じなら同じ原文を返し、1 か所でも変われば組み直す。
  Scenario: `同じ内容の 2 つの主張は別々の乱数を持つ`（乱数が 22 文字以上で、2 回の組み立てで異なり、識別子と一致しない）。
  検証: `VT attributes.test.ts`。**わざと壊して落ちるか**: 乱数を `id` から作ると落ちる（確かめたら戻す）
- [x] 4.2 `web/src/MasterView.tsx` と `Root.tsx` の `#/master`、`App.tsx` の見出しに入口。カードの構造は Q2 の逐語どおり（`deep.md`）、補足は行に常に出す（D13）。
  Scenario: `種類ごとのカードにいまの値と積んだ主張が全部出る` / `積んだ主張はいつからの新しい順に出る` / `補足は押さずに見える` / `書いた日時は主張を押したときだけ出る` /
  `訂正で取り消した主張は畳まれる` / `畳んだ取り消しは押すと出る` / `予定の主張は予定の文字とともに出る` / `いまの値が無い種類はまだ書いていないと出る` /
  `人物と場所のタブは無い` / `編集する操作が無い`（全部の主張の行を押した後に `input` / `textarea` が「書く」のフォームの外に 0 個）/ `稼働状況の画面からマスタ管理へ行ける` /
  `読み出しの失敗と主張が無いことを区別する`。
  検証: `VT master-view.test.tsx`、`VT app.test.tsx`（既存の稼働状況のテストが緑のまま）、`VT day-view.test.tsx`（ST16 の行き先が緑のまま）
- [x] 4.3 「書く」のフォーム（変わった / 間違っていた・取り消す主張の選択・値と「なし」・精度のラジオと選んだ欄だけ・補足・積む / やめる）と、種類を足す・名前を変える。
  送信中は「積む」を押せなくし、受理ならフォームを閉じて読み直し、受理でなければ理由を出して入力を残す。`fetch` はテストで差し替え、時刻は差し込む。
  Scenario: `変わったを選んで主張を積める` / `間違いを直すと取り消す主張を選べる` / `取り消す主張は最も新しく書いた主張が選ばれている` / `精度に年を選ぶと年の欄だけが出る` /
  `精度に分からないを選ぶと日付の欄が出ない` / `なしを選んで積める` / `主張した日時は入力させない` /
  `2 回押しても送る原文は 1 つ`（1 回目の `fetch` を失敗させてから押し直し、2 回の本文が同じ）/ `積めたらフォームが閉じて読み直す` /
  `断られたとき入力が残り理由が出る`（**HTTP 400** と `invalid_claim_value` の結果で作る）/ `届かなかったとき入力が残り届かなかったと出る`（`fetch` が投げる）/
  `画面から種類を足して名前を変えられる`。
  検証: `VT master-form.test.tsx`
- [x] 4.4 画面の下限（`day-view-limits.test.tsx` と同じ測り方）と色の出所。
  Scenario: `個人属性の画面は触れる対象の下限を満たす` / `個人属性の画面は文字のコントラストの下限を満たす` / `個人属性の画面はフォーカスの輪郭が見える` /
  `個人属性の画面は OS の明暗に追従する` / `個人属性の画面は確定した色だけを使う`（`MasterView.tsx` に `#` の色・`rgb(`・`hsl(` の直書きが無く、色は `tokens.ts` の `tone` / `SCHEMES` からだけ）。
  検証: `VT master-view-limits.test.tsx`、`grep -nE "#[0-9a-fA-F]{3,8}\b|rgb\(|hsl\(" web/src/MasterView.tsx` が 0 件

## 5. 道具

- [ ] 5.1 `tools/smoke.sh` に 1 段足す: `GET /attributes` で「住所」の識別子を引き、`/ingest` に住所 A → B → A の主張を送り（原文は `nonce` つき）、同じものをもう 1 回送り、
  `GET /attributes` の「住所」の `claims` が 3 件・`current.value` が A であることを `jq -e` で見る。
  検証: `tools/smoke.sh` rc=0
- [ ] 5.2 `tools/seed.sh normal` に、proto の導入直後のデータ（種類 5・主張 21 件。訂正 1・予定 1・いつからか分からない 1 を含む）を足す（確認バッチの画面の材料）。
  **まず `GET /attributes` で住所と職業を置き、その後に「副業」「同居」「生年月日」を足す**（先に足すと住所と職業が初期化で先に置かれて、同じ名前が 400 になる）。
  **何度当てても同じ結果にする** —— 種類は、いまの名前で既にあれば足さない。主張は固定の識別子・固定の `nonce`・固定の主張した日時で送る（同じ原文の再送は冪等で増えない）。
  検証: `tools/seed.sh normal` を **2 回続けて** rc=0 の後、
  `curl -sf -H "authorization: Bearer $API_TOKEN" "http://127.0.0.1:18787/attributes" | jq -e '([.kinds[].claims[]]|length) + ([.kinds[].superseded[]]|length) == 21 and (.kinds|length) == 5'` rc=0

## 6. 申し送りと INDEX

- [ ] 6.1 `docs/handoff/ST22.md` と `docs/handoff/ST23.md` の ST19 の項を、実装した錠と読み出しに 1 つずつ突き合わせる。ずれていたら申し送りを直す（ST22 / ST23 はまだ上流が始まっていないので差し戻しではない）。
  検証（すべて rc=0）:
  `grep -q "deleted_at" docs/handoff/ST22.md && grep -q "deleted_at" migrations/*_personal_attributes.sql`（錠が通す列の名前が一致）/
  `grep -q "event_id = NEW.id" docs/handoff/ST23.md && grep -q "event_id = NEW.id" migrations/*_personal_attributes.sql`（台帳の照合が一致）/
  `grep -q "raw = ''" docs/handoff/ST23.md && grep -rq "Scenario: 本文を消去した主張は出ず、その取り消しも効かない" crates/server/src`（消去した主張の読み方に試験がある）

## 7. まとめの検査

- [ ] 7.1 検証: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` rc=0、
  `cd web && npm run test && npm run lint && npm run build` rc=0
- [ ] 7.2 検証: `python3 scripts/check_scenarios.py . st19-personal-attributes` rc=0（106 本すべてに印）
- [ ] 7.3 検証: `python3 scripts/check_chain.py .` rc=0、`openspec validate st19-personal-attributes --strict` rc=0、
  `tools/check-migrations.sh` / `tools/check-openapi.sh` / `tools/check-boundaries.sh` / `tools/check-immutable.sh` / `tools/check-licenses.sh` がすべて rc=0
- [ ] 7.4 PR 本文に **仮決め（D3 / D6 / D7 / D13）と反転条件**を列挙する。検証: `gh pr view --json body -q .body | grep -cE "D(3|6|7|13)（仮）"` が 4 以上
- [ ] 7.5 `docs/handoff/` を読み直す（開始時と PR 前の 2 回）。検証: `ls docs/handoff/ST19.md 2>/dev/null` が空か、あればその各項目に PR 本文で触れている

## 人間の確認待ち

**機械で確かめられないのは「違和感」だけ**（2026-09-14 の決定）。正しさは 6 章までのテストが持つ。
確認バッチ（`/verify`）の手順書が、この Story について「触ってみて違和感は無かったか」を 1 問だけ聞く
（見るもの: `SEED=normal` のマスタ管理の画面で、住所のカードに積んだ主張が全部並び、主張を押すと書いた日時が出ること。「書く」で主張を 1 件積めること）。
