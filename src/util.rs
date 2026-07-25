use brdb::{
    BString, Brick, PrefabIntVec3, PrefabJson, PrefabPivot, PrefabPivots, PrefabVec3, World,
};
use std::ffi::OsStr;
use std::path::PathBuf;

pub struct GenOptions {
    pub size: u16,
    pub scale: u32,
    pub asset: BString,
    pub cull: bool,
    pub micro: bool,
    pub stud: bool,
    pub snap: bool,
    pub img: bool,
    pub glow: bool,
    pub hdmap: bool,
    pub lrgb: bool,
    pub nocollide: bool,
    pub quadtree: bool,
    pub greedy: bool,
    /// Height threshold above which to generate full solid fill layers (0 = disabled)
    pub gen_full_layers_above_height: u32,
}

// convert gamma to linear gamma
pub fn to_linear_gamma(c: u8) -> u8 {
    let cf = (c as f64) / 255.0;
    (if cf > 0.04045 {
        (cf / 1.055 + 0.0521327).powf(2.4) * 255.0
    } else {
        cf / 12.192 * 255.0
    }) as u8
}

// convert sRGB to linear rgb
pub fn to_linear_rgb(rgb: [u8; 4]) -> [u8; 4] {
    [
        to_linear_gamma(rgb[0]),
        to_linear_gamma(rgb[1]),
        to_linear_gamma(rgb[2]),
        rgb[3],
    ]
}

// given an array of bricks, create a save
pub fn bricks_to_save(bricks: Vec<Brick>) -> World {
    let mut world = World::new();
    world.add_bricks(bricks);
    world.meta.bundle.description = "Save generated from heightmap file".to_string();
    make_prefab(&mut world);
    world
}

/// Mark the world as a prefab so the game can snap it to the grid.
///
/// Without `Meta/Prefab.json` there are no pivots to place against and the game
/// drops the bricks wherever the cursor is, ignoring the grid entirely. Measured
/// against a brick placed by hand on the ground — edges on multiples of 10,
/// bottom at 0 — a pasted save landed on `x ≡ 5 (mod 10)` with its bottom at 5:
/// half a stud off horizontally and five units in the air, no matter what the
/// generator emitted.
///
/// The layout mirrors what the game itself writes when it re-saves one of our
/// files as a prefab: every pivot centered on the prefab's local origin,
/// `halfExtent` the true brick bounding box, and `addedGlobalGridOffset` the
/// negated bounds center, so that `center + addedGlobalGridOffset == 0`.
fn make_prefab(world: &mut World) {
    let Some((min, max)) = world.brick_bounds() else {
        return;
    };

    // Kept exact: a bounding box with an odd span centers on a half unit, and
    // only the integral part can go in the (integer) grid offset. The pivot
    // center carries whatever is left, which is how it stays the local origin.
    let center = [
        (min.x + max.x) as f64 / 2.0,
        (min.y + max.y) as f64 / 2.0,
        (min.z + max.z) as f64 / 2.0,
    ];
    let grid_offset = [
        -(center[0].round() as i32),
        -(center[1].round() as i32),
        -(center[2].round() as i32),
    ];

    let pivot = PrefabPivot {
        center: PrefabVec3 {
            x: center[0] + grid_offset[0] as f64,
            y: center[1] + grid_offset[1] as f64,
            z: center[2] + grid_offset[2] as f64,
        },
        half_extent: PrefabVec3 {
            x: (max.x - min.x) as f64 / 2.0,
            y: (max.y - min.y) as f64 / 2.0,
            z: (max.z - min.z) as f64 / 2.0,
        },
    };

    world.meta.bundle.level_type = "Prefab".to_string();
    world.meta.prefab = Some(PrefabJson {
        pivots: PrefabPivots {
            bottom_studs_pivot: pivot,
            studs_expanded_pivot: pivot,
            top_studs_pivot: pivot,
            bounds_pivot: pivot,
            ..Default::default()
        },
        added_global_grid_offset: PrefabIntVec3 {
            x: grid_offset[0],
            y: grid_offset[1],
            z: grid_offset[2],
        },
        ..Default::default()
    });
}

// get extension from filename
#[allow(unused)]
pub fn file_ext(filename: &PathBuf) -> Option<&str> {
    filename.extension().and_then(OsStr::to_str)
}

// write a world to a .brz or .brdb file based on the extension
pub fn write_world(world: &World, out_file: &str) -> Result<(), String> {
    if out_file.to_lowercase().ends_with(".brz") {
        let brz = world
            .to_brz_vec()
            .map_err(|e| format!("failed to encode brz: {e}"))?;
        std::fs::write(out_file, brz).map_err(|e| format!("failed to write file: {e}"))?;
    } else if out_file.to_lowercase().ends_with(".brdb") {
        world
            .write_brdb(out_file)
            .map_err(|e| format!("failed to write file: {e}"))?;
    } else {
        return Err("output file must end with .brz or .brdb".to_string());
    }
    Ok(())
}
