use std::{env, path::PathBuf, process::Command};

fn main() {
    for file in ["go.mod", "go.sum", "main.go"] {
        println!("cargo:rerun-if-changed=tunnel/{file}");
    }
    println!("cargo:rerun-if-env-changed=GNX_GO");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    assert_eq!(
        env::var("CARGO_CFG_TARGET_ARCH").as_deref(),
        Ok("x86_64"),
        "el adaptador de conexión requiere Windows x64"
    );
    let go = env::var_os("GNX_GO").unwrap_or_else(|| "go".into());
    let mut command = Command::new(go);
    command
        .env("GOTOOLCHAIN", "local")
        .env("GOENV", "off")
        .env("GOWORK", "off")
        .env("GOFLAGS", "");
    let toolchain = command
        .arg("version")
        .output()
        .expect("Go 1.27.1 requerido para compilar el adaptador Tailcat");
    assert!(
        toolchain.status.success() && toolchain.stdout.starts_with(b"go version go1.27.1 "),
        "se requiere el toolchain fijado Go 1.27.1"
    );
    let go = command.get_program().to_owned();
    let output = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
    let built = Command::new(go)
        .current_dir("tunnel")
        .env("GOTOOLCHAIN", "local")
        .env("GOENV", "off")
        .env("GOWORK", "off")
        .env("GOFLAGS", "")
        .env("GOOS", "windows")
        .env("GOARCH", "amd64")
        .env("GOAMD64", "v1")
        .env("CGO_ENABLED", "0")
        .args([
            "build",
            "-mod=readonly",
            "-trimpath",
            "-buildvcs=false",
            "-ldflags=-s -w",
            "-o",
        ])
        .arg(output.join("gnx-tailcat-relay.exe"))
        .arg(".")
        .status()
        .expect("no se pudo iniciar el build del adaptador Tailcat");
    assert!(built.success(), "falló el build del adaptador Tailcat");
}
