//! The single project palette, mirrored from `assets/palette/golden-hour.gpl`.
//!
//! The .gpl file is the authoring source of truth (artists read it); these
//! constants are the code-side mirror the shell renders with. The
//! `all_constants_match_the_gpl` test pins the two together, so adding or
//! editing a palette color requires editing both.

use bevy::prelude::*;

/// The 32 palette colors as 0xRRGGBB constants, in .gpl order.
pub const VOID_SHADOW: u32 = 0x0D0A14;
pub const DUSK_OUTLINE: u32 = 0x251D33;
pub const GRASS_SHADOW: u32 = 0x4C5A24;
pub const GRASS_BASE: u32 = 0x6E7C30;
pub const GRASS_LIGHT: u32 = 0x9AA040;
pub const DRY_GOLD: u32 = 0xCFA14A;
pub const SUNLIT_STRAW: u32 = 0xECCD6D;
pub const FOLIAGE_SHADOW: u32 = 0x1F3A2A;
pub const FOLIAGE_BASE: u32 = 0x2C5A36;
pub const FOLIAGE_LIGHT: u32 = 0x4A7A44;
pub const TRUNK_BARK: u32 = 0x4A3226;
pub const DIRT_SHADOW: u32 = 0x5F3F2C;
pub const DIRT_BASE: u32 = 0x96603A;
pub const DIRT_LIGHT: u32 = 0xD89A5E;
pub const ASPHALT_DARKEST: u32 = 0x121016;
pub const ASPHALT_BASE: u32 = 0x23202D;
pub const ASPHALT_LIGHT: u32 = 0x3B3748;
pub const WORN_EDGE: u32 = 0x7D7688;
pub const KERB_SHADOW: u32 = 0x7C1626;
pub const KERB_BASE: u32 = 0xD8263C;
pub const KERB_LIGHT: u32 = 0xFF6A52;
pub const PLAYER_BLUE: u32 = 0x0F8FC9;
pub const RIVAL_WHITE: u32 = 0xEDF1F4;
pub const RIVAL_ORANGE: u32 = 0xEF6A1F;
pub const RIVAL_PLUM: u32 = 0xA04A86;
pub const GLASS_DARK: u32 = 0x1B2632;
pub const CREAM_HIGHLIGHT: u32 = 0xFFF3D4;
pub const AMBER_ACCENT: u32 = 0xFFB52E;
pub const SIGNAL_CYAN: u32 = 0x37DFE8;
pub const BRAKELIGHT_RED: u32 = 0xFF3B4D;
pub const WATER_DEEP: u32 = 0x173F66;
pub const KERB_BONE: u32 = 0xE6D3A3;

/// Every constant as (gpl entry name, hex), pinned against the .gpl by test.
pub const ALL: [(&str, u32); 32] = [
    ("Void Shadow", VOID_SHADOW),
    ("Dusk Outline", DUSK_OUTLINE),
    ("Grass Shadow", GRASS_SHADOW),
    ("Grass Base", GRASS_BASE),
    ("Grass Light", GRASS_LIGHT),
    ("Dry Gold", DRY_GOLD),
    ("Sunlit Straw", SUNLIT_STRAW),
    ("Foliage Shadow", FOLIAGE_SHADOW),
    ("Foliage Base", FOLIAGE_BASE),
    ("Foliage Light", FOLIAGE_LIGHT),
    ("Trunk Bark", TRUNK_BARK),
    ("Dirt Shadow", DIRT_SHADOW),
    ("Dirt Base", DIRT_BASE),
    ("Dirt Light", DIRT_LIGHT),
    ("Asphalt Darkest", ASPHALT_DARKEST),
    ("Asphalt Base", ASPHALT_BASE),
    ("Asphalt Light", ASPHALT_LIGHT),
    ("Worn Edge", WORN_EDGE),
    ("Kerb Shadow", KERB_SHADOW),
    ("Kerb Base", KERB_BASE),
    ("Kerb Light", KERB_LIGHT),
    ("Player Blue", PLAYER_BLUE),
    ("Rival White", RIVAL_WHITE),
    ("Rival Orange", RIVAL_ORANGE),
    ("Rival Plum", RIVAL_PLUM),
    ("Glass Dark", GLASS_DARK),
    ("Cream Highlight", CREAM_HIGHLIGHT),
    ("Amber Accent", AMBER_ACCENT),
    ("Signal Cyan", SIGNAL_CYAN),
    ("Brakelight Red", BRAKELIGHT_RED),
    ("Water Deep", WATER_DEEP),
    ("Kerb Bone", KERB_BONE),
];

/// Converts a palette constant to a Bevy color.
pub fn color(rgb: u32) -> Color {
    let [r, g, b] = [(rgb >> 16) & 0xFF, (rgb >> 8) & 0xFF, rgb & 0xFF];
    Color::srgb_u8(r as u8, g as u8, b as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_constants_match_the_gpl_source_of_truth() {
        let gpl = include_str!("../../../assets/palette/golden-hour.gpl");
        let entries: Vec<(String, String)> = gpl
            .lines()
            .filter_map(|line| {
                let (rgb_name, hex) = line.split_once('#')?;
                let name = rgb_name.split('\t').nth(1)?.trim().to_owned();
                Some((name, hex.trim().to_ascii_lowercase()))
            })
            .collect();
        assert_eq!(entries.len(), ALL.len(), ".gpl entry count vs constants");
        for (name, hex) in &entries {
            let (_, rgb) = ALL
                .iter()
                .find(|(const_name, _)| const_name == name)
                .unwrap_or_else(|| panic!("no constant for .gpl entry {name:?}"));
            assert_eq!(
                &format!("{rgb:06x}"),
                hex,
                "constant for {name:?} diverges from the .gpl"
            );
        }
    }
}
