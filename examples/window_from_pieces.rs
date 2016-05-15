
extern crate glfw;
extern crate glfw_window;
extern crate window;

use glfw::Context;
use glfw_window::GlfwWindow;

fn main() {
    let glfw = glfw::init(glfw::FAIL_ON_ERRORS).unwrap();
    let (mut glfw_window, glfw_events) = glfw.create_window(640, 480, "GLFW Window", glfw::WindowMode::Windowed).unwrap();
    
    glfw_window.set_all_polling(true);
    glfw_window.make_current();
    
    let _: GlfwWindow = GlfwWindow::from_pieces(glfw_window, glfw, glfw_events, true);
}