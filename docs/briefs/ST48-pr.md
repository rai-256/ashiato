# ST48 古い版が新しい DB を書き換えない（上流）

change: `openspec/changes/st48-db-version-guard/`。新しい capability `distribution`（ADDED 7・Scenario 41 本）。満たす要件は FR-117・FR-118・FR-129。`requires: [ST01]`。

> **第 1 回の深掘りの答えを待たずに、推奨の答えで先に書いた**（2026-10-20 の流れ）。下の Q1〜Q5 は**本人の答えではなく推奨**で、`deep.md` は `（答え待ち・推奨: …）` のまま。
> 答えが推奨と違えば `spec --answers` が仕様に写し直す。5 問とも B（仮でよい）で、A の問いは無い。

## 深掘り（第 1 回。推奨で書いた）

| Q | 推奨（仕様はこれで書いた） | どこに効くか |
|---|---|---|
| Q1 移行の当て方と、配った移行を直すこと | まだ当たっていない版だけ当てる。配った印（タグ `v*`）の版にある移行の名前と中身を変えると CI が落ちる。印が無い間は何も凍結しない | Requirement 2・5、design D2・D9・D11、tasks 2・6 |
| Q2 版の記録が無い DB | 全部を当て、全部に「記録より前から当たっていた」の印を付けて記録する | Requirement 1、design D3、tasks 2.2 |
| Q3 DB の方が古いときの起動 | 起動しない（終了コード 3・`kind=db_behind`・「先に migrate を」） | Requirement 4、design D17、tasks 5 |
| Q4 止まったときの終わり方 | 専用の終了コード 3。退避を取る前に止め、DB に何も書かない | Requirement 3・4、design D4・D6、tasks 4 |
| Q5 確認バッチと worktree の DB | 確認バッチは回ごとに別の DB（compose の project）。止まったら作り直さずに手順を示す。止める仕組みを外す抜け道は作らない | Requirement 7、design D13・D14、tasks 8 |

聞かずに決めたもの（`premises.md` の C1〜C7）: 記録の置き場と形を凍結し形の版を持つ（D1・D5）/ 配った移行の名前は消さない・変えない（D9）/ 記録に名前・指紋・時刻・版番号と commit（D1・D8）/
記録は `core` の外、アプリの役割は読むだけ（D1）/ 当てる経路は同じ記録を書く・拒否の試験は使い捨ての database（D12・D16）/ `db-roles.sh` は版に依らない操作だけ（D15）/ 当て終えた版だけ記録・DB の錠（D11）。

## 仮決め（B）と反転条件（design の（仮））

- **D5（仮）** 版の記録が読めない（形の版が違う・列が違う）なら止める（`kind=ledger_unreadable`）。反転条件: 形の版を上げずに列を足す必要が出たら人間へ
- **D7（仮）** 記録と指紋の違う版は当て直さず警告だけ（起動も）。反転条件: 配った版で違いが出た・門の定義の食い違いが見つかったら止める側へ
- **D8（仮）** 組み立ての印は `CARGO_PKG_VERSION` と commit（`ASHIATO_BUILD_COMMIT` → `git rev-parse HEAD` → `unknown`）。反転条件: ST52 が版の出所を 1 つにしたらそちらから
- **D9（仮）** 配った印はタグ `v[0-9]*`、凍結の検査は `tools/check-released-migrations.sh`（CI はタグを取り寄せてから）。ST52 へ `docs/handoff/ST52.md` で渡す
- **D10（仮）** 名前の時刻 `202610250000` 以後の版は自分の取引を持たない・`.down.sql` が記録の行を消す（走っている ST23・ST29・ST41 の版は境より前）
- **D13（仮）** 確認バッチの `run.sh` は `COMPOSE_PROJECT_NAME=ashiato-verify-<tag>`。`stack.sh` は同じ port を持つ別の project の DB を止める（消さない）。反転条件: 置き場が積もったら片付けの道具を別の change で
- **D17（仮）** サーバの起動の検査はアプリの役割で読み、待ち受けの前に。反転条件: 検査が目に見えて遅くなったら絞る

## capability

`distribution` を新しく作る（INDEX.md の表で ST48 が先頭。前倒しは無い）。正典に `migrate` の振る舞いは無く、MODIFIED は無い。
記録の表は `core` の外なので、書き出し（`data-durability`）の分類にも掛からない。

## tasks

9 Task（版の記録の形と読み方 / 当てる処理 / 試験の足場 / migrate の止め方 / 起動の検査 / 配った移行の凍結 / これから足す版の規則 / 開発と確認の DB / 全体の確かめ）。Scenario 41 本を全部名指し。

## レビュー

- `review/deep.md`（問いの段）: 11 件。escalated 7（問いを組み直して Q1〜Q5 に反映）・fixed premises.md 4
- `review/spec.md`（この段）: 14 件。fixed 12（spec 9・tasks 1・design 1・deep.md 1）、escalated 1（R13: Story の前提の誤り。Q5 が正しい前提で問い、Story の文は stories.json から直した）、rejected 1（R14: 台本の名前を spec から外さない）

## 走っている change との重なり（差し戻さない）

CH04（`testdb.rs` の作り替え。当てる処理は 1 つの関数に寄せ、CH04 の `--lint` には `db-ok:` を付ける）/ ST23・ST41（main の移行の書き換え。印が無いので凍結に掛からない）/ ST29（版を 1 本足す。差分で当たる）。

🤖 Generated with [Claude Code](https://claude.com/claude-code)
