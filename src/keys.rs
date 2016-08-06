extern crate tcod

use tcod::input::{Key, KeyCode};

fn handle_keys(key: Key, tcod: &mut Tcod, game: &mut Game,
               objects: &mut Vec<Object>) -> PlayerAction {
    use PlayerAction::*;

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
        (Key { code: KeyCode::Up, .. }, true) |
        (Key { code: KeyCode::NumPad8, ..}, true) |
        (Key { printable: 'k', ..}, true) => {
            player_move_or_attack(0, -1, game, objects);
            TookTurn
        }
        ///////////////////////////////////////////////////
        // Down
        ///////////////////////////////////////////////////
        (Key { code: KeyCode::Down, .. }, true) |
        (Key { code: KeyCode::NumPad2, ..}, true) |
        (Key { printable: 'j', ..}, true) => {
            player_move_or_attack(0, 1, game, objects);
            TookTurn
        }
        ///////////////////////////////////////////////////
        // Left
        ///////////////////////////////////////////////////
        (Key { code: KeyCode::Left, .. }, true) |
        (Key { code: KeyCode::NumPad4, ..}, true) |
        (Key { printable: 'h', ..}, true) => {
            player_move_or_attack(-1, 0, game, objects);
            TookTurn
        }
        ///////////////////////////////////////////////////
        // Right
        ///////////////////////////////////////////////////
        (Key { code: KeyCode::Right, .. }, true) |
        (Key { code: KeyCode::NumPad6, ..}, true) |
        (Key { printable: 'l', ..}, true) => {
            player_move_or_attack(1, 0, game, objects);
            TookTurn
        }
        ///////////////////////////////////////////////////
        // Up Left
        ///////////////////////////////////////////////////
        (Key { code: KeyCode::Home, .. }, true) |
        (Key { code: KeyCode::NumPad7, ..}, true) |
        (Key { printable: 'y', ..}, true) => {
            player_move_or_attack(-1, -1, game, objects);
            TookTurn
        }
        ///////////////////////////////////////////////////
        // Up Right
        ///////////////////////////////////////////////////
        (Key { code: KeyCode::PageUp, .. }, true) |
        (Key { code: KeyCode::NumPad9, ..}, true) |
        (Key { printable: 'u', ..}, true) => {
            player_move_or_attack(1, -1, game, objects);
            TookTurn
        }
        ///////////////////////////////////////////////////
        // Down Left
        ///////////////////////////////////////////////////
        (Key { code: KeyCode::End, .. }, true) |
        (Key { code: KeyCode::NumPad1, ..}, true) |
        (Key { printable: 'b', ..}, true) => {
            player_move_or_attack(-1, 1, game, objects);
            TookTurn
        }
        ///////////////////////////////////////////////////
        // Down Right
        ///////////////////////////////////////////////////
        (Key { code: KeyCode::PageDown, .. }, true) |
        (Key { code: KeyCode::NumPad3, ..}, true) |
        (Key { printable: 'n', ..}, true) => {
            player_move_or_attack(1, 1, game, objects);
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
            let item_id = objects.iter().position(
                |object| {
                    object.pos() == objects[consts::PLAYER].pos() &&
                        object.item.is_some()
                });
            if let Some(item_id) = item_id {
                pick_item_up(item_id, game, objects);
            }
            DidntTakeTurn
        }
        (Key { printable: 'i', .. }, true) => {
            let inventory_index = inventory_menu(
                &mut game.inventory,
                "Press the key next to an item to use it, \
                 or any other to cancel.\n",
                &mut tcod.root);
            if let Some(inventory_index) = inventory_index {
                use_item(tcod, game, inventory_index, objects);
            }
            DidntTakeTurn
        }
        (Key { printable: 'd', .. }, true) => {
            let inventory_index = inventory_menu(
                &mut game.inventory,
                "Press the key next to an item to drop it, \
                 or any other to cancel.\n",
                &mut tcod.root);
            if let Some(inventory_index) = inventory_index {
                drop_item(inventory_index, game, objects);
            }
            DidntTakeTurn
        }
        (Key { printable: '>', .. }, true) => {
            let player_on_stairs = objects.iter().any(
                |object| {
                    object.pos() == objects[consts::PLAYER].pos() &&
                        object.name == "stairs up"
                });
            if player_on_stairs {
                next_level(tcod, objects, game);
            }
            DidntTakeTurn
        }
        _ => DidntTakeTurn,
    }
}
