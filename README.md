# epoxy-rs

Rust bindings for `libepoxy`, an OpenGL function pointer manager.

Bindings generated using a custom generator for the `gl_generator` crate.

## Why would I use this over the `gl` crate?

You probably shouldn't, `gl` does the same type of function pointer management
but requires a `get_proc_address` or similar kind of function to locate the
needed function pointers. This isn't a problem when using a library such as
`glutin` for window management, but is a problem when interacting with a Gdk
`GdkGLContext`. Using `gl` together with `epoxy` (such as writing a wrapper
which `dlsym`'s the appropriate symbols and returns the `epoxy_*` versions)
causes issues, as both libraries attempt to lazily resolve symbols and cache
function pointer values.

This crate is mainly useful for using OpenGL functions to draw to a `GtkGLArea`
together with the `gtk` crate. Gdk uses epoxy in the background to set up an
OpenGL context for you, but doesn't provide an easy way to locate the proper
OpenGL functions after initialization. This crate attempts to address this
issue.

## Can I draw to a `GtkGLArea` using `glium`?

Hopefully soon.
