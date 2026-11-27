// Generated from crates/robroz-tokens/tokens.json. Do not edit.
// Regenerate: cargo run -p robroz-tokens --bin gen

/// `tokens.json` version.
pub const VERSION: &str = "2026-11-20";

/// Shared brand values (same on every skin).
pub mod base {
    #[allow(unused_imports)]
    use crate::{Rgb, Rgba};
    pub mod avatar {
        #[allow(unused_imports)]
        use crate::{Rgb, Rgba};
        pub const COLORS: f32 = 10.0;
        pub const CREATE_EMPTY_OPACITY: f32 = 0.4;
        pub mod dark {
            #[allow(unused_imports)]
            use crate::{Rgb, Rgba};
            pub const C0: Rgb = [0xf0, 0x8a, 0x95];
            pub const C1: Rgb = [0xe9, 0x8b, 0xc6];
            pub const C2: Rgb = [0xc3, 0x9c, 0xf0];
            pub const C3: Rgb = [0xa3, 0xa6, 0xf5];
            pub const C4: Rgb = [0x7f, 0xb0, 0xec];
            pub const C5: Rgb = [0x5c, 0xc2, 0xcc];
            pub const C6: Rgb = [0x6c, 0xc7, 0xa0];
            pub const C7: Rgb = [0xb5, 0xc2, 0x5a];
            pub const C8: Rgb = [0xcf, 0x9e, 0x7c];
            pub const C9: Rgb = [0xaa, 0xa6, 0x9e];
        }
        pub const DEFAULT_HUES: f32 = 8.0;
        pub const EYE_DARK: Rgb = [0x16, 0x18, 0x1d];
        pub const EYE_LIGHT: Rgb = [0xff, 0xff, 0xff];
        pub const EYE_RADIUS: f32 = 0.055;
        pub mod light {
            #[allow(unused_imports)]
            use crate::{Rgb, Rgba};
            pub const C0: Rgb = [0xc0, 0x3a, 0x4b];
            pub const C1: Rgb = [0xb4, 0x3a, 0x86];
            pub const C2: Rgb = [0x8a, 0x49, 0xc4];
            pub const C3: Rgb = [0x58, 0x56, 0xc9];
            pub const C4: Rgb = [0x2f, 0x6d, 0xb5];
            pub const C5: Rgb = [0x12, 0x75, 0x80];
            pub const C6: Rgb = [0x2a, 0x76, 0x5c];
            pub const C7: Rgb = [0x65, 0x70, 0x1b];
            pub const C8: Rgb = [0x8a, 0x5a, 0x3c];
            pub const C9: Rgb = [0x6e, 0x6c, 0x66];
        }
        pub const MORE_BUTTON: f32 = 32.0;
        pub const NAME_HEIGHT: f32 = 52.0;
        pub const NAME_RADIUS: f32 = 16.0;
        pub const PAGE_MAX: f32 = 440.0;
        pub const PREVIEW: f32 = 160.0;
        pub const PREVIEW_GLYPH: f32 = 112.0;
        pub const PREVIEW_RADIUS: f32 = 32.0;
        pub const RULE: &str = "bro avatar = shape 0-11 + color 0-9 (robroz#194 indices). c0-c7 are hues, c8 brown, c9 grey, no white. Default from hash(bro id): shape mod 12, color mod 8. Light set >= 4.5:1 on window and surface (name text uses it); dark set >= 4.5:1 on dark window and surface. Eyes are two round dots.";
        pub const SECTION_GAP: f32 = 24.0;
        pub const SELECTED_RING: f32 = 2.0;
        pub const SHAPE_CELL: f32 = 56.0;
        pub const SHAPE_COLS: f32 = 6.0;
        pub const SHAPE_GLYPH: f32 = 36.0;
        pub const SHAPES: f32 = 12.0;
        pub const SWATCH: f32 = 32.0;
        pub const SWATCH_HIT: f32 = 44.0;
        pub const TITLE_HEIGHT: f32 = 40.0;
        pub const TITLE_MAX: f32 = 3.0;
        pub const TITLE_OVERLAP: f32 = 8.0;
        pub const TITLE_PAD: f32 = 8.0;
        pub const TITLE_RING: f32 = 2.0;
        pub const TITLE_SIZE: f32 = 24.0;
    }
    pub mod brand {
        #[allow(unused_imports)]
        use crate::{Rgb, Rgba};
        pub const INK: Rgb = [0x16, 0x18, 0x1d];
        pub const ORANGE: Rgb = [0xc8, 0x55, 0x1f];
        pub const PAPER: Rgb = [0xf4, 0xf1, 0xea];
        pub const RULE: &str = "orange only marks the primary action, 'waiting on you' and switch-on; one primary per card";
        pub const WHITE: Rgb = [0xff, 0xff, 0xff];
    }
    pub mod chat {
        #[allow(unused_imports)]
        use crate::{Rgb, Rgba};
        pub const AVATAR: f32 = 24.0;
        pub const AVATAR_GAP: f32 = 8.0;
        pub const BUBBLE_COMPACT_FRAC: f32 = 0.8;
        pub const BUBBLE_MAX_FRAC: f32 = 0.68;
        pub const BUBBLE_MINE_SIZE: f32 = 15.0;
        pub const BUBBLE_PAD_H: f32 = 14.0;
        pub const BUBBLE_PAD_V: f32 = 10.0;
        pub const BUBBLE_RADIUS: f32 = 18.0;
        pub const COLUMN_MARGIN: f32 = 24.0;
        pub const COLUMN_MAX: f32 = 720.0;
        pub const COMPOSER_BOTTOM: f32 = 16.0;
        pub const COMPOSER_HEIGHT: f32 = 48.0;
        pub const COMPOSER_PAD_LEFT: f32 = 16.0;
        pub const COMPOSER_RADIUS: f32 = 20.0;
        pub const COMPOSER_SEND: f32 = 32.0;
        pub const COMPOSER_SEND_EMPTY_OPACITY: f32 = 0.4;
        pub const COMPOSER_SEND_INSET: f32 = 8.0;
        pub const DATE_GAP: f32 = 16.0;
        pub const DATE_SIZE: f32 = 12.0;
        pub const GAP_BETWEEN: f32 = 16.0;
        pub const GAP_IN: f32 = 4.0;
        pub const GROUP_SECS: f32 = 300.0;
        pub const OWN_CORNER: f32 = 4.0;
        pub const OWN_RADIUS: f32 = 16.0;
        pub const THUMB_SIZE: f32 = 56.0;
        pub const TIME_SIZE: f32 = 12.0;
    }
    pub mod contrast {
        #[allow(unused_imports)]
        use crate::{Rgb, Rgba};
        pub const GLASS_RULE: &str = "measure text against the worst case: 100% orange wallpaper under the window tint + surface";
        pub const GRAPHICS_MIN: f32 = 3.0;
        pub const TEXT_MIN: f32 = 4.5;
    }
    pub mod layout {
        #[allow(unused_imports)]
        use crate::{Rgb, Rgba};
        pub const CARD_GAP: f32 = 10.0;
        pub const MIN_WIDTH: f32 = 390.0;
        pub const NARROW_BREAKPOINT: f32 = 480.0;
        pub const RULE: &str = "width <= 480 or touch input: sidebar collapses to a drawer, option buttons wrap, every hit area >= max(skin.touch_target, 44)";
        pub const STREAM_PADDING: f32 = 16.0;
        pub const TOUCH_FLOOR_NARROW: f32 = 44.0;
    }
    pub const SPACE: [f32; 11] = [2.0, 4.0, 6.0, 8.0, 10.0, 12.0, 14.0, 16.0, 20.0, 24.0, 32.0];
    pub mod status_card {
        #[allow(unused_imports)]
        use crate::{Rgb, Rgba};
        pub const DARK_STROKE: Rgb = [0x3f, 0x44, 0x4d];
        pub const PALETTE: &str = "robroz-card::palette() (frozen, not restated here)";
    }
    pub mod status_dot {
        #[allow(unused_imports)]
        use crate::{Rgb, Rgba};
        pub mod dark {
            #[allow(unused_imports)]
            use crate::{Rgb, Rgba};
            pub const BLOCKED: Rgb = [0xf5, 0xb7, 0x00];
            pub const DONE: Rgb = [0x4f, 0x9a, 0x53];
            pub const EXPIRED: Rgb = [0x6f, 0x6c, 0x66];
            pub const WAITING: Rgb = [0xc8, 0x55, 0x1f];
        }
        pub mod light {
            #[allow(unused_imports)]
            use crate::{Rgb, Rgba};
            pub const BLOCKED: Rgb = [0xf5, 0xb7, 0x00];
            pub const DONE: Rgb = [0x2e, 0x7d, 0x32];
            pub const EXPIRED: Rgb = [0x9a, 0x97, 0x8f];
            pub const WAITING: Rgb = [0xc8, 0x55, 0x1f];
        }
    }
    pub mod type_scale {
        #[allow(unused_imports)]
        use crate::{Rgb, Rgba};
        pub const BODY: f32 = 14.0;
        pub const BODY_LINE_HEIGHT: f32 = 22.0;
        pub const BUTTON: f32 = 13.0;
        pub const CAPTION: f32 = 11.0;
        pub const DISPLAY: f32 = 22.0;
        pub const LIST_TITLE: f32 = 15.0;
        pub const META: f32 = 12.5;
        pub const MONO: &str = "IBM Plex Mono";
        pub const SMALL: f32 = 12.0;
        pub const TITLE: f32 = 20.0;
        pub const WEIGHTS: [f32; 2] = [400.0, 700.0];
    }
}

