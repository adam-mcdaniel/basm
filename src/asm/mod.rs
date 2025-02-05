use super::*;
use tracing::*;

mod parse;
pub use parse::parse;

use std::sync::RwLock;
use std::collections::HashMap;

pub const NULL: StaticLocation = StaticLocation::Address(0);

pub const STACK_SIZE: usize = 1000;
pub const HEAP_SIZE: usize = 1000;
pub const CALL_STACK_SIZE: usize = 1000;

pub const CELLS_PER_STACK_ELEMENT: usize = 2;

macro_rules! registers {
    ($($reg_name:ident),* $(,)?) => {
        iota::iota! {
            $(
                pub const $reg_name: StaticLocation = StaticLocation::Address(iota + 1).named(stringify!($reg_name));
            )*

            pub const REGISTER_COUNT: usize = iota;

            pub const REGISTERS: [StaticLocation; REGISTER_COUNT] = [
                $($reg_name),*
            ];

            pub const REGISTER_NAMES: [&'static str; REGISTER_COUNT] = [
                $(stringify!($reg_name)),*
            ];
        }
    };
}

registers!(
    NEXT_BASIC_BLOCK,
    CURRENT_BASIC_BLOCK,
    CURRENT_BASIC_BLOCK_EQ0,
    CURRENT_BASIC_BLOCK_EQ1,

    SP,
    HP,
    IDX_TEMP,
    VAL_TEMP,
    CALL_SP,
    PUSH_TEMP,


    T0,
    T1,
    T2,
    T3,
    T4,
    T5,

    R0,
    R1,
    R2,
    R3,
    R4,
    R5,

    DYN_OP_TEMP0,
    DYN_OP_TEMP1,
    DYN_OP_TEMP2,

    PUT_INT0,
    PUT_INT1,
    PUT_INT2,
    PUT_INT3,
    PUT_INT4,
    PUT_INT5,
    PUT_INT6,
    PUT_INT7,
    PUT_INT8,
    PUT_INT10,
    
    JMP_TEMP,
    
    SET_TEMP,
    DYN_SET_TEMP,
    EQUALS_TEMP0,
    EQUALS_TEMP1,
    NOT_EQUALS_TEMP0,
    NOT_EQUALS_TEMP1,

    MATH_TEMP0,
    MATH_TEMP1,
    MATH_TEMP2,
    MATH_TEMP3,

    IF_TEMP0,

    R6,
    R7,
    R8,
    R9,
    R10,
    R11,
    R12,
    R13,
    R14,
    R15,

    ZERO,
    TRASH,
);

fn debug_helper(regs: &[StaticLocation]) -> String {
    let mut result = String::new();

    for reg in regs {
        result.push_str(&TRASH.putmsg(&format!("{reg}=")));
        result.push_str(&reg.putint());
        result.push_str(&TRASH.putmsg(&format!("\n")));
    }

    result
}

pub fn allocate_registers_and_stack() -> Table {
    // Allocate the registers
    info!("Allocating registers...");
    let register_addresses = global_alloc(REGISTER_COUNT + 100);
    info!("Allocated registers at {register_addresses}");
    // Allocate the stack
    info!("Allocating stack...");
    Table::allocate(STACK_SIZE)
}

pub fn allocate_heap() -> Table {
    info!("Allocating heap...");
    Table::allocate(HEAP_SIZE)
}

pub fn allocate_call_stack() -> Table {
    info!("Allocating call stack...");
    Table::allocate(CALL_STACK_SIZE)
}

pub fn allocate_string(string: &str) -> (StaticLocation, String) {
    // Allocate the size of the string
    let result_addr = global_alloc(string.len() + 1);
    // Write the string to the global
    let mut result_code = String::new();
    for (i, ch) in string.chars().enumerate() {
        let ch = ch as u8;
        let cell = result_addr.off(i as i64);
        result_code.push_str(&cell.set_const(ch as u64));
    }
    (result_addr, result_code)
}

pub fn register(name: &str) -> StaticLocation {
    // REGISTERS[i]
    let index = REGISTER_NAMES.iter().position(|&r| r == name).expect(format!("Unknown register: {name}").as_str());
    REGISTERS[index]
}

