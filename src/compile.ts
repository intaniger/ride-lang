// The compiler: a `ride` source file becomes bytecode for a stack machine.
//
// Every error in this language is a compile-time error, so the compiler returns diagnostics
// and never throws on bad input. Five stages:
//
//   1. collect    gather the state record, the functions, and their parameters
//   2. check      names, arity, and the first-order rule
//   3. acyclic    reject recursion, so the frame bound is provable
//   4. emit       walk each function body to postfix, with jump placeholders
//   5. link       lay the functions out, then patch call targets and jump offsets
//
// Stage 4 does NOT simplify the arithmetic. The shape the author wrote is the shape that runs.
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
    isLetIn,
    isNegate,
    isNumberLiteral,
    isRef,
    isStateDecl,
} from './generated/ast.js';

// ── the instruction set ─────────────────────────────────────────────────────────

export type Op =
    | { op: 'Push'; value: number }
    | { op: 'LoadState'; index: number }
    | { op: 'LoadLocal'; index: number }
    | { op: 'StoreLocal'; index: number }
    | { op: 'Call'; target: number; arity: number; fpDelta: number }
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
    arity: number;
    frame: number;
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

    const entry = byName.get(ENTRY_NAME);
    if (!entry) {
        errors.push({ message: `no \`let ${ENTRY_NAME} = …\` to start from` });
    } else if (entry.params.length > 0) {
        errors.push({
            message: `\`${ENTRY_NAME}\` takes its input from \`state\`, so it must have no parameters`,
            at: ENTRY_NAME,
        });
    }

    // ── 3. acyclic (needs only names, so it runs before emission) ───────────
    // Reported here rather than left to the runtime verifier: the compiler knows the names,
    // so it can say which cycle, and a name is more use than an opcode index.
    for (const f of funcs) {
        const cycle = findCycle(f.name, byName);
        if (cycle) {
            errors.push({
                message:
                    `\`${f.name}\` is recursive (${cycle.join(' → ')}). ` +
                    'ride forbids recursion: an acyclic call graph is what makes the ' +
                    'frame bound provable at compile time.',
                at: f.name,
            });
            break; // one report is enough; the whole cycle is named
        }
    }

    if (errors.length > 0) return { ok: false, errors };

    // ── 2 + 4. check and emit, one walk per function ────────────────────────
    const emitted = new Map<string, { code: Op[]; frame: number; arity: number }>();
    for (const f of funcs) {
        const out = emitFunction(f, stateFields, byName, errors);
        emitted.set(f.name, out);
    }
    if (errors.length > 0) return { ok: false, errors };

    // ── 5. link ─────────────────────────────────────────────────────────────
    // The entry function goes first, because evaluation starts at `funcs[0]`.
    const order = [ENTRY_NAME, ...funcs.map((f) => f.name).filter((n) => n !== ENTRY_NAME)];

    const layout: Func[] = [];
    let cursor = 0;
    for (const name of order) {
        const e = emitted.get(name)!;
        layout.push({ name, entry: cursor, len: e.code.length, arity: e.arity, frame: e.frame });
        cursor += e.code.length;
    }
    const entryOf = new Map(layout.map((f) => [f.name, f.entry]));

    const code: Op[] = [];
    for (const f of layout) {
        const e = emitted.get(f.name)!;
        for (const instr of e.code) {
            if (instr.op === 'Jump' || instr.op === 'JumpIfFalse') {
                // Jump targets are emitted relative to the function, then rebased.
                code.push({ ...instr, target: instr.target + f.entry });
            } else if (instr.op === 'Call') {
                // `target` holds a placeholder index into `order`; swap it for the entry.
                const calleeName = order[instr.target];
                code.push({
                    op: 'Call',
                    target: entryOf.get(calleeName)!,
                    arity: instr.arity,
                    fpDelta: instr.fpDelta,
                });
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

/** Compile one function body. Jump targets are function-relative; `link` rebases them. */
function emitFunction(
    f: FunDecl,
    stateFields: string[],
    byName: Map<string, FunDecl>,
    errors: Diagnostic[],
): { code: Op[]; frame: number; arity: number } {
    const code: Op[] = [];

    // Frame layout: parameters first, then one slot per `let` in source order.
    const slots = new Map<string, number>();
    f.params.forEach((p, i) => slots.set(p.name, i));
    let frame = f.params.length;

    duplicates(f.params.map((p) => p.name)).forEach((n) =>
        errors.push({ message: `duplicate parameter \`${n}\` in \`${f.name}\``, at: f.name }),
    );

    /** Order is the declaration order of `order` in `compile`; resolved in `link`. */
    const callPlaceholder = (name: string): number => {
        const names = [ENTRY_NAME, ...[...byName.keys()].filter((n) => n !== ENTRY_NAME)];
        return names.indexOf(name);
    };

    const walk = (e: Expression): void => {
        if (isNumberLiteral(e)) {
            code.push({ op: 'Push', value: e.value });
            return;
        }

        if (isRef(e)) {
            // Lexical scope: a local shadows a state field.
            const slot = slots.get(e.name);
            if (slot !== undefined) {
                code.push({ op: 'LoadLocal', index: slot });
                return;
            }
            const field = stateFields.indexOf(e.name);
            if (field >= 0) {
                code.push({ op: 'LoadState', index: field });
                return;
            }
            if (byName.has(e.name)) {
                // The first-order rule. A function has no value, so it cannot be mentioned
                // except in a call.
                errors.push({
                    message:
                        `\`${e.name}\` is a function, so it cannot be used as a value. ` +
                        'ride is first-order: write `' + e.name + '(…)` to call it.',
                    at: e.name,
                });
                code.push({ op: 'Push', value: 0 }); // keep the stack shape for later checks
                return;
            }
            errors.push({ message: `unknown name \`${e.name}\``, at: e.name });
            code.push({ op: 'Push', value: 0 });
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

        if (isLetIn(e)) {
            // The binding is evaluated once, here, and stored. Reads become `LoadLocal`.
            walk(e.value);
            let slot = slots.get(e.name);
            if (slot === undefined) {
                slot = frame++;
            }
            slots.set(e.name, slot);
            code.push({ op: 'StoreLocal', index: slot });
            walk(e.body);
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

            const callee = byName.get(e.callee);
            if (!callee) {
                // A parameter or local with this name is a value, not a function.
                if (slots.has(e.callee) || stateFields.includes(e.callee)) {
                    errors.push({
                        message:
                            `\`${e.callee}\` is a number, not a function. ` +
                            'ride is first-order, so only a declared function can be called.',
                        at: e.callee,
                    });
                } else {
                    errors.push({ message: `unknown function \`${e.callee}\``, at: e.callee });
                }
                code.push({ op: 'Push', value: 0 });
                return;
            }
            if (e.args.length !== callee.params.length) {
                errors.push({
                    message: `\`${e.callee}\` takes ${callee.params.length} argument(s), got ${e.args.length}`,
                    at: e.callee,
                });
            }
            e.args.forEach(walk);
            code.push({
                op: 'Call',
                target: callPlaceholder(e.callee),
                arity: callee.params.length,
                fpDelta: 0, // the caller's own frame size; filled in below, once it is final
            });
            return;
        }

        errors.push({ message: 'unsupported expression' });
    };

    walk(f.body);
    code.push({ op: 'Ret' });

    // `fpDelta` is the caller's frame size, which is only final once the whole body is walked
    // (a `let` deep in a branch can still widen the window).
    for (const instr of code) {
        if (instr.op === 'Call') instr.fpDelta = frame;
    }

    return { code, frame, arity: f.params.length };
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

/** Depth-first search for a cycle reachable from `start`, by function name. */
function findCycle(start: string, byName: Map<string, FunDecl>): string[] | undefined {
    const path: string[] = [];
    const onPath = new Set<string>();
    const done = new Set<string>();

    const visit = (name: string): string[] | undefined => {
        if (onPath.has(name)) return [...path.slice(path.indexOf(name)), name];
        if (done.has(name)) return undefined;
        const decl = byName.get(name);
        if (!decl) return undefined;

        onPath.add(name);
        path.push(name);
        for (const callee of calleeNames(decl.body, byName)) {
            const found = visit(callee);
            if (found) return found;
        }
        path.pop();
        onPath.delete(name);
        done.add(name);
        return undefined;
    };
    return visit(start);
}

/** Every user function this expression calls, directly. */
function calleeNames(e: Expression, byName: Map<string, FunDecl>): string[] {
    const out: string[] = [];
    const walk = (n: Expression): void => {
        if (isCall(n)) {
            if (byName.has(n.callee)) out.push(n.callee);
            n.args.forEach(walk);
        } else if (isBinary(n) || isImplicitMul(n)) {
            walk(n.left);
            walk(n.right);
        } else if (isNegate(n)) {
            walk(n.operand);
        } else if (isLetIn(n)) {
            walk(n.value);
            walk(n.body);
        } else if (isIfElse(n)) {
            walk(n.condition);
            walk(n.whenTrue);
            walk(n.whenFalse);
        }
    };
    walk(e);
    return out;
}
