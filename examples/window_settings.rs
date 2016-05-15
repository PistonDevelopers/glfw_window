extern crate glfw_window;
extern crate window;

use glfw_window::GlfwWindow;
use window::WindowSettings;

fn main() {
    let _ = GlfwWindow::new(
        WindowSettings::new("GLFW Window", (640, 480))
            .fullscreen(false)
            .vsync(true) // etc
    ).unwrap();
}