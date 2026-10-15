// SPDX-License-Identifier: AGPL-3.0-only
//! 全記録の書き出しの骨組み（ST33 / design D2 / D3 / D8 / D9）。
//!
//! 本体は「分類 → 列の突き合わせ → 写し（除く範囲を当てた表ごとの JSON Lines・説明書・目録）を
//! 合言葉で暗号化した zip として `.partial-….zip` に書く → `rename`」。台帳と前のファイルの片付けは `job`。
//! 試験が段ごとに失敗を差し込む口（`Hooks`）はここで持つ。
//!
//! 分類の正本は `CATALOG` の 1 つ。説明書の列の意味もここから引く（2 か所で持たない）。
//! 記録の値・合言葉・置き場のフルパスは、エラーの文言にもログにも載せない。

use std::collections::BTreeSet;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use std::time::Duration;

use zeroize::Zeroizing;

mod bloom;
mod doc;
mod dump;
mod excluded;
mod job;
mod ledger;
mod summary;

pub use bloom::Bloom;
pub use doc::*;
pub use dump::{format_time, Dumped};
pub use excluded::*;
pub use job::*;
pub use ledger::*;
pub use summary::*;

/// 合言葉の下限（Unicode の符号位置。C23。上げるのはよいが、下げるのは本人に返す）。
pub const MIN_PASSPHRASE_CHARS: usize = 12;

/// 合言葉。本体の引数にだけ渡し、どこにも置かない。`Debug` では伏せ、捨てるとき上書きする（Q9）。
pub struct Passphrase(Zeroizing<String>);

impl Passphrase {
    pub fn new(phrase: String) -> Self {
        Self(Zeroizing::new(phrase))
    }

    /// 既に `Zeroizing` に包んだ合言葉を受ける（口が本体から直に読んだもの。写しを作らない）。
    pub fn from_zeroizing(phrase: Zeroizing<String>) -> Self {
        Self(phrase)
    }

    /// 暗号化に渡すときだけ読む。
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for Passphrase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Passphrase(..)")
    }
}

/// 失敗の種別。応答の `last_failure.kind`（design の GET /export）にそのまま出る。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailKind {
    /// 分類の無い表・列がある（D8）。
    UnclassifiedTable,
    /// 読み出しに欠けた列がある（D9）。
    MissingColumn,
    Io,
    Db,
    /// 遅れて届いた位置の印を付ける段（D6）で失敗した。
    LateMark,
}

impl FailKind {
    pub const ALL: [Self; 5] = [
        Self::UnclassifiedTable,
        Self::MissingColumn,
        Self::Io,
        Self::Db,
        Self::LateMark,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::UnclassifiedTable => "unclassified_table",
            Self::MissingColumn => "missing_column",
            Self::Io => "io",
            Self::Db => "db",
            Self::LateMark => "late_mark",
        }
    }

    /// 台帳の `reason`（`ExportError` の `Display` = `<種別>: <理由>`）から種別を戻す。
    /// 書く側（`ledger::insert_failed` が `ExportError` から作る）と同じ形だけを読み、種別で始まらなければ `None`。
    pub fn of_reason(reason: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| {
            reason
                .strip_prefix(k.as_str())
                .is_some_and(|rest| rest.starts_with(':'))
        })
    }
}

/// 書き出しの失敗。`reason` は表・列・段の名前だけで、記録の値を含まない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportError {
    pub kind: FailKind,
    pub reason: String,
}

impl ExportError {
    pub fn new(kind: FailKind, reason: impl Into<String>) -> Self {
        Self {
            kind,
            reason: reason.into(),
        }
    }

    fn io(what: &str, e: &std::io::Error) -> Self {
        // パスを含む `Display` ではなく種別だけを書く（置き場のフルパスを出さない）。
        Self::new(FailKind::Io, format!("{what}: {:?}", e.kind()))
    }
}

impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.kind.as_str(), self.reason)
    }
}

impl std::error::Error for ExportError {}

// ---------------------------------------------------------------------------
// 置き場の決め方（D3）
// ---------------------------------------------------------------------------

