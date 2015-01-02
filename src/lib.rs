#![deny(missing_docs)]

//! A GLFW window back-end for the Piston game engine.

extern crate glfw;
extern crate collections;
extern crate gl;
extern crate window;
extern crate shader_version;
extern crate input;
extern crate quack;

// External crates.
use quack::{ ActOn, GetFrom, SetAt };
use collections::RingBuf;
use glfw::Context;
use input::{
    keyboard,
    MouseButton,
    Button,
    Input,
    Motion,
};
use window::{
    WindowSettings,
    ShouldClose, Size, PollEvent, SwapBuffers,
    CaptureCursor, DrawSize, Title,
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
    event_queue: RingBuf<input::Input>,
    // Used to compute relative mouse movement.
    last_mouse_pos: Option<(f64, f64)>,
    // The back-end does not remember the title.
    title: String,
    exit_on_esc: bool,
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
        glfw.window_hint(glfw::WindowHint::ContextVersion(major as u32, minor as u32));
        glfw.window_hint(glfw::WindowHint::OpenglForwardCompat(true));
        glfw.window_hint(glfw::WindowHint::OpenglProfile(glfw::OpenGlProfileHint::Core));
        glfw.window_hint(glfw::WindowHint::Samples(settings.samples as u32));

        // Create GLFW window.
        let (window, events) = glfw.create_window(
            settings.size[0],
            settings.size[1],
            settings.title.as_slice(), glfw::WindowMode::Windowed
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
                glfw::WindowEvent::Key(glfw::Key::Escape, _, glfw::Action::Press, _)
                if self.exit_on_esc => {
                    self.window.set_should_close(true);
                }
                glfw::WindowEvent::Char(ch) => {
                    self.event_queue.push_back(Input::Text(ch.to_string()));
                }
                glfw::WindowEvent::Key(key, _, glfw::Action::Press, _) => {
                    self.event_queue.push_back(
                        Input::Press(Button::Keyboard(glfw_map_key(key)))
                    );
                }
                glfw::WindowEvent::Key(key, _, glfw::Action::Release, _) => {
                    self.event_queue.push_back(
                        Input::Release(Button::Keyboard(glfw_map_key(key)))
                    );
                }
                glfw::WindowEvent::MouseButton(button, glfw::Action::Press, _) => {
                    self.event_queue.push_back(
                        Input::Press(Button::Mouse(glfw_map_mouse(button)))
                    );
                }
                glfw::WindowEvent::MouseButton(button, glfw::Action::Release, _) => {
                    self.event_queue.push_back(
                        Input::Release(Button::Mouse(glfw_map_mouse(button)))
                    );
                }
                glfw::WindowEvent::CursorPos(x, y) => {
                    self.event_queue.push_back(Input::Move(Motion::MouseCursor(x, y)));
                    match self.last_mouse_pos {
                        Some((lx, ly)) => {
                            self.event_queue.push_back(
                                Input::Move(Motion::MouseRelative(x - lx, y - ly))
                            )
                        }
                        None => {}
                    };
                    self.last_mouse_pos = Some((x, y));
                }
                glfw::WindowEvent::Scroll(x, y) => {
                    self.event_queue.push_back(Input::Move(Motion::MouseScroll(x, y)));
                }
                glfw::WindowEvent::Size(w, h) => {
                    self.event_queue.push_back(Input::Resize(w as u32, h as u32));
                }
                glfw::WindowEvent::Focus(focus) => {
                    self.event_queue.push_back(Input::Focus(focus));
                }
                _ => {}
            }
        }
    }
}

impl GetFrom<GlfwWindow> for Size {
    fn get_from(obj: &GlfwWindow) -> Size {
        let (w, h) = obj.window.get_size();
        Size([w as u32, h as u32])
    }
}

impl GetFrom<GlfwWindow> for ShouldClose {
    fn get_from(obj: &GlfwWindow) -> ShouldClose {
        ShouldClose(obj.window.should_close())
    }
}

impl ActOn<GlfwWindow, Option<Input>> for PollEvent {
    fn act_on(self, window: &mut GlfwWindow) -> Option<Input> {
        window.flush_messages();

        if window.event_queue.len() != 0 {
            window.event_queue.pop_front()
        } else {
            None
        }
    }
}

impl ActOn<GlfwWindow, ()> for SwapBuffers {
    fn act_on(self, window: &mut GlfwWindow) {
        use glfw::Context;

        window.window.swap_buffers();
    }
}

