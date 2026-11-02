# ST52 — 3 つの部品を 1 つの版で、3 つのチャンネルに配る

## deep.md に積んだ未決（A）

- なし（`deep.md` に `(未回答)` は無い）

## 仮決め（B）と Ruling

- **Task の ledger の `Ruling:`: なし**（どの Task にも ledger `progress.md` が作られておらず、`Ruling:` 行は 0 件）
- design の仮決め（台帳 `provisional.jsonl`。確認バッチで一覧に出る）:
  - D2（仮）配った印 `v*` は消さない・動かさない。ルールセットで拒み、公開の前に確かめる
  - D3（仮）正式な鍵と開発版の鍵は RSA 4096・有効期限 25 年以上で作り、作る台本が確かめる
  - D4（仮）本人のいまのサーバは、印を打った直後にその印の版へ移す
  - D5（仮）PC の収集アプリを新しい版から古い版へ入れ直すことを ST52 では止めない（ST58）
  - D6（仮）緊急の修正の枝 `release/<MAJOR>.<MINOR>` は、配った正式版の印から切り、移行と封筒の形を変えない。
    finish で、比べる範囲を `migrations/ORDER` から `migrations/` の中身全体へ広げた（ORDER が同じでも配った移行の中身を書き換えた印を公開しない。`review/code.md` R24。決定の向きは同じ）
  - D21（仮）配布用の Environment `release` は印 `v*` にだけ開く（開発版の Environment は枝 `main`）
  - D23（仮）開発版の携帯の APK の道具は ST52 が作って確かめ、開発版のリリースに載せるのは ST54
  - D24（仮）祖先を見る印の判定も完全な履歴で走らせ、浅い木では落ちる

## Task

- 全 11 Task を埋めた（実装の 24 項目は `[x]`）
- 残した項目（正常な残り）: 9.1（PR・gate の段。`check-guards.sh --self-test` は PASS）、12.1〜12.4（人間の確認: 下書きのリリース・Environment の守り・ルールセットの読み・手順書）

## finish で処置した指摘（`review/code.md` R24〜R41。code-verify・security review・/code-review）

- fixed（2f284a59・432cfd77）:
  - R24 修正の枝で配った移行の中身を変えた印を `version.sh plan` が拒む（上記 D6）
  - R25・R33 `check-build-stamp.sh release` が自分で `<出所>-beta.2` を 4 部品に渡し、`check-artifacts.sh apk-version --expect-version` で APK の版を渡した値と比べる
  - R26 `sign-apk.sh` が鍵の証明書の指紋をチャンネルの期待値（正式 `ASHIATO_EXPECT_CERT_SHA256`・開発版 `ASHIATO_EXPECT_DEV_CERT_SHA256`）と比べ、違えば署名しない。release.yml の署名の段に指紋を渡す
  - R27 25 年に満たない 4096 bit の鍵を落とす self-test / R28 nightly の後に移行を変えても通る self-test
  - R29 `check-artifacts.sh`・`sign-apk.sh`・`make-key.sh` の self-test を ci.yml の package-linux で走らせ、`check-guards.sh workflows` が抜けを見張る
  - R31 `draft.sh` が run の出所（この repo の枝・`ci`・成功）を確かめ、fork の PR の配布物を下書きに載せない
  - R32 stack.sh が画面を組み込んだバイナリかを印のファイルで見て作り直す / R39 e2e の job の二重の組み立てを消した
  - R34 形の検査を出所の版の上で崩す / R35 浅いかの問い合わせの失敗で落とす / R36 版番号を 10 進数で読む
  - R37 build.sh の一時の置き場を trap で消す / R38 画面の印の試験を 1 つの名前にまとめた / R40 出所の版の読み方を言語ごとに 1 つに / R41 gradle の使われない行を消した
- 壊して落ちるのを確かめた: R24（比べる行を消す → `version.sh --self-test` rc=1）・R36（`10#` を外す → rc=1）・R27（下限 1 年・鍵 400 日 → `make-key.sh --self-test` rc=1）・R28（印の型 `'*'` → NG）・R29（ci.yml から 1 本消す → `check-guards.sh workflows` rc=1）・R26 の指紋の渡し（release.yml から消す → rc=1）・R37（trap を外す → 一時の置き場が残る）
- **注意**: tasks 3.1 の凍結した検証コマンドは `apk-version` を引数なしで呼ぶので、渡した版番号を見ないまま（旧コマンドは FAIL しないので `plan_fix.py` では直せない）。Scenario「組み立てた APK の versionCode は、渡した版番号から計算した値」は 3.4（CI の build-stamp の job）が担保する
- followup: R30（確認バッチの準備が本人の携帯に debug の鍵の版を入れうる。deep.md の K1）→ `docs/handoff/ST53.md`。ST53 の上流で引き受けるか `hx change` で CH を起こす
- scoped re-review 1 回: R24〜R41 は ADDRESSED（R30 は handoff の commit で閉じる）、新しい Critical / Important なし。Minor 1 件（keytool の失敗で理由が出ない）は 432cfd77 で直した

## 通した検証コマンド

`evidence.jsonl` の最後の記録（HEAD 432cfd77。全部 `PASS`・終了コード 0）:

| 項目 | コマンド（要約） | rc |
|---|---|---|
| 1.1・8.1 | `version.sh --self-test` / `code 0.3.0-beta.2 = 30002 && code 1.0.0 = 1000099` | 0 |
| 1.2 | `grep -q "release/" CLAUDE.md` | 0 |
| 2.2 | `--test server_startup st52_bin_` / `check-openapi.sh` / `migration_ledger` | 0 |
| 3.1 | gradle `verifyVersionCodeTable` / `<出所>-beta.2` の `assembleRelease && apk-version` / `--self-test-gradle-range` | 0 |
| 3.2 | vitest `build-stamp.test.ts` / `npm run build` | 0 |
| 3.3 | `check-version.sh --self-test` / 本体 | 0 |
| 3.4 | `check-build-stamp.sh release` / `android` | 0 |
| 4.1・4.2 | `check-released-migrations.sh` / `check-contract-version.sh`（`--self-test` と本体） | 0 |
| 4.3・11.1 | handoff の grep / `docs/release.md` の 8 語 / `check_scenarios.py` | 0 |
| 5.3 | e2e `st52-served-ui.spec.ts` / `login.spec.ts stack.spec.ts` | 0 |
| 6.1・6.2 | `build.sh --self-test` / `check-artifacts.sh --self-test` | 0 |
| 7.1・7.2 | `make-key.sh --self-test`・P17 の grep / `sign-apk.sh --self-test` | 0 |
| 9.2・9.3 | `check-guards.sh --self-test`・`workflows` / `package.sh --self-test` / actionlint 3 本 / 判定の job の `fetch-depth: 0` | 0 |
| 10.1 | `draft.sh --self-test` | 0 |

ほかに手元で: `vitest run` 全体 512 passed（1 回目は裏の重い組み立てと同時で 2 本落ち、走らせ直して全部通った）・`check_scenarios.py` OK・`check-shared-appends.sh` OK・`check-tmp-hygiene.sh` OK・`provisional.py check` rc=0・`review_triage.py` OK・`openspec validate --strict` OK

## 未処置の申し送り

- ST53: K1（`tools/verify-prep.sh`。R30）・IMP-01（印を push できる人と正式な鍵。既存）・最初の印と開発版の公開
- ST54: 開発版の署名は `ASHIATO_EXPECT_DEV_CERT_SHA256` を渡さないと署名しない（R26）・開発版の鍵と APK・R11 の nightly の DB
- ST58: D5（PC の収集アプリの入れ直し）
