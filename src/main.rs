extern crate battlecode_engine as engine;
extern crate battlecode as bc;
extern crate fnv;
extern crate rand;

use engine::controller::*;
use engine::world::*;
use engine::unit::*;
use engine::map::*;
use engine::location::*;

use Location::*;
use Team::*;
use Direction::*;
use UnitType::*;

use bc::navigate::*;
use bc::karbonite::*;

use fnv::FnvHashMap;
use fnv::FnvHashSet;

use rand::distributions::{IndependentSample, Range};

const DIRECTIONS: [Direction;9] = [Center,North,Northeast,East,Southeast,South,Southwest,West,Northwest];

fn loc(unit: &Unit) -> MapLocation {
    unit.location().map_location().unwrap()
}

fn main() {

    let mut gc = GameController::new_player_env().unwrap();
    let mut nav = Navigator::new(&gc);

    // RESEARCH QUEUE
    gc.queue_research(Knight);
    gc.queue_research(Knight);
    gc.queue_research(Healer);
    gc.queue_research(Rocket);
    gc.queue_research(Mage);
    gc.queue_research(Knight);
    gc.queue_research(Healer);
    gc.queue_research(Healer);
    gc.queue_research(Mage);
    gc.queue_research(Mage);

    let (starting_units, starting_en_units): (Vec<_>, Vec<_>) = gc
        .starting_map(gc.planet())
        .initial_units.iter()
        .cloned()
        .partition(|unit| unit.team() == gc.team());

    let start = starting_units
        .get(0)
        .map(|unit| loc(unit));

    let mut loc_num = 0;
    let mut rally = starting_en_units
        .get(loc_num)
        .map(|unit| loc(unit));

    let mut karb_locs = FnvHashMap::default();
    let starting_map = gc.starting_map(gc.planet()).clone();

    for x in 0..starting_map.width {
        for y in 0..starting_map.height {
            let loc = MapLocation::new(gc.planet(),x as i32,y as i32);
            let karb = starting_map.initial_karbonite[y][x].clone();
            if karb > 0 {
                karb_locs.insert(loc,karb);
            }
        }
    }

    let mut prod_num = 0;
    let production_queue = [Knight, Knight, Knight, Healer,Ranger];

    let mut seen_locs = FnvHashMap::default();

    loop {
        nav.refresh(&gc);

        // Update Karb Map
        karb_locs.retain(|&loc,_| !(gc.can_sense_location(loc) && gc.karbonite_at(loc).unwrap() <= 0));
        karb_locs.iter_mut()
            .filter(|&(&loc, _)| gc.can_sense_location(loc))
            .for_each(|(&loc, karb)| *karb = gc.karbonite_at(loc).unwrap());

        if gc.planet() == Planet::Mars {
            if gc.asteroid_pattern().has_asteroid(gc.round()) {
                let asteroid_pattern = gc.asteroid_pattern();
                let asteroid = asteroid_pattern.asteroid(gc.round()).unwrap();
                karb_locs.insert(asteroid.location, asteroid.karbonite);
            }
        }

        seen_locs.iter_mut().for_each(|(_, time)| *time += 1);
        if rally != None && gc.has_unit_at_location(rally.unwrap()) && gc.sense_unit_at_location(rally.unwrap()).unwrap().team() == gc.team() && gc.sense_unit_at_location(rally.unwrap()).unwrap().unit_type() != Worker {
            loc_num = (loc_num +1)%starting_en_units.len();
            if loc_num < starting_en_units.len() {
                rally = starting_en_units.get(loc_num).map(|unit| loc(unit));
            }
        }

        for x in 0..starting_map.width {
            for y in 0..starting_map.height {
                let loc = MapLocation::new(gc.planet(),x as i32,y as i32);
                if gc.can_sense_location(loc) && starting_map.is_passable_terrain_at(loc).unwrap() {
                    seen_locs.insert(loc, 0);
                }
            }
        }

        // Collect Units
        let (fin_facts,un_facts):(Vec<_>,Vec<_>) = get_type(&gc,Factory)
            .into_iter().partition(|fact| fact.structure_is_built().unwrap());

        let (fin_rockets,un_rockets):(Vec<_>,Vec<_>) = get_type(&gc,Rocket)
            .into_iter().partition(|rocket| rocket.structure_is_built().unwrap());

        let workers = get_type(&gc, Worker);
        let knights = get_type(&gc, Knight);
        let rangers = get_type(&gc, Ranger);
        let mages = get_type(&gc, Mage);
        let healers = get_type(&gc, Healer);

        // FACTORY
        for fact in &fin_facts {
            if workers.len() == 0 {
                try_produce(&mut gc,fact,Worker);
            }
            else if !(gc.research_info().unwrap().get_level(&Rocket) > 0 && un_rockets.len() + fin_rockets.len() ==0) {
                if try_produce(&mut gc, fact, production_queue[prod_num%production_queue.len()]) {
                    prod_num = (prod_num+1)%production_queue.len();
                }
            }
            try_unload(&mut gc,fact)
        }

        // ROCKET
        for rocket in &fin_rockets {
            if !rocket.rocket_is_used().unwrap() {
                let num_loaded = try_load(&mut gc, rocket);
                if rocket.structure_garrison().unwrap().len() + num_loaded >= 8 {
                    let x_range = Range::new(0, gc.starting_map(Planet::Mars).width);
                    let y_range = Range::new(0, gc.starting_map(Planet::Mars).height);
                    let mut rng = rand::thread_rng();
                    let x = x_range.ind_sample(&mut rng);
                    let y = y_range.ind_sample(&mut rng);

                    let loc = MapLocation::new(Planet::Mars,x as i32,y as i32);
                    if gc.can_launch_rocket(rocket.id(),loc) {
                        gc.launch_rocket(rocket.id(),loc);
                    }
                }
            }
            else {
                if rocket.structure_garrison().unwrap().len() > 0 {
                    try_unload(&mut gc, rocket);
                }
            }
        }

        let workers = get_type(&gc, Worker);

        // WORKER
        for worker in &workers {
            if (fin_facts.len() + un_facts.len() != 0
            && !(gc.research_info().unwrap().get_level(&Rocket) > 0
            && un_rockets.len() + fin_rockets.len() ==0)
            && workers.len() <10) || gc.round() > 750 {
                try_replicate(&mut gc, &worker);
            }
        }

        let workers = get_type(&gc, Worker);

        for worker in &workers {
            try_build(&mut gc, worker)
            || (fin_facts.len() + un_facts.len() < 5 && try_blueprint(&mut gc, &nav, worker,Factory))
            || (gc.research_info().unwrap().get_level(&Rocket)>0 && try_blueprint(&mut gc, &nav, worker,Rocket))
            || try_harvest(&mut gc, worker);
        }
        assign_workers(&mut nav, workers, &karb_locs, &un_facts, &un_rockets, &fin_rockets);

        // KNIGHT
        for knight in &knights {
            try_attack(&mut gc, &mut nav, knight); 
            try_javelin(&mut gc, &mut nav, knight);

            let knight_loc = loc(knight);

            let mut nearby_units = gc.sense_nearby_units_by_team(knight_loc, knight.vision_range()*2, gc.team().other());
            nearby_units.sort_by_key(|en| nav.moves_between(&knight_loc, &loc(en)));
            if nearby_units.len() != 0 {
                let friends = gc.sense_nearby_units_by_team(knight_loc, 9, gc.team());
                let mut enemies = gc.sense_nearby_units_by_team(knight_loc, 50, gc.team().other());
                enemies.retain(|en| en.unit_type().is_robot() && en.unit_type() != Worker && en.unit_type() != Healer);
                enemies.retain(|en| loc(en).distance_squared_to(knight_loc) <= en.attack_range().unwrap());
                if friends.len() >= enemies.len() || (enemies.len() != 0 && nav.moves_between(&knight_loc, &loc(&nearby_units[0])) <= 3) {
                    try_move_to(&mut nav, knight, &loc(&nearby_units[0]));
                }
                else {
                    let my_loc = knight_loc;
                    let en_loc = loc(&nearby_units[0]);
                    if start != None {
                        try_move_to(&mut nav, knight, &start.unwrap());
                    }
                }
            } else if fin_rockets.len() != 0 {
                try_move_to(&mut nav, knight, &loc(&fin_rockets[0]));
            } else if rally != None {
                try_move_to(&mut nav, knight, &rally.unwrap());
            }
        }

        // RANGER
        for ranger in &rangers {
            try_attack(&mut gc, &mut nav, ranger);

            let ranger_loc = loc(ranger);

            let mut nearby_units = gc.sense_nearby_units_by_team(ranger_loc, 50, gc.team().other());
            nearby_units.sort_by_key(|en| nav.moves_between(&ranger_loc, &loc(en)));
            if nearby_units.len() != 0 {
                let friends = gc.sense_nearby_units_by_team(ranger_loc, 25, gc.team());
                let mut enemies = gc.sense_nearby_units_by_team(ranger_loc, 50, gc.team().other());
                enemies.retain(|en| en.unit_type().is_robot() && en.unit_type() != Worker && en.unit_type() != Healer);
                enemies.retain(|en| loc(en).distance_squared_to(ranger_loc) < en.attack_range().unwrap());
                if friends.len() < enemies.len() || enemies.len() != 0 && nav.moves_between(&ranger_loc, &loc(&enemies[0])) <= 5 {
                    let my_loc = ranger_loc;
                    let en_loc = loc(&nearby_units[0]);
                    if start != None {
                        try_move_to(&mut nav, ranger, &start.unwrap());
                    }
                }
            }
            else if fin_rockets.len() != 0 {
                try_move_to(&mut nav, ranger, &loc(&fin_rockets[0]));
            }
            else if rally != None && try_move_to(&mut nav, ranger, &rally.unwrap()) {

            }
        }

        let mut overcharged_units = Vec::new();

        // Healer
        for healer in &healers {

            try_heal(&mut gc, &mut nav, healer);
            let healer_loc = loc(healer);

            if let Some(overcharged) = try_overcharge(&mut gc, &mut nav, healer) {
                overcharged_units.push(overcharged);
            }

            let mut nearby_units = gc.sense_nearby_units_by_team(healer_loc, 50, gc.team().other());
            nearby_units.sort_by_key(|en| nav.moves_between(&healer_loc, &loc(en)));
            if nearby_units.len() != 0 {
                let friends = gc.sense_nearby_units_by_team(healer_loc, 25, gc.team());
                let mut enemies = gc.sense_nearby_units_by_team((healer_loc), 50, gc.team().other());
                enemies.retain(|en| en.unit_type().is_robot() && en.unit_type() != Worker && en.unit_type() != Healer);
                enemies.retain(|en| loc(en).distance_squared_to(healer_loc) < en.attack_range().unwrap());
                if friends.len() < enemies.len() || enemies.len() != 0 && nav.moves_between(&healer_loc, &loc(&enemies[0])) <= 5 {
                    let my_loc = healer_loc;
                    let en_loc = loc(&nearby_units[0]);
                    if start != None {
                        try_move_to(&mut nav, healer, &start.unwrap());
                    }
                }
            }
            else if fin_rockets.len() != 0 {
                try_move_to(&mut nav, healer, &loc(&fin_rockets[0]));
            }
            else if rally != None {
                try_move_to(&mut nav, healer, &rally.unwrap());
            }
        }

        for unit_id in overcharged_units {
            let unit = gc.unit(unit_id).unwrap();
            try_attack(&mut gc, &mut nav, &unit) || try_javelin(&mut gc, &mut nav, &unit);
        }

        nav.execute(&mut gc);

        for knight in &knights { try_attack(&mut gc, &mut nav, knight); try_javelin(&mut gc, &mut nav, knight); }
        for ranger in &rangers { try_attack(&mut gc, &mut nav, ranger); }
        for healer in &healers { try_heal(&mut gc, &mut nav, healer); }

        gc.next_turn();
    }
}

