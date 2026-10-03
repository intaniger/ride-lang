// A reference evaluator, in TypeScript.
//
// This is the twin of `runtime/src/eval.rs`. It exists so the front end can be tested without
// a Rust toolchain, and so the two implementations can be compared against each other.
//
// Every arithmetic result passes through `Math.fround`, because the runtime works in `f32` and
// JavaScript numbers are `f64`. Without that rounding the two would drift and the comparison
// would be worthless.
//
// ONE CAVEAT, stated rather than hidden: the transcendentals are not guaranteed bit-identical.
// `Math.cos` computes in `f64` and is rounded to `f32` here, while Rust's `f32::cos` computes in
// `f32` throughout. The two can differ in the last bit. Treat this evaluator as exact for the
// arithmetic, comparison, branch and call instructions, and as approximate for
// `sin cos sqrt exp pow`.

import type { Bytecode, Op } from './compile.js';

const f32 = Math.fround;

export class EvaluateError extends Error {}

/**
 * Run a compiled program against one STATE record.
 *
 * `state` is indexed by the order of the `state { … }` declaration, which `Bytecode.stateFields`
 * records by name.
 */
export function evaluate(bytecode: Bytecode, state: readonly number[]): number {
    if (state.length < bytecode.stateArity) {
        throw new EvaluateError(
            `state record has ${state.length} field(s), the program needs ${bytecode.stateArity}`,
        );
    }

    const code = bytecode.code;
    const stack: number[] = [];
    const frame: number[] = [];
    const calls: { pc: number; fp: number }[] = [];

    let fp = 0;
    let pc = bytecode.funcs[0].entry;
    // A step ceiling. The runtime proves termination from forward-only jumps plus an acyclic
    // call graph; this evaluator is a reference, so it guards instead of proving.
    let steps = 0;
    const LIMIT = 1_000_000;

    for (;;) {
        if (++steps > LIMIT) throw new EvaluateError('step limit exceeded');
        const instr: Op = code[pc];

        switch (instr.op) {
            case 'Push':
                stack.push(f32(instr.value));
                pc++;
                break;
            case 'LoadState':
                stack.push(f32(state[instr.index]));
                pc++;
                break;
            case 'LoadLocal':
                stack.push(frame[fp + instr.index]);
                pc++;
                break;
            case 'StoreLocal':
                frame[fp + instr.index] = stack.pop()!;
                pc++;
                break;

            case 'Add': bin(stack, (a, b) => a + b); pc++; break;
            case 'Sub': bin(stack, (a, b) => a - b); pc++; break;
            case 'Mul': bin(stack, (a, b) => a * b); pc++; break;
            case 'Div': bin(stack, (a, b) => a / b); pc++; break;
            case 'Pow': bin(stack, (a, b) => Math.pow(a, b)); pc++; break;
            case 'Neg': un(stack, (a) => -a); pc++; break;

            case 'Sin': un(stack, Math.sin); pc++; break;
            case 'Cos': un(stack, Math.cos); pc++; break;
            case 'Sqrt': un(stack, Math.sqrt); pc++; break;
            case 'Abs': un(stack, Math.abs); pc++; break;
            case 'Exp': un(stack, Math.exp); pc++; break;
            case 'Sign':
                // Matches the runtime: the sign of zero is zero, not one.
                un(stack, (a) => (a === 0 ? 0 : Math.sign(a)));
                pc++;
                break;

            case 'Max': bin(stack, (a, b) => Math.max(a, b)); pc++; break;
            case 'Min': bin(stack, (a, b) => Math.min(a, b)); pc++; break;
            case 'Step': {
                const x = stack.pop()!;
                const edge = stack.pop()!;
                stack.push(x < edge ? 0 : 1);
                pc++;
                break;
            }
            case 'Mix': {
                const t = stack.pop()!;
                const b = stack.pop()!;
                const a = stack.pop()!;
                stack.push(f32(f32((1 - t) * a) + f32(t * b)));
                pc++;
                break;
            }
            case 'Clamp': {
                const hi = stack.pop()!;
                const lo = stack.pop()!;
                const x = stack.pop()!;
                stack.push(f32(Math.min(Math.max(x, lo), hi)));
                pc++;
                break;
            }
            case 'Select': {
                const b = stack.pop()!;
                const a = stack.pop()!;
                const x = stack.pop()!;
                const edge = stack.pop()!;
                stack.push(x < edge ? a : b);
                pc++;
                break;
            }

            case 'Lt': cmp(stack, (a, b) => a < b); pc++; break;
            case 'Gt': cmp(stack, (a, b) => a > b); pc++; break;
            case 'Le': cmp(stack, (a, b) => a <= b); pc++; break;
            case 'Ge': cmp(stack, (a, b) => a >= b); pc++; break;
            case 'Eq': cmp(stack, (a, b) => a === b); pc++; break;

            case 'Jump':
                pc = instr.target;
                break;
            case 'JumpIfFalse':
                pc = stack.pop() === 0 ? instr.target : pc + 1;
                break;

            case 'Call': {
                const args = stack.splice(stack.length - instr.arity, instr.arity);
                calls.push({ pc: pc + 1, fp });
                fp += instr.fpDelta;
                args.forEach((v, i) => (frame[fp + i] = v));
                pc = instr.target;
                break;
            }
            case 'Ret': {
                const back = calls.pop();
                if (!back) return stack[0];
                pc = back.pc;
                fp = back.fp;
                break;
            }

            default: {
                const never: never = instr;
                throw new EvaluateError(`unknown instruction ${JSON.stringify(never)}`);
            }
        }
    }
}

function bin(stack: number[], f: (a: number, b: number) => number): void {
    const b = stack.pop()!;
    const a = stack.pop()!;
    stack.push(f32(f(a, b)));
}

function un(stack: number[], f: (a: number) => number): void {
    stack.push(f32(f(stack.pop()!)));
}

function cmp(stack: number[], f: (a: number, b: number) => boolean): void {
    const b = stack.pop()!;
    const a = stack.pop()!;
    stack.push(f(a, b) ? 1 : 0);
}
