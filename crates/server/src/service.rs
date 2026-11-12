//! Windows サービスの起動口と、サービス起動前に共有する純粋な待機処理。

use std::collections::BTreeMap;
use std::future::Future;
use std::io::Write as _;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::Notify;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DbServiceState {
    Stopped,
    Running,
    StartPending,
    StopPending,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DbWatchDecision {
    Restart,
    Ignore,
    Log(String),
}

/// サーバのサービスが DB のサービスの状態を読む間隔（D35 の「5 秒ごと」）。
pub const DB_WATCH_INTERVAL: Duration = Duration::from_secs(5);

pub fn decide_db_watch(
    server_running: bool,
    db_state: Result<DbServiceState, String>,
) -> DbWatchDecision {
    if !server_running {
        return DbWatchDecision::Ignore;
    }
    match db_state {
        Ok(DbServiceState::Stopped) => DbWatchDecision::Restart,
        Ok(_) => DbWatchDecision::Ignore,
        Err(reason) => DbWatchDecision::Log(reason),
    }
}

/// 見張る DB のサービスの名前。インストーラが `server.json` の `ASHIATO_DB_SERVICE` に書く
/// （配る形は `ashiato-db`、開発版は `ashiato-db-dev`。review R66）。鍵が無ければ配る形の名前にする。
pub fn configured_db_service_name(value: Option<&str>) -> &str {
    value
        .filter(|name| !name.is_empty())
        .unwrap_or("ashiato-db")
}

pub fn should_log_db_watch_error(previous: Option<&str>, current: &str) -> bool {
    previous != Some(current)
}

/// 見張りの失敗の理由を、変わったときだけ書くための控え。うまくいったら忘れるので、
/// 回復の後に同じ理由が再発したらまた書く（review R65）。
#[derive(Debug, Default)]
pub struct DbWatchErrorLog {
    last: Option<String>,
}

impl DbWatchErrorLog {
    /// 失敗の理由を控え、書くべき（前と違う理由）なら `true` を返す。
    pub fn failed(&mut self, reason: &str) -> bool {
        let log = should_log_db_watch_error(self.last.as_deref(), reason);
        self.last = Some(reason.to_owned());
        log
    }

    /// うまくいった。控えた理由を忘れる。
    pub fn succeeded(&mut self) {
        self.last = None;
    }
}

#[cfg(windows)]
fn db_service_state(code: u32) -> DbServiceState {
    match code {
        1 => DbServiceState::Stopped,
        4 => DbServiceState::Running,
        2 => DbServiceState::StartPending,
        3 => DbServiceState::StopPending,
        _ => DbServiceState::Other,
    }
}

#[cfg(windows)]
async fn watch_db_service() {
    let mut interval = tokio::time::interval(DB_WATCH_INTERVAL);
    let service_name =
        configured_db_service_name(std::env::var("ASHIATO_DB_SERVICE").ok().as_deref()).to_owned();
    // 同じ理由を 5 秒ごとに書かない。うまくいったら忘れ、回復の後に同じ理由が再発したらまた書く（review R65）
    let mut query_errors = DbWatchErrorLog::default();
    let mut start_errors = DbWatchErrorLog::default();
    loop {
        tokio::select! {
            () = STOP.wait() => return,
            _ = interval.tick() => {
                let service_for_query = service_name.clone();
                let state = tokio::task::spawn_blocking(move || {
                    ashiato_winsvc::query_db_service_state(&service_for_query)
                })
                    .await
                    .map_err(|error| error.to_string())
                    .and_then(|result| result)
                    .map(db_service_state);
                match decide_db_watch(true, state) {
                    DbWatchDecision::Restart => {
                        query_errors.succeeded();
                        // StartService は SCM の答えを待って塞ぐので、async の糸の外で呼ぶ（review R64）
                        let service_for_start = service_name.clone();
                        let started = tokio::task::spawn_blocking(move || {
                            ashiato_winsvc::start_db_service(&service_for_start)
                        })
                            .await
                            .map_err(|error| error.to_string())
                            .and_then(|result| result);
                        match started {
                            Ok(()) => {
                                start_errors.succeeded();
                                tracing::warn!(kind = "db_service_restarted", "DB のサービスを起こし直した");
                            }
                            Err(reason) => {
                                if start_errors.failed(&reason) {
                                    tracing::error!(kind = "db_service_start_failed", %reason, "DB のサービスを起こし直せない");
                                }
                            }
                        }
                    }
                    DbWatchDecision::Log(reason) => {
                        if query_errors.failed(&reason) {
                            tracing::error!(kind = "db_service_query_failed", %reason);
                        }
                    }
                    DbWatchDecision::Ignore => {
                        query_errors.succeeded();
                        start_errors.succeeded();
                    }
                }
            }
        }
    }
}

/// 停止の知らせ。知らせが待つ者より先に来ても取りこぼさない（旗を立ててから起こす）。
#[derive(Debug, Default)]
pub struct StopSignal {
    stopped: AtomicBool,
    notify: Notify,
}

impl StopSignal {
    pub const fn new() -> Self {
        Self {
            stopped: AtomicBool::new(false),
            notify: Notify::const_new(),
        }
    }

    /// 停止を知らせる。待っている者を全部起こし、後から待つ者もすぐ返る。
    pub fn request(&self) {
        self.stopped.store(true, Ordering::SeqCst);
        self.notify.notify_waiters();
    }

    pub fn is_requested(&self) -> bool {
        self.stopped.load(Ordering::SeqCst)
    }

    /// 停止の知らせを待つ。既に知らせがあればすぐ返る。
    pub async fn wait(&self) {
        loop {
            let notified = self.notify.notified();
            tokio::pin!(notified);
            // 旗を見る前に待ちを登録しておき、間に来た notify_waiters を取りこぼさない。
            notified.as_mut().enable();
            if self.is_requested() {
                return;
            }
            notified.await;
        }
    }
}

static STOP: StopSignal = StopSignal::new();

/// サービス設定の読み込みに失敗した理由。値やパスは含めない。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsError {
    /// ファイルが無い。
    Missing,
    /// ファイルを開けない。
    Unreadable,
    /// JSON または設定の形が違う。
    Malformed,
}

