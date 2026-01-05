// OSV Status Bar standalone binary
slint::include_modules!();

fn main() {
    let window = OsvBarWindow::new().unwrap();
    
    // Update time periodically (simplified - real impl would use proper timer)
    let time = chrono::Local::now().format("%H:%M").to_string();
    // window.set_time(time.into());
    
    window.run().unwrap();
}