/// 置き場を決める。環境は引数で受ける（純関数）。
/// `ASHIATO_EXPORT_DIR` → `LOCALAPPDATA\ashiato\export` → `XDG_DATA_HOME/ashiato/export` → `HOME/.local/share/ashiato/export`。
/// 空の値は無いものとして扱う。どれも無ければ `None`。**ドキュメント・デスクトップの下を既定にしない**（C16）。
pub fn export_dir(env: impl Fn(&str) -> Option<String>) -> Option<PathBuf> {
    let get = |k: &str| env(k).filter(|v| !v.is_empty());
    if let Some(dir) = get("ASHIATO_EXPORT_DIR") {
        return Some(PathBuf::from(dir));
    }
    let app = |base: PathBuf| base.join("ashiato").join("export");
    if let Some(local) = get("LOCALAPPDATA") {
        return Some(app(PathBuf::from(local)));
    }
    if let Some(xdg) = get("XDG_DATA_HOME") {
        return Some(app(PathBuf::from(xdg)));
    }
    get("HOME").map(|home| app(PathBuf::from(home).join(".local").join("share")))
}

// ---------------------------------------------------------------------------
// 分類（D8）
// ---------------------------------------------------------------------------

/// 表の分類。
#[derive(Debug)]
pub enum Class {
    /// 書き出す表。`view` は読む元（`core.` の下の名前）、`excluded_columns` は出さない列、
    /// `columns` は出す列とその意味（説明書の列の説明になる）。
    Include {
        file: &'static str,
        view: &'static str,
        excluded_columns: &'static [&'static str],
        columns: &'static [(&'static str, &'static str)],
    },
    /// 書き出さない表。理由は説明書の「入らないもの」に使える文。
    Exclude { reason: &'static str },
}

const EVENT_COLUMNS: &[(&str, &str)] = &[
    ("id", "記録の識別子（収集側で振る）"),
    ("user_id", "利用者の識別子"),
    ("logical_source", "記録のソースの名前"),
    ("external_id", "外部サービス上の識別子"),
    ("device_id", "記録した端末"),
    ("origin", "由来（収集・取り込みなど）"),
    ("event_time", "出来事が起きた時刻（UTC）"),
    ("ingest_time", "このシステムに入った時刻（UTC）"),
    ("tz_offset_min", "出来事の時点の UTC からのずれ（分）"),
    ("tz_id", "出来事の時点のタイムゾーン識別子"),
    ("schema_version", "書かれた時点の記録の形の版"),
    ("unit_system", "単位系"),
    ("crs", "座標系"),
    ("sensitivity", "感度（大きいほど厳しい）"),
    ("content_hash", "内容のハッシュ（同じ内容の判定）"),
    ("raw", "収集側から届いた原文（逐語の文字列）"),
    ("payload", "解析済みの内容"),
    ("deleted_at", "論理削除の時刻"),
    ("deleted_by", "論理削除を行った主体"),
    ("source_updated_at", "元のサービスでの更新時刻（UTC）"),
    ("external_ref", "外部サービス上の参照"),
];

const EVENT_VERSION_COLUMNS: &[(&str, &str)] = &[
    ("id", "前の版の識別子"),
    ("event_id", "元の記録の識別子"),
    ("user_id", "利用者の識別子"),
    ("logical_source", "記録のソースの名前"),
    ("version_no", "版の番号"),
    ("event_time", "出来事が起きた時刻（UTC）"),
    ("content_hash", "内容のハッシュ"),
    ("raw", "その版の原文（逐語の文字列）"),
    ("payload", "その版の解析済みの内容"),
    ("source_updated_at", "元のサービスでの更新時刻（UTC）"),
    ("external_ref", "外部サービス上の参照"),
    ("superseded_at", "次の版に置き換わった時刻（UTC）"),
    ("tz_offset_min", "出来事の時点の UTC からのずれ（分）"),
    ("tz_id", "出来事の時点のタイムゾーン識別子"),
    ("schema_version", "書かれた時点の記録の形の版"),
    ("unit_system", "単位系"),
    ("crs", "座標系"),
];

