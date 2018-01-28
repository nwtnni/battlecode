use engine::controller::*;
use engine::location::*;
use engine::unit::*;

const KNIGHT_RANGE: i16 = 2;
const RANGER_RANGE: i16 = 50;
const RANGER_BLIND: i16 = 10;
const MAGE_RANGE: i16 = 30;
const HEALER_RANGE: i16 = 30;

pub fn influence(gc: &GameController) -> Vec<(i32, i32, i16)> {
    let map = gc.starting_map(gc.planet());
    let w = map.width as i32;
    let h = map.height as i32;
    let team = gc.team();
    let mut influence = Vec::new();

    for y in 0..h {
        for x in 0..w {
            if map.is_passable_terrain[y as usize][x as usize]
            && gc.can_sense_location(MapLocation::new(gc.planet(), x, y)) {
                influence.push((x, y, 0 as i16));
            }
        }
    }

    for unit in gc.units().iter().filter(|unit| unit.location().is_on_map()) {
        let ally = unit.team() == team;
        let location = unit.location().map_location().unwrap();
        let (x, y) = (location.x as i32, location.y as i32);
        match unit.unit_type() {
            UnitType::Knight => knight(&mut influence, (x, y), ally),
            UnitType::Ranger => ranger(&mut influence, (x, y), ally),
            UnitType::Healer => healer(&mut influence, (x, y), ally),
            UnitType::Mage => mage(&mut influence, (x, y), ally),
            _ => (),
        }
    }

    influence.retain(|&(_, _, i)| i >= 0);
    influence
}

fn distance((x1, y1): (i32, i32), (x2, y2): (i32, i32)) -> i16 {
    ((x2 - x1)*(x2 - x1) + (y2 - y1)*(y2 - y1)) as i16
}

fn ranger(influence: &mut Vec<(i32, i32, i16)>, (x1, y1): (i32, i32), ally: bool) {
    for &mut (x2, y2, ref mut i) in influence {
        let d = distance((x1, y1), (x2, y2));
        if d <= RANGER_BLIND {
            *i += if ally { d } else { -d };
        } else if d <= RANGER_RANGE {
            *i -= if ally { -d } else { d };
        }
    }
}

fn knight(influence: &mut Vec<(i32, i32, i16)>, (x1, y1): (i32, i32), ally: bool) {
    for &mut (x2, y2, ref mut i) in influence {
        if distance((x1, y1), (x2, y2)) <= KNIGHT_RANGE {
            *i += if ally { 2 } else { -2 };
        }
    }
}

fn mage(influence: &mut Vec<(i32, i32, i16)>, (x1, y1): (i32, i32), ally: bool) {
    for &mut (x2, y2, ref mut i) in influence {
        if distance((x1, y1), (x2, y2)) <= MAGE_RANGE {
            *i += if ally { 1 } else { -1 };
        }
    }
}

fn healer(influence: &mut Vec<(i32, i32, i16)>, (x1, y1): (i32, i32), ally: bool) {
    for &mut (x2, y2, ref mut i) in influence {
        if distance((x1, y1), (x2, y2)) <= HEALER_RANGE {
            *i += if ally { 1 } else { -1 };
        }
    }
}
