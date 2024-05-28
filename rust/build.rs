use std::process::Command;

fn main() {
    let output = Command::new("git")
        .args(["rev-parse", "--short=8", "HEAD"])
        .output()
        .expect("Failed to execute git command");
    let git_commit_hash =
        String::from_utf8(output.stdout).expect("Failed to convert git output to string");
    println!("cargo:rustc-env=GIT_COMMIT_HASH={}", git_commit_hash.trim());
}
