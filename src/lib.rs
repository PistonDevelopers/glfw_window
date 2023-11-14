#![deny(missing_docs)]

//! A GLFW window back-end for the Piston game engine.

extern crate gl;
extern crate glfw;
extern crate input;
extern crate shader_version;
extern crate window;

// External crates.
use glfw::{Context, Joystick, JoystickId};
use input::{
    keyboard, Button, ButtonArgs, ButtonState, CloseArgs, ControllerAxisArgs, ControllerButton,
    Event, FileDrag, Input, Motion, MouseButton, ResizeArgs,
};
use std::collections::VecDeque;
use std::error::Error;
use std::time::Duration;
use std::collections::HashMap;
use glfw::GlfwReceiver as Receiver;
use window::{
    AdvancedWindow, Api, BuildFromWindowSettings, OpenGLWindow, Position, ProcAddress, Size,
    UnsupportedGraphicsApiError, Window, WindowSettings,
};

pub use shader_version::OpenGL;

// list of joysticks to check
const JOYSTICKS: [JoystickId; 16] = [
    JoystickId::Joystick1,
    JoystickId::Joystick2,
    JoystickId::Joystick3,
    JoystickId::Joystick4,
    JoystickId::Joystick5,
    JoystickId::Joystick6,
    JoystickId::Joystick7,
    JoystickId::Joystick8,
    JoystickId::Joystick9,
    JoystickId::Joystick10,
    JoystickId::Joystick11,
    JoystickId::Joystick12,
    JoystickId::Joystick13,
    JoystickId::Joystick14,
    JoystickId::Joystick15,
    JoystickId::Joystick16,
];

/// Contains stuff for game window.
pub struct GlfwWindow {
    /// The window.
    pub window: glfw::PWindow,
    /// Receives events from window.
    events: Receiver<(f64, glfw::WindowEvent)>,
    /// GLFW context.
    pub glfw: glfw::Glfw,
    event_queue: VecDeque<Input>,
    // Used to compute relative mouse movement.
    last_mouse_pos: Option<(f64, f64)>,
    // The back-end does not remember the title.
    title: String,
    exit_on_esc: bool,
    automatic_close: bool,

    /// ignore controller axis inputs below this threshold
    pub joystick_deadzone: f64,
    joysticks: Vec<JoystickHelper>,
}

impl GlfwWindow {
    /// Create a new game window from an existing GLFW window.
    pub fn from_pieces(
        mut win: glfw::PWindow,
        glfw: glfw::Glfw,
        events: Receiver<(f64, glfw::WindowEvent)>,
        exit_on_esc: bool,
    ) -> GlfwWindow {
        win.set_all_polling(true);
        win.make_current();
        let title = "<unknown window title, created from_pieces>";

        // setup joysticks
        let mut joysticks = Vec::new();
        for &i in &JOYSTICKS {
            joysticks.push(JoystickHelper::new(glfw.get_joystick(i)));
        }

        GlfwWindow {
            joysticks,
            window: win,
            events,
            glfw,
            exit_on_esc,
            event_queue: VecDeque::new(),
            last_mouse_pos: None,
            title: title.to_string(),
            automatic_close: true,
            joystick_deadzone: 0.0,
        }
    }

