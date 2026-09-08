# あしあと。(ashiato)

行動履歴を記録・可視化するライフログ。全記録を単一のデータストアに集め、AI と衛星アプリに読ませる。

- 要件: [`docs/requirements.md`](docs/requirements.md)
- Story: [`docs/stories/INDEX.md`](docs/stories/INDEX.md)
- UI の方向: [`docs/ui-direction.md`](docs/ui-direction.md)
- 製造準備: [`docs/production-prep.md`](docs/production-prep.md)

## 構成

| 記号 | 何か | 言語 |
|---|---|---|
| S-01 | バックエンド（API・取り込み・プラグイン基盤） | Rust |
| C-01 | 収集アプリ（携帯端末） | Kotlin / Android |
| C-02 | 収集アプリ（PC） | Rust |
| V-01 | 画面 | TypeScript + React |
| D-01 | 正統データストア | PostgreSQL |

## 0 から動かす

必要なもの: Rust（stable）/ Node 24 / Docker。

```bash
git clone <このリポジトリ> && cd ashiato2
cp .env.example .env          # 実値はここに。コミットしない
./tools/dev.sh                # 依存の導入 → DB 起動 → サーバと画面の起動
```

画面は http://127.0.0.1:5173 、API は http://127.0.0.1:18787 に出る。

動いていることを機械で確かめるには:

```bash
./tools/smoke.sh              # ダミー 1 件を入れて → 保存され → 取り出せる、まで通す
```

## ライセンス

AGPL-3.0-only（[`LICENSE`](LICENSE)）。貢献は [`CONTRIBUTING.md`](CONTRIBUTING.md) の CLA の下で受け付ける。