pub fn register_name(i: usize) -> &'static str {
    REGISTER_NAMES[i]
}
lazy_static! {
    pub static ref STACK_HEAP_CALL_STACK: (Table, Table, Table) = {
        let stack = allocate_registers_and_stack();
        let heap = allocate_heap();
        let call_stack = allocate_call_stack();
        (stack, heap, call_stack)
    };

    pub static ref STACK: Table = STACK_HEAP_CALL_STACK.0;
    pub static ref HEAP: Table = STACK_HEAP_CALL_STACK.1;
    pub static ref CALL_STACK: Table = STACK_HEAP_CALL_STACK.2;
}


#[derive(Debug, Clone, Copy)]
pub enum Operand {
    /// A static location is one of:
    /// - A register "R0" (=StaticLocation::register("R0"))
    /// 
    /// A dynamic location is one of:
    /// - A static location (=DynamicLocation::Static(StaticLocation))
    /// - A stack dereference of a static location "SP[R0]" (=DynamicLocation::DerefStack(StaticLocation))
    /// - A heap dereference of a static location "HP[R0]" (=DynamicLocation::DerefHeap(StaticLocation))
    Location(DynamicLocation),
    /// A constant value
    Immediate(u64),
}

#[derive(Debug, Clone)]
pub enum BasicBlockOp {
    /// `push R0` gets the value of R0 and pushes it onto the stack
    /// `push 1` pushes the immediate 1 onto the stack
    Push(Operand),
    /// `pop R0` pops the top of the stack into R0
    /// `pop` pops the top of the stack
    /// 
    /// An immediate may not be used here
    Pop(Option<DynamicLocation>),

    /// Get a character from the input and store it in the operand,
    /// if supplied. Otherwise, the character is discarded.
    /// 
    /// `getchar R0` gets a character from the user and stores it in R0.
    /// `getchar SP[R0]` gets a character from the user and stores it in the stack location pointed to by R0.
    GetChar(Option<DynamicLocation>),

    /// Get a character operand and print it.
    PutChar(Operand),
    /// Get an integer operand and print it.
    PutInt(Operand),

    /// Set a dynamic location to a value.
    Set {
        src: Operand,
        dest: DynamicLocation,
    },

    /// Get the effective address of a dynamic location and store it in the destination.
    GetAddr {
        src: DynamicLocation,
        dest: DynamicLocation,
        offset: Option<Operand>,
        negative: bool,
    },

    /// Add two operands and store the result in the destination.
    Add {
        lhs: Operand,
        rhs: Operand,
        dest: DynamicLocation,
    },

    /// Subtract two operands and store the result in the destination.
    Sub {
        lhs: Operand,
        rhs: Operand,
        dest: DynamicLocation,
    },

    /// Multiply two operands and store the result in the destination.
    Mul {
        lhs: Operand,
        rhs: Operand,
        dest: DynamicLocation,
    },

    /// Divide two operands and store the result in the destination.
    Div {
        lhs: Operand,
        rhs: Operand,
        dest: DynamicLocation,
    },
    /// Modulo two operands and store the result in the destination.
    Mod {
        lhs: Operand,
        rhs: Operand,
        dest: DynamicLocation,
    },

    /// Negate an operand and store the result in the destination.
    Neg {
        src: Operand,
        dest: DynamicLocation,
    },

    /// Check if two operands are equal and store the result in the destination.
    Eq {
        lhs: Operand,
        rhs: Operand,
        dest: DynamicLocation,
    },

    /// Check if two operands are not equal and store the result in the destination.
    Ne {
        lhs: Operand,
        rhs: Operand,
        dest: DynamicLocation,
    },

    Inc(DynamicLocation, Option<u64>),
    Dec(DynamicLocation, Option<u64>),

    /// Print a hexadecimal dump
    HexDump,
    /// Print a decimal dump
    DecimalDump,
}

fn push_to_call_stack(loc: StaticLocation) -> String {
    CALL_SP.inc()
    + &CALL_STACK.set(CALL_SP, loc)
}

fn pop_from_call_stack(loc: StaticLocation) -> String {
    CALL_STACK.get(CALL_SP, loc)
    + &CALL_SP.dec()
}

fn push(op: Operand) -> String {
    use Operand::*;
    match op {
        Immediate(n) => {
            SP.inc()
            + &SP.stack_deref().set_const(n as u64)
        }
        Location(loc) => {
            DynamicLocation::from(PUSH_TEMP).set_from(loc)
            + &SP.inc()
            + &SP.stack_deref().set_from(PUSH_TEMP)
        }
    }
}