impl std::fmt::Display for SettingsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let reason = match self {
            Self::Missing => "settings_missing",
            Self::Unreadable => "settings_unreadable",
            Self::Malformed => "settings_malformed",
        };
        f.write_str(reason)
    }
}

impl std::error::Error for SettingsError {}

/// JSON 設定ファイルの値。秘密はこの型の Debug/Display に現れない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings(pub BTreeMap<String, String>);

/// `server.json` を読み、文字列のキーと値だけを受け付ける。
pub fn load_settings(path: &Path) -> Result<Settings, SettingsError> {
    let bytes = std::fs::read(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            SettingsError::Missing
        } else {
            SettingsError::Unreadable
        }
    })?;
    let value: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|_| SettingsError::Malformed)?;
    let object = value.as_object().ok_or(SettingsError::Malformed)?;
    let mut values = BTreeMap::new();
    for (key, value) in object {
        let value = value.as_str().ok_or(SettingsError::Malformed)?;
        values.insert(key.clone(), value.to_owned());
    }
    Ok(Settings(values))
}

/// 設定をこのプロセスの環境へ反映する。親プロセスやログへ値を返さない。
pub fn apply_settings(settings: &Settings) {
    for (key, value) in &settings.0 {
        std::env::set_var(key, value);
    }
}

/// インストール版のサービスが設定ファイルの置き場を基準に使う既定の状態置き場。
/// 設定や環境で明示された値は呼び出し側で優先できるよう、純粋な計算として公開する。
pub fn installed_default_directories(settings_path: &Path) -> [std::path::PathBuf; 3] {
    let state = settings_path
        .parent()
        .and_then(Path::parent)
        .unwrap_or_else(|| Path::new("."));
    [
        state.join("export"),
        state.join("media"),
        state.join("archive-copies"),
    ]
}

/// サービスの口では環境へ入れない鍵。表の所有者の合言葉（`DATABASE_OWNER_URL`）は移行（インストーラが
/// 起こす `migrate`）だけが使い、サーバの起動は読まない（lib.rs の `run_migrate` だけ）。インストーラは
/// それを `server.json` でなく管理者と SYSTEM だけが読める `owner.json` に置く（design D4。deep.md Q11）。
/// 手で `server.json` に書かれても、サーバのプロセスに持たせない（review R48）。
pub const SERVICE_EXCLUDED_KEYS: [&str; 1] = ["DATABASE_OWNER_URL"];

