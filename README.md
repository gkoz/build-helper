# build-helper

A build library for embedding GTK resources. Requires `glib-compile-resources`
utility.

`Cargo.toml`:

```toml
[package]
build = "build.rs"

[build-dependencies.build-helper]
git = "https://github.com/gkoz/build-helper.git"

[dependencies.glib]
version = "0.0.9"

[dependencies.gio]
version = "0.0.1"
```

`build.rs`:

```rust
extern crate build_helper;
use std::process;

fn main() {
    if let Err(error) = build_helper::resources::compile("exampleapp") {
        println!("{}", error);
        process::exit(1);
    }
}
```

`main.rs`:

```rust
extern crate gtk;

mod resources {
    include!(concat!(env!("OUT_DIR"), "/exampleapp_resources.rs"));
}

fn main() {
    gtk::init().unwrap();
    resources::register();
    let builder = gtk::Builder::new_from_resource("/org/gtk/exampleapp/window.ui");
}
```

`exampleapp.gresource.xml`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<gresources>
  <gresource prefix="/org/gtk/exampleapp">
    <file preprocess="xml-stripblanks">window.ui</file>
  </gresource>
</gresources>
```
