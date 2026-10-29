spec-digest: fb049a9163202078f40a76caa07a0a5848f4b7a13b42b34903b019cb116cf023
- Q10: 写った
答え以外の変更: あり（D24（仮）の新設 —— 200 の読めない答えが 24 時間続いた後に、知らせだけでなく「S-01 の更新を待っている」状態そのものに（e）として入る。design D9 の（e）・D22 の「促す」の項と 7 日前の知らせの分け方の D24 への参照・device / desktop の Requirement の状態の 1 項（「…24 時間続いた」）・device の Requirement の「（通知と上限の 7 日前の知らせも同じ）」と 7 日前の知らせの句の（e）だけの添え文言・device の Scenario「読めない 200 の答えが続いて止まっている間の 7 日前の知らせは網の確認を添える」と tasks 11.1 のその印と `Retention.kt` の句・tasks 8.1 / 10.1 の「D9 の a〜e」・provisional.jsonl の D24 の 1 行）
答え以外の変更の決定: D24

## 確かめたこと（観点 6）

- 機械: `openspec validate st50-contract-version --strict` → valid（rc=0）。`check_chain.py .` → `chain: OK`（rc=0。FR-126 の★ 2026-10-29 を含めて観点 8 は通る）。
  `check_scenarios.py . st50-contract-version` → rc=1 だが [FAIL] は全部 ST50 の未実装の Scenario（上流の段では想定どおり）
- 写しの元: `deep-answers-2.txt` Q10「200 の読めない答えが 24 時間続いたら促す」、`deep-questions-r2.json` Q10 の選択肢の detail（最初の 24 時間は送らずに残す・促さない／網の確認を添えた文言／読める答えで数え直す／携帯・PC とも同じ）、
  `docs/requirements.md:1054` FR-126 ★ 2026-10-29 補足（数え始めは再起動をまたぐ・読める答えでだけ数え直す・届かない / 時間切れ / 5xx では数え直さない・文言「サーバを更新するか、つながっている網を確かめてください」）
- design: D22 を書き直し（数え始め・数え直し・促す・文言・携帯と PC で同じ）、D9（e）、D11、D5 / D6 の文言の注記、冒頭注記、Open Questions。文言は FR-126 ★ の「サーバを更新するか…」で、選択肢の detail の「S-01 を更新するか…」ではなく要件の現在の本文に揃っている
- specs: device-collection / desktop-collection の Requirement 本文に 24 時間・再起動をまたぐ・読める答えでだけ数え直す・到達できない / 時間切れ / サーバ側の失敗で数え直さない・文言がある。
  Scenario は各 cap で改名 1（23 時間では促さない）+ 新設 3（24 時間で網の確認を添えて促す／再起動と到達できない間が挟まっても数え始めは変わらない／読める答えで数え直す）。数値 24 時間は 23 時間 / 24 時間の両側で Scenario にある