/// サービス設定を反映し、未指定の 3 つの状態置き場だけを補う。サーバの起動で使わない秘密は入れない。
pub fn apply_settings_for_service(settings_path: &Path, settings: &Settings) {
    for (key, value) in &settings.0 {
        if !SERVICE_EXCLUDED_KEYS.contains(&key.as_str()) {
            std::env::set_var(key, value);
        }
    }
    let defaults = installed_default_directories(settings_path);
    for (key, directory) in [
        ("ASHIATO_EXPORT_DIR", defaults[0].clone()),
        ("ASHIATO_MEDIA_DIR", defaults[1].clone()),
        ("ASHIATO_ARCHIVE_COPY_DIR", defaults[2].clone()),
    ] {
        let missing = std::env::var_os(key).is_none_or(|value| value.is_empty());
        if missing {
            std::env::set_var(key, directory);
        }
    }
}

/// DB 接続の試行結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitOutcome {
    /// DB が利用可能になった。
    Ready,
    /// 制限時間を使い切った。
    TimedOut,
    /// 待っている間に停止の知らせが来た。
    Stopped,
    /// 待っても直らない失敗（合言葉の誤り・接続先の書き誤りなど）。起こし直しても同じなので待たない（review R80）。
    Rejected,
}

/// DB への接続の失敗が、待てば直るものか。
pub trait DbWaitError {
    /// 待っても直らない失敗なら `true`。既定は待つ。
    fn is_permanent(&self) -> bool {
        false
    }
}

impl DbWaitError for &str {}

impl DbWaitError for sqlx::Error {
    /// 接続の文字列の誤り、認証の失敗（28P01・28000）、DB が無い（3D000）は、DB が起きても直らない。
    /// 起動の途中の断り（57P03）や待ち受けがまだ無いのは待つ。
    fn is_permanent(&self) -> bool {
        match self {
            sqlx::Error::Configuration(_) => true,
            sqlx::Error::Database(error) => {
                matches!(error.code().as_deref(), Some("28P01" | "28000" | "3D000"))
            }
            _ => false,
        }
    }
}

/// サービスの口が DB を待つ上限（spec の 120 秒）。
pub const DB_WAIT: Duration = Duration::from_secs(120);
/// DB に繋ぎ直す間隔（spec の 1 秒）。
pub const DB_RETRY: Duration = Duration::from_secs(1);
/// サーバのログを残す日数（spec の 30 日。これより古い日付のファイルを消す）。
pub const SERVER_LOG_KEEP_DAYS: u64 = 30;

/// 更新係の置き場。サーバのサービスは環境変数ではなく、この系統の定数を使う。
pub const UPDATER_RELEASE_DIR: &str = r"C:\ProgramData\ashiato-updater";
pub const UPDATER_DEVELOPMENT_DIR: &str = r"C:\ProgramData\ashiato-updater-dev";

pub fn installed_updater_directory() -> std::path::PathBuf {
    let directory = if env!("ASHIATO_VERSION").contains("-dev.") {
        UPDATER_DEVELOPMENT_DIR
    } else {
        UPDATER_RELEASE_DIR
    };
    std::path::PathBuf::from(directory)
}

/// DB を 1 秒ごとに試し、指定時間を超えたら値を返す。
/// 待っても直らない失敗（`DbWaitError::is_permanent`）はすぐ `Rejected` を返す。
pub async fn wait_for_db<F, Fut, E>(timeout: Duration, mut connect: F) -> WaitOutcome
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<(), E>>,
    E: DbWaitError,
{
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        match connect().await {
            Ok(()) => return WaitOutcome::Ready,
            Err(error) if error.is_permanent() => return WaitOutcome::Rejected,
            Err(_) => {}
        }
        let Some(remaining) = deadline.checked_duration_since(tokio::time::Instant::now()) else {
            return WaitOutcome::TimedOut;
        };
        if remaining.is_zero() {
            return WaitOutcome::TimedOut;
        }
        tokio::time::sleep(DB_RETRY.min(remaining)).await;
    }
}

/// DB を待つ間も停止の知らせを受ける。知らせが来れば待ちをやめて `Stopped` を返す。
pub async fn wait_for_db_until_stopped<F, Fut, E>(
    timeout: Duration,
    connect: F,
    stop: &StopSignal,
) -> WaitOutcome
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<(), E>>,
    E: DbWaitError,
{
    tokio::select! {
        outcome = wait_for_db(timeout, connect) => outcome,
        () = stop.wait() => WaitOutcome::Stopped,
    }
}