impl SetAt<GlfwWindow> for CaptureCursor {
    fn set_at(self, window: &mut GlfwWindow) {
        let CaptureCursor(enabled) = self;
        if enabled {
            window.window.set_cursor_mode(glfw::CursorMode::Disabled)
        } else {
            window.window.set_cursor_mode(glfw::CursorMode::Normal);
            window.last_mouse_pos = None;
        }
    }
}

impl SetAt<GlfwWindow> for ShouldClose {
    fn set_at(self, window: &mut GlfwWindow) {
        let ShouldClose(val) = self;
        window.window.set_should_close(val);
    }
}

impl GetFrom<GlfwWindow> for DrawSize {
    fn get_from(obj: &GlfwWindow) -> DrawSize {
        let (w, h) = obj.window.get_framebuffer_size();
        DrawSize([w as u32, h as u32])
    }
}

impl GetFrom<GlfwWindow> for Title {
    fn get_from(obj: &GlfwWindow) -> Title {
        Title(obj.title.clone())
    }
}

impl SetAt<GlfwWindow> for Title {
    fn set_at(self, window: &mut GlfwWindow) {
        let Title(val) = self;
        window.window.set_title(val.as_slice())
    }
}

impl GetFrom<GlfwWindow> for ExitOnEsc {
    fn get_from(obj: &GlfwWindow) -> ExitOnEsc {
        ExitOnEsc(obj.exit_on_esc)
    }
}

impl SetAt<GlfwWindow> for ExitOnEsc {
    fn set_at(self, window: &mut GlfwWindow) {
        let ExitOnEsc(val) = self;
        window.exit_on_esc = val;
    }
}

