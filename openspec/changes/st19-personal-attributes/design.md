# ST19 設計 — 個人属性を上書きせず履歴で残す

## Context

D-01 に本人が自分について書く枠は無い。`core.event` の `origin` は day one から `'authored'` を持ち、取り込み口（`/ingest`）は
本人が書いた記録を端末識別子なしで受け付ける（`crates/server/src/ingest.rs:131`）。
一方で **ST03 の錠と門は `origin = 'collected'` だけを見る**（`migrations/202609120944_gates.sql:40` / `:82` / `:241-248`）ので、
本人が書いた記録は書き換えも行の削除も通り、`tools/check-immutable.sh:147-159` はそれを OK として確かめている。
動機は `proposal.md` の Why。

深掘りで本人が決めた 4 件と、聞かないで決めた C1〜C13 は `deep.md`。**ここで決めるのは、それを実装に落とすときの技術判断だけ**で、
観測可能な振る舞いは `specs/personal-entities/spec.md` に置いてある。

## Goals / Non-Goals

**Goals**
- `personal-entities` を作り、個人属性の主張を積む・読む・画面で書く
- FR-44 の「書き換えない」を DB が強制し、FR-50 / FR-51 の開口部（削除の印・台帳つきの消去）だけを通す（Q1）
- S-6 マスタ管理の骨格（タブと 1 枚のカード）を置き、ST20 / ST21 が同じ画面に積めるようにする

**Non-Goals**
- **主張を消す操作**（画面・API）。ST22 / ST23。ST19 は錠を開けておき、読み出しが削除の印を効かせるところまで
- **人物・場所**（ST20 / ST21）。タブの置き場だけ
- **感度を変える操作**（ST24）/ **AI への経路**（ST27）/ **呼び出し元ごとの利用者の確かめ**（ST29。画面はほかの画面と同じく利用者を名乗らない）
- **主張を 1 日の一覧（S-2）や検索に出すこと**（ST25 / ST26）
- **住所を場所（ST21）に結ぶ・構造にすること**（C8）
- **`record-envelope` の要件の変更。** 主張は既存の約束のまま入る（D1）

## Decisions

### D1. 主張は `core.event` の 1 行。ソースは `s01-attribute`。取り込み口を通す

**`origin='authored'`、`logical_source='s01-attribute'` の行として、`/ingest` から入れる。**
製造準備 A-1（書き込みは自前の取り込み口 1 本に集め、冪等・原文保存を必ずそこに通す）に従う。
エンベロープ（FR-19〜FR-29）・読み出し（`core.event_live`）・削除の印（FR-50）・感度（PERM-2）・書き出し（ST33）が既存のまま掛かる。

| 列 | 入れるもの |
|---|---|
| `id` | 主張の識別子（画面が振る UUID。FR-21） |
| `event_time` / `tz_offset_min` / `tz_id` | **主張した日時**（画面が「積む」を押した時刻と、そのときの地域。C4 / C13） |
| `ingest_time` | D-01 に入った時刻（DB の既定。FR-19） |
| `external_id` / `external_ref` | 持たない（D5 で断る） |
| `device_id` | NULL |
| `raw` | 画面が組んだ JSON の**文字列**（下）。受け取ったまま（C11） |
| `payload` | **サーバが `raw` から組み直した値**（NFC。送り主の `payload` は使わない —— 原文と解析済みがずれる経路を作らない） |
| `sensitivity` | 2（ローカル AI まで。D3） |
| `content_hash` | 既存の `content_hash`（SHA-256(ソース, 出来事の時刻, 原文)） |

原文の形（`schema_version = 1`）:

```json
{"claim":"<id と同じ UUID>","nonce":"<128 bit の乱数を base64url>","kind":"<種類の UUID>",
 "value":"東京都 目黒区" | null,
 "valid_from":{"precision":"year"|"month"|"day"|"unknown","date":"2019"|"2019-10"|"2019-10-01"|null},
 "supersedes":"<取り消す主張の UUID>" | null,
 "note":"..." | null}
```

`payload` は `raw` から `nonce` を**除いた**同じ形（D4）。`value: null` が「なし」（C10）。

**書くたびに 1 件増える理由（C2）**: 原文に `claim` と `nonce` が入り、出来事の時刻が主張した日時なので、**内容の鍵は主張ごとに必ず違う**。
同じ原文の再送（通信が切れて画面が送り直す）は同じ鍵になり、既存の冪等で 1 件に畳まれる。**FR-22 / FR-50 の文面を変えない。**

