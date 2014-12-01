#![deny(missing_docs)]

//! A GLFW window back-end for the Piston game engine.

extern crate glfw;
extern crate collections;
extern crate gl;
extern crate window;
extern crate shader_version;
extern crate input;
extern crate current;

// External crates.
use current::{ Get, Modifier };
use collections::RingBuf;
use glfw::Context;
use input::{
    keyboard,
    mouse,
    InputEvent,
};
use window::{
    Window,
    WindowSettings,
    ShouldClose, Size, PollEvent, SwapBuffers,
    CaptureCursor, DrawSize, Title, SetTitle,
    ExitOnEsc
};
use shader_version::opengl::OpenGL;

/// Contains stuff for game window.
pub struct GlfwWindow {
    /// The window.
    pub window: glfw::Window,
    /// Receives events from window.
    events: Receiver<(f64, glfw::WindowEvent)>,
    /// GLFW context.
    pub glfw: glfw::Glfw,
    event_queue: RingBuf<input::InputEvent>,
    // Used to compute relative mouse movement.
    last_mouse_pos: Option<(f64, f64)>,
    // The back-end does not remember the title.
    title: String,
    exit_on_esc: bool,
}

#[test]
fn test_glfw_window() {
    fn foo<T: Window>() {}

    foo::<GlfwWindow>();
}

impl GlfwWindow {
    /// Create a new game window from an existing GLFW window.
    pub fn from_pieces(win: glfw::Window, glfw: glfw::Glfw,
                       events: Receiver<(f64, glfw::WindowEvent)>,
                       exit_on_esc: bool) -> GlfwWindow {
        win.set_all_polling(true);
        win.make_current();
        let title = "<unknown window title, created from_pieces>";
        GlfwWindow {
            window: win,
            events: events,
            glfw: glfw,
            exit_on_esc: exit_on_esc,
            event_queue: RingBuf::new(),
            last_mouse_pos: None,
            title: title.to_string(),
        }
    }

    /// Creates a new game window for GLFW.
    pub fn new(opengl: OpenGL, settings: WindowSettings) -> GlfwWindow {
        use glfw::Context;

        // Initialize GLFW.
        let glfw = glfw::init(glfw::FAIL_ON_ERRORS).unwrap();

        let (major, minor) = opengl.get_major_minor();

        // Make sure we have the right GL version.
        glfw.window_hint(glfw::ContextVersion(major as u32, minor as u32));
        glfw.window_hint(glfw::OpenglForwardCompat(true));
        glfw.window_hint(glfw::OpenglProfile(glfw::OpenGlProfileHint::Core));
        glfw.window_hint(glfw::Samples(settings.samples as u32));

        // Create GLFW window.
        let (window, events) = glfw.create_window(
            settings.size[0],
            settings.size[1],
            settings.title.as_slice(), glfw::Windowed
        ).expect("Failed to create GLFW window.");
        window.set_all_polling(true);
        window.make_current();

        // Load the OpenGL function pointers
        gl::load_with(|s| window.get_proc_address(s));

        GlfwWindow {
            window: window,
            events: events,
            glfw: glfw,
            event_queue: RingBuf::new(),
            last_mouse_pos: None,
            title: settings.title,
            exit_on_esc: settings.exit_on_esc,
        }
    }

    fn flush_messages(&mut self) {
        if self.event_queue.len() != 0 {
            return;
        }

        self.glfw.poll_events();
        for (_, event) in glfw::flush_messages(&self.events) {
            match event {
                glfw::WindowEvent::Key(glfw::Key::Escape, _, glfw::Press, _)
                if self.exit_on_esc => {
                    self.window.set_should_close(true);
                }
                glfw::WindowEvent::Char(ch) => {
                    self.event_queue.push_back(input::Text(ch.to_string()));
                }
                glfw::WindowEvent::Key(key, _, glfw::Press, _) => {
                    self.event_queue.push_back(
                        input::Press(input::Keyboard(glfw_map_key(key)))
                    );
                }
                glfw::WindowEvent::Key(key, _, glfw::Release, _) => {
                    self.event_queue.push_back(
                        input::Release(input::Keyboard(glfw_map_key(key)))
                    );
                }
                glfw::WindowEvent::MouseButton(button, glfw::Press, _) => {
                    self.event_queue.push_back(
                        input::Press(input::Mouse(glfw_map_mouse(button)))
                    );
                }
                glfw::WindowEvent::MouseButton(button, glfw::Release, _) => {
                    self.event_queue.push_back(
                        input::Release(input::Mouse(glfw_map_mouse(button)))
                    );
                }
                glfw::WindowEvent::CursorPos(x, y) => {
                    self.event_queue.push_back(input::Move(input::MouseCursor(x, y)));
                    match self.last_mouse_pos {
                        Some((lx, ly)) => {
                            self.event_queue.push_back(
                                input::Move(input::MouseRelative(x - lx, y - ly))
                            )
                        }
                        None => {}
                    };
                    self.last_mouse_pos = Some((x, y));
                }
                glfw::WindowEvent::Scroll(x, y) => {
                    self.event_queue.push_back(input::Move(input::MouseScroll(x, y)));
                }
                glfw::WindowEvent::Size(w, h) => {
                    self.event_queue.push_back(input::Resize(w as u32, h as u32));
                }
                glfw::WindowEvent::Focus(focus) => {
                    self.event_queue.push_back(input::Focus(focus));
                }
                _ => {}
            }
        }
    }
}

