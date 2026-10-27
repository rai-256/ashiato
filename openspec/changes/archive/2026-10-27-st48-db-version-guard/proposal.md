# ST48 — 古い版が新しい DB を書き換えない

> 第 1 回の深掘りの答えを待たずに、推奨の答え（`deep.md` の `（答え待ち・推奨: …）`）で先に書いた（2026-10-20 の流れ）。
> 本人の答えが推奨と違えば `spec --answers` が写し直す。5 問とも B（仮でよい）で、A の問いは無い。

## Why

いまの `ashiato-server migrate` は `migrations/ORDER` の全部の版を毎回当て直し、どの版まで当たっているかを DB に残していない
（`crates/server/src/lib.rs` の `migrate()`）。門の関数 `core.reject_collected_rewrite` は 3 つの版で `CREATE OR REPLACE` されるので、
古い版の migrate が新しい版で育った DB に当たると、門が古い定義に戻る（`premises.md`。整合の確認 `docs/audit/2026-10-25-consistency.md` の C4）。
起動の台本は必ずサーバより先に migrate を走らせるので、起動の時の拒否（FR-117）だけでは間に合わない。

ベータ版から正式版へ移る・古い版に戻すことは、配布（ST52 以後）で日常になる。止める仕組みは**最初に配る版に入っていないと、
その版へ戻したときに働かない**（要件「配布と更新」の冒頭）。

あわせて、全部を当て直す形は、後の版がビューの列を足すたびに前の版のファイルを書き換えさせている（走っている ST23・ST41 が実例。`premises.md`）。
版を数えて「まだの分だけ当てる」にすると、配った移行の中身を凍結できる（Q1）。

## What Changes

- **DB の中に、当てた移行の版を記録する**（FR-118）。置き場は記録の schema（`core`）の外の `migration_ledger`。
  名前・中身の指紋（sha256）・当てた時刻・当てた組み立ての版番号と commit・「記録より前から当たっていた」の印を持つ。アプリの役割は読むだけ（C1・C3・C4）
- **migrate は、まだ当たっていない版だけを当てる**（Q1 の推奨）。比べ方は名前の集合（ORDER は時刻順でない）。1 版ごとに当てて記録し、途中で落ちたら当て終えた版だけが残る。
  2 つの migrate は DB の錠で 1 つずつにする（C7）。版の記録の無い ST48 より前の DB には一度だけ全部を当て、全部に印を付けて記録する（Q2 の推奨）
- **知らない版が当たった DB には、migrate が 1 つも当てずに止まる**（FR-129）。退避も取らず DB に何も書かず、終了コード 3 で終わり、
  DB の知らない版（名前と、当てた組み立ての版番号と commit）と自分の版を示す。移行の失敗（終了コード 1）と区別する（Q4 の推奨）
- **サーバは DB と版が食い違えば起動しない**（FR-117）。DB に知らない版がある・DB にまだ当たっていない版がある（Q3 の推奨）・記録が無い・記録が読めない、のどれでも終了コード 3
- **配った移行は名前も中身も変えさせない**（Q1 の推奨・C2）。配った印（リリースのタグ `v*`。ST52 が付け始める）の版にある移行を、ORDER から消す・名前を変える・中身を変えると CI が落ちる。
  印が無い間は何も凍結しない（走っている ST23・ST41 の書き換えは止めない）
- **これから足す移行は、版の記録と一緒に当たり、一緒に戻る**。自分の `BEGIN` / `COMMIT` を持たせない・`.down.sql` は版の記録の行も消す、を `tools/check-migrations.sh` が見る（C5・C7）
- **確認バッチは回ごとに別の DB（docker compose の project）に立つ**（Q5 の推奨）。前の回の DB は消さずに残る。worktree・確認バッチの DB が版の食い違いで止まったら、
  `tools/stack.sh` は作り直さずに立て直しの手順を示して止まる。止める仕組みを外す設定は作らない

## Capabilities

### New Capabilities

- `distribution`: 配布と更新。ST48 はその最初の Story で、DB の版の記録と、版の食い違いで止める振る舞いを置く（INDEX.md の capability 表で ST48 が先頭。前倒しは無い）

### Modified Capabilities

（無い。移行の当て方を持つ Requirement は正典に無い —— `openspec/specs/*/spec.md` に `migrate` の振る舞いは書かれていない。2026-10-25 に grep で確かめた）

## Impact

- **コード**: `crates/server/src/lib.rs`（`migrate()`・`run_migrate()`・`run()` の起動の検査）、新しい `crates/server/src/migration_ledger.rs` と `migration_ledger.sql`、
  `crates/server/src/grants.sql`（記録の読み取り）、`crates/server/build.rs`（組み立ての commit）、`crates/server/src/testdb.rs`（当て方を同じ関数に寄せる）、
  `crates/server/tests/server_startup.rs`（実行ファイルの終了コード）
- **台本**: `tools/check-migrations.sh`（新しい版の規則）、新しい `tools/check-released-migrations.sh`（配った移行の凍結）、`tools/stack.sh`（別の project の DB を止める・止まったときの手順）、
  `tools/verify-prep.sh`（`run.sh` が回ごとの project を渡す）、`tools/dev.sh`（止まったときの手順）、`.github/workflows/ci.yml`、`docs/testing.md`
- **移行の版は足さない**。記録の表は ORDER の外の bootstrap（`migration_ledger.sql`）で作る（記録を読む前に要るので。design D1）
- **書き出し（ST33・data-durability）には掛からない**。書き出しは `core` の表だけを分類と突き合わせる（`crates/server/src/export.rs` の `table_schema = 'core'`）。記録の表は `core` の外なので、分類に足さなくても書き出しは止まらない
- **やらないこと**: ST48 より前の組み立て（版の記録を知らない）が ST48 以後の DB に当たることは止められない —— その組み立ては配らない（FR-117 / FR-118 / FR-129 は最初に配る版に入る）。
  版番号の出所を 1 つにすること・リリースのタグを付けることは ST52。更新の自動の戻しは ST58（終了コード 3 を読む側）。古い確認バッチの DB を片付ける道具は作らない
- **走っている change との重なり（差し戻さない。後から merge する側が rebase で追従する）**:
  - **CH04**（試験の DB）: `testdb.rs` を回ごとの database とテンプレートに作り替え、テンプレートに `crate::MIGRATIONS` を直に当てる。ST48 は当てる処理を 1 つの関数に寄せ、
    `testdb.rs` 側はそれを呼ぶ差だけにする。CH04 の `--lint`（`CREATE DATABASE` は `testdb.rs` の中だけ）に掛かる ST48 の試験の行は `db-ok:` の理由を付ける
  - **ST23・ST41**: main にある移行の中身を書き換えている。印（タグ）がまだ無いので、凍結の検査は落ちない。両方の新しい版の名前は 2026-10-25 より前の時刻なので、
    新しい版の規則（`BEGIN` / `COMMIT` を持たない・`.down.sql` が記録の行を消す）の外にある
  - **ST29**: 移行を 1 本足す（名前の時刻は 2026-10-25 より前）。ST48 の後に入っても、まだ当たっていない版として差分で当たる
  - **CH03**: 試験の一時ファイル。重なる表もファイルも無い
- **ST52 へ**: 配った印のタグの型 `v*` を `docs/handoff/ST52.md` で渡す（ST52 が別の型にするなら、`tools/check-released-migrations.sh` の型を一緒に変える）
