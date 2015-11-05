# epoxy-rs

Rust bindings for `libepoxy`, an OpenGL function pointer manager.

Bindings generated using a custom generator for the `gl_generator` crate.

## Why would I use this over the `gl` crate?

You probably shouldn't, `gl` does the same type of function pointer management
but requires a `get_proc_address` or similar kind of function to locate the
needed function pointers. This isn't a problem when using a library such as
`glutin` for window management, but is a problem when interacting with a Gdk
`GdkGLContext`. Using `gl` together with `epoxy` with the default generators
causes issues, as both libraries attempt to lazily resolve symbols and cache
function pointer values.

This crate is mainly useful for using OpenGL functions to draw to a `GtkGLArea`
together with the `gtk` crate. Gdk uses epoxy in the background to set up an
OpenGL context for you, but doesn't provide an easy way to locate the proper
OpenGL functions after initialization. This crate attempts to address this
issue.

## Using the bindings

`epoxy` currently requires a loader function similar to the `gl` crate, except
instead of locating the raw OpenGL functions it locates the equivalent
`libepoxy` symbols, preventing the conflicts described above. Code using the
`shared_library` crate like the following should work in most cases:

```
epoxy::load_with(|s| {
    unsafe {
        match DynamicLibrary::open(None).unwrap().symbol(s) {
            Ok(v) => v,
            Err(_) => ptr::null(),
        }
    }
});
gl::load_with(epoxy::get_proc_addr);
```

A future version of the library may default to using this implementation.

## Can I draw to a `GtkGLArea` using `glium`?

Hopefully soon.

## The bindings are missing some functions!

Try upgrading to libepoxy 1.3.1, current version in the Ubuntu repository is
1.2. Grabbing the xenial debs from
https://launchpad.net/ubuntu/xenial/amd64/libepoxy0/1.3.1-1 worked for me.