fn pop(op: Option<DynamicLocation>) -> String {
    match op {
        None => {
            SP.dec()
        }
        Some(loc) => {
            loc.set_from(SP.stack_deref())
            + &SP.dec()
        }
    }
}

impl BasicBlockOp {
    pub fn assemble(&self) -> String {
        use BasicBlockOp::*;
        use Operand::*;
        match self {
            Push(op) => {
                push(*op)
            }

            Pop(op) => {
                pop(*op)
            }

            HexDump => {
                "#".to_string()
            }
            DecimalDump => {
                "$".to_string()
            }

            Inc(loc, amount) => {
                match amount {
                    None => {
                        loc.inc()
                    }
                    Some(amount) => {
                        loc.add_const(*amount as i64)
                    }
                }
            }

            Dec(loc, amount) => {
                match amount {
                    None => {
                        loc.dec()
                    }
                    Some(amount) => {
                        loc.sub_const(*amount as i64)
                    }
                }
            }

            GetAddr { src, dest, offset, negative } => {
                match (src, offset) {
                    (DynamicLocation::Static(loc), None) => {
                        dest.set_const(loc.address() as u64)
                    }
                    (DynamicLocation::DerefStack(loc), None) => {
                        dest.set_from(*loc)
                        // + &dest.add_const(*offset)
                    }
                    (DynamicLocation::DerefHeap(loc), None) => {
                        dest.set_from(*loc)
                        // + &dest.add_const(*offset)
                    }

                    (DynamicLocation::Static(loc), Some(Immediate(n))) => {
                        dest.set_const((loc.address() as i64 + if *negative {-(*n as i64)} else {*n as i64}) as u64)
                    }
                    (DynamicLocation::Static(loc), Some(Location(offset))) => {
                        dest.set_const(loc.address() as u64)
                        + &DynamicLocation::plus(*dest, *dest, *offset)
                    }

                    (DynamicLocation::DerefStack(loc), Some(Immediate(n))) => {
                        dest.set_from(*loc)
                        + &if *negative {
                            dest.sub_const(*n as i64)
                        } else {
                            dest.add_const(*n as i64)
                        }
                    }
                    (DynamicLocation::DerefStack(loc), Some(Location(offset))) => {
                        dest.set_from(*loc)
                        + &if *negative {
                            DynamicLocation::minus(*dest, *dest, *offset)
                        } else {
                            DynamicLocation::plus(*dest, *dest, *offset)
                        }
                    }

                    (DynamicLocation::DerefHeap(loc), Some(Immediate(n))) => {
                        dest.set_from(*loc)
                        + &if *negative {
                            dest.sub_const(*n as i64)
                        } else {
                            dest.add_const(*n as i64)
                        }
                    }
                    (DynamicLocation::DerefHeap(loc), Some(Location(offset))) => {
                        dest.set_from(*loc)
                        // + &DynamicLocation::plus(*dest, *dest, *offset)
                        + &if *negative {
                            DynamicLocation::minus(*dest, *dest, *offset)
                        } else {
                            DynamicLocation::plus(*dest, *dest, *offset)
                        }
                    }
                }
            }

            Set { src, dest} => {
                match src {
                    Immediate(n) => {
                        dest.set_const(*n as u64)
                    }
                    Location(loc) => {
                        dest.set_from(*loc)
                    }
                }
            }

            GetChar(None) => {
                TRASH.getchar()
            }
            GetChar(Some(loc)) => {
                loc.getchar()
            }

            PutChar(Immediate(n)) => {
                TRASH.set_const(*n as u64)
                + &TRASH.putchar()
            }
            PutChar(Location(loc)) => {
                loc.putchar()
            }

            PutInt(Immediate(n)) => {
                TRASH.set_const(*n as u64)
                + &TRASH.putint()
            }
            PutInt(Location(loc)) => {
                loc.putint()
            }

            Add { lhs, rhs, dest } => {
                let lhs = match lhs {
                    Immediate(n) => {
                        DynamicLocation::from(T0).set_const(*n as u64)
                    }
                    Location(loc) => {
                        DynamicLocation::from(T0).set_from(*loc)
                    }
                };
                let rhs = match rhs {
                    Immediate(n) => {
                        DynamicLocation::from(T1).set_const(*n as u64)
                    }
                    Location(loc) => {
                        DynamicLocation::from(T1).set_from(*loc)
                    }
                };
                lhs
                + &rhs
                + &DynamicLocation::static_binop(StaticLocation::plus, *dest, T0.into(), T1.into())
            }

            Sub { lhs, rhs, dest } => {
                let lhs = match lhs {
                    Immediate(n) => {
                        DynamicLocation::from(T0).set_const(*n as u64)
                    }
                    Location(loc) => {
                        DynamicLocation::from(T0).set_from(*loc)
                    }
                };
                let rhs = match rhs {
                    Immediate(n) => {
                        DynamicLocation::from(T1).set_const(*n as u64)
                    }
                    Location(loc) => {
                        DynamicLocation::from(T1).set_from(*loc)
                    }
                };
                lhs
                + &rhs
                + &DynamicLocation::static_binop(StaticLocation::minus, *dest, T0.into(), T1.into())
            }

            Mul { lhs, rhs, dest } => {
                let lhs = match lhs {
                    Immediate(n) => {
                        DynamicLocation::from(T0).set_const(*n as u64)
                    }
                    Location(loc) => {
                        DynamicLocation::from(T0).set_from(*loc)
                    }
                };
                let rhs = match rhs {
                    Immediate(n) => {
                        DynamicLocation::from(T1).set_const(*n as u64)
                    }
                    Location(loc) => {
                        DynamicLocation::from(T1).set_from(*loc)
                    }
                };
                lhs
                + &rhs
                + &DynamicLocation::static_binop(StaticLocation::times, *dest, T0.into(), T1.into())
            }

            Div { lhs, rhs, dest } => {
                let lhs = match lhs {
                    Immediate(n) => {
                        DynamicLocation::from(T0).set_const(*n as u64)
                    }
                    Location(loc) => {
                        DynamicLocation::from(T0).set_from(*loc)
                    }
                };
                let rhs = match rhs {
                    Immediate(n) => {
                        DynamicLocation::from(T1).set_const(*n as u64)
                    }
                    Location(loc) => {
                        DynamicLocation::from(T1).set_from(*loc)
                    }
                };
                lhs
                + &rhs
                + &DynamicLocation::static_binop(StaticLocation::divide, *dest, T0.into(), T1.into())
            }


            Mod { lhs, rhs, dest } => {
                todo!()
            }

            Neg { src, dest } => {
                let src = match src {
                    Immediate(n) => {
                        DynamicLocation::from(T0).set_const(*n as u64)
                    }
                    Location(loc) => {
                        DynamicLocation::from(T0).set_from(*loc)
                    }
                };
                src
                + &DynamicLocation::static_unop(StaticLocation::negate, *dest, T0.into())
            }

            Eq { lhs, rhs, dest } => {
                let lhs = match lhs {
                    Immediate(n) => {
                        DynamicLocation::from(T0).set_const(*n as u64)
                    }
                    Location(loc) => {
                        DynamicLocation::from(T0).set_from(*loc)
                    }
                };
                let rhs = match rhs {
                    Immediate(n) => {
                        DynamicLocation::from(T1).set_const(*n as u64)
                    }
                    Location(loc) => {
                        DynamicLocation::from(T1).set_from(*loc)
                    }
                };
                lhs
                + &rhs
                // + &debug_helper(&[T0, T1, EQUALS_TEMP0, EQUALS_TEMP1, DYN_OP_TEMP2, DYN_OP_TEMP0, DYN_OP_TEMP1])
                + &DynamicLocation::equals(*dest, T0.into(), T1.into())
                // + &debug_helper(&[T0, T1, EQUALS_TEMP0, EQUALS_TEMP1, DYN_OP_TEMP2, DYN_OP_TEMP0, DYN_OP_TEMP1])
            }

            Ne { lhs, rhs, dest } => {
                let lhs = match lhs {
                    Immediate(n) => {
                        DynamicLocation::from(T0).set_const(*n as u64)
                    }
                    Location(loc) => {
                        DynamicLocation::from(T0).set_from(*loc)
                    }
                };
                let rhs = match rhs {
                    Immediate(n) => {
                        DynamicLocation::from(T1).set_const(*n as u64)
                    }
                    Location(loc) => {
                        DynamicLocation::from(T1).set_from(*loc)
                    }
                };
                lhs
                + &rhs
                + &DynamicLocation::static_binop(StaticLocation::not_equals, *dest, T0.into(), T1.into())
            }
        }
    }
}

