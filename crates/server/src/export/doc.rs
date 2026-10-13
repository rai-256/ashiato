// SPDX-License-Identifier: AGPL-3.0-only
//! 書き出しの説明書 `README.txt` と目録 `manifest.json`（ST33 / design D9）。
//!
//! 列の意味・出さない列・入らない表は `CATALOG` から引く（分類を 2 か所で持たない）。
//! **除いた行の件数はどこにも書かない**（C8）。ここにある数は「書いた行の数」と「書いたバイト数」だけ。

use serde_json::{json, Value};

use super::{Class, CATALOG};

/// 同じ内容の行を 1 件として読む規則（C3）。`core.event_folded`（`202609120943_version_and_ledger.sql`）の定義と
/// 食い違わないことを、テストが `pg_get_viewdef` と突き合わせる。**ビューが変わったらここを合わせる。**
pub const FOLD_RULE: &str = "core.event_folded と同じ規則で読む。user_id・logical_source・content_hash が同じ行を 1 件とみなし、\
時刻はいちばん早い event_time、原文（raw）・内容（payload）・由来（origin）は ingest_time, id の順で最初の行、\
感度（sensitivity）はいちばん厳しい（大きい）値、外部の識別子（external_id）は全部を並べて持つ。\
この書き出しはこの規則を適用せず、内容が同じ行も畳まずにそのまま出している。";

/// 書いたファイル 1 つの実績。`rows` は書いた行を数えた値（数え直しではない）。
#[derive(Debug, Clone)]
pub struct FileStat {
    pub name: &'static str,
    /// 読んだ元（`core.event_live` のような名前）。
    pub source: String,
    pub rows: u64,
    pub bytes: u64,
    pub columns: &'static [(&'static str, &'static str)],
}

/// 入らないものの一覧（説明書にも目録にも同じものを書く）。
pub fn not_included() -> Vec<String> {
    let mut out = vec![
        "本人が消した場面: 削除の印の付いた記録と、本人が消した滞在の時間に入る緯度経度を持つ記録（それらの前の版を含む）".to_string(),
        "取り込み元の原本のファイル（D-02 の原本）。この書き出しは DB の記録だけを持つ".to_string(),
    ];
    for (table, class) in CATALOG {
        match class {
            Class::Include {
                excluded_columns, ..
            } => {
                for col in *excluded_columns {
                    out.push(format!("{table}.{col}: 出さない列"));
                }
            }
            Class::Exclude { reason } => out.push(format!("表 {table}: {reason}")),
        }
    }
    out.push(
        "event_versions.jsonl は、前の版の表の列だけを持つ。親の記録の sensitivity・origin・external_id は events.jsonl にある".to_string(),
    );
    out
}

/// 説明書（日本語。外の道具なしで読める）。
pub fn readme(
    export_id: &str,
    snapshot_at: &str,
    exported_at: &str,
    schema_version: &str,
    files: &[FileStat],
) -> String {
    let mut s = String::new();
    s.push_str("ashiato 書き出し — 説明書\n\n");
    s.push_str("1. これは何か\n");
    s.push_str("  ashiato に記録した行動の全記録の写しです。\n");
    s.push_str(&format!("  書き出しの識別子: {export_id}\n"));
    s.push_str(&format!("  写しの時点: {snapshot_at}\n"));
    s.push_str(&format!("  書き出した時刻: {exported_at}\n"));
    s.push_str(&format!("  記録の形の版: {schema_version}\n"));
    s.push_str(
        "  この zip は書き出すときに入れた合言葉で暗号化してある。7-Zip などの AES に対応した道具で開く。\n",
    );
    s.push_str("  中のファイルは UTF-8 の JSON Lines（1 行が 1 件の JSON）。manifest.json にも同じ内容を機械で読める形で書いてある。\n\n");

    s.push_str("2. ファイル（名前・元・行の数・列）\n");
    for f in files {
        s.push_str(&format!("\n  {}\n", f.name));
        s.push_str(&format!("    元: {}\n", f.source));
        s.push_str(&format!("    行の数: {}\n", f.rows));
        s.push_str("    列:\n");
        for (col, meaning) in f.columns {
            s.push_str(&format!("      {col} — {meaning}\n"));
        }
    }
    s.push_str(
        "\n  README.txt — この説明書\n  manifest.json — 目録（上と同じ内容を JSON で持つ）\n\n",
    );

    s.push_str("3. 入らないもの\n");
    for line in not_included() {
        s.push_str(&format!("  - {line}\n"));
    }
    s.push('\n');

    s.push_str("4. 同じ内容の行を 1 件として読む規則\n");
    s.push_str(&format!("  {FOLD_RULE}\n\n"));

    s.push_str("5. 時刻の書き方\n");
    s.push_str("  時刻はすべて UTC で、2026-09-07T01:00:00.000000Z のようにマイクロ秒まで書き `Z` で終わる。日付だけの列は YYYY-MM-DD。\n");
    s.push_str("  出来事が起きた場所の時刻は、tz_offset_min（UTC からのずれ。分）と tz_id（タイムゾーン）の列から読む。\n");
    s
}

/// 目録。`rows` は書いた行の数。
pub fn manifest(
    export_id: &str,
    snapshot_at: &str,
    exported_at: &str,
    schema_version: &str,
    files: &[FileStat],
) -> Value {
    json!({
        "format": 1,
        "export_id": export_id,
        "snapshot_at": snapshot_at,
        "exported_at": exported_at,
        "schema_version": schema_version,
        "files": files.iter().map(|f| json!({
            "name": f.name,
            "source": f.source,
            "rows": f.rows,
            "bytes": f.bytes,
            "columns": f.columns.iter()
                .map(|(name, meaning)| json!({"name": name, "meaning": meaning}))
                .collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "not_included": not_included(),
        "fold_rule": FOLD_RULE,
    })
}
