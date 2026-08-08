use std::env;
use std::process::Command;

/// Run git and return trimmed stdout, or Err with context.
fn git(args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .output()
        .map_err(|error| format!("failed to invoke git: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git {} failed: {}", args.join(" "), stderr.trim()));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// UTC ISO-8601 build timestamp, e.g. 2026-08-08T18:00:00Z.
fn build_timestamp() -> String {
    Command::new("date")
        .args(["-u", "+%Y-%m-%dT%H:%M:%SZ"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

fn main() {
    // Cargo sets PROFILE for build scripts: "debug" or "release".
    let is_release = env::var("PROFILE").as_deref() == Ok("release");

    let sha = match git(&["rev-parse", "HEAD"]) {
        Ok(sha) => sha,
        Err(error) => {
            if is_release {
                panic!(
                    "RELEASE BUILD REJECTED: cannot resolve git HEAD ({}); \
                     refusing to produce a binary with unknown provenance",
                    error
                );
            }
            eprintln!("warning: git HEAD unavailable; embedding treeState=unknown ({error})");
            emit("unknown", "unknown");
            return;
        }
    };

    let porcelain = match git(&["status", "--porcelain"]) {
        Ok(status) => status,
        Err(error) => {
            if is_release {
                panic!(
                    "RELEASE BUILD REJECTED: cannot check worktree cleanliness ({}); \
                     refusing to produce a binary from an unverifiable tree",
                    error
                );
            }
            eprintln!(
                "warning: cannot check worktree cleanliness; embedding treeState=unknown ({error})"
            );
            emit(&sha, "unknown");
            return;
        }
    };

    if !porcelain.is_empty() && is_release {
        let entries = porcelain.lines().count();
        panic!(
            "RELEASE BUILD REJECTED: worktree is dirty ({} uncommitted/untracked entries).\n\
             Formal release builds require a clean Git tree so the artifact maps to exactly one SHA.\n\
             Commit or stash the changes, or build from a pristine checkout.\n\
             git status --porcelain:\n{}",
            entries, porcelain
        );
    }

    let tree_state = if porcelain.is_empty() { "clean" } else { "dirty" };
    emit(&sha, tree_state);
}

fn emit(sha: &str, tree_state: &str) {
    println!("cargo:rustc-env=GIT_SHA={sha}");
    println!("cargo:rustc-env=GIT_TREE_STATE={tree_state}");
    println!("cargo:rustc-env=BUILD_TIMESTAMP={}", build_timestamp());
}
