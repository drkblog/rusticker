use clap::ValueEnum;

pub mod stickerize;
pub use stickerize::remove_background;

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ModelType {
    U2netp,
    Rmbg,
    Birefnet,
}
