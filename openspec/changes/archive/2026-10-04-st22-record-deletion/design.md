# ST22 設計 — 記録を消したことにできる

## Context

いま `core.event` には削除の 2 列（`deleted_at` / `deleted_by`）があり、読み出し（`core.event_live`）も
取り込みの側の再流入の防止（ST03）も派生の作り直しの側の「消した時間帯に戻さない」（ST16）も**それを前提に動いている**。
足りないのは**印を付ける操作**だけで、いま `deleted_at` を書くのは ST16 の作り直し・down 移行・テストだけである。
動機は `proposal.md` の Why。深掘りで本人が決めた 4 件と、聞かずに決めた既定 C1〜C10 は `deep.md`。

**ここで決めるのは、その決定を実装に落とすときの技術判断だけ**で、観測可能な振る舞いは
`specs/record-deletion/spec.md` と `specs/browsing-views/spec.md` に置いてある。

読む前提になる既存の形:

| 場所 | いまの形 |
|---|---|
| `core.event` | `deleted_at` / `deleted_by`。凍結のトリガはこの 2 列を対象外にしてある（`202609120944_gates.sql`） |
| `stay_store::Mark` | 滞在の `deleted_by` を 4 つに読み分ける。`rebuild:` で始まらない値（`NULL` を含む）は**本人が消した** |
| `stay_store::rebuild_day` | 1 トランザクション。先頭で `pg_advisory_xact_lock(LOCK_KEY, hashtext(user))`。本人が消した滞在は触らず、その時間帯と重なる滞在に `rebuild:erased-range` を付ける |
| `stay_store::day_view` | 滞在は `core.event_live`、**隠れた滞在の時間は素の `core.event`** から引いて「移動」で埋めないようにしている（空白。ST16 の D8（仮）） |
| `/ingest` | 削除済みの内容は入れない（ST03）。受理の応答は変えないまま、入れなかったことをログに残す |
| `coverage.rs` | 素の `core.event` を読む（削除済みも数える。ST02 の D34） |

## Goals / Non-Goals

**Goals**

- 滞在を消す・戻す操作を口と画面に置き、消した判断を**上書きされない台帳**にも残す（C2）
- 消した時間が、1 日の一覧で「記録なし」にも「移動」にも「空白」にもならず「消した」と出る（Q4）
- ST16 の作り直しと**同じ錠**の上で動かし、作り直しを巻き戻させない（R44）

**Non-Goals**

- **本文の物理削除**（FR-51 / FR-52）。ST23。ここで作る台帳は「消したことにする」の台帳で、ST03 の `core.erasure_ledger`（消去の台帳）とは別
- **滞在以外を 1 件ずつ消す口**（C3）。位置・PC のウィンドウ・ブラウザ履歴・個人属性の主張を個別に消す口は作らない。
  **個人属性の主張を消す操作は ST23 へ渡した**（spec-review R1。`docs/handoff/ST23.md` と `docs/stories/INDEX.md` の ST22 の訂正）——
  ST19 は錠を開けて待っており（`docs/handoff/ST22.md` の st19 Q1。FR-44 ★ 2026-09-15 は**本人の決定**）、
  `record-deletion` を次に触るのは ST23 で、ST23 は同じ画面（S-6）に本文の消去の操作を作る。**Non-Goals に書くだけでは鎖が切れる**ので、宛先のファイルに置いた
- **ブラウザの側で消えた訪問を ashiato でも消すこと**（`docs/handoff/ST22.md` の st08 R4）。ST08 は「消えた事実を残す（ashiato からは消さない）」と決め、
  その記録はまだ 1 件も無い（ST08 の下流が未着手）。**判定を作るのはブラウザ履歴を画面に見せる Story**で、ST22 は自動で消す経路を一切作らない
  （「扉を開けたままにする既定」—— 消さない側はいつでも消せる。`docs/handoff/ST23.md` に置いた）
- **`core.event` への直の INSERT を DB で締めること**（`docs/handoff/ST22.md` の st19 R20）。締めると `tools/check-immutable.sh` の台本が置けなくなる。
  ST22 は錠に触らない（削除の 2 列は既に素通し）ので、`record-envelope` 全体の話として ST23 が決める
- **利用者ごとの認証**（ST28）。資格情報は読み出しと同じ共有の 1 本のまま、利用者は消す行から取る（C10）
- **感度**（ST24）/ **書き出しの側の除外**（ST33）

## Decisions

