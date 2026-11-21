# 深掘り（軽い路）

人間の深掘りと合意を省いた change（2026-11-09 の軽い路）。AI が入口で省く理由を申告し、`scripts/lanes.py entry` が
触れる予定のパスで裏を取った。実装の後に実際の diff で裏を取り直し、外れたら通常の路へ昇格する（`lane-verdict.json`）。

- 理由: same（振る舞いが変わらない）
- 申告の根拠: 夜間の脆弱性検査が挙げた 4 件を互換の範囲で上げるだけ。chrono・libsqlite3-sys（rusqlite 経由）・rustix は Cargo.toml / Cargo.lock の版を上げ、上げられないものは AGENTS.md の規則どおり osv-scanner.toml の例外の表に理由と 90 日以内の期限で書く。同梱の PostgreSQL は pgsql.lock の version・url・sha256 を 17.12 にそろえるだけで major（17）は変えない（installer の build.rs が読むのは major だけ）。何を・なぜ・どこまでは依頼が決めてあり、要件は変えない。API・送信の契約・移行には触れない。新しい版が足す依存の許諾文が要れば tools/licenses に置く。minor の更新と戻しの確かめは docs/release.md §PostgreSQL の小さい版を上げる PR の手順で行う。走っている CH24 も Cargo.lock と osv-scanner.toml を触るので、後から merge する側が rebase で合わせる
- 触れる予定のパス: Cargo.toml / Cargo.lock / crates/*/Cargo.toml / osv-scanner.toml / tools/release/pgsql.lock / tools/licenses/** / docs/release.md / docs/changes/CH26.md

## spec-review から人間へ返す問い（第 1 回。`deep-questions-r1.json`）

### R2. 17.6 から 17.12 への更新と戻しを、どこまで確かめるか

- 出所: `review/spec.md` の R2（kind: premise）。軽い路の申告は「minor の更新と戻しの確かめは docs/release.md §PostgreSQL の小さい版を上げる PR の手順で行う」と言ったが、design D5 は Windows の zip どうしの更新と戻しを Linux の PostgreSQL の確かめで代えていた。依頼の「既に入っている PC で…確かめる」を狭めるので本人に問う
- 選択肢: A. PostgreSQL の部分（置き場の 17.6→17.12→17.6・17.12 の退避を 17.6 で戻す）を Linux で確かめ、Windows は本人の PC で見る（推奨）／ B. Windows でも 17.6 の zip から 17.12 の zip への更新と戻しを通す ／ C. CI で前の lock の zip を組み、毎回の PR で走らせる（通常の路）
- 答え: A. PostgreSQL の部分を Linux で確かめ、Windows は本人の PC で見る（本人の選択。推奨と同じ・補足なし。`deep-answers-1.txt`）
- 決まったこと: 17.6 → 17.12 の確かめは、PostgreSQL の置き場の入れ替え（17.6 → 17.12 → 17.6）と「17.12 の退避を 17.6 の pg_restore で戻す」を Linux で機械が走らせる（design D5・tasks 3.1）。Windows の配布物どうし（17.6-1 の zip → 17.12-1 の zip）の更新と戻しはこの PR では走らせず、本人の PC で確認バッチの問い（tasks 5.1）として見る。`docs/release.md` の「前の版を同梱した zip から更新と戻しを通す」手順はこの PR では省く（手順の本文は変えない）。CI は変えない
- 要件へ戻すもの: なし（要件は変えない change。依頼の「どこまで」の確かめ方を本人が決めた）
