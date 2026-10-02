# 録画（ST22「記録の削除」）

人間が後から動画で動作を見るための録画を、1 コマンドで撮ります。
**合否はテストのアサーションが決めます。** 動画は見るための材料です。見やすさと人間の承認は自動で判定しません。

| ファイル | 役目 |
|---|---|
| `record-st22.sh` | 入口。`setup`（初回）と `run [<commit>]`（毎回） |
| `st22/st22-erase-reload.rec.ts` | 録画のシナリオ。削除前 → 削除操作 → 削除後 → 再読み込み後も消えている → 後始末で戻す。アサーションは `web/e2e/day-erase.spec.ts` の写しに、再読み込みの確認を足したもの |
| `st22/playwright.recording.config.ts` | 録画の設定（video・trace on、操作の注釈と工程の見出しを焼き込む、`retries: 0`）。既存の `day-erase.spec.ts` 3 本も一緒に撮る |
| `check-video.mjs` | 動画が再生できるかを機械で確かめる（Chromium に読ませて 1 秒以上進むか） |
| `summarize.py` | 1 回ぶんの記録（`run.json` / `RUN.md`）と一覧（`runs.tsv`）を書く |

## 構成

```
WSL                                                   Windows
 一時 worktree（<commit> をそのまま取り出す）           C:\dev\ashiato2-recordings\_work\<run>\web
  + tools/recording/st22/ を web/ に重ねる   ──写す──▶   （同じ web/。npm ci → Playwright で操作・録画）
  cargo build --release / vite build                         │
  tools/stack.sh up（専用の DB・port・合言葉）◀── http://127.0.0.1:<port> ─┘
```

- **対象コミットのコード**から起動・録画する。Windows 側へ写す画面とテストも同じ worktree のもの
- 録画用のシナリオと設定は道具の側（このフォルダ）にあり、対象コミットに重ねる。重ねたファイルの sha256 は記録に残る
- 指定したコミットを一時 worktree に取り出すので、**手元の未コミットの変更は録画に含まれない**（記録にもそう書く）。
  `tools/recording` 自体に未コミットの変更があれば、記録に列挙する

## 初回セットアップ（1 度だけ）

1. 前提: WSL に Docker（compose）・Rust（cargo）・Node.js・Python 3・openssl、Windows に Node.js
2. 確認と保存先の作成:

   ```bash
   tools/recording/record-st22.sh setup
   ```

3. Playwright のブラウザは、初回の `run` の中で WSL・Windows の両方に自動で入る（数分。2 回目からは何もしない）

## 通常の再実行

```bash
tools/recording/record-st22.sh run              # HEAD を録画
tools/recording/record-st22.sh run 23938e7      # コミットを指定
```

- 所要: 初回（ビルドのキャッシュ無し）約 2.5 分。以後は約 1.5 分（実測 95 秒）
- rc: 0 = テストが全部通り、かつ本命の動画が再生できた。1 = それ以外。2 = 使い方の誤り・同時実行
- 同時に 2 本は走らせない（`~/.cache/ashiato2-rec/lock`）

## 保存先と開き方

**`C:\dev\ashiato2-recordings\st22\<日時>-<コミット>\`**（WSL では `/mnt/c/dev/ashiato2-recordings/st22/…`）。
**実行ごとに新しいフォルダ**を作り、前の実行（失敗も含む）は上書きしない。**手で消すまで残る。**

| ファイル | 中身 | 開き方 |
|---|---|---|
| `RUN.md` | 先頭の表に テストの合否 / 録画 / 再生できるか / 人間の承認（未実施）。対象・環境・コマンド・片付けの結果 | テキストで読む |
| `ST22-erase-reload.webm` | 本命の動画 | Edge / Chrome で開く |
| `ST22-erase-reload.trace.zip` | 同じ実行の trace | `npx playwright show-trace <file>`、または https://trace.playwright.dev に落とす |
| `playwright/report/` | 4 本ぶんのレポート（動画・trace つき） | `npx playwright show-report playwright\report` |
| `playwright/results/` | 4 本ぶんの動画・trace・スクリーンショット・`results.json` | — |
| `run.json` | 記録の機械向けの形 | — |
| `video-check.jsonl` | 動画ごとの「再生できるか」の確かめ | — |
| `logs/` | build / stack / windows-setup / playwright / cleanup | — |
| `overlay/` | 重ねた録画用ファイルの写し | — |

一覧は `C:\dev\ashiato2-recordings\st22\runs.tsv`（1 回 1 行。追記のみ）。

消すとき: 要らない実行のフォルダをそのまま消す（`runs.tsv` の行は履歴として残る）。
ビルドのキャッシュ `~/.cache/ashiato2-rec/target` も消してよい（次回のビルドが遅くなるだけ）。

## 専用環境と片付け

- DB は実行ごとの compose project（`ashiato2rec<日時><コミット>`）で、空いている port（55440〜 / 18810〜 / 5210〜）に立てる。
  合言葉は毎回乱数。偽データは `SEED=normal` を作り直した直後
- cargo の出力は専用のキャッシュ（`~/.cache/ashiato2-rec/target`）。リポジトリの `target/` は使わない
- **成功でも失敗でも（Ctrl-C でも）**、終了時に次を片付ける。結果は `logs/cleanup.log` と `RUN.md` に残る
  - 今回立てたサーバ・画面（プロセスグループごと）
  - 今回の compose project の DB（volume ごと）
  - 一時 worktree
  - Windows 側の作業場所
- **既存の DB・worktree・他の compose project には触らない**（片付けの対象はこの実行の名前を持つものだけ）
