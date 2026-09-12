# ST03 実装レビュー（code 段階・独立）

対象: `feat/st03-idempotent-ingest` の HEAD `dc02b01`（`git diff origin/main` の 30 ファイル）、
`openspec/changes/st03-idempotent-ingest/{deep,design,tasks,specs}`、`tools/`、`migrations/`、`collector-android/`。

**実装した文脈は聞いていない。申告を疑って、全部自分で走らせた。コードは 1 文字も触っていない**
（検証のための改変は必ず元に戻した。`git status` は空）。
採番は **R94 から**（R1〜R64 は deep、R65〜R93 は spec のレビューで使用済み）。

`kind` は `scripts/review_triage.py` が受ける語だけを使う（`technical` / `conflict` / `irreversible` /
`daily` / `premise` / `defer`）。失われるものは `- loss:` の欄で書く。

## 申告の検証（2026-09-12 実行。すべて実測）

| 申告 | 実測したコマンド | 結果 | 一致 |
|---|---|---|---|
| `tasks.md` 60 件すべて `[x]` | 本文の検証コマンドを 1 件ずつ（下表） | 実在し rc=0 | **一致** |
| `cargo test -p ashiato-server` 136 passed / 0 failed | `cargo test -p ashiato-server` | `136 passed; 0 failed` | **一致** |
| `cargo clippy --all-targets -- -D warnings` rc 0 | 同 | rc=0 | **一致** |
| `./tools/smoke.sh` rc=0（手順 36 まで） | 同（`docker compose down -v` 後） | rc=0 / `== ` 行 40 / 「縦串 OK」 | **一致** |
| `./tools/check-immutable.sh` rc=0 | 同（クリーンな DB から） | rc=0 / `OK` 42 行 / 「書き換え禁止 OK」 | **一致** |
| `check_scenarios.py` `scenarios: OK`（55 Scenario 担保） | 同 | `Scenario 70 件 / 担保あり 70 / 人間の確認待ち 0` rc=0。change の 55 件は全部に印がある | **一致**（ただし R97 / R98） |
| `check_chain.py` / `review_triage.py` rc=0 | 同 | rc=0 / rc=0 | **一致** |
| Android `:app:testDebugUnitTest` BUILD SUCCESSFUL（**109 tests**） | `--rerun-tasks` で走らせ、`test-results/*.xml` を合算 | BUILD SUCCESSFUL / **tests=106** / failures=0 | **不一致（件数。3 件多く申告）** |
| CI の残り（`check-migrations` / `check-boundaries` / `check-openapi`） | 3 本とも実行 | 3 本とも rc=0 | 一致 |
| 人間の確認待ち 0 件 | `tasks.md` §14 と `check_scenarios.py` | 0 件で整合 | 一致 |

**捏造も空テストも無かった。** `tasks.md` の 60 件は本文に書いた検証手段が実在し、テスト名も全部実体がある
（`optional_new_fields_parse` / `external_id_kind_defaults_to_record` / `erasure_removes_parent_and_all_versions` …）。
`grep` 系の検証（1.3 の `0` / 3.3 / 9.2 / 13.1 の `6` / 13.2 の `5`）も実測で成立した。
**ずれは「門が守ると書いてあるもの」と「門が実際に見ているもの」の差、および
「本人が決めたこと」と「回帰で守られていること」の差に集まっている。**

### 手 1: 固定値の独立再計算 —— 一致

`hash_is_pinned` の期待値を別実装（python `hashlib`）で独立に算出した。

```
$ python3 -c 'import hashlib,struct
h=hashlib.sha256(); f=lambda b:(h.update(struct.pack(">Q",len(b))),h.update(b))
f(b"test"); f(struct.pack(">q",1757000000*1000000)); f(b"{\"v\":\"x\"}"); print(h.hexdigest())'
39d0ebc5c3c1d17a5deec93e03e60c3be02e5ddd741a2d5575b789c16ee15df1
```

`crates/server/src/ingest.rs:260` の直書きと**バイト一致**。`git diff origin/main` にこの定数は
出てこない（tasks 4.5 の「期待値を変えずに」は守られている）。**Q15（利用者識別子は索引にだけ）も
この 1 本で守られている** —— 鍵に `user_id` を足すと期待値が変わるので必ず落ちる。

### 手 2: ガードをわざと壊した（門の突然変異 5 種 × `check-immutable.sh`）

使い捨ての DB（`docker compose up -d db` → `MIGRATIONS` の順に 12 版）に対して、
`migrations/202609120944_gates.sql` を 1 か所ずつ壊した複製で `./tools/check-immutable.sh` を走らせた。
**各回の前に `docker compose down -v`**（残留した DB のままだと手順が重複キーで落ちて rc が意味を失う）。

| 壊した箇所 | `check-immutable.sh` | `cargo test` |
|---|---|---|
| `require_version_or_ledger` を `RETURN NULL` に（門を外す） | **rc=1**（`NG raw が書き換えられた` ほか 6 行） | — |
| `is_erasure := true`（消去の形で絞らない） | **rc=1**（`NG 履歴を書いても書き換えが通らない` ほか） | — |
| 外部識別子の凍結を外す | **rc=1**（`NG external_id が書き換えられた`） | — |
| 「消去は親とすべての履歴を同じまとまりで」の検査を外す | **rc=0（緑のまま。空振り）** | rc≠0（`erasure_removes_parent_and_all_versions` が落ちる） |
| 履歴の `IF NEW.raw <> ''`（消去以外の書き換えを拒む錠）を外す | **rc=0（緑のまま）** | **136 passed（誰も落ちない）** → R97 |