**登録簿の行**: `('s01-attribute', '個人属性', 86400, 'none')`。`expected_gap_sec` は稼働状況に使われない（`coverage::must_sources()` は NFR-13 の 5 ソースだけ）が列が `NOT NULL`。
粒度 `none` のソースでは、送り主が外部識別子を付けても重複の判定にも更新の経路にも乗らない（`crates/server/src/lib.rs:344-356`）。

- **代わりに考えたもの**: 専用の表（`core.self_assertion`。旧 ashiato の案）。感度・削除の印・書き出しを表ごとに持ち直すことになり、
  ST22 / ST24 / ST33 が表の数だけ手を入れる。§5「扉ではないもの」がテーブルの置き場を移行できると明記している
- **代わりに考えたもの**: 主張専用の書き込み口（`POST /attributes/claims`）。A-1 の 1 本の口が 2 本になり、`/ingest` から検査を迂回して主張のソースへ書ける穴が残る

### D2. 主張の行の錠 —— 新しい関数で、ST03 の関数は書き換えない

移行に 3 つを足す。**ST03 の `core.reject_collected_rewrite` / `core.require_version_or_ledger` / `core.reject_collected_delete` は変えない**
（変えると ST03 の `.down.sql` と `tools/check-immutable.sh` の前提が動く。ST16 が `core.reject_truncate()` を使わなかったのと同じ理由）。

1. **即時の錠 `core.reject_claim_rewrite()`**（`BEFORE UPDATE`、行ごと）
   - `NEW.logical_source = 's01-attribute' AND OLD.logical_source IS DISTINCT FROM 's01-attribute'` → 拒む（付け替えで主張を捏造しない）
   - `OLD.logical_source = 's01-attribute'` のとき、`id` / `user_id` / `origin` / `logical_source` / `external_id` / `external_ref` / `device_id` /
     `event_time` / `tz_offset_min` / `tz_id` / `ingest_time` / `schema_version` / `unit_system` / `crs` / `source_updated_at` の変化を拒む。
     **`user_id` も凍結する**（ST03 は収集側の設定ミスを直す余地で凍結しなかったが、主張は取り消し先を同じ利用者の中で引くので、動かすと取り消しが別の利用者を指す）
2. **COMMIT 時の門 `core.require_claim_erasure_ledger()`**（`CONSTRAINT TRIGGER … DEFERRABLE INITIALLY DEFERRED`）
   - `OLD.logical_source <> 's01-attribute'` → 素通し
   - `raw` / `payload` / `content_hash` が変わらなければ素通し（削除の印・感度だけの書き換え）
   - 変わったなら、**ST03 と同じ消去の形**（`raw = ''` / `payload = '{}'` / `event_time` と `content_hash` は不変）かつ
     **同じトランザクションに `core.erasure_ledger` の `event_id = NEW.id AND scope = 'event' AND txid = pg_current_xact_id()` の行**があるときだけ通す
     （`gates.sql:112-116` と同じ照合。**その主張の行でなければ通さない** —— 別の記録の台帳 1 行で何件でも消去できる形にしない。spec-review R7）。
     消去の形でなければ台帳があっても拒む（主張には前の版の経路が無い。台帳 1 行で値を植え替える ST03 の R51 / R95 と同じ穴を作らない）
3. **行の削除の拒否 `core.reject_claim_delete()`**（`BEFORE DELETE`、行ごと）。表の切り詰めは ST03 の文トリガが既に拒む

`tools/check-immutable.sh` の「本人が書いた記録は書き換えられる」の段は、**いまは `WHERE origin = 'authored'` で本人が書いた全行を書き換える**（`tools/check-immutable.sh:154-155`）。
主張の行が DB にあるとこの文は門で落ちるので、**段の `WHERE` を `logical_source = 'immutable-check'` に絞り、主張の行を入れた後で通ることを確かめる**（spec-review R14。当初は「そのまま残る」と書いたが事実と違った）。

### D3（仮）. 既定の感度はコードの分岐で持つ

`ingest_one` の INSERT で、`logical_source = 's01-attribute'` なら `sensitivity = 2`（ローカル AI まで。PERM-4 / Q3）を入れる。それ以外は DB の既定（1）のまま。
**取り込みの契約に感度の欄は足さない**（ST16 の Q3 の context (b) と同じ。足すと `record-envelope` と収集側の契約が動く）。

- **反転条件**: ST24 が登録簿に「ソースごとの既定の感度」を持たせたとき、この分岐を登録簿の値に移す（値は 2 のまま）

