//! Shared design data; no GUI rendering types.
pub const CANVAS: u32 = 0x14161a;
pub const SURFACE: u32 = 0x1b1e24;
pub const ELEVATED: u32 = 0x282c34;
pub const BORDER: u32 = 0x414752;
pub const ACCENT: u32 = 0x8daafa;
pub const TEXT: u32 = 0xe1e7f1;
pub const MUTED: u32 = 0x939aa8;
pub const SUCCESS: u32 = 0x85cbb0;
pub const WARNING: u32 = 0xe8b777;
pub const ERROR: u32 = 0xeb8589;
pub const RADIUS: f32 = 6.;
pub const SPACING: f32 = 4.;
pub const INTER: &[u8] = include_bytes!("../../assets/fonts/Inter.ttf");
pub const MONO: &[u8] = include_bytes!("../../assets/fonts/JetBrainsMono.ttf");
