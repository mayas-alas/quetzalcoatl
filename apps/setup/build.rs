fn main() {
    #[cfg(windows)]
    {
        let manifest = r#"<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0"><trustInfo xmlns="urn:schemas-microsoft-com:asm.v3"><security><requestedPrivileges><requestedExecutionLevel level="requireAdministrator" uiAccess="false"/></requestedPrivileges></security></trustInfo></assembly>"#;
        winresource::WindowsResource::new()
            .set_manifest(manifest)
            .compile()
            .expect("embed gnx elevation manifest");
    }
}