### D4. 消去の後に値を当てられない —— 原文にだけ乱数を入れる

画面が主張ごとに 128 bit の乱数（`crypto.getRandomValues`）を原文の `nonce` に入れる。**`payload` にも他の列にも写さない。**
消去は `raw` と `payload` を空にするので乱数も消え、残った `id` / `event_time` / `content_hash` と値の候補から鍵を作り直せない（C12）。
`claim`（= `id`）だけでは守れない —— `id` の列は消去の後も残る（deep-review R3 の指摘を実装に落とすときに分かった）。

サーバは `nonce` が 16 バイト以上（base64url で 22 文字以上）であることを検査する（D5）。短い乱数は総当たりの範囲に入る。
サーバは乱数を作らない（画面が組んだ原文を受け取ったまま保存するので、サーバが足すと原文が変わる）。

**Rust のテストは画面の組み立てを呼べない**ので、C12 の確かめは 2 か所に分ける —— 乱数の強さと導き方（識別子と一致しない・同じ内容でも異なる）は `web` の `attributes.test.ts`、
「乱数を知らなければ鍵を作り直せない」と「乱数が解析済みに写らない」は Rust の結合テスト。わざと壊す確かめは、画面の組み立てで乱数を識別子にすると `attributes.test.ts` が落ち、
サーバで `payload` に乱数を残すと結合テストが落ちること（spec-review R2）。

### D5. 主張の検査の場所と、理由の種別

`crates/server/src/attributes.rs` に原文の解釈と形の検査を置き、`ingest_one` は `logical_source = 's01-attribute'` のときだけ呼ぶ。
**DB を見る検査（種類がその利用者にあるか / 取り消す主張があるか）は、記録を入れるのと同じトランザクションで行う**（競合の扱いは下の「取り消し先は行錠を取らずに読む」）。

`IngestError` に足す種別と当たる条件は spec「形の合わない主張は受け付けない」の表（**置き場は spec**。spec-review R5 / R12）。
実装は `ClaimInvalid` の列挙から `IngestError` へ写す。検査の順は、形（`malformed_claim`）→ 由来と端末（`claim_not_authored`）→ 外部識別子 → 値 → 「いつから」→ DB を見る 2 つ（種類・取り消し先）。

**応答は既存の 1 件ごとの結果の形のまま**（識別子と種別だけで、値を含まない）。ログにも値・補足・種類の名前を出さない（製造準備 A-2）。

**取り消し先は行錠を取らずに読む**（spec-review R17）。読んだ直後に別のまとまりが取り消し先に削除の印を付けても、本文を消去しても、
**読み出しは消した主張・消去した主張を出さない**（D6 の手順 1）ので、その取り消しは効く相手がいないだけで害が無い。spec も「消した主張を取り消し先に指せる」を認めている。
当初は「同じトランザクションで読むので競合を避ける」と書いたが、既定の分離レベルでは避けられないので理由を差し替えた。

### D6（仮）. いまの値の導き方

`attributes.rs` の純粋な関数 `view(kinds, claims, today) -> AttributesView` で組む（DB を持たないので単体テストで固定する）。

1. 削除の印の付いた主張（`core.event_live` で除く）と、**本文を消去した主張（`raw = ''`）** を除く。消去した主張は種類も値も読めないので、並べる置き場を持たない（spec-review R18）
2. 残った主張の `supersedes` を集め、指された主張を「取り消された」側へ移す（取り消した主張の識別子を添える）
3. 残りを種類ごとに、比較の鍵（精度 `unknown` → 最小 / `year` → その年の 1 月 1 日 / `month` → その月の 1 日 / `day` → その日）→ 主張した日時 → D-01 に入った時刻で並べる
4. 鍵が今日（`Asia/Tokyo`）以前のうち最後が「いまの値」、今日より後は「予定」（鍵の昇順）
5. 積んだ主張は鍵の降順・同じ鍵は主張した日時の降順（精度 `unknown` は最後）

- **反転条件**: 本人が同じ種類に同時に 2 つの値を持ちたいと言ったとき（Q4 の第 2 選択肢へ）/ 年だけの主張をその年の初めから有効とみなすことに違和感が出たとき（年末までは前の値のまま、などに）/
  日の区切りを旅行先の地域にしたくなったとき。どれも主張を書き換えずに計算し直せる

### D7（仮）. 種類は別の表で、名前は追記のみ。住所と職業は最初の読み出しで置く