門そのものは本物で、主要な 3 経路は台本が捕まえる。**捕まえないものが 2 つあり、片方は
`cargo test` も捕まえない**（R97）。

---

## R94. 門は「同じまとまりに履歴行があるか」しか見ておらず、**でっち上げの履歴 1 行で原文が消える**

- 成果物: migrations/202609120944_gates.sql:100-106 / tools/check-immutable.sh:276-287
- 根拠: 使い捨ての DB（12 版適用済み）に `collected` の 1 行（`raw='{"v":1}'`）を置き、
  **前の版とは無関係な履歴を 1 行書いてから**書き換えた。1 トランザクション（`psql -c` の 1 文）:

  ```sql
  INSERT INTO core.event_version (event_id,user_id,logical_source,version_no,
                                  event_time,content_hash,raw,payload)
    VALUES ('7777…','0000…','probe-src',1,'2000-01-01','junk','{"junk":1}','{}');
  UPDATE core.event SET raw='{"t":2}', content_hash='hx' WHERE id='7777…';
  ```
  → **rc=0**。`SELECT raw FROM core.event` = `{"t":2}` / `SELECT raw FROM core.event_version` = `{"junk":1}`。
  **元の `{"v":1}` はどこにも無い**（台帳にも行は無い）。
  spec は「**更新前の版**の履歴行が同じトランザクションで書かれたときにのみ許す」
  （specs/record-envelope/spec.md の「収集した記録の書き換えと消去は…」）と書いており、
  実装は `EXISTS (… WHERE v.event_id = NEW.id AND v.txid = pg_current_xact_id())` の
  **件数 1 件以上**しか見ていない（gates.sql:100-106）。
  これは design が台帳側について自分で塞いだ穴（「台帳を 1 行書いて改竄が通った（実測）」→ D4 の
  「消去の形で分岐する」）と**同型のものが履歴側に残っている**状態。
  `tools/check-immutable.sh:276-287` は「前の版を写す履歴」しか書かないので、この経路を 1 度も通らない。
- kind: technical
- 処置: fixed D17
- 提案: 門に「履歴行が OLD と一致すること」を足す（`v.event_time = OLD.event_time AND
  v.content_hash = OLD.content_hash AND v.raw = OLD.raw` の EXISTS にする。plpgsql で 1 行）。
  併せて `check-immutable.sh` に「中身の違う履歴を書いた書き換えは拒まれる」を 1 本置く ——
  Q17 の「掛けずに積んだ期間は後から検算できない」がそのまま当てはまるので、merge 前に閉じる価値がある。

## R95. 消去の顔（`raw=''`）にすれば、**台帳 1 行で `payload` を好きな内容に差し替えられる**

- 成果物: migrations/202609120944_gates.sql:62-95
- 根拠: 同じ DB で、台帳 1 行と同じまとまりで撃った:

  ```sql
  INSERT INTO core.erasure_ledger (event_id,user_id,logical_source,scope,erased_by)
    VALUES ('7777…','0000…','probe-src','event','probe');
  UPDATE core.event SET raw='', payload='{"forged":true}' WHERE id='7777…';
  ```
  → **rc=0**。`is_erasure` の判定（gates.sql:77-81）は `NEW.raw = '' AND OLD.raw <> ''` と
  `event_time` / `content_hash` の不変だけを見ており、**`payload` を見ていない**。
  履歴行は 1 行も要らない。spec の Scenario「台帳の行があっても、消去でない書き換えは通らない」は
  `tools/check-immutable.sh:322-330` が `raw` を書き換える形でだけ確かめており、
  **`raw` を空にしながら `payload` を差し替える形は検査に無い**。
  効き方が重い理由: 閲覧・検索・AI が読むのは `payload` 側で、`raw` は消えている ——
  **原文から引き直す手段が無い状態で、解析済みの内容だけが偽物になる。**
- kind: technical
- 処置: fixed D17
- 提案: `is_erasure` に `NEW.payload = '{}'::jsonb`（または `payload` が空になる向き）を足し、
  台本に「台帳あり・`raw=''`・`payload` を偽物へ」を 1 本足す。

## R96. `logical_source` は凍結されておらず、**1 文で書き換わって重複判定が黙って当たらなくなる**

- 成果物: migrations/202609120944_gates.sql:31-49（即時の錠）, 62-67（門の `content_changed`）
- 根拠: 列ごとに 1 文ずつ撃った結果（`collected` の行に対して）:

  ```
  rc=1  SET external_id='forged'          （拒否）
  rc=1  SET external_ref='forged'         （拒否）
  rc=0  SET logical_source='probe-other'  ← 通る
  rc=0  SET user_id='1111…'               ← 通る（Q15 が「後から直せる」と決めているので意図どおり）
  rc=0  SET device_id='forged'            ← 通る
  rc=0  SET tz_id='UTC'                   （0004 が明示的に開けている）
  ```
  `logical_source` は `content_hash` の入力（ingest.rs:177）なので、動かした行は
  **以後どの再送とも一致しない**。これは design D12 が名前を付けている不可逆そのもの
  （「移し替えると鍵が古い名前で計算されたまま残り、その行は以後どの再送とも一致しない」）。
  門は `raw` / `payload` / `event_time` / `content_hash` の変化だけを `content_changed` と見るので、
  履歴も台帳も残らずに通る。spec の SHALL は「収集した記録の**書き換え**を、
  更新前の版の履歴行が…あるときにのみ許す」で、列を限っていない。
  D10 は「凍結する列を増やす」を決めた箇所だが、`logical_source` はそこにも挙がっていない。
