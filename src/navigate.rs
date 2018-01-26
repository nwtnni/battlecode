use fnv::FnvHashMap;
use std::collections::BinaryHeap;
use std::cmp::Ordering;
use engine::map::*;
use engine::controller::*;
use engine::location::*;

const AROUND: [Point; 9] = [
    (-1, 1), (0, 1), (1, 1),
    (-1, 0), (0, 0), (1, 0),
    (-1, -1), (0, -1), (1, -1),
];

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
    cache: FnvHashMap<Point, Vec<i16>>,
}

impl Navigator {
    pub fn new(map: &PlanetMap) -> Self {
        let w = map.width as i16;
        let h = map.height as i16;
        let cache = FnvHashMap::default();

        let mut terrain = Vec::new();
        for _ in 0..(w*h) { terrain.push(Vec::new()); }

        for y in 0..h {
            for x in 0..w {
                let mut adj = &mut terrain[(y*w + x) as usize];
                for &(dx, dy) in &AROUND {
                    let i = y + dy;
                    let j = x + dx;
                    if i < 0 || i >= h || j < 0 || j >= w || (dx == 0 && dy == 0) {
                        continue
                    } else if map.is_passable_terrain[i as usize][j as usize] {
                        adj.push((j, i));
                    }
                }
            }
        }
        Navigator { w, h, terrain, cache }
    }

    pub fn moves_between(&mut self, start: &MapLocation, end: &MapLocation) -> i16 {
        let (sx, sy) = (start.x as i16, start.y as i16);
        let (ex, ey) = (end.x as i16, end.y as i16);
        if !self.cache.contains_key(&(ex, ey)) {
            self.cache_bfs(end);
        }
        self.cache[&(ex, ey)][self.index(sx, sy)]
    }

    pub fn smart(&mut self, gc: &GameController, start: &MapLocation, end: &MapLocation) -> Option<Direction> {
        let (ex, ey) = (end.x as i16, end.y as i16);
        if !self.cache.contains_key(&(ex, ey)) {
            self.cache_bfs(end);
        }
        self.a_star(gc, start, end).or(self.bfs(gc, start, end))
    }

    pub fn dumb(&mut self, gc: &GameController, start: &MapLocation, end: &MapLocation) -> Option<Direction> {
        let (ex, ey) = (end.x as i16, end.y as i16);
        if !self.cache.contains_key(&(ex, ey)) {
            self.cache_bfs(end);
        }
        self.bfs(gc, start, end)
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
            if d < node.d { continue }

            for &(x, y) in &self.terrain[node_index] {
                let index = self.index(x, y);
                let da = d + 1;
                let db = distances[index];

                if da < db {
                    distances[index] = da;
                    heap.push(Node { d: da, x, y });
                }
            }
        }
        self.cache.insert((ex, ey), distances);
    }

    fn a_star(&self, gc: &GameController, start: &MapLocation, end: &MapLocation) -> Option<Direction> {
        if start == end { return None }
        let (sx, sy) = (start.x as i16, start.y as i16);
        let (ex, ey) = (end.x as i16, end.y as i16);

        let heuristic = &self.cache[&(ex, ey)];
        let mut distances = vec![i16::max_value(); (self.w*self.h) as usize];
        let mut heap = BinaryHeap::default();
        let mut path = FnvHashMap::default();

        distances[self.index(sx, sy)] = 0;
        heap.push(HNode { h: 0, d: 0, x: sx, y: sy });

        while let Some(node) = heap.pop() {

            // Skip explored nodes
            let node_index = self.index(node.x, node.y);
            let d = distances[node_index];
            if d < node.d { continue }

            // Found goal
            if node.x == ex && node.y == ey { break }

            for &(x, y) in &self.terrain[node_index] {
                let next = MapLocation::new(gc.planet(), x as i32, y as i32);
                if !(x == ex && y == ey) && gc.can_sense_location(next) && !gc.is_occupiable(next).unwrap() {
                    continue
                }

                let next_index = (y*self.w + x) as usize;
                let (da, db) = (d + 1, distances[next_index]);

                if da < db {
                    distances[next_index] = da;
                    path.insert((x, y), (node.x, node.y));
                    heap.push(HNode { h: da + heuristic[next_index], d: da , x, y });
                }
            }
        }

        // Retrace path
        let mut node = (ex, ey);
        while let Some(&prev) = path.get(&node) {
            if prev == (sx, sy) { break } else { node = prev; }
        }

        Self::to_direction(node.0 - sx, node.1 - sy)
    }

    fn bfs(&self, gc: &GameController, start: &MapLocation, end: &MapLocation) -> Option<Direction> {
        let (sx, sy) = (start.x as i16, start.y as i16);
        let (ex, ey) = (end.x as i16, end.y as i16);
        let distances = &self.cache[&(ex, ey)];
        let (mut x, mut y) = (0, 0);
        let mut min = i16::max_value();

        for &(dx, dy) in &AROUND {
            let (i, j) = (sx + dx, sy + dy);
            if i < 0 || i >= self.h || j < 0 || j >= self.w { continue }
            let distance = distances[self.index(j, i)];
            if distance > min { continue }
            let target = MapLocation::new(start.planet, j as i32, i as i32);
            if (dx == 0 && dy == 0) || !gc.can_sense_location(target) || gc.is_occupiable(target).unwrap() {
                x = dx;
                y = dy;
                min = distance;
            }
        }

        Self::to_direction(x, y)
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
