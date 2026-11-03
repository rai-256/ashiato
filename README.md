# あしあと。(ashiato)

行動履歴を記録・可視化するライフログ。全記録を単一のデータストアに集め、**衛星アプリを作るための基盤**として渡す。
衛星（旅行実績・読書ログ・家計簿・睡眠・分析・地図など）は本体の外で作り、何を読み書きできるかは本人の承認で決まる。
AI はその衛星の 1 つである（要件 §1.1 / §1.3）。

> ★ 2026-10-11 訂正（2026-10-11 の整合の確認）。従来は「AI と衛星アプリに読ませる」で、AI を主に置いていた。
> 要件の改訂（衛星の基盤が主、AI は衛星の 1 つ。プラグインは衛星に統合）に合わせた。

- 要件: [`docs/requirements.md`](docs/requirements.md)
- Story: [`docs/stories/INDEX.md`](docs/stories/INDEX.md)
- UI の方向: [`docs/ui-direction.md`](docs/ui-direction.md)
- 製造準備: [`docs/production-prep.md`](docs/production-prep.md)

## 構成

| 記号 | 何か | 言語 |
|---|---|---|
| S-01 | バックエンド（API・取り込み・衛星の基盤（登録・承認・資格情報）。★ 2026-10-11: 旧「プラグイン基盤」） | Rust |
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

本体は AGPL-3.0-only に、Google Play 開発者サービスの指定ライブラリとの組み合わせを許す追加の許可を付けています。正本は英文で、[`LICENSES/AdditionRef-ashiato-play-services.txt`](LICENSES/AdditionRef-ashiato-play-services.txt) にあります。

追加の許可の日本語は参考訳です。

> GNU AGPL version 3 第 7 条に基づく追加の許可。本プログラムまたは対象著作物を、Android SDK のライセンス（Android Software Development Kit License）の条項の対象となる部分を含む、下記の Google Play services ライブラリ（いずれの版でもよいが、改変版を除く）とリンクまたは組み合わせて改変した場合、本プログラムのライセンサーは、その組み合わせによる成果物を伝達する追加の許可をあなたに付与します。ソース形式でない組み合わせの対応ソースには、使用したそれらのライブラリの部分のソースコードを含めません。
>
> 対象ライブラリ: `play-services-location`、`play-services-base`、`play-services-basement`、`play-services-tasks`

貢献は [`CONTRIBUTING.md`](CONTRIBUTING.md) の CLA の下で受け付けます。
