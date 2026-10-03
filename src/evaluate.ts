// A reference evaluator, in TypeScript.
//
// It exists so the front end can be tested without anything but Node: compile a program, run
// the bytecode, compare the number.
//
// Every arithmetic result passes through `Math.fround`, because the runtime works in `f32` and
// JavaScript numbers are `f64`. Without that rounding the answers would drift from the ones the
// runtime gives, and the comparison would be worthless.

import type { Bytecode, Op } from './compile.js';

const f32 = Math.fround;

export class EvaluateError extends Error {}

/** Run a compiled program and return the one number it computes. */
export function evaluate(bytecode: Bytecode): number {
    const code = bytecode.code;
    const stack: number[] = [];

    let pc = bytecode.funcs[0].entry;

    for (;;) {
        const instr: Op = code[pc];

        switch (instr.op) {
            case 'Push':
                stack.push(f32(instr.value));
                pc++;
                break;

            case 'Add': bin(stack, (a, b) => a + b); pc++; break;
            case 'Sub': bin(stack, (a, b) => a - b); pc++; break;
            case 'Mul': bin(stack, (a, b) => a * b); pc++; break;
            case 'Div': bin(stack, (a, b) => a / b); pc++; break;

            case 'Ret':
                return stack[0];

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
