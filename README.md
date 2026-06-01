# lau-vibe-visualizer

Turns the mono-dimensional **vibe** scalar into voxel colors, materials, heights, and particle effects. The vibe is one number (-1.0 to 1.0) — everything visual follows deterministically from that number.

## What This Does

`lau-vibe-visualizer` is the rendering math layer for the Lau voxel game. Given a 16×16 grid of vibe values (a `VibeField`), it computes:

- **Colors** for every voxel column, interpolated between cold blue (negative), gray (neutral), and warm gold (positive)
- **Materials** — 9 voxel types from ice to starblock, determined by vibe thresholds
- **Heights** — base height ± vibe × 16 blocks
- **Particle density** — absolute vibe scaled to 0–100

It also provides `RoomVisualization`, a ready-to-use struct that packages a vibe field with visualization mode and color map, plus conservation error checking between fields.

## Key Idea

**One number drives everything.** Every visual property — color, material, height, particles — is a pure function of the vibe scalar. No lookup tables, no randomness, no hidden state. This makes the visual system:
- **Deterministic**: same vibe always produces the same visual
- **Testable**: every mapping has clear boundary conditions
- **Reversible**: you can infer approximate vibe from visual properties

The 16×16 grid maps to a Minecraft-style chunk (one column per vibe value), and the conservation check ensures vibe is preserved across transformations.

## Install

Add to your `Cargo.toml`:

```toml
[dependencies]
lau-vibe-visualizer = { git = "https://github.com/SuperInstance/lau-vibe-visualizer" }
```

Requires Rust **2021 edition**.

## Quick Start

```rust
use lau_vibe_visualizer::*;

// Create a radial vibe field — high in center, decaying outward
let field = VibeField::radial(1.0, 0.1);

// Wrap it in a room visualization
let viz = RoomVisualization::new("room-42", field, VizMode::ColorMap);

// Query visual properties at any point
let color = viz.color_at(7, 7);      // PackedColor near center
let height = viz.height_at(7, 7);    // base_height + vibe * 16
let material = viz.material_at(0, 0); // VoxelMaterial at corner
let particles = viz.particle_density_at(7, 7);

// Check conservation between two field states
let before = VibeField::from_fn(|_, _| 1.0);
let after = VibeField::from_fn(|_, _| 0.9);
let error = before.conservation_error(&after);
// error = |256.0 - 230.4| = 25.6

// Bulk conversion
let map = VibeColorMap::default_map();
let colors = before.to_colors(&map);
let materials = before.to_materials();
```

## API Reference

### PackedColor

A 32-bit RGBA color stored as `R(8) | G(8) | B(8) | A(8)`.

| Method | Description |
|---|---|
| `new(r, g, b, a)` | Construct from components |
| `r()`, `g()`, `b()`, `a()` | Extract individual channels |
| `lerp(&other, t)` | Linear interpolation, `t` clamped to [0, 1] |

### VibeColorMap

| Method | Signature | Description |
|---|---|---|
| `default_map()` | `Self` | Blue(60,60,200) ← Gray(140,140,140) → Gold(255,200,50) |
| `vibe_to_color(vibe)` | `PackedColor` | Map vibe [-1,1] to color via linear interpolation |

### VoxelMaterial (Enum)

9 variants: `Ice`, `Water`, `Stone`, `Grass`, `Wood`, `Sand`, `Crystal`, `GlowStone`, `StarBlock`.

| Method | Signature | Description |
|---|---|---|
| `from_vibe(vibe)` | `Self` | Select material by vibe range |
| `color()` | `PackedColor` | Default color for this material |
| `label()` | `&str` | Human-readable name |

Material vibe ranges and colors:

| Material | Vibe Range | Color |
|---|---|---|
| Ice | < -0.7 | (180, 220, 255, 255) |
| Water | -0.7 to -0.3 | (40, 100, 200, 220) |
| Stone | -0.3 to -0.1 | (120, 115, 110, 255) |
| Grass | -0.1 to 0.1 | (80, 160, 60, 255) |
| Wood | 0.1 to 0.3 | (139, 90, 43, 255) |
| Sand | 0.3 to 0.5 | (210, 190, 140, 255) |
| Crystal | 0.5 to 0.7 | (180, 100, 255, 200) |
| GlowStone | 0.7 to 0.9 | (255, 220, 100, 255) |
| StarBlock | ≥ 0.9 | (255, 255, 200, 255) |

### VibeField

A 16×16 grid of `f64` vibe values.