種類は記録ではない（本人の出来事ではなく、主張を束ねる器）ので `core.event` に入れない。

```sql
core.attribute_kind      (id uuid PK, user_id uuid NOT NULL, created_at timestamptz NOT NULL DEFAULT now())
core.attribute_kind_name (id bigint IDENTITY PK, kind_id uuid NOT NULL, -- (kind_id, user_id) → attribute_kind (id, user_id)
                          user_id uuid NOT NULL, name text NOT NULL CHECK (name <> ''), created_at timestamptz NOT NULL DEFAULT now())
```

- いまの名前は `kind_id` ごとに `id` が最大の行。**2 表とも UPDATE / DELETE / TRUNCATE を拒む**（ST16 の `stay_criteria` と同じ形。台帳は追記のみ）
- `core.attribute_kind_name` の `(kind_id, user_id)` は `core.attribute_kind` の `(id, user_id)` を外部キーで指す（`attribute_kind` に `UNIQUE (id, user_id)`）—— **種類と名前の利用者が食い違う行を DB が作らせない**（spec-review R16）
- 名前は NFC にして入れる。**いまの名前との重なり**は、利用者ごとの `pg_advisory_xact_lock(<鍵>, hashtext(user_id::text))` を取ったトランザクションの中で確かめて断る（一意索引は「いまの名前」を表せない）。
  名前を変える口は、同じ錠の中で**その種類がその利用者のものか**を確かめ、違えば無い種類と同じ 400
- **住所と職業の初期化**（`ensure_initial_kinds`）: `GET /attributes` と `POST /attributes/kinds` の**両方の先頭で**、同じ利用者ごとの錠を取ってから、その利用者の種類が 0 件なら 2 つを入れる。
  識別子は `uuid` の v5（利用者の UUID を名前空間に、`address` / `job`）。**名前の行は種類の `INSERT … ON CONFLICT DO NOTHING RETURNING id` が行を返したときだけ入れる** ——
  種類の主キーだけで衝突を止めると、同時に 0 件を見た 2 本が名前の行を 2 本ずつ積み、追記のみの台帳から消せない（spec-review R15）。錠を取るので、初期化と「住所」を足す要求が重なっても、いまの名前が「住所」の種類は 1 つ。
  `uuid` の `v5` 機能を足す（同じ crate。ライセンスは変わらない）
- 口: `POST /attributes/kinds`（`{user_id, name}` → `{id}`）/ `POST /attributes/kinds/{id}/names`（`{user_id, name}`）。空・重なる・無い（別の利用者の）種類は 400
- **反転条件**: 読み出しに書き込みがあることが問題になったとき（読み取り専用の複製から読む、など）—— 住所と職業を明示の初期化の口へ移す。識別子は v5 なので変わらない

### D8. 読み出しの口 `GET /attributes`

`GET /attributes?user_id=`（省けば nil UUID。ほかの読み出しと同じ。ST29 まで利用者は名乗り）。応答:

```json
{"today":"2026-09-15",
 "kinds":[{"id":"…","name":"住所",
   "current":Claim|null, "upcoming":[Claim], "claims":[Claim], "superseded":[Claim]}]}
Claim = {"id","value":string|null,"valid_from":{"precision","date"},"asserted_at":"RFC 3339（地域のずれつき）",
         "ingested_at":"RFC 3339","supersedes":id|null,"superseded_by":id|null,"note":string|null}
```

- 並び・含むもの・感度で絞らないことは spec「種類ごとのいまの値と履歴を読める」に置いた（spec-review R12）。種類の「作った順」は `core.attribute_kind.created_at`、同時刻は `id`
- OpenAPI に載せる（`tools/check-openapi.sh`）

### D9. 画面 —— `#/master`、1 枚のカードに全部

`web/src/MasterView.tsx` と `web/src/attributes.ts`（型・形の検査・日付の書き方・原文の組み立て）。`Root.tsx` に `#/master` を足す。
**`#/day/…` とルート（S-1）は変えない**（ST16）。稼働状況（`App.tsx`）の見出しの並びに「マスタ管理」への入口を 1 つ置く（仮。入口の置き場を決め直すのは ST25）。

構造は本人が proto で決めたとおり（Q2 の逐語。`deep.md`）:

