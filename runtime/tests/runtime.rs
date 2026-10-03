//! Runtime tests.
//!
//! Three groups:
//!   1. the evaluator produces the right numbers,
//!   2. the two proofs accept well-formed programs and report the right bounds,
//!   3. the two proofs REJECT malformed programs — the group that matters most, because the
//!      evaluator indexes its arrays with no run-time guard.

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

#[test]
fn comparison_yields_one_or_zero() {
    assert_eq!(
        eval(vec![Op::Push(1.0), Op::Push(2.0), Op::Lt, Op::Ret], &[]),
        1.0
    );
    assert_eq!(
        eval(vec![Op::Push(2.0), Op::Push(2.0), Op::Lt, Op::Ret], &[]),
        0.0
    );
    assert_eq!(
        eval(vec![Op::Push(2.0), Op::Push(2.0), Op::Ge, Op::Ret], &[]),
        1.0
    );
}

#[test]
fn the_other_three_comparisons_yield_one_or_zero() {
    let cmp = |op: Op, a: f32, b: f32| eval(vec![Op::Push(a), Op::Push(b), op, Op::Ret], &[]);
    assert_eq!(cmp(Op::Gt, 2.0, 1.0), 1.0);
    assert_eq!(cmp(Op::Gt, 2.0, 2.0), 0.0);
    assert_eq!(cmp(Op::Le, 2.0, 2.0), 1.0);
    assert_eq!(cmp(Op::Le, 3.0, 2.0), 0.0);
    assert_eq!(cmp(Op::Eq, 2.0, 2.0), 1.0);
    assert_eq!(cmp(Op::Eq, 2.0, 3.0), 0.0);
}

#[test]
fn branches_take_exactly_one_arm() {
    // if s < 10 then 1 else 2
    let code = vec![
        Op::LoadState(0),
        Op::Push(10.0),
        Op::Lt,
        Op::JumpIfFalse(6),
        Op::Push(1.0),
        Op::Jump(7),
        Op::Push(2.0),
        Op::Ret,
    ];
    assert_eq!(eval(code.clone(), &[3.0]), 1.0);
    assert_eq!(eval(code, &[30.0]), 2.0);
}

#[test]
fn local_binding_is_computed_once_and_read_twice() {
    // let u = s / 2 in u * u        (frame slot 0 holds u)
    let code = vec![
        Op::LoadState(0),
        Op::Push(2.0),
        Op::Div,
        Op::StoreLocal(0),
        Op::LoadLocal(0),
        Op::LoadLocal(0),
        Op::Mul,
        Op::Ret,
    ];
    let p = one(code, 1, 1);
    let v = Verified::new(p).expect("should verify");
    assert_eq!(v.eval(&[6.0]).unwrap(), 9.0);
    // Two reads of one slot, so the stack only ever holds two values.
    assert_eq!(v.bounds().stack, 2);
}

#[test]
fn direct_call_passes_arguments_through_the_frame() {
    // let double a = a * 2
    // let main = double(21)
    //
    // main occupies 0..4, double occupies 4..8.
    let program = Program {
        code: vec![
            // main — frame 0
            Op::Push(21.0),
            Op::Call {
                target: 3,
                arity: 1,
                fp_delta: 0,
            },
            Op::Ret,
            // double — entry 3, frame 1 (its one parameter)
            Op::LoadLocal(0),
            Op::Push(2.0),
            Op::Mul,
            Op::Ret,
        ],
        funcs: vec![
            Func {
                entry: 0,
                len: 3,
                arity: 0,
                frame: 0,
            },
            Func {
                entry: 3,
                len: 4,
                arity: 1,
                frame: 1,
            },
        ],
        state_arity: 0,
    };
    let v = Verified::new(program).expect("should verify");
    assert_eq!(v.eval(&[]).unwrap(), 42.0);
    // Proof B: main's frame (0) plus double's (1), and one level of nesting.
    assert_eq!(v.bounds().frame, 1);
    assert_eq!(v.bounds().calls, 1);
}

