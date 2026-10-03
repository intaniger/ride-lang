//! The evaluator.
//!
//! One loop over a program counter, on one fixed array. No allocation, no heap, no recursion in
//! the evaluator itself.
//!
//! Nothing here checks an index. Every index is in range because `Verified` can only be built
//! by `verify`, and the proof there is exactly the guarantee this loop relies on:
//!
//! * Proof A bounds `top`, so the operand stack never overflows and never underflows.
//! * The program has no jump, so every instruction runs once and the loop ends at `Ret`.

use crate::op::{Op, Program};
use crate::verify::{verify, Bounds, VerifyError, MAX_STACK};

/// The one error evaluation can produce.
///
/// Everything else is proven at verification time. This single case remains because the length
/// of the STATE record is a property of the *call*, not of the program.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvalError {
    StateTooShort,
}

impl EvalError {
    pub fn code(&self) -> &'static str {
        match self {
            EvalError::StateTooShort => "state_too_short",
        }
    }
}

/// A program that has passed both proofs.
///
/// This type cannot be built any other way, so holding one is the proof that evaluating it is
/// safe. That is the whole point of keeping it separate from `Program`.
#[derive(Clone, Debug)]
pub struct Verified {
    program: Program,
    bounds: Bounds,
}

impl Verified {
    /// Run both proofs over `program` and keep the result.
    pub fn new(program: Program) -> Result<Self, VerifyError> {
        let bounds = verify(&program)?;
        Ok(Verified { program, bounds })
    }

    /// What the proofs established. Useful to a host that wants to report the real cost of a
    /// program rather than the ceiling.
    pub fn bounds(&self) -> Bounds {
        self.bounds
    }

    pub fn program(&self) -> &Program {
        &self.program
    }

    /// Evaluate the entry function against one STATE record.
    pub fn eval(&self, state: &[f32]) -> Result<f32, EvalError> {
        if state.len() < self.program.state_arity as usize {
            return Err(EvalError::StateTooShort);
        }
        Ok(run(&self.program, state))
    }
}

/// The dispatch loop. Separate from `eval` so the bounds check happens exactly once.
fn run(p: &Program, state: &[f32]) -> f32 {
    let mut stack = [0.0f32; MAX_STACK];

    let mut top = 0usize; // live operand count; the top value is stack[top - 1]
    let mut pc = p.funcs[0].entry as usize;

    loop {
        match p.code[pc] {
            // ── operands ────────────────────────────────────────────────────
            Op::Push(v) => {
                stack[top] = v;
                top += 1;
                pc += 1;
            }
            Op::LoadState(i) => {
                stack[top] = state[i as usize];
                top += 1;
                pc += 1;
            }

            // ── arithmetic ──────────────────────────────────────────────────
            Op::Add => {
                top -= 1;
                stack[top - 1] += stack[top];
                pc += 1;
            }
            Op::Sub => {
                top -= 1;
                stack[top - 1] -= stack[top];
                pc += 1;
            }
            Op::Mul => {
                top -= 1;
                stack[top - 1] *= stack[top];
                pc += 1;
            }
            Op::Div => {
                top -= 1;
                stack[top - 1] /= stack[top];
                pc += 1;
            }
            Op::Pow => {
                top -= 1;
                stack[top - 1] = stack[top - 1].powf(stack[top]);
                pc += 1;
            }
            Op::Neg => {
                stack[top - 1] = -stack[top - 1];
                pc += 1;
            }

            // ── builtins ────────────────────────────────────────────────────
            Op::Sin => {
                stack[top - 1] = stack[top - 1].sin();
                pc += 1;
            }
            Op::Cos => {
                stack[top - 1] = stack[top - 1].cos();
                pc += 1;
            }
            Op::Sqrt => {
                stack[top - 1] = stack[top - 1].sqrt();
                pc += 1;
            }
            Op::Abs => {
                stack[top - 1] = stack[top - 1].abs();
                pc += 1;
            }
            Op::Sign => {
                // 0.0 maps to 0.0, not to 1.0 — `f32::signum` returns 1.0 for +0.0, which is
                // not the sign function this language means.
                let x = stack[top - 1];
                stack[top - 1] = if x == 0.0 { 0.0 } else { x.signum() };
                pc += 1;
            }
            Op::Exp => {
                stack[top - 1] = stack[top - 1].exp();
                pc += 1;
            }
            Op::Max => {
                top -= 1;
                stack[top - 1] = stack[top - 1].max(stack[top]);
                pc += 1;
            }
            Op::Min => {
                top -= 1;
                stack[top - 1] = stack[top - 1].min(stack[top]);
                pc += 1;
            }
            Op::Step => {
                // step(edge, x): postfix pushes edge then x, so x is on top.
                top -= 1;
                let x = stack[top];
                let edge = stack[top - 1];
                stack[top - 1] = if x < edge { 0.0 } else { 1.0 };
                pc += 1;
            }
            Op::Mix => {
                // mix(a, b, t): read all three before shrinking, then write into a's slot.
                let t = stack[top - 1];
                let b = stack[top - 2];
                let a = stack[top - 3];
                top -= 2;
                stack[top - 1] = (1.0 - t) * a + t * b;
                pc += 1;
            }
            Op::Clamp => {
                let hi = stack[top - 1];
                let lo = stack[top - 2];
                let x = stack[top - 3];
                top -= 2;
                stack[top - 1] = x.max(lo).min(hi);
                pc += 1;
            }
            Op::Select => {
                let b = stack[top - 1];
                let a = stack[top - 2];
                let x = stack[top - 3];
                let edge = stack[top - 4];
                top -= 3;
                stack[top - 1] = if x < edge { a } else { b };
                pc += 1;
            }

            // ── control flow ────────────────────────────────────────────────
            Op::Ret => {
                // The entry function has returned. Its single value is the result.
                return stack[0];
            }
        }
    }
}
