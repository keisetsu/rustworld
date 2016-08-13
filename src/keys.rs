use tcod::input::{Key, KeyCode};

use game::{
    self,
    Game,
};

use game::PlayerAction;
use game::PlayerAction::*;

use ui::{Ui, inventory_menu};

use consts;
use log::MessageLog;
use object::Object;
use object::actor;

pub fn handle_keys(key: Key, game_ui: &mut Ui, game: &mut Game,
               objects: &mut Vec<Object>) -> PlayerAction {
    let player_alive = objects[consts::PLAYER].alive;
    match (key, player_alive) {
        // Exit: Ctrl+q
        (Key { printable: 'q', ctrl: true, .. }, _) => Exit,
        //*************************************************
        // Movement keys
        //*************************************************

        ///////////////////////////////////////////////////
        // Up
        ///////////////////////////////////////////////////
        (Key { code: KeyCode::Up, ctrl: false, alt: false, .. }, true) |
        (Key { code: KeyCode::NumPad8, ..}, true) |
        (Key { printable: 'k', ..}, true) => {
            actor::player_move_or_attack(0, -1, game, objects);
            TookTurn
        }
        ///////////////////////////////////////////////////
        // Down
        ///////////////////////////////////////////////////
        (Key { code: KeyCode::Down, ctrl: false, alt: false,.. }, true) |
        (Key { code: KeyCode::NumPad2, ctrl: false, alt: false, ..}, true) |
        (Key { printable: 'j', ..}, true) => {
            actor::player_move_or_attack(0, 1, game, objects);
            TookTurn
        }
        ///////////////////////////////////////////////////
        // Left
        ///////////////////////////////////////////////////
        (Key { code: KeyCode::Left, ctrl: false, alt: false, .. }, true) |
        (Key { code: KeyCode::NumPad4, ctrl: false, alt: false, ..}, true) |
        (Key { printable: 'h', ctrl: false, alt: false, ..}, true) => {
            actor::player_move_or_attack(-1, 0, game, objects);
            TookTurn
        }
        ///////////////////////////////////////////////////
        // Right
        ///////////////////////////////////////////////////
        (Key { code: KeyCode::Right, ctrl: false, alt: false, .. }, true) |
        (Key { code: KeyCode::NumPad6, ctrl: false, alt: false, ..}, true) |
        (Key { printable: 'l', ..}, true) => {
            actor::player_move_or_attack(1, 0, game, objects);
            TookTurn
        }
        ///////////////////////////////////////////////////
        // Up Left
        ///////////////////////////////////////////////////
        (Key { code: KeyCode::Home, .. }, true) |
        (Key { code: KeyCode::NumPad7, ..}, true) |
        (Key { printable: 'y', ..}, true) => {
            actor::player_move_or_attack(-1, -1, game, objects);
            TookTurn
        }
        ///////////////////////////////////////////////////
        // Up Right
        ///////////////////////////////////////////////////
        (Key { code: KeyCode::PageUp, .. }, true) |
        (Key { code: KeyCode::NumPad9, ..}, true) |
        (Key { printable: 'u', ..}, true) => {
            actor::player_move_or_attack(1, -1, game, objects);
            TookTurn
        }
        ///////////////////////////////////////////////////
        // Down Left
        ///////////////////////////////////////////////////
        (Key { code: KeyCode::End, .. }, true) |
        (Key { code: KeyCode::NumPad1, ..}, true) |
        (Key { printable: 'b', ..}, true) => {
            actor::player_move_or_attack(-1, 1, game, objects);
            TookTurn
        }
        ///////////////////////////////////////////////////
        // Down Right
        ///////////////////////////////////////////////////
        (Key { code: KeyCode::PageDown, .. }, true) |
        (Key { code: KeyCode::NumPad3, ..}, true) |
        (Key { printable: 'n', ..}, true) => {
            actor::player_move_or_attack(1, 1, game, objects);
            TookTurn
        }
        ///////////////////////////////////////////////////
        // Wait (Don't move)
        ///////////////////////////////////////////////////
        (Key { code: KeyCode::Spacebar, ..}, true) |
        (Key { code: KeyCode::NumPad5, ..}, true) => {
            TookTurn
        }
        //*************************************************
        // End movement keys
        //*************************************************

        ///////////////////////////////////////////////////
        // Pick up
        ///////////////////////////////////////////////////
        (Key { printable: ',', ..}, true) => {
            let (player_x, player_y) = objects[consts::PLAYER].pos();
            let names = game.map[player_x as usize][player_y as usize]
                .items.iter().
                map(|obj| obj.name.clone())
                .collect::<Vec<_>>();

            game.log.info(names.join(", "));

            let item_id = objects.iter().position(
                |object| {
                    object.pos() == objects[consts::PLAYER].pos() &&
                        object.item.is_some()
                });
            if let Some(item_id) = item_id {
                actor::pick_item_up(item_id, game, objects);
            }
            DidntTakeTurn
        }
        (Key { printable: 'i', ctrl: false, alt: false, .. }, true) => {
            let inventory_index = inventory_menu(
                &mut game.inventory,
                "Press the key next to an item to use it, \
                 or any other to cancel.\n",
                &mut game_ui.root);
            if let Some(inventory_index) = inventory_index {
                actor::use_item(game_ui, game, inventory_index, objects);
            }
            DidntTakeTurn
        }
        (Key { printable: 'd', ctrl: false, alt: false, .. }, true) => {
            let inventory_index = inventory_menu(
                &mut game.inventory,
                "Press the key next to an item to drop it, \
                 or any other to cancel.\n",
                &mut game_ui.root);
            if let Some(inventory_index) = inventory_index {
                actor::drop_item(inventory_index, game, objects);
            }
            DidntTakeTurn
        }
        (Key { printable: '>', ctrl: false, alt: false, .. }, true) => {
            // let player_on_stairs = game.map.iter().any(
            //     |object| {
            //         object.pos() == objects[consts::PLAYER].pos() &&
            //             object.name == "stairs up"
            //     });
            let (player_x, player_y) = objects[consts::PLAYER].pos();
            let player_on_stairs = game.map[player_x as usize][player_y as usize]
                .items.iter().any(|object| object.name == "stairs up");
            if player_on_stairs {
                game.log.success("Go upstairs!");
            }
            DidntTakeTurn
        }
        _ => DidntTakeTurn,
    }
}
