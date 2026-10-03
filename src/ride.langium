// ─────────────────────────────────────────────────────────────────────────────
// ride — a small pure functional language over numbers.
//
// A program declares a STATE record and a set of functions. Evaluating the entry
// function against one STATE record yields one number. There is nothing else: no
// strings, no collections, no effects, no state carried between calls.
// ─────────────────────────────────────────────────────────────────────────────

grammar Ride

entry Program:
    (declarations+=Declaration)*;

Declaration:
    StateDecl | ImportDecl | FunDecl;

// The record the host supplies, declared once. Field order is the wire order: the host
// writes the fields in this order, and `LoadState(i)` reads the i-th.
StateDecl:
    'state' '{' fields+=StateField (',' fields+=StateField)* '}';

StateField:
    name=ID;

// Take named functions from another file. There is no `export`: every top-level
// function is importable.
ImportDecl:
    'import' '{' names+=ID (',' names+=ID)* '}' 'from' path=STRING;

// A named function. Zero parameters is the normal shape for an entry point.
//
// Parameters are written bare and juxtaposed, which does NOT make the language curried:
// this is declaration syntax, and a call site always uses parentheses.
FunDecl:
    'let' name=ID (params+=Param)* '=' body=Expression;

Param:
    name=ID;

Expression:
    LetIn;

// A local binding. Scoped to `body`, so it is never evaluated on a path that does not
// reach it — which is why the binding sits inside the branch arm that uses it.
LetIn infers Expression:
    {infer LetIn} 'let' name=ID '=' value=Expression 'in' body=Expression
    | IfElse;

// `else` is mandatory. A branch with no `else` could produce no value.
IfElse infers Expression:
    {infer IfElse} 'if' condition=Expression 'then' whenTrue=Expression 'else' whenFalse=Expression
    | Comparison;

// Non-associative: `a < b < c` is rejected rather than silently grouped.
Comparison infers Expression:
    Additive ({infer Binary.left=current} operator=('<=' | '>=' | '==' | '<' | '>') right=Additive)?;


Additive infers Expression:
    Multiplicative ({infer Binary.left=current} operator=('+' | '-') right=Multiplicative)*;

Multiplicative infers Expression:
    Power ({infer Binary.left=current} operator=('*' | '/') right=Power)*;

// Right-associative, so `2^3^2` is `2^(3^2)`.
Power infers Expression:
    Unary ({infer Binary.left=current} operator='^' right=Power)?;

Unary infers Expression:
    {infer Negate} '-' operand=Unary
    | Juxtaposition;

// Implicit multiplication: `4(2 + 3)` and `0.6133x` mean multiplication.
Juxtaposition infers Expression:
    Primary ({infer ImplicitMul.left=current} right=Primary)*;

Primary infers Expression:
    {infer Call} callee=ID '(' (args+=Expression (',' args+=Expression)*)? ')'
    | {infer NumberLiteral} value=NUMBER
    | {infer Ref} name=ID
    | '(' Expression ')';

hidden terminal WS: /\s+/;
hidden terminal ML_COMMENT: /\/\*[\s\S]*?\*\//;
hidden terminal SL_COMMENT: /\/\/[^\n\r]*/;

terminal NUMBER returns number: /[0-9]+(\.[0-9]+)?/;
terminal ID: /[_a-zA-Z][\w_]*/;
terminal STRING: /"[^"]*"|'[^']*'/;
