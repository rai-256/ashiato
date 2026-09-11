# ST03 設計 — 同じ記録を何度送っても増えない

## Context

現物は ST01 が作った取り込み経路（`crates/server/src/{lib.rs,ingest.rs}` と ST01 の 4 本の移行 `202609081618_envelope` 〜 `202609100000_immutable_origin`）。
動機は `proposal.md`（Why）、本人が決めたことは `deep.md`、振る舞いは `specs/` を見ること。

ここで押さえる前提は 3 つ。

1. **移行の名前は作成時刻 `YYYYMMDDHHMM_<slug>.sql`。連番ではない**（2026-09-12 に変えた —— ST02 と並走して 0007 を取り合い、番号をずらす PR が要った）。ST02（merge 済み）は `202609111111_coverage_rebuild` / `202609111112_immutable_heartbeat` / `202609112113_source_lifecycle`（退役と引き継ぎの列）まで使っている。適用の順は `crates/server/src/lib.rs` の `MIGRATIONS` 配列が持つ
2. **`collection-coverage` には触らない。** 退役の状態と途絶・通知の除外は ST02 の担当で、
   PR #20 にコメントで渡してある。ST03 が作るのは**登録簿の列だけ**（登録簿は `record-envelope`）
3. **CI が「拒まれること」を検査している 4 列を、この change が開ける。**
   `tools/check-immutable.sh` を作り替えないと CI が落ち、**消すと守りが黙って消える**

## Goals / Non-Goals

**Goals** —— `specs/` の振る舞いを満たすこと。とくに、**書き換えと消去の門を DB 側に置く**こと。

**Non-Goals**

- 畳んで読む形の**適用**（閲覧・検索・AI・書き出し）。ST03 は置き場だけを作る（Q8）
- 派生の作り直し（FR-31）。ST16（Q7）
- 「対象ごと」の識別子しか持たないソースで外部の更新を追うこと（Q25）
- 本文の物理削除そのもの（FR-51）。ST03 が作るのは**台帳と門**で、消す操作は ST23
- 稼働記録の状態・途絶・通知・達成日（ST02）

## Decisions

### D1. 索引を 2 段にする —— 内容側は部分索引

```sql
CREATE UNIQUE INDEX event_dedup_ext  ON core.event (user_id, logical_source, external_id)
  WHERE external_id IS NOT NULL;
CREATE UNIQUE INDEX event_dedup_hash ON core.event (user_id, logical_source, content_hash)
  WHERE external_id IS NULL;
CREATE INDEX event_hash_all ON core.event (user_id, logical_source, content_hash);  -- 非一意
```

Q6（外部識別子を優先）を成り立たせるには内容側を狭めるほかない。実測で確認済み。
3 本目は一意ではなく、Q8 の畳み込みと Q19 の削除済みの判定が使う（両方が同じ索引に乗る）。

**代替案**: 内容側を一意のまま残す → Q6 の「識別子が違えば別の記録」が成立しない（実測で一意違反）。

**`content_hash` の作り方は ST01 のまま**（`logical_source` + `event_time` + `raw`）。
Q15 で「利用者識別子は索引にだけ」と決まったので、`hash_is_pinned` の期待値は変わらない。

### D2. `ON CONFLICT` は部分索引の述語ごと書く

`ON CONFLICT (logical_source, content_hash)` は D1 の索引に対して
`there is no unique or exclusion constraint matching …` で**文として落ちる**（実測）。
`user_id` を足しただけでも同じ。述語（`WHERE external_id IS NULL`）を文に書く。

**外部識別子を持つ記録と持たない記録で撃つ文が変わる**ので、取り込みは
「外部識別子があるか」で経路を分ける。

### D3. 取り込みを 1 件 1 トランザクションにする

Q10 / Q23 の門は**制約トリガ**で実装する（D4）。制約トリガは **COMMIT 時に落ちる**ので、
まとめ送りを 1 トランザクションにすると **1 件の失敗が全件を巻き戻す** ——
**ST01 の `design.md` の D9 / D19**（まとめ送りの部分失敗で成功分だけを取り除く / 500 は
まとめ送り全体を落とすので 1 件の恒久的な失敗が後続を永久に止める）に正面から反する。

**1 件ごとにトランザクションを張る。** 取り込みと稼働記録の書き込みは**同じ**トランザクションに
束ねる（R13。いまは別々に撃っており、更新経路ができると「履歴だけ残って本表が古い」が起きる）。

### D4. 門は遅延制約トリガ ＋ トランザクション識別子

**門の最終形**: 「同じトランザクションに**履歴行**がある書き換え、または同じトランザクションに
**台帳行**がある消去だけを通す」。本表と履歴の**両方**に置く —— 片方だけだと、
本表は消せるのに履歴が消せない「消せない DB」が残る（R41 / R52）。

