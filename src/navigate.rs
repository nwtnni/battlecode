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

const SEARCH_DEPTH: Time = 8;
const EXPIRE_TIME: Time = 4;
const MAX_HEAT: Heat = 10;

type Distance = i8;
type Time = i16;
type Heat = i8;
type ID = u16;
type Point = (Distance, Distance);
type TimePoint = (Distance, Distance, Time, Heat);

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
    unmoved: FnvHashSet<Point>,
    targets: FnvHashMap<u16, Point>,

    // Execution order
    moves: FnvHashMap<u16, Option<Direction>>,
    order: Vec<u16>,
}

impl Navigator {
    pub fn new(gc: &GameController) -> Self {
        let map = gc.starting_map(gc.planet());
        let w = map.width as Distance;
        let h = map.height as Distance;
        let t = 0;

        let mut terrain = vec![Vec::new(); w as usize * h as usize];
        let enemies = FnvHashSet::default();
        let cache = FnvHashMap::default();
        let expiration = FnvHashMap::default();
        let routes = FnvHashMap::default();
        let reserved = FnvHashSet::default();
        let targets = FnvHashMap::default();
        let unmoved = FnvHashSet::default();
        let moves = FnvHashMap::default();
        let order = Vec::new();

        for y in 0..h {
            for x in 0..w {
                let mut adj = &mut terrain[(y as usize * w as usize) + x as usize];
                for &(dx, dy) in &AROUND {
                    let (nx, ny) = (x + dx, y + dy);
                    if nx >= 0 && nx < w && ny >= 0 && ny < h
                    && map.is_passable_terrain[ny as usize][nx as usize] {
                        adj.push((nx, ny));
                    }
                }
            }
        }
        Navigator { w, h, t, terrain, cache, enemies,
            expiration, reserved, routes, targets, unmoved, order, moves,
        }
    }

    pub fn refresh(&mut self, gc: &GameController) {
        let enemy = gc.team().other();
        let origin = MapLocation::new(gc.planet(), 0, 0);
        self.t += 1;
        self.enemies = gc.sense_nearby_units_by_team(origin, 2500, enemy)
            .into_iter()
            .filter_map(|enemy| enemy.location().map_location().ok())
            .map(|enemy| (enemy.x as Distance, enemy.y as Distance))
            .collect::<FnvHashSet<_>>();

        for (id, expiration) in self.expiration.iter_mut() {
            *expiration -= 1;
            if *expiration == 0 {
                self.order.retain(|&other| other != *id);
                self.targets.remove(id).unwrap();
                for (x, y, t, _) in self.routes.remove(id).unwrap() {
                    self.reserved.remove(&(x, y, t));
                }
            }
        }
        self.expiration.retain(|_, &mut expiration| expiration != 0);
        self.moves.clear();
        self.unmoved.clear();
        for unit in gc.my_units() {
            if !self.targets.contains_key(&unit.id()) {
                if let Ok(p) = unit.location().map_location() {
                    self.unmoved.insert((p.x as Distance, p.y as Distance));
                }
            }
        }
    }

    pub fn moves_between(&mut self, start: &MapLocation, end: &MapLocation) -> i32 {
        let (sx, sy) = (start.x as Distance, start.y as Distance);
        let (ex, ey) = (end.x as Distance, end.y as Distance);
        if !self.cache.contains_key(&(ex, ey)) {
            self.cache_bfs(end);
        }
        self.cache[&(ex, ey)][self.index(sx, sy)] as i32
    }

    pub fn neighbors(&self, start: &MapLocation) -> usize {
        let (sx, sy) = (start.x as Distance, start.y as Distance);
        self.terrain[self.index(sx, sy)].len()
    }

    pub fn navigate(&mut self, unit: &Unit, end: &MapLocation) {
        let id = unit.id();
        let start = unit.location().map_location().unwrap();
        let heat = unit.movement_heat().unwrap() as Heat;
        let (sx, sy) = (start.x as Distance, start.y as Distance);
        let (ex, ey) = (end.x as Distance, end.y as Distance);
        if !self.cache.contains_key(&(ex, ey)) {
            self.cache_bfs(end);
        }

        if self.moves_between(&start, &end) as usize > (self.w as usize*self.h as usize) {
            return
        }

        if let Some(&(x, y)) = self.targets.get(&id) {
            if x == ex && y == ey {
                let route = self.routes.get(&id).unwrap();
                if let Some(current) = route.iter()
                    .position(|&(x, y, t, h)| sx == x && sy == y && t == self.t && h == heat) {
                    let (x, y, _, _) = route[current - 1];
                    self.moves.insert(id, Self::to_direction(x - sx, y - sy));
                    return
                }
            }
            self.order.retain(|&other| other != id);
            self.expiration.remove(&id).unwrap();
            self.targets.remove(&id).unwrap();
            for (x, y, t, _) in self.routes.remove(&id).unwrap() {
                self.reserved.remove(&(x, y, t));
            }
        }
        self.a_star(unit, &start, end)
    }

