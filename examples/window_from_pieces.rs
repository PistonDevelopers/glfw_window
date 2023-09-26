
extern crate glfw;
extern crate glfw_window;

use glfw::Context;
use glfw_window::GlfwWindow;

fn main() {
    let mut glfw = glfw::init_no_callbacks().unwrap();
    let (mut glfw_window, glfw_events) = glfw.create_window(640, 480, "GLFW Window", glfw::WindowMode::Windowed).unwrap();
    
    glfw_window.set_all_polling(true);
    glfw_window.make_current();
    
    let _: GlfwWindow = GlfwWindow::from_pieces(glfw_window, glfw, glfw_events, true);
}
