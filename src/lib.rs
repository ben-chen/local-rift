#![deny(unused_must_use)]
pub mod activated;
pub mod api;
pub mod battlefields;
pub mod cards;
pub mod deck;
pub mod game;
pub mod gear;
pub mod hidden;
pub mod legend_catalog;
pub mod legends;
pub mod payment;
pub mod rules;
pub mod xp;

pub mod trigger_rules;

pub mod payment_plan;

mod resources;

pub use game::play;

pub mod continuous;
pub use game::empower;

pub mod numeric;
pub use numeric::{DamageAmount, Might, SignedMight, UnitStats};

pub use game::temporary;

pub use game::vision;
