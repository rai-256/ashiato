# ST48 の前提 —— 外の形式・振る舞い

**外のサービスの書き出しや端末の形式に依る決定は無い**（この Story が決めるのは自分の DB に残す版の記録と、自分の移行の止め方）。
依るのは repo の中の移行の形と台本で、問いを立てる前に repo の実物で確かめた（2026-10-25）。

| 前提 | 何が依るか | 確かめ方 | 結果 | 出所 | 確かさ |
|---|---|---|---|---|---|
| `migrations/ORDER` の並びは作成時刻の順ではない | 「新しい版」を名前の時刻や ORDER の位置で比べられるか（Q1・Q2 の土台） | `grep -v '^#' migrations/ORDER` を読む | 末尾が `202610131200` → `202610211000` → `202610181200`（`merge=union` で merge の順に積まれる） | 実物（repo） | 5 |
| 門の関数 `core.reject_collected_rewrite` は複数の版で `CREATE OR REPLACE` される | 古い版の移行が新しい DB に当たると門が古い定義に戻る（FR-129 の理由） | `grep -l reject_collected_rewrite migrations/*.sql` | `202609082001_immutable_collected`・`202609100000_immutable_origin`・`202609120944_gates` の 3 本 | 実物（repo） | 5 |
| いまの `migrate()` は `MIGRATIONS` の全部を毎回当て、最後に `GRANTS` を当てる。版を数えていない | FR-118（記録が無い）・Q1 | `crates/server/src/lib.rs:186-197` を読む | そのとおり。当てた版の記録は DB に無い | 実物（repo） | 2 |
| main にある移行は 2 回当てても落ちない | Q2 の「記録の無い DB に一度全部を当てる」 | `tools/check-immutable.sh`（ORDER の全版を当て、もう一度当てる。CI で走る） | 検査はある。ただし後の版がビューの列を足すと、前の版の当て直しが落ちる —— ST23（b051f808）と ST41（3983a055）は、そのため main の移行の中身を書き換えた。「全部当て直す」を続けられるかの根拠にはならない（Q1） | 既存のテスト材料・実物（走っている枝の commit） | 3 |
| 起動の台本は、サーバを起こす前に必ず `migrate` を走らせる | FR-117 の起動の拒否より先に FR-129 が効く必要がある・Q4 | `tools/stack.sh:85-89`・`tools/dev.sh:23`・`tools/smoke.sh:48` を読む | 3 つとも `db-roles.sh` → `migrate` → サーバの順（`db-roles.sh` は版の検査より前に管理者で DB に触る） | 実物（repo） | 2 |
| main に入った移行の中身が後から書き換えられたことがある | Q2（配った後に移行を直すと、名前だけの比べ方では古い中身を当て直す） | `git log --first-parent main -- migrations/<版>.sql` を全版で見る | 3 本（`dedup_indexes`・`version_and_ledger`・`gates`）が 2026-09-12 に main の上で 1 回直されている。以後は Story のブランチの中でだけ直している | 実物（repo） | 5 |
| 移行の中には自分で `BEGIN` / `COMMIT` を持つものがある | 版を当てることと記録することを 1 つの取引に包めるか（design で決める） | `grep -n BEGIN migrations/*.sql` | `202609082001_immutable_collected`・`202609112113_source_lifecycle` が持つ | 実物（repo） | 5 |
| 確認バッチの DB は回をまたいで残り、確認の間は電話の記録が入る | Q5（止まったときに作り直すと記録を捨てる） | `tools/verify-prep.sh:72-80`（`run.sh` は `STACK_RESET` を付けない）・`tools/stack.sh:24`（作り直すのは `STACK_RESET=1` か `HARNESS_VERIFY=1` だけ）・`tools/ports.sh:9-10`（DB 55499・サーバ 18787） | そのとおり | 実物（repo） | 2 |
| いまの組み立ての版はどれも 0.1.0 | Q4（「どちらの版か」を版番号だけでは示せない） | `Cargo.toml` の `version`（`crates/server/Cargo.toml` は `version.workspace = true`） | `0.1.0`。版の出所を 1 つにするのは後の ST52 | 実物（repo） | 5 |
| 移行は `migrate()` の外でも当たる | 記録の無い・欠けた DB が日常に出る（族の兄弟。design で記録の書き手を決める） | `crates/server/src/testdb.rs:93-96`・`tools/check-immutable.sh:21-34`・`openspec/changes/ch04-test-db-growth/design.md:43` | 試験の足場は `MIGRATIONS` を直に当て、検査は psql で当てる | 実物（repo） | 2 |

## 聞かずに決めるもの（C。spec の段で design の D にする）

- **C1** 版の記録の置き場と形を凍結の試験で固定し、形の版を持たせる。記録が読めない・知らない形なら止める側に倒す（review/deep.md R2・R3。配った版に焼き付くので、後の版が動かすと古い版が「記録の無い DB」と読む）
- **C2** 配った移行の名前は ORDER から消さない・変えない（Q1 の推奨の検査が一緒に見る）。正式版の枝の緊急修正の移行は同じ名前で幹へ入れる（名前の集合で比べるので、幹が取り込めば先へ進める）（R4）
- **C3** 記録には、移行の名前・中身の指紋・当てた時刻・当てた組み立ての版番号と commit を持たせる（列を持つ既定。いまの版番号はどれも 0.1.0 で区別できない）（R7）
- **C4** 記録の表は `core` の外に置き、アプリの役割は読むだけにする（`grants.sql` の既定の権限で `core` の新しい表はアプリが書ける）（R8）
- **C5** 移行を当てる経路（`migrate()`・試験の足場 `testdb.rs`・CH04 の雛形）は同じ記録を書く。拒否の試験は worktree で共有する DB に書かず、使い捨ての database で行う。`.down.sql` は記録の行も消す（R9）
- **C6** `tools/db-roles.sh` は版に依らない操作だけにする（版の検査より前に管理者で走るので）（R10）
- **C7** 移行が途中で落ちたら当て終えた版だけを記録する。2 つの migrate は DB の錠で 1 つずつにする（族の兄弟）