```sql
-- *_version_and_ledger の移行で 2 表とも txid を持って作る（この列は後付けではない）
CREATE TABLE core.event_version (
  …, raw text NOT NULL, txid xid8 NOT NULL DEFAULT pg_current_xact_id()
);
CREATE TABLE core.erasure_ledger (
  …, txid xid8 NOT NULL DEFAULT pg_current_xact_id()
);
-- *_gates の移行で門を 2 本
CREATE CONSTRAINT TRIGGER event_requires_version
  AFTER UPDATE ON core.event DEFERRABLE INITIALLY DEFERRED
  FOR EACH ROW EXECUTE FUNCTION core.require_version_or_ledger();
CREATE CONSTRAINT TRIGGER version_requires_ledger
  AFTER UPDATE ON core.event_version DEFERRABLE INITIALLY DEFERRED
  FOR EACH ROW EXECUTE FUNCTION core.require_ledger_for_erasure();
```

**履歴の `UPDATE` は「台帳のある消去」だけが通る**（Q17 の答え。Q21 で感度と削除の列が消えたので、
残る正当な操作は本文の消去 1 つだけ）。台帳は無条件で追記のみ。

**消去は親とその記録のすべての履歴を同じトランザクションで消す。** 分けると、
2 段目（履歴だけを消す操作）が R40 の開口部そのものになる。

「この書き換えと同じトランザクションで履歴行（または台帳行）が書かれたか」を COMMIT の瞬間に見る。
実測: 履歴を書かない UPDATE は COMMIT で落ち、**順序に依存しない**（更新の後に履歴を書いてもよい）。
**同じトランザクションの中で履歴を消しても落ちる。**

**門は「消去の形」で分岐する。** 台帳行があるだけで通すと、台帳を 1 行書いて改竄が通った（実測）。
消去（原文が空になる）のときだけ台帳を見て、それ以外は履歴を見る。
**原文が空であることを印にできるのは、`ingest.rs` が空の原文を受け口で断っているため**
（正規の取り込みでは空の原文が入らない）。

**代替案**: `SECURITY DEFINER` の関数だけに更新権限を与える → 取り込み口の書き忘れを DB が止められない。

### D5. 錠は `UPDATE` だけでなく `DELETE` と `TRUNCATE` にも掛ける

**いまの 0002 / 0004 は `UPDATE` しか見ていない。** 実測で `DELETE FROM core.event` が 3 行消し、
履歴表の `TRUNCATE` も通った（行トリガは `TRUNCATE` で撃たれない）。
本表・履歴・台帳の 3 つに、行の削除を拒む `BEFORE DELETE` と `BEFORE TRUNCATE` を置く。

### D6. 履歴は感度と削除の印を持たない（Q21）

持たせると、親を締めても前の版が緩いまま残る（実測）。**伝播の処理が存在しなければ書き忘れも無い。**
読み出しは親と束ねたビュー（`core.event_version_live`）越しにだけ行う ——
本表が `core.event_live` で同じ危険を塞いでいるのと同じ手当て（A-3）。
**履歴の `raw` は `text`**（理由は移行 0003 と同じ。`jsonb` はキー順・重複キー・数値表記を保たない）。

### D7. 登録簿の 3 列

| 列 | 型 | 既定 | 誰が読むか |
|---|---|---|---|
| `external_id_kind` | `text` | `'record'` | ST03（受け付けるかの判定） |
| `retired_on` | `date` | `NULL` | **ST02 が作り、ST02 が読む**（状態・途絶・通知・分母） |
| `succeeds` | `text` | `NULL` | **ST02 が作り、ST02 が読む**（収集開始日を引き継ぐ） |

> **2026-09-11 の判断。** 当初は 3 列とも ST03 が作る設計だったが、**`retired_on` と `succeeds` は
> ST02 が作る**ことにした。理由は 2 つ —— (a) 読む側が稼働記録（`collection-coverage`）で、
> ST03 は 1 度も読まない。**列の意味は退役と引き継ぎで、記録の骨格ではない** (b) ST02 は
> 実装が済んでおり、この 2 列が無いと 8 状態と引き継ぎを実装もテストもできない。
> **ST03 は `external_id_kind` だけを作る。**

`external_id_kind` の既定を `'record'` にするのは Q16 / Q18 —— 書き忘れると全件 400 になって気付く。
**既存の登録は 5 か所に手書きで散っている**（`tools/seed.sh` / `tools/smoke.sh` ×2 /
`tools/check-immutable.sh` / `collector-android/README.md`）ので、
**端末のソースに `'none'` を明示する移行が要る**（書かないと断られる）。

`retired_on` を日付にするのは Q26 / R56 —— 真偽値だと退役より前の本物の途絶が遡って消える（実測）。

### D8. 記録の 2 列

| 列 | 型 | 用途 |
|---|---|---|
| `source_updated_at` | `timestamptz` | Q20。古い到着で書き換えない判定 |
| `external_ref` | `text` | Q24。対象ごとの識別子。**索引を張らない**（判定に使わない） |

`source_updated_at` の**型が論点**だった（Q20 の文面は「更新時刻**または版**」）。
時刻型で作ると不透明な版（ETag・世代番号）を後から入れられない。
**時刻型で作り、不透明な版しか返さないソースが出たら「届いた順」に倒す**
（ソース単位ではなく**記録単位**で判定する —— 同じソースでも項目が欠ける到着がある）。

