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
    isCall,
    isFunDecl,
    isIfElse,
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
    | { op: 'Jump'; target: number }
    | { op: 'JumpIfFalse'; target: number }
    | { op: Nullary };

/** Every operation that carries no immediate. */
type Nullary =
    | 'Add' | 'Sub' | 'Mul' | 'Div' | 'Pow' | 'Neg'
    | 'Sin' | 'Cos' | 'Sqrt' | 'Abs' | 'Sign' | 'Exp'
    | 'Max' | 'Min' | 'Step' | 'Mix' | 'Clamp' | 'Select'
    | 'Lt' | 'Gt' | 'Le' | 'Ge' | 'Eq'
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

// ── builtins ────────────────────────────────────────────────────────────────────

/** The twelve builtins, with their arity. Argument order matches the runtime's stack order. */
const BUILTINS: Record<string, { arity: number; op: Nullary }> = {
    sin: { arity: 1, op: 'Sin' },
    cos: { arity: 1, op: 'Cos' },
    sqrt: { arity: 1, op: 'Sqrt' },
    abs: { arity: 1, op: 'Abs' },
    sign: { arity: 1, op: 'Sign' },
    exp: { arity: 1, op: 'Exp' },
    max: { arity: 2, op: 'Max' },
    min: { arity: 2, op: 'Min' },
    step: { arity: 2, op: 'Step' },
    mix: { arity: 3, op: 'Mix' },
    clamp: { arity: 3, op: 'Clamp' },
    select: { arity: 4, op: 'Select' },
};

const COMPARISONS: Record<string, Nullary> = {
    '<': 'Lt',
    '>': 'Gt',
    '<=': 'Le',
    '>=': 'Ge',
    '==': 'Eq',
};

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
    for (const f of layout) {
        const e = emitted.get(f.name)!;
        for (const instr of e.code) {
            if (instr.op === 'Jump' || instr.op === 'JumpIfFalse') {
                // Jump targets are emitted relative to the function, then rebased.
                code.push({ ...instr, target: instr.target + f.entry });
            } else {
                code.push(instr);
            }
        }
    }

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
            const op = ARITHMETIC[e.operator] ?? COMPARISONS[e.operator];
            if (!op) {
                errors.push({ message: `unknown operator \`${e.operator}\`` });
                return;
            }
            code.push({ op });
            return;
        }

        if (isIfElse(e)) {
            //   <condition>
            //   JumpIfFalse  →  else
            //   <whenTrue>
            //   Jump         →  join
            //   <whenFalse>
            //   join:
            walk(e.condition);
            const toElse = code.length;
            code.push({ op: 'JumpIfFalse', target: -1 });
            walk(e.whenTrue);
            const toJoin = code.length;
            code.push({ op: 'Jump', target: -1 });
            patch(code, toElse, code.length);
            walk(e.whenFalse);
            patch(code, toJoin, code.length);
            return;
        }

        if (isCall(e)) {
            const builtin = BUILTINS[e.callee];
            if (builtin) {
                if (e.args.length !== builtin.arity) {
                    errors.push({
                        message: `\`${e.callee}\` takes ${builtin.arity} argument(s), got ${e.args.length}`,
                        at: e.callee,
                    });
                }
                e.args.forEach(walk);
                code.push({ op: builtin.op });
                return;
            }

            errors.push({ message: `unknown function \`${e.callee}\``, at: e.callee });
            code.push({ op: 'Push', value: 0 });
            return;
        }

        errors.push({ message: 'unsupported expression' });
    };

    walk(f.body);
    code.push({ op: 'Ret' });

    return { code };
}

function patch(code: Op[], at: number, target: number): void {
    const instr = code[at];
    if (instr.op === 'Jump' || instr.op === 'JumpIfFalse') instr.target = target;
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
