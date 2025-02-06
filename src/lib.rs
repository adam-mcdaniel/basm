//! # basm - Brainfuck Assembly
//! 
//! A library for compiling Brainfuck to assembly and other languages.
//! 
//! ```rust
//! use basm::{Program, simplify_bf};
//! 
//! fn main() {
//!     let program = Program::parse("
//!         log \"Hello world!\"
//!         R0 = 5
//!         R1 = 10
//! 
//!         R2 add R0, R1
//!         log \" + \"
//!         putint R1
//!         log \" = \"
//!         putint R2
//!     ").expect("Failed to parse assembly");
//!     let bf = program.assemble();
//!     let optimized_bf = simplify_bf(bf);
//!     println!("{}", optimized_bf);
//! }
//! ```

#![recursion_limit = "1024"]
use lazy_static::lazy_static;
use std::sync::RwLock;
use tracing::info;

mod asm;
pub use asm::*;

mod symbol;
pub use symbol::*;

mod bf;
pub use bf::{
    compile_and_run, compile_and_run_with_input, compile_to_c, compile_to_exe, simplify_bf, compile_to_ook
};

pub mod util;

pub fn init_logging() {
    #[cfg(debug_assertions)]
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .try_init();
    #[cfg(not(debug_assertions))]
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .without_time()
        .with_target(false)
        .try_init();
}

pub const TAPE_SIZE: usize = 30_000;

lazy_static! {
    // pub static ref NEXT_GLOBAL: RwLock<StaticLocation> = RwLock::new(STACK_END.off(1));
    static ref NEXT_GLOBAL: RwLock<StaticLocation> = RwLock::new(StaticLocation::Address(0));
}

pub fn global_alloc(size: usize) -> StaticLocation {
    let mut next = NEXT_GLOBAL.write().unwrap();
    let result = *next;
    *next = next.off(size as i64);
    result
}

#[derive(Debug, Clone, Copy)]
pub struct Table {
    data_cells: usize,
    start_data: StaticLocation,
    temp0: StaticLocation,
    temp1: StaticLocation,
    temp2: StaticLocation,
}

impl Table {
    pub fn new(data_cells: usize, start_location: StaticLocation) -> Self {
        let base = start_location;
        let start_data = base.off(1);
        let temp0 = base.off(2);
        let temp1 = base.off(3);
        let temp2 = base.off(0);

        Self {
            data_cells,
            start_data,
            temp0,
            temp1,
            temp2,
        }
    }

    pub fn allocate(data_cells: usize) -> Self {
        let base = global_alloc(4 + 2 * data_cells);
        info!("Allocated table {}", base);
        Self::new(data_cells, base)
    }

    pub fn total_size(&self) -> usize {
        self.data_cells * 2 + 4
    }

    pub fn start(&self) -> StaticLocation {
        self.start_data
    }

    pub fn end(&self) -> StaticLocation {
        self.start_data.off(self.data_cells as i64)
    }

    pub fn set(&self, index: StaticLocation, value: StaticLocation) -> String {
        let x = self.start_data;
        let y = index;
        let z = value;

        let temp0 = self.temp0;
        let temp1 = self.temp1;
        let temp2 = self.temp2;

        temp0.zero()
            + &temp1.zero()
            + &temp2.zero()
            + &while_on(&y, temp1.inc() + &temp2.inc() + &y.dec())
            + &while_on(&temp2, y.inc() + &temp2.dec())
            + &while_on(&z, temp0.inc() + &temp2.inc() + &z.dec())
            + &while_on(&temp2, z.inc() + &temp2.dec())
            + &x.to()
            + ">>[[>>]+[<<]>>-]+"
            + "[>>]<[-]<[<<]"
            + ">[>[>>]<+<[<<]>-]"
            + ">[>>]<<[-<<]"
            + &x.from()
    }

    pub fn set_const(&self, index: StaticLocation, value: u64) -> String {
        let z = VAL_TEMP;
        z.set_const(value) + &self.set(index, z)
    }

