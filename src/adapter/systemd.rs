pub fn active(name: &str) -> bool {
    if !crate::domain::compute::REQUIRED.contains(&name) {
        return false;
    }
    std::process::Command::new("systemctl")
        .args(["is-active", "--quiet", &format!("gnx-{name}.service")])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}