lazy_static! {
    pub static ref BASIC_BLOCK: RwLock<usize> = RwLock::new(0);
    pub static ref BASIC_BLOCK_NAMES: RwLock<HashMap<Symbol, BasicBlock>> = RwLock::new(HashMap::new());
    pub static ref BASIC_BLOCK_IDS: RwLock<HashMap<usize, BasicBlock>> = RwLock::new(HashMap::new());
}

fn next_basic_block_number() -> usize {
    let mut bb = BASIC_BLOCK.write().unwrap();
    *bb += 1;
    *bb
}

fn add_basic_block(bb: BasicBlock) {
    let mut bb_names = BASIC_BLOCK_NAMES.write().unwrap();
    let mut bb_ids = BASIC_BLOCK_IDS.write().unwrap();
    if let Some(ref label) = bb.label {
        bb_names.insert(label.clone(), bb.clone());
    }
    bb_ids.insert(bb.number, bb);
}

#[derive(Debug, Clone)]
pub struct Program(pub Vec<Op>);

impl Program {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn parse(source: &str) -> Result<Self, String> {
        parse(source)
    }

    pub fn push(&mut self, op: Op) {
        self.0.push(op);
    }

    fn assemble_ops(&self) -> String {
        self.0.iter().map(|op| op.assemble()).collect()
    }

