include!("../../tools/windows_resource.rs");

fn main() {
    compile_product_resources(&[BinaryResource {
        stem: "gnx-service",
        original_filename: "gnx-service.exe",
        description: "Quetzalcoatl Service",
    }]);
}
