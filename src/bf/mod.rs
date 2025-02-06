use std::{
    fs::File,
    io::Write,
    process::{Command, Stdio},
    sync::Mutex,
};

use lazy_static::lazy_static;
use tracing::*;

mod parse;
// Create a compile lock
lazy_static! {
    static ref COMPILE_LOCK: Mutex<()> = Mutex::new(());
}

pub fn simplify_bf(mut bf: String) -> String {
    info!("Emitting brainfuck...");
    // Parse with nom
    let ops = match parse::parse(&bf) {
        Ok(ops) => ops,
        Err(e) => {
            error!("Failed to parse brainfuck: {e}");
            return String::new();
        }
    };
    bf.clear();
    for op in &ops {
        // bf.push_str(&op.to_bf());
        op.write_bf(&mut bf, 1);
    }
    bf
}

pub fn compile_to_ook(mut bf: String) -> String {
    info!("Compiling brainfuck to Ook...");
    // Parse with nom
    let ops = match parse::parse(&bf) {
        Ok(ops) => ops,
        Err(e) => {
            error!("Failed to parse brainfuck: {e}");
            return String::new();
        }
    };
    bf.clear();
    for op in &ops {
        op.write_ook(&mut bf, 1);
    }
    bf
}

pub fn compile_to_c(mut bf: String, bytes: u8) -> String {
    info!("Compiling brainfuck to C...");
    // Parse with nom
    let ops = match parse::parse(&bf) {
        Ok(ops) => ops,
        Err(e) => {
            error!("Failed to parse brainfuck: {e}");
            return String::new();
        }
    };
    bf.clear();

    bf.push_str("#include <stdio.h>\n");
    bf.push_str("#include <stdlib.h>\n");
    bf.push_str("int main() {\n");
    if bytes == 1 {
        bf.push_str("    unsigned char *tape = calloc(30000, sizeof(char));\n");
        bf.push_str("    unsigned char *ptr = tape;\n");
    } else if bytes == 2 {
        bf.push_str("    unsigned short *tape = calloc(30000, sizeof(short));\n");
        bf.push_str("    unsigned short *ptr = tape;\n");
    } else if bytes == 4 {
        bf.push_str("    unsigned int *tape = calloc(30000, sizeof(int));\n");
        bf.push_str("    unsigned int *ptr = tape;\n");
    } else {
        panic!("Unsupported cell size: {bytes}");
    }
    bf.push_str("    char ch;\n");

    for op in &ops {
        bf.push_str("    ");
        // bf.push_str(&op.to_c());
        op.write_c(&mut bf);
        bf.push('\n');
    }
    bf.push_str("    free(tape);\n");
    bf.push_str("    return 0;\n");
    bf.push_str("}\n");

    bf
}

pub fn compile_to_exe(mut bf: String, bytes: u8) -> Result<(), Box<dyn std::error::Error>> {
    info!("Writing brainfuck to file...");
    let mut file = File::create("main.bf")?;
    // file.write_all(bf.as_bytes())?;
    bf = simplify_bf(bf.clone());
    file.write_all(bf.as_bytes())?;

    info!("Compiling brainfuck...");
    let c = compile_to_c(bf, bytes);
    info!("Creating output file...");
    let mut file = File::create("main.c")?;
    file.write_all(c.as_bytes())?;

    info!("Compiling to executable...");
    let child = Command::new("gcc")
        .arg("main.c")
        .arg("-o")
        .arg("main")
        .output()?;
    // Check if the compilation was successful
    if !child.status.success() {
        let error_message = String::from_utf8_lossy(&child.stderr);
        return Err(format!("Compilation failed: {}", error_message).into());
    }

    Ok(())
}

pub fn compile_and_run(bf: String, bytes: u8) -> Result<(), Box<dyn std::error::Error>> {
    let lock = COMPILE_LOCK.lock().unwrap();
    compile_to_exe(bf, bytes)?;
    info!("Running executable...");
    let mut child = Command::new("./main").spawn()?;
    child.wait()?;
    drop(lock);
    Ok(())
}

