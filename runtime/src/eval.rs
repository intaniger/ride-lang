//! The evaluator.
//!
//! One loop over a program counter, on three fixed arrays. No allocation, no heap, no recursion
//! in the evaluator itself.
//!
//! The loop is driven by an explicit `pc` rather than by iterating the instruction slice,
//! because an iterator cannot jump. That single change is what the branch instructions need.
//!
//! Nothing here checks an index. Every index is in range because `Verified` can only be built
//! by `verify`, and the two proofs there are exactly the guarantees this loop relies on:
//!
//! * Proof A bounds `top`, so the operand stack never overflows and never underflows.
//! * Proof B bounds `fp` and `rp`, so the frame array and the return stack never overflow.
//! * Forward-only jumps and an acyclic call graph bound the number of steps, so the loop ends.

use crate::op::{Op, Program};
use crate::verify::{verify, Bounds, VerifyError, MAX_CALLS, MAX_FRAME, MAX_STACK};

/// A saved call site: where to resume, and which frame window to resume into.
///
/// Both halves are needed. Saving only `pc` would leave the caller reading the callee's locals
/// after the return.
#[derive(Clone, Copy, Debug)]
struct Resume {
    pc: u32,
    fp: u32,
}

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
    let mut frame = [0.0f32; MAX_FRAME];
    let mut calls = [Resume { pc: 0, fp: 0 }; MAX_CALLS];

    let mut top = 0usize; // live operand count; the top value is stack[top - 1]
    let mut fp = 0usize; // base of the running function's frame window
    let mut rp = 0usize; // live entries on the return stack
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
            Op::LoadLocal(i) => {
                stack[top] = frame[fp + i as usize];
                top += 1;
                pc += 1;
            }
            Op::StoreLocal(i) => {
                top -= 1;
                frame[fp + i as usize] = stack[top];
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

            // ── comparison ──────────────────────────────────────────────────
            // 1.0 for true, 0.0 for false, so a truth value needs no extra representation.
            Op::Lt => {
                top -= 1;
                stack[top - 1] = bool_f32(stack[top - 1] < stack[top]);
                pc += 1;
            }
            Op::Gt => {
                top -= 1;
                stack[top - 1] = bool_f32(stack[top - 1] > stack[top]);
                pc += 1;
            }
            Op::Le => {
                top -= 1;
                stack[top - 1] = bool_f32(stack[top - 1] <= stack[top]);
                pc += 1;
            }
            Op::Ge => {
                top -= 1;
                stack[top - 1] = bool_f32(stack[top - 1] >= stack[top]);
                pc += 1;
            }
            Op::Eq => {
                top -= 1;
                stack[top - 1] = bool_f32(stack[top - 1] == stack[top]);
                pc += 1;
            }

            // ── control flow ────────────────────────────────────────────────
            Op::Jump(t) => {
                pc = t as usize;
            }
            Op::JumpIfFalse(t) => {
                top -= 1;
                pc = if stack[top] == 0.0 { t as usize } else { pc + 1 };
            }

            Op::Call {
                target,
                arity,
                fp_delta,
            } => {
                let n = arity as usize;
                top -= n; // take the arguments off the operand stack
                calls[rp] = Resume {
                    pc: (pc + 1) as u32,
                    fp: fp as u32,
                };
                rp += 1;
                fp += fp_delta as usize; // the callee's window sits above the caller's
                frame[fp..fp + n].copy_from_slice(&stack[top..top + n]);
                pc = target as usize;
            }
            Op::Ret => {
                if rp == 0 {
                    // The entry function has returned. Its single value is the result.
                    return stack[0];
                }
                rp -= 1;
                let back = calls[rp];
                pc = back.pc as usize;
                fp = back.fp as usize;
                // The return value stays where the callee left it, which is exactly where the
                // caller expects its operand.
            }
        }
    }
}

#[inline]
fn bool_f32(b: bool) -> f32 {
    if b {
        1.0
    } else {
        0.0
    }
}
