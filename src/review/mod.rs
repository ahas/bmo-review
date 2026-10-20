//! The code review (`docs/code-review.md`): a project's checkout read at a revision, fenced to what its tree
//! holds and what the exclusions leave; the reviews of a revision, their comments and the items filed from them.

pub mod definitions;
pub mod filing;
pub mod git;
pub mod projects;
pub mod reading;
pub mod records;
pub mod reviews;
pub mod secrets;
pub mod workspace;