### D1. 削除の台帳は新しい表 `core.deletion_ledger`。追記のみ、1 操作 1 行

```sql
CREATE TABLE core.deletion_ledger (
  seq            bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,  -- 並び（時刻は同値になりうる）
  event_id       uuid NOT NULL,          -- **FK は張らない**（ST03 の erasure_ledger と同じ向き）
  user_id        uuid NOT NULL,
  logical_source text NOT NULL,
  action         text NOT NULL CHECK (action IN ('erase', 'restore')),
  cause_event_id uuid NOT NULL,          -- 原因になった滞在（滞在自身の行は自分の id）
  mark           text NOT NULL,          -- そのとき書いた deleted_by の値（D4）
  at             timestamptz NOT NULL DEFAULT now(),
  txid           xid8 NOT NULL DEFAULT pg_current_xact_id()
);
CREATE INDEX deletion_ledger_by_event ON core.deletion_ledger (event_id, seq);
CREATE INDEX deletion_ledger_by_cause ON core.deletion_ledger (cause_event_id, seq);
```

UPDATE / DELETE / TRUNCATE を拒むトリガを**この移行専用の関数**（`core.reject_deletion_ledger_change()`）で置く
—— ST03 / ST16 の関数を使い回すと、それを落とす down 移行が依存で当たらなくなる（ST16 が `tools/check-immutable.sh` で実測）。

**並びの鍵を `seq`（識別）にする理由**: 戻すときに「その記録について**最後に**書かれた行」を引く（D6）。`at` は同じトランザクションの行で同値になり、
`uuid` は順序を持たない。

> 採らなかった案: `deleted_at` / `deleted_by` の 2 列だけで済ませる。上書きなので、戻した瞬間に「一度消した」事実と時刻が消え、
> 連鎖で消えた位置と本人が個別に消した位置も区別できない（レビュー R1。`loss: uncaptured`）。
> 採らなかった案 2: ST03 の `core.erasure_ledger` に相乗りする。あれは**本文の消去**（FR-51）の台帳で、
> `scope IN ('event','version')` と「同じトランザクションに行が無ければ消去を拒む」門が掛かっている。意味の違う 2 つを同じ表に混ぜると、ST23 が門を書けない。

### D2. 口は `POST /stays/erase` と `POST /stays/restore` の 2 本。滞在の識別子だけを受ける（C3）

| 口 | 本体 | 応答 |
|---|---|---|
| `POST /stays/erase` | `{"stay_id": "<uuid>", "user_id": "<uuid>"?}` | `200 {"erased": {"stays": 1, "locations": 12}}` |
| `POST /stays/restore` | `{"stay_ids": ["<uuid>", …], "user_id": "<uuid>"?}` | `200 {"restored": {"stays": 1, "locations": 12}}` |
| `GET /stays/detail?stay_id=&user_id=` | —— | `200 {"stay_id", "start", "end", "counts": [{"logical_source", "display_name", "count"}, …]}` |

- **既存の `/stays` の応答と `/ingest` の応答は変えない**（`record-envelope` の要件に触れない。C9）
- **利用者は行から取る**（C10）。`user_id` が来ていて行の利用者と違えば **404**（存在を漏らさない）。
  取り違えは `kind = "erase_user_mismatch"` のログに残す（値は載せない）—— 404 だけだと設定の誤りが見えない
- **戻すが複数を受けるのは、「消した」の行が複数の滞在を表しうるから**（D7）。1 つのまとまりで戻す（部分的な失敗を画面に持ち込まない）
- 消す側が 1 件なのは、画面が 1 行ずつしか消さないため。まとめて消す口は要求が出てから足す

> 採らなかった案: `DELETE /stays/{id}`。HTTP の意味としては近いが、**行は消えない**（印を付けるだけ）ので誤読を招く。
> 戻す口が `DELETE` の逆にならないことも理由（`POST /stays/{id}/restore` と非対称になる）。

### D3. 連鎖の範囲は「消す滞在の始まり〜終わり」に入る、基準のソースの記録（Q1）

消す滞在の期間は **`stay_store::span_of` が読むもの**（始まりは行の `event_time`、終わりは `payload->>'end'`。読めなければ始まりと同じ）を使う。
この期間（端を含む）に `event_time` が入り、`logical_source` が**その利用者の現在の基準の `sources`**（既定 `c01-location`）にある記録に、同じまとまりで印を付ける。

