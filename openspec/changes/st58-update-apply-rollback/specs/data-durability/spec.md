## MODIFIED Requirements

### Requirement: 書き出しは、更新の履歴と収集アプリの版の変化と生存信号の版を、説明つきで持つ

THE SYSTEM SHALL 全記録の書き出しに、更新の履歴の全行を 1 行 1 件で持つファイル `update_history.jsonl` を入れ、`seq`・`at`・`component`・`via`・`from_version`・`to_version`・`outcome`・`applied_versions`・`failed_version`・`sqlstate` の全列を出す。
THE SYSTEM SHALL 全記録の書き出しに、収集アプリの版の変化の読み口の全行を 1 行 1 件で持つファイル `collector_version_changes.jsonl` を入れる。
THE SYSTEM SHALL 生存信号のファイル `heartbeats.jsonl` に列 `collector_version` を出す。
THE SYSTEM SHALL 説明書に、3 つの説明を書く: 更新の履歴は「S-01 が自分の版の変化を書いた。前の版が空なら記録の始まり。落ちた回の行は、退避から D-01 を戻すと消えることがある。移行の結果 `rolled_back` は、更新の手順が退避から D-01 を戻した回で、前の版は落ちた新しい版、今の版は戻した先の版。直前に `failed` が無ければ移行は通ったが新しい版が待ち受けを開かずに戻した回で、当てた版（`applied_versions`）はその回に当てて戻した版の名前。`failed` があれば移行が落ちて戻した回。`rolled_back` は前の版に数えない。前の版の表が受け付けなかった回は、次に通った更新の後に控えから写すので書いた順（`seq`）では後ろに並び、時刻（`at`）は戻した時」、
収集アプリの版の変化は「生存信号から導いた。端末・部品ごとに発信の時刻の順に並べて、版が変わった所。収集アプリの申告で、S-01 は確かめていない」、生存信号の `collector_version` は「信号を積んだ時に動いていた収集アプリの申告。空なら申告なし」。
THE SYSTEM SHALL 更新の履歴の表を、書き出しの分類（記録の schema のどの表も入れるか入れないかが決まっている）に載せる。

導出元: FR-121。扉 #33・#14。深掘り（ST51）Q2（推奨）・Q5（推奨）・premises の C5・C8（design D3（仮）・D11）。review/spec.md R8・R10。
D-01 を書き出しから作り直しても、途絶の理由を区別する材料が残るように、書き出しに入れる。
★ ST58: 移行の結果 `rolled_back` の説明を足した（distribution の「更新の履歴は、決めた置き場と列の形で持つ」。深掘り（ST58）Q6・Q9）。

#### Scenario: 書き出しに更新の履歴の全行と全列が入る

- **GIVEN** 更新の履歴に、移行の結果 `applied` の行と `failed` の行がある D-01
- **WHEN** 全記録を書き出す
- **THEN** zip の `update_history.jsonl` は 2 行で、どちらの行も `seq`・`at`・`component`・`via`・`from_version`・`to_version`・`outcome`・`applied_versions`・`failed_version`・`sqlstate` を持ち、値は D-01 の行と同じ

#### Scenario: 書き出しに収集アプリの版の変化が入る

- **GIVEN** 端末 `d1` の位置の生存信号が版 `0.1.0+<commit A>` から `0.1.0+<commit B>` へ変わった D-01
- **WHEN** 全記録を書き出す
- **THEN** zip の `collector_version_changes.jsonl` は 1 行で、前の版 `0.1.0+<commit A>`・今の版 `0.1.0+<commit B>`・端末 `d1` を持つ

#### Scenario: 書き出しの生存信号のファイルに版の列が入る

- **GIVEN** `client_version` に `0.1.0+<commit A>` を持って届いた生存信号が 1 件ある D-01
- **WHEN** 全記録を書き出す
- **THEN** zip の `heartbeats.jsonl` のその行の `collector_version` は `0.1.0+<commit A>`

#### Scenario: 説明書が更新の履歴と版の変化と生存信号の版を説明する

- **WHEN** 全記録を書き出す
- **THEN** 説明書に `update_history.jsonl` と `collector_version_changes.jsonl` の節があり、`heartbeats.jsonl` の節に `collector_version` の説明がある
- **AND** `update_history.jsonl` の節に「戻すと消えることがある」と `rolled_back` と「`failed` が無ければ」と「控えから写す」、`collector_version_changes.jsonl` の節に「S-01 は確かめていない」、`collector_version` の説明に「空なら申告なし」の語句がある
