pub(crate) mod consts;
pub(crate) mod macros;
mod palette;
pub(crate) mod panel;
pub(crate) mod utils;
pub(crate) mod widgets;

pub(crate) use palette::{get as palette, init as init_palette};
