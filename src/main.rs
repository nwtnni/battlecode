extern crate battlecode_engine as engine;
extern crate battlecode as bc;

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

fn main() {

    let mut gc = GameController::new_player_env().unwrap();
    let mut navigator = Navigator::new(gc.starting_map(gc.planet()));
    let end = MapLocation::new(gc.planet(), 0, 0);

    loop {
        for unit in &gc.my_units() {
            let start = unit.location().map_location().unwrap();
            let id = unit.id();
            if let Some(dir) = navigator.navigate(&gc, &start, &end) {
                if gc.can_move(id, dir) && gc.is_move_ready(id) {
                    gc.move_robot(id, dir);
                }
            }
        }

        println!("{}", gc.get_time_left_ms());
        gc.next_turn();
    }
}
