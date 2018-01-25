use fnv::FnvHashMap;
use std::collections::BinaryHeap;
use std::cmp::Ordering;
use engine::map::*;
use engine::controller::*;
use engine::location::*;

const AROUND: [i16; 3] = [-1, 0, 1];

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
pub struct Route {
    w: i16,
    h: i16,
    distances: Vec<i16>,
}

#[derive(Debug)]
pub struct Navigator {
    w: i16,
    h: i16,
    terrain: Vec<Vec<Point>>,
    cache: FnvHashMap<Point, Route>,
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
                for &dy in &AROUND {
                    for &dx in &AROUND {
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
        }
        Navigator { w, h, terrain, cache }
    }

    pub fn navigate(&mut self, gc: &GameController, start: &MapLocation, end: &MapLocation) -> Option<Direction> {
        if start == end { return None }
        let key = (end.x as i16, end.y as i16);
        if !self.cache.contains_key(&key) {
            self.cache.insert(key, Route::new(&self.terrain, self.w, self.h, end));
        }

        let route = &self.cache[&key];
        let mut distances = vec![i16::max_value(); (self.w*self.h) as usize];
        let mut heap = BinaryHeap::default();
        let mut path = FnvHashMap::default();

        distances[(start.y as i16 * self.w + start.x as i16) as usize] = 0;
        heap.push(HNode { h: 0, d: 0, x: start.x as i16, y: start.y as i16 });

        while let Some(node) = heap.pop() {

            // Skip explored nodes
            let node_idx = (node.y*self.w + node.x) as usize;
            let d = distances[node_idx];
            if d < node.d { continue }

            // Found goal
            if node.x as i32 == end.x && node.y as i32 == end.y { break }

            for &(x, y) in &self.terrain[node_idx] {

                // Skip nodes that are occupied
                let neighbor = MapLocation::new(gc.planet(), x as i32, y as i32);
                if !(x as i32 == end.x && y as i32 == end.y)
                && gc.can_sense_location(neighbor)
                && !gc.is_occupiable(neighbor).unwrap() {
                    continue
                }

                let next_idx = (y*self.w + x) as usize;
                let da = d + 1;
                let db = distances[next_idx];

                if da < db {
                    distances[next_idx] = da;
                    path.insert((x, y), (node.x, node.y));
                    heap.push(HNode { h: da + route.distance(&neighbor), d: da , x, y });
                }
            }
        }

        let mut node = (end.x as i16, end.y as i16);
        while let Some(&prev) = path.get(&node) {
            if prev == (start.x as i16, start.y as i16) {
                break
            } else {
                node = prev;
            }
        }

        let (mut x, mut y) = (node.0 - start.x as i16, node.1 - start.y as i16);

        if x < -1 || x > 1 || y < -1 || y > 1 {
            let mut min = i16::max_value();
            for &dy in &AROUND {
                for &dx in &AROUND {
                    let (i, j) = (start.y as i16 + dy, start.x as i16 + dx);
                    if i < 0 || i >= self.h || j < 0 || j >= self.w || (dx == 0 && dy == 0) {
                        continue
                    }
                    let target = MapLocation::new(start.planet, j as i32, i as i32);
                    let distance = route.distance(&target);
                    if distance < min && (!gc.can_sense_location(target) || gc.is_occupiable(target).unwrap()) {
                        x = dx;
                        y = dy;
                        min = distance;
                    }
                }
            }

            if distances[(start.y*self.w as i32 + start.x) as usize] <= min { return None }
        } else if x == 0 && y == 0 { return None }

        Some(match (x, y) {
            (-1, -1) => Direction::Southwest,
            (-1, 0) => Direction::West,
            (-1, 1) => Direction::Northwest,
            (0, -1) => Direction::South,
            (0, 1) => Direction::North,
            (1, -1) => Direction::Southeast,
            (1, 0) => Direction::East,
            (1, 1) => Direction::Northeast,
            _ => unreachable!()
        })
    }

    pub fn between(&mut self, from: &MapLocation, to: &MapLocation) -> i16 {
        let key = (to.x as i16, to.y as i16);
        if !self.cache.contains_key(&key) {
            self.cache.insert(key, Route::new(&self.terrain, self.w, self.h, to));
        }
        self.cache[&key].distance(from)
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

impl Route {
    pub fn new(terrain: &Vec<Vec<Point>>, w: i16, h: i16, end: &MapLocation) -> Self {
        let mut distances = vec![i16::max_value(); (w*h) as usize];
        let mut heap = BinaryHeap::default();

        distances[(end.y as i16 * w + end.x as i16) as usize] = 0;
        heap.push(Node { d: 0, x: end.x as i16, y: end.y as i16 });

        while let Some(node) = heap.pop() {
            let d = distances[(node.y*w + node.x) as usize];
            if d < node.d { continue }

            for &(x, y) in &terrain[(node.y*w + node.x) as usize] {
                let index = (y*w + x) as usize;
                let da = d + 1;
                let db = distances[index];

                if da < db {
                    distances[index] = da;
                    heap.push(Node { d: da, x, y });
                }
            }
        }

        Route { w, h, distances }
    }

    pub fn distance(&self, start: &MapLocation) -> i16 {
        self.distances[(start.y as i16 * self.w + start.x as i16) as usize]
    }
}
