//! Shared types for Authit.
//!
//! This crate contains types shared between client and server.

mod error;
mod kanidm;
mod session;

pub use error::Error;
pub use kanidm::{Entry, Group, Person};
pub use session::{decode_session, encode_session, UserSession, SESSION_COOKIE_NAME};
