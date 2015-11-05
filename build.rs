// Based heavily on StaticGenerator from the gl_generator crate, original copyright below:
//
// Copyright 2013-2014 The gl-rs developers. For a full listing of the authors,
// refer to the AUTHORS file at the top-level directory of this distribution.
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

extern crate gl_generator;
extern crate khronos_api;

use std::env;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use gl_generator::registry::{Registry, Ns};
use std::io;

#[allow(missing_copy_implementations)]
struct EpoxyGenerator;

impl gl_generator::generators::Generator for EpoxyGenerator {
    fn write<W>(&self, registry: &Registry, ns: Ns, dest: &mut W) -> io::Result<()> where W: io::Write {
        try!(write_header(dest));
        try!(write_type_aliases(&ns, dest));
        try!(write_enums(registry, dest));
        try!(write_get_proc_address(registry, &ns, dest));
        try!(write_fns(registry, &ns, dest));
        Ok(())
    }
}

/// Creates a `__gl_imports` module which contains all the external symbols that we need for the
///  bindings.
fn write_header<W>(dest: &mut W) -> io::Result<()> where W: io::Write {
    writeln!(dest, r#"
mod __gl_imports {{
    extern crate libc;
    pub use std::mem;
    pub use std::ptr;
}}"#
    )
}

/// Creates a `types` module which contains all the type aliases.
///
/// See also `gl_generator::generators::gen_type_aliases`.
fn write_type_aliases<W>(ns: &Ns, dest: &mut W) -> io::Result<()> where W: io::Write {
    try!(writeln!(dest, r#"
pub mod types {{
    #![allow(non_camel_case_types)]
    #![allow(non_snake_case)]
    #![allow(dead_code)]
    #![allow(missing_copy_implementations)]"#
    ));

    try!(gl_generator::generators::gen_type_aliases(ns, dest));

    writeln!(dest, r#"}}"#)
}

/// Creates all the `<enum>` elements at the root of the bindings.
fn write_enums<W>(registry: &Registry, dest: &mut W) -> io::Result<()> where W: io::Write {
    for e in registry.enum_iter() {
        try!(gl_generator::generators::gen_enum_item(e, "types::", dest));
    }

    Ok(())
}

/// Creates all the functions corresponding to the GL bindings.
fn write_get_proc_address<W>(registry: &Registry, ns: &Ns, dest: &mut W) -> io::Result<()> where W: io::Write {
    try!(writeln!(dest, r#"
pub fn get_proc_address(symbol: &str) -> *const __gl_imports::libc::c_void {{
    unsafe {{
        match &symbol[..] {{"#
    ));

    for c in registry.cmd_iter() {
        try!(writeln!(dest, r#"
        "{symbol}" => __gl_imports::mem::transmute::<_, *const __gl_imports::libc::c_void>({name}),"#,
            symbol = gl_generator::generators::gen_symbol_name(ns, &c.proto.ident),
            name = c.proto.ident,
        ));
    }

    try!(writeln!(dest, r#"_ => __gl_imports::ptr::null(),"#));

    writeln!(dest, r#"}}}}}}"#)
}

/// Creates all the functions corresponding to the GL bindings.
fn write_fns<W>(registry: &Registry, ns: &Ns, dest: &mut W) -> io::Result<()> where W: io::Write {
    for c in registry.cmd_iter() {
        try!(writeln!(dest, r#"
#[allow(non_snake_case, unused_variables, dead_code)]
pub extern fn {name}({typed_params}) -> {return_suffix} {{
    unsafe {{
        __gl_imports::mem::transmute::<_, extern "system" fn({typed_params}) -> {return_suffix}>(ffi::epoxy_{symbol})({idents})
    }}
}}"#,
            name = c.proto.ident,
            symbol = gl_generator::generators::gen_symbol_name(ns, &c.proto.ident),
            typed_params = gl_generator::generators::gen_parameters(c, true, true).join(", "),
            return_suffix = gl_generator::generators::gen_return_type(c),
            idents = gl_generator::generators::gen_parameters(c, true, false).join(", "),
        ));
    }

    try!(writeln!(dest, r#"
#[allow(non_snake_case)]
#[allow(unused_variables)]
#[allow(dead_code)]
mod ffi {{
    use __gl_imports;
    extern "system" {{"#
    ));

    for c in registry.cmd_iter() {
        try!(writeln!(dest, r#"
#[link_name="epoxy_{symbol}"] pub static epoxy_{symbol}: *const *const __gl_imports::libc::c_void;"#,
            symbol = gl_generator::generators::gen_symbol_name(ns, &c.proto.ident),
        ));
    }

    writeln!(dest, r#"}}}}"#)
}

pub fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest = Path::new(&out_dir);

    let mut file = BufWriter::new(File::create(&dest.join("bindings.rs")).unwrap());

    gl_generator::generate_bindings(EpoxyGenerator,
                                    gl_generator::registry::Ns::Gl,
                                    gl_generator::Fallbacks::None,
                                    khronos_api::GL_XML, vec![],
                                    "4.5", "core",
                                    &mut file).unwrap();
}
