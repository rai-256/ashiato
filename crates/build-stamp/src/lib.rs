// SPDX-License-Identifier: AGPL-3.0-only
//! 組み立ての印 `<版番号>+<commit>[.dirty][.debug]`（ST49 / design D5・D6）。S-01 と C-02 の `build.rs` が共有する。
//!
//! 依存は `std` だけ。git は `std::process::Command` で呼ぶ。
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

/// 組み立ての commit を渡す環境変数（空でなければ `git` より先に使う）。
pub const COMMIT_ENV: &str = "ASHIATO_BUILD_COMMIT";
/// 組み立てに渡す版番号。
pub const VERSION_ENV: &str = "ASHIATO_BUILD_VERSION";

/// 組み立てた版の系統。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    Dev,
    Official,
}

impl Channel {
    /// 系統を契約で使う文字列に変換する。
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Dev => "dev",
            Self::Official => "official",
        }
    }
}

/// 版番号から組み立ての系統を判定する。
pub fn channel(version: &str) -> Channel {
    if version.contains("-dev.") {
        Channel::Dev
    } else {
        Channel::Official
    }
}

/// 出所の版番号と、組み立てに渡された版番号を検査する。
pub fn version(source: &str) -> Result<String, String> {
    let Some(given) = std::env::var(VERSION_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    else {
        return Ok(source.to_string());
    };
    let source_parts = parse_version(source).ok_or_else(|| {
        format!("版番号の出所が決めた形でない: 出所 {source}、渡した版番号 {given}")
    })?;
    let given_parts = parse_version(&given)
        .ok_or_else(|| format!("版番号が決めた形でない: 出所 {source}、渡した版番号 {given}"))?;
    if source_parts.base != given_parts.base {
        return Err(format!(
            "版番号の出所が一致しない: 出所 {source}、渡した版番号 {given}"
        ));
    }
    Ok(given)
}

#[derive(Debug, PartialEq, Eq)]
struct ParsedVersion<'a> {
    base: (&'a str, &'a str, &'a str),
}

fn parse_version(value: &str) -> Option<ParsedVersion<'_>> {
    let (base, suffix) = value
        .split_once('-')
        .map_or((value, None), |(b, s)| (b, Some(s)));
    let mut numbers = base.split('.');
    let base = (numbers.next()?, numbers.next()?, numbers.next()?);
    if numbers.next().is_some()
        || base.0.is_empty()
        || base.1.is_empty()
        || base.2.is_empty()
        || !base.0.chars().all(|c| c.is_ascii_digit())
        || !base.1.chars().all(|c| c.is_ascii_digit())
        || !base.2.chars().all(|c| c.is_ascii_digit())
    {
        return None;
    }
    match suffix {
        None => {}
        Some(suffix) if suffix.starts_with("beta.") => {
            // 数字だけかを先に見る（`u8` の parse は `+5` も通す。final review R12）
            let digits = suffix.strip_prefix("beta.")?;
            if digits.is_empty()
                || !digits.chars().all(|c| c.is_ascii_digit())
                || digits.starts_with('0')
            {
                return None;
            }
            // N は 1〜98（spec。99 は正式版の versionCode の末尾。final review R8）
            let n = digits.parse::<u8>().ok()?;
            if !(1..=98).contains(&n) {
                return None;
            }
        }
        Some(suffix) if suffix.starts_with("dev.") => {
            let date = suffix.strip_prefix("dev.")?;
            if date.len() != 8 || !date.chars().all(|c| c.is_ascii_digit()) {
                return None;
            }
        }
        Some(_) => return None,
    }
    Some(ParsedVersion { base })
}

/// 印を組む。形は `<version>+<commit>[.dirty][.debug]`。
/// `.dirty` は `source_paths` の下に未 commit の変更（未追跡を含む）があるときだけ、`.debug` は `profile` が `release` でないとき。
pub fn stamp(version: &str, profile: &str, repo: &Path, source_paths: &[&str]) -> String {
    stamp_with_commit(version, profile, repo, source_paths).0
}

/// `stamp` と、印に入れた commit（40 桁・`unknown`・環境変数の値）の組。
/// commit を印の文字列から切り出さずに渡す（版番号が `+` を持っても、環境変数の値が `.dirty` で終わっても崩れない）。
pub fn stamp_with_commit(
    version: &str,
    profile: &str,
    repo: &Path,
    source_paths: &[&str],
) -> (String, String) {
    let env_commit = std::env::var(COMMIT_ENV).ok();
    let commit = commit(env_commit.as_deref(), repo);
    let dirty = if is_dirty(repo, source_paths) {
        ".dirty"
    } else {
        ""
    };
    let debug = if profile == "release" { "" } else { ".debug" };
    (format!("{version}+{commit}{dirty}{debug}"), commit)
}

