//! okf-pack — an OKF-compatible knowledge-context format and bidirectional
//! adapter, packaged as a hot-swappable Spaces pack.
//!
//! The internal form (`okf-ext/0.1`) is structurally an OKF bundle with a
//! richer, closed vocabulary and typed links; canonical pure-OKF is emitted
//! only at the export boundary. See the design plan
//! `~/.claude/plans/humming-fluttering-quilt.md`.

pub mod canonical;
pub mod cli;
pub mod embed;
pub mod graph;
pub mod models;
pub mod okf;
pub mod parse;
pub mod privilege;
pub mod rpc;
pub mod scaffold;
pub mod store;
pub mod surface;
pub mod validate;
