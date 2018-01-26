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

use fnv::FnvHashMap;
use fnv::FnvHashSet;

use rand::distributions::{IndependentSample, Range};

const DIRECTIONS: [Direction;9] = [Center,North,Northeast,East,Southeast,South,Southwest,West,Northwest];

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
    let starting_units = gc.starting_map(gc.planet()).initial_units.iter().filter(|unit| unit.team()== gc.team()).cloned().collect::<Vec<_>>();
    let start = starting_units.get(0).map(|unit| {
        unit.location().map_location().unwrap()
    });
    let starting_en_units = gc.starting_map(gc.planet()).initial_units.iter().filter(|unit| unit.team()== gc.team().other()).cloned().collect::<Vec<_>>();
    let rally = starting_en_units.get(0).map(|unit| {
        unit.location().map_location().unwrap()
    });

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
    let production_queue = [Knight,Knight, Knight, Knight,Healer];

    loop {
        println!("{}", gc.get_time_left_ms());
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
                try_produce(&mut gc, fact, production_queue[prod_num%production_queue.len()]);
                prod_num = (prod_num+1)%production_queue.len();
            }
            try_unload(&mut gc,fact)
        }

        // ROCKET
        for rocket in &fin_rockets {
            if !rocket.rocket_is_used().unwrap() {
                let num_loaded = try_load(&mut gc, rocket);
                if rocket.structure_garrison().unwrap().len() + num_loaded >= 8 {
                    let xRange = Range::new(0, gc.starting_map(Planet::Mars).width);
                    let yRange = Range::new(0, gc.starting_map(Planet::Mars).height);
                    let mut rng = rand::thread_rng();
                    let x = xRange.ind_sample(&mut rng);
                    let y = yRange.ind_sample(&mut rng);

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
            if !worker.location().is_on_map() {
                continue
            }

            if fin_facts.len() + un_facts.len() != 0 && !(gc.research_info().unwrap().get_level(&Rocket) > 0 && un_rockets.len() + fin_rockets.len() ==0) && workers.len() <10 {
                try_replicate(&mut gc, &worker);
            }
        }


        let workers = get_type(&gc, Worker);
        for worker in &workers {
            if !worker.location().is_on_map() {
                continue
            }


            if try_build(&mut gc, worker) {

            }
            else if fin_facts.len() + un_facts.len() < 4 && try_blueprint(&mut gc,worker,Factory) {

            }
            else if gc.research_info().unwrap().get_level(&Rocket)>0 && try_blueprint(&mut gc,worker,Rocket){

            }
            else if try_harvest(&mut gc, worker) {

            }

            if un_facts.len() > 0 {
                try_move_to(&mut gc, &mut nav, worker, &un_facts[0].location().map_location().unwrap());
            }
            else if un_rockets.len() > 0{
                try_move_to(&mut gc, &mut nav, worker, &un_rockets[0].location().map_location().unwrap());
            }
            else if fin_rockets.len() > 0 && gc.planet() != Planet::Mars {
                try_move_to(&mut gc, &mut nav, worker, &fin_rockets[0].location().map_location().unwrap());
            }
            else if karb_locs.keys().len() >0 {
                let mut locs = karb_locs.keys().collect::<Vec<_>>();
                locs.sort_by_key(|loc| nav.moves_between(&worker.location().map_location().unwrap(),loc));
                try_move_to(&mut gc, &mut nav, worker, locs[0]);
            }
        }
        // KNIGHT
        for knight in &knights {
            if !knight.location().is_on_map() {
                continue
            }

            if try_attack(&mut gc, &mut nav, knight) {

            }
            if try_javelin(&mut gc, &mut nav, knight) {

            }

            let mut nearby_units = gc.sense_nearby_units_by_team(knight.location().map_location().unwrap(), knight.vision_range(), gc.team().other());
            nearby_units.sort_by_key(|en| nav.moves_between(&knight.location().map_location().unwrap(), &en.location().map_location().unwrap()));
            if nearby_units.len() != 0 {
                let friends = gc.sense_nearby_units_by_team(knight.location().map_location().unwrap(), 9, gc.team());
                let mut enemies = gc.sense_nearby_units_by_team(knight.location().map_location().unwrap(),50, gc.team().other());
                enemies.retain(|en| en.unit_type().is_robot() && en.unit_type() != Worker);
                if friends.len() >= enemies.len() {
                    try_move_to(&mut gc, &mut nav, knight, &nearby_units[0].location().map_location().unwrap());
                }
                else {
                    let my_loc = knight.location().map_location().unwrap();
                    let en_loc = nearby_units[0].location().map_location().unwrap();
                    if start != None {
                        try_move_to(&mut gc, &mut nav, knight, &start.unwrap());
                    }
                }
            }
            if fin_rockets.len() != 0 {
                try_move_to(&mut gc, &mut nav, knight, &fin_rockets[0].location().map_location().unwrap());
            }
            else if rally != None {
                try_move_to(&mut gc, &mut nav, knight, &rally.unwrap());
            }
        }

        // RANGER
        for ranger in &rangers {
            if !ranger.location().is_on_map() {
                continue
            }

            if try_attack(&mut gc, &mut nav, ranger) {
            }
            if fin_rockets.len() != 0 {
                try_move_to(&mut gc, &mut nav, ranger, &fin_rockets[0].location().map_location().unwrap());
            }
            else if rally != None && try_move_to(&mut gc, &mut nav, ranger, &rally.unwrap()) {

            }
        }

        let mut overcharged_units = Vec::new();

        // Healer
        for healer in &healers {
            if !healer.location().is_on_map() {
                continue
            }

            if try_heal(&mut gc, &mut nav, healer) {

            }

            let overcharged = try_overcharge(&mut gc, &mut nav, healer);
            if overcharged != None {
                overcharged_units.push(overcharged.unwrap());
            }

            else if fin_rockets.len() != 0 {
                try_move_to(&mut gc, &mut nav, healer, &fin_rockets[0].location().map_location().unwrap());
            }
            else if rally != None {
                try_move_to(&mut gc, &mut nav, healer, &rally.unwrap());
            }
        }

        if overcharged_units.len() > 0 {
            println!("OVERCHARGING!: {:?}",overcharged_units);
        }
        for unit_id in overcharged_units {
            let unit = gc.unit(unit_id).unwrap();
            if try_attack(&mut gc, &mut nav, &unit) {

            }
            if try_javelin(&mut gc, &mut nav, &unit) {

            }
        }

        gc.next_turn();
    }
}

