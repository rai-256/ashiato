# CH24 の前提

| 前提 | 何が依るか | 確かめ方 | 結果 | 出所 | 確かさ |
|---|---|---|---|---|---|
| zip 2.6.1 の勧告は直った版がもう出ている | 例外の理由が事実と違うこと・上げる先 | osv.dev の `v1/query` に `zip` の 2.6.1・9.0.5・9.7.3 を問い合わせた（2026-11-20） | 2.6.1 は 9 件（GHSA-8hxf・m665・q7v9 と RUSTSEC-2026-0367・0368・0369・0429・0545・0548）。9.0.5 と 9.7.3 は 0 件 | 公式文書（osv.dev） | 4 |
| 例外の表の zip の行は 7 つ | 消す行 | `osv-scanner.toml` を読んだ。RUSTSEC-2026-0545・0548 は GHSA-8hxf・m665 の別名で、表に行が無い | GHSA 3 行と RUSTSEC-2026-0367・0368・0369・0429 の 4 行。どれも理由は「上げる予定: 互換範囲外の修正版待ち」 | 実物 | 5 |
| RUSTSEC-2026-0367〜0369・0429 は 9.0.0 で直った | 4 行も消してよいこと | osv.dev の `v1/vulns/<id>` の affected の範囲 | どれも `fixed: 9.0.0` | 公式文書（osv.dev） | 4 |
| base の全体の検査は zip と無関係な勧告で既に落ちる | 検証に全体の検査でなく PR の検査を使うこと | `tools/check-vulns.sh` を base で走らせた（2026-11-20） | rc=1（chrono・libsqlite3-sys・rustix の 3 件）。`--base <merge-base>` の PR の検査は rc=0 | 実物 | 5 |
| zip 9 は rust 1.88 以上を求める | 組み立てが通るか | `cargo info zip@9`（9.7.3） | `rust-version: 1.88`。手元の rustc は 1.99、CI は `dtolnay/rust-toolchain` の stable | 実物 | 4 |
| zip 9 の既定の features は aes-crypto と deflate を含む | 書き出しの AES と Takeout の deflate を読むこと | `cargo info zip@9` | `default = [aes-crypto, bzip2, deflate64, deflate, lzma, ppmd, time, zstd, xz]` | 実物 | 4 |
| AE-1 / AE-2 の選び方は 2.6.1 と 9.7.3 で同じ（書いた量が 20 バイト未満なら AE-2） | 上げた後に書いた zip の AES の形が変わらないこと | 両方の版の `src/write.rs` を読んだ（2.6.1 は 1087 行付近、9.7.3 は 2979 行付近） | 同じ規則 | 公式文書（crate の source） | 2 |
| 手元に AES の zip を読める別の道具が無い | 「今と同じに読める」を何で確かめるか | `which 7z 7za 7zz`・`python3 -c 'import pyzipper'` | どれも無い。`unzip` は AES を読めない | 実物 | 5 |

確かさ 2 の AE-1 / AE-2 の前提は、`specs/data-durability` の Scenario「上げた後に書いた書き出しの zip の項目は、前の版と同じ AES の形で書かれる」が
上げた後の版で走らせて確かめる（tasks 2.2。外れれば落ちる）。
