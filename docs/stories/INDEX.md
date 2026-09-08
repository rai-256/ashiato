# Story 一覧

- 要件 `docs/requirements.md` から分解。**Story 36 本 / capability 13 本**
- `layer` は `requires` の DAG の深さ。**layer 0 は依存なし＝すぐ着手できる**
- `doors` と逐語引用は `scripts/make_story.py` が要件本文から機械的に埋めている（手で写していない）

## 着手できる順（layer）

- **layer 0**: ST01
- **layer 1**: ST02, ST03, ST05, ST07, ST08, ST11, ST16, ST19, ST21, ST22, ST25, ST28, ST33
- **layer 2**: ST04, ST12, ST14, ST15, ST17, ST20, ST23
- **layer 3**: ST06, ST09, ST13, ST18, ST26, ST34
- **layer 4**: ST10, ST24, ST30, ST35, ST36
- **layer 5**: ST27, ST29, ST31
- **layer 6**: ST32

## 一覧

| id | 表題 | layer | satisfies | requires | doors |
|---|---|---|---|---|---|
| [ST01](ST01.md) | 位置の記録が端末から自宅 PC へ届き、原文ごと残る | 0 | FR-1, FR-10, FR-18, FR-19, FR-20, FR-21, FR-24, FR-25, FR-26, FR-27, FR-28, FR-29, FR-30, FR-61, PERM-1, NFR-1 | — | 6, 7, 8, 9, 10, 11, 13, 16 |
| [ST02](ST02.md) | 収集が動いていたかが日単位で見える | 1 | FR-33, FR-54, NFR-13 | ST01 | 14 |
| [ST03](ST03.md) | 同じ記録を何度送っても増えない | 1 | FR-22, FR-23 | ST01 | 12 |
| [ST04](ST04.md) | 圏外でも記録が失われない | 2 | FR-8, FR-9, NFR-7 | ST02, ST03 | 14 |
| [ST05](ST05.md) | 端末時計のずれを測って残す | 1 | FR-7 | ST01 | 5 |
| [ST06](ST06.md) | 携帯端末のアプリ利用を集める | 3 | FR-2 | ST04 | — |
| [ST07](ST07.md) | PC のアクティブウィンドウを集める | 1 | FR-12 | ST01 | — |
| [ST08](ST08.md) | PC のブラウザ履歴を集める | 1 | FR-13 | ST01 | — |
| [ST09](ST09.md) | 写真と動画をメタごと取り込み、写真は原本も置く | 3 | FR-3, FR-4, FR-5, FR-6, FR-32, NFR-6 | ST04 | 18, 19, 20, 22, 25 |
| [ST10](ST10.md) | 原本と記録の食い違いを毎週見つける | 4 | FR-72 | ST09 | 18, 20 |
| [ST11](ST11.md) | 健康データの履歴権限を初回起動で要求する | 1 | FR-11 | ST01 | 21 |
| [ST12](ST12.md) | 書庫を置くだけで過去のデータが入る | 2 | FR-14, FR-16, FR-17, FR-55 | ST03 | — |
| [ST13](ST13.md) | アカウント系のソースを定期取得する | 3 | FR-15, NFR-3, NFR-12 | ST12 | — |
| [ST14](ST14.md) | 収集が途切れたら気づける | 2 | FR-35 | ST02 | — |
| [ST15](ST15.md) | 収集をソース単位・期間指定で止められる | 2 | FR-34, FR-53 | ST02 | 14 |
| [ST16](ST16.md) | 位置から滞在を作り、派生を作り直せる | 1 | FR-31, FR-76 | ST01 | 7 |
| [ST17](ST17.md) | 毎日 30 秒で「その日どう感じたか」を残す | 2 | FR-36, FR-37, FR-39, FR-40, FR-41, FR-42, FR-43, FR-57 | ST16 | 1, 2 |
| [ST18](ST18.md) | 主観の紐づけ先を後から増やせる | 3 | FR-38 | ST17 | 2 |
| [ST19](ST19.md) | 個人属性を上書きせず履歴で残す | 1 | FR-44, FR-45 | ST01 | 3 |
| [ST20](ST20.md) | 人物を登録して滞在に紐づける | 2 | FR-46, FR-47, PERM-5 | ST16 | 4 |
| [ST21](ST21.md) | 場所を登録し、識別子を変えない | 1 | FR-48, FR-49 | ST01 | 17 |
| [ST22](ST22.md) | 記録を消したことにできる | 1 | FR-50 | ST01 | — |
| [ST23](ST23.md) | 本文を本当に消せる | 2 | FR-51, FR-52 | ST22 | — |
| [ST24](ST24.md) | 記録に感度を持たせ、既定で守る | 4 | PERM-2, PERM-3, PERM-4, PERM-6, PERM-9 | ST09, ST17 | 15 |
| [ST25](ST25.md) | 1 日を時刻順に見る | 1 | FR-56 | ST01 | — |
| [ST26](ST26.md) | 語で探す | 3 | FR-58, NFR-4 | ST17 | — |
| [ST27](ST27.md) | AI から MCP で問い合わせる | 5 | FR-59, FR-60, NFR-14 | ST24, ST26 | 15, 26 |
| [ST28](ST28.md) | 外からは Tailscale 網内でしか届かない | 1 | PERM-7, NFR-15 | ST01 | 23 |
| [ST29](ST29.md) | プラグインを登録し、承認して初めて読ませる | 5 | FR-62, FR-63, FR-64, FR-65, PERM-8 | ST24 | 15 |
| [ST30](ST30.md) | 毎日と毎週のバックアップが回る | 4 | FR-66, FR-67, FR-70, NFR-5 | ST09 | 20, 23 |
| [ST31](ST31.md) | 暗号化した写しを拠点外へ送る | 5 | FR-68, FR-69, NFR-9 | ST30 | 23, 24 |
| [ST32](ST32.md) | 復元テストを促す | 6 | FR-71, NFR-10 | ST31 | — |
| [ST33](ST33.md) | 全記録を外部の道具で読める形に書き出す | 1 | NFR-16 | ST01 | — |
| [ST34](ST34.md) | 健康データを取り込む | 3 | FR-73, NFR-2 | ST11, ST04 | 21 |
| [ST35](ST35.md) | 画面を一定間隔で取る | 4 | FR-74 | ST09 | 22, 25 |
| [ST36](ST36.md) | 意味で探す | 4 | FR-75 | ST26 | — |

