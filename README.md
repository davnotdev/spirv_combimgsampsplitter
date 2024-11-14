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
layout(set = 0, binding = 2) uniform texture2D u_texture_array[];
layout(set = 0, binding = 3) uniform sampler u_sampler_array[];
```

> Enjoy!

## Notes on WGSL Translation

### Naga

| Test                | Status | Notes                                                                                                                            |
| ------------------- | ------ | -------------------------------------------------------------------------------------------------------------------------------- |
| `test.frag`         | ✅     |                                                                                                                                  |
| `test_nested.frag`  | ✅     |                                                                                                                                  |
| `test_arrayed.frag` | 🆗     | [#1](https://github.com/davnotdev/spirv_combimgsampsplitter/issues/1) Requires simple mod                                        |
| `test_mixed.frag`   | ❌     | [#1](https://github.com/davnotdev/spirv_combimgsampsplitter/issues/1) and [WGPU #6523](https://github.com/gfx-rs/wgpu/pull/6523) |                |

### Tint

| Test                | Status | Notes                        |
| ------------------- | ------ | ---------------------------- |
| `test.frag`         | ✅     |                              |
| `test_nested.frag`  | ✅     |                              |
| `test_arrayed.frag` | ❌     | Binding Arrays Not Suppprted |
| `test_mixed.frag`   | ❌     | Binding Arrays Not Supported |

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
