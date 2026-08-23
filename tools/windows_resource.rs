use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct BinaryResource {
    pub stem: &'static str,
    pub original_filename: &'static str,
    pub description: &'static str,
}

pub fn compile_product_resources(binaries: &[BinaryResource]) {
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let manifest_dir = PathBuf::from(
        env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is unavailable"),
    );
    let repo_root = manifest_dir
        .parent()
        .and_then(Path::parent)
        .expect("product packages must remain under apps/");
    let icon = repo_root
        .join("installer")
        .join("assets")
        .join("branding")
        .join("icon.ico");
    if !icon.is_file() {
        panic!("Quetzalcoatl icon is absent: {}", icon.display());
    }

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is unavailable"));
    let compiler = find_resource_compiler();
    println!("cargo:rerun-if-changed={}", icon.display());
    println!("cargo:rerun-if-changed=../../tools/windows_resource.rs");

    for binary in binaries {
        let resource = compile_resource(&compiler, &out_dir, &icon, binary);
        println!(
            "cargo:rustc-link-arg-bin={}={}",
            binary.stem,
            resource.display()
        );
    }
}

fn compile_resource(
    compiler: &Path,
    out_dir: &Path,
    icon: &Path,
    binary: &BinaryResource,
) -> PathBuf {
    let source = out_dir.join(format!("{}.rc", binary.stem));
    let output = out_dir.join(format!("{}.res", binary.stem));
    let icon_path = icon.to_string_lossy().replace('\\', "/");
    let version = env::var("CARGO_PKG_VERSION").expect("CARGO_PKG_VERSION is unavailable");
    let numeric_version = windows_numeric_version(&version);
    let contents = format!(
        r#"
1 ICON "{icon_path}"

1 VERSIONINFO
FILEVERSION {numeric_version}
PRODUCTVERSION {numeric_version}
FILEFLAGSMASK 0x3fL
FILEFLAGS 0x0L
FILEOS 0x40004L
FILETYPE 0x1L
FILESUBTYPE 0x0L
BEGIN
    BLOCK "StringFileInfo"
    BEGIN
        BLOCK "040904B0"
        BEGIN
            VALUE "CompanyName", "GNX Labs\0"
            VALUE "FileDescription", "{description}\0"
            VALUE "FileVersion", "{version}\0"
            VALUE "InternalName", "{stem}\0"
            VALUE "LegalCopyright", "Copyright (c) 2008-2020 GNX Labs, Hector AB and other contributors\0"
            VALUE "OriginalFilename", "{original_filename}\0"
            VALUE "ProductName", "Quetzalcoatl\0"
            VALUE "ProductVersion", "{version}\0"
        END
    END
    BLOCK "VarFileInfo"
    BEGIN
        VALUE "Translation", 0x0409, 1200
    END
END
"#,
        description = binary.description,
        stem = binary.stem,
        original_filename = binary.original_filename,
    );
    fs::write(&source, contents).expect("cannot write Windows resource script");

    let status = Command::new(compiler)
        .arg("/nologo")
        .arg(format!("/fo{}", output.display()))
        .arg(&source)
        .status()
        .unwrap_or_else(|error| {
            panic!(
                "cannot execute Windows resource compiler {}: {error}",
                compiler.display()
            )
        });
    if !status.success() {
        panic!(
            "Windows resource compiler failed for {} with status {status}",
            source.display()
        );
    }
    output
}

fn windows_numeric_version(version: &str) -> String {
    let mut parts = version
        .split('.')
        .map(|part| {
            part.parse::<u16>()
                .expect("package version must be numeric")
        })
        .collect::<Vec<_>>();
    if parts.len() != 3 {
        panic!("package version must contain major.minor.patch");
    }
    parts.push(0);
    parts
        .iter()
        .map(u16::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn find_resource_compiler() -> PathBuf {
    if let Some(explicit) = env::var_os("RC") {
        let candidate = PathBuf::from(explicit);
        if candidate.is_file() {
            return candidate;
        }
    }

    if let Some(path) = env::var_os("PATH") {
        for directory in env::split_paths(&path) {
            let candidate = directory.join("rc.exe");
            if candidate.is_file() {
                return candidate;
            }
        }
    }

    let mut candidates = Vec::new();
    for variable in ["ProgramFiles(x86)", "ProgramFiles"] {
        let Some(root) = env::var_os(variable) else {
            continue;
        };
        let bin = PathBuf::from(root)
            .join("Windows Kits")
            .join("10")
            .join("bin");
        let Ok(entries) = fs::read_dir(bin) else {
            continue;
        };
        for entry in entries.flatten() {
            let candidate = entry.path().join("x64").join("rc.exe");
            if candidate.is_file() {
                candidates.push(candidate);
            }
        }
    }
    candidates.sort();
    candidates.pop().unwrap_or_else(|| {
        panic!(
            "Windows SDK rc.exe was not found; install the Windows SDK or set RC to its absolute path"
        )
    })
}
