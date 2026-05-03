use std::{collections::HashSet, fs};

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Máscara Tiled para GID local (remove bits de flip nos 3 bits altos).
pub const TILED_GID_MASK: u32 = 0x1FFF_FFFF;
pub const TILED_FLIP_H: u32 = 0x8000_0000;
pub const TILED_FLIP_V: u32 = 0x4000_0000;
pub const TILED_FLIP_D: u32 = 0x2000_0000;

#[inline]
pub fn tiled_base_gid(gid: u32) -> u32 {
    gid & TILED_GID_MASK
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TilemapNode {
    pub map_width: u32,
    pub map_height: u32,
    pub tile_width: u32,
    pub tile_height: u32,
    pub layers: Vec<TileLayer>,
    pub tilesets: Vec<TilesetInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileLayer {
    pub name: String,
    pub data: Vec<u32>,
    pub visible: bool,
    pub opacity: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TilesetInfo {
    pub first_gid: u32,
    pub columns: u32,
    pub tile_width: u32,
    pub tile_height: u32,
    pub texture_path: String,
}

impl TilemapNode {
    pub fn load_tiled_json(path: &str) -> Result<TilemapNode, String> {
        let text = fs::read_to_string(path)
            .map_err(|e| format!("Falha ao ler tilemap '{}': {}", path, e))?;
        let root: Value = serde_json::from_str(&text)
            .map_err(|e| format!("JSON Tiled inválido '{}': {}", path, e))?;

        let orientation = root
            .get("orientation")
            .and_then(Value::as_str)
            .unwrap_or("orthogonal");
        if orientation != "orthogonal" {
            return Err(format!(
                "Tilemap '{}' usa orientation='{}'; RS2 suporta apenas 'orthogonal'",
                path, orientation
            ));
        }

        let map_width = read_u32(&root, "width")?;
        let map_height = read_u32(&root, "height")?;
        let tile_width = read_u32(&root, "tilewidth")?;
        let tile_height = read_u32(&root, "tileheight")?;

        let mut layers = Vec::new();
        if let Some(raw_layers) = root.get("layers").and_then(Value::as_array) {
            for layer in raw_layers {
                if layer.get("type").and_then(Value::as_str) != Some("tilelayer") {
                    continue;
                }
                let data_values = layer
                    .get("data")
                    .and_then(Value::as_array)
                    .ok_or_else(|| "Layer tilelayer sem campo data em array".to_string())?;
                let mut data = Vec::with_capacity(data_values.len());
                for value in data_values {
                    let gid = value
                        .as_u64()
                        .ok_or_else(|| "GID inválido em layer tilelayer".to_string())?;
                    data.push(u32::try_from(gid).map_err(|_| "GID fora de u32".to_string())?);
                }
                layers.push(TileLayer {
                    name: layer
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or("Layer")
                        .to_string(),
                    data,
                    visible: layer.get("visible").and_then(Value::as_bool).unwrap_or(true),
                    opacity: layer
                        .get("opacity")
                        .and_then(Value::as_f64)
                        .unwrap_or(1.0) as f32,
                });
            }
        }

        let mut tilesets = Vec::new();
        if let Some(raw_tilesets) = root.get("tilesets").and_then(Value::as_array) {
            for tileset in raw_tilesets {
                tilesets.push(TilesetInfo {
                    first_gid: read_u32_with_default(tileset, "firstgid", 1)?,
                    columns: read_u32_with_default(tileset, "columns", 1)?,
                    tile_width: read_u32_with_default(tileset, "tilewidth", tile_width)?,
                    tile_height: read_u32_with_default(tileset, "tileheight", tile_height)?,
                    texture_path: tileset
                        .get("image")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                });
            }
        }

        Ok(TilemapNode {
            map_width,
            map_height,
            tile_width,
            tile_height,
            layers,
            tilesets,
        })
    }

    pub fn build_colliders(&self, solid_gids: &[u32], layer_idx: usize) -> Vec<(f32, f32, f32, f32)> {
        let Some(layer) = self.layers.get(layer_idx) else {
            return Vec::new();
        };
        let width = self.map_width as usize;
        let height = self.map_height as usize;
        if width == 0 || height == 0 || layer.data.is_empty() {
            return Vec::new();
        }

        let solids: HashSet<u32> = solid_gids.iter().copied().filter(|gid| *gid != 0).collect();
        if solids.is_empty() {
            return Vec::new();
        }

        let mut used = vec![false; width.saturating_mul(height)];
        let mut rects = Vec::new();

        for row in 0..height {
            for col in 0..width {
                let idx = row * width + col;
                if used.get(idx).copied().unwrap_or(true) || !self.is_solid_at(layer, &solids, col, row) {
                    continue;
                }

                let mut rect_w = 1usize;
                while col + rect_w < width {
                    let next_idx = row * width + col + rect_w;
                    if used.get(next_idx).copied().unwrap_or(true)
                        || !self.is_solid_at(layer, &solids, col + rect_w, row)
                    {
                        break;
                    }
                    rect_w += 1;
                }

                let mut rect_h = 1usize;
                'grow_h: while row + rect_h < height {
                    for x in col..(col + rect_w) {
                        let next_idx = (row + rect_h) * width + x;
                        if used.get(next_idx).copied().unwrap_or(true)
                            || !self.is_solid_at(layer, &solids, x, row + rect_h)
                        {
                            break 'grow_h;
                        }
                    }
                    rect_h += 1;
                }

                for y in row..(row + rect_h) {
                    for x in col..(col + rect_w) {
                        let mark_idx = y * width + x;
                        if let Some(slot) = used.get_mut(mark_idx) {
                            *slot = true;
                        }
                    }
                }

                rects.push((
                    col as f32 * self.tile_width as f32,
                    row as f32 * self.tile_height as f32,
                    rect_w as f32 * self.tile_width as f32,
                    rect_h as f32 * self.tile_height as f32,
                ));
            }
        }

        rects
    }

    pub fn get_tile(&self, layer: usize, col: u32, row: u32) -> u32 {
        let Some(idx) = self.index(col, row) else {
            return 0;
        };
        self.layers
            .get(layer)
            .and_then(|l| l.data.get(idx))
            .copied()
            .unwrap_or(0)
    }

    pub fn set_tile(&mut self, layer: usize, col: u32, row: u32, gid: u32) {
        let Some(idx) = self.index(col, row) else {
            return;
        };
        if let Some(tile_layer) = self.layers.get_mut(layer) {
            if let Some(tile) = tile_layer.data.get_mut(idx) {
                *tile = gid;
            }
        }
    }

    /// Tileset ativo para o GID (maior `first_gid` ≤ gid) e índice local no atlas.
    pub fn resolve_tileset_for_gid(&self, gid: u32) -> Option<(&TilesetInfo, u32)> {
        let gid = tiled_base_gid(gid);
        if gid == 0 {
            return None;
        }
        let ts = self
            .tilesets
            .iter()
            .filter(|ts| gid >= ts.first_gid)
            .max_by_key(|ts| ts.first_gid)?;
        Some((ts, gid.saturating_sub(ts.first_gid)))
    }

    fn index(&self, col: u32, row: u32) -> Option<usize> {
        if col >= self.map_width || row >= self.map_height {
            return None;
        }
        Some(row as usize * self.map_width as usize + col as usize)
    }

    fn is_solid_at(&self, layer: &TileLayer, solids: &HashSet<u32>, col: usize, row: usize) -> bool {
        let idx = row * self.map_width as usize + col;
        layer
            .data
            .get(idx)
            .map(|gid| solids.contains(gid))
            .unwrap_or(false)
    }
}

fn read_u32(root: &Value, key: &str) -> Result<u32, String> {
    let value = root
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("Campo '{}' ausente ou inválido", key))?;
    u32::try_from(value).map_err(|_| format!("Campo '{}' fora de u32", key))
}

fn read_u32_with_default(root: &Value, key: &str, default: u32) -> Result<u32, String> {
    match root.get(key).and_then(Value::as_u64) {
        Some(value) => u32::try_from(value).map_err(|_| format!("Campo '{}' fora de u32", key)),
        None => Ok(default),
    }
}
