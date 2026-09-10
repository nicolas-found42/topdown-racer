//! Headless core of the game: the Track parser and the fixed-step
//! simulation. No Bevy types, no rendering — everything here compiles and
//! tests without the engine, per the two agreed seams (spec stories 30-32).

pub mod ai;
pub mod simulation;
pub mod track;

pub mod practice;