- タブ 1 つ（「個人属性」）。**人物・場所のタブは描かない**（押しても何も無いタブを置かない。ST20 / ST21 が足す）。タブの並びは `role="tablist"`
- 種類ごとのカード: 種類の名前（押すと名前を変える欄）/ いまの値（無ければ「まだ書いていない」、`null` なら「なし」）と「いつから」/ 予定 /「書く」/ **積んだ主張の全部**
- 主張の行は `<button aria-expanded>`。**値・「いつから」・補足は常に見え**、押すと主張した日時を出す（補足の扱いは D13）
- 取り消された主張は「訂正で取り消した N 件」の `<button aria-expanded>` に畳む
- 「書く」を開くと: 「変わった」/「前の書き込みが間違っていた」（ラジオ）→ 後者なら取り消す主張の選択（既定は主張した日時が最も新しい、取り消されていない主張）/
  値の欄と「なし」のチェック / 精度のラジオ（年・年月・年月日・分からない）と、選んだ精度の欄だけ / 補足 /「積む」「やめる」
- 画面の下に「種類を足す」
- **proto で描いた「いまの値を大きく」と「書いた日を押したときだけ」はそのまま**。表面は `tokens.ts`（`SCHEMES` と `tone`）だけから引く

**「積む」の組み立て**（`attributes.ts`）: 押した時点で `id`（`crypto.randomUUID()`）・`nonce`・主張した日時（`new Date()` と `Intl` の地域）を決め、原文を組んで
`POST /api/ingest` に 1 件で送る。**入力を変えずに押し直した・届かずに送り直すときは同じ原文を再送する**（冪等で 1 件。spec「二重に押しても 1 件」）。
入力を 1 か所でも変えたら組み直す（同じ `id` で別の原文を送ると `id_reused` で断られる）。送っている間は「積む」を押せなくする。
受理なら `GET /attributes` を読み直してフォームを閉じる。受理でなければ種別を文に直して出し、**入力は消さない**。

**1 件だけ送って断られると `/ingest` は HTTP 400 を返す**（1 件も受け付けなかったとき。`lib.rs:892-896`）。画面は既存の読み出しの形（`if (!res.ok) throw`。`App.tsx:53`）を写さず、
**200 でも 400 でも本文の 1 件ごとの結果を読み、`accepted` と `error` で分ける**。本文が読めない・届かない（`fetch` が投げる / 5xx / 401）ときだけ「届かなかった」とする（spec-review R19）。

| 種別 | 画面の文 |
|---|---|
| `invalid_claim_value` | 値が空です。値を入れるか「なし」を選んでください |
| `invalid_valid_from` | 「いつから」の日付が読めません |
| `unknown_attribute_kind` | この種類が見つかりません。画面を読み直してください |
| `invalid_supersedes` | 取り消す主張が見つかりません。画面を読み直してください |
| それ以外 | 受け付けられませんでした（種別の名前） |
| 届かなかった | サーバに届きませんでした。入力はそのまま残っています |

### D10. 変えないもの

- `record-envelope` の要件と取り込みの契約の形（`IngestRequest` の欄）。足すのは理由の種別の値だけ（`check-openapi.sh` は欄の名前を見るので Kotlin 側は変わらない）
- 収集側（`collector-android` / `collector-windows`）。主張のソースへは送らない
- ST03 の錠と門の関数、ST16 の滞在の経路と行き先
- `tools/check-immutable.sh` の既存の確かめ（足すだけ）

### D11. 完了の判定を機械で確かめる

- 「住所を 2 回変えると、3 つの主張が残っている」→ 結合テスト（Scenario: `住所を 2 回変えると 3 つの主張が残る`）と `tools/smoke.sh`（A → B → A と書いて 3 件）
- 「各主張に『いつそう書いたか』と『いつからそうだったか』が別々に入っている」→ 結合テスト（Scenario: `主張した日時といつからが別々に入る`）と、
  画面のテスト（Scenario: `書いた日時は主張を押したときだけ出る`）

### D12. 移行は 1 本

`migrations/YYYYMMDDHHMM_personal_attributes.sql`（作成時刻）と `.down.sql`。中身は D2 の 3 関数とトリガ、D7 の 2 表と錠、登録簿の 1 行。
**当て直せる形**（`IF NOT EXISTS` / `CREATE OR REPLACE` / `DROP TRIGGER IF EXISTS` / `ON CONFLICT DO NOTHING`）。`MIGRATIONS` 配列の末尾に足す。
`.down.sql` は、**主張の行か種類の行が 1 つでも残っていれば、登録簿の行も種類の 2 表も残す**
（当初は「主張の行」だけを見ていたが、**まだ主張を書いていない種類が戻しで消えた** —— 本人が名前を
決めたという事実そのものが成果物で、台帳は追記のみなので作り直せない。review/code.md R7）（`DELETE FROM core.source WHERE logical_source = 's01-attribute' AND NOT EXISTS (SELECT 1 FROM core.event WHERE logical_source = 's01-attribute')` と、
同じ条件の `DO` ブロックで 2 表を落とす）。錠の関数とトリガは落とす。主張が原文の中で指す種類の識別子の名前を、戻しで失わないため（spec-review R20。当初は「外部キーで当たる」を前提にしていたが、当たると戻しが途中で止まる）。

