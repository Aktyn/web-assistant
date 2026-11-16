pub(super) mod consts;
pub(super) mod macros;
mod palette;
pub(super) mod panel;
pub(super) mod utils;
pub(super) mod widgets;

pub(crate) use palette::{get as palette, init as init_palette};