const SOURCE_COLUMNS: &[(&str, &str)] = &[
    ("logical_source", "ソースの名前"),
    ("display_name", "表示名"),
    ("expected_gap_sec", "想定される最大の無通信間隔（秒）"),
    ("registered_at", "登録した時刻（UTC）"),
    ("user_id", "利用者の識別子"),
    ("collection_started_on", "収集を始めた日"),
    ("retired_on", "収集をやめた日"),
    ("succeeds", "引き継ぎ元のソース"),
    ("external_id_kind", "外部の識別子の種類"),
];

const ATTRIBUTE_KIND_COLUMNS: &[(&str, &str)] = &[
    ("id", "属性の種類の識別子"),
    ("seq", "作った順の番号"),
    ("user_id", "利用者の識別子"),
    ("created_at", "作った時刻（UTC）"),
];

/// 場所の器（ST21）。座標も名前も持たない —— 場所の名前と座標は場所の記録（`events.jsonl` の `s01-place`）にある。
const PLACE_COLUMNS: &[(&str, &str)] = &[
    ("id", "場所の識別子（場所の記録が指す）"),
    ("seq", "作った順の番号"),
    ("user_id", "利用者の識別子"),
    ("created_at", "作った時刻（UTC）"),
];

const ATTRIBUTE_KIND_NAME_COLUMNS: &[(&str, &str)] = &[
    ("id", "名前の識別子"),
    ("kind_id", "属性の種類の識別子"),
    ("user_id", "利用者の識別子"),
    ("name", "属性の名前"),
    ("created_at", "付けた時刻（UTC）"),
];

const STAY_CRITERIA_COLUMNS: &[(&str, &str)] = &[
    ("id", "滞在の基準の識別子"),
    ("user_id", "利用者の識別子"),
    ("radius_m", "同じ場所とみなす半径（m）"),
    ("min_minutes", "滞在とみなす最短の時間（分）"),
    ("gap_minutes", "同じ滞在とつなぐ最大の空き（分）"),
    ("sources", "滞在の材料にするソース"),
    ("created_at", "基準を決めた時刻（UTC）"),
];

const HEARTBEAT_COLUMNS: &[(&str, &str)] = &[
    ("id", "稼働の信号の識別子"),
    ("user_id", "利用者の識別子"),
    ("logical_source", "ソースの名前"),
    ("device_id", "信号を出した端末"),
    ("emitted_at", "収集側が信号を作った時刻（UTC）"),
    ("received_at", "受け取った時刻（UTC）"),
    ("capturable", "取得できる状態だったか"),
    ("blockers", "満たされていなかったもの"),
];

const COVERAGE_SPAN_COLUMNS: &[(&str, &str)] = &[
    ("id", "区間の識別子"),
    ("user_id", "利用者の識別子"),
    ("logical_source", "ソースの名前"),
    ("kind", "区間の種類"),
    ("started_at", "始まりの時刻（UTC）"),
    ("ended_at", "終わりの時刻（UTC。空なら続いている）"),
    ("event_count", "破棄した件数"),
    ("note", "注記"),
];

const DROP_REPORT_COLUMNS: &[(&str, &str)] = &[
    ("id", "破棄の報告の識別子"),
    ("user_id", "利用者の識別子"),
    ("logical_source", "ソースの名前"),
    ("device_id", "捨てた端末"),
    ("reason", "捨てた理由"),
    ("range_start", "捨てた記録の最初の時刻（UTC）"),
    ("range_end", "捨てた記録の終わりの時刻（UTC。含まない）"),
    ("count", "捨てた件数"),
    ("created_at", "端末が報告を作った時刻（UTC）"),
    ("received_at", "受け取った時刻（UTC）"),
    ("content_hash", "内容のハッシュ"),
    ("raw", "報告の原文（逐語の文字列）"),
];

const DROP_REPORT_HOUR_COLUMNS: &[(&str, &str)] = &[
    ("report_id", "破棄の報告の識別子"),
    ("hour", "時（UTC）"),
    ("count", "その時に捨てた件数"),
];

