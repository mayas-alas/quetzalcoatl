#[test]
fn dependency_boundaries() {
    for dir in ["src/domain", "src/app"] {
        for f in std::fs::read_dir(dir).unwrap() {
            let s = std::fs::read_to_string(f.unwrap().path()).unwrap();
            for forbidden in ["crate::adapter", "std::process", "std::fs", "windows_sys"] {
                assert!(!s.contains(forbidden), "{dir} imports {forbidden}");
            }
        }
    }
}