### D13（仮）. 補足は行に常に出す

本人が見た proto（`proto.html` の `claimRow`）は、どの設定でも補足を行の中に出していた。Q2 で押したときだけにしたのは**主張した日時**だけなので、補足はそれに合わせない。
当初の spec と D9 は補足も押したときだけにしていたが、本人が選んでいない軸を足していた（spec-review R1）。

- **反転条件**: 補足が長く、カードが読みにくいと本人が言ったとき（押したときだけにする / 1 行で切る）。どれも表示だけで、保存と読み出しは変わらない

### D14. 種類の並びは単調増加の列で決める（`created_at` では決まらない）

**下流で分かった。** D8 は「種類の『作った順』は `core.attribute_kind.created_at`、同時刻は `id`」と書いていたが、
PostgreSQL の `now()` は**トランザクションの開始時刻**なので、**同じまとまりで作った種類は全部同時刻**になる。
住所と職業は `ensure_initial_kinds` の 1 つのまとまりで置かれ、`POST /attributes/kinds` も
**同じまとまりで初期化してから足す**ので、実際には 3 つとも同時刻に並ぶ。
tie を割る `id` は v5 / v4 の UUID なので、**並びがでたらめになる**（実測: 住所・職業・副業 →「副業・職業・住所」）。

`core.attribute_kind` に `seq bigint GENERATED ALWAYS AS IDENTITY` を持たせ、読み出しは `ORDER BY k.seq` で引く。
`created_at` は「いつ作ったか」の事実として残す（並びには使わない）。

- **代わりに考えたもの**: `clock_timestamp()`（文ごとの実時刻）。同じマイクロ秒に入れば tie が残り、
  そのとき何が起きるかが「たまたま」になる。spec は「作った順に返る」と言い切っているので、順序は正確に決まるほうがよい
- **代わりに考えたもの**: 名前の台帳の `MIN(id)`（既にある単調増加の列）で並べる。種類の並びが名前の台帳の
  書き方に依存し、名前を変える実装を触ると並びが動きうる

## Risks / Trade-offs

- [画面が長い（10 年後の量で 4.3 画面、1 画面目にいまの値は 1 種類）] → 本人が承知で選んだ（Q2）。畳む形に戻さない。種類が増えて読めなくなったら、そのとき深掘りで問い直す
- [主張した日時は画面の端末の時計] → 時計がずれていると「いつ書いたか」がずれる。D-01 に入った時刻（サーバの時計）が別の欄に残るので、後から突き合わせられる。画面の時計のずれは測らない（FR-7 は C-01 のもの）
- [読み出しに書き込みがある（D7）] → 最初の 1 回だけ。v5 の識別子と `ON CONFLICT` で同時でも増えない。反転条件は D7
- [利用者を名乗れる（`docs/production-prep.md:167`）] → 他の利用者を名乗って主張を書ける・読める。ST29 の担当。ST19 は取り消し先の検査と名前の変更を利用者の中に閉じる（D5 / D7）ので、名乗った利用者の外の主張と種類は壊せない。
  **任意の利用者の UUID で読み出すたびに、その利用者の住所と職業（種類 2 行・名前 2 行）が消せない行として増える**（D7 の初期化。spec-review R15 (d)）。1 回の読み出しで 4 行なので、呼び出し元が限られる ST29 まで受け入れる
- [`payload` を原文から組み直す] → 送り主の `payload` は捨てる。原文から組むので失われる値は無い（C11）
- [ST04 の下流と同じファイル（`lib.rs` の `MIGRATIONS` と route / `docs/openapi.json` / `tools/*.sh`）に追記する] → 後から merge する側が追従する。移行の名前は作成時刻で取り合わない

## Migration Plan

1. 移行を当てる（起動のたびに全版を当てる既存の形。`migrate()`）
2. 既存の行は触らない（主張のソースの行はまだ無い）
3. 戻すときは `.down.sql`。主張の行があるうちは登録簿の行が消えない（D12）