**`payload.start` を読まない**（spec-review R13）—— 既存の読み手 3 か所（`load_existing` / `day_view` / 吸収）はすべて `span_of` を通しており、
取り込みの口から入った `s01-stay` は `payload` の形が崩れていることがある。読み方を 2 通りにすると、
「一覧に出ている範囲」と「消える範囲」がずれる。

- **端を含む**のは、滞在の端の点がその滞在を作った点だから（`stay::detect` は端の点を使う）
- **基準の `sources` を使う**のは、滞在を作った入力と消す対象をずらさないため。基準を変えても、消すときの基準で決まる
- **位置以外（PC のウィンドウ・ブラウザ履歴・写真・主観）は触らない**（Q1 の本人の答え）
- 隠された滞在（`rebuild:erased-range`）や吸収された滞在（`rebuild:absorbed`）は `s01-stay` なので、この範囲に入らない

### D4. `deleted_by` に書く値は 3 つ。どれも `rebuild:` で始まらない（C1）

| 何を消したか | 値 |
|---|---|
| 本人が滞在を消した | `user` |
| その滞在の連鎖で消えた位置（D3） | `user:cascade` |
| 消した時間帯に後から届いて印を付けた位置（D6） | `user:late` |

滞在の側は `stay_store::Mark::of` が `rebuild:` で始まらない値を**本人が消した**と読むので、`user` で条件を満たす（R14）。
位置の側の値は作り直しの判定に使われないが、**台帳の `mark` と同じ語**を書いて、後から経路を読み分けられるようにする。
「誰が」ではなく「どの経路で」を持つ —— 利用者は行の `user_id` にある。

### D5. 消す・戻すは 1 トランザクション。先頭で作り直しと同じ錠を取り、作り直しは**コミットの後**に呼ぶ

```
tx = begin
  pg_advisory_xact_lock(4816016, hashtext(user))      -- stay_store::LOCK_KEY（R44）
  滞在に印（すでに印があれば何も書かない。C4）
  範囲の位置に印（すでに印がある行は飛ばす）
  台帳に 1 行ずつ（状態が変わった行だけ。C2）
commit
→ 触れた日（Asia/Tokyo。滞在が日をまたげば 2 日）ごとに 1 回 rebuild_day（C7 / R37）
```

- **作り直しを同じトランザクションに入れない**理由: `rebuild_day` は自分で `begin` と錠を取る形（`pool` を受ける）で、
  中に押し込むと ST16 の錠の規律（1 関数 1 トランザクション）を書き換えることになる。
  錠は `pg_advisory_xact_lock` なのでコミットで外れ、その直後の作り直しは**もう印の付いた行を読む**
- **作り直しが失敗しても消したことは残す**（spec）。`/ingest` の後の作り直しと同じ扱いで、
  `kind = "stay.rebuild"` に利用者・日・種別だけを出す（値は出さない）。次に位置が届くか手の作り直しで直る
- **戻すときも同じ**。戻した時間帯の `rebuild:erased-range` は作り直しが外す（ST16 の D4 が毎回計算し直す。R19 の後段はこれで満たされる）

### D6. 消した時間帯に後から届いた位置に印を付けるのは、**取り込みの後の作り直しの中**（Q2）

`stay_store::rebuild_day` の中、錠を取った後・滞在を組み立てる前に、
「この範囲にある**本人が消した滞在**（`Mark::UserDeleted`）の時間帯に入る、印の無い基準のソースの記録」に `user:late` の印と台帳の行を付ける。
原因（`cause_event_id`）はその滞在。

- **取り込みの受け入れのトランザクションに入れない**理由: 受け入れは滞在を読まない（読ませると `/ingest` が滞在の錠を待つ）。
  作り直しは**錠の中**なので、消す操作と競合しない。取り込みは受け入れの後に必ず作り直しを呼ぶ（`rebuild_stays_after_ingest`。基準のソースだけ）
- `POST /stays/rebuild`（手の作り直し）でも同じ経路が走る。**同じ印を二度付けない**（印のある行は飛ばす）ので何度走ってもよい
- 戻すときは台帳の原因で引くので、`user:cascade` と `user:late` は同じ操作で戻る（spec）

**残る穴**（Risks に再掲）: 作り直しが失敗した間だけ、その位置は読み出しに出る。

### D7. 「消した」の区間は、`day_view` が**隠れた時間をつないで 1 行**にする（Q4）

いま `day_view` が「移動で埋めない」ために引いている素の `core.event` の行（`deleted_at IS NOT NULL` かつ
`deleted_by` が `rebuild:absorbed` でない = 本人が消した滞在と `rebuild:erased-range` で隠れた滞在）を、**そのまま行にする**。

