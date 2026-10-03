// The public surface: text in, bytecode out.
//
// `compileSource` is the whole front end. It parses with Langium, then runs the five compiler
// stages in `compile.ts`. Nothing here touches the filesystem, so the same entry point serves
// a CLI, a test, and an editor with no disk.

import {
    createDefaultCoreModule,
    createDefaultSharedCoreModule,
    EmptyFileSystem,
    inject,
    URI,
    type LangiumCoreServices,
    type LangiumSharedCoreServices,
} from 'langium';
import { RideGeneratedModule, RideGeneratedSharedModule } from './generated/module.js';
import type { Program as AstProgram } from './generated/ast.js';
import { compile, type CompileResult, type Diagnostic } from './compile.js';

export * from './compile.js';
export { evaluate } from './evaluate.js';

/** Build the language services. One set is enough for a whole process. */
export function createRideServices(): {
    shared: LangiumSharedCoreServices;
    Ride: LangiumCoreServices;
} {
    const shared = inject(
        createDefaultSharedCoreModule(EmptyFileSystem),
        RideGeneratedSharedModule,
    );
    const Ride = inject(createDefaultCoreModule({ shared }), RideGeneratedModule);
    shared.ServiceRegistry.register(Ride);
    return { shared, Ride };
}

let cached: ReturnType<typeof createRideServices> | undefined;

function services(): ReturnType<typeof createRideServices> {
    if (!cached) cached = createRideServices();
    return cached;
}

/** Parse one source string. Syntax errors come back as diagnostics, never as a throw. */
export async function parse(
    text: string,
    uri = 'file:///input.ride',
): Promise<{ ok: true; ast: AstProgram } | { ok: false; errors: Diagnostic[] }> {
    const { shared } = services();
    const doc = shared.workspace.LangiumDocumentFactory.fromString<AstProgram>(
        text,
        URI.parse(uri),
    );
    await shared.workspace.DocumentBuilder.build([doc]);

    const errors: Diagnostic[] = [
        ...doc.parseResult.lexerErrors.map((e) => ({
            message: e.message,
            at: `line ${e.line ?? '?'}`,
        })),
        ...doc.parseResult.parserErrors.map((e) => ({
            message: e.message,
            at: e.token?.image,
        })),
    ];
    if (errors.length > 0) return { ok: false, errors };
    return { ok: true, ast: doc.parseResult.value };
}

/** Parse, then compile. The one call a host needs. */
export async function compileSource(text: string, uri?: string): Promise<CompileResult> {
    const parsed = await parse(text, uri);
    if (!parsed.ok) return { ok: false, errors: parsed.errors };
    return compile(parsed.ast);
}
