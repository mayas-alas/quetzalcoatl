include!("../../tools/windows_resource.rs");

fn main() {
    compile_product_resources(&[BinaryResource {
        stem: "gnx-bootstrap",
        original_filename: "gnx-bootstrap.exe",
        description: "Quetzalcoatl Setup Bootstrap",
    }]);
}
