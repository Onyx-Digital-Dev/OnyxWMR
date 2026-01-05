// OSV Control Panel standalone binary
slint::include_modules!();

fn main() {
    let window = OsvControlWindow::new().unwrap();
    window.run().unwrap();
}
