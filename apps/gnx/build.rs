include!("../../tools/windows_resource.rs");

fn main() {
    compile_product_resources(&[
        BinaryResource {
            stem: "gnx",
            original_filename: "gnx.exe",
            description: "Quetzalcoatl CLI",
        },
        BinaryResource {
            stem: "gnx-tray",
            original_filename: "gnx-tray.exe",
            description: "Quetzalcoatl Tray",
        },
    ]);
}