/// 書き出しの分類の正本（D8）。core の**すべての表**を `Include` か `Exclude` に分ける。
/// 後から表や列を足す Story は、ここに足さないと書き出しのテストが落ちる（黙って入る・落ちるを止める）。
pub const CATALOG: &[(&str, Class)] = &[
    (
        "event",
        Class::Include {
            file: "events.jsonl",
            view: "event_live",
            excluded_columns: &[],
            columns: EVENT_COLUMNS,
        },
    ),
    (
        "event_version",
        Class::Include {
            file: "event_versions.jsonl",
            view: "event_version_live",
            excluded_columns: &["txid"],
            columns: EVENT_VERSION_COLUMNS,
        },
    ),
    (
        "source",
        Class::Include {
            file: "sources.jsonl",
            view: "source",
            excluded_columns: &[],
            columns: SOURCE_COLUMNS,
        },
    ),
    (
        "attribute_kind",
        Class::Include {
            file: "attribute_kinds.jsonl",
            view: "attribute_kind",
            excluded_columns: &[],
            columns: ATTRIBUTE_KIND_COLUMNS,
        },
    ),
    (
        "attribute_kind_name",
        Class::Include {
            file: "attribute_kind_names.jsonl",
            view: "attribute_kind_name",
            excluded_columns: &[],
            columns: ATTRIBUTE_KIND_NAME_COLUMNS,
        },
    ),
    (
        "place",
        Class::Include {
            file: "places.jsonl",
            view: "place",
            excluded_columns: &[],
            columns: PLACE_COLUMNS,
        },
    ),
    (
        "stay_criteria",
        Class::Include {
            file: "stay_criteria.jsonl",
            view: "stay_criteria",
            excluded_columns: &[],
            columns: STAY_CRITERIA_COLUMNS,
        },
    ),
    (
        "heartbeat",
        Class::Include {
            file: "heartbeats.jsonl",
            view: "heartbeat",
            excluded_columns: &["attempts", "successes", "raw", "content_hash"],
            columns: HEARTBEAT_COLUMNS,
        },
    ),
    (
        "coverage_span",
        Class::Include {
            file: "coverage_spans.jsonl",
            view: "coverage_span",
            excluded_columns: &[],
            columns: COVERAGE_SPAN_COLUMNS,
        },
    ),
    (
        "drop_report",
        Class::Include {
            file: "drop_reports.jsonl",
            view: "drop_report",
            excluded_columns: &[],
            columns: DROP_REPORT_COLUMNS,
        },
    ),
    (
        "drop_report_hour",
        Class::Include {
            file: "drop_report_hours.jsonl",
            view: "drop_report_hour",
            excluded_columns: &[],
            columns: DROP_REPORT_HOUR_COLUMNS,
        },
    ),
    (
        "coverage",
        Class::Exclude {
            reason: "稼働状況の導出（削除済みの記録も数えているため）",
        },
    ),
    (
        "erasure_ledger",
        Class::Exclude {
            reason: "消したことの台帳（痕跡を残さない）",
        },
    ),
    (
        "deletion_ledger",
        Class::Exclude {
            reason: "消したことの台帳（痕跡を残さない）",
        },
    ),
    (
        "export_ledger",
        Class::Exclude {
            reason: "書き出しの台帳（内部の記録）",
        },
    ),
    (
        "access_log",
        Class::Exclude {
            reason: "読み出しの記録（内部の記録）",
        },
    ),
    (
        "web_session",
        Class::Exclude {
            reason: "画面のログインの状態（内部の記録）",
        },
    ),
    (
        "stay_absorbed",
        Class::Exclude {
            reason: "滞在の導出の内部の表",
        },
    ),
    ("archive_ledger", ARCHIVE_INTERNAL),
    ("archive_ledger_source", ARCHIVE_INTERNAL),
    ("archive_file", ARCHIVE_INTERNAL),
    ("archive_shape_confirmation", ARCHIVE_INTERNAL),
    ("archive_sighting", ARCHIVE_INTERNAL),
    ("archive_scan_counter", ARCHIVE_INTERNAL),
    ("archive_pending_shape", ARCHIVE_INTERNAL),
    ("archive_reread_failure", ARCHIVE_INTERNAL),
];

/// 書庫の取り込みの内部の表。書庫から入れた記録は `event` にあり、写しのファイルは DB の外にある（C6）。
const ARCHIVE_INTERNAL: Class = Class::Exclude {
    reason: "書庫の取り込みの内部の表（入れた記録は event にある）",
};

