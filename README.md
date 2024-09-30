# SPIRV Combined Image Sampler Splitter

[![Version Badge](https://img.shields.io/crates/v/spirv_combimgsampsplitter)](https://crates.io/crates/spirv_combimgsampsplitter)
[![Docs Badge](https://img.shields.io/docsrs/spirv_combimgsampsplitter/latest)](https://docs.rs/spirv_combimgsampsplitter/latest/spirv_combimgsampsplitter/)
[![License Badge](https://img.shields.io/crates/l/spirv_combimgsampsplitter)](LICENSE)
[![Downloads Badge](https://img.shields.io/crates/d/spirv_combimgsampsplitter)](https://crates.io/crates/spirv_combimgsampsplitter)

It is commonly known that [WebGpu does not support combined image samplers](https://github.com/gpuweb/gpuweb/issues/770).
This makes adding WebGpu support for existing OpenGL or Vulkan renderers impossible without workarounds.
This is one such workaround.
By reading and modifying SPIRV byte code, combined image samplers can be split into their respective texture and sampler.
Special edge cases such as the use of combined image samplers in function parameters and nested functions are also handled.

```glsl
layout(set = 0, binding = 0) uniform sampler2D u_texture;

// is converted into...

layout(set = 0, binding = 1) uniform sampler u_sampler;
layout(set = 0, binding = 2) uniform texture2D u_texture;
```

> Enjoy!

## Library Usage

```rust
let spv_bytes: Vec<u8> = fs::read("in.spv").unwrap();

let spv: Vec<u32> = spirv_combimgsampsplitter::u8_slice_to_u32_vec(&spv_bytes);
let out_spv: Vec<u32> = spirv_combimgsampsplitter::combimgsampsplitter(&spv).unwrap();

let out_spv_bytes = spirv_combimgsampsplitter::u32_slice_to_u8_vec(&out_spv);
fs::write("out.spv", out_spv_bytes).unwrap();
```

## CLI Usage

```bash
spirv_combimgsampsplitter in.spv out.spv
# or
cargo r -- in.spv out.spv
```

