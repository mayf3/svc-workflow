//! PostgreSQL implementation of the ADC legacy initial-import primitive.

mod receipt;
mod transaction;
mod validation;

pub use transaction::import;