    /// Creates a new game window for GLFW.
    pub fn new(settings: &WindowSettings) -> Result<GlfwWindow, Box<dyn Error>> {
        use glfw::SwapInterval;

        // Initialize GLFW.
        let mut glfw = glfw::init_no_callbacks()?;

        let api = settings
            .get_maybe_graphics_api()
            .unwrap_or(Api::opengl(3, 2));
        if api.api != "OpenGL" {
            return Err(UnsupportedGraphicsApiError {
                found: api.api,
                expected: vec!["OpenGL".into()],
            }
            .into());
        };

        // Make sure we have the right GL version.
        glfw.window_hint(glfw::WindowHint::ContextVersion(api.major, api.minor));
        glfw.window_hint(glfw::WindowHint::Resizable(settings.get_resizable()));
        glfw.window_hint(glfw::WindowHint::Decorated(settings.get_decorated()));
        glfw.window_hint(glfw::WindowHint::TransparentFramebuffer(
            settings.get_transparent(),
        ));
        // Set sRGB.
        glfw.window_hint(glfw::WindowHint::SRgbCapable(settings.get_srgb()));
        if api >= Api::opengl(3, 2) {
            if cfg!(target_os = "macos") {
                glfw.window_hint(glfw::WindowHint::OpenGlForwardCompat(true));
            }
            glfw.window_hint(glfw::WindowHint::OpenGlProfile(
                glfw::OpenGlProfileHint::Core,
            ));
        }
        if settings.get_samples() != 0 {
            glfw.window_hint(glfw::WindowHint::Samples(Some(
                settings.get_samples() as u32
            )));
        }

        // Create GLFW window.
        let (mut window, events) = glfw
            .create_window(
                settings.get_size().width as u32,
                settings.get_size().height as u32,
                &settings.get_title(),
                glfw::WindowMode::Windowed,
            )
            .ok_or("Failed to create GLFW window.")?;
        window.set_all_polling(true);
        window.make_current();

        if settings.get_vsync() {
            glfw.set_swap_interval(SwapInterval::Sync(1));
        } else {
            glfw.set_swap_interval(SwapInterval::None);
        }

        // Load the OpenGL function pointers.
        gl::load_with(|s| window.get_proc_address(s) as *const _);

        // setup joysticks
        let mut joysticks = Vec::new();
        if settings.get_controllers() {
            for &i in &JOYSTICKS {
                joysticks.push(JoystickHelper::new(glfw.get_joystick(i)));
            }
        }

        Ok(GlfwWindow {
            joysticks,
            window,
            events,
            glfw,
            event_queue: VecDeque::new(),
            last_mouse_pos: None,
            title: settings.get_title(),
            exit_on_esc: settings.get_exit_on_esc(),
            automatic_close: settings.get_automatic_close(),
            joystick_deadzone: 0.0,
        })
    }

    fn flush_messages(&mut self) {
        for (_, event) in glfw::flush_messages(&self.events) {
            match event {
                glfw::WindowEvent::Key(glfw::Key::Escape, _, glfw::Action::Press, _)
                    if self.exit_on_esc =>
                {
                    self.window.set_should_close(true);
                }
                glfw::WindowEvent::Close => {
                    if !self.automatic_close {
                        self.window.set_should_close(false);
                    }
                    self.event_queue.push_back(Input::Close(CloseArgs));
                }
                glfw::WindowEvent::Char(ch) => {
                    self.event_queue.push_back(Input::Text(ch.to_string()));
                }
                glfw::WindowEvent::Key(key, scancode, glfw::Action::Press, _) => {
                    self.event_queue.push_back(Input::Button(ButtonArgs {
                        state: ButtonState::Press,
                        button: Button::Keyboard(glfw_map_key(key)),
                        scancode: Some(scancode as i32),
                    }));
                }
                glfw::WindowEvent::Key(key, scancode, glfw::Action::Release, _) => {
                    self.event_queue.push_back(Input::Button(ButtonArgs {
                        state: ButtonState::Release,
                        button: Button::Keyboard(glfw_map_key(key)),
                        scancode: Some(scancode as i32),
                    }));
                }
                glfw::WindowEvent::MouseButton(button, glfw::Action::Press, _) => {
                    self.event_queue.push_back(Input::Button(ButtonArgs {
                        state: ButtonState::Press,
                        button: Button::Mouse(glfw_map_mouse(button)),
                        scancode: None,
                    }));
                }
                glfw::WindowEvent::MouseButton(button, glfw::Action::Release, _) => {
                    self.event_queue.push_back(Input::Button(ButtonArgs {
                        state: ButtonState::Release,
                        button: Button::Mouse(glfw_map_mouse(button)),
                        scancode: None,
                    }));
                }
                glfw::WindowEvent::CursorPos(x, y) => {
                    self.event_queue
                        .push_back(Input::Move(Motion::MouseCursor([x, y])));
                    match self.last_mouse_pos {
                        Some((lx, ly)) => self
                            .event_queue
                            .push_back(Input::Move(Motion::MouseRelative([x - lx, y - ly]))),
                        None => (),
                    }
                    self.last_mouse_pos = Some((x, y));
                }
                glfw::WindowEvent::Scroll(x, y) => {
                    self.event_queue
                        .push_back(Input::Move(Motion::MouseScroll([x, y])));
                }
                glfw::WindowEvent::Size(w, h) => {
                    let draw_size = self.draw_size();
                    self.event_queue.push_back(Input::Resize(ResizeArgs {
                        window_size: [w as f64, h as f64],
                        draw_size: draw_size.into(),
                    }));
                }
                glfw::WindowEvent::Focus(focus) => {
                    self.event_queue.push_back(Input::Focus(focus));
                }
                glfw::WindowEvent::CursorEnter(cursor) => {
                    self.event_queue.push_back(Input::Cursor(cursor));
                }
                glfw::WindowEvent::FileDrop(files) => {
                    for file in files {
                        self.event_queue
                            .push_back(Input::FileDrag(FileDrag::Drop(file)))
                    }
                }
                _ => (),
            }
        }

        // println!("checking gamepads");
        for j in self.joysticks.iter_mut() {
            j.update(&mut self.event_queue, self.joystick_deadzone);
        }
    }