## capability（仕様の置き場）

OpenSpec の capability は `openspec/specs/<名前>/spec.md` に残り続ける。**Story は archive されて
使い捨てになるが、capability は残る。名前は後から変えられない**（OpenSpec が明示している:
*"Do not move or rename the capability."*）。迷ったら粗いほうに寄せた。

| capability | 何の能力か | 積む Story |
|---|---|---|
| `record-envelope` | 記録の骨格・原文・エンベロープ・API 契約 | ST01, ST03, ST05 |

> **訂正（2026-09-08、ST01 の上流工程で判明）**: 当初 ST01 を `record-envelope` にだけ
> 割り当てていたが、FR-1（C-01 が 60 秒間隔で位置を取る）と FR-10（到達できるとき送る）は
> **端末側の振る舞い**であって記録の骨格ではない。この表のままだと FR-1 の置き場が
> ST04（layer 2）まで存在しないことになる。**capability 名は変えず**、
> `device-collection` の作成を ST01 に前倒した。後続の割り当ては壊れていない。
| `device-collection` | 携帯端末からの収集 | **ST01**, ST04, ST06, ST09, ST11, ST34, ST35 |
| `desktop-collection` | PC からの収集 | ST07, ST08 |
| `external-ingestion` | 外部サービスからの取り込み | ST12, ST13 |
| `collection-coverage` | 収集の稼働状況・通知・停止 | ST02, ST14, ST15 |
| `derived-records` | 派生（滞在） | ST16 |
| `subjective-log` | 主観・感情の記録 | ST17, ST18 |
| `personal-entities` | 個人属性・人物・場所 | ST19, ST20, ST21 |
| `record-deletion` | 削除 | ST22, ST23 |
| `data-sensitivity` | 感度・アクセス制御・プラグイン権限 | ST24, ST28, ST29 |
| `browsing-views` | 閲覧と検索 | ST25, ST26, ST36 |
| `ai-access` | AI からの問い合わせ | ST27 |
| `data-durability` | バックアップ・整合・可搬性 | ST10, ST30, ST31, ST32, ST33 |

## どの Story にも拾われていない要件

**NFR-8（開発費 月 2,000 円以内）と NFR-11（開発に充てられる時間 週 15 時間）の 2 件。**
これは実装する対象ではなく**プロジェクトの制約**なので、意図的にどの Story にも割り当てていない。
`python3 scripts/check_chain.py` はこの 2 件を「未回収」として FAIL を返し続ける。**これは正しい状態**。

## 着手前に閉じる必要がある扉

| 扉 | 期限 | 効く Story |
|---|---|---|
| #26 成功条件 2 の質問セット 10 問 | `/stories` 着手前（**現在未了**） | ST27 |
| #24 バックアップ暗号鍵の保管方式 | 初回バックアップの実行前 | ST31 |
| #25 画面キャプチャの保持期間と文字抽出エンジン | 画面キャプチャの収集開始前 | ST35 |
