//! The bound proof.
//!
//! Bytecode may reach this runtime from a source the runtime does not control. Nothing may be
//! evaluated until the proof passes, because the evaluator's array indexing has no run-time
//! guard — the proof IS the guard.
//!
//! **Proof A — the operand stack, per function.**
//!
//! Walk the function once and track how many values sit on the stack. Each instruction pops a
//! known number and pushes a known number, so the height after it is known. The walk rejects an
//! underflow, an overflow, and a function that does not leave exactly one result.
//!
//! The walk keeps a table of heights, one slot per instruction, and checks it on arrival at an
//! index that holds a recorded height. Straight-line code records only the entry. Control flow
//! will write the rest.

use crate::op::{Op, Program};

/// Instruction ceiling for one program.
pub const MAX_CODE: usize = 4096;
/// Operand-stack ceiling. Proof A must fit inside it.
pub const MAX_STACK: usize = 32;

/// Why a program was rejected.
///
/// Each variant has a stable string code. Errors leave this runtime as codes, never as a panic
/// and never as a formatted message, so a host can branch on one without parsing prose.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerifyError {
    NoFunctions,
    CodeTooLong,
    FuncOutOfRange,
    MissingRet,
    BadStateIndex,
    BadLocalIndex,
    HeightMismatch,
    Unreachable,
    StackUnderflow,
    StackOverflow,
    NotOneResult,
}

impl VerifyError {
    /// The stable code for this error.
    pub fn code(&self) -> &'static str {
        match self {
            VerifyError::NoFunctions => "no_functions",
            VerifyError::CodeTooLong => "code_too_long",
            VerifyError::FuncOutOfRange => "func_out_of_range",
            VerifyError::MissingRet => "missing_ret",
            VerifyError::BadStateIndex => "bad_state_index",
            VerifyError::BadLocalIndex => "bad_local_index",
            VerifyError::HeightMismatch => "height_mismatch",
            VerifyError::Unreachable => "unreachable",
            VerifyError::StackUnderflow => "stack_underflow",
            VerifyError::StackOverflow => "stack_overflow",
            VerifyError::NotOneResult => "not_one_result",
        }
    }
}

/// What the proof established.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bounds {
    /// Deepest operand stack any path reaches. Proof A.
    pub stack: usize,
}

/// Run the proof. This is the only way to obtain `Bounds`, and `Verified` is the only thing
/// the evaluator accepts.
pub fn verify(p: &Program) -> Result<Bounds, VerifyError> {
    if p.funcs.is_empty() {
        return Err(VerifyError::NoFunctions);
    }
    if p.code.len() > MAX_CODE {
        return Err(VerifyError::CodeTooLong);
    }

    // Proof A, once per function.
    let mut stack = 0usize;
    for fi in 0..p.funcs.len() {
        stack = stack.max(verify_func(p, fi)?);
    }
    if stack > MAX_STACK {
        return Err(VerifyError::StackOverflow);
    }

    Ok(Bounds { stack })
}

// ── Proof A ─────────────────────────────────────────────────────────────────────

/// Proof A for one function. Returns the deepest operand stack any path through it reaches.
fn verify_func(p: &Program, fi: usize) -> Result<usize, VerifyError> {
    let f = p.funcs[fi];
    let lo = f.entry as usize;
    let hi = f.end() as usize;
    if lo >= hi || hi > p.code.len() {
        return Err(VerifyError::FuncOutOfRange);
    }
    if p.code[hi - 1] != Op::Ret {
        return Err(VerifyError::MissingRet);
    }
    if f.arity as usize > f.frame as usize {
        // Parameters occupy the first `arity` frame slots, so the window cannot be narrower.
        return Err(VerifyError::BadLocalIndex);
    }

    let mut expected: Vec<Option<i64>> = vec![None; hi - lo];
    // A function is entered with an empty operand stack. Its arguments are already in the frame.
    expected[0] = Some(0);

    // `None` means the height is not known here: either nothing reaches this index, or the
    // previous instruction ended a path (an unconditional jump, or a return).
    let mut height: Option<i64> = None;
    let mut max: i64 = 0;

    for pc in lo..hi {
        let i = pc - lo;

        // A join point. Agree with what was recorded, or adopt it after a path ended.
        if let Some(e) = expected[i] {
            match height {
                None => height = Some(e),
                Some(cur) if cur != e => return Err(VerifyError::HeightMismatch),
                Some(_) => {}
            }
        }
        let mut cur = match height {
            Some(c) => c,
            // Nothing can reach this instruction. The compiler never emits dead code, so this
            // is a malformed program rather than a missed optimisation.
            None => return Err(VerifyError::Unreachable),
        };

        // `need` guards every pop. Without it a malformed program would index below the stack.
        macro_rules! need {
            ($n:expr) => {
                if cur < $n {
                    return Err(VerifyError::StackUnderflow);
                }
            };
        }

        match p.code[pc] {
            Op::Push(_) => cur += 1,
            Op::LoadState(ix) => {
                if ix >= p.state_arity {
                    return Err(VerifyError::BadStateIndex);
                }
                cur += 1;
            }
            // Binary: pop two, push one.
            Op::Add
            | Op::Sub
            | Op::Mul
            | Op::Div
            | Op::Pow
            | Op::Max
            | Op::Min
            | Op::Step => {
                need!(2);
                cur -= 1;
            }

            // Unary: rewrite in place.
            Op::Neg | Op::Sin | Op::Cos | Op::Sqrt | Op::Abs | Op::Sign | Op::Exp => {
                need!(1);
            }

            // Ternary: pop three, push one.
            Op::Mix | Op::Clamp => {
                need!(3);
                cur -= 2;
            }

            // Quaternary: pop four, push one.
            Op::Select => {
                need!(4);
                cur -= 3;
            }

            Op::Ret => {
                // A function returns exactly one value, so its last instruction must find
                // exactly one. This is also what makes `Call`'s "+1" above true.
                if cur != 1 {
                    return Err(VerifyError::NotOneResult);
                }
                height = None;
                continue;
            }
        }

        max = max.max(cur);
        height = Some(cur);
    }

    Ok(max as usize)
}
