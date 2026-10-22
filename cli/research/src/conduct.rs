//! Coordinating a `conduct` run: which spawns each plan offers, what the run
//! remembers between plans, and what it has seen the set show.

pub mod concurrency;
pub mod ledger;
pub mod memory;
pub mod observed;
pub mod replaced_run;
pub mod search_budget;
pub mod spawn;
pub mod window;