impl Get<Size> for GlfwWindow {
    fn get(&self) -> Size {
        let (w, h) = self.window.get_size();
        Size([w as u32, h as u32])
    }
}

impl Get<ShouldClose> for GlfwWindow {
    fn get(&self) -> ShouldClose {
        ShouldClose(self.window.should_close())
    }
}

impl PollEvent<InputEvent> for GlfwWindow {
    fn poll_event(&mut self) -> Option<input::InputEvent> {
        self.flush_messages();

        if self.event_queue.len() != 0 {
            self.event_queue.pop_front()
        } else {
            None
        }
    }
}

impl SwapBuffers for GlfwWindow {
    fn swap_buffers(&mut self) {
        use glfw::Context;

        self.window.swap_buffers();
    }
}

impl Modifier<GlfwWindow> for CaptureCursor {
    fn modify(self, window: &mut GlfwWindow) {
        let CaptureCursor(enabled) = self;
        if enabled {
            window.window.set_cursor_mode(glfw::CursorMode::Disabled)
        } else {
            window.window.set_cursor_mode(glfw::CursorMode::Normal);
            window.last_mouse_pos = None;
        }
    }
}

impl Modifier<GlfwWindow> for ShouldClose {
    fn modify(self, window: &mut GlfwWindow) {
        let ShouldClose(val) = self;
        window.window.set_should_close(val);
    }
}

impl Get<DrawSize> for GlfwWindow {
    fn get(&self) -> DrawSize {
        let (w, h) = self.window.get_framebuffer_size();
        DrawSize([w as u32, h as u32])
    }
}

impl Get<Title> for GlfwWindow {
    fn get(&self) -> Title {
        Title(self.title.clone())
    }
}

impl Modifier<GlfwWindow> for Title {
    fn modify(self, window: &mut GlfwWindow) {
        let Title(val) = self;
        window.window.set_title(val.as_slice())
    }
}

impl Get<ExitOnEsc> for GlfwWindow {
    fn get(&self) -> ExitOnEsc {
        ExitOnEsc(self.exit_on_esc)
    }
}

impl Modifier<GlfwWindow> for ExitOnEsc {
    fn modify(self, window: &mut GlfwWindow) {
        let ExitOnEsc(val) = self;
        window.exit_on_esc = val;
    }
}