- 重なる / 隣り合う区間は**つないで 1 行**にする（**振る舞いなので specs に置いた** —— spec-review R5）。
  理由: `rebuild:erased-range` の滞在は消した範囲より広いことがあり（FR-50「丸ごと隠す」）、別々に出すと同じ時間に 2 行が重なる
- 行が持つ識別子（`stay_ids`）は、**その区間に重なる「本人が消した滞在」だけ**。`rebuild:erased-range` は作り直しが外すので戻す対象に要らない
- 「消した」の区間は、記録なし・移動の計算から**先に差し引く**（消した時間が「記録なし」に吸われていた実測 R2 を閉じる）。
  移動の側の規則も specs に置いた（spec-review R3。正典は「位置の記録が無い時間」しか例外にしていなかった）
- 差し引いた**残りの断片**は、あらためて「記録が無いとみなす間隔」で測り直す（spec-review R4）——
  09:59 の点と 11:00 の消した端の間に 1 分の「記録なし」を生やさない
- 応答の種類は `erased`。`EntryKind` に 1 つ足すだけで、既存の 3 種類の意味は変えない

> **（仮）**: つないで 1 行にする粒度。**反転条件** —— 1 日に消した滞在が何件も並ぶ使い方が実際に出て、
> 「どれを戻すのか」が行から分からなくなったら、消した滞在ごとに 1 行へ割る（台帳があるので後から割れる）。

### D8. 詳細の件数は、開いたときに `GET /stays/detail` を 1 回叩く（Q4）

滞在の時間に重なる `core.event_live` の行を `logical_source` ごとに数えて返す（滞在自身は除く）。
登録簿（`core.source`）の表示名を添える。

- **一覧の応答に載せない**理由: 一覧は 9 行前後で、開かれない行の件数まで数えるのは無駄。
  開くのは「消す前に何が消えるか」を見るときで、1 行ずつしか開かない（D9）
- **読み出しに出ている記録だけ数える**（spec）。消した後に開き直すと 0 件になり、消えたことが画面で分かる

> **（仮）**: 開くたびに叩く形。**反転条件** —— 実機で開くたびの待ちが体感できたら、一覧の応答に件数を載せる（読み出しは 1 回で済む）。

### D9. 画面は `DayView.tsx` の行を「開ける行」に変える。確認はその場、戻すは確認なし

- 行全体を `<button>`（`aria-expanded`）にして、選ぶと**その行の中に**詳細が開く（`ui-direction`「詳細はその場で開く」）。
  **一度に 1 件**（別の行を開くと前は閉じる）—— 状態は `openId` 1 つで持つ
- 詳細の末尾に「この滞在を消す」（44×44 px 以上。`MIN_TARGET_PX` は 24 なので、この操作だけ別の定数 `DESTRUCTIVE_TARGET_PX = 44`）
- 確認は**同じ行の中**に出す（`window.confirm` は使わない —— 実寸も文面も機械から読めない）。
  文面に一緒に消える位置の件数（D8 で取った数）を入れる。「消す」「やめる」の 2 つ
- 「消した」の行は薄い 1 行（記録なしと同じ濃さ）＋「戻す」。**戻すに確認を置かない**

> **（仮）**: 戻すに確認を置かないこと。**反転条件** —— 誤って戻す操作が実際に起きたら確認を足す。
> 戻すは失われるものを作らない（消えた状態はいつでも作り直せる）ので、消すのと対称にしない。

### D10. 移行は 1 本。名前は**作成時刻** `YYYYMMDDHHMM_deletion_ledger.sql`

`core.deletion_ledger` と索引 2 本とトランザクション 3 つ（UPDATE / DELETE / TRUNCATE）を 1 本に入れ、
`crates/server/src/lib.rs` の `MIGRATIONS` 配列の末尾に足す。当て直せる形（`IF NOT EXISTS`）。
`.down.sql` は表とトリガと関数を落とす。**連番を使わない**（並走する Story と番号を取り合う）。

### D11. 変えないもの