/// The default solid skin (robroz-ui's own), light and dark.
pub mod solid {
    #[allow(unused_imports)]
    use crate::{Rgb, Rgba};
    pub mod dark {
        #[allow(unused_imports)]
        use crate::{Rgb, Rgba};
        pub mod controls {
            #[allow(unused_imports)]
            use crate::{Rgb, Rgba};
            pub const BUTTON_GAP: f32 = 8.0;
            pub const BUTTON_H: f32 = 32.0;
            pub const BUTTON_PX: f32 = 14.0;
            pub const BUTTON_SHAPE: &str = "rounded";
            pub const BUTTON_WEIGHT: &str = "400 / primary+selected 700";
            pub const CARD_PAD: &str = "12 / 14";
            pub const CARD_PAD_SIZE: f32 = 12.0;
            pub const CARD_PAD_WEIGHT: u16 = 14;
            pub const CHECKBOX: &str = "14 r3";
            pub const ROW_H: f32 = 52.0;
            pub const SECONDARY: &str = "outlined";
            pub const SIDEBAR_W: f32 = 220.0;
            pub const SWITCH: &str = "36×20 knob 16";
            pub const SWITCH_LABEL: bool = false;
            pub const TITLEBAR_H: &str = "system";
            pub const TOUCH_TARGET: f32 = 32.0;
            pub const WINDOW_BUTTONS: &str = "system";
        }
        pub mod material {
            #[allow(unused_imports)]
            use crate::{Rgb, Rgba};
            // BACKDROP: not used by this skin
            pub const BLUR: f32 = 0.0;
            // FALLBACK: not used by this skin
            // HIGHLIGHT: not used by this skin
            pub const MATERIAL_KIND: &str = "Solid";
        }
        pub mod motion {
            #[allow(unused_imports)]
            use crate::{Rgb, Rgba};
            pub const MOTION: &str = "120ms ease-out";
            pub const MOTION_MS: u32 = 120;
            pub const REDUCED_MOTION: &str = "0ms";
            pub const REDUCED_MOTION_MS: u32 = 0;
        }
        pub mod palette {
            #[allow(unused_imports)]
            use crate::{Rgb, Rgba};
            pub const ACCENT: Rgb = [0xc8, 0x55, 0x1f];
            pub const ACCENT_HOVER: Rgb = [0xb2, 0x4a, 0x1a];
            pub const ACCENT_PRESSED: Rgb = [0x9f, 0x42, 0x17];
            pub const ACCENT_STRONG: Rgb = [0xb8, 0x4d, 0x1b];
            pub const BORDER: Rgb = [0x2d, 0x31, 0x39];
            pub const BORDER2: Rgb = [0x3f, 0x44, 0x4d];
            pub const CONTENT: Rgb = [0x16, 0x18, 0x1d];
            pub const CONTROL: Rgba = Rgba { rgb: [0x00, 0x00, 0x00], alpha: 0.0 };
            pub const CONTROL_CHECKED: Rgb = [0x19, 0x1c, 0x21];
            // CONTROL_EDGE_BOTTOM: not used by this skin
            pub const CONTROL_HOVER: Rgb = [0x19, 0x1c, 0x21];
            pub const DIVIDER: Rgb = [0x2d, 0x31, 0x39];
            pub const FOCUS: Rgb = [0xc8, 0x55, 0x1f];
            // FOCUS_INNER: not used by this skin
            pub const KNOB: Rgb = [0xf4, 0xf1, 0xea];
            pub const ON_ACCENT: Rgb = [0xff, 0xff, 0xff];
            pub const SELECTION: Rgba = Rgba { rgb: [0xf4, 0xf1, 0xea], alpha: 0.08 };
            pub const SIDEBAR: Rgb = [0x19, 0x1c, 0x21];
            // STATE_LAYER: not used by this skin
            pub const SUNKEN: Rgb = [0x19, 0x1c, 0x21];
            pub const SURFACE: Rgb = [0x1e, 0x21, 0x27];
            pub const TEXT: Rgb = [0xf4, 0xf1, 0xea];
            pub const TEXT2: Rgb = [0xa9, 0xa5, 0x9c];
            pub const TEXT3: Rgb = [0x8f, 0x8c, 0x84];
            pub const TOGGLE_OFF: Rgb = [0x56, 0x5b, 0x64];
            pub const WINDOW: Rgb = [0x16, 0x18, 0x1d];
            // WINDOW_EDGE: not used by this skin
        }
        pub mod radii {
            #[allow(unused_imports)]
            use crate::{Rgb, Rgba};
            pub const R_BUTTON: f32 = 6.0;
            pub const R_CARD: f32 = 8.0;
            pub const R_PATH: f32 = 4.0;
            pub const R_PREVIEW: f32 = 6.0;
            pub const R_ROW: f32 = 6.0;
            pub const R_SWITCH: f32 = 10.0;
            pub const R_TAG: f32 = 4.0;
            // R_WINDOW: not used by this skin
        }
        pub mod shadow {
            #[allow(unused_imports)]
            use crate::{Rgb, Rgba};
            // SH_CARD: not used by this skin
            pub const SH_FLYOUT: &str = "0 4 16 rgba(0,0,0,0.18)";
            pub const SH_WINDOW: &str = "system";
        }
        pub mod stroke {
            #[allow(unused_imports)]
            use crate::{Rgb, Rgba};
            pub const FOCUS_OFFSET: f32 = 2.0;
            pub const W_CARD: f32 = 1.0;
            pub const W_CONTROL: f32 = 1.0;
            pub const W_DIVIDER: f32 = 1.0;
            pub const W_FOCUS: f32 = 2.0;
        }
        pub mod type_ {
            #[allow(unused_imports)]
            use crate::{Rgb, Rgba};
            pub const F_BODY: f32 = 14.0;
            pub const F_BUTTON: f32 = 13.0;
            pub const F_NAME: &str = "13/700";
            pub const F_NAME_SIZE: f32 = 13.0;
            pub const F_NAME_WEIGHT: u16 = 700;
            pub const F_STATUS: f32 = 12.0;
            pub const F_TITLE: &str = "15/700";
            pub const F_TITLE_SIZE: f32 = 15.0;
            pub const F_TITLE_WEIGHT: u16 = 700;
            pub const FONT: &str = "system-ui + Noto Sans SC";
        }
    }
    pub mod light {
        #[allow(unused_imports)]
        use crate::{Rgb, Rgba};
        pub mod controls {
            #[allow(unused_imports)]
            use crate::{Rgb, Rgba};
            pub const BUTTON_GAP: f32 = 8.0;
            pub const BUTTON_H: f32 = 32.0;
            pub const BUTTON_PX: f32 = 14.0;
            pub const BUTTON_SHAPE: &str = "rounded";
            pub const BUTTON_WEIGHT: &str = "400 / primary+selected 700";
            pub const CARD_PAD: &str = "12 / 14";
            pub const CARD_PAD_SIZE: f32 = 12.0;
            pub const CARD_PAD_WEIGHT: u16 = 14;
            pub const CHECKBOX: &str = "14 r3";
            pub const ROW_H: f32 = 52.0;
            pub const SECONDARY: &str = "outlined";
            pub const SIDEBAR_W: f32 = 220.0;
            pub const SWITCH: &str = "36×20 knob 16";
            pub const SWITCH_LABEL: bool = false;
            pub const TITLEBAR_H: &str = "system";
            pub const TOUCH_TARGET: f32 = 32.0;
            pub const WINDOW_BUTTONS: &str = "system";
        }
        pub mod material {
            #[allow(unused_imports)]
            use crate::{Rgb, Rgba};
            // BACKDROP: not used by this skin
            pub const BLUR: f32 = 0.0;
            // FALLBACK: not used by this skin
            // HIGHLIGHT: not used by this skin
            pub const MATERIAL_KIND: &str = "Solid";
        }
        pub mod motion {
            #[allow(unused_imports)]
            use crate::{Rgb, Rgba};
            pub const MOTION: &str = "120ms ease-out";
            pub const MOTION_MS: u32 = 120;
            pub const REDUCED_MOTION: &str = "0ms";
            pub const REDUCED_MOTION_MS: u32 = 0;
        }
        pub mod palette {
            #[allow(unused_imports)]
            use crate::{Rgb, Rgba};
            pub const ACCENT: Rgb = [0xc8, 0x55, 0x1f];
            pub const ACCENT_HOVER: Rgb = [0xb2, 0x4a, 0x1a];
            pub const ACCENT_PRESSED: Rgb = [0x9f, 0x42, 0x17];
            pub const ACCENT_STRONG: Rgb = [0xb8, 0x4d, 0x1b];
            pub const BORDER: Rgb = [0xe3, 0xdd, 0xd1];
            pub const BORDER2: Rgb = [0xcf, 0xc8, 0xba];
            pub const CONTENT: Rgb = [0xf4, 0xf1, 0xea];
            pub const CONTROL: Rgba = Rgba { rgb: [0x00, 0x00, 0x00], alpha: 0.0 };
            pub const CONTROL_CHECKED: Rgb = [0xef, 0xeb, 0xe3];
            // CONTROL_EDGE_BOTTOM: not used by this skin
            pub const CONTROL_HOVER: Rgb = [0xef, 0xeb, 0xe3];
            pub const DIVIDER: Rgb = [0xe3, 0xdd, 0xd1];
            pub const FOCUS: Rgb = [0xc8, 0x55, 0x1f];
            // FOCUS_INNER: not used by this skin
            pub const KNOB: Rgb = [0xff, 0xff, 0xff];
            pub const ON_ACCENT: Rgb = [0xff, 0xff, 0xff];
            pub const SELECTION: Rgba = Rgba { rgb: [0x16, 0x18, 0x1d], alpha: 0.08 };
            pub const SIDEBAR: Rgb = [0xef, 0xeb, 0xe3];
            // STATE_LAYER: not used by this skin
            pub const SUNKEN: Rgb = [0xef, 0xeb, 0xe3];
            pub const SURFACE: Rgb = [0xfb, 0xfa, 0xf7];
            pub const TEXT: Rgb = [0x16, 0x18, 0x1d];
            pub const TEXT2: Rgb = [0x5c, 0x5a, 0x55];
            pub const TEXT3: Rgb = [0x75, 0x72, 0x69];
            pub const TOGGLE_OFF: Rgb = [0xa3, 0x9c, 0x8f];
            pub const WINDOW: Rgb = [0xf4, 0xf1, 0xea];
            // WINDOW_EDGE: not used by this skin
        }
        pub mod radii {
            #[allow(unused_imports)]
            use crate::{Rgb, Rgba};
            pub const R_BUTTON: f32 = 6.0;
            pub const R_CARD: f32 = 8.0;
            pub const R_PATH: f32 = 4.0;
            pub const R_PREVIEW: f32 = 6.0;
            pub const R_ROW: f32 = 6.0;
            pub const R_SWITCH: f32 = 10.0;
            pub const R_TAG: f32 = 4.0;
            // R_WINDOW: not used by this skin
        }
        pub mod shadow {
            #[allow(unused_imports)]
            use crate::{Rgb, Rgba};
            // SH_CARD: not used by this skin
            pub const SH_FLYOUT: &str = "0 4 16 rgba(0,0,0,0.18)";
            pub const SH_WINDOW: &str = "system";
        }
        pub mod stroke {
            #[allow(unused_imports)]
            use crate::{Rgb, Rgba};
            pub const FOCUS_OFFSET: f32 = 2.0;
            pub const W_CARD: f32 = 1.0;
            pub const W_CONTROL: f32 = 1.0;
            pub const W_DIVIDER: f32 = 1.0;
            pub const W_FOCUS: f32 = 2.0;
        }
        pub mod type_ {
            #[allow(unused_imports)]
            use crate::{Rgb, Rgba};
            pub const F_BODY: f32 = 14.0;
            pub const F_BUTTON: f32 = 13.0;
            pub const F_NAME: &str = "13/700";
            pub const F_NAME_SIZE: f32 = 13.0;
            pub const F_NAME_WEIGHT: u16 = 700;
            pub const F_STATUS: f32 = 12.0;
            pub const F_TITLE: &str = "15/700";
            pub const F_TITLE_SIZE: f32 = 15.0;
            pub const F_TITLE_WEIGHT: u16 = 700;
            pub const FONT: &str = "system-ui + Noto Sans SC";
        }
    }
}