/// 作り直しの印を cargo へ出す。HEAD・HEAD が指す参照・`packed-refs`・部品のソースの道・`ASHIATO_BUILD_COMMIT`。
/// `index` は出さない（review/code.md R18）: `git add` や IDE の `git status` のたびに S-01 と C-02 が組み直される。
/// `.dirty` の変化はソースの道（cargo は dir の中を全部見る）と HEAD・参照で拾える。
pub fn emit_rerun(repo: &Path, source_paths: &[&str]) {
    let mut out = std::io::stdout().lock();
    for line in rerun_lines(repo, source_paths) {
        // 出力先が壊れていても組み立ては止めない（印が古くなるだけ）
        let _ = writeln!(out, "{line}");
    }
}

fn rerun_lines(repo: &Path, source_paths: &[&str]) -> Vec<String> {
    let mut lines = vec![
        format!("cargo:rerun-if-env-changed={COMMIT_ENV}"),
        format!("cargo:rerun-if-env-changed={VERSION_ENV}"),
    ];
    // 存在しない道を出すと cargo は毎回作り直すので、あるものだけを出す
    let mut watch: Vec<PathBuf> = Vec::new();
    if let (Some(git_dir), Some(common_dir)) = (
        git_path(repo, "--git-dir"),
        git_path(repo, "--git-common-dir"),
    ) {
        watch.push(git_dir.join("HEAD"));
        if let Some(r) = git(repo, &["symbolic-ref", "-q", "HEAD"]) {
            watch.push(common_dir.join(r));
        }
        watch.push(common_dir.join("packed-refs"));
    }
    watch.extend(source_paths.iter().map(|p| repo.join(p)));
    lines.extend(
        watch
            .iter()
            .filter(|p| p.exists())
            .map(|p| format!("cargo:rerun-if-changed={}", p.display())),
    );
    lines
}

fn commit(env_commit: Option<&str>, repo: &Path) -> String {
    env_commit
        .map(str::trim)
        .filter(|c| !c.is_empty())
        .map(str::to_string)
        .or_else(|| git(repo, &["rev-parse", "HEAD"]))
        .unwrap_or_else(|| "unknown".to_string())
}

fn is_dirty(repo: &Path, source_paths: &[&str]) -> bool {
    let mut args = vec!["status", "--porcelain", "--"];
    args.extend(source_paths);
    git(repo, &args).is_some()
}

fn git_path(repo: &Path, flag: &str) -> Option<PathBuf> {
    let p = git(repo, &["rev-parse", "--path-format=absolute", flag])?;
    Some(PathBuf::from(p))
}