fn get_type(gc: &GameController, unit_type: UnitType) -> Vec<Unit> {
    gc.my_units().into_iter()
        .filter(|unit| unit.unit_type() == unit_type)
        .filter(|unit| unit.location().is_on_map())
        .collect::<Vec<_>>()
}

// WORKER METHODS
fn try_harvest(gc: &mut GameController, unit: &Unit) -> bool {
    for &d in &DIRECTIONS {
        if gc.can_harvest(unit.id(),d) {
            gc.harvest(unit.id(),d);
            return true
        }
    }
    return false
}

fn try_replicate(gc: &mut GameController, unit: &Unit) -> bool {
    for d in Direction::all() {
        if gc.can_replicate(unit.id(),d) {
            gc.replicate(unit.id(),d);
            return true
        }
    }
    return false
}

fn try_blueprint(gc: &mut GameController, nav: &Navigator, unit: &Unit, building_type: UnitType) -> bool {
    let location = loc(unit);
    for d in Direction::all() {
        if gc.can_blueprint(unit.id(),building_type,d)
        && nav.neighbors(&location.add(d)) > 4 {
            gc.blueprint(unit.id(),building_type,d);
            return true
        }
    }
    return false
}

fn try_build(gc: &mut GameController, unit: &Unit) -> bool {
    let units = gc.sense_nearby_units(loc(unit),2);
    for building in units {
        if gc.can_build(unit.id(),building.id()) {
            gc.build(unit.id(),building.id());
            return true
        }
    }
    return false
}

