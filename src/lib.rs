//! Gtk-rs build helpers.
//!
//! This library is intended to be used in build scripts to assist in building
//! GTK apps. It should only be added as a cargo 'build-dependency'.
//!
//! Default features:
//!
//! * `resources`: compiling and embedding GLib resources.

/// Compiles and prepares GLib resources for embedding into an application.
#[cfg(feature = "resources")]
pub mod resources;
