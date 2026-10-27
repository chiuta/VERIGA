//! VERIGA — motor clean-room de verificare a legăturilor.
//!
//! Scris de la zero, pe baza comportamentului documentat al verificatoarelor de
//! legături în general; nu conține cod sau resurse din alte produse.

pub mod check;
pub mod config;
pub mod engine;
pub mod extract;
pub mod fixer;
pub mod insights;
pub mod model;
pub mod netguard;
pub mod report;
pub mod robots;
pub mod rules;
pub mod srcscan;
pub mod urlutil;

pub use config::{Config, OrphanConfig};
pub use engine::{Control, recheck, run};
pub use model::{Event, Hop, LinkKind, LinkResult, Parent, Progress, Session, Status};
