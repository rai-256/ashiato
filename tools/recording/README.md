# 録画の差し込み口（ashiato2 側）

録画の仕組みはハーネス（harness2 の `scripts/record-run`。README の「録画」）が持つ。ここにあるのは
**ashiato2 だけのこと** —— 何を撮るか（シナリオ）と、どう起動するか（専用環境）。

| ファイル | 役目 |
|---|---|
| `recording.json` | 置き場（`C:\dev\ashiato2-recordings`）・Playwright の設定・環境の台本・シナリオの一覧 |
| `playwright.recording.config.ts` | 録画用の Playwright の設定（video・trace on、操作の注釈と工程の見出しを焼き込む、`retries: 0`、`forbidOnly`）。録画のたびに対象コミットの `web/` へ写される |
| `st22/st22-erase-reload.rec.ts` | ST22「記録の削除」: 削除前 → 削除操作 → 削除後 → 再読み込み後も消えている → 後始末で戻す。アサーションは `web/e2e/day-erase.spec.ts` の写しに、再読み込みの確認を足したもの。`day-erase.spec.ts` の 3 本も一緒に撮る |
| `../record-env.sh` | 専用環境。`up` は対象コミットのコードをビルドし、専用の compose project・空いている port・乱数の合言葉・作り直した偽データで `tools/stack.sh up` を立てる。`down` はその compose project だけを volume ごと消し、残りがあれば rc=1 |

```bash
scripts/record-run setup                 # 初回: 前提の道具と差し込み口
scripts/record-run run 23938e7           # 手で 1 回撮る（コミットを省くと HEAD）
```

確認バッチ（`scripts/hx verify`）は、束ねた Story にシナリオがあれば自動で撮り、手順書の「違和感」の問いに動画の場所を添える。

シナリオを足すとき: `recording.json` の `scenarios` に 1 行（`id` / `story` / `spec` / 一緒に走らせる既存のテスト `with` /
対象コミットに要るファイル `requires`）、`tools/recording/st<NN>/` に `.rec.ts`。**アサーションは既存の e2e から写して弱めない**。
見るための停止は `test.step` の見出しと短い `waitForTimeout` だけにする。
