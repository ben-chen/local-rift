#![deny(unused_must_use)]
pub mod activated;
pub mod api;
pub mod battlefields;
pub mod cards;
pub mod deck;
pub mod game;
pub mod gear;
pub mod hidden;
pub mod legality;
pub mod legend_catalog;
pub mod legends;
pub mod payment;
pub mod restrictions;
pub mod rules;
pub mod xp;

pub mod trigger_rules;

pub mod payment_plan;

mod resources;

pub use game::play;
pub use game::show_off;

pub mod continuous;
pub use game::empower;

pub mod numeric;
pub mod turn_keywords;
pub use numeric::{DamageAmount, Might, SignedMight, UnitStats};

pub use game::temporary;

pub use game::vision;

pub mod runes;

pub mod match_ai;
pub mod match_series;
