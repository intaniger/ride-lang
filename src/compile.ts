// The compiler: a `ride` source file becomes bytecode for a stack machine.
//
// Every error in this language is a compile-time error, so the compiler returns diagnostics
// and never throws on bad input. Three stages so far:
//
//   1. collect    gather the functions and find the entry point
//   2. emit       walk each function body to postfix
//   3. link       lay the functions out, entry point first
//
// Stage 2 does NOT simplify the arithmetic. The shape the author wrote is the shape that runs.
// That is deliberate: an algebraic simplifier reassociates, `f32` addition is not associative,
// and a reader comparing source to behaviour would have to account for the difference.

import type {
    Expression,
    FunDecl,
    Program as AstProgram,
    StateDecl,
} from './generated/ast.js';
import {
    isBinary,
    isFunDecl,
    isImplicitMul,
    isNegate,
    isNumberLiteral,
    isRef,
    isStateDecl,
} from './generated/ast.js';

// ── the instruction set ─────────────────────────────────────────────────────────

export type Op =
    | { op: 'Push'; value: number }
    | { op: 'LoadState'; index: number }
    | { op: Nullary };

/** Every operation that carries no immediate. */
type Nullary =
    | 'Add' | 'Sub' | 'Mul' | 'Div' | 'Pow' | 'Neg'
    | 'Ret';

export interface Func {
    name: string;
    entry: number;
    len: number;
}

export interface Bytecode {
    code: Op[];
    funcs: Func[];
    stateArity: number;
    /** Field names in wire order, so a host knows what to write and in which slot. */
    stateFields: string[];
}

export interface Diagnostic {
    message: string;
    /** The source construct the message is about, when one is identifiable. */
    at?: string;
}

export type CompileResult =
    | { ok: true; bytecode: Bytecode }
    | { ok: false; errors: Diagnostic[] };

const ARITHMETIC: Record<string, Nullary> = {
    '+': 'Add',
    '-': 'Sub',
    '*': 'Mul',
    '/': 'Div',
    '^': 'Pow',
};

/** The function evaluation starts from. */
export const ENTRY_NAME = 'main';

// ── the compiler ────────────────────────────────────────────────────────────────

export function compile(ast: AstProgram): CompileResult {
    const errors: Diagnostic[] = [];

    // ── 1. collect ──────────────────────────────────────────────────────────
    const stateDecls = ast.declarations.filter(isStateDecl) as StateDecl[];
    if (stateDecls.length > 1) {
        errors.push({ message: 'more than one `state` declaration' });
    }
    const stateFields = stateDecls[0]?.fields.map((f) => f.name) ?? [];
    duplicates(stateFields).forEach((n) =>
        errors.push({ message: `duplicate state field \`${n}\``, at: n }),
    );

    const funcs = ast.declarations.filter(isFunDecl) as FunDecl[];
    duplicates(funcs.map((f) => f.name)).forEach((n) =>
        errors.push({ message: `duplicate function \`${n}\``, at: n }),
    );

    const byName = new Map<string, FunDecl>();
    for (const f of funcs) if (!byName.has(f.name)) byName.set(f.name, f);

    if (!byName.has(ENTRY_NAME)) {
        errors.push({ message: `no \`let ${ENTRY_NAME} = …\` to start from` });
    }

    if (errors.length > 0) return { ok: false, errors };

    // ── 2. emit, one walk per function ──────────────────────────────────────
    const emitted = new Map<string, { code: Op[] }>();
    for (const f of funcs) {
        const out = emitFunction(f, stateFields, errors);
        emitted.set(f.name, out);
    }
    if (errors.length > 0) return { ok: false, errors };

    // ── 3. link ─────────────────────────────────────────────────────────────
    // The entry function goes first, because evaluation starts at `funcs[0]`.
    const order = [ENTRY_NAME, ...funcs.map((f) => f.name).filter((n) => n !== ENTRY_NAME)];

    const layout: Func[] = [];
    let cursor = 0;
    for (const name of order) {
        const e = emitted.get(name)!;
        layout.push({ name, entry: cursor, len: e.code.length });
        cursor += e.code.length;
    }

    const code: Op[] = [];
    for (const f of layout) code.push(...emitted.get(f.name)!.code);

    return {
        ok: true,
        bytecode: {
            code,
            funcs: layout,
            stateArity: stateFields.length,
            stateFields,
        },
    };
}

/** Compile one function body. */
function emitFunction(
    f: FunDecl,
    stateFields: string[],
    errors: Diagnostic[],
): { code: Op[] } {
    const code: Op[] = [];

    const walk = (e: Expression): void => {
        if (isNumberLiteral(e)) {
            code.push({ op: 'Push', value: e.value });
            return;
        }

        if (isRef(e)) {
            const field = stateFields.indexOf(e.name);
            if (field >= 0) {
                code.push({ op: 'LoadState', index: field });
                return;
            }
            errors.push({ message: `unknown name \`${e.name}\``, at: e.name });
            code.push({ op: 'Push', value: 0 }); // keep the stack shape for later checks
            return;
        }

        if (isNegate(e)) {
            walk(e.operand);
            code.push({ op: 'Neg' });
            return;
        }

        if (isImplicitMul(e)) {
            walk(e.left);
            walk(e.right);
            code.push({ op: 'Mul' });
            return;
        }

        if (isBinary(e)) {
            walk(e.left);
            walk(e.right);
            const op = ARITHMETIC[e.operator];
            if (!op) {
                errors.push({ message: `unknown operator \`${e.operator}\`` });
                return;
            }
            code.push({ op });
            return;
        }

        errors.push({ message: 'unsupported expression' });
    };

    walk(f.body);
    code.push({ op: 'Ret' });

    return { code };
}

function duplicates(names: string[]): string[] {
    const seen = new Set<string>();
    const dupes = new Set<string>();
    for (const n of names) {
        if (seen.has(n)) dupes.add(n);
        seen.add(n);
    }
    return [...dupes];
}