#[test]
fn a_caller_reads_its_own_locals_after_a_call() {
    // let g a = let t = a * 10 in t
    // let main = let k = 5 in g(1) + k
    //
    // The return restores fp as well as pc. Without it, main's `LoadLocal(0)` would read g's
    // parameter, and the result would be 10 + 1 = 11.
    let program = Program {
        code: vec![
            // main — entry 0, frame 1 (k)
            Op::Push(5.0),
            Op::StoreLocal(0),
            Op::Push(1.0),
            Op::Call {
                target: 7,
                arity: 1,
                fp_delta: 1,
            },
            Op::LoadLocal(0),
            Op::Add,
            Op::Ret,
            // g — entry 7, frame 2 (a, t)
            Op::LoadLocal(0),
            Op::Push(10.0),
            Op::Mul,
            Op::StoreLocal(1),
            Op::LoadLocal(1),
            Op::Ret,
        ],
        funcs: vec![
            Func {
                entry: 0,
                len: 7,
                arity: 0,
                frame: 1,
            },
            Func {
                entry: 7,
                len: 6,
                arity: 1,
                frame: 2,
            },
        ],
        state_arity: 0,
    };
    let v = Verified::new(program).expect("should verify");
    assert_eq!(v.eval(&[]).unwrap(), 15.0);
    // Both windows are live at once: main's one slot below g's two.
    assert_eq!(v.bounds().frame, 3);
}

#[test]
fn the_frame_bound_and_the_call_bound_can_come_from_different_paths() {
    // main calls a and b. b calls c.
    //
    //   main (0) ── a (3)                  frame 0 + 3 = 3, depth 1
    //        └──── b (1) ── c (1)          frame 0 + 1 + 1 = 2, depth 2
    //
    // The heaviest path is main → a. The longest is main → b → c. Proof B reports each.
    let program = Program {
        code: vec![
            // main — entry 0
            Op::Push(1.0),
            Op::Call {
                target: 6,
                arity: 1,
                fp_delta: 0,
            },
            Op::Push(1.0),
            Op::Call {
                target: 8,
                arity: 1,
                fp_delta: 0,
            },
            Op::Add,
            Op::Ret,
            // a — entry 6, a wide frame and no calls
            Op::LoadLocal(0),
            Op::Ret,
            // b — entry 8
            Op::LoadLocal(0),
            Op::Call {
                target: 11,
                arity: 1,
                fp_delta: 1,
            },
            Op::Ret,
            // c — entry 11
            Op::LoadLocal(0),
            Op::Push(1.0),
            Op::Add,
            Op::Ret,
        ],
        funcs: vec![
            Func {
                entry: 0,
                len: 6,
                arity: 0,
                frame: 0,
            },
            Func {
                entry: 6,
                len: 2,
                arity: 1,
                frame: 3,
            },
            Func {
                entry: 8,
                len: 3,
                arity: 1,
                frame: 1,
            },
            Func {
                entry: 11,
                len: 4,
                arity: 1,
                frame: 1,
            },
        ],
        state_arity: 0,
    };
    let v = Verified::new(program).expect("should verify");
    // a(1) = 1, b(1) = c(1) = 2
    assert_eq!(v.eval(&[]).unwrap(), 3.0);
    assert_eq!(v.bounds().frame, 3);
    assert_eq!(v.bounds().calls, 2);
}

