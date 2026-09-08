//! Pure domain contracts for `reitti`.
//!
//! This crate deliberately has no clap, HTTP client, or filesystem dependency.
//! Provider adapters and side effects belong in `reitti-cli`.

mod model;
mod provider;
mod reference;
mod request_id;
mod time;

pub use model::*;
pub use provider::*;
pub use reference::*;
pub use request_id::*;
pub use time::*;
