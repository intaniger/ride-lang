//! The two bound proofs.
//!
//! Bytecode may reach this runtime from a source the runtime does not control. Nothing may be
//! evaluated until both proofs pass, because the evaluator's array indexing has no run-time
//! guard — the proofs ARE the guard.
//!
//! **Proof A — the operand stack, per function.**
//!
//! A single accumulating pass over the instruction array is sound only for straight-line code.
//! It assumes the next instruction always runs after this one, and a jump breaks that: the
//! running total then describes no real execution.
//!
//! The fix is a height table checked at the target instead of accumulated. Each jump writes the
//! height it leaves behind into `expected[target]`. On arrival at an index that holds a recorded
//! height, the height carried must equal it. Two jumps to one target must agree. This is the
//! same rule a WebAssembly validator applies at a block boundary, and it is sound here only
//! because jumps are forward-only.
//!
//! **Proof B — the frame array and the return stack, across the call graph.**
//!
//! Proof A bounds one function. It says nothing about how many frames are live at once. The
//! language forbids recursion, so the call graph is acyclic — and `verify` is where that stops
//! being a convention and becomes a checked fact. With no cycle the graph is a DAG, so the
//! frame bound is the heaviest root-to-leaf path and the return-stack bound is the longest one.

use crate::op::{Func, Op, Program};

/// Instruction ceiling for one program.
pub const MAX_CODE: usize = 4096;
/// Operand-stack ceiling. Proof A must fit inside it.
pub const MAX_STACK: usize = 32;
/// Frame-array ceiling. Proof B must fit inside it.
pub const MAX_FRAME: usize = 64;
/// Return-stack ceiling, so call depth. Proof B must fit inside it.
pub const MAX_CALLS: usize = 16;
/// Function-table ceiling. Bounds the verifier's own recursion over the call graph.
pub const MAX_FUNCS: usize = 256;

/// Why a program was rejected.
///
/// Each variant has a stable string code. Errors leave this runtime as codes, never as a panic
/// and never as a formatted message, so a host can branch on one without parsing prose.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerifyError {
    NoFunctions,
    TooManyFunctions,
    CodeTooLong,
    FuncOutOfRange,
    MissingRet,
    BadStateIndex,
    BadLocalIndex,
    BadJumpTarget,
    BackwardJump,
    HeightMismatch,
    Unreachable,
    StackUnderflow,
    StackOverflow,
    NotOneResult,
    BadCallTarget,
    ArityMismatch,
    FrameDeltaMismatch,
    CallGraphCycle,
    FrameOverflow,
    CallDepthOverflow,
}

impl VerifyError {
    /// The stable code for this error.
    pub fn code(&self) -> &'static str {
        match self {
            VerifyError::NoFunctions => "no_functions",
            VerifyError::TooManyFunctions => "too_many_functions",
            VerifyError::CodeTooLong => "code_too_long",
            VerifyError::FuncOutOfRange => "func_out_of_range",
            VerifyError::MissingRet => "missing_ret",
            VerifyError::BadStateIndex => "bad_state_index",
            VerifyError::BadLocalIndex => "bad_local_index",
            VerifyError::BadJumpTarget => "bad_jump_target",
            VerifyError::BackwardJump => "backward_jump",
            VerifyError::HeightMismatch => "height_mismatch",
            VerifyError::Unreachable => "unreachable",
            VerifyError::StackUnderflow => "stack_underflow",
            VerifyError::StackOverflow => "stack_overflow",
            VerifyError::NotOneResult => "not_one_result",
            VerifyError::BadCallTarget => "bad_call_target",
            VerifyError::ArityMismatch => "arity_mismatch",
            VerifyError::FrameDeltaMismatch => "frame_delta_mismatch",
            VerifyError::CallGraphCycle => "call_graph_cycle",
            VerifyError::FrameOverflow => "frame_overflow",
            VerifyError::CallDepthOverflow => "call_depth_overflow",
        }
    }
}

/// What the two proofs established.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bounds {
    /// Deepest operand stack any path reaches. Proof A.
    pub stack: usize,
    /// Total frame slots live at once on the heaviest call path. Proof B.
    pub frame: usize,
    /// Deepest call nesting. Proof B.
    pub calls: usize,
}