/// 表の分類を引く。
pub fn class_of(table: &str) -> Option<&'static Class> {
    CATALOG.iter().find(|(n, _)| *n == table).map(|(_, c)| c)
}

// ---------------------------------------------------------------------------
// 一覧（information_schema）と突き合わせ（D8 / D9）
// ---------------------------------------------------------------------------

/// `core` の表と列の一覧。分類の段に渡す差し替え口の対象。
/// `columns` は表とビューの両方の `(名前, 列)`（ビューの列は D9 の突き合わせに使う）。
#[derive(Debug, Clone, Default)]
pub struct Inventory {
    /// BASE TABLE の名前。
    pub tables: Vec<String>,
    pub columns: Vec<(String, String)>,
}

/// 分類の無いもの。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unclassified {
    Table(String),
    Column { table: String, column: String },
}

impl std::fmt::Display for Unclassified {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Table(t) => write!(f, "表 {t}"),
            Self::Column { table, column } => write!(f, "列 {table}.{column}"),
        }
    }
}

/// `core` の表と列を読む。分類と突き合わせの両方が同じ一覧を使う（同じトランザクションで読む）。
pub async fn read_inventory(pool: &sqlx::PgPool) -> Result<Inventory, ExportError> {
    let mut conn = pool
        .acquire()
        .await
        .map_err(|e| ExportError::new(FailKind::Db, format!("一覧を読めない: {e}")))?;
    read_inventory_in(&mut conn).await
}

/// 渡された接続（写しのトランザクションなど）で読む。
async fn read_inventory_in(conn: &mut sqlx::PgConnection) -> Result<Inventory, ExportError> {
    let db = |e: sqlx::Error| ExportError::new(FailKind::Db, format!("一覧を読めない: {e}"));
    let tables: Vec<(String,)> = sqlx::query_as(
        "SELECT table_name::text FROM information_schema.tables
          WHERE table_schema = 'core' AND table_type = 'BASE TABLE'
          ORDER BY table_name",
    )
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    let columns: Vec<(String, String)> = sqlx::query_as(
        "SELECT table_name::text, column_name::text FROM information_schema.columns
          WHERE table_schema = 'core'
          ORDER BY table_name, ordinal_position",
    )
    .fetch_all(&mut *conn)
    .await
    .map_err(db)?;
    Ok(Inventory {
        tables: tables.into_iter().map(|(t,)| t).collect(),
        columns,
    })
}

/// 分類の無い表と、`Include` の表で出すとも出さないとも決まっていない列を返す。
pub fn unclassified(inv: &Inventory) -> Vec<Unclassified> {
    let mut out: Vec<Unclassified> = inv
        .tables
        .iter()
        .filter(|t| class_of(t).is_none())
        .map(|t| Unclassified::Table(t.clone()))
        .collect();
    for (table, column) in &inv.columns {
        if let Some(Class::Include {
            excluded_columns,
            columns,
            ..
        }) = class_of(table)
        {
            let known = columns.iter().any(|(n, _)| n == column)
                || excluded_columns.contains(&column.as_str());
            if !known {
                out.push(Unclassified::Column {
                    table: table.clone(),
                    column: column.clone(),
                });
            }
        }
    }
    out
}

