use std::{cmp::Ordering, collections::{BinaryHeap, HashMap, HashSet}};

use serde::{Deserialize, Serialize};

use crate::world::tilemap::TilemapNode;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavGrid {
    pub width: u32,
    pub height: u32,
    pub cell_size: f32,
    pub allow_diagonal: bool,
    pub solid: Vec<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OpenNode {
    f: i32,
    g: i32,
    x: i32,
    y: i32,
}

impl Ord for OpenNode {
    fn cmp(&self, other: &Self) -> Ordering {
        // BinaryHeap é max-heap; inverte a ordenação para virar min-heap por f-score.
        other
            .f
            .cmp(&self.f)
            .then_with(|| other.g.cmp(&self.g))
            .then_with(|| other.y.cmp(&self.y))
            .then_with(|| other.x.cmp(&self.x))
    }
}

impl PartialOrd for OpenNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl NavGrid {
    pub fn new(width: u32, height: u32, cell_size: f32) -> NavGrid {
        let safe_cell_size = if cell_size.is_finite() && cell_size > 0.0 { cell_size } else { 1.0 };
        NavGrid {
            width,
            height,
            cell_size: safe_cell_size,
            allow_diagonal: false,
            solid: vec![false; width as usize * height as usize],
        }
    }

    pub fn from_tilemap(map: &TilemapNode, solid_gids: &[u32], layer: usize) -> NavGrid {
        let mut grid = NavGrid::new(map.map_width, map.map_height, map.tile_width.max(1) as f32);
        let solids: HashSet<u32> = solid_gids.iter().copied().filter(|gid| *gid != 0).collect();
        for row in 0..map.map_height {
            for col in 0..map.map_width {
                let gid = map.get_tile(layer, col, row);
                if solids.contains(&gid) {
                    grid.set_solid(col as i32, row as i32, true);
                }
            }
        }
        grid
    }

    pub fn set_solid(&mut self, x: i32, y: i32, solid: bool) {
        if let Some(idx) = self.index(x, y) {
            if let Some(cell) = self.solid.get_mut(idx) {
                *cell = solid;
            }
        }
    }

    pub fn is_solid(&self, x: i32, y: i32) -> bool {
        let Some(idx) = self.index(x, y) else {
            return true;
        };
        self.solid.get(idx).copied().unwrap_or(true)
    }

    pub fn find_path(&self, from: (f32, f32), to: (f32, f32)) -> Option<Vec<(f32, f32)>> {
        if self.width == 0 || self.height == 0 || self.cell_size <= 0.0 || !self.cell_size.is_finite() {
            return None;
        }

        let start = self.world_to_cell(from.0, from.1);
        let goal = self.world_to_cell(to.0, to.1);
        if !self.in_bounds(start.0, start.1) || !self.in_bounds(goal.0, goal.1) {
            return None;
        }
        if self.is_solid(start.0, start.1) || self.is_solid(goal.0, goal.1) {
            return None;
        }
        if start == goal {
            return Some(vec![self.cell_to_world_center(start.0, start.1)]);
        }

        let mut open = BinaryHeap::new();
        let mut came_from: HashMap<(i32, i32), (i32, i32)> = HashMap::new();
        let mut g_score: HashMap<(i32, i32), i32> = HashMap::new();
        let mut closed: HashSet<(i32, i32)> = HashSet::new();

        g_score.insert(start, 0);
        open.push(OpenNode {
            f: self.heuristic(start, goal),
            g: 0,
            x: start.0,
            y: start.1,
        });

        while let Some(current) = open.pop() {
            let current_pos = (current.x, current.y);
            if current_pos == goal {
                return Some(self.reconstruct_path(came_from, current_pos));
            }
            if !closed.insert(current_pos) {
                continue;
            }

            for (nx, ny, step_cost) in self.neighbors(current.x, current.y) {
                let neighbor = (nx, ny);
                if closed.contains(&neighbor) || self.is_solid(nx, ny) {
                    continue;
                }

                if step_cost > 10 {
                    // Evita cortar canto na diagonal quando duas células ortogonais bloqueiam a passagem.
                    let dx = nx - current.x;
                    let dy = ny - current.y;
                    if self.is_solid(current.x + dx, current.y) || self.is_solid(current.x, current.y + dy) {
                        continue;
                    }
                }

                let tentative_g = current.g.saturating_add(step_cost);
                if tentative_g < *g_score.get(&neighbor).unwrap_or(&i32::MAX) {
                    came_from.insert(neighbor, current_pos);
                    g_score.insert(neighbor, tentative_g);
                    open.push(OpenNode {
                        f: tentative_g.saturating_add(self.heuristic(neighbor, goal)),
                        g: tentative_g,
                        x: nx,
                        y: ny,
                    });
                }
            }
        }

        None
    }

    fn index(&self, x: i32, y: i32) -> Option<usize> {
        if !self.in_bounds(x, y) {
            return None;
        }
        Some(y as usize * self.width as usize + x as usize)
    }

    fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.width as i32 && y < self.height as i32
    }

    fn world_to_cell(&self, x: f32, y: f32) -> (i32, i32) {
        ((x / self.cell_size).floor() as i32, (y / self.cell_size).floor() as i32)
    }

    fn cell_to_world_center(&self, x: i32, y: i32) -> (f32, f32) {
        ((x as f32 + 0.5) * self.cell_size, (y as f32 + 0.5) * self.cell_size)
    }

    fn neighbors(&self, x: i32, y: i32) -> Vec<(i32, i32, i32)> {
        let mut out = Vec::with_capacity(if self.allow_diagonal { 8 } else { 4 });
        for (dx, dy, cost) in [(0, -1, 10), (1, 0, 10), (0, 1, 10), (-1, 0, 10)] {
            let nx = x + dx;
            let ny = y + dy;
            if self.in_bounds(nx, ny) {
                out.push((nx, ny, cost));
            }
        }
        if self.allow_diagonal {
            for (dx, dy, cost) in [(-1, -1, 14), (1, -1, 14), (1, 1, 14), (-1, 1, 14)] {
                let nx = x + dx;
                let ny = y + dy;
                if self.in_bounds(nx, ny) {
                    out.push((nx, ny, cost));
                }
            }
        }
        out
    }

    fn heuristic(&self, a: (i32, i32), b: (i32, i32)) -> i32 {
        let dx = (a.0 - b.0).abs();
        let dy = (a.1 - b.1).abs();
        if self.allow_diagonal {
            14 * dx.min(dy) + 10 * (dx.max(dy) - dx.min(dy))
        } else {
            10 * (dx + dy)
        }
    }

    fn reconstruct_path(&self, came_from: HashMap<(i32, i32), (i32, i32)>, mut current: (i32, i32)) -> Vec<(f32, f32)> {
        let mut cells = vec![current];
        while let Some(prev) = came_from.get(&current).copied() {
            current = prev;
            cells.push(current);
        }
        cells.reverse();
        cells.into_iter().map(|(x, y)| self.cell_to_world_center(x, y)).collect()
    }
}
