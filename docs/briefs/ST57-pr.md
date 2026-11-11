# ST57 上流 — 新しい版が出たら知らせる

change: `openspec/changes/st57-update-notice/`（FR-112・FR-113・FR-115・FR-116）

> 深掘り第 1 回は**推奨の答えで先に**仕様を書いた版（2026-10-20 の流れ）。本人の答えが推奨と違えば `spec --answers` が写し直す。

## 深掘りで本人が決めること（第 1 回。答え待ち・推奨）

| Q | 段 | 推奨 |
|---|---|---|
| Q1 収集アプリはどこに聞くか | **A（exported）** | 要件どおり、サーバの更新係・携帯・PC がそれぞれ配布元に直に聞く（問い合わせの元の IP と時刻は配布元に残る） |
| Q2 どの版を「新しい版」とみなすか | 仮でよい | 入っている版のチャンネルで決める（正式版は正式版、ベータ版はベータ版と正式版、開発版は印 `nightly`） |
| Q3 サーバの確認を切る口 | 仮でよい | インストーラの `update-check off` / `on` |
| Q4 収集アプリの確認を切る口 | 仮でよい | 端末ごと（携帯は設定の画面のスイッチ、PC は `update-check off` / `on` と `setup` の問い） |
| Q5 携帯での知らせ方 | 仮でよい | 画面に出し、新しい版ごとに 1 回通知（推奨のままなら record で FR-113 を ★ 改訂） |
| Q6 PC の通知の回数 | 仮でよい | 新しい版ごとに 1 回 |
| Q7 画面のどこに出すか | 画面（proto） | どの画面でも見出しの上の帯に 1 行・設定の画面に「更新の確認」の節 |
| Q8 「更新あり」を押したら | 仮でよい | 何も起きない（入れる口は ST58） |
| Q9 確かめられない日が続いたら | 仮でよい | 7 日確かめられなければ「更新を確かめられていない」（推奨のままなら FR-113 を ★ 改訂） |
| Q10 携帯はどの網で聞くか | 仮でよい | 網を問わない |

聞かない既定（`defaults.md` C1〜C10 = design D1〜D10）: 更新係は別の crate・別の実行ファイルで DB の部品を持たない・問い合わせは決まった形だけ（`GET`・本文なし・固定の `User-Agent`・自分の版を送らない）・結果は「見つけた最新の版」で持つ・確かめた記録は追記・1 日 1 回は 24 時間で未来の時刻は期限切れ扱い・更新係の置き場は状態の置き場の外・走らせ直しで足し消す口で消す・宛先は組み立ての定数・サーバは印と時刻だけで状態を見分ける・Linux は確かめない。

## 仮決め（design の（仮））

D11（名前）・D12（TLS は `rustls` + OS の根）・D13（見出しは 5 つの名前の内側）・D14（`GET /update-status` の符号と読み方）・D15（48 時間で「更新係が動いていない」）・D16（携帯の機会は収集のサービスの起動と生存信号）・D17（PC は `setup` の問いと `update-check` の口）・D18（Windows の CI は更新係の外向きを塞ぐ）・D19（記録は 1 MB で古い半分を退ける）・D20（文）・D21（開発版も SemVer で比べる）・**D22**（Q5・Q9 の知らせは仮に足し、決まったら record で FR-113 を ★ 改訂）・**D23**（確認の無い前の版から入れ替えた収集アプリは、切り方を 1 回示してから確かめる。review/spec.md R16 の `exported` を失わない側で閉じた。`provisional.jsonl` に載せた）。どれも反転条件つき。

## capability と requires

- `distribution` に ADDED 14 Requirement・MODIFIED 5 Requirement（インストール・走らせ直し・消す口・Windows の配布物の中身・依存の許諾の一覧）。Scenario は 130 本で、ST57 が印を置くのは 90 本（ADDED 85・MODIFIED で足した 5）。INDEX の割り当てどおりで、前倒しは無い
- requires の ST52・ST55 は archive 済み。走っている change との重なりは無い（`spec_overlap.py candidates` 0 件）
- **移行は無い**。サーバの依存に HTTP の部品を足さない（`check-offsite.sh` は今のまま）。更新係は新しい実行ファイル `ashiato-updater.exe` とサービス `ashiato-updater`
- 画面の Scenario は `web/e2e/st57-update-notice.spec.ts`
- 申し送り（下流で書く）: `docs/handoff/ST58.md`（`GET /update-status` の形・押す口・切った確認を更新で戻さない・前の版へ戻したときの更新係）・`docs/handoff/ST60.md`（Linux の更新係）

## tasks

10 Task・15 項目（Global Constraints つき）+ 人間の確認待ち 2 件（物理: realdata。本物の配布元での確かめと TLS・本人の PC の走らせ直し）。

## レビュー

- `review/deep.md`: 16 件（escalated 12・fixed 4）
- `review/spec.md`: 18 件 —— fixed 16（technical）・fixed D22 仮 1（conflict。Q5・Q9 の要件の改訂の預け先）・fixed D23 仮 1（irreversible / exported。入れ替えの族。失わない側で閉じ、台帳に載せた）
- `review_triage.py` OK・`spec_overlap.py check` OK・`openspec validate --strict` OK・`evidence.py lint` OK・`check_chain.py` OK。`provisional.py check` は D23 の別の席の確かめ待ち（グラフの `provisional_review` が書く）

🤖 Generated with [Claude Code](https://claude.com/claude-code)
