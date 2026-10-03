//! `ride` — the runtime.
//!
//! A `ride` program is a pure function from a STATE record of numbers to one number. This crate
//! verifies compiled bytecode and evaluates it.
//!
//! ```
//! use ride_runtime::{Func, Op, Program, Verified};
//!
//! // let main = if s < 10.0 then 1.0 else 2.0        (s is state field 0)
//! let program = Program {
//!     code: vec![
//!         Op::LoadState(0),
//!         Op::Push(10.0),
//!         Op::Lt,
//!         Op::JumpIfFalse(6),
//!         Op::Push(1.0),
//!         Op::Jump(7),
//!         Op::Push(2.0),
//!         Op::Ret,
//!     ],
//!     funcs: vec![Func { entry: 0, len: 8, arity: 0, frame: 0 }],
//!     state_arity: 1,
//! };
//!
//! let v = Verified::new(program).expect("the proof passes");
//! assert_eq!(v.eval(&[3.0]).unwrap(), 1.0);
//! assert_eq!(v.eval(&[30.0]).unwrap(), 2.0);
//! ```
//!
//! ## Structure
//!
//! * [`op`] — the instruction set and the shape of a compiled program.
//! * [`verify`] — the bound proof. Nothing is evaluated until it passes.
//! * [`eval`] — the dispatch loop, on one fixed array.
//!
//! ## What this crate does not do
//!
//! It does not parse. The surface language is compiled by the Langium front end in `src/`, which
//! emits the bytecode this crate consumes. Keeping the parser out of the runtime is what lets
//! the runtime ship with no dependencies.

#![forbid(unsafe_code)]

pub mod eval;
pub mod op;
pub mod verify;

pub use eval::{EvalError, Verified};
pub use op::{Func, Op, Program};
pub use verify::{verify, Bounds, VerifyError};
