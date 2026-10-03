//! Runtime tests.
//!
//! Three groups:
//!   1. the evaluator produces the right numbers,
//!   2. the proof accepts well-formed programs and reports the right bounds,
//!   3. the proof REJECTS malformed programs — the group that matters most, because the
//!      evaluator indexes its array with no run-time guard.

use ride_runtime::{EvalError, Func, Op, Program, Verified, VerifyError};

/// A single-function program over `state_arity` fields.
fn one(code: Vec<Op>, state_arity: u8, frame: u8) -> Program {
    let len = code.len() as u32;
    Program {
        code,
        funcs: vec![Func {
            entry: 0,
            len,
            arity: 0,
            frame,
        }],
        state_arity,
    }
}

fn eval(code: Vec<Op>, state: &[f32]) -> f32 {
    Verified::new(one(code, state.len() as u8, 0))
        .expect("program should verify")
        .eval(state)
        .expect("state should be long enough")
}

// ── 1. the evaluator ────────────────────────────────────────────────────────────

#[test]
fn arithmetic() {
    // 2 + 3 * 4  (postfix: 2 3 4 * +)
    let got = eval(
        vec![
            Op::Push(2.0),
            Op::Push(3.0),
            Op::Push(4.0),
            Op::Mul,
            Op::Add,
            Op::Ret,
        ],
        &[],
    );
    assert_eq!(got, 14.0);
}

#[test]
fn the_rest_of_the_arithmetic() {
    // Sub and Div are asymmetric, so these also pin the operand order: the top is the right.
    assert_eq!(eval(vec![Op::Push(7.0), Op::Push(2.0), Op::Sub, Op::Ret], &[]), 5.0);
    assert_eq!(eval(vec![Op::Push(7.0), Op::Push(2.0), Op::Div, Op::Ret], &[]), 3.5);
    assert_eq!(eval(vec![Op::Push(2.0), Op::Push(10.0), Op::Pow, Op::Ret], &[]), 1024.0);
    assert_eq!(eval(vec![Op::Push(3.0), Op::Neg, Op::Ret], &[]), -3.0);
}

#[test]
fn the_other_builtins_keep_their_published_meaning() {
    let un = |op: Op, x: f32| eval(vec![Op::Push(x), op, Op::Ret], &[]);
    let bin = |op: Op, a: f32, b: f32| eval(vec![Op::Push(a), Op::Push(b), op, Op::Ret], &[]);
    assert_eq!(un(Op::Abs, -2.0), 2.0);
    assert_eq!(un(Op::Sqrt, 9.0), 3.0);
    assert_eq!(un(Op::Exp, 0.0), 1.0);
    assert_eq!(un(Op::Sin, 0.0), 0.0);
    assert_eq!(un(Op::Cos, 0.0), 1.0);
    assert_eq!(bin(Op::Max, 2.0, 7.0), 7.0);
    assert_eq!(bin(Op::Min, 2.0, 7.0), 2.0);

    // clamp(x, lo, hi)
    let clamp = |x: f32| {
        eval(
            vec![Op::Push(x), Op::Push(0.0), Op::Push(10.0), Op::Clamp, Op::Ret],
            &[],
        )
    };
    assert_eq!(clamp(-1.0), 0.0);
    assert_eq!(clamp(5.0), 5.0);
    assert_eq!(clamp(12.0), 10.0);

    // select(edge, x, a, b) is a below the edge and b at or above it.
    let select = |x: f32| {
        eval(
            vec![
                Op::Push(10.0),
                Op::Push(x),
                Op::Push(1.0),
                Op::Push(2.0),
                Op::Select,
                Op::Ret,
            ],
            &[],
        )
    };
    assert_eq!(select(5.0), 1.0);
    assert_eq!(select(10.0), 2.0);
}

#[test]
fn reads_named_state_fields() {
    // state { a, b, c } ⊢ c - a
    let got = eval(
        vec![Op::LoadState(2), Op::LoadState(0), Op::Sub, Op::Ret],
        &[10.0, 20.0, 35.0],
    );
    assert_eq!(got, 25.0);
}

#[test]
fn step_and_mix_keep_their_published_meaning() {
    // step(edge, x) is 0.0 below the edge and 1.0 at or above it.
    let step = |edge: f32, x: f32| {
        eval(
            vec![Op::Push(edge), Op::Push(x), Op::Step, Op::Ret],
            &[],
        )
    };
    assert_eq!(step(10.0, 9.99), 0.0);
    assert_eq!(step(10.0, 10.0), 1.0);
    assert_eq!(step(10.0, 10.01), 1.0);

    // mix(a, b, t) returns a at t = 0 and b at t = 1.
    let mix = |a: f32, b: f32, t: f32| {
        eval(
            vec![Op::Push(a), Op::Push(b), Op::Push(t), Op::Mix, Op::Ret],
            &[],
        )
    };
    assert_eq!(mix(4.0, 8.0, 0.0), 4.0);
    assert_eq!(mix(4.0, 8.0, 1.0), 8.0);
    assert_eq!(mix(4.0, 8.0, 0.5), 6.0);
}