- kind: technical
- 処置: fixed D17
- 提案: 即時の錠（`reject_collected_rewrite`）に `logical_source` を足す（`user_id` は Q15 のとおり開ける。
  `device_id` は 0004 が意図を書いていないので、開けるなら理由を D に書く）。
  併せて `check-immutable.sh` の凍結列のループ（tools/check-immutable.sh:369-375）に 1 語足す。

## R97. 履歴の「消去以外は書き換えられない」錠が、**どの検査からも観測されていない**（空振り）

- 成果物: migrations/202609120944_gates.sql:138-140 / tools/check-immutable.sh:299-311
- 根拠: `IF NEW.raw <> '' THEN RAISE` を丸ごと消した複製で `./tools/check-immutable.sh` → **rc=0
  （書き換え禁止 OK）**、`cargo test -p ashiato-server` → **136 passed**。
  台本の該当行（`UPDATE core.event_version SET raw='{"forged":1}'` / check-immutable.sh:301）は
  **台帳の無いまとまりで撃っている**ので、錠を外しても遅延制約トリガ（台帳が無い）が拒み、
  緑のまま通る —— つまりこの検査は「守りたいもの」を観測していない。
  いまの実装では台帳を足しても拒まれる（実測: 台帳 1 行 ＋ `UPDATE core.event_version SET raw='{"forged":1}'`
  → rc=1「履歴の原文は本文の消去以外で書き換えられない」）ので**現物は安全**。危ないのは検査のほう。
  同じ台本は「消去は親とすべての履歴を同じまとまりで」も見ていない（そちらは
  `cargo test erasure_removes_parent_and_all_versions` が捕まえることを突然変異で確認した）。
- kind: technical
- 処置: fixed 7.4b
- 提案: 台本に「**台帳を書いたうえで**履歴の原文を空以外へ書き換えると拒まれる」を 1 本足す
  （`${ledger_row/SCOPE/version}` ＋ `UPDATE … SET raw='{"forged":1}'` が rc≠0 であること）。
  R94 の 1 本と同じ場所に置ける。

## R98. 収集側の spec の Scenario が、**いまの実装では成立しない**（印の付いたテストが逆を主張している）

- 成果物: openspec/changes/st03-idempotent-ingest/specs/device-collection/spec.md（「一部が失敗しても成功分は残らない」）
  / collector-android/app/src/test/kotlin/dev/ashiato/collector/SenderTest.kt:56-73
- 根拠: spec の Scenario は

  ```
  #### Scenario: 一部が失敗しても成功分は残らない
  - WHEN まとめて送った中の一部が失敗する
  - THEN 成功した分は未送信から取り除かれる
  - AND 失敗した分だけが未送信に残る
  ```
  この Scenario の印が置かれたテスト（SenderTest.kt:56-57 の 2 つの印）は
  `assertEquals(0, outbox.size())`（同 71 行）——**失敗した 1 件も残らない**ことを主張している。
  実装（Sender.kt の `dropPermanentlyRejected = true` 経路）では、応答が読めれば
  受理分と恒久的な拒否分の**両方**を取り除き、応答が読めない・一時的な失敗なら**1 件も**取り除かない。
  つまり「成功分だけが消えて失敗分が残る」状態はどの条件でも作れず、
  **この Scenario には通る witness が 1 つも無い**（`一時的な失敗では1件も取り除かない`
  も「成功した分は取り除かれる」を満たさない）。
  同じ MODIFIED 要件の本文にも「まとめた中の一部だけが失敗したとき、失敗した分だけを未送信に残す」が
  残っており（ST01 由来の行）、その 2 行下に「恒久的に断られた記録を…取り除く」がある。
  design D13〜D15 の囲み（「spec の 2 文がぶつかる箇所の読み」）は**読み**を書いているが、
  **spec 側の Scenario と要件の文は 1 文字も直っていない** —— archive するとこの矛盾が正典に入る。
  `check_scenarios.py` は名前の一致だけを見るので緑（実測: `scenarios: OK`）。
- kind: conflict
- 処置: fixed D14 仮
- 提案: Scenario を実装に合わせて書き換える（例: 「一時的な失敗では 1 件も取り除かれない」＋
  「恒久的に断られた分と成功分は取り除かれる」の 2 本に割る）か、
  要件本文の ST01 由来の 1 行に「一時的な失敗のとき」を入れる。
  **どちらも spec の変更**なので、design に `D<n>（仮）` を立てるか本人へ返す判断が要る。

## R99. D13（「記録ごと」でないソースへ届いた `external_id` を捨てずに `external_ref` へ回す）を、**1 本のテストも固定していない**

- 成果物: crates/server/src/lib.rs:271-290（`place_identifiers`）
- 根拠: `place_identifiers` の else 側を **捨てる側**へ書き換えて走らせた:

  ```rust
  // 変更前: (None, req.external_ref.clone().or_else(|| req.external_id.clone()))
  // 変更後: (None, req.external_ref.clone())
  ```
  → `cargo test -p ashiato-server` **136 passed; 0 failed**（元に戻した）。
  `tools/smoke.sh` の対象ごとの手順（手順 32）も `"external_id":null` を送るので通らない。
  `subject_ref_is_not_used_for_dedup` も `external_id` を明示的に `null` にしている
  （dedup_tests.rs:648-668）。**`subject` / `none` のソースへ `external_id` が届く経路を
  叩くテストが 1 本も無い。**
  D13 は「捨てると、捨てたものは復元できない」を理由に（仮）で決めた判断なので、
  回帰が無いと**次の実装が黙って捨てる側へ倒せる**（D13 の反転条件は「ST12 / ST13 が
  断るべきと判断したとき」で、無言の反転は含まれていない）。
