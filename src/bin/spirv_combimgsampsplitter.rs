use std::{env, fs, process};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        eprintln!("Usage: combimgsampsplitter <input.spv> <output.spv>");
        process::exit(1);
    }

    let input_path = &args[1];
    let output_path = &args[2];

    let spv_bytes = fs::read(input_path).unwrap();

    let spv = spirv_combimgsampsplitter::u8_slice_to_u32_vec(&spv_bytes);
    let out_spv = spirv_combimgsampsplitter::combimgsampsplitter(&spv).unwrap();
    let out_spv_bytes = spirv_combimgsampsplitter::u32_slice_to_u8_vec(&out_spv);

    fs::write(output_path, out_spv_bytes).unwrap();
}
