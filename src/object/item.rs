use rustc_serialize;

use tcod::input::{self, Event, KeyCode};

use ai::Ai;
use consts;
use game::Game;
use log::MessageLog;
use map;
use object::Object;
use ui::{render_all, Ui};

pub enum UseResult {
    UsedUp,
    Cancelled,
}

#[derive(Clone, Copy, Debug, PartialEq, RustcEncodable, RustcDecodable)]
pub enum Function {
    Confuse,
    Fireball,
    Heal,
    Lightning,
}

pub fn heal_player(_game_ui: &mut Ui, game: &mut Game,
                   actors: &mut [Object]) -> UseResult {
    if let Some(fighter) = actors[consts::PLAYER].fighter {
        if fighter.hp == fighter.max_hp {
            game.log.alert( "You are already at full health.");
            return UseResult::Cancelled;
        }
        game.log.success( "Your wounds start to feel better!");
        actors[consts::PLAYER].heal(3);
        return UseResult::UsedUp;
    }
    UseResult::Cancelled
}

pub fn cast_confuse(game_ui: &mut Ui, game: &mut Game,
                    actors: &mut [Object]) -> UseResult {
    game.log.info( "Left-click an enemy to confuse it, or right-click \
                   to cancel.");
    let monster_id = target_monster(game_ui, game, actors, Some(5.0));
    if let Some(monster_id) = monster_id {
        let old_ai = actors[monster_id].ai.take().unwrap_or(Ai::Basic);
        actors[monster_id].ai = Some(Ai::Confused {
            previous_ai: Box::new(old_ai),
            num_turns: 3,
        });
        game.log.info(
            format!("The eyes of the {} look vacant and it starts to \
                     stumble around!", actors[monster_id].name));
        UseResult::UsedUp
    } else {
        game.log.alert( "No enemy is within range.");
        UseResult::Cancelled
    }
}

pub fn cast_fireball(game_ui: &mut Ui, game: &mut Game,
                     actors: &mut [Object]) -> UseResult {
    game.log.info( "Left-click a target tile for the molotov, \
                   or right-click to cancel.");
    let (x, y) = match target_tile(game_ui, game, actors, None) {
        Some(tile_pos) => tile_pos,
        None => return UseResult::Cancelled,
    };

    game.log.success(
        format!("The molotov explodes, burning everything within a {} \
                 radius!", 5));

    for obj in actors {
        if obj.distance(x, y) <= 5.0 && obj.fighter.is_some() {
            game.log.success(
                format!("The {} gets burned for {} hit points.",
                        obj.name, 10));
            obj.take_damage(10, &mut game.log);
        }
    }
    UseResult::UsedUp
}


pub fn cast_lightning(game_ui: &mut Ui, game: &mut Game,
                      actors: &mut [Object]) -> UseResult {
    let monster_id = closest_monster(10, actors, game_ui);
    if let Some(monster_id) = monster_id {
        game.log.success(
            format!("A lightning bolt strikes the {} with loud thunder! \
                     The damage is {} hit points.",
                    actors[monster_id].name, 10));
        actors[monster_id].take_damage(10, &mut game.log);
        UseResult::UsedUp
    } else {
        game.log.alert( "No enemy is within range.");
        UseResult::Cancelled
    }
}

fn target_tile(game_ui: &mut Ui, game: &mut Game, actors: &[Object],
               max_range: Option<f32>)
               -> Option<(i32, i32)> {
    loop {
        game_ui.root.flush();
        let event = input::check_for_event(input::KEY_PRESS |
                                           input::MOUSE)
            .map(|e| e.1);
        let mut key = None;
        match event {
            Some(Event::Mouse(m)) => game_ui.mouse = m,
            Some(Event::Key(k)) => key = Some(k),
            None => {}
        }
        render_all(game_ui, game, actors, false);

        let (x, y) = (game_ui.mouse.cx as i32, game_ui.mouse.cy as i32);
        let in_fov = (x < map::FLOOR_WIDTH) && (y < map::FLOOR_HEIGHT) &&
            game_ui.fov.is_in_fov(x, y);
        let in_range = max_range.map_or(true,
                                        |range| actors[consts::PLAYER]
                                        .distance(x, y) <= range);
        if game_ui.mouse.lbutton_pressed && in_fov && in_range {
            return Some((x, y))
        }

        let escape = key.map_or(false, |k| k.code == KeyCode::Escape);
        if game_ui.mouse.rbutton_pressed || escape {
            return None
        }
    }
}

fn target_monster(game_ui: &mut Ui, game: &mut Game, actors: &[Object],
                  max_range: Option<f32>) -> Option<usize> {
    loop {
        match target_tile(game_ui, game, actors, max_range) {
            Some((x, y)) => {
                for(id, obj) in actors.iter().enumerate() {
                    if obj.pos() == (x, y) && obj.fighter.is_some() &&
                        id != consts::PLAYER {
                            return Some(id)
                        }
                }
            }
            None => return None,
        }
    }
}

fn closest_monster(max_range: i32, actors: &mut [Object], game_ui: &Ui)
                   -> Option<usize> {
    let mut closest_enemy = None;
    let mut closest_dist = (max_range + 1) as f32;

    for (id, object) in actors.iter().enumerate() {
        if (id != consts::PLAYER) &&
            object.fighter.is_some() &&
            object.ai.is_some() &&
            game_ui.fov.is_in_fov(object.x, object.y) {
                let dist = actors[consts::PLAYER].distance_to(object);
                if dist < closest_dist {
                    closest_enemy = Some(id);
                    closest_dist = dist;
                }
            }
    }
    closest_enemy
}
