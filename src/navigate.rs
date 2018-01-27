use fnv::*;
use std::collections::BinaryHeap;
use std::cmp::Ordering;
use engine::controller::*;
use engine::location::*;
use engine::unit::*;

const AROUND: [Point; 8] = [
    (-1, 1), (0, 1), (1, 1),
    (-1, 0), (1, 0),
    (-1, -1), (0, -1), (1, -1),
];

const SEARCH_DEPTH: Time = 16;
const EXPIRE_TIME: Time = 8;
const MAX_HEAT: Heat = 10;

type Distance = i8;
type Time = i16;
type Heat = i8;
type ID = u16;
type Point = (Distance, Distance);
type TimePoint = (Distance, Distance, Time);

#[derive(Debug, Eq, PartialEq)]
struct Node {
    d: Distance,
    x: Distance,
    y: Distance,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ANode {
    a: Distance,
    x: Distance,
    y: Distance,
    t: Time,
    h: Heat,
}

#[derive(Debug)]
pub struct Navigator {
    w: Distance,
    h: Distance,
    t: Time,

    // Static map information
    terrain: Vec<Vec<Point>>,
    enemies: FnvHashSet<Point>,
    cache: FnvHashMap<Point, Vec<Distance>>,

    // Dynamic ally information
    expiration: FnvHashMap<ID, Time>,
    reserved: FnvHashSet<(Distance, Distance, Time)>,
    routes: FnvHashMap<ID, Vec<TimePoint>>,
    targets: FnvHashMap<u16, Point>,
}

impl Navigator {
    pub fn new(gc: &GameController) -> Self {
        let map = gc.starting_map(gc.planet());
        let w = map.width as Distance;
        let h = map.height as Distance;
        let t = 0;

        let mut terrain = vec![Vec::new(); (w*h) as usize];
        let enemies = FnvHashSet::default();
        let cache = FnvHashMap::default();
        let expiration = FnvHashMap::default();
        let routes = FnvHashMap::default();
        let reserved = FnvHashSet::default();
        let targets = FnvHashMap::default();

        for y in 0..h {
            for x in 0..w {
                let mut adj = &mut terrain[(y*w + x) as usize];
                for &(dx, dy) in &AROUND {
                    let (i, j) = (y + dy, x + dx);
                    if i < 0 || i >= h || j < 0 || j >= w || (dx == 0 && dy == 0) {
                        continue
                    } else if map.is_passable_terrain[i as usize][j as usize] {
                        adj.push((j, i));
                    }
                }
            }
        }
        Navigator { w, h, t, terrain, cache, enemies,
            expiration, reserved, routes, targets
        }
    }

    pub fn refresh(&mut self, gc: &GameController) {
        let enemy = gc.team().other();
        let origin = MapLocation::new(gc.planet(), 0, 0);
        self.t += 1;
        self.enemies = gc.sense_nearby_units_by_team(origin, 2500, enemy)
            .into_iter()
            .map(|enemy| enemy.location().map_location().unwrap())
            .map(|enemy| (enemy.x as Distance, enemy.y as Distance))
            .collect::<FnvHashSet<_>>();

        for (id, expiration) in self.expiration.iter_mut() {
            *expiration -= 1;
            if *expiration == 0 {
                self.targets.remove(id).unwrap();
                for point in self.routes.remove(id).unwrap() {
                    self.reserved.remove(&point);
                }
            }
        }
        self.expiration.retain(|_, &mut expiration| expiration != 0);
    }

    pub fn moves_between(&mut self, start: &MapLocation, end: &MapLocation) -> i32 {
        let (sx, sy) = (start.x as Distance, start.y as Distance);
        let (ex, ey) = (end.x as Distance, end.y as Distance);
        if !self.cache.contains_key(&(ex, ey)) {
            self.cache_bfs(end);
        }
        self.cache[&(ex, ey)][self.index(sx, sy)] as i32
    }

