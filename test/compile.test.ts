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

const run = (b: Bytecode) => evaluate(b);

// ── the shape of the language ───────────────────────────────────────────────────

describe('numbers in, one number out', () => {
    test('a program with no state is a constant function', async () => {
        const b = await build('let main = 2 + 3 * 4');
        expect(run(b)).toBe(14);
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
    test('multiplication binds tighter than addition', async () => {
        expect(run(await build('let main = 1 + 2 * 3'))).toBe(7);
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
});

describe('diagnostics', () => {
    test('a missing entry point', async () => {
        expect(await errorsOf('let f = 1')).toEqual([
            'no `let main = …` to start from',
        ]);
    });

    test('a duplicate function', async () => {
        const errors = await errorsOf('let f = 1\nlet f = 2\nlet main = 3');
        expect(errors).toContain('duplicate function `f`');
    });

    test('a syntax error comes back as a diagnostic, not a throw', async () => {
        const errors = await errorsOf('let main = 2 +');
        expect(errors.length).toBeGreaterThan(0);
    });
});
