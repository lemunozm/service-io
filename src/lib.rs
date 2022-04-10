//! Check the [Github README](https://github.com/lemunozm/service-io),
//! to see an overview of the library.

#[cfg(doctest)]
// Tells rustdoc where is the README to compile and test the rust code found there
doc_comment::doctest!("../README.md");

pub mod channel;
pub mod interface;
pub mod message;

pub mod engine;

pub mod connectors;
pub mod services;

pub mod util;
