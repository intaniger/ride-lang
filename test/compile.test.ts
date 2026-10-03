import { describe, expect, test } from 'vitest';
import { compileSource, evaluate, type Bytecode } from '../src/index.js';

/** Compile and fail the test on any diagnostic. */
async function build(source: string): Promise<Bytecode> {
    const result = await compileSource(source);
    if (!result.ok) {
        throw new Error('expected a clean compile, got:\n  ' + result.errors.map((e) => e.message).join('\n  '));
    }
    return result.bytecode;
}

/** Compile and return the diagnostics, failing if it unexpectedly succeeded. */
async function errorsOf(source: string): Promise<string[]> {
    const result = await compileSource(source);
    if (result.ok) throw new Error('expected a rejection, but the program compiled');
    return result.errors.map((e) => e.message);
}

const run = (b: Bytecode, state: number[] = []) => evaluate(b, state);

// ── the shape of the language ───────────────────────────────────────────────────

describe('numbers in, one number out', () => {
    test('a program with no state is a constant function', async () => {
        const b = await build('let main = 2 + 3 * 4');
        expect(run(b)).toBe(14);
        expect(b.stateArity).toBe(0);
    });

    test('state fields are read by name and written in declaration order', async () => {
        const b = await build(`
            state { a, b, c }
            let main = c - a
        `);
        expect(b.stateFields).toEqual(['a', 'b', 'c']);
        expect(run(b, [10, 20, 35])).toBe(25);
        // Read by index, in the declared order.
        expect(b.code).toEqual([
            { op: 'LoadState', index: 2 },
            { op: 'LoadState', index: 0 },
            { op: 'Sub' },
            { op: 'Ret' },
        ]);
    });

    test('a state record that is too short is refused', async () => {
        const b = await build('state { x, y }\nlet main = x + y');
        expect(() => run(b, [1])).toThrow(/needs 2/);
    });

    test('the record length is checked before the first instruction, not at a read', async () => {
        // main never reads y, and the call is still refused: the length is a property of the call.
        const b = await build('state { x, y }\nlet main = 7');
        expect(() => run(b, [1])).toThrow(/needs 2/);
    });

    test('evaluation starts at main, wherever it is declared', async () => {
        const b = await build('let f = 1\nlet main = 2');
        expect(b.funcs[0].name).toBe('main');
        expect(run(b)).toBe(2);
    });

    test('every result is rounded to f32', async () => {
        // In f64, 0.1 + 0.2 is 0.30000000000000004. In f32 it is 0.30000001192092896.
        const f32 = Math.fround;
        const got = run(await build('let main = 0.1 + 0.2'));
        expect(got).toBe(f32(f32(0.1) + f32(0.2)));
        expect(got).not.toBe(0.1 + 0.2);
    });
});

describe('operators', () => {
    test('power is right-associative', async () => {
        // 2^(3^2) = 512, not (2^3)^2 = 64
        expect(run(await build('let main = 2^3^2'))).toBe(512);
    });

    test('multiplication binds tighter than addition', async () => {
        expect(run(await build('let main = 1 + 2 * 3'))).toBe(7);
    });

    test('unary minus', async () => {
        expect(run(await build('let main = -(3 - 5)'))).toBe(2);
    });

    test('operators on one level associate left', async () => {
        // (10 - 4) - 3 = 3, not 10 - (4 - 3) = 9
        expect(run(await build('let main = 10 - 4 - 3'))).toBe(3);
        // (16 / 4) / 2 = 2, not 16 / (4 / 2) = 8
        expect(run(await build('let main = 16 / 4 / 2'))).toBe(2);
    });

    test('the parser groups, and the compiler emits that shape unsimplified', async () => {
        // 2 + 3 * 4 reaches the compiler as 2 + (3 * 4). It is not folded to 14.
        const b = await build('let main = 2 + 3 * 4');
        expect(b.code).toEqual([
            { op: 'Push', value: 2 },
            { op: 'Push', value: 3 },
            { op: 'Push', value: 4 },
            { op: 'Mul' },
            { op: 'Add' },
            { op: 'Ret' },
        ]);
    });

    test('power binds tighter than multiplication', async () => {
        // 2 * (3^2) = 18, not (2 * 3)^2 = 36
        expect(run(await build('let main = 2 * 3^2'))).toBe(18);
    });

    test('unary minus binds tighter than power', async () => {
        // (-2)^2 = 4, not -(2^2) = -4
        expect(run(await build('let main = -2^2'))).toBe(4);
    });

    test('unary minus is one Neg, not a subtraction from zero', async () => {
        expect((await build('let main = -(3 - 5)')).code).toEqual([
            { op: 'Push', value: 3 },
            { op: 'Push', value: 5 },
            { op: 'Sub' },
            { op: 'Neg' },
            { op: 'Ret' },
        ]);
    });
});