- kind: technical
- 処置: fixed 8.7
- 提案: `subject` のソースへ `external_id` を載せた 1 件を送り、
  (a) 受理される (b) `external_id` 列が NULL のまま (c) `external_ref` に値が回っている
  の 3 点を見るテストを 1 本足す（既存の `subject_ref_is_not_used_for_dedup` の隣）。

## R100. 「取り込まなかった 1 件でも稼働記録の行を立てる」を、**どのテストも固定していない**

- 成果物: crates/server/src/lib.rs:389-403（削除済みの判定）, 471-489（稼働記録の UPSERT）
- 根拠: 削除済みで取り込まなかったときに**稼働記録を当てずに早く返す**改変を入れて走らせた:

  ```rust
  if let Some(id) = blocked_by_deleted { return Ok(IngestResult::stored(id, true)); }
  ```
  → `cargo test -p ashiato-server` **136 passed; 0 failed**（元に戻した）。
  design の Risks が「**削除済みの内容が届いた 1 件でも稼働記録の行を立てる** → …
  立てないと、そういう到着だけの日が⑥「途絶」に見える（扉 #14 が区別したかったものが壊れる）」と
  明記している判断が、回帰で守られていない。
  これは ST01 のレビューが同型で 1 度捕まえた穴（R47 / F16。重複だけが届いた日）の**再発**で、
  そちらは `duplicate_only_day_still_gets_a_row`（api_tests.rs:811）が守っている。
  引き直せない点も同じ: 取り込まなかった到着は `core.event` に行を残さないので、
  ST02 の `202609111111_coverage_rebuild` でも**その日は復元できない**
  （`core.coverage` は門の対象外で、`TRUNCATE core.coverage` も通る。実測 rc=0）。
- kind: technical
- 処置: fixed 12.2
- 提案: `deleted_duplicate_is_accepted` の隣に「稼働記録の行だけを消してから
  削除済みの内容を送り、行が立ち直り `event_count` が 0 のまま」を見る 1 本を足す
  （api_tests.rs:811 と同じ作り）。

## R101. `(None, None, None)` の分岐は到達不能で、注釈も誤り。到達したら **DB に無い識別子を受理として返す**

- 成果物: crates/server/src/lib.rs:512-520
- 根拠: `folded` は `(row, blocked_by_deleted) == (None, None)` のときに必ず `Some` になる
  （lib.rs:512-516 で `load_stored` を呼ぶ）ので、`(None, None, None)` は構造上作れない。
  `load_stored` は `fetch_one`（lib.rs:544-551）なので、注釈が言う「同時に論理削除が走った」場合は
  **この分岐ではなく 500（`ingest.stored_lookup`）になる** —— しかも論理削除は行を消さないし、
  物理削除は門が拒む（実測: `DELETE FROM core.event` rc=1）ので、その状況自体が起きない。
  万一到達すれば `IngestResult::stored(req.id, true)` を返し、
  spec の MODIFIED「結果に載せる識別子を、**格納されている記録の識別子**とする」に反する
  （**DB に無い識別子が受理として返る** ——「重複のとき送り主の識別子をそのまま返しており、
  DB に無い識別子が受理として返っていた（実測）」と spec 自身が名指しした事故と同じ形）。
- kind: technical
- 処置: fixed 4.8
- 提案: `folded` を `Option` で受けるのをやめて `(None, None)` の分岐で `Stored` を直に持つか、
  到達不能を `unreachable!()` ではなく 500（`internal_at`）にする。注釈の「同時に論理削除が走った
  ときだけ起きる」は事実と違うので消す。

## R102. 畳んで読む形の「どの行を代表にするか」は **design に無い判断**で、後続 4 Story がそれを引き継ぐ

- 成果物: migrations/202609120943_version_and_ledger.sql:78-92
- 根拠: `core.event_folded` は畳んだ 1 件の `id` / `raw` / `payload` / `origin` を
  **`ingest_time` が最も古い行**から採り（`(array_agg(… ORDER BY ingest_time, id))[1]`。同 84-87 行）、
  `event_time` は `min`、`sensitivity` は `max`（厳しい側）にしている。
  `design.md` に `folded` / 畳み込みの語は D1 の索引の話（41 行目）と Non-Goals（22 行目）しか無く、
  **代表の選び方はどの D にも無い**。spec も「1 件として読む形を提供する」までで、
  どの内容が出るかを決めていない。`cargo test folded_view_returns_one_row` は件数（1 件・元 2 行）だけを見る
  （dedup_tests.rs:788-813）ので、代表の選び方を変えても緑のまま。
  Q8 の答えは「置き場だけを作る。使うのは閲覧・検索・AI・書き出し」なので、
  **4 つの Story がこの選び方を前提に実装する**ことになる（＝後から変えると 4 か所が動く）。
  `sensitivity` を `max` にしたのは扉 #15 に沿っていて妥当だが、**それも SQL のコメントにしか無い**。
- kind: technical
- 処置: fixed D16 仮
- 提案: design に `D16（仮）`として「代表は `ingest_time` の最も古い行 / `event_time` は最小 /
  感度は最も厳しい側」と反転条件（実データを見る ST12 / 閲覧の ST25）を 1 段落で書く。
  テストを足すなら `ingest_time` が違う 2 行で代表が決まることを 1 本。

