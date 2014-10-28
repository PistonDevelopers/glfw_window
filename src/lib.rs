#![deny(missing_doc)]

//! A GLFW window back-end for the Piston game engine.

extern crate glfw;
extern crate collections;
extern crate gl;
extern crate event;
extern crate shader_version;
extern crate input;

pub use window_glfw::WindowGLFW;

mod window_glfw;

