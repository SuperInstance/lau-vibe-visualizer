//! Lau Vibe Visualizer — turns the mono-dimensional vibe scalar into visuals.
//!
//! The vibe is ONE number. Everything visual follows from that number.
//! No hand-waving — the math is deterministic and verifiable.

use serde::{Deserialize, Serialize};

/// A packed voxel color — low-level, 32 bits.
/// R(8) | G(8) | B(8) | A(8)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackedColor(pub u32);

impl PackedColor {
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self(((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8) | (a as u32))
    }

    pub fn r(&self) -> u8 { ((self.0 >> 24) & 0xFF) as u8 }
    pub fn g(&self) -> u8 { ((self.0 >> 16) & 0xFF) as u8 }
    pub fn b(&self) -> u8 { ((self.0 >> 8) & 0xFF) as u8 }
    pub fn a(&self) -> u8 { (self.0 & 0xFF) as u8 }

    /// Linearly interpolate two colors.
    pub fn lerp(&self, other: &PackedColor, t: f64) -> PackedColor {
        let t = t.clamp(0.0, 1.0);
        PackedColor::new(
            lerp_u8(self.r(), other.r(), t),
            lerp_u8(self.g(), other.g(), t),
            lerp_u8(self.b(), other.b(), t),
            lerp_u8(self.a(), other.a(), t),
        )
    }
}

fn lerp_u8(a: u8, b: u8, t: f64) -> u8 {
    (a as f64 + (b as f64 - a as f64) * t).round() as u8
}

/// Vibe → color mapping. The vibe scalar (-1 to 1) maps deterministically to color.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VibeColorMap {
    /// Negative vibe → cold colors (blue/purple)
    pub negative: PackedColor,
    /// Zero vibe → neutral (gray)
    pub neutral: PackedColor,
    /// Positive vibe → warm colors (gold/green)
    pub positive: PackedColor,
}

impl VibeColorMap {
    pub fn default_map() -> Self {
        Self {
            negative: PackedColor::new(60, 60, 200, 255),   // cool blue
            neutral: PackedColor::new(140, 140, 140, 255),   // gray
            positive: PackedColor::new(255, 200, 50, 255),   // warm gold
        }
    }

    /// Convert vibe scalar to color.
    pub fn vibe_to_color(&self, vibe: f64) -> PackedColor {
        let vibe = vibe.clamp(-1.0, 1.0);
        if vibe < 0.0 {
            self.neutral.lerp(&self.negative, -vibe)
        } else {
            self.neutral.lerp(&self.positive, vibe)
        }
    }
}

/// Voxel material type — determined by vibe level.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum VoxelMaterial {
    Ice,        // vibe < -0.7
    Water,      // -0.7 to -0.3
    Stone,      // -0.3 to -0.1
    Grass,      // -0.1 to 0.1
    Wood,       // 0.1 to 0.3
    Sand,       // 0.3 to 0.5
    Crystal,    // 0.5 to 0.7
    GlowStone,  // 0.7 to 0.9
    StarBlock,  // > 0.9
}

impl VoxelMaterial {
    pub fn from_vibe(vibe: f64) -> Self {
        match vibe {
            v if v < -0.7 => Self::Ice,
            v if v < -0.3 => Self::Water,
            v if v < -0.1 => Self::Stone,
            v if v < 0.1 => Self::Grass,
            v if v < 0.3 => Self::Wood,
            v if v < 0.5 => Self::Sand,
            v if v < 0.7 => Self::Crystal,
            v if v < 0.9 => Self::GlowStone,
            _ => Self::StarBlock,
        }
    }

    pub fn color(&self) -> PackedColor {
        match self {
            Self::Ice => PackedColor::new(180, 220, 255, 255),
            Self::Water => PackedColor::new(40, 100, 200, 220),
            Self::Stone => PackedColor::new(120, 115, 110, 255),
            Self::Grass => PackedColor::new(80, 160, 60, 255),
            Self::Wood => PackedColor::new(139, 90, 43, 255),
            Self::Sand => PackedColor::new(210, 190, 140, 255),
            Self::Crystal => PackedColor::new(180, 100, 255, 200),
            Self::GlowStone => PackedColor::new(255, 220, 100, 255),
            Self::StarBlock => PackedColor::new(255, 255, 200, 255),
        }
    }

    pub fn label(&self) -> &str {
        match self {
            Self::Ice => "Ice", Self::Water => "Water", Self::Stone => "Stone",
            Self::Grass => "Grass", Self::Wood => "Wood", Self::Sand => "Sand",
            Self::Crystal => "Crystal", Self::GlowStone => "GlowStone", Self::StarBlock => "StarBlock",
        }
    }
}