    pub fn assemble(&self) -> String {
        // Add a while loop while the basic block is not 0
        CURRENT_BASIC_BLOCK.set_const(1)
        + &NEXT_BASIC_BLOCK.set_const(1)
        + &while_on(&NEXT_BASIC_BLOCK,
            NEXT_BASIC_BLOCK.inc()
            // // Debugging
            // + &TRASH.putmsg("Current basic block is: ")
            // + &CURRENT_BASIC_BLOCK.putint()
            // + &TRASH.putmsg("\n")

            + &self.assemble_ops()
            + &CURRENT_BASIC_BLOCK.set_from(NEXT_BASIC_BLOCK)
            // // Debugging
            // + &TRASH.putmsg("Next basic block is: ")
            // + &NEXT_BASIC_BLOCK.putint()
            // + &TRASH.putmsg("\n")
        )
    }
}

#[derive(Debug, Clone)]
pub enum Op {
    BasicBlock(BasicBlock),
    Label(Symbol, BasicBlock),
    Quit(usize),
    Jmp(usize, Symbol),
    Call(usize, Symbol),
    Return(usize),
    JmpIf(usize, DynamicLocation, Symbol),
}

impl Op {
    pub fn goto_next_basic_block(&self) -> String {
        match self {
            Op::BasicBlock(bb)
            | Op::Label(_, bb) => {
                // Write it to the current basic block register
                // let followed_immediately_by = bb.next_basic_block();
                // NEXT_BASIC_BLOCK.set_const(followed_immediately_by as u64)
                "".to_string()
            }
            Op::Quit(_) => {
                NEXT_BASIC_BLOCK.set_const(0)
            }
            Op::Jmp(_current, label) => {
                let bb_names = BASIC_BLOCK_NAMES.read().unwrap();
                let next = bb_names.get(label).map(|bb| bb.number).expect(&format!("Unknown basic block {label}"));
                NEXT_BASIC_BLOCK.set_const(next as u64)
            }

            Op::Call(_current, label) => {
                // This will push the next basic block onto the stack,
                // and set the "next" basic block to the label
                // todo!()
                
                let bb_names = BASIC_BLOCK_NAMES.read().unwrap();
                let next = bb_names.get(label).map(|bb| bb.number).expect(&format!("Unknown basic block {label}"));

                push_to_call_stack(NEXT_BASIC_BLOCK)
                + &NEXT_BASIC_BLOCK.set_const(next as u64)
                // // Debugging
                // + &TRASH.putmsg(&format!("Calling basic block {label}\n"))
                // + &debug_helper(&[CALL_SP])
            }

            Op::Return(current) => {
                // This will pop the next basic block from the stack,
                // and set the "next" basic block to the label
                pop_from_call_stack(NEXT_BASIC_BLOCK)
                // // Debugging
                // + &TRASH.putmsg(&format!("Retuning from {current}\n"))
                // + &debug_helper(&[CALL_SP])
            }
            Op::JmpIf(current, location, label) => {
                let followed_immediately_by = current + 1;
                let bb_names = BASIC_BLOCK_NAMES.read().unwrap();
                let next = bb_names.get(label).map(|bb| bb.number).expect(&format!("Unknown basic block {label}"));

                // let bb_names = BASIC_BLOCK_NAMES.read().unwrap();
                // bb_names.get(name).map(|bb| bb.number)
                DynamicLocation::from(JMP_TEMP).set_from(*location)
                + &if_stmt(&JMP_TEMP,
                    NEXT_BASIC_BLOCK.set_const(next as u64)
                )
                + &JMP_TEMP.dec()
                + &if_stmt(&JMP_TEMP,
                    NEXT_BASIC_BLOCK.set_const(followed_immediately_by as u64)
                )
            }
        }
    }

