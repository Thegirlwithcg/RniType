//! rnitype-core — แกนของ RniType
//!
//! ลำดับชั้น (ตาม `Design/03-engineering/01-architecture.md`):
//! `device` → [`normalizer`] → [`gesture`] → [`keymap`] → injector
//!
//! ไฟล์นี้ไม่มี dependency ภายนอก ไม่มี `unsafe` และไม่รู้จัก Windows
//! เพื่อให้ทดสอบได้ด้วย `cargo test` บนเครื่องไหนก็ได้
#![forbid(unsafe_code)]

pub mod action;
pub mod gesture;
pub mod keymap;
pub mod normalizer;
pub mod pad;
pub mod typing;

pub use action::{Action, Command, Key, KeyPress, Mode, MouseButton, NamedKey};
pub use gesture::{Commit, GestureConfig, GestureEngine, GestureState};
pub use keymap::Lang;
pub use normalizer::{Dir8, Normalizer, NormalizerConfig, SectorTracker};
pub use pad::{Buttons, PadState, Vec2};
pub use typing::TypingPipeline;