fn glfw_map_key(keycode: glfw::Key) -> keyboard::Key {
    use input::keyboard::Key;

    match keycode {
        glfw::Key::Num0 => Key::D0,
        glfw::Key::Num1 => Key::D1,
        glfw::Key::Num2 => Key::D2,
        glfw::Key::Num3 => Key::D3,
        glfw::Key::Num4 => Key::D4,
        glfw::Key::Num5 => Key::D5,
        glfw::Key::Num6 => Key::D6,
        glfw::Key::Num7 => Key::D7,
        glfw::Key::Num8 => Key::D8,
        glfw::Key::Num9 => Key::D9,
        glfw::Key::A => Key::A,
        glfw::Key::B => Key::B,
        glfw::Key::C => Key::C,
        glfw::Key::D => Key::D,
        glfw::Key::E => Key::E,
        glfw::Key::F => Key::F,
        glfw::Key::G => Key::G,
        glfw::Key::H => Key::H,
        glfw::Key::I => Key::I,
        glfw::Key::J => Key::J,
        glfw::Key::K => Key::K,
        glfw::Key::L => Key::L,
        glfw::Key::M => Key::M,
        glfw::Key::N => Key::N,
        glfw::Key::O => Key::O,
        glfw::Key::P => Key::P,
        glfw::Key::Q => Key::Q,
        glfw::Key::R => Key::R,
        glfw::Key::S => Key::S,
        glfw::Key::T => Key::T,
        glfw::Key::U => Key::U,
        glfw::Key::V => Key::V,
        glfw::Key::W => Key::W,
        glfw::Key::X => Key::X,
        glfw::Key::Y => Key::Y,
        glfw::Key::Z => Key::Z,
        glfw::Key::Apostrophe => Key::Unknown,
        glfw::Key::Backslash => Key::Backslash,
        glfw::Key::Backspace => Key::Backspace,
        glfw::Key::CapsLock => Key::CapsLock,
        glfw::Key::Delete => Key::Delete,
        glfw::Key::Comma => Key::Comma,
        glfw::Key::Down => Key::Down,
        glfw::Key::End => Key::End,
        glfw::Key::Enter => Key::Return,
        glfw::Key::Equal => Key::Equals,
        glfw::Key::Escape => Key::Escape,
        glfw::Key::F1 => Key::F1,
        glfw::Key::F2 => Key::F2,
        glfw::Key::F3 => Key::F3,
        glfw::Key::F4 => Key::F4,
        glfw::Key::F5 => Key::F5,
        glfw::Key::F6 => Key::F6,
        glfw::Key::F7 => Key::F7,
        glfw::Key::F8 => Key::F8,
        glfw::Key::F9 => Key::F9,
        glfw::Key::F10 => Key::F10,
        glfw::Key::F11 => Key::F11,
        glfw::Key::F12 => Key::F12,
        glfw::Key::F13 => Key::F13,
        glfw::Key::F14 => Key::F14,
        glfw::Key::F15 => Key::F15,
        glfw::Key::F16 => Key::F16,
        glfw::Key::F17 => Key::F17,
        glfw::Key::F18 => Key::F18,
        glfw::Key::F19 => Key::F19,
        glfw::Key::F20 => Key::F20,
        glfw::Key::F21 => Key::F21,
        glfw::Key::F22 => Key::F22,
        glfw::Key::F23 => Key::F23,
        glfw::Key::F24 => Key::F24,
        // Possibly next code.
        glfw::Key::F25 => Key::Unknown,
        glfw::Key::Kp0 => Key::NumPad0,
        glfw::Key::Kp1 => Key::NumPad1,
        glfw::Key::Kp2 => Key::NumPad2,
        glfw::Key::Kp3 => Key::NumPad3,
        glfw::Key::Kp4 => Key::NumPad4,
        glfw::Key::Kp5 => Key::NumPad5,
        glfw::Key::Kp6 => Key::NumPad6,
        glfw::Key::Kp7 => Key::NumPad7,
        glfw::Key::Kp8 => Key::NumPad8,
        glfw::Key::Kp9 => Key::NumPad9,
        glfw::Key::KpDecimal => Key::NumPadDecimal,
        glfw::Key::KpDivide => Key::NumPadDivide,
        glfw::Key::KpMultiply => Key::NumPadMultiply,
        glfw::Key::KpSubtract => Key::NumPadMinus,
        glfw::Key::KpAdd => Key::NumPadPlus,
        glfw::Key::KpEnter => Key::NumPadEnter,
        glfw::Key::KpEqual => Key::NumPadEquals,
        glfw::Key::LeftShift => Key::LShift,
        glfw::Key::LeftControl => Key::LCtrl,
        glfw::Key::LeftAlt => Key::LAlt,
        glfw::Key::LeftSuper => Key::LGui,
        glfw::Key::RightShift => Key::RShift,
        glfw::Key::RightControl => Key::RCtrl,
        glfw::Key::RightAlt => Key::RAlt,
        glfw::Key::RightSuper => Key::RGui,
        // Map to backslash?
        glfw::Key::GraveAccent => Key::Unknown,
        glfw::Key::Home => Key::Home,
        glfw::Key::Insert => Key::Insert,
        glfw::Key::Left => Key::Left,
        glfw::Key::LeftBracket => Key::LeftBracket,
        glfw::Key::Menu => Key::Menu,
        glfw::Key::Minus => Key::Minus,
        glfw::Key::NumLock => Key::NumLockClear,
        glfw::Key::PageDown => Key::PageDown,
        glfw::Key::PageUp => Key::PageUp,
        glfw::Key::Pause => Key::Pause,
        glfw::Key::Period => Key::Period,
        glfw::Key::PrintScreen => Key::PrintScreen,
        glfw::Key::Right => Key::Right,
        glfw::Key::RightBracket => Key::RightBracket,
        glfw::Key::ScrollLock => Key::ScrollLock,
        glfw::Key::Semicolon => Key::Semicolon,
        glfw::Key::Slash => Key::Slash,
        glfw::Key::Space => Key::Space,
        glfw::Key::Tab => Key::Tab,
        glfw::Key::Up => Key::Up,
        glfw::Key::World1 => Key::Unknown,
        glfw::Key::World2 => Key::Unknown,
        // _ => keyboard::Unknown,
    }
}

fn glfw_map_mouse(mouse_button: glfw::MouseButton) -> MouseButton {
    match mouse_button {
        glfw::MouseButton::Button1 => MouseButton::Left,
        glfw::MouseButton::Button2 => MouseButton::Right,
        glfw::MouseButton::Button3 => MouseButton::Middle,
        glfw::MouseButton::Button4 => MouseButton::X1,
        glfw::MouseButton::Button5 => MouseButton::X2,
        glfw::MouseButton::Button6 => MouseButton::Button6,
        glfw::MouseButton::Button7 => MouseButton::Button7,
        glfw::MouseButton::Button8 => MouseButton::Button8,
    }
}
