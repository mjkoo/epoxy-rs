// Based heavily on StaticStructGenerator from the gl_generator crate, original copyright below:
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
struct EpoxyStructGenerator;

impl gl_generator::generators::Generator for EpoxyStructGenerator {
    fn write<W>(&self, registry: &Registry, ns: Ns, dest: &mut W) -> io::Result<()> where W: io::Write {
        try!(write_header(dest));
        try!(write_type_aliases(&ns, dest));
        try!(write_enums(registry, dest));
        try!(write_struct(&ns, dest));
        try!(write_impl(registry, &ns, dest));
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
}}"#
    )
}

/// Creates a `types` module which contains all the type aliases.
///
/// See also `generators::gen_type_aliases`.
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

/// Creates a stub structure.
///
/// The name of the struct corresponds to the namespace.
fn write_struct<W>(ns: &Ns, dest: &mut W) -> io::Result<()> where W: io::Write {
    writeln!(dest, r#"
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[allow(dead_code)]
#[derive(Copy, Clone)]
pub struct {ns};"#,
        ns = ns.fmt_struct_name(),
    )
}

/// Creates the `impl` of the structure created by `write_struct`.
fn write_impl<W>(registry: &Registry, ns: &Ns, dest: &mut W) -> io::Result<()> where W: io::Write {
    try!(writeln!(dest, r#"impl {ns} {{"#, ns = ns.fmt_struct_name()));

    for c in registry.cmd_iter() {
        try!(writeln!(dest, r#"
#[allow(non_snake_case, unused_variables, dead_code)] #[inline]
pub unsafe fn {name}(&self, {typed_params}) -> {return_suffix} {{
    __gl_imports::mem::transmute::<_, extern "system" fn({typed_params}) -> {return_suffix}>(epoxy_{name})({idents})
}}"#,
            name = c.proto.ident,
            typed_params = gl_generator::generators::gen_parameters(c, true, true).join(", "),
            return_suffix = gl_generator::generators::gen_return_type(c),
            idents = gl_generator::generators::gen_parameters(c, true, false).join(", "),
        ));
    }

    writeln!(dest, r#"}}"#)
}

/// io::Writes all functions corresponding to the GL bindings.
///
/// These are foreign functions, they don't have any content.
fn write_fns<W>(registry: &Registry, ns: &Ns, dest: &mut W) -> io::Result<()> where W: io::Write {
    try!(writeln!(dest, r#"
#[allow(non_snake_case)]
#[allow(unused_variables)]
#[allow(dead_code)]
extern "system" {{"#
    ));

    for c in registry.cmd_iter() {
        try!(writeln!(dest, r#"
#[link_name="epoxy_{symbol}"] static epoxy_{name}: *const *const __gl_imports::libc::c_void;"#,
            symbol = gl_generator::generators::gen_symbol_name(ns, &c.proto.ident),
            name = c.proto.ident,
        ));
    }

    writeln!(dest, r#"}}"#)
}

pub fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest = Path::new(&out_dir);

    let mut file = BufWriter::new(File::create(&dest.join("bindings.rs")).unwrap());

    // From glium build/main.rs
    let extensions = vec![
        "GL_AMD_depth_clamp_separate".to_string(),
        "GL_APPLE_vertex_array_object".to_string(),
        "GL_ARB_bindless_texture".to_string(),
        "GL_ARB_buffer_storage".to_string(),
        "GL_ARB_compute_shader".to_string(),
        "GL_ARB_copy_buffer".to_string(),
        "GL_ARB_debug_output".to_string(),
        "GL_ARB_depth_texture".to_string(),
        "GL_ARB_direct_state_access".to_string(),
        "GL_ARB_draw_buffers".to_string(),
        "GL_ARB_ES2_compatibility".to_string(),
        "GL_ARB_ES3_compatibility".to_string(),
        "GL_ARB_ES3_1_compatibility".to_string(),
        "GL_ARB_ES3_2_compatibility".to_string(),
        "GL_ARB_framebuffer_sRGB".to_string(),
        "GL_ARB_geometry_shader4".to_string(),
        "GL_ARB_gpu_shader_fp64".to_string(),
        "GL_ARB_gpu_shader_int64".to_string(),
        "GL_ARB_invalidate_subdata".to_string(),
        "GL_ARB_multi_draw_indirect".to_string(),
        "GL_ARB_occlusion_query".to_string(),
        "GL_ARB_pixel_buffer_object".to_string(),
        "GL_ARB_robustness".to_string(),
        "GL_ARB_shader_image_load_store".to_string(),
        "GL_ARB_shader_objects".to_string(),
        "GL_ARB_texture_buffer_object".to_string(),
        "GL_ARB_texture_float".to_string(),
        "GL_ARB_texture_multisample".to_string(),
        "GL_ARB_texture_rg".to_string(),
        "GL_ARB_texture_rgb10_a2ui".to_string(),
        "GL_ARB_transform_feedback3".to_string(),
        "GL_ARB_vertex_buffer_object".to_string(),
        "GL_ARB_vertex_shader".to_string(),
        "GL_ATI_draw_buffers".to_string(),
        "GL_ATI_meminfo".to_string(),
        "GL_EXT_debug_marker".to_string(),
        "GL_EXT_direct_state_access".to_string(),
        "GL_EXT_framebuffer_blit".to_string(),
        "GL_EXT_framebuffer_multisample".to_string(),
        "GL_EXT_framebuffer_object".to_string(),
        "GL_EXT_framebuffer_sRGB".to_string(),
        "GL_EXT_gpu_shader4".to_string(),
        "GL_EXT_packed_depth_stencil".to_string(),
        "GL_EXT_provoking_vertex".to_string(),
        "GL_EXT_texture_array".to_string(),
        "GL_EXT_texture_buffer_object".to_string(),
        "GL_EXT_texture_compression_s3tc".to_string(),
        "GL_EXT_texture_filter_anisotropic".to_string(),
        "GL_EXT_texture_integer".to_string(),
        "GL_EXT_texture_sRGB".to_string(),
        "GL_EXT_transform_feedback".to_string(),
        "GL_GREMEDY_string_marker".to_string(),
        "GL_KHR_robustness".to_string(),
        "GL_NVX_gpu_memory_info".to_string(),
        "GL_NV_conditional_render".to_string(),
        "GL_NV_vertex_attrib_integer_64bit".to_string(),
    ];

    gl_generator::generate_bindings(EpoxyStructGenerator,
                                    gl_generator::registry::Ns::Gl,
                                    gl_generator::Fallbacks::None,
                                    khronos_api::GL_XML, extensions,
                                    "4.5", "compatibility",
                                    &mut file).unwrap();
}
