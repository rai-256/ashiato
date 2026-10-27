// SPDX-License-Identifier: AGPL-3.0-only
//! 組み立ての印 `<版番号>+<commit>[.dirty][.debug]`（ST49 / design D5・D6）。S-01 と C-02 の `build.rs` が共有する。
//!
//! 依存は `std` だけ。git は `std::process::Command` で呼ぶ。
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

/// 組み立ての commit を渡す環境変数（空でなければ `git` より先に使う）。
pub const COMMIT_ENV: &str = "ASHIATO_BUILD_COMMIT";

/// 印を組む。形は `<version>+<commit>[.dirty][.debug]`。
/// `.dirty` は `source_paths` の下に未 commit の変更（未追跡を含む）があるときだけ、`.debug` は `profile` が `release` でないとき。
pub fn stamp(version: &str, profile: &str, repo: &Path, source_paths: &[&str]) -> String {
    let env_commit = std::env::var(COMMIT_ENV).ok();
    let commit = commit(env_commit.as_deref(), repo);
    let dirty = if is_dirty(repo, source_paths) {
        ".dirty"
    } else {
        ""
    };
    let debug = if profile == "release" { "" } else { ".debug" };
    format!("{version}+{commit}{dirty}{debug}")
}

/// 作り直しの印を cargo へ出す。HEAD・HEAD が指す参照・`packed-refs`・`index`・部品のソースの道・`ASHIATO_BUILD_COMMIT`。
pub fn emit_rerun(repo: &Path, source_paths: &[&str]) {
    let mut out = std::io::stdout().lock();
    for line in rerun_lines(repo, source_paths) {
        // 出力先が壊れていても組み立ては止めない（印が古くなるだけ）
        let _ = writeln!(out, "{line}");
    }
}

fn rerun_lines(repo: &Path, source_paths: &[&str]) -> Vec<String> {
    let mut lines = vec![format!("cargo:rerun-if-env-changed={COMMIT_ENV}")];
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
        watch.push(git_dir.join("index"));
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

/// 成功して出力が空でないときだけ `Some`。`GIT_OPTIONAL_LOCKS=0` で `git status` が `index` を書き換えない。
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
        has(gd.join("index"));
        has(common.join("refs/heads/feat"));
        has(fs::canonicalize(wt_path.join("crates/a")).expect("canon"));
        assert!(lines.contains(&format!("cargo:rerun-if-env-changed={COMMIT_ENV}")));
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