/// Run both proofs. This is the only way to obtain `Bounds`, and `Verified` is the only thing
/// the evaluator accepts.
pub fn verify(p: &Program) -> Result<Bounds, VerifyError> {
    if p.funcs.is_empty() {
        return Err(VerifyError::NoFunctions);
    }
    if p.funcs.len() > MAX_FUNCS {
        return Err(VerifyError::TooManyFunctions);
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

    // Proof B, once for the whole graph.
    let (frame, calls) = verify_call_graph(p)?;
    if frame > MAX_FRAME {
        return Err(VerifyError::FrameOverflow);
    }
    if calls > MAX_CALLS {
        return Err(VerifyError::CallDepthOverflow);
    }

    Ok(Bounds {
        stack,
        frame,
        calls,
    })
}

// ── Proof A ─────────────────────────────────────────────────────────────────────

/// Record the height a jump leaves behind, at its target.
///
/// Rejects a target outside this function and a target that is not strictly ahead. Rejects a
/// second write that disagrees with the first — that is two paths reaching one point with
/// different stacks, which no well-formed program does.
fn record(
    expected: &mut [Option<i64>],
    lo: usize,
    hi: usize,
    pc: usize,
    target: u32,
    height: i64,
) -> Result<(), VerifyError> {
    let t = target as usize;
    if t < lo || t >= hi {
        return Err(VerifyError::BadJumpTarget);
    }
    if t <= pc {
        return Err(VerifyError::BackwardJump);
    }
    let slot = &mut expected[t - lo];
    match *slot {
        None => {
            *slot = Some(height);
            Ok(())
        }
        Some(e) if e == height => Ok(()),
        Some(_) => Err(VerifyError::HeightMismatch),
    }
}

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
            Op::LoadLocal(ix) => {
                if ix >= f.frame {
                    return Err(VerifyError::BadLocalIndex);
                }
                cur += 1;
            }
            Op::StoreLocal(ix) => {
                if ix >= f.frame {
                    return Err(VerifyError::BadLocalIndex);
                }
                need!(1);
                cur -= 1;
            }

            // Binary: pop two, push one.
            Op::Add
            | Op::Sub
            | Op::Mul
            | Op::Div
            | Op::Pow
            | Op::Max
            | Op::Min
            | Op::Step
            | Op::Lt
            | Op::Gt
            | Op::Le
            | Op::Ge
            | Op::Eq => {
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

            Op::Jump(t) => {
                record(&mut expected, lo, hi, pc, t, cur)?;
                height = None; // this path ends here
                continue;
            }
            Op::JumpIfFalse(t) => {
                need!(1);
                cur -= 1; // the test is consumed either way
                record(&mut expected, lo, hi, pc, t, cur)?;
                height = Some(cur); // the fall-through path continues
                continue;
            }

            Op::Call {
                target,
                arity,
                fp_delta,
            } => {
                let callee = p
                    .funcs
                    .iter()
                    .find(|g| g.entry == target)
                    .ok_or(VerifyError::BadCallTarget)?;
                if callee.arity != arity {
                    return Err(VerifyError::ArityMismatch);
                }
                // The callee's window sits just above the caller's, so the immediate must be
                // the caller's own frame size. Proof B accounts for the callee's window.
                if fp_delta != f.frame {
                    return Err(VerifyError::FrameDeltaMismatch);
                }
                need!(arity as i64);
                cur -= arity as i64;
                cur += 1; // the callee's Ret leaves exactly one value
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

// ── Proof B ─────────────────────────────────────────────────────────────────────

/// Build the call graph, reject any cycle, and return `(frame slots, call depth)` for the
/// heaviest path.
fn verify_call_graph(p: &Program) -> Result<(usize, usize), VerifyError> {
    let n = p.funcs.len();
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (fi, f) in p.funcs.iter().enumerate() {
        for pc in f.entry as usize..f.end() as usize {
            if let Op::Call { target, .. } = p.code[pc] {
                let callee = p
                    .funcs
                    .iter()
                    .position(|g| g.entry == target)
                    .ok_or(VerifyError::BadCallTarget)?;
                if !adj[fi].contains(&callee) {
                    adj[fi].push(callee);
                }
            }
        }
    }

    // 0 = unvisited, 1 = on the current path, 2 = finished.
    let mut colour = vec![0u8; n];
    let mut best = vec![(0usize, 0usize); n];

    let mut frame = 0usize;
    let mut calls = 0usize;
    for i in 0..n {
        let (f, d) = walk(i, &adj, &p.funcs, &mut colour, &mut best)?;
        frame = frame.max(f);
        calls = calls.max(d);
    }
    Ok((frame, calls))
}

/// Depth-first walk that detects a cycle and memoises the heaviest tail from each node.
///
/// Meeting a node already on the current path is a cycle, so the program recurses and its
/// frame use cannot be bounded. That is the whole reason recursion is forbidden.
fn walk(
    i: usize,
    adj: &[Vec<usize>],
    funcs: &[Func],
    colour: &mut [u8],
    best: &mut [(usize, usize)],
) -> Result<(usize, usize), VerifyError> {
    match colour[i] {
        1 => return Err(VerifyError::CallGraphCycle),
        2 => return Ok(best[i]),
        _ => {}
    }
    colour[i] = 1;

    let mut tail_frame = 0usize;
    let mut tail_depth = 0usize;
    for &j in &adj[i] {
        let (f, d) = walk(j, adj, funcs, colour, best)?;
        tail_frame = tail_frame.max(f);
        tail_depth = tail_depth.max(d + 1);
    }

    colour[i] = 2;
    best[i] = (funcs[i].frame as usize + tail_frame, tail_depth);
    Ok(best[i])
}