    pub fn assemble(&self) -> String {
        let number = match self {
            Op::BasicBlock(bb)
            | Op::Label(_, bb) => {
                bb.number
            }
            Op::Quit(number)
            | Op::Jmp(number, _)
            | Op::Call(number, _)
            | Op::Return(number)
            | Op::JmpIf(number, _, _) => {
                *number
            }
        };
        
        CURRENT_BASIC_BLOCK_EQ1.set_const(number as u64)
        + &StaticLocation::equals(CURRENT_BASIC_BLOCK_EQ0, CURRENT_BASIC_BLOCK, CURRENT_BASIC_BLOCK_EQ1)
        + &if_stmt(&CURRENT_BASIC_BLOCK_EQ0, 
            // TRASH.putmsg(&format!("Executing basic block: {self:?}\n\n"))
            // + &
            match self {
                Op::BasicBlock(bb) => {
                    bb.assemble()
                }
                Op::Label(_, bb) => {
                    bb.assemble()
                }
                Op::Quit(..)
                | Op::Call(..)
                | Op::Return(..)
                | Op::Jmp(..)
                | Op::JmpIf(..) => {
                    // TRASH.putmsg("Executing jump\n") + &
                    self.goto_next_basic_block()
                }
            }
            // + &TRASH.putmsg(&format!("Done executing block {self:?}\n"))
            // + &debug_helper(&[CURRENT_BASIC_BLOCK, NEXT_BASIC_BLOCK, CURRENT_BASIC_BLOCK_EQ0])
        )
        // + &CURRENT_BASIC_BLOCK_EQ0.dec()
        // + &if_stmt(&CURRENT_BASIC_BLOCK_EQ0, 
        //     TRASH.putmsg(&format!("Not executing basic block: {self:?}\n\n")))
        // + &TRASH.putmsg(&format!("Done executing block {self:?}\n"))
        // + &debug_helper(&[CURRENT_BASIC_BLOCK, NEXT_BASIC_BLOCK, CURRENT_BASIC_BLOCK_EQ0])

    }
}


#[derive(Debug, Clone)]
pub struct BasicBlock {
    label: Option<Symbol>,
    number: usize,
    ops: Vec<BasicBlockOp>,
}

impl BasicBlock {
    pub fn new(label: Option<impl Into<Symbol>>, ops: Vec<BasicBlockOp>) -> Self {
        let result = Self {
            label: label.map(Into::into),
            number: next_basic_block_number(),
            ops,
        };
        add_basic_block(result.clone());
        result
    }

    pub fn next_basic_block(&self) -> usize {
        self.number + 1
    }

    pub fn assemble_ops(&self) -> String {
        self.ops.iter().map(|op| op.assemble()).collect()
    }