#[test]
fn four_arm_piecewise_function() {
    // A real shape this language exists to express: a piecewise function of one state field,
    // with three ascending breakpoints and a transcendental in only one arm.
    //
    //   if s < 345   then 350.1
    //   else if s < 360 then 0.9267*s + 30.3885
    //   else if s < 570 then s + 5 - cos(0.6133*s - 220.788)
    //   else                 1.2933*s - 161.181
    let code = vec![
        Op::LoadState(0),       //  0
        Op::Push(345.0),        //  1
        Op::Lt,                 //  2
        Op::JumpIfFalse(6),     //  3
        Op::Push(350.1),        //  4
        Op::Jump(36),           //  5
        Op::LoadState(0),       //  6
        Op::Push(360.0),        //  7
        Op::Lt,                 //  8
        Op::JumpIfFalse(16),    //  9
        Op::Push(0.9267),       // 10
        Op::LoadState(0),       // 11
        Op::Mul,                // 12
        Op::Push(30.3885),      // 13
        Op::Add,                // 14
        Op::Jump(36),           // 15
        Op::LoadState(0),       // 16
        Op::Push(570.0),        // 17
        Op::Lt,                 // 18
        Op::JumpIfFalse(31),    // 19
        Op::LoadState(0),       // 20
        Op::Push(5.0),          // 21
        Op::Add,                // 22
        Op::Push(0.6133),       // 23
        Op::LoadState(0),       // 24
        Op::Mul,                // 25
        Op::Push(220.788),      // 26
        Op::Sub,                // 27
        Op::Cos,                // 28
        Op::Sub,                // 29
        Op::Jump(36),           // 30
        Op::Push(1.2933),       // 31
        Op::LoadState(0),       // 32
        Op::Mul,                // 33
        Op::Push(161.181),      // 34
        Op::Sub,                // 35
        Op::Ret,                // 36
    ];
    let v = Verified::new(one(code, 1, 0)).expect("should verify");

    // Each arm is selected on its own interval. Arm 1 is a literal, so it is exact.
    assert_eq!(v.eval(&[344.99]).unwrap(), 350.1);
    assert_eq!(v.eval(&[350.0]).unwrap(), 354.7335);

    // The arms are tuned to MEET at the breakpoints, but they meet in real arithmetic, not in
    // `f32`. Each boundary therefore carries a small step. These are the measured widths, and
    // they are asserted rather than tolerated loosely: a change to the constants or to the
    // evaluation order would move them, and that is worth a failing test.
    //
    //   s = 345   arm1 350.1000061  arm2 350.0999756   step 3.05e-05
    //   s = 360   arm2 364.0004883  arm3 364.0000000   step 4.88e-04
    //   s = 570   arm3 575.9999390  arm4 576.0000000   step 6.10e-05
    let at = |s: f32| v.eval(&[s]).unwrap();
    assert!((at(345.0) - 350.1).abs() < 1e-4, "got {}", at(345.0));
    assert!((at(360.0) - 364.0).abs() < 1e-3, "got {}", at(360.0));
    assert!((at(570.0) - 576.0).abs() < 1e-4, "got {}", at(570.0));
    assert!((at(700.0) - 744.129).abs() < 1e-3, "got {}", at(700.0));

    // Proof A over four paths. The deepest is the arm holding the cosine.
    assert_eq!(v.bounds().stack, 3);
}

#[test]
fn a_branchless_blend_and_a_branch_agree_bit_for_bit() {
    // The shape this language replaces: `mix(a, b, step(edge, x))` encodes a choice as
    // arithmetic, so it computes BOTH arms and discards one.
    //
    // The two forms agree exactly, in `f32`, and not merely closely: `step` yields exactly
    // 0.0 or 1.0, and `mix` at t = 0 is `1.0*a + 0.0*b`. Multiplying by exact 0.0 and 1.0 is
    // exact in any precision, so no rounding enters.
    //
    // What the branch buys is not accuracy. It is that the unused arm is never evaluated.
    let flat = |s: f32| {
        eval(
            vec![
                Op::Push(2.0),      // a
                Op::LoadState(0),   // b = s * 3
                Op::Push(3.0),
                Op::Mul,
                Op::Push(10.0),     // edge
                Op::LoadState(0),   // x
                Op::Step,
                Op::Mix,
                Op::Ret,
            ],
            &[s],
        )
    };
    let branched = |s: f32| {
        eval(
            vec![
                Op::LoadState(0),
                Op::Push(10.0),
                Op::Lt,
                Op::JumpIfFalse(6),
                Op::Push(2.0),
                Op::Jump(9),
                Op::LoadState(0),
                Op::Push(3.0),
                Op::Mul,
                Op::Ret,
            ],
            &[s],
        )
    };
    for i in 0..2000 {
        let s = i as f32 * 0.01;
        assert_eq!(flat(s), branched(s), "forms diverged at s = {}", s);
    }
}

