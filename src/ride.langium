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
    StateDecl | FunDecl;

// The record the host supplies, declared once. Field order is the wire order: the host
// writes the fields in this order, and `LoadState(i)` reads the i-th.
StateDecl:
    'state' '{' fields+=StateField (',' fields+=StateField)* '}';

StateField:
    name=ID;

// A named function. `main` is the entry point.
FunDecl:
    'let' name=ID '=' body=Expression;

Expression:
    Additive;

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
