use derive_more::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, From, Into)]
pub struct GuiComponentId(pub u128);

impl Default for GuiComponentId {
    fn default() -> Self {
        Self::generate()
    }
}

impl GuiComponentId {
    pub fn generate() -> Self {
        Self(rand::random())
    }
}

pub mod button;
pub mod menu;
pub mod text_box;