    fn wait_event(&mut self) -> Event {
        loop {
            if self.event_queue.len() == 0 {
                self.glfw.wait_events();
                self.flush_messages();
            }
            if let Some(event) = self.event_queue.pop_front() {
                return Event::Input(event, None);
            }
        }
    }

    fn wait_event_timeout(&mut self, timeout: Duration) -> Option<Event> {
        if self.event_queue.len() == 0 {
            let timeout_secs =
                timeout.as_secs() as f64 + (timeout.subsec_nanos() as f64 / 1_000_000_000.0);
            self.glfw.wait_events_timeout(timeout_secs);
            self.flush_messages();
        }
        self.event_queue.pop_front().map(|x| Event::Input(x, None))
    }

    fn poll_event(&mut self) -> Option<Event> {
        if self.event_queue.len() == 0 {
            self.glfw.poll_events();
            self.flush_messages();
        }
        self.event_queue.pop_front().map(|x| Event::Input(x, None))
    }

    fn capture_cursor(&mut self, enabled: bool) {
        if enabled {
            self.window.set_cursor_mode(glfw::CursorMode::Disabled);
        } else {
            self.window.set_cursor_mode(glfw::CursorMode::Normal);
            self.last_mouse_pos = None;
        }
    }
}

impl BuildFromWindowSettings for GlfwWindow {
    fn build_from_window_settings(settings: &WindowSettings) -> Result<GlfwWindow, Box<dyn Error>> {
        GlfwWindow::new(settings)
    }
}

impl Window for GlfwWindow {
    fn size(&self) -> Size {
        let (w, h) = self.window.get_size();
        Size {
            width: w as f64,
            height: h as f64,
        }
    }

    fn draw_size(&self) -> Size {
        let (w, h) = self.window.get_framebuffer_size();
        Size {
            width: w as f64,
            height: h as f64,
        }
    }

    fn set_should_close(&mut self, value: bool) {
        self.window.set_should_close(value);
    }

    fn should_close(&self) -> bool {
        self.window.should_close()
    }

    fn swap_buffers(&mut self) {
        self.window.swap_buffers()
    }

    fn wait_event(&mut self) -> Event {
        self.wait_event()
    }

    fn wait_event_timeout(&mut self, timeout: Duration) -> Option<Event> {
        self.wait_event_timeout(timeout)
    }

    fn poll_event(&mut self) -> Option<Event> {
        self.poll_event()
    }
}

impl AdvancedWindow for GlfwWindow {
    fn get_title(&self) -> String {
        self.title.clone()
    }

    fn set_title(&mut self, value: String) {
        self.window.set_title(&value)
    }

    fn get_automatic_close(&self) -> bool {
        self.automatic_close
    }

    fn set_automatic_close(&mut self, value: bool) {
        self.automatic_close = value;
    }

