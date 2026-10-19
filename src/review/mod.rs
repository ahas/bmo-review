//! The code review (`docs/code-review.md`): a worker's checkout read at a commit, fenced to what its tree holds
//! and what the exclusions leave.

pub mod checkouts;
pub mod definitions;
pub mod git;
pub mod reading;
pub mod request;
pub mod secrets;
pub mod workspace;
