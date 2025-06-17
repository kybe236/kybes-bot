use tokio::process::Command;

pub async fn get_git_hash() -> Option<String> {
    let output = Command::new("git")
        .args(&["rev-parse", "HEAD"])
        .output()
        .await
        .ok()?;

    if output.status.success() {
        let hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Some(hash)
    } else {
        None
    }
}
