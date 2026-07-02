//! Pure-logic library at the heart of `springup`.
//!
//! `springup-core` owns everything that is *not* a terminal:
//! - The fully-resolved [`plan::ProjectPlan`] data model and its validation rules.
//! - The Spring Initializr REST client ([`initializr`]) with metadata caching.
//! - The custom template layer ([`template`]) driven by [`minijinja`] and embedded assets.
//! - The [`manifest`] (`springup.toml`) reader/writer for project-level state.
//! - The [`config`] module for global user configuration and precedence resolution.
//!
//! Everything in this crate is unit-testable, free of TTY I/O, and async-runtime aware.
//! The CLI crate (`springup-cli`) wraps these types with prompts, spinners, and pretty output.

#![forbid(unsafe_code)]
#![warn(
    missing_docs,
    missing_debug_implementations,
    rust_2018_idioms,
    unreachable_pub,
    clippy::doc_markdown,
    clippy::explicit_into_iter_loop,
    clippy::explicit_iter_loop,
    clippy::inconsistent_struct_constructor,
    clippy::map_unwrap_or,
    clippy::redundant_closure_for_method_calls,
    clippy::match_same_arms
)]

pub mod config;
pub mod error;
pub mod initializr;
pub mod manifest;
pub mod plan;
pub mod template;

pub use error::{Error, Result};
pub use plan::{
    ArchitectureKind, BuildTool, DependencyId, ExtraFeature, Language, Packaging, ProjectPlan,
};