## R103. 門は `SET session_replication_role='replica'` の 1 文で外れる。**アプリと同じ役割が superuser**

- 成果物: migrations/202609120944_gates.sql 全体 / docker-compose.yml（`POSTGRES_USER: ashiato`）
  / crates/server/src/lib.rs（`DATABASE_URL` の 1 本の役割で接続）
- 根拠: `psql -U ashiato` で撃った（`SELECT usesuper FROM pg_user WHERE usename=current_user` → **t**）:

  ```
  SET session_replication_role='replica';
  UPDATE core.event SET raw='{"forged":1}' WHERE id='7777…';
  DELETE FROM core.event_version; DELETE FROM core.event WHERE id='7777…';
  → rc=0。残った記録 0 / 履歴 0

  ALTER TABLE core.event DISABLE TRIGGER ALL;
  UPDATE core.event SET raw='{"forged":2}' …; → rc=0（event.raw={"forged":2}）
  ```
  spec は「これらの制限を、**取り込み口の外から加えられた操作にも適用する**」と書き、
  0002 / 0004 / gates.sql のコメントは脅威として「同じ PC の第三者製プラグイン（PERM-8）や
  psql を直に叩く運用」を名指ししている。**その主体がこの 1 文を打てる。**
  design D4 の代替案は `SECURITY DEFINER` だけを検討して退けており、
  **役割を分ける（アプリは非 superuser・表の所有者でない）という選択肢が検討の記録に無い。**
  ST29（FR-77）がプラグインを別プロセスの HTTP に閉じているので実務上の露出は小さいが、
  `docs/stories/INDEX.md` を見ても **DB の役割分離を持つ Story は無い**（ST28 は網、ST29 は
  プラグインの宣言と承認）——つまり誰の担当でもない。
- kind: technical
- 処置: deferred ST28
- 提案: 「門は同じ役割から外せる」ことを design の Risks に 1 行残し、
  `docs/handoff/` か ST28 / ST29 の入口へ「アプリ用の非所有者ロールを作る」を申し送る。
  （いま塞ぐなら移行 1 本で `CREATE ROLE ashiato_app NOINHERIT` ＋ 所有者と分離だが、
  `tools/*.sh` と CI の接続文字列が全部動くので ST03 の範囲では重い。）

## R104. 「削除済みの記録は、どの経路からも戻らない」の見出しが、**deep が残すと決めた経路を覆っていない**

- 成果物: openspec/changes/st03-idempotent-ingest/specs/record-envelope/spec.md（該当 Requirement の見出し）
- 根拠: 直接の SQL で戻せることを実測した:

  ```
  UPDATE core.event SET deleted_at=now(), deleted_by='probe' WHERE id='7777…';  → rc=0
  UPDATE core.event SET deleted_at=NULL,  deleted_by=NULL   WHERE id='7777…';  → rc=0
  SELECT coalesce(deleted_at::text,'NULL') … → NULL（復活した）
  ```
  門は「論理削除・感度・更新時刻だけの書き換えは素通し」（gates.sql:68-70）なので意図どおりで、
  `deep.md` 第 5 回も「**論理削除の取り消しは残っている**（実験 AJ4）—— Q23 で選択肢 1 を採ったので、
  選択肢 2 が持っていた『取り消せない削除になる』代償を避けられている」と**残すことを利点として**書いている。
  問題は spec 側だけ ——見出しが「どの経路からも戻らない」で、
  同じ change が「これらの制限を、取り込み口の外から加えられた操作にも適用する」と書いているため、
  **正典だけを読む次の実装者は「psql からも戻らない」と読む**。
  SHALL 本文は 4 つとも「届いたとき」の話なので、直せるのは見出しと 1 行の注記。
- kind: conflict
- 処置: fixed D19 仮
- 提案: 見出しを「削除済みの記録は、**取り込み口のどの経路からも**戻らない」にするか、
  本文に「取り込みの経路について定める。人が直接取り消す操作は FR-50 の取り消しとして残る（deep 第 5 回 AJ4）」を 1 行足す。

---

## 手 3: Scenario と test の突合（55 件すべて読んだ）

印の位置は 55/55 実在し、`check_scenarios.py` は緑。**印の先が Scenario の主張を観測していないものは 2 件**
（R98 の「一部が失敗しても成功分は残らない」、R97 の「履歴は消去以外の書き換えも行の削除もできない」）。
ST01 で出た「spec は『バイト単位』／テストは `->>` で取り出した文字列」型の空振りは**今回は無い** ——
`version_raw_is_byte_identical`（dedup_tests.rs:324-347）は `raw` を `text` のまま比べ、
さらに「`jsonb` に通すと値が変わる」ことを `assert_ne!` で確かめて**検査が空振りしていないこと自体を固定**している。
`tools/check-immutable.sh:94` も同じ向き（`raw->>` を使わない理由をコメントに書いている）。

弱いが穴とは呼べないもの（指摘にしない。根拠だけ残す）:

- 「退役は日付で残る」（dedup_tests.rs:726-757）は `testdb::retire` で書いた値を読み返すだけ。
  固定できているのは**列の型が `date`** であることだけで、退役の振る舞いは ST02 の担当。
- 「派生の作り直しはこの capability が畳まない」（同 700-718）は内容の違う 2 件を入れて 2 行を数える。
  いまの実装では畳まれる道が無いので、事実上「何もしていないこと」の確認。
  ただし将来 derived 用の畳み込みを足せば落ちるので、役には立つ。
