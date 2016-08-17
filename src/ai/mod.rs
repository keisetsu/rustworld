use rustc_serialize;

use rand::{self, Rng};

use tcod::map::Map as FovMap;

use log::MessageLog;
use map::Map;
use object::Object;
use object::actor;

use consts;
use utils;

use game::Game;

#[derive(Clone, Debug, RustcEncodable, RustcDecodable)]
pub enum Ai {
    Basic,
    Chrysalis,
    Confused{previous_ai: Box<Ai>, num_turns: i32},
}

pub fn ai_take_turn(monster_id: usize, game: &mut Game, actors: &mut [Object],
                fov_map: &FovMap) {
    if let Some(ai) = actors[monster_id].ai.take() {
        let new_ai = match ai {
            Ai::Basic => ai_basic(monster_id, game, actors, fov_map),
            Ai::Chrysalis => ai_chrysalis(monster_id, game, actors, fov_map),
            Ai::Confused{previous_ai, num_turns} => ai_confused(
                monster_id, game, actors, previous_ai, num_turns)
        };
        actors[monster_id].ai = Some(new_ai);
    }
}

fn ai_move_randomly(monster_id: usize, map: &Map, actors: &mut[Object]) {
    actor::move_by(monster_id,
            rand::thread_rng().gen_range(-1, 2),
            rand::thread_rng().gen_range(-1, 2),
            map, actors);
}

fn ai_basic(monster_id: usize, game: &mut Game, actors: &mut [Object],
            fov_map: &FovMap) -> Ai {
    let (monster_x, monster_y) = actors[monster_id].pos();
    if fov_map.is_in_fov(monster_x, monster_y) {
        if actors[monster_id].distance_to(&actors[consts::PLAYER]) >= 2.0 {
            let (player_x, player_y) = actors[consts::PLAYER].pos();
            actor::move_towards(monster_id, player_x, player_y, &mut game.map, actors);
        } else if actors[consts::PLAYER].fighter.map_or(false, |f| f.hp > 0) {
            let (monster, player) = utils::mut_two(monster_id, consts::PLAYER, actors);
            monster.attack(player, &mut game.log);
        }
    } else {
        ai_move_randomly(monster_id, &game.map, actors);
    }
    Ai::Basic
}

fn ai_chrysalis(monster_id: usize, game: &mut Game, actors: &mut [Object],
                fov_map: &FovMap) -> Ai {
    let (monster_x, monster_y) = actors[monster_id].pos();
    if fov_map.is_in_fov(monster_x, monster_y) {
        if actors[monster_id].distance_to(&actors[consts::PLAYER]) >= 2.0 {
            let (player_x, player_y) = actors[consts::PLAYER].pos();
            actor::move_towards(monster_id, player_x, player_y, &mut game.map, actors);
        } else if actors[consts::PLAYER].fighter.map_or(false, |f| f.hp > 0) {
            let (monster, player) = utils::mut_two(monster_id, consts::PLAYER, actors);
            monster.attack(player, &mut game.log);
        }
    }
    Ai::Chrysalis
}

fn ai_confused(monster_id: usize, game: &mut Game, actors: &mut [Object],
               previous_ai: Box<Ai>, num_turns: i32)
               -> Ai {

    if num_turns >= 0 {
        game.log.status_change(format!("The {} is confused!",
                             actors[monster_id].name));
        ai_move_randomly(monster_id, &game.map, actors);
        Ai::Confused{previous_ai: previous_ai, num_turns: num_turns - 1}
    } else {
        game.log.status_change(format!("The {} is no longer confused!",
                                  actors[monster_id].name));
        *previous_ai
    }
}
