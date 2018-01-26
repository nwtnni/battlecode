use fnv::*;
use std::collections::BinaryHeap;
use std::cmp::Ordering;
use engine::map::*;
use engine::controller::*;
use engine::location::*;
use engine::world::*;

const AROUND: [Point; 9] = [
    (-1, 1), (0, 1), (1, 1),
    (-1, 0), (0, 0), (1, 0),
    (-1, -1), (0, -1), (1, -1),
];

const SEARCH_DEPTH: i16 = 8;

type Point = (i16, i16);

#[derive(Debug, Eq, PartialEq)]
struct Node {
    d: i16,
    x: i16,
    y: i16,
}

#[derive(Debug, Eq, PartialEq)]
struct HNode {
    h: i16,
    d: i16,
    x: i16,
    y: i16,
}

#[derive(Debug)]
pub struct Navigator {
    w: i16,
    h: i16,
    terrain: Vec<Vec<Point>>,
    enemies: FnvHashSet<Point>,
    reserve: FnvHashSet<(i16, i16, i16)>,
    cache: FnvHashMap<Point, Vec<i16>>,
}

impl Navigator {
    pub fn new(gc: &GameController) -> Self {
        let map = gc.starting_map(gc.planet());
        let w = map.width as i16;
        let h = map.height as i16;

        let mut terrain = vec![Vec::new(); (w*h) as usize];
        let cache = FnvHashMap::default();
        let enemies = FnvHashSet::default();
        let reserve = FnvHashSet::default();

        for y in 0..h {
            for x in 0..w {
                let mut adj = &mut terrain[(y*w + x) as usize];
                for &(dx, dy) in &AROUND {
                    let (i, j) = (y + dy, x + dx);
                    if i < 0 || i >= h || j < 0 || j >= w {
                        continue
                    } else if map.is_passable_terrain[i as usize][j as usize] {
                        adj.push((j, i));
                    }
                }
            }
        }
        Navigator { w, h, terrain, cache, reserve, enemies }
    }

    pub fn refresh(&mut self, gc: &GameController) {
        let enemy = gc.team().other();
        let origin = MapLocation::new(gc.planet(), 0, 0);
        self.enemies = gc.sense_nearby_units_by_team(origin, 2500, enemy)
            .into_iter()
            .map(|enemy| enemy.location().map_location().unwrap())
            .map(|enemy| (enemy.x as i16, enemy.y as i16))
            .collect::<FnvHashSet<_>>();
        self.reserve.clear();
    }

    pub fn moves_between(&mut self, start: &MapLocation, end: &MapLocation) -> i16 {
        let (sx, sy) = (start.x as i16, start.y as i16);
        let (ex, ey) = (end.x as i16, end.y as i16);
        if !self.cache.contains_key(&(ex, ey)) {
            self.cache_bfs(end);
        }
        self.cache[&(ex, ey)][self.index(sx, sy)]
    }

    pub fn smart(&mut self, start: &MapLocation, end: &MapLocation) -> Option<Direction> {
        let (ex, ey) = (end.x as i16, end.y as i16);
        if !self.cache.contains_key(&(ex, ey)) {
            self.cache_bfs(end);
        }
        self.a_star(start, end)
    }

    fn index(&self, x: i16, y: i16) -> usize { (y*self.w + x) as usize }

    fn to_direction(dx: i16, dy: i16) -> Option<Direction> {
        match (dx, dy) {
            (-1 , -1) => Some(Direction::Southwest),
            (-1 ,  0) => Some(Direction::West),
            (-1 ,  1) => Some(Direction::Northwest),
            (0  , -1) => Some(Direction::South),
            (0  ,  1) => Some(Direction::North),
            (1  , -1) => Some(Direction::Southeast),
            (1  ,  0) => Some(Direction::East),
            (1  ,  1) => Some(Direction::Northeast),
            _ => None,
        }
    }

    fn cache_bfs(&mut self, end: &MapLocation) {
        let (ex, ey) = (end.x as i16, end.y as i16);
        let mut distances = vec![i16::max_value(); (self.w*self.h) as usize];
        let mut heap = BinaryHeap::default();
        distances[self.index(ex, ey)] = 0;
        heap.push(Node { d: 0, x: ex, y: ey });

        while let Some(node) = heap.pop() {
            let node_index = self.index(node.x, node.y);
            let d = distances[node_index];

            for &(x, y) in &self.terrain[node_index] {
                let next_index = self.index(x, y);
                let (da, db) = (d + 1, distances[next_index]);
                if da < db {
                    distances[next_index] = da;
                    heap.push(Node { d: da, x, y });
                }
            }
        }
        self.cache.insert((ex, ey), distances);
    }

    fn a_star(&mut self, start: &MapLocation, end: &MapLocation) -> Option<Direction> {
        if start == end { return None }
        let (sx, sy) = (start.x as i16, start.y as i16);
        let (ex, ey) = (end.x as i16, end.y as i16);

        let heuristic = &self.cache[&(ex, ey)];
        let mut distances = vec![(self.w*self.h + 1); (self.w*self.h) as usize];
        let mut heap = BinaryHeap::default();
        let mut path = FnvHashMap::default();
        let mut frontier = Vec::new();
        let mut found = -1;

        distances[self.index(sx, sy)] = 0;
        heap.push(HNode { h: 0, d: 0, x: sx, y: sy });

        while let Some(node) = heap.pop() {
            // Found goal
            if node.x == ex && node.y == ey { found = node.d; break }

            let node_index = self.index(node.x, node.y);
            let d = distances[node_index];

            // End search
            if d >= SEARCH_DEPTH {
                for &(x, y) in &self.terrain[node_index] {
                    let next_index = self.index(x, y);
                    let estimate = heuristic[next_index] + d + 1;
                    path.insert((x, y, d + 1), (node.x, node.y, d));
                    frontier.push(HNode { h: estimate, d: d + 1, x, y });
                }
            } else {
                for &(x, y) in &self.terrain[node_index] {
                    let next_index = self.index(x, y);
                    let (da, db) = (d + 1, distances[next_index]);

                    if !self.reserve.contains(&(x, y, da))
                    && (!self.enemies.contains(&(x, y)) || (x == ex && y == ey))
                    && da < db {
                        distances[next_index] = da;
                        path.insert((x, y, da), (node.x, node.y, d));
                        heap.push(HNode { h: da + heuristic[next_index], d: da , x, y });
                    }
                }
            }
        }

        // Retrace path
        let mut node = if found >= 0 { (ex, ey, found) } else {
            let end = frontier.into_iter().min();
            match end {
                Some(point) => (point.x, point.y, point.d),
                None => (sx, sy, 0),
            }
        };

        while let Some(&prev) = path.get(&node) {
            let (x1, y1, _) = prev;
            let (x2, y2, t2) = node;
            self.reserve.insert((x1, y1, t2));
            self.reserve.insert((x2, y2, t2));
            if prev == (sx, sy, 0) { break } else { node = prev; }
        }

        self.reserve.insert((sx, sy, 0));
        Self::to_direction(node.0 - sx, node.1 - sy)
    }
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        other.d.cmp(&self.d)
            .then_with(|| self.x.cmp(&other.x))
            .then_with(|| self.y.cmp(&other.y))
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for HNode {
    fn cmp(&self, other: &Self) -> Ordering {
        other.h.cmp(&self.h)
            .then_with(|| other.d.cmp(&self.d))
            .then_with(|| self.x.cmp(&other.x))
            .then_with(|| self.y.cmp(&other.y))
    }
}

impl PartialOrd for HNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
