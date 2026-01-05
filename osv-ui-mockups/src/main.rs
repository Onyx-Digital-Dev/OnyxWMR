// OSV Preview - Combined view of all components
slint::include_modules!();

fn main() {
    let window = OsvPreview::new().unwrap();
    window.run().unwrap();
}
