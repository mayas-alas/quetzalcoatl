fn main() {
    let target = std::env::var("TARGET").unwrap_or_else(|_| "unknown".to_string());
    let build_epoch = std::env::var("SOURCE_DATE_EPOCH").unwrap_or_else(|_| "0".to_string());
    let commit = std::env::var("GNX_GIT_COMMIT").unwrap_or_else(|_| "unversioned".to_string());

    println!("cargo:rustc-env=GNX_BUILD_TARGET={target}");
    println!("cargo:rustc-env=GNX_BUILD_EPOCH={build_epoch}");
    println!("cargo:rustc-env=GNX_GIT_COMMIT={commit}");
    println!("cargo:rerun-if-env-changed=SOURCE_DATE_EPOCH");
    println!("cargo:rerun-if-env-changed=GNX_GIT_COMMIT");
}