pub fn compile_and_run_with_input(
    bf: String,
    input: &str,
    bytes: u8,
) -> Result<String, Box<dyn std::error::Error>> {
    let lock = COMPILE_LOCK.lock().unwrap();
    compile_to_exe(bf, bytes)?;
    // std::process::Command::new(&format!("./{filename}")).status()?;
    info!("Running executable with input...");
    let mut child = Command::new(&format!("./main"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    {
        let stdin = child.stdin.as_mut().unwrap();
        stdin.write_all(input.as_bytes())?;

        // Print the output
        let output = child.wait_with_output()?;
        drop(lock);
        Ok(String::from_utf8(output.stdout)?)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    /// Move pointer (>^N) or (<^N)
    Move(i32),
    /// Add (-^N or +^N)
    Add(i32),
    /// Set to zero ([-])
    Zero,
    /// Print to stdout (.)
    Put,
    /// Read from stdin (,)
    Get,
    /// Start of loop ([)
    While,
    /// End of loop (])
    End,
    /// Hexadecimal dump (#)
    HexDump,
    /// Decimal dump ($)
    DecDump,
}

impl Op {
    pub fn coalesce(&mut self, other: Self) -> bool {
        match (self, other) {
            (Self::Move(x), Self::Move(y)) => {
                *x += y;
                true
            }
            (Self::Add(x), Self::Add(y)) => {
                *x += y;
                true
            }
            (Self::Zero, Self::Zero) => true,
            _ => false,
        }
    }

    pub fn write_ook(&self, ook: &mut String, target_cell_bytes: u8) {
        if target_cell_bytes != 1 {
            panic!("Unsupported cell size: {target_cell_bytes}");
        }

        match self {
            Self::Move(n) => {
                if *n > 0 {
                    ook.push_str(&"Ook. Ook? ".repeat(*n as usize));
                } else {
                    ook.push_str(&"Ook? Ook. ".repeat(*n as usize));
                }
            }
            Self::Add(n) => {
                if *n > 0 {
                    ook.push_str(&"Ook. Ook. ".repeat(*n as usize));
                } else {
                    ook.push_str(&"Ook! Ook! ".repeat(-*n as usize));
                }
            }
            Self::Zero => {
                Self::While.write_ook(ook, target_cell_bytes);
                Self::Add(-1).write_ook(ook, target_cell_bytes);
                Self::End.write_ook(ook, target_cell_bytes);
            }
            Self::Put => ook.push_str("Ook! Ook. "),
            Self::Get => ook.push_str("Ook. Ook! "),

            Self::While => ook.push_str("Ook! Ook? "),
            Self::End => ook.push_str("Ook? Ook! "),
            _ => {}
        }
    }

    pub fn write_bf(&self, bf: &mut String, target_cell_bytes: u8) {
        if target_cell_bytes == 1 {
            match self {
                Self::Move(n) => {
                    if *n > 0 {
                        bf.push_str(&">".repeat(*n as usize));
                    } else {
                        bf.push_str(&"<".repeat(-*n as usize));
                    }
                }
                Self::Add(n) => {
                    if *n > 0 {
                        bf.push_str(&"+".repeat(*n as usize));
                    } else {
                        bf.push_str(&"-".repeat(-*n as usize));
                    }
                }
                Self::Zero => bf.push_str("[-]"),
                Self::Put => bf.push('.'),
                Self::Get => bf.push(','),
                Self::While => bf.push('['),
                Self::End => bf.push(']'),
                Self::HexDump => bf.push('#'),
                Self::DecDump => bf.push('$'),
            }
        } else if target_cell_bytes == 2 {
            match self {
                Self::Move(n) => {
                    if *n > 0 {
                        bf.push_str(&">>>".repeat(*n as usize));
                    } else {
                        bf.push_str(&"<<<".repeat(-*n as usize));
                    }
                }
                Self::Add(n) => {
                    if *n > 0 {
                        bf.push_str(&"+[<+>>>+<<-]<[>+<-]+>>>[<<<->>>[-]]<<<[->>+<<]>".repeat(*n as usize));
                    } else {
                        bf.push_str(&"[<+>>>+<<-]<[>+<-]+>>>[<<<->>>[-]]<<<[->>-<<]>-".repeat(-*n as usize));
                    }
                }
                Self::Zero => {
                    Self::While.write_bf(bf, target_cell_bytes);
                    Self::Add(-1).write_bf(bf, target_cell_bytes);
                    Self::End.write_bf(bf, target_cell_bytes);
                },
                Self::Put => bf.push('.'),
                Self::Get => bf.push(','),
                Self::While => bf.push_str("[>>+>>>+<<<<<-]>>>>>[<<<<<+>>>>>-]<<<[[-]<<<+>>>]<[>+>>>+<<<<-]>>>>[<<<<+>>>>-]<<<[[-]<<<+>>>]<<<[[-]>"),
                Self::End => bf.push_str("[>>+>>>+<<<<<-]>>>>>[<<<<<+>>>>>-]<<<[[-]<<<+>>>]<[>+>>>+<<<<-]>>>>[<<<<+>>>>-]<<<[[-]<<<+>>>]<<<]>"),
                Self::HexDump => bf.push('#'),
                Self::DecDump => bf.push('$'),
            }
        } else if target_cell_bytes == 4 {
            match self {
                Self::Move(n) => {
                    if *n > 0 {
                        bf.push_str(&">>>>>".repeat(*n as usize));
                    } else {
                        bf.push_str(&"<<<<<".repeat(-*n as usize));
                    }
                }
                Self::Add(n) => {
                    if *n > 0 {
                        bf.push_str(&"+[<+>>>>>+<<<<-]<[>+<-]+>>>>>[<<<<<->>>>>[-]]<<<<<[->>+[<<+>>>>>+<<<-]<<[>>+<<-]+>>>>>[<<<<<->>>>>[-]]<<<<<[->>>+[<<<+>>>>>+<<-]<<<[>>>+<<<-]+>>>>>[<<<<<->>>>>[-]]<<<<<[->>>>+<<<<]]]>".repeat(*n as usize));
                    } else {
                        bf.push_str(&"[<+>>>>>+<<<<-]<[>+<-]+>>>>>[<<<<<->>>>>[-]]<<<<<[->>[<<+>>>>>+<<<-]<<[>>+<<-]+>>>>>[<<<<<->>>>>[-]]<<<<<[->>>[<<<+>>>>>+<<-]<<<[>>>+<<<-]+>>>>>[<<<<<->>>>>[-]]<<<<<[->>>>-<<<<]>>>-<<<]>>-<<]>-".repeat(-*n as usize));
                    }
                }
                Self::Zero => {
                    Self::While.write_bf(bf, target_cell_bytes);
                    Self::Add(-1).write_bf(bf, target_cell_bytes);
                    Self::End.write_bf(bf, target_cell_bytes);
                },
                Self::Put => bf.push('.'),
                Self::Get => bf.push(','),
                Self::While => bf.push_str("[>>>>+>>>>>+<<<<<<<<<-]>>>>>>>>>[<<<<<<<<<+>>>>>>>>>-]<<<<<[[-]<<<<<+>>>>>]<<<[>>>+>>>>>+<<<<<<<<-]>>>>>>>>[<<<<<<<<+>>>>>>>>-]<<<<<[[-]<<<<<+>>>>>]<<[>>+>>>>>+<<<<<<<-]>>>>>>>[<<<<<<<+>>>>>>>-]<<<<<[[-]<<<<<+>>>>>]<[>+>>>>>+<<<<<<-]>>>>>>[<<<<<<+>>>>>>-]<<<<<[[-]<<<<<+>>>>>]<<<<<[[-]>"),
                Self::End => bf.push_str("[>>>>+>>>>>+<<<<<<<<<-]>>>>>>>>>[<<<<<<<<<+>>>>>>>>>-]<<<<<[[-]<<<<<+>>>>>]<<<[>>>+>>>>>+<<<<<<<<-]>>>>>>>>[<<<<<<<<+>>>>>>>>-]<<<<<[[-]<<<<<+>>>>>]<<[>>+>>>>>+<<<<<<<-]>>>>>>>[<<<<<<<+>>>>>>>-]<<<<<[[-]<<<<<+>>>>>]<[>+>>>>>+<<<<<<-]>>>>>>[<<<<<<+>>>>>>-]<<<<<[[-]<<<<<+>>>>>]<<<<<]>"),
                Self::HexDump => bf.push('#'),
                Self::DecDump => bf.push('$'),
            }
        } else {
            panic!("Unsupported cell size: {target_cell_bytes}");
        }
    }

    //     pub fn to_c(&self) -> String {
    //         match self {
    //             Self::Move(n) => format!("ptr += {n};"),
    //             Self::Add(n) => format!("*ptr += {n};"),
    //             Self::Zero => "*ptr = 0;".to_string(),
    //             Self::Put => "putchar(*ptr);".to_string(),
    //             Self::Get => "*ptr = (ch = getchar()) == EOF? 0 : ch;".to_string(),
    //             Self::While => "while (*ptr) {".to_string(),
    //             Self::End => "}".to_string(),
    //             Self::HexDump => r#"for (int i = 0; i < 0x100; i++) {
    //     if (i % 16 == 0) {
    //         printf("%03d-%03d: ", i, i + 15);
    //     }
    //     printf("%02x ", tape[i]);
    //     if ((i + 1) % 16 == 0) {
    //         printf("\n");
    //     }
    // }"#.to_string(),
    //             Self::DecDump => r#"for (int i = 0; i < 0x100; i++) {
    //     if (i % 16 == 0) {
    //         printf("%03d-%03d: ", i, i + 15);
    //     }
    //     printf("%3d ", tape[i]);
    //     if ((i + 1) % 16 == 0) {
    //         printf("\n");
    //     }
    // }"#.to_string(),
    //         }
    //     }

    pub fn write_c(&self, bf: &mut String) {
        match self {
            Self::Move(n) => bf.push_str(&format!("ptr += {n};")),
            Self::Add(n) => bf.push_str(&format!("*ptr += {n};")),
            Self::Zero => bf.push_str("*ptr = 0;"),
            Self::Put => bf.push_str("putchar(*ptr);"),
            Self::Get => bf.push_str("*ptr = (ch = getchar()) == EOF? 0 : ch;"),
            Self::While => bf.push_str("while (*ptr) {"),
            Self::End => bf.push('}'),
            Self::HexDump => bf.push_str(
                r#"for (int i = 0; i < 0x100; i++) {
    if (i % 16 == 0) {
        printf("%03d-%03d: ", i, i + 15);
    }
    printf("%02x ", tape[i]);
    if ((i + 1) % 16 == 0) {
        printf("\n");
    }
}"#,
            ),
            Self::DecDump => bf.push_str(
                r#"for (int i = 0; i < 0x100; i++) {
    if (i % 16 == 0) {
        printf("%03d-%03d: ", i, i + 15);
    }
    printf("%3d ", tape[i]);
    if ((i + 1) % 16 == 0) {
        printf("\n");
    }
}"#,
            ),
        }
    }
}