    pub fn navigate(&mut self, unit: &Unit, end: &MapLocation) -> Option<Direction> {
        let id = unit.id();
        let start = unit.location().map_location().unwrap();
        let (sx, sy) = (start.x as Distance, start.y as Distance);
        let (ex, ey) = (end.x as Distance, end.y as Distance);

        if sx == ex && sy == ey { return None }
        else if let Some(&(x, y)) = self.targets.get(&id) {
            if x == ex && y == ey {
                let route = self.routes.get(&id).unwrap();
                let current = route.iter()
                    .position(|&(x, y, t)| sx == x && sy == y && t == self.t)
                    .unwrap();
                let (x, y, _) = route[current - 1];
                return Self::to_direction(x - sx, y - sy)
            } else {
                self.expiration.remove(&id).unwrap();
                self.targets.remove(&id).unwrap();
                for point in self.routes.remove(&id).unwrap() {
                    self.reserved.remove(&point);
                }
            }
        }

        if !self.cache.contains_key(&(ex, ey)) {
            self.cache_bfs(end);
        }
        self.a_star(id, &start, end)
    }

    fn index(&self, x: Distance, y: Distance) -> usize { (y*self.w + x) as usize }

    fn to_direction(dx: Distance, dy: Distance) -> Option<Direction> {
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
        let (ex, ey) = (end.x as Distance, end.y as Distance);
        let mut distances = vec![i8::max_value(); (self.w*self.h) as usize];
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

    fn a_star(&mut self, unit: &Unit, start: &MapLocation, end: &MapLocation) {
        if start == end { return }
        let (sx, sy) = (start.x as Distance, start.y as Distance);
        let (ex, ey) = (end.x as Distance, end.y as Distance);
        let start_index = self.index(sx, sy);

        let heuristic = &self.cache[&(ex, ey)];
        let max_depth = SEARCH_DEPTH + self.t;
        let cd = unit.movement_cooldown().unwrap() as Heat;
        let mut distances = vec![i8::max_value(); (self.w*self.h) as usize];
        let mut heap = BinaryHeap::default();
        let mut path = FnvHashMap::default();
        let mut best = ANode {
            a: heuristic[start_index] + SEARCH_DEPTH as Distance,
            x: sx,
            y: sy,
            t: self.t + SEARCH_DEPTH,
            h: unit.movement_heat().unwrap() as Heat,
        };

        distances[self.index(sx, sy)] = 0;
        heap.push(ANode {
            a: heuristic[start_index],
            x: sx,
            y: sy,
            t: self.t,
            h: unit.movement_heat().unwrap() as Heat,
        });

        while let Some(node) = heap.pop() {

            // End search
            if node.t == max_depth { best = node; break }
            let node_index = self.index(node.x, node.y);

            // Otherwise
            if node.h > MAX_HEAT {
                let cost = if node.x == ex && node.y == ey { 0 } else { 1 };
                if !self.reserved.contains(&(node.x, node.y, node.t + 1))
                && !self.enemies.contains(&(node.x, node.y)) {
                    heap.push(ANode {
                        a: node.a + cost,
                        x: node.x,
                        y: node.y,
                        t: node.t + 1,
                        h: node.h - MAX_HEAT,
                    });
                    path.insert
                }
            } else {
                for (x, y) in self.terrain[node_index] {
                    if !self.reserved.contains(&(x, y, node.t + 1))
                    && !self.enemies.contains(&(x, y)) {
                        heap.push(ANode {
                            a: heuristic[self.index(x, y)] + node.a + 1,
                            x: x,
                            y: y,
                            t: node.t + 1,
                            h: node.h + cd - MAX_HEAT,
                        });
                    }
                }
            }
        };
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

impl Ord for ANode {
    fn cmp(&self, other: &Self) -> Ordering {
        other.a.cmp(&self.a)
            .then_with(|| other.t.cmp(&self.t))
            .then_with(|| self.x.cmp(&other.x))
            .then_with(|| self.y.cmp(&other.y))
    }
}

impl PartialOrd for ANode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