- 「畳んで読む置き場が引ける」（tools/smoke.sh:630-637）は `smoke-subj` の 2 行が
  **内容が違うので 2 グループ**になる。畳み込みそのものを見ているのは
  `folded_view_returns_one_row` のほうだけ。

## 手 4: 本人の決定が test で固定されているか

26 件を 1 件ずつ当てた。**固定されているのは 24 件**（Q1/Q2/Q3/Q4/Q5/Q6/Q7/Q8/Q9/Q10/Q11/Q12/Q13/Q14(申し送り)/
Q15/Q16/Q17/Q18/Q19/Q20/Q21/Q23/Q25/Q26）。Q22（古いソース名のまま残す）はコードを伴わない運用の決定。
**回帰の無いものは（仮）の D13 と、design の Risks に書いた稼働記録の扱い**（→ R99 / R100）。
加えて**生存信号を捨てないこと（D14）はクラス単位では固定されているが、配線は固定されていない**:

- `HeartbeatOutboxTest.一部だけ受け付けられたら、その分だけ取り除かれる` は既定（`false`）の
  `Sender` を使うので、**既定を `true` にすれば落ちる**（＝D14 の芯は守られている）。
- ただし `LocationService.kt:143-147` の `beatSender` に `dropPermanentlyRejected = true` を
  書き足しても、テストは 1 本も落ちない（テストは `Sender` を直に組み立てる）。
  **配線だけが裸**。R99 / R100 と同種なので独立の R にはしないが、1 本足すなら安い。

## 手 5: tasks の `[x]` と実体 —— 全 60 件で一致

抜き取りではなく 60 件すべての本文の検証手段を確かめた。テスト名が挙がっているのに実体が無いもの、
コマンドが無いものは **0 件**。`grep` で判定する 5 件（1.3 / 3.3 / 9.2 / 13.1 / 13.2）も実測で成立。
13.2 の「ST02 側の現物を確かめた結果」5 件も突き合わせた —— R-a の参照先は
`specs/collection-coverage/spec.md:362,364` と書いてあるが**正典の側は 26 行のスタブ**で、
実体は `openspec/changes/st02-collection-coverage/specs/collection-coverage/spec.md:362,364`
（そこに「8 つのいずれか」と「退役」がある。実測）。**主張は正しく、パスの書き方だけが曖昧。**
R-c / R-e も `coverage.rs:565-580` / `:795` が handoff の記述どおりだった。

## 手 6: 隙間（黙って消える経路を探した）

見つかったものは R94 / R95 / R96 / R100 / R103。**塞がっていて安心できたもの**を根拠として残す:

- **同じ収集側 `id` での再送が 500 にならない**。`ON CONFLICT (user_id, logical_source, content_hash)
  WHERE external_id IS NULL DO NOTHING` は主キーの衝突を巻き込まないか実測した ——
  同じ `id` ＋ 同じ鍵は **rc=0（DO NOTHING）**、同じ `id` ＋ 違う鍵は
  `duplicate key value violates unique constraint "event_pkey"` で rc=1。
  後者はアプリ側の `id_reused`（lib.rs:365-381）が格納の前に 400 で断るので、
  **まとめ送り全体が 500 で永久に止まる経路は閉じている**。
- **台帳 1 行で 4 行消す**は塞がっている（実測 rc=1。門が `l.event_id = NEW.id` を要求する）。
  前のまとまりの台帳の使い回しも rc=1（`l.txid = pg_current_xact_id()`）。
- **`DELETE` / `TRUNCATE`** は `core.event` / `event_version` / `erasure_ledger` / `heartbeat` /
  `source` の 5 表で rc=1。`core.coverage` の `TRUNCATE` は rc=0（導出の帳簿。R100 で触れた）。
- **1 件 1 トランザクション**は成立している（`one_bad_item_does_not_roll_back_others` が
  「同じまとまりの 2 件目が 1 件目の重複になる」ことで COMMIT 済みを観測している）。

---

# 処置（2026-09-12。実装側が記入）

`superpowers:receiving-code-review` の規範どおり、**1 件ずつ根拠を再現してから**書いた。
鵜呑みにも空返事にもしない —— 採らなかったものは理由を書く。

**4 系統のレビューを 1 つの表に畳んである**（`code-verify` の R94〜R104、
`pr-review-toolkit` の `code-reviewer` / `silent-failure-hunter` / `pr-test-analyzer`）。
後者 3 つの指摘には **R105 以降**を振った。

## 申告のずれ 1 件

**Android のテスト件数を 109 と申告したが、その時点では 106 だった**（`code-verify` が
XML を合算して実測）。数えずに書いた。いまは 3 本足したので **109**（同じ方法で再確認済み）。

## A（失われるもの）に触るものが 1 件あった —— 向きを確かめたうえで `fixed`

**R107（収集側が理由を見ずにバッチ全件を捨てる）は `loss: discarded` の型。**
本人は Q4 / Q5 で「恒久的に断られた記録は諦める（捨てる）」と決めているが、
**どれが「恒久的」かは列挙していない。** `unknown_source` は登録簿に 1 行足せば通り、
`missing_external_id` は `external_id_kind` を直せば通る —— **どちらも恒久ではない。**
実装が本人の決定を広げすぎた（事実の誤り）であって、狭めても決定に反しないので
`escalated` ではなく `fixed`。レビューが勧めた「既定を `'none'` に倒す」は
**Q16 の本人の答えに正面から反するので採らない。**