fn glfw_map_key(keycode: glfw::Key) -> keyboard::Key {
    match keycode {
        glfw::Key::Num0 => keyboard::D0,
        glfw::Key::Num1 => keyboard::D1,
        glfw::Key::Num2 => keyboard::D2,
        glfw::Key::Num3 => keyboard::D3,
        glfw::Key::Num4 => keyboard::D4,
        glfw::Key::Num5 => keyboard::D5,
        glfw::Key::Num6 => keyboard::D6,
        glfw::Key::Num7 => keyboard::D7,
        glfw::Key::Num8 => keyboard::D8,
        glfw::Key::Num9 => keyboard::D9,
        glfw::Key::A => keyboard::A,
        glfw::Key::B => keyboard::B,
        glfw::Key::C => keyboard::C,
        glfw::Key::D => keyboard::D,
        glfw::Key::E => keyboard::E,
        glfw::Key::F => keyboard::F,
        glfw::Key::G => keyboard::G,
        glfw::Key::H => keyboard::H,
        glfw::Key::I => keyboard::I,
        glfw::Key::J => keyboard::J,
        glfw::Key::K => keyboard::K,
        glfw::Key::L => keyboard::L,
        glfw::Key::M => keyboard::M,
        glfw::Key::N => keyboard::N,
        glfw::Key::O => keyboard::O,
        glfw::Key::P => keyboard::P,
        glfw::Key::Q => keyboard::Q,
        glfw::Key::R => keyboard::R,
        glfw::Key::S => keyboard::S,
        glfw::Key::T => keyboard::T,
        glfw::Key::U => keyboard::U,
        glfw::Key::V => keyboard::V,
        glfw::Key::W => keyboard::W,
        glfw::Key::X => keyboard::X,
        glfw::Key::Y => keyboard::Y,
        glfw::Key::Z => keyboard::Z,
        glfw::Key::Apostrophe => keyboard::Unknown,
        glfw::Key::Backslash => keyboard::Backslash,
        glfw::Key::Backspace => keyboard::Backspace,
        glfw::Key::CapsLock => keyboard::CapsLock,
        glfw::Key::Delete => keyboard::Delete,
        glfw::Key::Comma => keyboard::Comma,
        glfw::Key::Down => keyboard::Down,
        glfw::Key::End => keyboard::End,
        glfw::Key::Enter => keyboard::Return,
        glfw::Key::Equal => keyboard::Equals,
        glfw::Key::Escape => keyboard::Escape,
        glfw::Key::F1 => keyboard::F1,
        glfw::Key::F2 => keyboard::F2,
        glfw::Key::F3 => keyboard::F3,
        glfw::Key::F4 => keyboard::F4,
        glfw::Key::F5 => keyboard::F5,
        glfw::Key::F6 => keyboard::F6,
        glfw::Key::F7 => keyboard::F7,
        glfw::Key::F8 => keyboard::F8,
        glfw::Key::F9 => keyboard::F9,
        glfw::Key::F10 => keyboard::F10,
        glfw::Key::F11 => keyboard::F11,
        glfw::Key::F12 => keyboard::F12,
        glfw::Key::F13 => keyboard::F13,
        glfw::Key::F14 => keyboard::F14,
        glfw::Key::F15 => keyboard::F15,
        glfw::Key::F16 => keyboard::F16,
        glfw::Key::F17 => keyboard::F17,
        glfw::Key::F18 => keyboard::F18,
        glfw::Key::F19 => keyboard::F19,
        glfw::Key::F20 => keyboard::F20,
        glfw::Key::F21 => keyboard::F21,
        glfw::Key::F22 => keyboard::F22,
        glfw::Key::F23 => keyboard::F23,
        glfw::Key::F24 => keyboard::F24,
        // Possibly next code.
        glfw::Key::F25 => keyboard::Unknown,
        glfw::Key::Kp0 => keyboard::NumPad0,
        glfw::Key::Kp1 => keyboard::NumPad1,
        glfw::Key::Kp2 => keyboard::NumPad2,
        glfw::Key::Kp3 => keyboard::NumPad3,
        glfw::Key::Kp4 => keyboard::NumPad4,
        glfw::Key::Kp5 => keyboard::NumPad5,
        glfw::Key::Kp6 => keyboard::NumPad6,
        glfw::Key::Kp7 => keyboard::NumPad7,
        glfw::Key::Kp8 => keyboard::NumPad8,
        glfw::Key::Kp9 => keyboard::NumPad9,
        glfw::Key::KpDecimal => keyboard::NumPadDecimal,
        glfw::Key::KpDivide => keyboard::NumPadDivide,
        glfw::Key::KpMultiply => keyboard::NumPadMultiply,
        glfw::Key::KpSubtract => keyboard::NumPadMinus,
        glfw::Key::KpAdd => keyboard::NumPadPlus,
        glfw::Key::KpEnter => keyboard::NumPadEnter,
        glfw::Key::KpEqual => keyboard::NumPadEquals,
        glfw::Key::LeftShift => keyboard::LShift,
        glfw::Key::LeftControl => keyboard::LCtrl,
        glfw::Key::LeftAlt => keyboard::LAlt,
        glfw::Key::LeftSuper => keyboard::LGui,
        glfw::Key::RightShift => keyboard::RShift,
        glfw::Key::RightControl => keyboard::RCtrl,
        glfw::Key::RightAlt => keyboard::RAlt,
        glfw::Key::RightSuper => keyboard::RGui,
        // Map to backslash?
        glfw::Key::GraveAccent => keyboard::Unknown,
        glfw::Key::Home => keyboard::Home,
        glfw::Key::Insert => keyboard::Insert,
        glfw::Key::Left => keyboard::Left,
        glfw::Key::LeftBracket => keyboard::LeftBracket,
        glfw::Key::Menu => keyboard::Menu,
        glfw::Key::Minus => keyboard::Minus,
        glfw::Key::NumLock => keyboard::NumLockClear,
        glfw::Key::PageDown => keyboard::PageDown,
        glfw::Key::PageUp => keyboard::PageUp,
        glfw::Key::Pause => keyboard::Pause,
        glfw::Key::Period => keyboard::Period,
        glfw::Key::PrintScreen => keyboard::PrintScreen,
        glfw::Key::Right => keyboard::Right,
        glfw::Key::RightBracket => keyboard::RightBracket,
        glfw::Key::ScrollLock => keyboard::ScrollLock,
        glfw::Key::Semicolon => keyboard::Semicolon,
        glfw::Key::Slash => keyboard::Slash,
        glfw::Key::Space => keyboard::Space,
        glfw::Key::Tab => keyboard::Tab,
        glfw::Key::Up => keyboard::Up,
        glfw::Key::World1 => keyboard::Unknown,
        glfw::Key::World2 => keyboard::Unknown,
        // _ => keyboard::Unknown,
    }
}

fn glfw_map_mouse(mouse_button: glfw::MouseButton) -> mouse::Button {
    match mouse_button {
        glfw::MouseButton::Button1 => mouse::Left,
        glfw::MouseButton::Button2 => mouse::Right,
        glfw::MouseButton::Button3 => mouse::Middle,
        glfw::MouseButton::Button4 => mouse::X1,
        glfw::MouseButton::Button5 => mouse::X2,
        glfw::MouseButton::Button6 => mouse::Button6,
        glfw::MouseButton::Button7 => mouse::Button7,
        glfw::MouseButton::Button8 => mouse::Button8,
    }
}