    pub fn get(&self, index: StaticLocation, dst: StaticLocation) -> String {
        let x = dst;
        let y = self.start_data;
        let z = index;

        let temp0 = self.temp0;
        let temp1 = self.temp1;

        x.zero()
        + &temp0.zero()
        + &temp1.zero()
        // + &z.to() + &while_loop(z.from() + &TEMP1.inc() + &TEMP0.inc() + &z.to() + "-") + &z.from()
        + &while_on(&z, temp1.inc() + &temp0.inc() + &z.dec())
        // + &TEMP0.to() + &while_loop(TEMP0.from() + &z.inc() + &TEMP0.to() + "-") + &TEMP0.from()
        + &while_on(&temp0, z.inc() + &temp0.dec())
        + &y.to()
        + ">>[[>>]+[<<]>>-]+[>>]<[<[<<]>+<"
        + &y.from()
        + &x.inc()
        + &y.to()
        + ">>[>>]<-]<[<<]>[>[>>]<+<[<<]>-]>[>>]<<[-<<]"
        + &y.from()
    }
}

fn while_on(x: &StaticLocation, contents: String) -> String {
    format!(
        "{to}[{from}{contents}{to}]{from}",
        to = x.to(),
        from = x.from()
    )
}

fn if_stmt(x: &StaticLocation, contents: String) -> String {
    // ZERO.zero() + &x.to() + "[" + &x.from() + &contents + &ZERO.to() + "]" + &ZERO.from()
    IF_TEMP0.set_from(*x)
        + &IF_TEMP0.to()
        + "["
        + &IF_TEMP0.from()
        + &contents
        + &IF_TEMP0.to()
        + "[-]]"
        + &IF_TEMP0.from()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StaticLocation {
    /// A named location.
    Named { name: &'static str, addr: usize },
    /// A fixed address on the tape.
    Address(usize),
}

impl StaticLocation {
    pub fn register(name: &str) -> Self {
        register(name)
    }

    pub fn address(&self) -> usize {
        match self {
            StaticLocation::Address(addr) => *addr,
            StaticLocation::Named { addr, .. } => *addr,
        }
    }

    pub const fn off(self, offset: i64) -> Self {
        match self {
            StaticLocation::Address(addr) => StaticLocation::Address(addr + offset as usize),
            StaticLocation::Named { name, addr } => StaticLocation::Named {
                name,
                addr: addr + offset as usize,
            },
        }
    }

    pub const fn named(self, name: &'static str) -> Self {
        match self {
            StaticLocation::Address(addr) => StaticLocation::Named { name, addr },
            StaticLocation::Named { addr, .. } => StaticLocation::Named { name, addr },
        }
    }

    pub fn strip_name(self) -> Self {
        match self {
            StaticLocation::Named { addr, .. } => StaticLocation::Address(addr),
            _ => self,
        }
    }

    /// A fixed address on the tape.
    pub fn addr(addr: usize) -> Self {
        Self::Address(addr)
    }

    pub fn to(&self) -> String {
        match self {
            StaticLocation::Address(addr) => ">".repeat(*addr),
            StaticLocation::Named { name, addr } => format!("(to {name}: {})", ">".repeat(*addr)),
        }
    }

    pub fn zero(&self) -> String {
        self.to() + "[-]" + &self.from()
    }

    pub fn set_from(&self, src: StaticLocation) -> String {
        if self == &src {
            return String::new();
        }

        let y = src;
        let x: StaticLocation = *self;
        let temp0 = SET_TEMP;
        // temp0[-]
        // x[-]
        // y[x+temp0+y-]
        // temp0[y+temp0-]
        temp0.zero()
            + &x.zero()
            + &while_on(&y, x.inc() + &temp0.inc() + &y.dec())
            + &while_on(&temp0, y.inc() + &temp0.dec())
    }

    pub fn load_into(&self, dst: StaticLocation) -> String {
        dst.set_from(*self)
    }

    pub fn negate(dest: StaticLocation, src: StaticLocation) -> String {
        let temp0 = MATH_TEMP0;

        let x = dest;

        x.set_from(src)
            + &temp0.zero()
            + &while_on(&x, temp0.inc() + &x.dec())
            + &while_on(&temp0, x.dec() + &temp0.inc())
    }

    pub fn boolean_not(dest: StaticLocation, src: StaticLocation) -> String {
        // temp0[-]
        // x[temp0+x[-]]+
        // temp0[x-temp0-]
        let x = dest;
        let temp0 = MATH_TEMP0;
        temp0.zero()
        + &x.set_from(src)
        + &while_on(&x, temp0.inc() + &x.zero()) + &x.inc()
        + &while_on(&temp0, x.dec() + &temp0.dec())
    }

    pub fn equals(dest: StaticLocation, lhs: StaticLocation, rhs: StaticLocation) -> String {
        // temp0[-]
        // temp1[-]
        // x[temp1+x-]
        // y[temp1-temp0+y-]
        // temp0[y+temp0-]
        // temp1[x-temp1[-]]

        let x = dest;
        let y = EQUALS_TEMP0;

        // x[-y-x]+y[x-y[-]]
        x.set_from(lhs)
            + &y.set_from(rhs)
            + &while_on(&x, x.dec() + &y.dec())
            + &x.inc()
            + &while_on(&y, x.dec() + &y.zero())

        // temp0.zero()
        // + &temp1.zero()
        // + &x.set_from(lhs)
        // + &while_on(&x, temp1.inc() + &x.dec())
        // + &while_on(&y, temp1.dec() + &temp0.inc() + &y.dec())
        // + &while_on(&temp0, y.inc() + &temp0.dec())
        // + &while_on(&temp1, x.dec() + &temp1.zero())
    }

    pub fn not_equals(dest: StaticLocation, lhs: StaticLocation, rhs: StaticLocation) -> String {
        // temp0[-]
        // temp1[-]
        // x[temp1+x-]
        // y[temp1-temp0+y-]
        // temp0[y+temp0-]
        // temp1[x+temp1[-]]

        let x = dest;
        let y = rhs;

        let temp0 = NOT_EQUALS_TEMP0;
        let temp1 = NOT_EQUALS_TEMP1;

        temp0.zero()
            + &temp1.zero()
            + &x.set_from(lhs)
            + &while_on(&x, temp1.inc() + &x.dec())
            + &while_on(&y, temp1.dec() + &temp0.inc() + &y.dec())
            + &while_on(&temp0, y.inc() + &temp0.dec())
            + &while_on(&temp1, x.inc() + &temp1.zero())
    }

    pub fn plus(dest: StaticLocation, lhs: StaticLocation, rhs: StaticLocation) -> String {
        // temp0[-]
        // y[x+temp0+y-]
        // temp0[y+temp0-]

        let x = MATH_TEMP0;
        let y = rhs;

        let temp0 = MATH_TEMP1;

        temp0.zero()
            + &x.set_from(lhs)
            + &while_on(&y, x.inc() + &temp0.inc() + &y.dec())
            + &while_on(&temp0, y.inc() + &temp0.dec())
            + &dest.set_from(x)
    }

    pub fn minus(dest: StaticLocation, lhs: StaticLocation, rhs: StaticLocation) -> String {
        // temp0[-]
        // y[x-temp0+y-]
        // temp0[y+temp0-]

        let x = MATH_TEMP0;
        let y = rhs;

        let temp0 = MATH_TEMP1;

        temp0.zero()
            + &x.set_from(lhs)
            + &while_on(&y, x.dec() + &temp0.inc() + &y.dec())
            + &while_on(&temp0, y.inc() + &temp0.dec())
            + &dest.set_from(x)
    }

    pub fn times(dest: StaticLocation, lhs: StaticLocation, rhs: StaticLocation) -> String {
        // temp0[-]
        // temp1[-]
        // x[temp1+x-]
        // temp1[
        //  y[x+temp0+y-]temp0[y+temp0-]
        // temp1-]

        let x = dest;
        let y = rhs;

        let temp0 = MATH_TEMP0;
        let temp1 = MATH_TEMP1;

        temp0.zero()
            + &temp1.zero()
            + &x.set_from(lhs)
            + &while_on(&x, temp1.inc() + &x.dec())
            + &while_on(
                &temp1,
                while_on(&y, x.inc() + &temp0.inc() + &y.dec())
                    + &while_on(&temp0, y.inc() + &temp0.dec())
                    + &temp1.dec(),
            )
    }

    pub fn divide(dest: StaticLocation, lhs: StaticLocation, rhs: StaticLocation) -> String {
        // temp0[-]
        // temp1[-]
        // temp2[-]
        // temp3[-]
        // x[temp0+x-]
        // temp0[
        //  y[temp1+temp2+y-]
        //  temp2[y+temp2-]
        //  temp1[
        //   temp2+
        //   temp0-[temp2[-]temp3+temp0-]
        //   temp3[temp0+temp3-]
        //   temp2[temp1-[x-temp1[-]]+temp2-]
        //  temp1-]
        //  x+
        // temp0]

        let x = dest;
        let y = rhs;

        let temp0 = MATH_TEMP0;
        let temp1 = MATH_TEMP1;
        let temp2 = MATH_TEMP2;
        let temp3 = MATH_TEMP3;

        temp0.zero()
            + &temp1.zero()
            + &temp2.zero()
            + &temp3.zero()
            + &x.set_from(lhs)
            + &while_on(&x, temp0.inc() + &x.dec())
            + &while_on(
                &temp0,
                while_on(&y, temp1.inc() + &temp2.inc() + &y.dec())
                    + &while_on(&temp2, y.inc() + &temp2.dec())
                    + &while_on(
                        &temp1,
                        temp2.inc()
                            + &temp0.dec()
                            + &while_on(&temp0, temp2.zero() + &temp3.inc() + &temp0.dec())
                            + &while_on(&temp3, temp0.inc() + &temp3.dec())
                            + &while_on(
                                &temp2,
                                temp1.dec()
                                    + &while_on(&temp1, x.dec() + &temp1.zero())
                                    + &temp1.inc()
                                    + &temp2.dec(),
                            )
                            + &temp1.dec(),
                    )
                    + &x.inc(),
            )
    }

    pub fn putchar(&self) -> String {
        self.to() + "." + &self.from()
    }
    pub fn putmsg(&self, msg: &str) -> String {
        let mut result = String::new();
        for ch in msg.bytes() {
            result += &(self.set_const(ch as u64) + &self.putchar())
        }
        result
    }

    pub fn putint(&self) -> String {
        // "".to_string()
        PUT_INT0.set_from(*self)
        + &PUT_INT1.zero()
        + &PUT_INT2.zero()
        + &PUT_INT3.zero()
        + &PUT_INT4.set_const(1)
        + &PUT_INT5.zero()
        + &PUT_INT6.zero()
        + &PUT_INT7.zero()
        + &PUT_INT0.to()
        + ">[-]>[-]+>[-]+<[>[-<-<<[->+>+<<]>[-<+>]>>]++++++++++>[-]+>[-]>[-]>[-]<<<<<[->-[>+>>]>[[-<+>]+>+>>]<<<<<]>>-[-<<+>>]<[-]++++++++[-<++++++>]>>[-<<+>>]<<]<[.[-]<]<"
        + &PUT_INT0.from()
    }

    pub fn getchar(&self) -> String {
        self.to() + "," + &self.from()
    }

    pub fn inc(&self) -> String {
        self.add_const(1)
    }

    pub fn dec(&self) -> String {
        self.sub_const(1)
    }

    pub fn set_const(&self, literal: u64) -> String {
        self.to() + &"[-]" + &"+".repeat(literal as usize) + &self.from()
    }

    pub fn add_const(&self, literal: i64) -> String {
        if literal < 0 {
            return self.sub_const(-literal);
        }
        self.to() + &"+".repeat(literal as usize) + &self.from()
    }

    pub fn sub_const(&self, literal: i64) -> String {
        self.to() + &"-".repeat(literal as usize) + &self.from()
    }

    pub fn from(&self) -> String {
        match self {
            StaticLocation::Address(addr) => "<".repeat(*addr),
            StaticLocation::Named { name, addr } => format!("(from {name}: {})", "<".repeat(*addr)),
        }
    }

    pub fn stack_deref(self) -> DynamicLocation {
        DynamicLocation::from(self).stack_deref()
    }

    pub fn heap_deref(self) -> DynamicLocation {
        DynamicLocation::from(self).heap_deref()
    }
}

/// A StaticLocation on a brainfuck tape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DynamicLocation {
    DerefHeap(StaticLocation),
    DerefStack(StaticLocation),
    Static(StaticLocation),
}

impl DynamicLocation {
    pub const HOME: Self = Self::Static(StaticLocation::Address(0));

    /// Create a new StaticLocation at the given offset from the home StaticLocation.
    pub fn off(self, offset: i64) -> Self {
        match self {
            Self::Static(loc) => Self::Static(loc.off(offset)),
            _ => {
                panic!("Cannot offset a dereferenced location")
            }
        }
    }

    /// A fixed address on the tape.
    pub fn addr(addr: usize) -> Self {
        Self::Static(StaticLocation::Address(addr))
    }

    pub fn set_from(&self, src: impl Into<DynamicLocation>) -> String {
        let temp0 = DYN_SET_TEMP;

        let src = src.into();
        if self == &src {
            return String::new();
        }
        let dst = self.clone();
        use DynamicLocation::*;
        match (dst, src) {
            (Static(dst), Static(src)) => dst.set_from(src),

            (Static(dst), DerefStack(src)) => STACK.get(src, dst),
            (Static(dst), DerefHeap(src)) => HEAP.get(src, dst),
            (DerefStack(dst), Static(src)) => STACK.set(dst, src),
            (DerefHeap(dst), Static(src)) => HEAP.set(dst, src),

            (DerefStack(dst), DerefStack(src)) => {
                // Get the value of `src` into `temp0`
                STACK.get(src, temp0) + &STACK.set(dst, temp0)
            }
            (DerefHeap(dst), DerefHeap(src)) => {
                // Get the value of `src` into `temp0`
                HEAP.get(src, temp0) + &HEAP.set(dst, temp0)
            }

            (DerefStack(dst), DerefHeap(src)) => {
                // Get the value of `src` into `temp0`
                HEAP.get(src, temp0) + &STACK.set(dst, temp0)
            }
            (DerefHeap(dst), DerefStack(src)) => {
                // Get the value of `src` into `temp0`
                STACK.get(src, temp0) + &HEAP.set(dst, temp0)
            }
        }
    }

    /// Get the value at this location
    pub fn get_from(&self, dst: impl Into<DynamicLocation>) -> String {
        dst.into().set_from(self.clone())
    }

    pub fn set_const(&self, value: u64) -> String {
        match self {
            Self::Static(loc) => loc.set_const(value),
            Self::DerefStack(loc) => VAL_TEMP.set_const(value) + &STACK.set(*loc, VAL_TEMP),
            Self::DerefHeap(loc) => VAL_TEMP.set_const(value) + &HEAP.set(*loc, VAL_TEMP),
        }
    }

    pub fn inc(&self) -> String {
        self.add_const(1)
    }

    pub fn dec(&self) -> String {
        self.sub_const(1)
    }

    pub fn add_const(&self, value: i64) -> String {
        if value < 0 {
            return self.sub_const(-value);
        }
        match self {
            Self::Static(loc) => loc.add_const(value),
            Self::DerefStack(loc) => {
                STACK.get(*loc, VAL_TEMP) + &VAL_TEMP.add_const(value) + &STACK.set(*loc, VAL_TEMP)
            }
            Self::DerefHeap(loc) => {
                HEAP.get(*loc, VAL_TEMP) + &VAL_TEMP.add_const(value) + &HEAP.set(*loc, VAL_TEMP)
            }
        }
    }

    pub fn sub_const(&self, value: i64) -> String {
        if value < 0 {
            return self.add_const(-value);
        }
        match self {
            Self::Static(loc) => loc.sub_const(value),
            Self::DerefStack(loc) => {
                STACK.get(*loc, VAL_TEMP) + &VAL_TEMP.sub_const(value) + &STACK.set(*loc, VAL_TEMP)
            }
            Self::DerefHeap(loc) => {
                HEAP.get(*loc, VAL_TEMP) + &VAL_TEMP.sub_const(value) + &HEAP.set(*loc, VAL_TEMP)
            }
        }
    }

    pub fn static_binop(
        binop: impl Fn(StaticLocation, StaticLocation, StaticLocation) -> String,
        dest: DynamicLocation,
        lhs: DynamicLocation,
        rhs: DynamicLocation,
    ) -> String {
        Self::from(DYN_OP_TEMP0).set_from(lhs)
            + &Self::from(DYN_OP_TEMP1).set_from(rhs)
            + &binop(DYN_OP_TEMP2, DYN_OP_TEMP0, DYN_OP_TEMP1)
            + &dest.set_from(DYN_OP_TEMP2)
    }

    pub fn static_unop(
        unop: impl Fn(StaticLocation, StaticLocation) -> String,
        dest: DynamicLocation,
        src: DynamicLocation,
    ) -> String {
        Self::from(DYN_OP_TEMP0).set_from(src)
            + &unop(DYN_OP_TEMP1, DYN_OP_TEMP0)
            + &dest.set_from(DYN_OP_TEMP1)
    }

    pub fn negate(dest: DynamicLocation, src: DynamicLocation) -> String {
        Self::static_unop(StaticLocation::negate, dest, src)
    }

    pub fn boolean_not(dest: DynamicLocation, src: DynamicLocation) -> String {
        Self::static_unop(StaticLocation::boolean_not, dest, src)
    }

    pub fn getchar(&self) -> String {
        match self {
            Self::Static(loc) => loc.getchar(),
            Self::DerefStack(loc) => VAL_TEMP.getchar() + &STACK.set(*loc, VAL_TEMP),
            Self::DerefHeap(loc) => VAL_TEMP.getchar() + &HEAP.set(*loc, VAL_TEMP),
        }
    }

    pub fn putint(&self) -> String {
        match self {
            Self::Static(loc) => loc.putint(),
            Self::DerefStack(loc) => STACK.get(*loc, VAL_TEMP) + &VAL_TEMP.putint(),
            Self::DerefHeap(loc) => HEAP.get(*loc, VAL_TEMP) + &VAL_TEMP.putint(),
        }
    }

    pub fn putchar(&self) -> String {
        match self {
            Self::Static(loc) => loc.putchar(),
            Self::DerefStack(loc) => STACK.get(*loc, VAL_TEMP) + &VAL_TEMP.putchar(),
            Self::DerefHeap(loc) => HEAP.get(*loc, VAL_TEMP) + &VAL_TEMP.putchar(),
        }
    }

    pub fn plus(dest: DynamicLocation, lhs: DynamicLocation, rhs: DynamicLocation) -> String {
        Self::static_binop(StaticLocation::plus, dest, lhs, rhs)
    }
    pub fn minus(dest: DynamicLocation, lhs: DynamicLocation, rhs: DynamicLocation) -> String {
        Self::static_binop(StaticLocation::minus, dest, lhs, rhs)
    }
    pub fn times(dest: DynamicLocation, lhs: DynamicLocation, rhs: DynamicLocation) -> String {
        Self::static_binop(StaticLocation::times, dest, lhs, rhs)
    }
    pub fn divide(dest: DynamicLocation, lhs: DynamicLocation, rhs: DynamicLocation) -> String {
        Self::static_binop(StaticLocation::divide, dest, lhs, rhs)
    }

    pub fn equals(dest: DynamicLocation, lhs: DynamicLocation, rhs: DynamicLocation) -> String {
        Self::static_binop(StaticLocation::equals, dest, lhs, rhs)
    }

    pub fn not_equals(dest: DynamicLocation, lhs: DynamicLocation, rhs: DynamicLocation) -> String {
        Self::static_binop(StaticLocation::not_equals, dest, lhs, rhs)
    }

    pub fn stack_deref(self) -> Self {
        if let Self::Static(loc) = self {
            Self::DerefStack(loc)
        } else {
            panic!("Cannot dereference a non-static location")
        }
    }

    pub fn heap_deref(self) -> Self {
        if let Self::Static(loc) = self {
            Self::DerefHeap(loc)
        } else {
            panic!("Cannot dereference a non-static location")
        }
    }
}

impl std::fmt::Display for StaticLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StaticLocation::Address(addr) => write!(f, "@{addr}"),
            StaticLocation::Named { name, addr } => write!(f, "{name}@{addr}"),
        }
    }
}
impl std::fmt::Display for DynamicLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DynamicLocation::Static(loc) => write!(f, "{loc}"),
            DynamicLocation::DerefStack(loc) => write!(f, "[{loc}]"),
            DynamicLocation::DerefHeap(loc) => write!(f, "(heap) [{loc}]"),
        }
    }
}

