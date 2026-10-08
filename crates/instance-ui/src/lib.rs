//! The GameMaker-style egui look shared by the Instance IDE (`instance-ide`) and the
//! runner's own windows (`instance-runner`: Debug Information window, error dialog), so
//! both stay identical: visuals and fonts ([`theme`]), classic window chrome —
//! bevels, gradient captions, caption buttons, dialog frames ([`chrome`]) — and a few
//! original vector icons with flat toolbar buttons ([`icons`]).
//!
//! All artwork is drawn with the egui painter and authored for this project.

pub mod chrome;
pub mod fonts;
pub mod icons;
pub mod theme;

pub use egui;