    fn index(&self, x: Distance, y: Distance) -> usize { (y as usize * self.w as usize) + x as usize }

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

    fn sqdist((x1, y1): Point, (x2, y2): Point) -> i16 {
        let dx = (x2 as i16) - (x1 as i16);
        let dy = (y2 as i16) - (y1 as i16);
        dx*dx + dy*dy
    }

    fn cache_bfs(&mut self, end: &MapLocation) {
        let (ex, ey) = (end.x as Distance, end.y as Distance);
        let mut distances = vec![i8::max_value(); (self.w as usize * self.h as usize)];
        let mut heap = BinaryHeap::default();
        distances[self.index(ex, ey)] = 0;
        heap.push(Node { d: 0, x: ex, y: ey });

        while let Some(node) = heap.pop() {
            let node_index = self.index(node.x, node.y);
            let d = distances[node_index];
            if d < node.d { continue }

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
        let (sx, sy) = (start.x as Distance, start.y as Distance);
        let (ex, ey) = (end.x as Distance, end.y as Distance);
        let start_index = self.index(sx, sy);
        self.unmoved.remove(&(sx, sy));

        let heuristic = &self.cache[&(ex, ey)];
        let max_depth = SEARCH_DEPTH + self.t;

        let id = unit.id();
        let cd = unit.movement_cooldown().unwrap() as Heat;
        let heat = unit.movement_heat().unwrap() as Heat;

        let mut heap = BinaryHeap::default();
        let mut path = FnvHashMap::default();

        heap.push(ANode {
            a: heuristic[start_index],
            x: sx,
            y: sy,
            t: self.t,
            h: unit.movement_heat().unwrap() as Heat,
        });

        while let Some(node) = heap.pop() {

            // End search
            if node.t == max_depth {
                let mut route = Vec::new();
                let mut current = (node.x, node.y, node.t, node.h);

                while let Some(&prev) = path.get(&current) {
                    route.push(current);
                    let (x, y, t, _) = current;
                    self.reserved.insert((x, y, t));
                    if prev == (sx, sy, self.t, heat) { break } else { current = prev }
                }

                route.push((sx, sy, self.t, heat));
                self.expiration.insert(id, EXPIRE_TIME);
                self.reserved.insert((sx, sy, self.t));
                self.routes.insert(id, route);
                self.targets.insert(id, (ex, ey));
                self.order.push(id);
                self.moves.insert(id, Self::to_direction(current.0 - sx, current.1 - sy));
                return
            }

            // Staying still is always an option
            let next_cost = if Self::sqdist((node.x, node.y), (ex, ey)) <= 2 { node.a } else { node.a + 1 };
            let next_heat = if node.h < MAX_HEAT { 0 } else { node.h - MAX_HEAT };
            if !self.reserved.contains(&(node.x, node.y, node.t + 1)) {
                path.insert((node.x, node.y, node.t + 1, next_heat),
                            (node.x, node.y, node.t, node.h));
                heap.push(ANode {
                    a: next_cost,
                    x: node.x,
                    y: node.y,
                    t: node.t + 1,
                    h: next_heat,
                });
            }

            // Able to move if under max heat
            if node.h < MAX_HEAT {
                for &(x, y) in &self.terrain[self.index(node.x, node.y)] {
                    if !self.reserved.contains(&(x, y, node.t + 1))
                    && !self.enemies.contains(&(x, y))
                    && !self.unmoved.contains(&(x, y)) {
                        path.insert((x, y, node.t + 1, node.h + cd - MAX_HEAT),
                                    (node.x, node.y, node.t, node.h));
                        heap.push(ANode {
                            a: heuristic[self.index(x, y)] + (node.t - self.t + 1) as Distance,
                            x: x,
                            y: y,
                            t: node.t + 1,
                            h: node.h + cd - MAX_HEAT,
                        });
                    }
                }
            }
        }
        self.unmoved.insert((sx, sy));
        return
    }

    pub fn execute(&mut self, gc: &mut GameController) {
        let mut failed = FnvHashSet::default();
        for id in &self.order {
            if let Some(&Some(direction)) = self.moves.get(&id) {
                if gc.move_robot(*id, direction).is_err() {
                    failed.insert(*id);
                    self.expiration.remove(&id).unwrap();
                    self.targets.remove(&id).unwrap();
                    for (x, y, t, _) in self.routes.remove(&id).unwrap() {
                        self.reserved.remove(&(x, y, t));
                    }
                }
            }
        }
        for id in failed {
            self.order.retain(|&other| other != id);
        }
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
            .then_with(|| self.h.cmp(&other.h))
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