#[test]
fn sign_of_zero_is_zero() {
    // `f32::signum` returns 1.0 for +0.0. That is not the sign function this language means.
    assert_eq!(eval(vec![Op::Push(0.0), Op::Sign, Op::Ret], &[]), 0.0);
    assert_eq!(eval(vec![Op::Push(-3.0), Op::Sign, Op::Ret], &[]), -1.0);
    assert_eq!(eval(vec![Op::Push(3.0), Op::Sign, Op::Ret], &[]), 1.0);
}

// ── 2. the proof accepts, and reports real bounds ───────────────────────────────

#[test]
fn bounds_report_the_program_not_the_ceiling() {
    let v = Verified::new(one(
        vec![
            Op::Push(1.0),
            Op::Push(2.0),
            Op::Push(3.0),
            Op::Add,
            Op::Add,
            Op::Ret,
        ],
        0,
        0,
    ))
    .unwrap();
    assert_eq!(v.bounds().stack, 3);
}

// ── 3. the proof rejects ────────────────────────────────────────────────────────

/// Assert a program is rejected with a specific error.
fn reject(p: Program, want: VerifyError) {
    match Verified::new(p) {
        Ok(_) => panic!("expected rejection with {:?}, but the program verified", want),
        Err(got) => assert_eq!(got, want, "wrong rejection reason"),
    }
}

#[test]
fn rejects_reading_past_the_state_record() {
    reject(
        one(vec![Op::LoadState(3), Op::Ret], 2, 0),
        VerifyError::BadStateIndex,
    );
}

#[test]
fn rejects_a_stack_underflow() {
    reject(one(vec![Op::Add, Op::Ret], 0, 0), VerifyError::StackUnderflow);
}

#[test]
fn rejects_a_stack_overflow() {
    // `n` values live at once, then `n - 1` adds fold them back to one.
    let deep = |n: usize| {
        let mut code = vec![Op::Push(1.0); n];
        code.extend(vec![Op::Add; n - 1]);
        code.push(Op::Ret);
        one(code, 0, 0)
    };
    // 32 is the ceiling, and it is allowed.
    assert_eq!(Verified::new(deep(32)).unwrap().bounds().stack, 32);
    reject(deep(33), VerifyError::StackOverflow);
}

#[test]
fn rejects_a_function_that_does_not_leave_exactly_one_value() {
    reject(
        one(vec![Op::Push(1.0), Op::Push(2.0), Op::Ret], 0, 0),
        VerifyError::NotOneResult,
    );
}

#[test]
fn rejects_a_missing_ret() {
    reject(one(vec![Op::Push(1.0)], 0, 0), VerifyError::MissingRet);
}

#[test]
fn rejects_an_empty_program() {
    reject(
        Program {
            code: vec![],
            funcs: vec![],
            state_arity: 0,
        },
        VerifyError::NoFunctions,
    );
}

#[test]
fn every_error_has_a_distinct_code() {
    // The codes cross the host boundary, so a collision would make two faults
    // indistinguishable to a caller.
    let all = [
        VerifyError::NoFunctions,
        VerifyError::CodeTooLong,
        VerifyError::FuncOutOfRange,
        VerifyError::MissingRet,
        VerifyError::BadStateIndex,
        VerifyError::BadLocalIndex,
        VerifyError::HeightMismatch,
        VerifyError::Unreachable,
        VerifyError::StackUnderflow,
        VerifyError::StackOverflow,
        VerifyError::NotOneResult,
    ];
    let mut codes: Vec<&str> = all.iter().map(|e| e.code()).collect();
    let total = codes.len();
    codes.sort_unstable();
    codes.dedup();
    assert_eq!(codes.len(), total, "two errors share a code");
}

// ── the one runtime error ───────────────────────────────────────────────────────

#[test]
fn rejects_a_state_record_that_is_too_short() {
    // The only fault the proofs cannot catch, because it is a property of the call.
    let v = Verified::new(one(vec![Op::LoadState(1), Op::Ret], 2, 0)).unwrap();
    assert_eq!(v.eval(&[1.0]), Err(EvalError::StateTooShort));
    assert_eq!(v.eval(&[1.0, 2.0]), Ok(2.0));
}