/// A vibe field — the vibe at every point in a chunk (16x16 grid, one vibe per column).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VibeField {
    pub vibes: [[f64; 16]; 16],
}

impl VibeField {
    pub fn new() -> Self { Self { vibes: [[0.0; 16]; 16] } }

    pub fn from_fn(f: impl Fn(usize, usize) -> f64) -> Self {
        let mut field = Self::new();
        for x in 0..16 {
            for z in 0..16 {
                field.vibes[x][z] = f(x, z);
            }
        }
        field
    }

    /// Radial vibe field — highest at center, decays outward.
    pub fn radial(center_vibe: f64, decay: f64) -> Self {
        Self::from_fn(|x, z| {
            let dx = x as f64 - 7.5;
            let dz = z as f64 - 7.5;
            let dist = (dx * dx + dz * dz).sqrt();
            center_vibe * (-dist * decay).exp()
        })
    }

    /// Get the vibe at a position.
    pub fn get(&self, x: usize, z: usize) -> f64 {
        self.vibes[x.min(15)][z.min(15)]
    }

    /// Set the vibe at a position.
    pub fn set(&mut self, x: usize, z: usize, vibe: f64) {
        if x < 16 && z < 16 { self.vibes[x][z] = vibe; }
    }

    /// Average vibe across the field.
    pub fn average(&self) -> f64 {
        let sum: f64 = self.vibes.iter().flat_map(|row| row.iter()).sum();
        sum / 256.0
    }

    /// Conservation check: total vibe should be preserved.
    pub fn total_vibe(&self) -> f64 {
        self.vibes.iter().flat_map(|row| row.iter()).sum()
    }

    /// Convert the field to voxel materials.
    pub fn to_materials(&self) -> [[VoxelMaterial; 16]; 16] {
        let mut mats = [[VoxelMaterial::Grass; 16]; 16];
        for x in 0..16 {
            for z in 0..16 {
                mats[x][z] = VoxelMaterial::from_vibe(self.vibes[x][z]);
            }
        }
        mats
    }

    /// Convert to packed colors for the renderer.
    pub fn to_colors(&self, color_map: &VibeColorMap) -> [[PackedColor; 16]; 16] {
        let mut colors = [[PackedColor::new(0, 0, 0, 255); 16]; 16];
        for x in 0..16 {
            for z in 0..16 {
                colors[x][z] = color_map.vibe_to_color(self.vibes[x][z]);
            }
        }
        colors
    }

    /// Diff two vibe fields — returns the conservation error.
    pub fn conservation_error(&self, other: &VibeField) -> f64 {
        (self.total_vibe() - other.total_vibe()).abs()
    }
}

/// Visualization mode — how the renderer should display vibes.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum VizMode {
    /// Flat color per voxel from vibe
    ColorMap,
    /// Height of voxels proportional to vibe
    HeightMap,
    /// Material type from vibe bands
    MaterialView,
    /// Particle effects density from |vibe|
    ParticleDensity,
    /// Glow intensity from vibe
    GlowIntensity,
}

/// A complete visualization config for a room.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomVisualization {
    pub room_id: String,
    pub field: VibeField,
    pub mode: VizMode,
    pub color_map: VibeColorMap,
    pub base_height: i32,
}

impl RoomVisualization {
    pub fn new(room_id: &str, field: VibeField, mode: VizMode) -> Self {
        Self { room_id: room_id.into(), field, mode, color_map: VibeColorMap::default_map(), base_height: 32 }
    }

    /// Get the height at a position based on vibe.
    pub fn height_at(&self, x: usize, z: usize) -> i32 {
        let vibe = self.field.get(x, z);
        self.base_height + (vibe * 16.0) as i32
    }

    /// Get the color at a position.
    pub fn color_at(&self, x: usize, z: usize) -> PackedColor {
        self.color_map.vibe_to_color(self.field.get(x, z))
    }

    /// Get the material at a position.
    pub fn material_at(&self, x: usize, z: usize) -> VoxelMaterial {
        VoxelMaterial::from_vibe(self.field.get(x, z))
    }

