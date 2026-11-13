//! The styles, one module each.

mod amiga;
mod amiga60;
mod at_60_hz;
mod dos;
mod moon;
mod xbox;

pub use amiga::Amiga50Hz;
pub use amiga60::Amiga60Hz;
pub use dos::Dos70Hz;
pub use moon::Moon50Hz;
pub use xbox::Xbox60Hz;

#[cfg(test)]
mod tests;
