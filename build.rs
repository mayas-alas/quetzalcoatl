fn main() {
    let target = std::env::var("TARGET").unwrap_or_else(|_| "unknown".to_string());
    let build_epoch = std::env::var("SOURCE_DATE_EPOCH").unwrap_or_else(|_| "0".to_string());
    let commit = std::env::var("GNX_GIT_COMMIT").unwrap_or_else(|_| "unversioned".to_string());

    println!("cargo:rustc-env=GNX_BUILD_TARGET={target}");
    println!("cargo:rustc-env=GNX_BUILD_EPOCH={build_epoch}");
    println!("cargo:rustc-env=GNX_GIT_COMMIT={commit}");
    println!("cargo:rerun-if-env-changed=SOURCE_DATE_EPOCH");
    println!("cargo:rerun-if-env-changed=GNX_GIT_COMMIT");
    println!("cargo:rerun-if-changed=assets/branding-install-logo.ico");
    println!("cargo:rerun-if-changed=assets/tray-icon.ico");

    if target.contains("windows") {
        let mut resource = winresource::WindowsResource::new();
        resource
            .set_icon("assets/branding-install-logo.ico")
            .set_icon_with_id("assets/tray-icon.ico", "2")
            .set("ProductName", "Quetzalcoatl Next")
            .set(
                "FileDescription",
                "Quetzalcoatl Next Installer, CLI and Service",
            )
            .set("CompanyName", "Quetzalcoatl Next")
            .set("OriginalFilename", "gnx.exe")
            .set("InternalName", "gnx");
        resource
            .compile()
            .expect("no se pudieron compilar los recursos de branding Windows");
    }
}
