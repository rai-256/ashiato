# CH06 検証のたびにテスト用 DB を作り直す（上流）

change: `openspec/changes/ch06-reset-verify-db/`。新しい capability `verify-environment`（ADDED 6・Scenario 24 本）。
依頼の逐語・価値・壊してはいけないもの・完了の判定は `docs/changes/CH06.md`。`requires: []`（CH04 と重なるが依らない。design D9）。

> **第 1 回の深掘りの答えを待たずに、推奨の答えで先に書いた**（2026-10-20 の流れ）。下の Q1〜Q3 は**本人の答えではなく推奨**で、`deep.md` は `（答え待ち・推奨: …）` のまま。
> 答えが推奨と違えば `spec --answers` が仕様に写し直す。全問が B（仮でよい。消えるのは試験の DB の中身だけ）。
> Q3 の推奨（subnet の割り当て）に乗っているのは Requirement 5・6（Scenario 10 本）、tasks 1・2、design D5〜D7。Q3 の答えが違えば、ここが書き直しの範囲。

## 深掘り（第 1 回。推奨で書いた）

| Q | 推奨（仕様はこれで書いた） | どこに効くか |
|---|---|---|
| Q1 CH04 との重なり | 依頼どおり volume ごと毎回作り直す（CH04 と重ねる） | Requirement 1・2・4、design D1・D2・D4・D9、tasks 3〜5 |
| Q2 繋いでいるもの | 構わず作り直し、切った接続の数と出所を 1 行出す | Requirement 3、design D3、tasks 4.1・5.1 |
| Q3 アドレスの空きが尽きている | worktree ごとの subnet を被らない規則（port を通した台帳の番号）で割り当てる。消えた worktree の残りは片付けない | Requirement 5・6、design D5〜D7、tasks 1・2・4.2 |

聞かずに決めたもの（`premises.md` の C1〜C6）: CI でない main の checkout では作り直さない・CI は作り直す（C1）/ network を手放さず volume は名指しで消す（C2）/
役割は `db-roles.sh`、移行は試験の初回（C3）/ 検査は `tools/check-verify-env-reset.sh` を CI の rust job で（C4）/ 毎回作り直す（C5）/ 作り直せなければ非 0（C6）。

## 仮決め（B）と反転条件（design の（仮））

- **D1（仮）** 作り直しは volume の名前を控え、`rm -s -f db`（`-v` なし）→ 名指しの `volume rm` → `up`。反転条件: docker / compose の版で `rm` の volume の扱いが変わり、2 回続けての作り直しが落ちたら実測し直す
- **D2（仮）** 判定は印・worktree か・`CI` の表。`--plan` で docker に触らずに出す。反転条件: CI でない経路で `CI=true` が立つなら `GITHUB_ACTIONS` で見分ける
- **D5（仮）** subnet は DB の port から（`10.<200+⌊off/256⌋>.<off mod 256>.0/24`、off = port − 55400）。塞いでいる network はコンテナが無ければ消し、あれば非 0。
  **Q3 の「片付けない」の唯一の例外**。反転条件: 経路・幅・別の repo との重なり。本人が例外を認めなければ空の network も消さず非 0 にする
- **D6（仮）** subnet は `env-init.sh` が生成する `docker-compose.subnet.yml`（gitignore）と `.env` の `COMPOSE_FILE` で届ける。値は環境変数にも `.env` にも載せない（ハーネスが渡す古い値が勝つ）。反転条件: compose が `.env` の `COMPOSE_FILE` を読まなくなったら
- **D7（仮）** env-init は `COMPOSE_FILE` と subnet のファイルだけを保つ（合言葉と権限は変えない）・CI で作らない・録画の `.env` に触らない・override を並べる。`verify-prep.sh` も env-init を呼ぶ。反転条件: `COMPOSE_FILE` を書かれたくない木が出たら
- **D8（仮）** 検査の本体は 8 段（溜まった状態は代わりの `core.event` で作る）。CI だけの段（main の checkout の実行の経路）と worktree だけの段（塞いでいる network）があり、CI と `verify-run` の証跡を合わせて 24 本が通る。反転条件: CI で 90 秒を超えたら段をまとめる
- **D10（仮）** capability は CH04 の `test-database` と分けて新しい `verify-environment`。front matter の `capability` は空のまま

## capability

新しい `verify-environment`。前倒しは無い（INDEX.md の表には足さない —— change は front matter で持つ）。

## tasks

7 Task・16 項目。Scenario 24 本の置き場は `check-ports.sh --self-test` 1・`check-verify-env-reset.sh --self-test` 10・同じ台本の本体 13。
Task 1 が入るまで、この worktree の `verify-run` は DB を起動できず BLOCKED_INFRA になりうる（いまの機械はアドレスの空きが尽きている。P4）。

## レビュー

- deep-review（questions の段）: 9 件 —— fixed 9（問い・前提・C の直し。R2 と R7 は spec の段で `fixed D2 仮` / `fixed D1 仮` に直した）
- spec-review: 14 件 —— fixed 14（うち仮 10: D1×2・D5×2・D6×2・D7・D8×3。R10 は premise から technical に直した: 前提の外れは実物で確かめ済みで、人間に聞く事実が残らない）。
  主な直し: 2 回目以降の作り直しが no such volume で落ちる（R2）/ 台帳の番号を継いだ木が `Pool overlaps` で立たない（R3）/ 確認バッチの作業ツリーに subnet が届かない（R4）/
  `.env` の値が古い環境変数に負ける（R5 → subnet の届け方を生成したファイルに替えた）/ CI で自己検査が落ちる（R6）/ `CH06.md` の完了の判定が 2 つ（R7）
- `review_triage.py`・`spec_overlap.py check`（観点 7 は該当なし）・`openspec validate --strict`・`evidence.py lint` は rc=0

🤖 Generated with [Claude Code](https://claude.com/claude-code)