/// 成功して出力が空でないときだけ `Some`。`GIT_OPTIONAL_LOCKS=0` で `git status` が利用者の `index` を書き換えない。
fn git(repo: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(repo)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .output()
        .ok()
        .filter(|o| o.status.success())?;
    let text = String::from_utf8(out.stdout).ok()?;
    let text = text.trim_end_matches(['\n', '\r']);
    (!text.is_empty()).then(|| text.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::Mutex;

    /// `ASHIATO_BUILD_COMMIT` はプロセス全体の状態なので、`stamp` を呼ぶ試験は直列にする。
    static ENV: Mutex<()> = Mutex::new(());

    const RE_SHAPE: &str = "0.1.0+";

    fn git_ok(dir: &Path, args: &[&str]) {
        let st = Command::new("git")
            .args(["-c", "user.name=t", "-c", "user.email=t@t"])
            .args([
                "-c",
                "commit.gpgsign=false",
                "-c",
                "init.defaultBranch=main",
            ])
            .args(args)
            .current_dir(dir)
            .status()
            .expect("git を起こせる");
        assert!(st.success(), "git {args:?}");
    }

    /// `crates/a` と `crates/b` を持つ 1 commit の木。
    fn tree() -> (tempfile::TempDir, String) {
        let d = tempfile::tempdir().expect("tempdir");
        for p in ["crates/a", "crates/b"] {
            fs::create_dir_all(d.path().join(p)).expect("mkdir");
            fs::write(d.path().join(p).join("x.rs"), "fn main() {}\n").expect("write");
        }
        git_ok(d.path(), &["init", "-q"]);
        git_ok(d.path(), &["add", "."]);
        git_ok(d.path(), &["commit", "-q", "-m", "init"]);
        let head = git(d.path(), &["rev-parse", "HEAD"]).expect("HEAD");
        (d, head)
    }

    fn stamp_clean_env(version: &str, profile: &str, repo: &Path, paths: &[&str]) -> String {
        let _g = ENV
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        std::env::remove_var(COMMIT_ENV);
        stamp(version, profile, repo, paths)
    }

    #[test]
    fn st49_stamp_dirty_debug() {
        // Scenario: 印は版番号・commit・未 commit の印・組み立ての種類を持つ
        let (d, head) = tree();
        fs::write(d.path().join("crates/a/x.rs"), "fn main() { 1; }\n").expect("write");
        let s = stamp_clean_env("0.1.0", "debug", d.path(), &["crates/a"]);
        assert_eq!(s, format!("{RE_SHAPE}{head}.dirty.debug"));
    }

    #[test]
    fn st49_stamp_release_clean() {
        // Scenario: 綺麗な木の release の組み立ての印は、版番号と commit だけを持つ
        let (d, head) = tree();
        let s = stamp_clean_env("0.1.0", "release", d.path(), &["crates/a"]);
        assert_eq!(s, format!("{RE_SHAPE}{head}"));
    }

    #[test]
    fn st49_stamp_other_tree_is_not_dirty() {
        // Scenario: ほかの部品の木だけが変わっていても、印は .dirty を持たない
        let (d, head) = tree();
        fs::write(d.path().join("crates/b/x.rs"), "changed\n").expect("write");
        fs::write(d.path().join("crates/b/new.rs"), "new\n").expect("write");
        let s = stamp_clean_env("0.1.0", "release", d.path(), &["crates/a"]);
        assert_eq!(s, format!("{RE_SHAPE}{head}"));
    }

    #[test]
    fn st49_stamp_unknown_without_git() {
        // Scenario: commit が取れない場所での組み立ての印は unknown を持つ
        let d = tempfile::tempdir().expect("tempdir");
        let _g = ENV
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        std::env::remove_var(COMMIT_ENV);
        // `$TMPDIR` が repo の木の中にあっても、その repo を拾わない
        std::env::set_var(
            "GIT_CEILING_DIRECTORIES",
            d.path().parent().expect("parent"),
        );
        let s = stamp("0.1.0", "release", d.path(), &["crates/a"]);
        std::env::remove_var("GIT_CEILING_DIRECTORIES");
        assert_eq!(s, "0.1.0+unknown");
    }

    #[test]
    fn st49_stamp_env_commit_wins() {
        let (d, _head) = tree();
        let _g = ENV
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        std::env::set_var(COMMIT_ENV, "  abc123\n");
        let s = stamp("0.1.0", "release", d.path(), &["crates/a"]);
        std::env::set_var(COMMIT_ENV, "");
        let empty = stamp("0.1.0", "release", d.path(), &["crates/a"]);
        std::env::remove_var(COMMIT_ENV);
        assert_eq!(s, "0.1.0+abc123");
        assert!(
            empty.starts_with("0.1.0+") && empty.len() == "0.1.0+".len() + 40,
            "{empty}"
        );
    }

    #[test]
    fn st49_stamp_with_commit_does_not_parse_the_stamp() {
        // review/code.md R20: 版番号が `+` を持つ・環境変数の値が `.dirty` で終わる、でも commit は崩れない
        let (d, _head) = tree();
        let _g = ENV
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        std::env::set_var(COMMIT_ENV, "ci-1.dirty");
        let (s, c) = stamp_with_commit("0.1.0+rc1", "debug", d.path(), &["crates/a"]);
        std::env::remove_var(COMMIT_ENV);
        assert_eq!(s, "0.1.0+rc1+ci-1.dirty.debug");
        assert_eq!(c, "ci-1.dirty");
    }

    #[test]
    fn st52_version_given_version_starts_stamp_version() {
        // Scenario: 版番号を渡した組み立ての印は、渡した版番号で始まる
        let (d, head) = tree();
        let _g = ENV
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let given = "0.1.0-beta.2";
        std::env::set_var(VERSION_ENV, given);
        let resolved = version("0.1.0").expect("渡した版番号を受ける");
        let stamp = stamp(&resolved, "release", d.path(), &[]);
        std::env::remove_var(VERSION_ENV);
        assert_eq!(stamp, format!("{given}+{head}"));
    }

    #[test]
    fn st52_version_rejects_different_source_version() {
        // Scenario: 出所の版と違う版番号を渡すと、組み立てが失敗する
        let _g = ENV
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        std::env::set_var(VERSION_ENV, "0.2.0-beta.2");
        let error = version("0.1.0").expect_err("出所と違う版番号を拒む");
        std::env::remove_var(VERSION_ENV);
        assert!(error.contains("0.1.0"), "{error}");
        assert!(error.contains("0.2.0-beta.2"), "{error}");
    }

    #[test]
    fn st52_version_rejects_invalid_shapes() {
        // Scenario: 決めた形でない版番号を渡すと、組み立てが失敗する
        let _g = ENV
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        for given in [
            "0.1.0-rc.1",
            "0.1.0-beta.99",
            "0.1.0-beta.0",
            "0.1.0-dev.2026111",
            "0.1.0-beta.+5",
        ] {
            std::env::set_var(VERSION_ENV, given);
            let error = version("0.1.0").expect_err("決めた形でない版番号を拒む");
            assert!(error.contains("0.1.0"), "{given}: {error}");
            assert!(error.contains(given), "{given}: {error}");
        }
        std::env::remove_var(VERSION_ENV);
    }

    #[test]
    fn st52_version_accepts_beta_up_to_98() {
        // ベータ番号は 1〜98（spec。final review R8）。99 と符号つきは上の試験が拒む
        let _g = ENV
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        for given in ["0.1.0-beta.1", "0.1.0-beta.90", "0.1.0-beta.98"] {
            std::env::set_var(VERSION_ENV, given);
            let resolved = version("0.1.0");
            std::env::remove_var(VERSION_ENV);
            assert_eq!(resolved.as_deref(), Ok(given));
        }
    }

    #[test]
    fn st54_channel_is_dev_only_for_dev_versions() {
        // Scenario: 版番号に -dev. を持つ組み立ては開発版になる
        assert_eq!(channel("0.4.0-dev.20261102"), Channel::Dev);
        assert_eq!(channel("0.4.0"), Channel::Official);
        assert_eq!(channel("0.4.0-beta.2"), Channel::Official);
        assert_eq!(Channel::Dev.as_str(), "dev");
        assert_eq!(Channel::Official.as_str(), "official");
    }

    #[test]
    fn st49_stamp_rerun_paths_follow_worktree_git_dir() {
        let (d, _head) = tree();
        let wt = tempfile::tempdir().expect("tempdir");
        let wt_path = wt.path().join("wt");
        git_ok(
            d.path(),
            &[
                "worktree",
                "add",
                "-q",
                "-b",
                "feat",
                wt_path.to_str().expect("utf8"),
            ],
        );
        let lines = rerun_lines(&wt_path, &["crates/a"]);
        let common = fs::canonicalize(d.path().join(".git")).expect("canon");
        let has = |suffix: PathBuf| {
            let want = format!("cargo:rerun-if-changed={}", suffix.display());
            assert!(lines.contains(&want), "{want} が無い: {lines:#?}");
        };
        let gd = common.join("worktrees/wt");
        has(gd.join("HEAD"));
        assert!(
            !lines.iter().any(|l| l.ends_with("/index")),
            "index を見ると add のたびに組み直す: {lines:#?}"
        );
        has(common.join("refs/heads/feat"));
        has(fs::canonicalize(wt_path.join("crates/a")).expect("canon"));
        assert!(lines.contains(&format!("cargo:rerun-if-env-changed={COMMIT_ENV}")));
        assert!(lines.contains(&format!("cargo:rerun-if-env-changed={VERSION_ENV}")));
    }

    #[test]
    fn st49_stamp_status_does_not_touch_index() {
        let (d, _head) = tree();
        // 古い stat を index に残してから、status が index を書き直さないことを見る
        fs::write(d.path().join("crates/a/x.rs"), "touched\n").expect("write");
        let index = d.path().join(".git/index");
        let before = fs::metadata(&index)
            .expect("meta")
            .modified()
            .expect("mtime");
        let bytes = fs::read(&index).expect("read");
        std::thread::sleep(std::time::Duration::from_millis(1100));
        let _ = stamp_clean_env("0.1.0", "debug", d.path(), &["crates/a"]);
        let after = fs::metadata(&index)
            .expect("meta")
            .modified()
            .expect("mtime");
        assert_eq!(before, after);
        assert_eq!(bytes, fs::read(&index).expect("read"));
    }
}
