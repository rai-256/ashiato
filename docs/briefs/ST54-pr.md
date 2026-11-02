# ST54 — 開発版を正式版と並べて入れられる（上流）

> 第 1 回の深掘りは**推奨の答えで先に**書いた（2026-10-20 の流れ）。第 1 回の答え（2026-11-02）で Q1〜Q10 は本人が推奨のまま答えた。
> 合意では重大な IMP-01・IMP-02 を「仕様を直す」と答えたので、この版で直した（下の「合意の差し戻しで直したもの」）。

## 深掘りで本人が決めたこと（A。推奨のまま答えた）

- Q4（loss: discarded）開発版の DB が main の移行とずれたら → 推奨: 止まって原因を示し、DB は書き換えず main 側で直す（D4 `migrations/LINEAGE`・D5（仮）の CI の検査）
- Q6（loss: rewrite-all）開発版の収集アプリの送り先 → 推奨: 開発版のサーバにだけ送る（D7（仮）。`GET /contract` の応答の見出し `Ashiato-Channel` で系統を確かめる）
- Q9（loss: exported）完了を何で確かめるか → 推奨: 下書きのリリースと使い捨ての鍵（D10）。本物の開発版の鍵と公開は ST53 の後に本人が始める

仮でよい問い（B。推奨のまま答えた）: Q1 系統は組み立ての版番号で決める（D1）/ Q2 PC の開発版は置き場の下に `dev`・端末識別子に `-dev`・除外は渡された置き場から読む（D2）/ Q3 DB の系統は版の記録の最初の行で決め、違えば終了コード 4（D3）/
Q5 `nightly` は動かす方式のまま / Q7 名前に「開発版」/ Q8 説明文の先頭に「サポートしない」を公開のたびに書き直す（D9）/ Q10 開発版も全部の収集を動かす（D11）

## 仮決め（台帳 `provisional.jsonl`。別の席の確かめは merge gate の前にグラフが行う）

- D5（仮）開発版に載った移行を、旧名の対応や直しの移行なしに変えた PR を CI で落とす（`tools/check-nightly-migrations.sh`）
- D7（仮）正式版の収集アプリも開発版のサーバへは送らない（Q6 の逆向き。正式版の記録が開発版の DB に入らない）
- D8（仮）開発版のサーバの既定（待ち受け `127.0.0.1:18788`・クッキー `ashiato_session_dev`・退避 `backups/pre-migrate-dev`）・開発版の DB のポート 55433・PC の開発版の実行ファイル `ashiato-collector-windows-dev.exe`

## capability

- `distribution` だけ。前倒しは無い（INDEX.md の割り当てどおり）。PC の収集アプリ・携帯・画面のログインに触れる振る舞いも、どれも「開発版のときだけ」「相手の系統が違うときだけ」の分岐なので、系統の規則を 1 つの正典にまとめた。
  送信契約の問い合わせの答えの形（record-envelope）を変えないよう、系統は応答の見出しで運ぶ
- MODIFIED 2（署名・下書きのリリース）、REMOVED 2 と名前を変えた ADDED 2（「ST54 まで」の例外を外す。OpenSpec が MODIFIED で Scenario を落とせないため）、新しい ADDED 10
- Scenario 90 本（新しく印を置くのは 61 本）

## tasks

- 11 Task（実装 21 項目）＋人間の確認待ち 2 項目（開発版の下書きのリリース・並べて入れる手順書）
- ST52 の印 3 つ（「ST54 まで」の Scenario）を消して新しい向きに書き直す

## レビュー

- `review/deep.md`（問いの独立検査）R1〜R12: escalated 7（Q2・Q4・Q6・Q9・Q10 に入れた）・fixed 4（deep-questions 3・premises 1）・fixed D8 仮 1
- `review/spec.md`（spec-review）R1〜R15: fixed 14（spec 10・tasks 3・spec と design 1）・escalated 1（R5。Q2 の context に読み替えを足した）
  - 主なもの: R1 系統を `/contract` の本文の欄でなく見出しで運ぶ（record-envelope の答えの形と ST50 の試験を変えない）/ R12 開発版の下書きを正典の下書きの Requirement に足した / R11 名前と中身を両方変えた移行も落とす

  - 第 2 回（合意の差し戻しの直し）R16〜R18: fixed 3（tasks 2・proposal と ST58 の requires 1）

## 合意の差し戻しで直したもの（第 1 回の合意）

- IMP-01（PC の開発版で、ブラウザの履歴の除外が効かない恐れ）→ 除外の条項を「前景の窓の記録でもブラウザの履歴の取り込みでも」にし、Scenario「開発版は、ブラウザの履歴の取り込みでも渡された置き場の除外の登録を読む」を足した。D2・tasks 6.1 に履歴の取り込みの口（`history/collect.rs:315`・`Runtime::with_history`）と、直していないコードで落ちる試験の組み方を書いた
- IMP-02（更新の失敗からの戻しが終了コード 4 を知らない）→ D3 と tasks 3.2 で `docs/handoff/ST58.md` に「終了コード 4 の回は退避から戻さない」を書き足す。届くように ST58 の requires に ST54 を足した（INDEX.md の訂正 2026-11-02）

## 要件と Story へ戻したもの

- FR-104 の ★ 2026-11-01「開発版のリリースには ST54 まで C-01 を並べない」は期限を自分で持つので、改訂しない
- Q9（推奨のまま）: 完了の判定に、下書きのリリースと使い捨ての鍵で確かめる 1 行を `stories.json` に足した（D10）
- ST58 の requires に ST54（順の付け方だけ。要件は変わらない）

🤖 Generated with [Claude Code](https://claude.com/claude-code)