describe('implicit multiplication', () => {
    // A number next to a term means multiplication.
    test('a number next to a parenthesised term', async () => {
        expect(run(await build('let main = 4(2 + 3)'))).toBe(20);
    });

    test('a number next to a builtin call', async () => {
        // 2*exp(0) = 2
        expect(run(await build('let main = 2exp(0)'))).toBe(2);
    });

    test('compiles to an ordinary multiply', async () => {
        const b = await build('let main = 3(4)');
        expect(b.code).toEqual([
            { op: 'Push', value: 3 },
            { op: 'Push', value: 4 },
            { op: 'Mul' },
            { op: 'Ret' },
        ]);
    });

    test('a number next to a name reads the way it was written down', async () => {
        const juxtaposed = await build('state { x }\nlet main = 0.6133x - 220.788');
        const explicit = await build('state { x }\nlet main = 0.6133 * x - 220.788');
        expect(juxtaposed.code).toEqual(explicit.code);
        expect(run(juxtaposed, [400])).toBe(run(explicit, [400]));
    });

    test('a juxtaposed pair is one operand of a power', async () => {
        // (2x)^2 = 36 at x = 3, not 2(x^2) = 18
        expect(run(await build('state { x }\nlet main = 2x^2'), [3])).toBe(36);
    });

    test('unary minus applies to the whole product', async () => {
        // -(2x), so the Neg comes after the Mul. Both groupings give -6, so read the code.
        expect((await build('state { x }\nlet main = -2x')).code).toEqual([
            { op: 'Push', value: 2 },
            { op: 'LoadState', index: 0 },
            { op: 'Mul' },
            { op: 'Neg' },
            { op: 'Ret' },
        ]);
    });
});

describe('builtins', () => {
    test('all twelve are callable with their declared arity', async () => {
        expect(run(await build('let main = abs(-2)'))).toBe(2);
        expect(run(await build('let main = sign(0)'))).toBe(0);
        expect(run(await build('let main = sqrt(9)'))).toBe(3);
        expect(run(await build('let main = max(2, 7)'))).toBe(7);
        expect(run(await build('let main = min(2, 7)'))).toBe(2);
        expect(run(await build('let main = step(10, 9)'))).toBe(0);
        expect(run(await build('let main = step(10, 10)'))).toBe(1);
        expect(run(await build('let main = mix(4, 8, 0.5)'))).toBe(6);
        expect(run(await build('let main = clamp(12, 0, 10)'))).toBe(10);
        expect(run(await build('let main = select(10, 5, 1, 2)'))).toBe(1);
        expect(run(await build('let main = cos(0)'))).toBe(1);
        expect(run(await build('let main = sin(0)'))).toBe(0);
    });

    test('a wrong argument count is refused', async () => {
        expect(await errorsOf('let main = mix(1, 2)')).toEqual([
            '`mix` takes 3 argument(s), got 2',
        ]);
    });

    test('each builtin is one instruction, and the last argument is on top', async () => {
        // Arguments are walked left to right, so mix(a, b, t) pushes a, b, t and then mixes.
        expect((await build('let main = mix(1, 2, 3)')).code).toEqual([
            { op: 'Push', value: 1 },
            { op: 'Push', value: 2 },
            { op: 'Push', value: 3 },
            { op: 'Mix' },
            { op: 'Ret' },
        ]);
    });

    test('the transcendentals are computed in f64 and rounded to f32', async () => {
        expect(run(await build('let main = sin(1)'))).toBe(Math.fround(Math.sin(1)));
        expect(run(await build('let main = exp(1)'))).toBe(Math.fround(Math.exp(1)));
    });
});

describe('diagnostics', () => {
    test('an unknown name', async () => {
        expect(await errorsOf('let main = nope')).toEqual(['unknown name `nope`']);
    });

    test('an unknown function', async () => {
        expect(await errorsOf('let main = nope(1)')).toEqual(['unknown function `nope`']);
    });

    test('an unknown name inside an expression is one message, not a cascade', async () => {
        // The operators around the rejected name add no message of their own.
        expect(await errorsOf('let main = (nope + 1) * 2')).toEqual(['unknown name `nope`']);
    });

    test('a missing entry point', async () => {
        expect(await errorsOf('let f = 1')).toEqual([
            'no `let main = …` to start from',
        ]);
    });

    test('a duplicate function', async () => {
        const errors = await errorsOf('let f = 1\nlet f = 2\nlet main = 3');
        expect(errors).toContain('duplicate function `f`');
    });

    test('a duplicate state field', async () => {
        const errors = await errorsOf('state { x, x }\nlet main = x');
        expect(errors).toContain('duplicate state field `x`');
    });

    test('a syntax error comes back as a diagnostic, not a throw', async () => {
        const errors = await errorsOf('let main = 2 +');
        expect(errors.length).toBeGreaterThan(0);
    });
});
