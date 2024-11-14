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
layout(set = 0, binding = 1) uniform sampler2DArray u_texture_array;

// is converted into...

layout(set = 0, binding = 0) uniform texture2D u_texture;
layout(set = 0, binding = 1) uniform sampler u_sampler;

// *texture2DArray doesn't exist in glsl, but in wgsl, this would be texture_2d_array<f32>
layout(set = 0, binding = 2) uniform texture2DArray u_texture_array;
layout(set = 0, binding = 3) uniform sampler u_sampler;
```

> Enjoy!

## Notes on WGSL Translation

### Naga

| Test                | Status |
| ------------------- | ------ |
| `test.frag`         | ✅     |
| `test_nested.frag`  | ✅     |
| `test_arrayed.frag` | ✅     |
| `test_mixed.frag`   | ✅     |

### Tint

| Test                | Status |
| ------------------- | ------ |
| `test.frag`         | ✅     |
| `test_nested.frag`  | ✅     |
| `test_arrayed.frag` | ✅     |
| `test_mixed.frag`   | ✅     |

## Notes

- Translating `sampler2D[N]` and `sampler2DArray[N]` is NOT supported.
- After being split, the SPIR-V will not translate back to GLSL "one-to-one", the translation back to GLSL using either `naga` or `tint` creates a combined image sampler!
- Do NOT use older versions of this crate, they are buggy.

## Library Usage

Add this to your `Cargo.toml`:

```
spirv_combimgsampsplitter = "0.3"
```

```rust
let spv_bytes: Vec<u8> = fs::read("in.spv").unwrap();

let spv: Vec<u32> = spirv_combimgsampsplitter::u8_slice_to_u32_vec(&spv_bytes);
let out_spv: Vec<u32> = spirv_combimgsampsplitter::combimgsampsplitter(&spv).unwrap();

let out_spv_bytes = spirv_combimgsampsplitter::u32_slice_to_u8_vec(&out_spv);
fs::write("out.spv", out_spv_bytes).unwrap();
```

## CLI Usage

```bash
cargo install spirv_combimgsampsplitter
spirv_combimgsampsplitter in.spv out.spv
# or
git clone https://github.com/davnotdev/spirv_combimgsampsplitter
cd spirv_combimgsampsplitter
cargo r -- in.spv out.spv
```