/// `service --settings <場所>` の口の準備（tokio の起動前・1 本のスレッドの間に呼ぶ）。設定のファイルの場所を
/// `ASHIATO_SETTINGS_PATH` に置き、設定を読んで環境へ入れ、未指定の置き場（書き出し・原本・書庫の写し）を
/// 状態の置き場の下に補う。読めなければ理由をサーバのログに書いて返す（main.rs の Windows の口が呼ぶ。review R43）。
pub fn prepare_service(path: &Path) -> Result<(), SettingsError> {
    std::env::set_var("ASHIATO_SETTINGS_PATH", path);
    match load_settings(path) {
        Ok(settings) => {
            apply_settings_for_service(path, &settings);
            Ok(())
        }
        Err(reason) => {
            let _ = log_settings_refusal(path, reason);
            Err(reason)
        }
    }
}

/// 起動拒否の種別を、設定ファイルと同じ状態置き場のサーバログへ書く。
pub fn log_settings_refusal(path: &Path, error: SettingsError) -> std::io::Result<()> {
    let state = path
        .parent()
        .and_then(Path::parent)
        .unwrap_or_else(|| Path::new("."));
    let directory = state.join("logs").join("server");
    let today = log_day(chrono::Utc::now());
    let mut log = ServerLog::new(directory, today)?;
    log.write_at(today, &error.to_string())
}

/// サービス起動前の拒否を、端末と同じ理由でサーバログにも残す。
/// 書くのは `service` の口（設定のファイルの場所がある）ときだけ。開発の形では何も書かない（D17）。
pub fn log_startup_refusal(kind: &str) {
    let Some(directory) = std::env::var_os("ASHIATO_SETTINGS_PATH").and_then(|path| {
        Path::new(&path)
            .parent()
            .and_then(Path::parent)
            .map(|state| state.join("logs").join("server"))
    }) else {
        return;
    };
    let today = log_day(chrono::Utc::now());
    match ServerLog::new(directory, today)
        .and_then(|mut log| log.write_at(today, &format!("kind={kind}")))
    {
        Ok(()) => {}
        Err(_) => tracing::error!(
            kind,
            reason = "startup_refusal_log_unavailable",
            "起動を拒否した理由を書けない"
        ),
    }
}

/// Windows のサービス制御から停止を知らせる。書きかけのファイルの後片付けは、書き出しを止めた後に
/// `run_inner` が 1 回だけ行う（ここで消すと、まだ走っている書き出しのファイルを消してしまう。review R54）。
pub fn request_stop() {
    STOP.request();
}

pub async fn wait_for_stop() {
    STOP.wait().await;
}

/// 試験の組み立てだけが読む差し込み `ASHIATO_SERVICE_TEST_DB_DELAY_SEC`（D24）。
/// 秒の数ならその間 DB に繋がらない形にし、`never` なら最後まで繋がらない形にする。
#[cfg(ashiato_test_build)]
fn test_db_delay() -> Option<Duration> {
    match std::env::var("ASHIATO_SERVICE_TEST_DB_DELAY_SEC")
        .ok()?
        .as_str()
    {
        "never" => Some(Duration::MAX),
        value => value.parse().ok().map(Duration::from_secs),
    }
}

#[cfg(not(ashiato_test_build))]
fn test_db_delay() -> Option<Duration> {
    None
}

/// ログのファイルの日付。日の区切りは画面と同じ Asia/Tokyo（`coverage::DAY_TZ`。夏時間が無いので固定のずれ）。
/// UTC で区切ると、日本の 0 時〜9 時の行が前の日のファイルに入る（review R86）。
pub fn log_day(now: chrono::DateTime<chrono::Utc>) -> chrono::NaiveDate {
    let tokyo = chrono::FixedOffset::east_opt(9 * 3600).expect("+09:00");
    now.with_timezone(&tokyo).date_naive()
}

/// サーバの月日ごとのログを書き、日付の変わり目にファイルを開き直す書き手。
#[derive(Debug)]
pub struct ServerLog {
    directory: std::path::PathBuf,
    current: Option<(String, std::fs::File)>,
}

impl ServerLog {
    /// ログの置き場を作り、30 日より古い名前のログを片付ける。
    pub fn new(
        directory: impl Into<std::path::PathBuf>,
        today: chrono::NaiveDate,
    ) -> std::io::Result<Self> {
        let directory = directory.into();
        std::fs::create_dir_all(&directory)?;
        let log = Self {
            directory,
            current: None,
        };
        log.prune(today);
        Ok(log)
    }

