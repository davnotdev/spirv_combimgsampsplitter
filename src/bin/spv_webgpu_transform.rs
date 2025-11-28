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

    let mut out_correction_map = None;

    let out_spv = match mode.as_str() {
        "combimg" => {
            spirv_webgpu_transform::combimgsampsplitter(&spv, &mut out_correction_map).unwrap()
        }
        "dref" => spirv_webgpu_transform::drefsplitter(&spv, &mut out_correction_map).unwrap(),
        mode => {
            eprintln!("unknown mode {:?}", mode);
            process::exit(1)
        }
    };
    let out_spv_bytes = spirv_webgpu_transform::u32_slice_to_u8_vec(&out_spv);

    eprintln!("Writing patched result to {}", output_path);
    fs::write(output_path, out_spv_bytes).unwrap();

    if let Some(correction_map) = out_correction_map {
        eprintln!("Finished, patch summary: \n");

        for set in correction_map.sets {
            println!("Set {}:", set.set);
            for binding in set.bindings {
                println!("\tBinding {} <- {:?}", binding.binding, binding.corrections);
            }
        }
    } else {
        eprintln!("Finished, no patching done.");
    }
}