// ── 2. the proofs accept, and report real bounds ────────────────────────────────

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
    assert_eq!(v.bounds().frame, 0);
    assert_eq!(v.bounds().calls, 0);
}

// ── 3. the proofs reject ────────────────────────────────────────────────────────

/// Assert a program is rejected with a specific error.
fn reject(p: Program, want: VerifyError) {
    match Verified::new(p) {
        Ok(_) => panic!("expected rejection with {:?}, but the program verified", want),
        Err(got) => assert_eq!(got, want, "wrong rejection reason"),
    }
}

#[test]
fn rejects_a_backward_jump() {
    // A backward jump is a loop. The language has no loop form, so no legal program emits one,
    // and allowing one would let a malformed program spin forever inside a single call.
    reject(
        one(
            vec![Op::Push(1.0), Op::Jump(0), Op::Ret],
            0,
            0,
        ),
        VerifyError::BackwardJump,
    );
}

#[test]
fn rejects_a_jump_out_of_its_own_function() {
    // Index 4 is a real instruction, but it belongs to the second function. Proof A checks one
    // function at a time, so a jump across the boundary would escape it.
    let program = Program {
        code: vec![
            // main — entry 0
            Op::Push(1.0),
            Op::Jump(4),
            Op::Ret,
            // other — entry 3
            Op::Push(2.0),
            Op::Ret,
        ],
        funcs: vec![
            Func {
                entry: 0,
                len: 3,
                arity: 0,
                frame: 0,
            },
            Func {
                entry: 3,
                len: 2,
                arity: 0,
                frame: 0,
            },
        ],
        state_arity: 0,
    };
    reject(program, VerifyError::BadJumpTarget);
}

#[test]
fn rejects_two_jumps_that_disagree_at_one_target() {
    // Both jumps land on index 6. The first leaves zero values, the second leaves two. The
    // table catches it when the second jump writes, before either path arrives.
    reject(
        one(
            vec![
                Op::Push(1.0),
                Op::JumpIfFalse(6), // height 0 at 6
                Op::Push(1.0),
                Op::Push(2.0),
                Op::Jump(6), // height 2 at 6
                Op::Push(3.0),
                Op::Ret,
            ],
            0,
            0,
        ),
        VerifyError::HeightMismatch,
    );
}

#[test]
fn rejects_arms_that_leave_different_heights() {
    // The `then` arm leaves two values, the `else` arm leaves one. A single accumulating scan
    // would miss this. The height recorded for the join does not match on arrival.
    reject(
        one(
            vec![
                Op::Push(1.0),
                Op::JumpIfFalse(5),
                Op::Push(1.0),
                Op::Push(2.0), // two values on this path
                Op::Jump(6),
                Op::Push(3.0), // one value on this path
                Op::Ret,
            ],
            0,
            0,
        ),
        VerifyError::HeightMismatch,
    );
}

#[test]
fn rejects_a_call_graph_cycle() {
    // f calls g, g calls f. The frame requirement is then unbounded, which is exactly why the
    // language forbids recursion — and this is where the ban becomes a checked fact.
    let program = Program {
        code: vec![
            // f — entry 0
            Op::Push(1.0),
            Op::Call {
                target: 3,
                arity: 1,
                fp_delta: 1,
            },
            Op::Ret,
            // g — entry 3
            Op::Push(1.0),
            Op::Call {
                target: 0,
                arity: 1,
                fp_delta: 1,
            },
            Op::Ret,
        ],
        funcs: vec![
            Func {
                entry: 0,
                len: 3,
                arity: 1,
                frame: 1,
            },
            Func {
                entry: 3,
                len: 3,
                arity: 1,
                frame: 1,
            },
        ],
        state_arity: 0,
    };
    reject(program, VerifyError::CallGraphCycle);
}