    /// 30 日より古い名前のログを消す。起動のときと、日付が変わってファイルを開き直すたびに行う
    /// （止めずに動き続けるサービスでも古いログが溜まらないように。review R79）。
    /// 消せないファイル（ほかの手が開いている）は次の機会に回し、ログの書きもサービスも止めない。
    fn prune(&self, today: chrono::NaiveDate) {
        let cutoff = today - chrono::Days::new(SERVER_LOG_KEEP_DAYS);
        let Ok(entries) = std::fs::read_dir(&self.directory) else {
            return;
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            let Some(stamp) = name
                .strip_prefix("server-")
                .and_then(|s| s.strip_suffix(".log"))
            else {
                continue;
            };
            let Ok(date) = chrono::NaiveDate::parse_from_str(stamp, "%Y-%m-%d") else {
                continue;
            };
            if date < cutoff {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }

    /// 指定日の日誌へ 1 行書く。秘密の値を引数に含めない呼び出し側で使う。
    pub fn write_at(&mut self, date: chrono::NaiveDate, line: &str) -> std::io::Result<()> {
        self.write_bytes_at(date, format!("{line}\n").as_bytes())
    }

    fn write_bytes_at(&mut self, date: chrono::NaiveDate, bytes: &[u8]) -> std::io::Result<()> {
        let stamp = date.format("%Y-%m-%d").to_string();
        let needs_open = self
            .current
            .as_ref()
            .is_none_or(|(current, _)| current != &stamp);
        if needs_open {
            let file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(self.directory.join(format!("server-{stamp}.log")))?;
            let reopened = self.current.is_some();
            self.current = Some((stamp, file));
            if reopened {
                self.prune(date);
            }
        }
        self.current
            .as_mut()
            .ok_or_else(|| std::io::Error::other("log file was not opened"))?
            .1
            .write_all(bytes)
    }
}

#[derive(Debug, Clone)]
struct LogWriter(Arc<Mutex<ServerLog>>);

impl std::io::Write for LogWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let line = String::from_utf8_lossy(bytes);
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .write_bytes_at(log_day(chrono::Utc::now()), line.as_bytes())?;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// 停止時に書き途中の書き出し（`.partial-ashiato-export-*`）を残さない。
/// 消すのは書き出しの書きかけの名前だけ（置き場が本人のフォルダでも、ほかの隠しファイルに触れない）。
/// 1 つ消せなくても残りは消し、最初の失敗を返す（review R81）。
pub fn cleanup_partial_files(directory: &Path) -> std::io::Result<()> {
    if !directory.exists() {
        return Ok(());
    }
    let mut first_error = None;
    for entry in std::fs::read_dir(directory)? {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                first_error.get_or_insert(error);
                continue;
            }
        };
        let is_partial = entry
            .file_name()
            .to_str()
            .is_some_and(|name| name.starts_with(crate::export::PARTIAL_PREFIX));
        if !is_partial || !entry.file_type().is_ok_and(|kind| kind.is_file()) {
            continue;
        }
        if let Err(error) = std::fs::remove_file(entry.path()) {
            first_error.get_or_insert(error);
        }
    }
    first_error.map_or(Ok(()), Err)
}

/// 起こし直しても直らない起動の拒否（設定・合言葉・役割・DB の版・待ち受けの検査など）。
/// 理由は返す前にサーバのログへ書いてある。SCM へは終了コード 0 で「止まった」を返し、
/// `failureflag 1` の回復に起こし直させない（`service_exit_code`。review R21・R60）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StartupRefused {
    kind: &'static str,
}

impl StartupRefused {
    pub const fn new(kind: &'static str) -> Self {
        Self { kind }
    }

    pub const fn kind(self) -> &'static str {
        self.kind
    }
}

impl std::fmt::Display for StartupRefused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "startup_refused kind={}", self.kind)
    }
}

impl std::error::Error for StartupRefused {}

/// DB を 120 秒待っても繋げなかった。理由は返す前にサーバのログへ書いてある。
/// これだけは起こし直せば直りうるので、非 0 で終わって回復に任せる（spec distribution「サービスの回復が起こし直す」）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DatabaseUnavailable;

impl std::fmt::Display for DatabaseUnavailable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("database_unavailable")
    }
}

impl std::error::Error for DatabaseUnavailable {}