    fn get_exit_on_esc(&self) -> bool {
        self.exit_on_esc
    }

    fn set_exit_on_esc(&mut self, value: bool) {
        self.exit_on_esc = value
    }

    fn set_capture_cursor(&mut self, value: bool) {
        self.capture_cursor(value)
    }

    fn show(&mut self) {
        self.window.show();
    }

    fn hide(&mut self) {
        self.window.hide();
    }

    fn get_position(&self) -> Option<Position> {
        let (x, y) = self.window.get_pos();
        Some(Position { x: x, y: y })
    }

    fn set_position<P: Into<Position>>(&mut self, pos: P) {
        let pos: Position = pos.into();
        self.window.set_pos(pos.x, pos.y);
    }

    fn set_size<S: Into<Size>>(&mut self, size: S) {
        let size: Size = size.into();
        self.window.set_size(size.width as i32, size.height as i32);
    }
}

impl OpenGLWindow for GlfwWindow {
    fn get_proc_address(&mut self, proc_name: &str) -> ProcAddress {
        self.window.get_proc_address(proc_name) as *const _
    }

    fn is_current(&self) -> bool {
        self.window.is_current()
    }

    fn make_current(&mut self) {
        self.window.make_current()
    }
}

fn glfw_map_key(keycode: glfw::Key) -> keyboard::Key {
    use input::Key;

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
        glfw::Key::GraveAccent => Key::Backquote,
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
        glfw::Key::Unknown => Key::Unknown,
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

/// helper struct for joystick
struct JoystickHelper {
    /// joystick to check
    joystick: Joystick,

    // states
    buttons: HashMap<u8, bool>,
    axes: HashMap<u8, f64>,
    /// last known connected state
    connected: bool,
}
impl JoystickHelper {
    fn new(joystick: Joystick) -> Self {
        Self {
            joystick,
            connected: false,
            buttons: HashMap::new(),
            axes: HashMap::new(),
        }
    }

    fn update(&mut self, event_queue: &mut VecDeque<Input>, deadzone: f64) {
        match (self.joystick.is_present(), self.connected) {
            // not connected, and we know its not connected
            (false, false) => return,

            // was disconnected since last update
            (false, true) => {
                self.connected = false;
                // clear maps to free up memory
                self.buttons.clear();
                self.axes.clear();
                return;
            }

            // was connected since last update
            (true, false) => {
                // insert values
                self.connected = true;

                // only issue with this approach is a skipped input on the update the controller is connected
                // i dont think this is a big issue though
                for (axis, a) in self.joystick.get_axes().iter().enumerate() {
                    self.axes.insert(axis as u8, *a as f64);
                }
                for (button, a) in self.joystick.get_buttons().iter().enumerate() {
                    self.buttons.insert(button as u8, *a > 1);
                }

                // exit
                return;
            }

            // still connected, nothing to do
            (true, true) => {}
        }

        // check axes
        for (axis, a) in self.joystick.get_axes().iter().enumerate() {
            let previous = self.axes.get_mut(&(axis as u8)).unwrap();
            let a = *a as f64;

            if a == *previous || a.abs() < deadzone {
                // if the value is the same, or within the deadzone, dont do an update
                continue;
            } else {
                // new value, update existing value
                *previous = a
            }

            // add change as event
            event_queue.push_back(Input::Move(Motion::ControllerAxis(
                ControllerAxisArgs::new(self.joystick.id as u32, axis as u8, a),
            )));
        }

        // check buttons
        for (button, a) in self.joystick.get_buttons().iter().enumerate() {
            let previous = self.buttons.get_mut(&(button as u8)).unwrap();
            let pressed = *a > 0;

            if pressed == *previous {
                // if the value is the same, dont do an update
                continue;
            } else {
                // new value, update existing value
                *previous = pressed
            }

            // add change as event
            event_queue.push_back(Input::Button(ButtonArgs {
                state: if pressed {
                    ButtonState::Press
                } else {
                    ButtonState::Release
                },
                button: Button::Controller(ControllerButton::new(
                    self.joystick.id as u32,
                    button as u8,
                )),
                scancode: None,
            }));
        }
    }
}