**人間へ返す項目（A）は 0 件。**

## 一覧

| R | 何 | 処置 |
|---|---|---|
| R94 | 門が履歴の中身を `OLD` と突き合わせていない | `処置: fixed D17`（`gates.sql` の EXISTS に 4 列の一致を足した。台本に 1 本） |
| R95 | `is_erasure` が `payload` を縛っていない | `処置: fixed D17`（本表と履歴の両方。台本に 2 本） |
| R96 | `logical_source` / `id` が凍結一覧に無い | `処置: fixed D17`（`user_id` は Q15 のとおり開けたまま。理由を D17 の表に書いた） |
| R97 | 履歴の錠がどの検査からも観測されていない（空振り） | `処置: fixed 7.4b`（台本を「台帳を書いたうえで」撃つ形にした） |
| R98 | spec の Scenario と実装が逆を主張している | `処置: fixed D14 仮`（**spec 本文を直した**。下に理由） |
| R99 | D13 にテストが 1 本も無い | `処置: fixed 8.7`（`subject_source_keeps_a_misplaced_external_id`） |
| R100 | 取り込まなかった到着の稼働記録が無検証 | `処置: fixed 12.2`（`blocked_arrival_still_marks_the_day` / `update_only_day_gets_a_row_without_counting`） |
| R101 | 到達不能な分岐が送り主の識別子を返す | `処置: fixed 4.8`（500 にした。`accepted:true` に載る識別子は必ず DB から引いたものだけ） |
| R102 | 畳んだ 1 件の代表の選び方が design に無い | `処置: fixed D16 仮`（反転条件つき） |
| R103 | 門は `session_replication_role` の 1 文で外れる | `処置: deferred ST28`（`handoff.md` R-j。design の Risks にも残した。下に理由） |
| R104 | 「どの経路からも戻らない」が deep の決定を覆っていない | `処置: fixed D19 仮`（見出しと注記を直した。論理削除の取り消しは deep 第 5 回が**利点**として残している） |
| R105 | `dedup_indexes` が起動ごとに一意索引 3 本を作り直す | `処置: fixed`（`pg_indexes` を見て古い定義のときだけ作り替える） |
| R106 | 更新を止めたとき、削除済みの**別の行**の識別子が返る | `処置: fixed`（`load_stored` を `Option` にして、止めたときもその鍵の行を引く） |
| R107 | 収集側が理由を見ずにバッチ全件を捨てる | `処置: fixed D14 仮`（許可リスト。上の A の欄を見よ） |
| R108 | `accepted` の既定値 ＋ `ignoreUnknownKeys` で、200 の別配列 1 つで全件削除 | `処置: fixed`（既定値を外して必須欄にした。欠く応答は `unreadable_response`） |
| R109 | 取り込まなかった到着がログにも残らない | `処置: fixed`（`ingest_blocked_deleted` / `ingest_update_skipped` の 2 本。値は載せない） |
| R110 | `apply_external_update` の早期 return がログ 0 行 | `処置: fixed`（`Update` enum で理由を持ち帰り、捨てた側も `warn`） |
| R111 | 更新が `tz_id` / `schema_version` などを黙って捨てる | `処置: fixed D18`（一緒に動かし、前の値は履歴に残す。履歴表に 5 列） |
| R112 | 更新後の行への正常な再送が `id_reused` で誤爆する | `処置: fixed`（外部識別子で畳むソースでは内容の一致を求めない。`resend_after_update_is_not_id_reuse`） |
| R113 | 内容が同じ到着で `source_updated_at` の水位が進まない | `処置: fixed`（`advance_watermark`。`watermark_advances_on_identical_content`） |
| R114 | `authored` → `collected` の付け替えが履歴も台帳も無しに通る | `処置: fixed D17`（**入る**向きも塞いだ。0004 が閉じたのは出る向きだけだった） |
| R115 | 削除済みの判定が制約ではなく別文の `SELECT`（競合） | `処置: rejected: 下に理由`（design の Risks に残した） |
| R116 | 「保存済みの更新時刻を消さない」が `is_some()` しか見ていない | `処置: fixed`（等値で見る。`coalesce($6, now())` のバグを通していた） |
| R117 | `.down.sql` が 1 度も実行されていない | `処置: fixed`（`check-immutable.sh` の末尾で逆順に当て、当て直して門が効くことまで見る） |
| R118 | 捨てた記録を後から突き合わせる手段が残らない | `処置: fixed`（`dropped_item` に識別子。`id` は私的データではない） |
| R119 | 一時的失敗の判定が 401 と `>=500` の 2 本しかない | `処置: fixed`（サーバが約束しているのは 200 と 400 だけ。それ以外は一時的に倒す） |
| R120 | 除外系 2 本が「内容が違うから 2 行」だけで通る | `処置: fixed`（対象の識別子の保持・由来・履歴 0 件まで見る） |
| R121 | `IdReused` の 4 条件のうち 3 つが無検証 | `処置: fixed`（利用者 / ソース / 外部識別子の 3 通りを足した） |
| R122 | `smoke.sh` 手順 36 が畳み込みを 1 つも検証していない | `処置: fixed`（同じ本文を違う識別子で 2 件入れて 1 件に畳まれることを見る） |
| R123 | `subject` 種別で `external_ref` が鍵にも hash にも入らない | `処置: deferred ST12`（下に理由） |
| R124 | 消去済みの行に外部の更新が届くと `raw` が埋め戻る | `処置: deferred ST23`（`handoff.md` R-h） |
| R125 | `core.coverage` / `core.source` に削除・切り詰めの門が無い | `処置: followup ST02`（`docs/handoff/ST02.md` R-g） |
| R126 | 更新が日をまたぐと `event_count` が旧日に残る | `処置: followup ST02`（`docs/handoff/ST02.md` R-k） |
| R127 | `require_ledger_for_erasure` が台帳の `scope` を見ない | `処置: rejected: 下に理由` |
| R128 | 同じ記録への同時更新で `version_no` が衝突しうる | `処置: rejected: 下に理由`（design の Risks に残した） |

