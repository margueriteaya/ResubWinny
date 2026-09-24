//! Broadcast caption semantics shared by the Worker and the desktop service.
//!
//! B24 and B62 decide speaker, sound, music and continuation semantics through
//! this one model; each format layer only supplies its own source evidence.

pub mod arib_symbols;
pub mod caption_features;
