//! The code review (`docs/code-review.md`): a worker's checkout read at a commit, fenced to what its tree holds
//! and what the exclusions leave; the reviews pinned to it, the talk under each and the items filed from them.

pub mod checkouts;
pub mod definitions;
pub mod filing;
pub mod git;
pub mod names;
pub mod reading;
pub mod records;
pub mod request;
pub mod reviews;
pub mod secrets;
pub mod talking;
pub mod words;
pub mod workspace;
