# ST56 上流 — 収集アプリを入れた後に、送り先と資格情報を設定できる

change: `openspec/changes/st56-collector-connection-setup/`（FR-128）

> 深掘り第 1 回は**推奨の答えで先に**仕様を書いた版（2026-10-20 の流れ）。本人の答えが推奨と違えば `spec --answers` が写し直す。

## 深掘りで本人が決めること（第 1 回。答え待ち・推奨）

| Q | 段 | 推奨 |
|---|---|---|
| Q1 設定の値をどこから運ぶか | 仮でよい | 画面の設定に「収集アプリの設定」を置き、画面の合言葉を入れ直すと 1 行の設定の文字列を出す（D8） |
| Q2 PC の設定の口と置き方 | 仮でよい | 黒い窓で貼らせる。資格情報は DPAPI で包んで置き場に置き、台本に秘密を書かない（D15） |
| Q3 携帯の最初の起動の順 | 仮でよい | 最初に設定の画面。「後で」で飛ばせ、収集は始まる（D9） |
| Q4 設定した時点で確かめるか | 仮でよい | 確かめて結果を見せ、繋がらなくても保存する（D10） |
| Q5 断られ続けたときの知らせ | 仮でよい | 401 が 1 時間続いたら知らせ、24 時間ごとに出し直す。一度も届かない送り先は 24 時間（D12） |
| Q7 送り先を変えたときの未送信 | **A（exported）** | 未送信があれば件数と期間を見せ、「この送り先へ送る」を押すまで何も送らない（D13） |
| Q8 配る PC の置き場と端末の識別子 | **A（exported）** | `%LOCALAPPDATA%\ashiato\collector`、識別子は最初の起動で作って覚える（D16） |

聞かない既定（`review/deep.md` C1〜C7 = design D1〜D7）: 利用者は nil の UUID に固定し設定の文字列に入れない・PC は設定が無くても取って積む・携帯の設定はバックアップから外し `-P` は初期値・通知の開く先は送り先から・携帯は loopback と平文の送り先を断る・語は「送り先」・送り先は画面を開いているアドレスから。

## 仮決め（design の（仮））

D11（保存したら送る仕組みを立て直す）・D14（PC は窓で待ち、無ければ通知で `setup` へ）・D17（環境変数は印 `--config-from-env` / `ASHIATO_CONFIG_FROM_ENV=1` のときだけ）・D18（設定の文字列の形 `ashiato-collector:1:` + base64url の JSON）・D19（同じ送り先の比べ方）・D20（知らせの口と札）・D21（PC の保留は y/N）・D22（読み出しの記録と失敗の数えの共有）・D23（前の形の台本の置き場と識別子を引き継ぐ）・D24（最初の設定でも Q7 の保留に従う）。どれも反転条件つき。

## capability と requires

- `device-collection`・`desktop-collection`・`data-sensitivity` に ADDED（17 Requirement・81 Scenario）。INDEX の割り当てどおりで、前倒しは無い。`distribution` は手順書だけで delta を持たない
- **requires に ST54 を足した**（確かめが ST54 の `Ashiato-Channel` を読み、PC の既定の置き場に ST54 の `dev` / `-dev` の規則が当たる）。layer 5 → 6、ST59 も 6 → 7。INDEX.md に訂正の節。ST54 は下流を走っていて gate 前なので、`story_deps.py check --lane upstream ST56` は今は WAIT（ST54 の gate 待ち）
- 画面の部分が増える（ブリーフの「面」は空だが、Q1 で画面の設定の節、Q3 で携帯の設定の画面）。画面の Scenario は `web/e2e/st56-collector-setup.spec.ts`
- 申し送り: `docs/handoff/ST55.md`（「秘密は人が見ない」の書き分けと、`repair` の後の繋ぎ直しの節から `docs/collector-setup.md` を指す）・`docs/handoff/ST59.md`（合言葉の入れ直しの口と、値を変えた後の知らせ）

## tasks

12 Task（Global Constraints つき）+ 人間の確認待ち 2 件（物理: device・realdata）。

## レビュー

- `review/deep.md`: 11 件（escalated 8・fixed 3）
- `review/spec.md`: 15 件 —— fixed 13（technical）・fixed D24 仮 1（conflict。Q3 と Q7 の重なり）・escalated Q7 1（premise。保存の時の確かめが押す前に届くことを Q7 の context に足した）
- `review_triage.py` OK・`spec_overlap.py check` OK・`openspec validate --strict` OK・`evidence.py lint` OK・`check_chain.py` OK

🤖 Generated with [Claude Code](https://claude.com/claude-code)
