use std::path::Path;
use zeroize::Zeroizing;

fn read_private(path: &Path) -> Result<Zeroizing<String>, String> {
    std::fs::read_to_string(path)
        .map(Zeroizing::new)
        .map_err(|_| "RELEASE_PRIVATE_KEY_UNREADABLE".into())
}

fn fail(code: &str) -> ! {
    println!(
        "{}",
        serde_json::json!({"schema":1,"operation":"sign-release","state":"FAILED","code":code})
    );
    std::process::exit(1)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() == 3 && args[0] == "public-key" && args[1] == "--private-key" {
        let secret = read_private(Path::new(&args[2])).unwrap_or_else(|code| fail(&code));
        let public = gnx::release_auth::public_key(&secret).unwrap_or_else(|code| fail(&code));
        let public_bytes: [u8; 32] = hex::decode(&public).unwrap().try_into().unwrap();
        println!(
            "{}",
            serde_json::json!({"schema":1,"operation":"public-key","state":"READY","public_key":public,"key_id":gnx::release_auth::key_id(&public_bytes)})
        );
        return;
    }
    if args.len() == 7
        && args[0] == "sign"
        && args[1] == "--private-key"
        && args[3] == "--manifest"
        && args[5] == "--output"
    {
        let secret = read_private(Path::new(&args[2])).unwrap_or_else(|code| fail(&code));
        let manifest =
            std::fs::read(&args[4]).unwrap_or_else(|_| fail("RELEASE_MANIFEST_UNREADABLE"));
        let signature =
            gnx::release_auth::sign(&manifest, &secret).unwrap_or_else(|code| fail(&code));
        std::fs::write(&args[6], signature)
            .unwrap_or_else(|_| fail("RELEASE_SIGNATURE_WRITE_FAILED"));
        println!(
            "{}",
            serde_json::json!({"schema":1,"operation":"sign-release","state":"READY","code":"SIGNED"})
        );
        return;
    }
    fail("SIGNER_ARGUMENT_INVALID")
}
