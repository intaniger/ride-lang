//! The instruction set.
//!
//! A `ride` program is postfix bytecode for a stack machine. It takes a STATE record of numbers
//! and returns one number. There is no heap, no allocation per call, and no garbage collector.
//!
//! One property of the language shapes this set so far:
//!
//! * **Strict.** Operands are evaluated before the operation that consumes them, so the reading
//!   order is the evaluation order.
//!
//! Every variant owes its stack effect to `verify`, whose `match` is exhaustive with no
//! wildcard. Adding a variant here is therefore a compile error there, not a silent wrong proof.

/// One instruction. `Copy` so the dispatch loop reads by value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Op {
    // ── operands ────────────────────────────────────────────────────────────
    /// Push a literal.
    Push(f32),
    /// Push `state[i]` — one field of the record the host writes once per call.
    LoadState(u8),

    // ── arithmetic ──────────────────────────────────────────────────────────
    Add,
    Sub,
    Mul,
    Div,
    /// `f32::powf`. Spelled `^` in the surface language.
    Pow,
    Neg,

    // ── builtins ────────────────────────────────────────────────────────────
    // Six unary, three binary, two ternary, one quaternary.
    Sin,
    Cos,
    Sqrt,
    Abs,
    Sign,
    Exp,
    Max,
    Min,
    /// `step(edge, x)` = 0.0 when `x < edge`, else 1.0.
    Step,
    /// `mix(a, b, t)` = `(1 - t) * a + t * b`.
    Mix,
    /// `clamp(x, lo, hi)`.
    Clamp,
    /// `select(edge, x, a, b)` = `a` when `x < edge`, else `b`.
    Select,

    // ── control flow ────────────────────────────────────────────────────────
    /// Return. The result stays on the operand stack.
    Ret,
}

/// One compiled function.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Func {
    /// Index of this function's first instruction in `Program::code`.
    pub entry: u32,
    /// Instruction count, including the trailing `Ret`.
    pub len: u32,
    /// Parameter count.
    pub arity: u8,
    /// Frame slots: parameters first, then local bindings.
    pub frame: u8,
}

impl Func {
    /// One past this function's last instruction.
    pub fn end(&self) -> u32 {
        self.entry + self.len
    }
}

/// A compiled program: the code, the function table, and the width of the STATE record.
///
/// `funcs[0]` is the entry point. `eval` starts there.
#[derive(Clone, Debug, PartialEq)]
pub struct Program {
    pub code: Vec<Op>,
    pub funcs: Vec<Func>,
    /// Field count of the STATE record. `LoadState(i)` is in range when `i < state_arity`.
    pub state_arity: u8,
}