fn try_move_to(nav: &mut Navigator, unit: &Unit, loc: &MapLocation) -> bool {
    nav.navigate(unit, loc);
    return true
}

// FACTORY METHODS
fn try_produce(gc: &mut GameController, fact: &Unit, unit_type: UnitType) -> bool {
    if gc.can_produce_robot(fact.id(),unit_type) {
        gc.produce_robot(fact.id(),unit_type);
        return true
    }
    return false
}

fn try_unload(gc: &mut GameController, building: &Unit) {
    let mut num_units = building.structure_garrison().unwrap().len();
    for d in Direction::all() {
        if num_units > 0 && gc.can_unload(building.id(),d) {
            gc.unload(building.id(),d);
            num_units -= 1;
        }
    }
}

// ROCKET METHODS
fn try_load(gc: &mut GameController, rocket: &Unit) -> usize {
    let mut num_loaded = 0;
    for unit in gc.sense_nearby_units_by_team(loc(rocket), 2, gc.team()) {
        if rocket.structure_garrison().unwrap().len() < 8 && gc.can_load(rocket.id(),unit.id()) {
            gc.load(rocket.id(),unit.id());
            num_loaded += 1;
        }
    }
    return num_loaded
}

// ARMY METHODS
fn try_attack(gc: &mut GameController, nav: &mut Navigator, unit: &Unit) -> bool {
    let mut en_units = gc.sense_nearby_units_by_team(loc(unit) ,unit.attack_range().unwrap(),unit.team().other());
    en_units.sort_by_key(|en| en.health());
    if gc.is_attack_ready(unit.id()) {
        for enemy in en_units {
            if gc.can_attack(unit.id(),enemy.id()) {
                gc.attack(unit.id(),enemy.id());
                return true
            }
        }
    }
    return false
}