impl From<StaticLocation> for DynamicLocation {
    fn from(value: StaticLocation) -> Self {
        Self::Static(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    lazy_static! {
        static ref COMPILE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    }
    fn compile_and_run(filename: &str) {
        let lock = COMPILE_LOCK.lock().unwrap();

        // // super-bf ./test.b -e; ./main.exe
        // let mut cmd = std::process::Command::new("super-bf")
        //     .arg(filename)
        //     .arg("-e")
        //     .spawn()
        //     .expect("Failed to execute command");
        let mut cmd = std::process::Command::new("./compile")
            .arg(filename)
            .arg("output.c")
            .spawn()
            .expect("Failed to execute command");
        let output = cmd.wait_with_output().expect("Failed to read stdout");
        assert!(
            output.status.success(),
            "Command failed with status: {}",
            output.status
        );
        drop(lock);
        // // Now run the generated file
        // let mut cmd = std::process::Command::new("./main.exe")
        //     .spawn()
        //     .expect("Failed to execute command");
        // let output = cmd.wait_with_output().expect("Failed to read stdout");
        // assert!(output.status.success(), "Command failed with status: {}", output.status);
    }

    #[test]
    fn test_table() {
        let val = global_alloc(1);
        let idx = global_alloc(1);
        let dst = global_alloc(1);
        println!("idx: {idx:?}");
        println!("val: {val:?}");
        println!("dst: {dst:?}");

        let table = Table::allocate(10);

        println!("{table:#?}");

        let text = "Hi world!";

        for (i, byte) in text.bytes().enumerate() {
            println!("{}", idx.set_const(i as u64));
            println!("{}", val.set_const(byte as u64));
            println!("{}", table.set(idx, val));
        }

        for i in 0..text.len() {
            println!("{}", idx.set_const(i as u64));
            println!("{}", table.get(idx, val));
            println!("{}", val.putchar());
        }

        println!("{}", val.set_const(10));
        println!("{}", val.putchar());
    }

    #[test]
    fn test_divide() {
        allocate_registers_and_stack();

        let a = global_alloc(1);
        let b = global_alloc(1);
        let c = global_alloc(1);

        let mut result = String::new();
        // result += &a.set_const(0xff);
        // result += &a.get(b);
        result += &b.set_const(20);
        result += &c.set_const(2);
        result += "#";
        result += &StaticLocation::divide(a, b, c);
        result += "#";

        // println!("{result}");

        std::fs::File::create("test-math.b")
            .expect("Failed to create file")
            .write_all(result.as_bytes())
            .unwrap();

        compile_and_run("test-math.b");
    }

    #[test]
    fn test_multiply() {
        allocate_registers_and_stack();

        let a = global_alloc(1);
        let b = global_alloc(1);
        let c = global_alloc(1);

        let mut result = String::new();
        // result += &a.set_const(0xff);
        // result += &a.get(b);
        result += &b.set_const(20);
        result += &c.set_const(2);
        result += "#";
        result += &StaticLocation::times(a, b, c);
        result += "$";

        // println!("{result}");

        std::fs::File::create("test-math.b")
            .expect("Failed to create file")
            .write_all(result.as_bytes())
            .unwrap();

        compile_and_run("test-math.b");
    }

    #[test]
    fn test_subtract() {
        allocate_registers_and_stack();

        let a = global_alloc(1);
        let b = global_alloc(1);
        let c = global_alloc(1);

        let mut result = String::new();
        // result += &a.set_const(0xff);
        // result += &a.get(b);
        result += &b.set_const(20);
        result += &c.set_const(2);
        result += "#";
        result += &StaticLocation::minus(b, b, c);
        result += "$";

        // println!("{result}");

        std::fs::File::create("test-math.b")
            .expect("Failed to create file")
            .write_all(result.as_bytes())
            .unwrap();

        compile_and_run("test-math.b");
    }

    #[test]
    fn test_putint() {
        allocate_registers_and_stack();
        let a = global_alloc(1);
        let b = global_alloc(1);
        let c = global_alloc(1);
        let newline = global_alloc(1);

        let mut result = String::new();
        // result += &a.get(b);
        result += &a.set_const(2);
        result += &b.set_const(3);
        result += &c.set_const(7);
        result += &newline.set_const(10);

        // println!("{result}");
        result += &a.putint();
        result += &newline.putchar();
        result += &newline.putchar();
        result += &b.putint();
        result += &newline.putchar();
        result += &newline.putchar();
        result += &c.putint();
        result += &newline.putchar();
        result += &newline.putchar();

        std::fs::File::create("test-putint.b")
            .expect("Failed to create file")
            .write_all(result.as_bytes())
            .unwrap();

        compile_and_run("test-putint.b");
    }

    #[test]
    fn test_equals() {
        // allocate_registers_and_stack();
        let a = global_alloc(1);
        let b = global_alloc(1);
        let c = global_alloc(1);

        let mut result = String::new();
        // result += &a.set_const(0xff);
        // result += &a.get(b);
        result += &b.set_const(20);
        result += &c.set_const(20);
        result += "#";
        result += &StaticLocation::equals(a, b, c);
        result += "#";

        // println!("{result}");

        std::fs::File::create("test-equals.b")
            .expect("Failed to create file")
            .write_all(result.as_bytes())
            .unwrap();

        compile_and_run("test-equals.b");
    }

    #[test]
    fn test_if_stmt() {
        allocate_registers_and_stack();
        let a = global_alloc(1);
        let b = global_alloc(1);
        let c = global_alloc(1);

        let mut result = String::new();
        result += &b.set_const(20);
        result += &c.set_const(20);
        result += "$";
        result += &StaticLocation::equals(a, b, c);
        result += &if_stmt(&a, TRASH.putmsg("b == c\n"));
        result += &a.dec();
        result += &if_stmt(&a, TRASH.putmsg("b != c\n"));
        result += "$";

        // println!("{result}");

        std::fs::File::create("test-if-stmt.b")
            .expect("Failed to create file")
            .write_all(result.as_bytes())
            .unwrap();

        compile_and_run("test-if-stmt.b");
    }
}