| もの | なぜ |
|---|---|
| `/ingest` の応答 | C9。削除済みで入らなかった 1 件も「受理・重複」のまま。端末に「消された」を返す理由が無い |
| `coverage.rs`（稼働状況） | Q3。素の `core.event` を読むまま。**spec に Scenario を置いて固定する**（消すと減る実装に変わったら落ちる） |
| `record-envelope` / `derived-records` / `collection-coverage` の要件 | 並走している Story（ST05 / ST12 / ST04）と衝突させない。ST22 の振る舞いは `record-deletion` に ADDED で置く |
| `core.event` の凍結のトリガ | 削除の 2 列はもともと素通し。INSERT を締める話は ST23（Non-Goals） |

### D12. 削除済みの滞在は**専用のビュー越しに読む**（製造準備 A-3）

`day_view` はいま素の `core.event` を引いて隠れた滞在の時間を出している。ST22 はそこに erase / restore / `user:late` の印付け /
`stay_ids` の 4 経路を足すので、**素のテーブルの削除済み行を読む場所が 5 つに増える**。
製造準備 A-3（「論理削除はビュー越しでしか読めない形にし、素のテーブルを直接引かせない」。可逆性 **低**）に従い、
**読む側はビューに寄せる**:

```sql
CREATE OR REPLACE VIEW core.stay_erased AS
  SELECT id, user_id, event_time AS start_at,
         coalesce((payload->>'end')::timestamptz, event_time) AS end_at,
         deleted_at, deleted_by
    FROM core.event
   WHERE logical_source = 's01-stay' AND origin = 'derived' AND deleted_at IS NOT NULL;
```

- **緯度経度と `raw` を載せない** —— 一覧に要るのは識別子と時刻の範囲だけ（deep.md Q4 の R8）。
  消した場面の座標を、読み出しの経路に二度と載せない
- **書く側（erase / restore / `user:late` の印付け）は `core.event` を直に UPDATE する**（ビューは読み出しの形）。
  A-3 は「記録を返す経路」の規律で、印を書く操作はその外にある —— **これを書いておくのが D12 の目的**（書かないと決めたことにならない）
- 移行は D10 の 1 本に同梱する

### D13（仮）. 消す前の確認は FR-52 の前倒しではない

FR-52（本文の物理削除の前に件数を出して確認する）は ST23 の要件で、ST22 は**引かない**（spec-review R9）。
ST22 の確認は深掘り Q4 で本人が proto を見て選んだもので、対象は戻せる削除。件数を出す形が結果として似ているだけ。

**反転条件**: ST23 が FR-52 を実装するとき、論理削除の確認と文面・操作の形を揃える（揃えるなら ST23 側で `record-deletion` の
この Requirement を MODIFIED する）。揃えないなら、2 つの確認が並ぶ理由を ST23 の design に書く。

## Risks / Trade-offs

- **作り直しが失敗している間、後から届いた位置が読み出しに出る**（D6）→ 失敗はログに残り、次の位置の到着か手の作り直しで印が付く。
  消した滞在そのものは印が付いたままなので、一覧に「消した」の行は出続ける（画面には漏れない）。漏れるのは `GET /events` を直に読んだときだけ
- **「丸ごと隠す」の帰結で、消していない時間まで「消した」と出る**（D7）→ FR-50（★ 2026-09-13。ST16 の Q12 で本人が決めた）のまま。
  戻せば元に戻る（`rebuild:erased-range` は作り直しが外す）
- **台帳の行数は消した位置の件数だけ増える**（1 時間の滞在で 60 行）→ 索引 2 本で引く。記録そのものより 2 桁小さい
- **404 に寄せたので、利用者の取り違えが呼び出し元から見えない**（D2）→ ログの種別で見る
- **消す・戻すが滞在の錠を取るので、作り直しと直列になる**（D5）→ 作り直しは 1 日ぶんで数十 ms（ST16 の実測）。
  取り込みが待つのは最大でその 1 回ぶん

## Migration Plan

1. 移行 1 本を当てる（`core.deletion_ledger`）。**既存の行に触らない**ので、当てる前後で読み出しは変わらない
2. サーバを入れ替える（口 3 本と、`rebuild_day` の中の `user:late` の印）
3. 画面を入れ替える（開ける行・消す・戻す）
4. 巻き戻し: `.down.sql` で表を落とす。**すでに消した記録の印（`deleted_at` / `deleted_by`）は残る** ——
   台帳だけが消えるので、戻す操作は「どの位置がその滞在で消えたか」を引けなくなる。
   巻き戻すなら、戻す操作を先に済ませる（この 1 行を `.down.sql` の先頭にコメントで書く）

## Open Questions

無し。（仮）決めは D7 / D8 / D9 / D13 の 4 件で、反転条件はそれぞれの項に書いた。