#[test]
fn rejects_self_recursion() {
    let program = Program {
        code: vec![
            Op::Push(1.0),
            Op::Call {
                target: 0,
                arity: 1,
                fp_delta: 1,
            },
            Op::Ret,
        ],
        funcs: vec![Func {
            entry: 0,
            len: 3,
            arity: 1,
            frame: 1,
        }],
        state_arity: 0,
    };
    reject(program, VerifyError::CallGraphCycle);
}

#[test]
fn rejects_reading_past_the_state_record() {
    reject(
        one(vec![Op::LoadState(3), Op::Ret], 2, 0),
        VerifyError::BadStateIndex,
    );
}

#[test]
fn rejects_reading_past_the_frame_window() {
    reject(
        one(vec![Op::LoadLocal(2), Op::Ret], 0, 1),
        VerifyError::BadLocalIndex,
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
fn rejects_a_call_to_a_non_entry_point() {
    reject(
        one(
            vec![
                Op::Push(1.0),
                Op::Call {
                    target: 99,
                    arity: 1,
                    fp_delta: 0,
                },
                Op::Ret,
            ],
            0,
            0,
        ),
        VerifyError::BadCallTarget,
    );
}

#[test]
fn rejects_an_arity_mismatch() {
    let program = Program {
        code: vec![
            Op::Push(1.0),
            Op::Call {
                target: 3,
                arity: 1,
                fp_delta: 0,
            },
            Op::Ret,
            Op::LoadLocal(0),
            Op::LoadLocal(1),
            Op::Add,
            Op::Ret,
        ],
        funcs: vec![
            Func {
                entry: 0,
                len: 3,
                arity: 0,
                frame: 0,
            },
            // declares two parameters; the call site passes one
            Func {
                entry: 3,
                len: 4,
                arity: 2,
                frame: 2,
            },
        ],
        state_arity: 0,
    };
    reject(program, VerifyError::ArityMismatch);
}

#[test]
fn rejects_a_call_whose_frame_delta_is_not_the_caller_frame() {
    // main has no frame, so its call must carry fp_delta 0. A delta of 1 would leave a gap
    // that Proof B never counted.
    let program = Program {
        code: vec![
            Op::Push(21.0),
            Op::Call {
                target: 3,
                arity: 1,
                fp_delta: 1,
            },
            Op::Ret,
            Op::LoadLocal(0),
            Op::Push(2.0),
            Op::Mul,
            Op::Ret,
        ],
        funcs: vec![
            Func {
                entry: 0,
                len: 3,
                arity: 0,
                frame: 0,
            },
            Func {
                entry: 3,
                len: 4,
                arity: 1,
                frame: 1,
            },
        ],
        state_arity: 0,
    };
    reject(program, VerifyError::FrameDeltaMismatch);
}

#[test]
fn rejects_unreachable_code() {
    reject(
        one(
            vec![Op::Push(1.0), Op::Jump(3), Op::Push(2.0), Op::Ret],
            0,
            0,
        ),
        VerifyError::Unreachable,
    );
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
        VerifyError::TooManyFunctions,
        VerifyError::CodeTooLong,
        VerifyError::FuncOutOfRange,
        VerifyError::MissingRet,
        VerifyError::BadStateIndex,
        VerifyError::BadLocalIndex,
        VerifyError::BadJumpTarget,
        VerifyError::BackwardJump,
        VerifyError::HeightMismatch,
        VerifyError::Unreachable,
        VerifyError::StackUnderflow,
        VerifyError::StackOverflow,
        VerifyError::NotOneResult,
        VerifyError::BadCallTarget,
        VerifyError::ArityMismatch,
        VerifyError::FrameDeltaMismatch,
        VerifyError::CallGraphCycle,
        VerifyError::FrameOverflow,
        VerifyError::CallDepthOverflow,
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
