// OSV Application Launcher standalone binary
slint::include_modules!();

fn main() {
    let window = OsvIntakeWindow::new().unwrap();
    window.run().unwrap();
}
