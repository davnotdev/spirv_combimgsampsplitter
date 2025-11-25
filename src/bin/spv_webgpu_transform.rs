use std::{env, fs, process};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 4 {
        eprintln!("Usage: spv_webgpu_transform <combimg|dref> <input.spv> <output.spv>");
        process::exit(1);
    }

    let mode = &args[1];
    let input_path = &args[2];
    let output_path = &args[3];
    let spv_bytes = fs::read(input_path).unwrap();

    let spv = spirv_webgpu_transform::u8_slice_to_u32_vec(&spv_bytes);

    let out_spv = match mode.as_str() {
        "combimg" => spirv_webgpu_transform::combimgsampsplitter(&spv).unwrap(),
        "dref" => spirv_webgpu_transform::drefsplitter(&spv).unwrap(),
        mode => {
            eprintln!("unknown mode {:?}", mode);
            process::exit(1)
        }
    };
    let out_spv_bytes = spirv_webgpu_transform::u32_slice_to_u8_vec(&out_spv);

    fs::write(output_path, out_spv_bytes).unwrap();
}
