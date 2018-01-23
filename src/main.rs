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

    loop {
        println!("{}", gc.get_time_left_ms());
        gc.next_turn(); 
    }
}
