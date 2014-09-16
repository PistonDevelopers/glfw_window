# glfw_game_window [![Build Status](https://travis-ci.org/PistonDevelopers/glfw_game_window.svg)](https://travis-ci.org/PistonDevelopers/glfw_game_window)

A GLFW window back-end for the Piston game engine.

Maintainers: @TyOverby, @bvssvni, @Coeuvre

### How to create a window

```Rust
let mut window = WindowGLFW::new(
    piston::shader_version::opengl::OpenGL_3_2,
    piston::WindowSettings {
        title: "My application".to_string(),
        size: [640, 480],
        fullscreen: false,
        exit_on_esc: true,
        samples: 4,
    }
);
```

### How to set up Gfx

After you have created a window, do the following:

```Rust
let mut device = gfx::GlDevice::new(|s|
    self.window.get_proc_address(s)
);
let (w, h) = window.get_size();
let frame = gfx::Frame::new(w as u16, h as u16);
```