| Method | Signature | Description |
|---|---|---|
| `new()` | `Self` | All zeros |
| `from_fn(f)` | `Self` | Initialize with `f(x, z)` |
| `radial(center_vibe, decay)` | `Self` | Exponential radial falloff from center |
| `get(x, z)` | `f64` | Read vibe (clamped to 0–15) |
| `set(x, z, vibe)` | `()` | Write vibe (no-op if out of bounds) |
| `average()` | `f64` | Mean of all 256 values |
| `total_vibe()` | `f64` | Sum of all 256 values |
| `conservation_error(&other)` | `f64` | `|self.total - other.total|` |
| `to_materials()` | `[[VoxelMaterial; 16]; 16]` | Convert every cell to material |
| `to_colors(&map)` | `[[PackedColor; 16]; 16]` | Convert every cell to color |

### VizMode (Enum)

`ColorMap`, `HeightMap`, `MaterialView`, `ParticleDensity`, `GlowIntensity`

### RoomVisualization

| Method | Signature | Description |
|---|---|---|
| `new(room_id, field, mode)` | `Self` | Create visualization |
| `height_at(x, z)` | `i32` | `base_height + floor(vibe × 16)` |
| `color_at(x, z)` | `PackedColor` | Color from the color map |
| `material_at(x, z)` | `VoxelMaterial` | Material from vibe threshold |
| `particle_density_at(x, z)` | `u8` | `min(100, abs(vibe) × 100)` |

## How It Works

### Color Interpolation

`VibeColorMap::vibe_to_color` uses two-segment linear interpolation:

```
if vibe < 0:
    color = lerp(neutral, negative, |vibe|)
else:
    color = lerp(neutral, positive, vibe)
```

Each channel is interpolated independently with `round()` for integer output:
```
channel = round(a + (b - a) × t)
```

This produces a smooth gradient from blue through gray to gold.

### Material Selection

`VoxelMaterial::from_vibe` uses 9 half-open ranges covering [-1, 1]. The boundaries are at -0.7, -0.3, -0.1, 0.1, 0.3, 0.5, 0.7, 0.9. Each range maps to exactly one material, and the mapping is monotonically ordered from cold to warm.

### Radial Field

`VibeField::radial` generates a field using:

```
vibe(x, z) = center_vibe × e^(-dist × decay)
```

Where `dist = sqrt((x - 7.5)² + (z - 7.5)²)`, placing the center at (7.5, 7.5) — the geometric center of the 16×16 grid.

### Conservation Error

The conservation check is a simple total-energy comparison:

```
error = |Σ(field_a) - Σ(field_b)|
```

If the total vibe in a field changes between states, `conservation_error` quantifies by how much. Zero means perfect conservation.

### Height Mapping

Height is a linear function of vibe:

```
height = base_height + floor(vibe × 16)
```

With a base of 32 and vibe in [-1, 1], heights range from 16 to 48 — a 32-block range that maps nicely to a voxel chunk.

## The Math

### Linear Color Interpolation

For two colors A and B with parameter t ∈ [0, 1]:

```
C_r = round(A_r + (B_r - A_r) × t)
C_g = round(A_g + (B_g - A_g) × t)
C_b = round(A_b + (B_b - A_b) × t)
C_a = round(A_a + (B_a - A_a) × t)
```

This is standard linear interpolation in sRGB space. It's not perceptually uniform (gamma correction would be needed for that), but it's fast and deterministic.

### Exponential Decay

The radial field uses continuous exponential decay:

```
f(d) = v₀ × e^(-d × λ)
```

Where:
- `v₀` is the center vibe
- `d` is Euclidean distance from center
- `λ` is the decay rate

Properties:
- At d=0 (center): f = v₀ (full strength)
- As d→∞: f → 0
- The decay rate λ controls the "spread":
  - λ=0: flat field (no decay)
  - λ=1: ~37% at distance 1
  - λ=0.1: gentle gradient

### Conservation as L¹ Norm

The conservation error is the L¹ norm of the total vibe difference:

```
E = |Σᵢⱼ a[i][j] - Σᵢⱼ b[i][j]|
```

This measures total vibe gained or lost. For a 16×16 field, the maximum error (all vibes flip from -1 to 1) would be 512.

## Tests

**18 tests** covering:

- PackedColor construction and channel extraction
- Color linear interpolation (exact midpoint)
- Vibe-to-color mapping at boundaries (-1, 0, 1)
- Material selection across all 9 ranges
- Radial field: center is high, corners are lower
- VibeField average and total calculations
- Conservation error: zero for identical fields, nonzero for different
- Bulk to_materials and to_colors conversion
- RoomVisualization height mapping (positive and negative vibe)
- Particle density calculation
- Material colors are distinct
- Material labels

Run with `cargo test`.

## License

MIT
