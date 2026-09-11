# ST03 設計 — 同じ記録を何度送っても増えない

## Context

現物は ST01 が作った取り込み経路（`crates/server/src/{lib.rs,ingest.rs}` と移行 0001〜0004）。
動機は `proposal.md`（Why）、本人が決めたことは `deep.md`、振る舞いは `specs/` を見ること。

ここで押さえる前提は 3 つ。

1. **移行は 0007 から。** ST02（PR #20・未 merge）が 0005 / 0006 を使う
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
design D9 / D20 の「1 件の恒久的な失敗が後続を永久に止めない」に正面から反する。

**1 件ごとにトランザクションを張る。** 取り込みと稼働記録の書き込みは**同じ**トランザクションに
束ねる（R13。いまは別々に撃っており、更新経路ができると「履歴だけ残って本表が古い」が起きる）。

### D4. 門は遅延制約トリガ ＋ トランザクション識別子

```sql
ALTER TABLE core.event_version ADD COLUMN txid xid8 NOT NULL DEFAULT pg_current_xact_id();
CREATE CONSTRAINT TRIGGER event_requires_version
  AFTER UPDATE ON core.event DEFERRABLE INITIALLY DEFERRED
  FOR EACH ROW EXECUTE FUNCTION core.require_version_or_ledger();
```

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
| `external_id_kind` | `text` | `'record'` | ST03（400 の判定） |
| `retired_on` | `date` | `NULL` | **ST02**（状態・途絶・通知・分母） |
| `succeeds` | `text` | `NULL` | **ST02**（収集開始日を引き継ぐ） |

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

## Risks / Trade-offs

- **1 件 1 トランザクションで取り込みが遅くなる** → まとめ送りは 5 分間隔・200 件が上限で、
  NFR-1 の上限は 1 時間。余裕がある。測って遅ければ、門の要らない新規挿入だけを束ねる
- **門の分岐が「原文が空」を印にしている** → `ingest.rs` の空の原文の拒否が外れると印が壊れる。
  **その拒否をテストで固定してある**（`raw_that_cannot_be_stored_is_rejected`）ので、外れれば落ちる
- **台帳の件数を DB が検算しない**（実測: 台帳 1 行で 4 行消せた）→ 台帳の行と実際の消去の
  突き合わせは tasks の検査でやる。DB でやるには消去の対象を台帳に列挙させることになり、重い
- **ST02 との衝突** → `docs/stories/stories.json` は両方が触る（別の Story の項目）。
  ST03 が先に merge され、ST02 が rebase する順序で合意済み

## Migration Plan

0007 から順に、**前進のみ**（戻し手順は `.down.sql`）。

1. `0007` 登録簿の 3 列（`external_id_kind` は既定 `'record'`）＋ 既存 5 か所の端末ソースに `'none'`
2. `0008` 記録の 2 列（`source_updated_at` / `external_ref`）
3. `0009` 索引の作り替え（D1）
4. `0010` 履歴表と消去の台帳（`raw` は `text`。追記のみのトリガ）
5. `0011` 門のトリガ（D4）と、凍結列の追加・`DELETE` / `TRUNCATE` の錠（D5 / D10）

**`0009` より前に `ON CONFLICT` を直す**（D2）—— 索引を作り替えた瞬間に取り込みが落ちる。
移行とコードの順序が逆だと、その間の取り込みが全件 500 になる。

## Open Questions

- **履歴の上限**（無制限に積むか、古い版を捨てるか）。Q20 で往復による無限増加は止まったので、
  残るのは外部サービスが正当に何度も更新する場合だけ。**後から足せる**（可逆）ので、
  実データの増え方を見てから決める。NFR-5（年 3〜6 GB）に対する影響を ST30 の前に測る
