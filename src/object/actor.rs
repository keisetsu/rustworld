use tcod::colors;

use consts;
use game::Game;
use log;
use log::MessageLog;
use object::{self, Object};
use object::item::{self, Item};
use map::{self, Map};
use ui::Ui;
use utils;


#[derive(Clone, Copy, Debug, PartialEq, RustcEncodable, RustcDecodable)]
pub enum DeathCallback {
    Player,
    Monster,
}

impl DeathCallback {
    pub fn callback(self, object: &mut Object, messages: &mut log::Messages) {
        let callback: fn(&mut Object, &mut log::Messages) = match self {
            DeathCallback::Player => player_death,
            DeathCallback::Monster => monster_death,
        };
        callback(object, messages);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, RustcEncodable, RustcDecodable)]
pub struct Fighter {
    pub max_hp: i32,
    pub hp: i32,
    pub defense: i32,
    pub power: i32,
    pub on_death: DeathCallback,
}

pub fn move_by(id: usize, dx: i32, dy: i32, map: &Map, objects: &mut[Object]) {
    let (x, y) = objects[id].pos();
    if map::is_blocked(x + dx, y + dy, map, objects) == object::Blocks::No {
        objects[id].set_pos(x + dx, y + dy);
    }
}

pub fn move_towards(id: usize, target_x: i32, target_y: i32, map: &Map,
                objects: &mut [Object]) {
    let dx = target_x - objects[id].x;
    let dy = target_y - objects[id].y;
    let distance = ((dx.pow(2) + dy.pow(2)) as f32).sqrt();

    let dx = (dx as f32 / distance).round() as i32;
    let dy = (dy as f32 / distance).round() as i32;

    move_by(id, dx, dy, map, objects);
}

pub fn player_move_or_attack(dx: i32, dy: i32, game: &mut Game,
                         objects: &mut [Object]) {
    let x = objects[consts::PLAYER].x + dx;
    let y = objects[consts::PLAYER].y + dy;

    let target_id = objects.iter().position(|object| {
        object.fighter.is_some() && object.pos() == (x, y)
    });

    match target_id {
        Some(target_id) => {
            let (player, target) =
                utils::mut_two(consts::PLAYER, target_id, objects);
            player.attack(target, &mut game.log);
        }
        None => {
            move_by(consts::PLAYER, dx, dy, &mut game.map, objects);
        }
    }
}

fn player_death(player: &mut Object, log: &mut log::Messages) {
    log.alert("You died!");
    player.symbol = '%';
    player.color = colors::DARK_RED;
}

fn monster_death(monster: &mut Object, log: &mut log::Messages) {
    log.status_change(format!("{} is dead!", monster.name));
    monster.symbol = '%';
    monster.color = colors::DARK_RED;
    monster.blocks = object::Blocks::No;
    monster.blocks_view = object::Blocks::No;
    monster.fighter = None;
    monster.ai = None;
    monster.name = format!("remains of {}", monster.name);
}

pub fn drop_item(inventory_id: usize, game: &mut Game,
             objects: &mut Vec<Object>) {
    let mut item = game.inventory.remove(inventory_id);
    item.set_pos(objects[consts::PLAYER].x, objects[consts::PLAYER].y);
    game.log.info(format!("You dropped a {}.", item.name));
    objects.push(item);
}

pub fn pick_item_up(object_id: usize, game: &mut Game,
                objects: &mut Vec<Object>) {
    if game.inventory.len() as i32 >= consts::MAX_INVENTORY_ITEMS {
        game.log.alert(
                format!("Your inventory is full, cannot pickup {}.",
                        objects[object_id].name));
    } else {
        let item = objects.swap_remove(object_id);
        game.log.success( format!("You picked up a {}!", item.name));
        game.inventory.push(item);
    }
}

pub fn use_item(game_ui: &mut Ui, game: &mut Game, inventory_id: usize,
            objects: &mut [Object]) {
    if let Some(item) = game.inventory[inventory_id].item {
        let on_use:
        fn(&mut Ui, &mut Game, &mut [Object])
           -> item::UseResult = match item {
            Item::Confuse => item::cast_confuse,
            Item::Fireball => item::cast_fireball,
            Item::Heal => item::heal_player,
            Item::Lightning => item::cast_lightning,
        };
        match on_use(game_ui, game, objects) {
            item::UseResult::UsedUp => {
                game.inventory.remove(inventory_id);
            }
            item::UseResult::Cancelled => {
                game.log.info( "Cancelled");
            }
        }
    } else {
        game.log.alert(
                format!("The {} cannot be used.",
                        game.inventory[inventory_id].name));
    }
}