    /// Particle count for a position (0-100).
    pub fn particle_density_at(&self, x: usize, z: usize) -> u8 {
        let vibe = self.field.get(x, z);
        (vibe.abs() * 100.0).min(100.0) as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packed_color() {
        let c = PackedColor::new(255, 128, 64, 255);
        assert_eq!(c.r(), 255);
        assert_eq!(c.g(), 128);
        assert_eq!(c.b(), 64);
        assert_eq!(c.a(), 255);
    }

    #[test]
    fn test_color_lerp() {
        let a = PackedColor::new(0, 0, 0, 255);
        let b = PackedColor::new(100, 100, 100, 255);
        let mid = a.lerp(&b, 0.5);
        assert_eq!(mid.r(), 50);
        assert_eq!(mid.g(), 50);
    }

    #[test]
    fn test_vibe_to_color_negative() {
        let map = VibeColorMap::default_map();
        let c = map.vibe_to_color(-1.0);
        assert_eq!(c, map.negative);
    }

    #[test]
    fn test_vibe_to_color_positive() {
        let map = VibeColorMap::default_map();
        let c = map.vibe_to_color(1.0);
        assert_eq!(c, map.positive);
    }

    #[test]
    fn test_vibe_to_color_neutral() {
        let map = VibeColorMap::default_map();
        let c = map.vibe_to_color(0.0);
        assert_eq!(c, map.neutral);
    }

    #[test]
    fn test_material_from_vibe() {
        assert_eq!(VoxelMaterial::from_vibe(-0.8), VoxelMaterial::Ice);
        assert_eq!(VoxelMaterial::from_vibe(-0.5), VoxelMaterial::Water);
        assert_eq!(VoxelMaterial::from_vibe(0.0), VoxelMaterial::Grass);
        assert_eq!(VoxelMaterial::from_vibe(0.6), VoxelMaterial::Crystal);
        assert_eq!(VoxelMaterial::from_vibe(0.95), VoxelMaterial::StarBlock);
    }

    #[test]
    fn test_vibe_field_radial() {
        let field = VibeField::radial(1.0, 0.1);
        assert!(field.get(7, 7) > 0.5); // center is high
        assert!(field.get(0, 0) < field.get(7, 7)); // corner is lower
    }

    #[test]
    fn test_vibe_field_average() {
        let field = VibeField::from_fn(|_, _| 0.5);
        assert!((field.average() - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_vibe_field_total() {
        let field = VibeField::from_fn(|_, _| 1.0);
        assert!((field.total_vibe() - 256.0).abs() < 1e-10);
    }

    #[test]
    fn test_conservation_error() {
        let a = VibeField::from_fn(|_, _| 1.0);
        let b = VibeField::from_fn(|_, _| 1.0);
        assert!(a.conservation_error(&b) < 1e-10);
    }

    #[test]
    fn test_conservation_error_nonzero() {
        let a = VibeField::from_fn(|_, _| 1.0);
        let b = VibeField::from_fn(|_, _| 0.9);
        assert!(a.conservation_error(&b) > 20.0);
    }

    #[test]
    fn test_to_materials() {
        let field = VibeField::from_fn(|_, _| 0.6);
        let mats = field.to_materials();
        assert_eq!(mats[0][0], VoxelMaterial::Crystal);
    }

    #[test]
    fn test_to_colors() {
        let field = VibeField::from_fn(|_, _| 1.0);
        let map = VibeColorMap::default_map();
        let colors = field.to_colors(&map);
        assert_eq!(colors[0][0], map.positive);
    }

    #[test]
    fn test_room_viz_height() {
        let field = VibeField::from_fn(|_, _| 0.5);
        let viz = RoomVisualization::new("test", field, VizMode::HeightMap);
        assert_eq!(viz.height_at(0, 0), 40); // 32 + 0.5*16 = 40
    }

    #[test]
    fn test_room_viz_negative_height() {
        let field = VibeField::from_fn(|_, _| -0.5);
        let viz = RoomVisualization::new("test", field, VizMode::HeightMap);
        assert_eq!(viz.height_at(0, 0), 24); // 32 + (-0.5)*16 = 24
    }

    #[test]
    fn test_particle_density() {
        let field = VibeField::from_fn(|_, _| 0.8);
        let viz = RoomVisualization::new("test", field, VizMode::ParticleDensity);
        assert_eq!(viz.particle_density_at(0, 0), 80);
    }

    #[test]
    fn test_material_colors_differ() {
        let ice = VoxelMaterial::Ice.color();
        let fire = VoxelMaterial::StarBlock.color();
        assert_ne!(ice, fire);
    }

    #[test]
    fn test_material_labels() {
        assert_eq!(VoxelMaterial::Crystal.label(), "Crystal");
        assert_eq!(VoxelMaterial::GlowStone.label(), "GlowStone");
    }
}
