#![feature(new_zeroed_alloc)]

pub mod arm7tdmi;
pub mod audio;
pub mod cartridge;
pub mod gba;
pub mod input;
pub mod memory;
pub mod script;
pub mod video;

#[cfg(test)]
mod tests;