fn get_type(gc: &GameController, unit_type: UnitType) -> Vec<Unit> {
    gc.my_units().into_iter()
        .filter(|unit| unit.unit_type() == unit_type)
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

fn try_blueprint(gc: &mut GameController, unit: &Unit, building_type: UnitType) -> bool{
    for d in Direction::all() {
        if gc.can_blueprint(unit.id(),building_type,d) {
            gc.blueprint(unit.id(),building_type,d);
            return true
        }
    }
    return false
}

fn try_build(gc: &mut GameController, unit: &Unit) -> bool {
    let units = gc.sense_nearby_units(unit.location().map_location().unwrap(),2);
    for building in units {
        if gc.can_build(unit.id(),building.id()) {
            gc.build(unit.id(),building.id());
            return true
        }
    }
    return false
}

fn try_move_to(gc: &mut GameController, nav: &mut Navigator, unit: &Unit, loc: &MapLocation) -> bool {
    if let Some(dir) = nav.smart(&unit.location().map_location().unwrap(),loc) {
        if gc.is_move_ready(unit.id()) && gc.can_move(unit.id(),dir) {
            gc.move_robot(unit.id(), dir);
        }
        return true
    }
    return false
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
    for unit in gc.sense_nearby_units_by_team(rocket.location().map_location().unwrap(),2,gc.team()) {
        if rocket.structure_garrison().unwrap().len() < 8 && gc.can_load(rocket.id(),unit.id()) {
            gc.load(rocket.id(),unit.id());
            num_loaded += 1;
        }
    }
    return num_loaded
}

// ARMY METHODS
fn try_attack(gc: &mut GameController, nav: &mut Navigator, unit: &Unit) -> bool {
    let mut en_units = gc.sense_nearby_units_by_team(unit.location().map_location().unwrap(),unit.attack_range().unwrap(),unit.team().other());
    en_units.sort_by_key(|en| -nav.moves_between(&unit.location().map_location().unwrap(),&en.location().map_location().unwrap()));
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
    let mut units = gc.sense_nearby_units_by_team(healer.location().map_location().unwrap(), healer.attack_range().unwrap(),healer.team());
    units.sort_by_key(|en| -nav.moves_between(&healer.location().map_location().unwrap(),&en.location().map_location().unwrap()));
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
    let mut units = gc.sense_nearby_units_by_team(healer.location().map_location().unwrap(), healer.ability_range().unwrap(),healer.team());
    units.sort_by_key(|en| -nav.moves_between(&healer.location().map_location().unwrap(),&en.location().map_location().unwrap()));
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
    let mut en_units = gc.sense_nearby_units_by_team(knight.location().map_location().unwrap(),knight.ability_range().unwrap(),knight.team().other());
    en_units.sort_by_key(|en| nav.moves_between(&knight.location().map_location().unwrap(),&en.location().map_location().unwrap()));
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