    pub fn assemble(&self) -> String {
        // Add a check to make sure we're executing the correct basic block
        self.assemble_ops()
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use tracing::info;


    #[test]
    fn test_cat_program() {
        init_logging();

        let source = r#"
        main:
            cat:
                getchar R0
                R1 eq R0, 0
                jmp_if R1, end
                putchar R0
                jmp cat
        end:
            putchar 'B'
            putchar 'y'
            putchar 'e'
            putchar '!'
            putchar '\n'
            quit
        "#;
        // println!("{:#?}", parse(source));
        let program = match parse(source) {
            Ok(program) => program,
            Err(e) => panic!("Error: {}", e),
        };

        assert_eq!(compile_and_run_with_input(program.assemble(), "Hello!\n", 1).unwrap(), "Hello!\nBye!\n");

    }

    #[test]
    fn test_16_bit_program() {
        init_logging();

        let source = r#"
        main:
            putchar 'H'
            putchar 'i'
            putchar '!'
            putchar '\n'
            call calc_max_int
            putint R0
            putchar '\n'
            quit

        calc_max_int:
            R0 = 0
            dec R0
            ret
        "#;
        let program = match parse(source) {
            Ok(program) => program,
            Err(e) => panic!("Error: {}", e),
        };

        compile_and_run(program.assemble(), 1).unwrap();
    }

    #[test]
    fn test_math_program() {
        init_logging();

        let source = r#"
        main:
            R0 = 5
            R1 = 10
            R2 add R0, R1
            putint R2
            putchar '\n'
        "#;
        // println!("{:#?}", parse(source));
        let program = match parse(source) {
            Ok(program) => program,
            Err(e) => panic!("Error: {}", e),
        };

        assert_eq!(compile_and_run_with_input(program.assemble(), "", 1).unwrap(), "15\n");
    }

    #[test]
    fn test_inc_dec_program() {
        init_logging();

        let source = r#"
        main:
            [SP] = 5
            putint [SP]
            putchar '\n'
            dec [SP]
            putint [SP]
            putchar '\n'
        "#;
        // println!("{:#?}", parse(source));
        let program = match parse(source) {
            Ok(program) => program,
            Err(e) => panic!("Error: {}", e),
        };

        assert_eq!(compile_and_run_with_input(program.assemble(), "", 1).unwrap(), "5\n4\n");
    }


    #[test]
    fn test_call_ret_program() {
        init_logging();

        let source = r#"
        main:
            putchar 'F'
            putchar 'a'
            putchar 'c'
            putchar 't'
            putchar ' '
            putchar 'o'
            putchar 'f'
            putchar ' '
            R0 = 5
            putint R0
            putchar ':'
            putchar ' '
            push R0
            call fact
            putint [SP]
            putchar '\n'
            quit

        fact:
            R0 eq [SP], 1
            jmp_if R0, end

            push [SP]
            dec [SP]
            
            call fact
            pop R0
            [SP] mul R0
            ret
        end:
            [SP] = 1
            ret
        "#;
        // println!("{:#?}", parse(source));
        let program = match parse(source) {
            Ok(program) => program,
            Err(e) => panic!("Error: {}", e),
        };

        // // Write to a file
        // let mut file = std::fs::File::create("test-assembler.b").unwrap();
        // use std::io::Write;
        // file.write_all(program.assemble().as_bytes()).unwrap();

        let output = compile_and_run_with_input(program.assemble(), "", 1).unwrap();
        info!("Output: {}", output);
        assert_eq!(output, "Fact of 5: 120\n");
    }

    #[test]
    fn test_lea_program() {
        init_logging();

        let source = r#"
        main:
            push '\n'
            push '?'
            push '?'

            hex_dump
            putchar '\n'
            R0 lea [SP] - 1
            [R0] = '!'
            
            pop R1
            putchar R1

            pop R1
            putchar R1

            pop R1
            putchar R1
        "#;
        // println!("{:#?}", parse(source));
        let program = match parse(source) {
            Ok(program) => program,
            Err(e) => panic!("Error: {}", e),
        };

        compile_and_run_with_input(program.assemble(), "?!\n", 1).unwrap();
    }

    // fn compile_and_run(filename: &str) {
    //     let mut cmd = std::process::Command::new("./compile")
    //         .arg(filename)
    //         .arg("output.c")
    //         .spawn()
    //         .expect("Failed to execute command");
    //     let output = cmd.wait_with_output().expect("Failed to read stdout");
    //     assert!(output.status.success(), "Command failed with status: {}", output.status);
    // }

    #[test]
    fn test_register_addresses() {
        println!("{REGISTERS:?}");
        println!("{REGISTER_COUNT:?}");
        assert_eq!(NULL, StaticLocation::Address(0));
        for i in 0..REGISTER_COUNT {
            let name = register_name(i);
            let r = register(name);
            println!("{name} = {r:?}");
            // assert_eq!(r, StaticLocation::Address(i + 1).strip_name());
        }
        assert_eq!(REGISTERS.len(), REGISTER_COUNT);
    }
}