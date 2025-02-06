use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::process::{Command, Stdio};
use std::io::Write;
use basm::{compile_to_exe, init_logging, Program};
use lazy_static::lazy_static;

use tracing::*;

fn compile_example(name: &str, cell_bytes: u8) {
    // Read the assembly file from the `examples/` directory
    let asm_path = format!("examples/{}.basm", name);
    info!("Compiling example: {}", asm_path);
    let asm = std::fs::read_to_string(asm_path).expect("Failed to read assembly file");
    // Compile the assembly code to Brainfuck
    let bf = Program::parse(&asm).expect("Failed to parse assembly code").assemble();
    // Write the Brainfuck code to `main`
    compile_to_exe(bf, cell_bytes).expect("Failed to compile Brainfuck code");
}

lazy_static! {
    static ref RUN_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
}

fn benchmark_example(c: &mut Criterion, example: &str, cell_bytes: u8) {
    init_logging();
    let lock = RUN_LOCK.lock().unwrap(); // Ensure only one instance runs at a time

    // Compile the example Brainfuck program
    let example_name = example;
    compile_example(example_name, cell_bytes);
    info!("Compiled example: {}", example_name);
    c.bench_function(example, |b| {
        b.iter(|| {
            let mut child = Command::new("./main")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()
                .expect("Failed to start program");

            let stdin = child.stdin.as_mut().expect("Failed to open stdin");
            stdin.write_all(b"Hello!\n").expect("Failed to write to stdin");

            let output = child.wait_with_output().expect("Failed to read output");
            let output_str = String::from_utf8_lossy(&output.stdout);

            black_box(output_str); // Prevent compiler optimizations
        });
    });

    drop(lock); // Release the lock
}

fn benchmark_examples(c: &mut Criterion) {
    benchmark_example(c, "detect-cells", 1);
    benchmark_example(c, "fibonacci", 1);
    benchmark_example(c, "factorial", 1);
    benchmark_example(c, "cat", 1);
}


criterion_group!(benches, benchmark_examples);
criterion_main!(benches);