**同じ更新時刻で内容だけ違う到着は「新しい」として扱う**（`>=`）。
`>` にすると `accepted` を返しながら内容が変わらず、**応答から見えない**。

### D9. 削除済みの判定は挿入と更新の両方に当てる

```sql
SELECT 1 FROM core.event
 WHERE user_id=$1 AND logical_source=$2 AND content_hash=$3 AND deleted_at IS NOT NULL LIMIT 1;
```

D1 の 3 本目の索引に乗る。実測: `Index Scan` 1 回・バッファ 2 ページ・0.058 ms。
**外部識別子を持つ記録のときだけ撃つ** —— 持たない記録は `event_dedup_hash` が
削除済みの行も含めて弾くので、判定は要らない（実測）。

**更新の経路にも当てる。** 当てないと、生きている別の行が外部からの更新で消した本文に化ける（実測）。

### D10. 凍結する列を増やす

`external_id` と `external_ref` を凍結一覧に足す。**いまの 0004 に `external_id` が無く、
Q6 の部分索引の下では識別子を 1 文書き換えるだけで同じ本文が 2 行入る**（実測。履歴も台帳も残らない）。
`source_updated_at` は**凍結しない**（更新のたびに動く）。

### D11. `check-immutable.sh` を作り替える

いまは `raw` / `payload` / `event_time` / `content_hash` の UPDATE が**拒まれること**を CI で検査している。
ST03 がこの 4 列を開けるので、**「履歴を書かない書き換えは拒まれる / 書けば通る」の 2 本**にする。
D5 の `DELETE` / `TRUNCATE`、D10 の 2 列、台帳の追記のみも同じ台本に入れる。

### D12. 分割は取り込む前に行う。後から分けたら過去分は古い名前のまま

`content_hash` の入力に `logical_source` が入っている（ST01 の `ingest.rs`。
`field_boundaries_are_unambiguous` がソース名の違いで鍵が変わることを固定している）ので、
**ソース名を動かすと鍵が変わる**。分けたあとに同じ書庫を入れ直すと**過去分が全件二重に入る**（実測）。

**原則は「取り込む前に分ける」。** やむを得ず後から分けたときは、
**過去分を古い名前のまま残す**（Q22）—— 移し替えると鍵が古い名前で計算されたまま残り、
その行は以後どの再送とも一致しない（重複判定が黙って当たらない行ができる）。

分かれた名前をどう見せるかは `collection-coverage`（ST02）の担当。
古い名前には `retired_on` が立ち、`succeeds` で新しい名前へ繋がる（D7）。

## Risks / Trade-offs

- **1 件 1 トランザクションで取り込みが遅くなる** → まとめ送りは 5 分間隔・200 件が上限で、
  NFR-1 の上限は 1 時間。余裕がある。測って遅ければ、門の要らない新規挿入だけを束ねる
- **門の分岐が「原文が空」を印にしている** → `ingest.rs` の空の原文の拒否が外れると印が壊れる。
  **その拒否をテストで固定してある**（`raw_that_cannot_be_stored_is_rejected`）ので、外れれば落ちる
- **台帳の件数を DB が検算しない**（実測: 台帳 1 行で 4 行消せた）→ 台帳の行と実際の消去の
  突き合わせは tasks の検査でやる。DB でやるには消去の対象を台帳に列挙させることになり、重い
- **ST02 との衝突** → `docs/stories/stories.json` は両方が触る（別の Story の項目）。
  merge の順序は約束しない —— `merge_gate.sh` が main に rebase するので、後から merge する側が追従する
  （★ 2026-09-12 訂正。当初「ST03 が先に merge」と書いたが、実際は ST02 が先だった）

## Migration Plan

5 本を順に、**前進のみ**（戻し手順は `.down.sql`）。名前は作成時刻 `YYYYMMDDHHMM_<slug>.sql`
（連番ではない。ST02 が `202609112113_source_lifecycle` まで使っている）。順は `lib.rs` の `MIGRATIONS` 配列に足す順で決まる。

1. `*_source_columns` 登録簿の **1 列**（`external_id_kind`。既定 `'record'`）＋ 既存 5 か所の端末ソースに `'none'`
2. `*_event_columns` 記録の 2 列（`source_updated_at` / `external_ref`）
3. `*_dedup_indexes` 索引の作り替え（D1）
4. `*_version_and_ledger` 履歴表と消去の台帳（`raw` は `text`。追記のみのトリガ）
5. `*_gates` 門のトリガ（D4）と、凍結列の追加・`DELETE` / `TRUNCATE` の錠（D5 / D10）

**`*_dedup_indexes` より前に `ON CONFLICT` を直す**（D2）—— 索引を作り替えた瞬間に取り込みが落ちる。
移行とコードの順序が逆だと、その間の取り込みが全件 500 になる。

## Open Questions

- **履歴の上限**（無制限に積むか、古い版を捨てるか）。Q20 で往復による無限増加は止まったので、
  残るのは外部サービスが正当に何度も更新する場合だけ。**後から足せる**（可逆）ので、
  実データの増え方を見てから決める。NFR-5（年 3〜6 GB）に対する影響を ST30 の前に測る