/// 出すと決めた列のうち、読み出し（`view`）に無いもの `(view, column)`（D9）。
pub fn missing_view_columns(inv: &Inventory) -> Vec<(String, String)> {
    let have: BTreeSet<(&str, &str)> = inv
        .columns
        .iter()
        .map(|(t, c)| (t.as_str(), c.as_str()))
        .collect();
    let mut out = Vec::new();
    for (_, class) in CATALOG {
        if let Class::Include { view, columns, .. } = class {
            for (name, _) in *columns {
                if !have.contains(&(*view, *name)) {
                    out.push(((*view).to_string(), (*name).to_string()));
                }
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// 設定と差し替え口（D2）
// ---------------------------------------------------------------------------

/// 本体の段。後の Task が段を足す。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// 台帳の始まりの行を書く（D10。`POST /export` の求めの中。R22）。
    Start,
    /// 表と列の分類（D8）。
    Catalog,
    /// 列の突き合わせ（D9）。
    Columns,
    /// zip を作って `rename` する。
    Write,
    /// zip の項目（ファイル）を 1 つ書き始める。項目ごとに `fail` だけを呼ぶ（`around` は呼ばない）。
    /// 写しを流している途中（`.partial-…` に書きかけ）の失敗を差し込むのに使う。
    Item,
    /// 書き終えたファイルの大きさと SHA-256 を取る（台帳の終わりの行の `bytes` / `digest`。D16）。
    Measure,
    /// 遅れて届いた位置の印（D6）。写しの前。
    LateMark,
    /// 台帳の終わりの行を書く（D10）。
    Finish,
    /// 前の書き出しのファイルを消す（D16）。
    DeletePrevious,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Before,
    After,
}

/// 段の前後で呼ぶ関数が返す future。
pub type HookFuture = Pin<Box<dyn Future<Output = ()> + Send>>;

type InventoryHook = Arc<dyn Fn(&mut Inventory) + Send + Sync>;
type AroundHook = Arc<dyn Fn(Stage, Phase) -> HookFuture + Send + Sync>;
type FailHook = Arc<dyn Fn(Stage) -> Option<ExportError> + Send + Sync>;
type FileHook = Arc<dyn Fn(&'static str) -> HookFuture + Send + Sync>;
type HaltHook = Arc<dyn Fn(Stage) -> bool + Send + Sync>;

/// 段ごとの差し替え口（`StayRebuilder::from_fn` と同じ考え方）。
/// 表の権限を剥がす・名前を変えるやり方で失敗させない（共有の開発 DB を壊すため）。
#[derive(Clone)]
pub struct Hooks {
    inventory: InventoryHook,
    around: AroundHook,
    fail: FailHook,
    before_file: FileHook,
    halt: HaltHook,
}

impl Hooks {
    /// 何もしない（本番）。
    pub fn none() -> Self {
        Self {
            inventory: Arc::new(|_| {}),
            around: Arc::new(|_, _| Box::pin(async {})),
            fail: Arc::new(|_| None),
            before_file: Arc::new(|_| Box::pin(async {})),
            halt: Arc::new(|_| false),
        }
    }

    /// 分類の段に渡す表と列の一覧を書き換える。
    #[must_use]
    pub fn with_inventory(self, f: impl Fn(&mut Inventory) + Send + Sync + 'static) -> Self {
        Self {
            inventory: Arc::new(f),
            ..self
        }
    }

    /// 段の前後で呼ぶ。
    #[must_use]
    pub fn with_around(
        self,
        f: impl Fn(Stage, Phase) -> HookFuture + Send + Sync + 'static,
    ) -> Self {
        Self {
            around: Arc::new(f),
            ..self
        }
    }

    /// 段の前に失敗を差し込む。
    #[must_use]
    pub fn with_fail(
        self,
        f: impl Fn(Stage) -> Option<ExportError> + Send + Sync + 'static,
    ) -> Self {
        Self {
            fail: Arc::new(f),
            ..self
        }
    }
}

impl Hooks {
    /// 写しを読み始めた後、zip の項目（ファイル）を 1 つ書き始める前に呼ぶ。引数はその項目の名前。
    /// 「書く途中」で止める・記録を取り込む・滞在を消すのに使う。
    #[must_use]
    pub fn with_before_file(
        self,
        f: impl Fn(&'static str) -> HookFuture + Send + Sync + 'static,
    ) -> Self {
        Self {
            before_file: Arc::new(f),
            ..self
        }
    }
}

impl Hooks {
    /// その段の前でサーバが止まった状態を作る（台帳に何も書かずに本体を打ち切る）。
    /// 台帳の終わりの行を書く前に止まった書き出しを、新しい `App` で読むために使う。
    #[must_use]
    pub fn with_halt(self, f: impl Fn(Stage) -> bool + Send + Sync + 'static) -> Self {
        Self {
            halt: Arc::new(f),
            ..self
        }
    }
}

impl std::fmt::Debug for Hooks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Hooks")
    }
}

/// 本体が受ける設定。
#[derive(Debug, Clone)]
pub struct ExportConfig {
    /// 置き場。決まらなければ `None`（書き出しを押したときに `io` で失敗する。起動は止めない）。
    pub dir: Option<PathBuf>,
    /// 自己試験の遅延。zip の中のファイル 1 つを書くごとに待つ（e2e が「書き出し中」を観測するため）。
    pub delay: Duration,
    pub hooks: Hooks,
    /// 進み具合（書く行の数と書いた行の数）。本体が更新し、読む側が持つ。
    pub progress: Arc<Progress>,
}

/// 進み具合。`total` は写しの時点で数えた書く行の数（D4）、`written` は書いた行の数。
#[derive(Debug, Default)]
pub struct Progress {
    pub total: AtomicU64,
    pub written: AtomicU64,
}

impl ExportConfig {
    /// 環境から組む（`ASHIATO_EXPORT_DIR` ほか・`ASHIATO_EXPORT_SELFTEST_DELAY_MS`）。
    pub fn from_env() -> Self {
        let delay_ms = std::env::var("ASHIATO_EXPORT_SELFTEST_DELAY_MS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);
        Self {
            dir: export_dir(|k| std::env::var(k).ok()),
            delay: Duration::from_millis(delay_ms),
            hooks: Hooks::none(),
            progress: Arc::default(),
        }
    }

    /// 置き場を指す（試験・差し替え）。
    pub fn with_dir(dir: PathBuf) -> Self {
        Self {
            dir: Some(dir),
            delay: Duration::ZERO,
            hooks: Hooks::none(),
            progress: Arc::default(),
        }
    }

    /// 置き場が決まらない設定。
    pub fn without_dir() -> Self {
        Self {
            dir: None,
            delay: Duration::ZERO,
            hooks: Hooks::none(),
            progress: Arc::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// 本体
// ---------------------------------------------------------------------------

/// 段を 1 つ走らせる。前に呼ぶ関数 → 失敗の差し込み → 本体 → 後に呼ぶ関数。
async fn stage<T>(
    cfg: &ExportConfig,
    s: Stage,
    body: impl Future<Output = Result<T, ExportError>>,
) -> Result<T, ExportError> {
    (cfg.hooks.around)(s, Phase::Before).await;
    if let Some(e) = (cfg.hooks.fail)(s) {
        return Err(e);
    }
    let v = body.await?;
    (cfg.hooks.around)(s, Phase::After).await;
    Ok(v)
}

/// 書き終えた書き出し。
#[derive(Debug, Clone)]
pub struct Written {
    /// 書き終えたファイル（置き場の下の絶対のパス）。
    pub file: PathBuf,
    pub export_id: uuid::Uuid,
    pub dumped: Dumped,
}

/// 書き出しの本体。成功すれば書き終えたファイルの絶対のパスを返す。
pub async fn run(
    pool: &sqlx::PgPool,
    cfg: &ExportConfig,
    passphrase: Passphrase,
) -> Result<PathBuf, ExportError> {
    run_as(pool, cfg, passphrase, uuid::Uuid::new_v4())
        .await
        .map(|w| w.file)
}

/// 書き出しの識別子を呼び出し側が決める本体（注記 `ashiato-export <export_id>` になる）。
///
/// 分類 → 列の突き合わせ → 写しを zip に暗号化して流す → `rename`。合言葉は項目の暗号化にだけ使う。
/// 失敗したら `.partial-…` を残さない。
pub async fn run_as(
    pool: &sqlx::PgPool,
    cfg: &ExportConfig,
    passphrase: Passphrase,
    export_id: uuid::Uuid,
) -> Result<Written, ExportError> {
    run_named(pool, cfg, passphrase, export_id, &name_stamp()).await
}

/// 名前の日時（`YYYYMMDD-HHMMSS`。Asia/Tokyo）。Asia/Tokyo は夏時間の無い +09:00 なので、9 時間足して字面を取れば足りる。
pub(crate) fn name_stamp() -> String {
    (chrono::Utc::now() + chrono::Duration::hours(9))
        .format("%Y%m%d-%H%M%S")
        .to_string()
}

/// 名前の日時を呼び出し側が決める本体（台帳の始まりの行に書いた名前と、書き出すファイルの名前を揃えるため）。
pub(crate) async fn run_named(
    pool: &sqlx::PgPool,
    cfg: &ExportConfig,
    passphrase: Passphrase,
    export_id: uuid::Uuid,
    stamp: &str,
) -> Result<Written, ExportError> {
    // 分類・列の突き合わせ・写しは同じトランザクションの中で行う（D4 / D9）。
    let mut tx = dump::begin(pool).await?;
    let inv = stage(cfg, Stage::Catalog, async {
        let mut inv = read_inventory_in(&mut tx).await?;
        (cfg.hooks.inventory)(&mut inv);
        let bad = unclassified(&inv);
        if bad.is_empty() {
            Ok(inv)
        } else {
            let names: Vec<String> = bad.iter().map(ToString::to_string).collect();
            Err(ExportError::new(
                FailKind::UnclassifiedTable,
                format!("分類の無いもの: {}", names.join(", ")),
            ))
        }
    })
    .await?;

    stage(cfg, Stage::Columns, async {
        let missing = missing_view_columns(&inv);
        if missing.is_empty() {
            Ok(())
        } else {
            let names: Vec<String> = missing.iter().map(|(v, c)| format!("{v}.{c}")).collect();
            Err(ExportError::new(
                FailKind::MissingColumn,
                format!("読み出しに無い列: {}", names.join(", ")),
            ))
        }
    })
    .await?;

    stage(cfg, Stage::Write, async {
        let dir = cfg
            .dir
            .as_deref()
            .ok_or_else(|| ExportError::new(FailKind::Io, "置き場が決まらない"))?;
        write_zip(tx, cfg, &passphrase, export_id, dir, stamp).await
    })
    .await
}

/// 置き場を（無ければ 0700 で）作り、`.partial-….zip`（0600）に書いて閉じ、本来の名前へ `rename` する。
/// `.` で始まる名前は書き終えるまで。失敗したら消す。
async fn write_zip(
    tx: dump::Tx,
    cfg: &ExportConfig,
    passphrase: &Passphrase,
    export_id: uuid::Uuid,
    dir: &Path,
    stamp: &str,
) -> Result<Written, ExportError> {
    ensure_dir(dir)?;
    let partial = dir.join(format!(
        ".partial-ashiato-export-{stamp}-{}.zip",
        &uuid::Uuid::new_v4().simple().to_string()[..8]
    ));
    let file = create_private_file(&partial)?;
    let result = async {
        let (file, dumped) = dump::dump(tx, cfg, passphrase, &export_id.to_string(), file).await?;
        file.sync_all()
            .map_err(|e| ExportError::io("同期できない", &e))?;
        drop(file);
        // rename は同じ名前を置き換えるので、直前に空きを確かめ直す。
        let target = free_name(dir, stamp);
        std::fs::rename(&partial, &target)
            .map_err(|e| ExportError::io("名前を付けられない", &e))?;
        Ok(Written {
            file: target,
            export_id,
            dumped,
        })
    }
    .await;
    if result.is_err() {
        let _ = std::fs::remove_file(&partial);
    }
    result
}

/// まだ無い名前（`ashiato-export-<stamp>.zip`、あれば `-2`, `-3` …）。
pub fn free_name(dir: &Path, stamp: &str) -> PathBuf {
    let first = dir.join(format!("ashiato-export-{stamp}.zip"));
    if !first.exists() {
        return first;
    }
    let mut n = 2u32;
    loop {
        let p = dir.join(format!("ashiato-export-{stamp}-{n}.zip"));
        if !p.exists() {
            return p;
        }
        n += 1;
    }
}

/// 置き場が無ければ作る（Unix は 0700）。既にあるものの権限は変えない（本人の持ち物）。
fn ensure_dir(dir: &Path) -> Result<(), ExportError> {
    let mut b = std::fs::DirBuilder::new();
    b.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        b.mode(0o700);
    }
    b.create(dir)
        .map_err(|e| ExportError::io("置き場を作れない", &e))
}

/// 持ち主だけが読み書きできる新しいファイルを作る（Unix は 0600）。
fn create_private_file(path: &Path) -> Result<std::fs::File, ExportError> {
    let mut o = std::fs::OpenOptions::new();
    o.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        o.mode(0o600);
    }
    o.open(path)
        .map_err(|e| ExportError::io("ファイルを作れない", &e))
}
