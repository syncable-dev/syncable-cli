//! Dockerfile parser module.
//!
//! Provides:
//! - `instruction` - Dockerfile AST types
//! - `dockerfile` - nom-based parser implementation

pub mod dockerfile;
pub mod instruction;

pub use dockerfile::{ParseError, parse_dockerfile};
pub use instruction::*;