## 採らなかったものの理由（再現したうえで）

### R115（削除済みの判定の競合）—— `rejected`

判定と INSERT は同じトランザクションの別の文で、READ COMMITTED。
判定の後に別のまとまりが論理削除を COMMIT すると、その 1 件は入る —— **指摘は正しい**。

採らないのは、**塞ぐ手段がこの Story の範囲で釣り合わないから**。
`FOR SHARE` は効かない（ロックする行が無い）。制約へ寄せるには「削除済みの内容」を
一意制約で表す必要があり、Q6 の部分索引と干渉する。いまの運用は
**単一利用者・5 分間隔・1 件 1 トランザクションで直列**なので、窓は 1 件の処理時間
（実測 数 ms）しかなく、しかもその間に本人が同じ内容を消す必要がある。
**失われるものは無い**（起きるのは「消したはずの本文が戻る」で、記録は消えない）。
外部からの取り込み（ST12 / ST13）を**並列にするなら当たる**ので、design の Risks に残した。

### R127（台帳の `scope` を見ない）—— `rejected`

履歴側の門が `scope='version'` を要求すべき、という指摘。**採らない。**

tasks 7.4c と spec は「**消去は親とその記録のすべての履歴を同じトランザクションで**消す」と
決めている。親の消去は `scope='event'` の台帳 1 行で行うので、`scope='version'` を要求すると
**1 つの消去に台帳 2 行が要る**ことになり、決めた形と食い違う。
いまの実装は「その記録（`event_id`）についての台帳行が同じまとまりにあること」を要求しており、
**台帳 1 行が覆うのは 1 つの記録の親と履歴だけ**（別の記録は覆えない。実測で確認済み）。
`scope` は「何を消したか」を台帳に残すための欄で、門の条件ではない。

### R128（`version_no` の競合）—— `rejected`

`coalesce(max(v.version_no),0)+1` は同じ `event_id` への同時更新で一意違反になる。**正しい。**

採らないのは、**いま並列に更新する経路が無いから**（取り込みは 1 件ずつ直列、
外部からの取り込みは ST12 / ST13 でまだ無い）。連番をやめて `superseded_at` の順にすると
「何番目の版か」が読めなくなり、**Q17 が守りたい「その間に消された版があったか」の
検算ができなくなる**（連番の欠けがその印になる）。ST12 / ST13 が並列に取り込むなら、
そのとき採番を DB 側（シーケンスではなく `INSERT … SELECT` の排他）へ寄せる。
design の Risks に残した。

### R123（`subject` 種別で `external_ref` が畳み込みに入らない）—— `deferred ST12`

`raw` に対象の識別子を含まないソースで、`raw` と `event_time` が同じ 2 件が
1 件に畳まれ、2 件目の `external_ref` が黙って捨てられる。**経路としては正しい。**

いま直さないのは、**直し方が Q15 に触るから**。鍵に `external_ref` を入れるには
`content_hash` の作り方を変えるか（Q15 が「ST01 のまま」と決めている）、
部分索引を `NULLS NOT DISTINCT` にする（`none` 種別の畳み込みの形が変わる）必要がある。
**どちらも実物のソースを見ずに決めるものではない** —— deep.md Q25 自身が
「書庫の重複の扱いは ST12 が実物を見てから決める」と書いている。
`handoff.md` に置いていないのは、ST12 にはまだ `tasks.md` が無く `deferred` で足りるため。

### R103（門は同じ役割から外せる）—— `deferred ST28`

**指摘は正しく、spec の文（「取り込み口の外から加えられた操作にも適用する」）に対して
実装が届いていない。** ただし塞ぐには DB の役割分離が要り、
`tools/*.sh` と CI の接続文字列が全部動く。**どの Story の担当でもない**ので、
`handoff.md` R-j に「入口を作るところから要る」として残し、design の Risks にも書いた。
ST29（FR-77）がプラグインを別プロセスの HTTP に閉じているので、実務上の露出は小さい。

## R98 について（spec 本文を直した理由）

`device-collection` の「一部が失敗しても成功分は残らない」の AND 節が ST01 由来のまま
（`失敗した分だけが未送信に残る`）で、印の付いたテストが逆（`outbox.size() == 0`）を
主張していた。**どちらの読みでも通らない Scenario** が正典に入るところだった。

**本人の決定（FR-10 の改訂＝「恒久的に断られた記録は諦める」）が新しいので、
spec 側の古い文を合わせた。** 直したのは 3 行:

1. SHALL「まとめた中の一部だけが失敗したとき、**一時的な失敗で終わった分**を未送信に残す」
2. SHALL を 1 本追加「断られた理由が**受け手側の設定で変わりうるとき**は未送信に残す」（R107）
3. Scenario の AND 節「**一時的な失敗で**終わった分が未送信に残る」

Scenario の**名前は変えていない**（印が付いているので、変えると担保が切れる）。
`design.md` の D14 に読みと反転条件を書いてある。
