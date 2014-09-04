#![crate_name = "glfw_game_window"]
#![deny(missing_doc)]

//! A GLFW window back-end for the Piston game engine.

extern crate gfx;
extern crate piston;
extern crate glfw;
extern crate collections;
extern crate gl;

pub use window_glfw::WindowGLFW;

mod window_glfw;