/// サービスの終わり方。SCM へ返す終了コードと、終わりの理由をもうサーバのログに書いたかを決める。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceEnd {
    /// 停止の知らせで終わった（または正常に終わった）。
    Stopped,
    /// 起こし直しても直らない拒否。理由は書いてある。
    Refused,
    /// DB を待ちきれなかった。理由は書いてある。回復が起こし直す。
    DatabaseUnavailable,
    /// 非同期の実行環境を作れなかった。
    RuntimeUnavailable,
    /// それ以外の失敗（待った後の DB の切断・起動後の待ち受けの失敗など）。理由はまだ書いていない。
    /// 起こし直せば直りうるので回復に任せる（`reset= 86400` で 1 日 3 回まで）。
    Failed,
}

impl ServiceEnd {
    /// `run_service` の結果を分ける。型の付いた誤り（`StartupRefused`・`DatabaseUnavailable`）で見分け、文言は読まない。
    pub fn classify(result: &anyhow::Result<()>) -> Self {
        match result {
            Ok(()) => Self::Stopped,
            Err(error) if error.downcast_ref::<StartupRefused>().is_some() => Self::Refused,
            Err(error) if error.downcast_ref::<DatabaseUnavailable>().is_some() => {
                Self::DatabaseUnavailable
            }
            Err(_) => Self::Failed,
        }
    }

    /// SCM へ返す終了コード。`failureflag 1` の回復は「止まった かつ 非 0」で動くので、
    /// 起こし直しても直らない拒否は 0 にする（review R60）。
    pub const fn exit_code(self) -> u32 {
        match self {
            Self::Stopped | Self::Refused => 0,
            Self::DatabaseUnavailable | Self::RuntimeUnavailable | Self::Failed => 1,
        }
    }

    /// 終わりの理由がまだサーバのログに無いか（`kind=service_startup` を 1 行足すか。review R67）。
    pub const fn needs_log(self) -> bool {
        matches!(self, Self::Failed)
    }
}

/// DB を待ってから通常のサーバ起動へ渡す。
pub async fn run_service() -> anyhow::Result<()> {
    std::env::set_var("ASHIATO_SERVICE_MODE", "1");
    let settings = std::env::var_os("ASHIATO_SETTINGS_PATH");
    let state = settings
        .as_deref()
        .map(Path::new)
        .and_then(Path::parent)
        .and_then(Path::parent)
        .map_or_else(
            || std::path::PathBuf::from("logs/server"),
            |p| p.join("logs/server"),
        );
    let log = Arc::new(Mutex::new(ServerLog::new(
        state,
        log_day(chrono::Utc::now()),
    )?));
    tracing_subscriber::fmt()
        .with_writer(move || LogWriter(log.clone()))
        .with_ansi(false)
        .with_env_filter("info")
        .init();
    let Ok(url) = std::env::var("DATABASE_URL") else {
        log_startup_refusal("database_url_missing");
        return Err(StartupRefused::new("database_url_missing").into());
    };
    let delay = test_db_delay();
    let started = tokio::time::Instant::now();
    let outcome = wait_for_db_until_stopped(
        DB_WAIT,
        || {
            let url = url.clone();
            async move {
                if delay.is_some_and(|delay| started.elapsed() < delay) {
                    return Err(sqlx::Error::PoolTimedOut);
                }
                sqlx::PgPool::connect(&url).await.map(|_| ())
            }
        },
        &STOP,
    )
    .await;
    match outcome {
        WaitOutcome::Ready => {
            #[cfg(windows)]
            let watcher = tokio::spawn(watch_db_service());
            let result = crate::run_inner().await;
            #[cfg(windows)]
            watcher.abort();
            result
        }
        WaitOutcome::Stopped => {
            tracing::info!(
                kind = "service_stopped",
                "DB を待つ間に停止の知らせを受けた"
            );
            Ok(())
        }
        WaitOutcome::Rejected => {
            // 起こし直しても同じ失敗なので、終了コード 0 で止まる（R60 の拒否と同じ扱い。review R80）
            tracing::error!(
                kind = "database_rejected",
                "DB が接続を断ったため起動しない"
            );
            log_startup_refusal("database_rejected");
            Err(StartupRefused::new("database_rejected").into())
        }
        WaitOutcome::TimedOut => {
            tracing::error!(kind = "database_unavailable", "DB に繋げないため起動しない");
            log_startup_refusal("database_unavailable");
            Err(DatabaseUnavailable.into())
        }
    }
}

/// Windows 以外で誤ってサービス口を使ったときの拒否。
#[cfg(not(windows))]
pub fn windows_only() -> anyhow::Result<()> {
    anyhow::bail!("Windows でだけ使える")
}