- 旧い Scenario 名「（PC は）読めない 200 の答えだけでは更新を促さない」は specs / tasks / design / provisional.jsonl から消えている（残るのは deep.md の記録・go.json・review/* だけ。go.json は `reading --carry` が直す場所で spec の段の範囲外）
- tasks の `// Scenario:` 印: 新しい 8 本（device 4・desktop 4）は Scenario 名と一字一句で一致（`grep -cF` で各 1）。
  検証の本数の下限: 9.1 は 7 → 10（−1 +4。印の数 10 と一致）、11.1 は 8 → 12（−1 +4、R1 の直しで +1。印の数 12 と一致）。再確認（R1〜R3 の直しの後）: validate valid・chain OK・新しい印は名前と一字一句一致、503 の区間は FR-126 ★ の「5xx」の写し
- D24 の判定: FR-126 ★ と答えは「24 時間続いてから出す」までで、24 時間の後に状態（抜ける条件・D14 の飛ばす印の破棄・7 日前の知らせの添え文言）に入るか知らせだけかは決めていない。D24（仮）は design の節として書かれ、台帳にも載る。
  D22（仮でない）の本体・D5 / D6 / D11 の注記・Scenario 8 本は Q10 と FR-126 ★ の写しで、どの D<n> にも当たらない変更は見当たらなかった
- D22 の「404 が挟まっても数え始めを捨てない」は FR-126 ★ に明示が無いが、「読める答えを得たときだけ数え直す」の排他から導かれ、食い違いではない

## R1. 携帯の 7 日前の知らせで（e）だけのときの文言が Requirement の中で食い違い、Scenario も試験も無い
- 成果物: openspec/changes/st50-contract-version/specs/device-collection/spec.md
- 根拠: device-collection/spec.md:93「（e）だけのときは…網を確かめてください（通知と上限の 7 日前の知らせも同じ）」と、無変更の同:97「WHILE S-01 の更新を待っている THE SYSTEM SHALL 上限の 7 日前の知らせに『サーバを更新してください』を添える」が並ぶ。
  Scenario（同:152「契約の版で止まっている間の 7 日前の知らせは更新を促す」）の THEN は「『サーバを更新してください』が含まれる」で、網の確認の文言（「サーバを更新するか、…」）はこの部分文字列を含まないので、（e）だけの場合を測れない。tasks.md:221 の `Retention.kt` の句も「サーバを更新してください」を添えるのまま。
  加えて design D22（仮でない）の「文言」の項が 7 日前の知らせの分け方を言い切っているが、これは D24（仮）の中身（D24 の節が「D6 の上限の 7 日前の知らせへの添え文言」を挙げる）
- kind: technical
- 提案: 同:97 を「待っている理由に応じた文言（D22）を添える」に直し、（e）だけで 83 日に達したときの Scenario（THEN に網の確認の文言）と tasks 11.1 の印・`Retention.kt` の句を足す。D22 の 7 日前の知らせの一文に「（D24（仮））」を付ける

- 処置: fixed 11.1 —— specs/device-collection/spec.md（:97 に（e）だけのときの添え文言・Scenario「読めない 200 の答えが続いて止まっている間の 7 日前の知らせは網の確認を添える」）・tasks.md 11.1（印・`Retention.kt` の句・下限 12 本）・design.md D22（7 日前の知らせの分け方に D24（仮）を付けた）

## R2. 「時間切れ・5xx が挟まっても数え直さない」が Requirement の本文にだけあり、Scenario は到達できない（圏外）しか測らない
- 成果物: openspec/changes/st50-contract-version/specs/device-collection/spec.md・specs/desktop-collection/spec.md
- 根拠: requirements.md:1054 FR-126 ★「届かない・時間切れ・5xx が挟まっても数え直さない」。device-collection/spec.md:92・desktop-collection/spec.md:64 は本文で言うが、Scenario（device:142・desktop:113）の WHEN は「圏外」「到達できない」だけ。
  実装が 5xx の答えで数え始めを捨てても spec は落ちない
- kind: technical
- 提案: 両 cap の数え始めの Scenario の挟まる間に「S-01 が 503 を返す」区間を足すか、5xx だけの Scenario を 1 本足し、tasks 9.1 / 11.1 の印と下限を合わせる

- 処置: fixed 9.1 —— specs/device-collection/spec.md・specs/desktop-collection/spec.md（数え始めの Scenario の挟まる間に 503 の区間を足した）・tasks.md 9.1 / 11.1（試験の注記に 503 の区間）

## R3. tasks の順: 8.1 / 10.x が後の 9.1 / 11.1 の置き場に数え始めの時刻を持つ
- 成果物: openspec/changes/st50-contract-version/tasks.md
- 根拠: tasks.md 8.1「数え始めの時刻は 9.1 の判定と同じ手元のファイルに持つ」・10.x「数え始めの時刻は 11.1 の `SharedPreferences` に持ち」。置き場を作るタスクが使うタスクより後にある
- kind: technical
- 提案: 数え始めの時刻の置き場（読み書きの口）を 8.1 / 10.x 側で作ると書くか、9.1 / 11.1 が 8.1 / 10.x の置き場を使うと書き換える
- 処置: fixed tasks.md 8.1 / 10.1（数え始めの時刻の置き場をこの Task で作り、9.1 / 11.1 が同じ置き場に通知の時刻を足す、と書き換えた）
