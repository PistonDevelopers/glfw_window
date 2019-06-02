
extern crate glfw_window;
extern crate shader_version;
extern crate window;

use glfw_window::GlfwWindow;
use shader_version::OpenGL;
use window::WindowSettings;

fn main() {
    let _ = GlfwWindow::new(
        &WindowSettings::new("GLFW Window", (640, 480))
            .fullscreen(false)
            .vsync(true)
            .graphics_api(OpenGL::V2_1) // etc
    ).unwrap();
}