fn try_heal(gc: &mut GameController, nav: &mut Navigator, healer: &Unit) -> bool {
    let mut units = gc.sense_nearby_units_by_team(loc(healer), healer.attack_range().unwrap(),healer.team());
    units.sort_by_key(|en| -nav.moves_between(&loc(healer),&loc(en)));
    if gc.is_heal_ready(healer.id()) {
        for friend in units {
            if friend.health() != friend.max_health() && gc.can_heal(healer.id(),friend.id()) {
                gc.heal(healer.id(),friend.id());
                return true;
            }
        }
    }
    return false
}

fn try_overcharge(gc: &mut GameController, nav: &mut Navigator, healer: &Unit) -> Option<UnitID> {
    let mut units = gc.sense_nearby_units_by_team(loc(healer), healer.ability_range().unwrap(),healer.team());
    units.sort_by_key(|en| -nav.moves_between(&loc(healer),&loc(en)));
    if gc.is_overcharge_ready(healer.id()) {
        for friend in units {
            if !friend.unit_type().is_robot() || friend.unit_type() == Worker {
                continue
            }
            if friend.ability_heat().unwrap() >=10 && gc.can_overcharge(healer.id(),friend.id()) {
                gc.overcharge(healer.id(),friend.id());
                return Some(friend.id())
            }
        }
    }
    return None

}

fn try_javelin(gc: &mut GameController, nav: &mut Navigator, knight: &Unit) -> bool {
    let mut en_units = gc.sense_nearby_units_by_team(loc(knight),knight.ability_range().unwrap(),knight.team().other());
    en_units.sort_by_key(|en| nav.moves_between(&loc(knight),&loc(en)));
    if gc.is_javelin_ready(knight.id())  {
        for enemy in en_units {
            if gc.can_javelin(knight.id(),enemy.id()) {
                gc.javelin(knight.id(),enemy.id());
                return true
            }
        }
    }
    return false
}
