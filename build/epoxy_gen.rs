// Copyright 2015 Brendan Zabarauskas and the gl-rs developers
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::io;

use gl_generator::{Cmd, Generator, Registry};
use gl_generator::generators::{gen_enum_item, gen_parameters, gen_symbol_name, gen_types};

#[allow(missing_copy_implementations)]
pub struct EpoxyGenerator;

impl Generator for EpoxyGenerator {
    fn write<W>(&self, registry: &Registry, dest: &mut W) -> io::Result<()>
        where W: io::Write
    {
        write_header(dest)?;
        write_metaloadfn(dest)?;
        write_type_aliases(registry, dest)?;
        write_enums(registry, dest)?;
        write_fns(registry, dest)?;
        write_fnptr_struct_def(dest)?;
        write_ptrs(registry, dest)?;
        write_fn_mods(registry, dest)?;
        write_error_fns(dest)?;
        write_load_fn(registry, dest)?;
        write_get_proc_addr(registry, dest)?;
        Ok(())
    }
}

/// Creates a `__gl_imports` module which contains all the external symbols that we need for the
///  bindings.
fn write_header<W>(dest: &mut W) -> io::Result<()>
    where W: io::Write
{
    writeln!(dest,
             r#"
        mod __gl_imports {{
            pub extern crate libc;
            pub use std::mem;
            pub use std::ptr;
            pub use std::os::raw;
            pub use std::process::exit;
        }}
    "#)
}

/// Creates the metaloadfn function for fallbacks
fn write_metaloadfn<W>(dest: &mut W) -> io::Result<()> where W: io::Write {
    writeln!(dest, r#"
        fn metaloadfn<F>(mut loadfn: F,
                         symbol: &str,
                         fallbacks: &[&str]) -> *const *const __gl_imports::raw::c_void
                         where F: FnMut(&str) -> *const __gl_imports::raw::c_void {{
            let mut ptr = loadfn(symbol);
            if ptr.is_null() {{
                for &sym in fallbacks.iter() {{
                    ptr = loadfn(sym);
                    if !ptr.is_null() {{ break; }}
                }}
            }}
            ptr as *const *const __gl_imports::raw::c_void
        }}
    "#)
}

/// Creates a `types` module which contains all the type aliases.
///
/// See also `generators::gen_types`.
fn write_type_aliases<W>(registry: &Registry, dest: &mut W) -> io::Result<()>
    where W: io::Write
{
    writeln!(dest,
                  r#"
        pub mod types {{
            #![allow(non_camel_case_types, non_snake_case, dead_code, missing_copy_implementations)]
    "#)?;

    gen_types(registry.api, dest)?;

    writeln!(dest,
             "
        }}
    ")
}

/// Creates all the `<enum>` elements at the root of the bindings.
fn write_enums<W>(registry: &Registry, dest: &mut W) -> io::Result<()>
    where W: io::Write
{
    for enm in &registry.enums {
        gen_enum_item(enm, "types::", dest)?;
    }

    Ok(())
}

/// Creates the functions corresponding to the GL commands.
///
/// The function calls the corresponding function pointer stored in the `storage` module created
///  by `write_ptrs`.
fn write_fns<W>(registry: &Registry, dest: &mut W) -> io::Result<()> where W: io::Write {
    for c in &registry.cmds {
        if let Some(v) = registry.aliases.get(&c.proto.ident) {
            writeln!(dest, "/// Fallbacks: {}", v.join(", "))?;
        }

        writeln!(dest, r#"
            #[allow(non_snake_case, unused_variables, dead_code)] #[inline]
            pub unsafe fn {name}({params}) -> {return_suffix} {{
                __gl_imports::mem::transmute::<_, extern "system" fn({typed_params}) -> {return_suffix}>
                    (*storage::{name}.pf)({idents})
            }}"#,
            name = c.proto.ident,
            params = gen_parameters(c, true, true).join(", "),
            typed_params = gen_parameters(c, false, true).join(", "),
            return_suffix = gen_return_type(c),
            idents = gen_parameters(c, true, false).join(", "),
        )?;
    }

    Ok(())
}

fn gen_return_type(cmd: &Cmd) -> String {
    // turn the return type into a Rust type
    let ty = &cmd.proto.ty;

    // ... but there is one more step: if the Rust type is `c_void`, we replace it with `()`
    if ty == "__gl_imports::raw::c_void" {
        return "()".to_string();
    }

    ty.to_string()
}

/// Creates a `FnPtr` structure which contains the store for a single binding.
fn write_fnptr_struct_def<W>(dest: &mut W) -> io::Result<()> where W: io::Write {
    writeln!(dest, "
        #[allow(missing_copy_implementations)]
        pub struct FnPtr {{
            /// Pointer to the entry in libepoxy's dispatch table
            pf: *const *const __gl_imports::raw::c_void,
            /// True if the pointer points to a real function, false if points to an error fn
            is_loaded: bool,
        }}
        impl FnPtr {{
            /// Creates a `FnPtr` from a load attempt.
            pub fn new(ptr: *const *const __gl_imports::raw::c_void) -> FnPtr {{
                if ptr.is_null() {{
                    FnPtr {{
                        pf: &PMISSING_FN_EXIT,
                        is_loaded: false,
                    }}
                }} else {{
                    FnPtr {{ pf: ptr, is_loaded: true }}
                }}
            }}
        }}
    ")
}

/// Creates a `storage` module which contains a static `FnPtr` per GL command in the registry.
fn write_ptrs<W>(registry: &Registry, dest: &mut W) -> io::Result<()> where W: io::Write {
    writeln!(dest,
        "mod storage {{
            #![allow(non_snake_case, non_upper_case_globals)]
            use super::PMISSING_FN_EXIT;
            use super::FnPtr;")?;

    for c in &registry.cmds {
        writeln!(dest,
            "pub static mut {name}: FnPtr = FnPtr {{
                pf: &PMISSING_FN_EXIT,
                is_loaded: false,
            }};",
            name = c.proto.ident,
        )?;
    }

    writeln!(dest, "}}")
}

/// Creates one module for each GL command.
///
/// Each module contains `is_loaded` and `load_with` which interact with the `storage` module
///  created by `write_ptrs`.
fn write_fn_mods<W>(registry: &Registry, dest: &mut W) -> io::Result<()> where W: io::Write {
    for c in &registry.cmds {
        let fallbacks = match registry.aliases.get(&c.proto.ident) {
            Some(v) => {
                let names = v.iter().map(|name| format!(r#""{}""#,
                    gen_symbol_name(registry.api, &name[..]))).collect::<Vec<_>>();
                format!("&[{}]", names.join(", "))
            },
            None => "&[]".to_string(),
        };
        let fnname = &c.proto.ident[..];
        let symbol = gen_symbol_name(registry.api, &c.proto.ident[..]);
        let symbol = &symbol[..];

        writeln!(dest, r#"
            #[allow(non_snake_case)]
            pub mod {fnname} {{
                use super::{{storage, metaloadfn}};
                use super::FnPtr;
                #[inline]
                #[allow(dead_code)]
                pub fn is_loaded() -> bool {{
                    unsafe {{ storage::{fnname}.is_loaded }}
                }}
                #[allow(dead_code)]
                pub fn load_with<F>(loadfn: F) where F: FnMut(&str) -> *const super::__gl_imports::raw::c_void {{
                    unsafe {{
                        storage::{fnname} = FnPtr::new(metaloadfn(loadfn, "epoxy_{symbol}", {fallbacks}))
                    }}
                }}
            }}"#,
            fnname = fnname,
            fallbacks = fallbacks,
            symbol = symbol,
        )?;
    }

    Ok(())
}

/// Creates a `missing_fn_exit` function.
///
/// This function is the mock that is called if the real function could not be called.
fn write_error_fns<W>(dest: &mut W) -> io::Result<()> where W: io::Write {
    writeln!(dest, r#"
        #[inline(never)]
        extern fn missing_fn_exit() {{
            println!("function was not loaded");
            __gl_imports::exit(1);
        }}
        const PMISSING_FN_EXIT: *const __gl_imports::raw::c_void = missing_fn_exit as *const __gl_imports::raw::c_void;"#,
    )
}

/// Creates the `load_with` function.
///
/// The function calls `load_with` in each module created by `write_fn_mods`.
fn write_load_fn<W>(registry: &Registry, dest: &mut W) -> io::Result<()> where W: io::Write {
    writeln!(dest, r#"
        #[allow(dead_code)]
        pub fn load_with<F>(mut loadfn: F) where F: FnMut(&str) -> *const __gl_imports::raw::c_void {{
    "#)?;

    for c in &registry.cmds {
        writeln!(dest, "{name}::load_with(|s| loadfn(s));",
                      name = &c.proto.ident[..])?;
    }

    writeln!(dest, "
        }}
    ")
}

/// Creates the `get_proc_addr` function.
///
/// The function adds in a layer of indirection, but allows compatibility with the `gl` crate
fn write_get_proc_addr<W>(registry: &Registry, dest: &mut W) -> io::Result<()> where W: io::Write {
    writeln!(dest, r#"
        #[allow(dead_code)]
        pub fn get_proc_addr(symbol: &str) -> *const __gl_imports::raw::c_void {{
            match &symbol[..] {{
    "#)?;

    for c in &registry.cmds {
        writeln!(dest, r#"
            "{symbol}" => {name} as *const __gl_imports::raw::c_void,"#,
            symbol = gen_symbol_name(registry.api, &c.proto.ident[..]),
            name = &c.proto.ident[..],
        )?;
    }

    writeln!(dest, r#"
                _ => __gl_imports::ptr::null(),
            }}
        }}
    "#)
}
