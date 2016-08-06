use std::cmp;
use std::ascii::AsciiExt;

extern crate rand;
use rand::Rng;

extern crate rustc_serialize;

extern crate tcod;

use std::io::{Read, Write};
use std::fs::File;
use std::error::Error;

use rustc_serialize::json;

use tcod::console::{
    BackgroundFlag,
    blit,
    Console,
    FontLayout,
    FontType,
    Offscreen,
    Root,
    TextAlignment,
};

use tcod::map::Map as FovMap;
use tcod::colors::{self, Color};
use tcod::input::{self, Event, Key, KeyCode, Mouse};


mod consts;


#[derive(Clone, Copy, Debug, PartialEq)]
enum PlayerAction {
    TookTurn,
    DidntTakeTurn,
    Exit,
}

enum UseResult {
    UsedUp,
    Cancelled,
}

#[derive(Debug, RustcEncodable, RustcDecodable)]
enum Ai {
    Basic,
    Chrysalis,
    Confused{previous_ai: Box<Ai>, num_turns: i32},
}


struct Tcod {
    root: Root,
    con: Offscreen,
    panel: Offscreen,
    fov: FovMap,
    mouse: Mouse,
}

#[derive(RustcEncodable, RustcDecodable)]
struct Game {
    map: Map,
    log: Messages,
    inventory: Vec<Object>,
}


#[derive(Clone, Copy, Debug, PartialEq, RustcEncodable, RustcDecodable)]
enum Item {
    Confuse,
    Fireball,
    Heal,
    Lightning,
}

fn move_by(id: usize, dx: i32, dy: i32, map: &Map, objects: &mut[Object]) {
    let (x, y) = objects[id].pos();
    if !is_blocked(x + dx, y + dy, map, objects) {
        objects[id].set_pos(x + dx, y + dy);
    }
}

fn move_towards(id: usize, target_x: i32, target_y: i32, map: &Map,
                objects: &mut [Object]) {
    let dx = target_x - objects[id].x;
    let dy = target_y - objects[id].y;
    let distance = ((dx.pow(2) + dy.pow(2)) as f32).sqrt();

    let dx = (dx as f32 / distance).round() as i32;
    let dy = (dy as f32 / distance).round() as i32;

    move_by(id, dx, dy, map, objects);
}

fn player_move_or_attack(dx: i32, dy: i32, game: &mut Game,
                         objects: &mut [Object]) {
    let x = objects[consts::PLAYER].x + dx;
    let y = objects[consts::PLAYER].y + dy;

    let target_id = objects.iter().position(|object| {
        object.fighter.is_some() && object.pos() == (x, y)
    });

    match target_id {
        Some(target_id) => {
            let (player, target) = mut_two(consts::PLAYER, target_id, objects);
            player.attack(target, &mut game.log);
        }
        None => {
            move_by(consts::PLAYER, dx, dy, &mut game.map, objects);
        }
    }
}

fn target_tile(tcod: &mut Tcod, game: &mut Game, objects: &[Object],
               max_range: Option<f32>)
    -> Option<(i32, i32)> {
    loop {
        tcod.root.flush();
        let event = input::check_for_event(input::KEY_PRESS |
                                           input::MOUSE)
            .map(|e| e.1);
        let mut key = None;
        match event {
            Some(Event::Mouse(m)) => tcod.mouse = m,
            Some(Event::Key(k)) => key = Some(k),
            None => {}
        }
        render_all(tcod, game, objects, false);

        let (x, y) = (tcod.mouse.cx as i32, tcod.mouse.cy as i32);
        let in_fov = (x < consts::MAP_WIDTH) && (y < consts::MAP_HEIGHT) &&
            tcod.fov.is_in_fov(x, y);
        let in_range = max_range.map_or(true,
                                        |range| objects[consts::PLAYER]
                                    .distance(x, y) <= range);
        if tcod.mouse.lbutton_pressed && in_fov && in_range {
            return Some((x, y))
        }

        let escape = key.map_or(false, |k| k.code == KeyCode::Escape);
        if tcod.mouse.rbutton_pressed || escape {
            return None
        }
    }
}

fn target_monster(tcod: &mut Tcod, game: &mut Game, objects: &[Object],
                  max_range: Option<f32>) -> Option<usize> {
    loop {
        match target_tile(tcod, game, objects, max_range) {
            Some((x, y)) => {
                for(id, obj) in objects.iter().enumerate() {
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

fn drop_item(inventory_id: usize, game: &mut Game,
             objects: &mut Vec<Object>) {
    let mut item = game.inventory.remove(inventory_id);
    item.set_pos(objects[consts::PLAYER].x, objects[consts::PLAYER].y);
    game.log.add(format!("You dropped a {}.", item.name),
                 colors::YELLOW);
    objects.push(item);
}

fn pick_item_up(object_id: usize, game: &mut Game,
                objects: &mut Vec<Object>) {
    if game.inventory.len() as i32 >= consts::MAX_INVENTORY_ITEMS {
        game.log.add(
                format!("Your inventory is full, cannot pickup {}.",
                        objects[object_id].name),
                colors::RED);
    } else {
        let item = objects.swap_remove(object_id);
        game.log.add( format!("You picked up a {}!", item.name),
                colors::GREEN);
        game.inventory.push(item);
    }
}

fn mut_two<T>(first_index: usize, second_index: usize, items: &mut [T]) ->
    (&mut T, &mut T) {
        assert!(first_index != second_index);
        let split_at_index = cmp::max(first_index, second_index);
        let (first_slice, second_slice) = items.split_at_mut(split_at_index);
        if first_index < second_index {
            (&mut first_slice[first_index], &mut second_slice[0])
        } else {
            (&mut second_slice[0], &mut first_slice[second_index])
        }
    }

fn ai_take_turn(monster_id: usize, game: &mut Game, objects: &mut [Object],
                fov_map: &FovMap) {
    if let Some(ai) = objects[monster_id].ai.take() {
        let new_ai = match ai {
            Ai::Basic => ai_basic(monster_id, game, objects, fov_map),
            Ai::Chrysalis => ai_chrysalis(monster_id, game, objects, fov_map),
            Ai::Confused{previous_ai, num_turns} => ai_confused(
                monster_id, game, objects, previous_ai, num_turns)
        };
        objects[monster_id].ai = Some(new_ai);
    }
}
fn ai_move_randomly(monster_id: usize, map: &Map, objects: &mut[Object]) {
    move_by(monster_id,
            rand::thread_rng().gen_range(-1, 2),
            rand::thread_rng().gen_range(-1, 2),
            map, objects);
}

fn ai_basic(monster_id: usize, game: &mut Game, objects: &mut [Object],
            fov_map: &FovMap) -> Ai {
    let (monster_x, monster_y) = objects[monster_id].pos();
    if fov_map.is_in_fov(monster_x, monster_y) {
        if objects[monster_id].distance_to(&objects[consts::PLAYER]) >= 2.0 {
            let (player_x, player_y) = objects[consts::PLAYER].pos();
            move_towards(monster_id, player_x, player_y, &mut game.map, objects);
        } else if objects[consts::PLAYER].fighter.map_or(false, |f| f.hp > 0) {
            let (monster, player) = mut_two(monster_id, consts::PLAYER, objects);
            monster.attack(player, &mut game.log);
        }
    } else {
        ai_move_randomly(monster_id, &game.map, objects);
    }
    Ai::Basic
}

fn ai_chrysalis(monster_id: usize, game: &mut Game, objects: &mut [Object],
                fov_map: &FovMap) -> Ai {
    let (monster_x, monster_y) = objects[monster_id].pos();
    if fov_map.is_in_fov(monster_x, monster_y) {
        if objects[monster_id].distance_to(&objects[consts::PLAYER]) >= 2.0 {
            let (player_x, player_y) = objects[consts::PLAYER].pos();
            move_towards(monster_id, player_x, player_y, &mut game.map, objects);
        } else if objects[consts::PLAYER].fighter.map_or(false, |f| f.hp > 0) {
            let (monster, player) = mut_two(monster_id, consts::PLAYER, objects);
            monster.attack(player, &mut game.log);
        }
    }
    Ai::Chrysalis
}

fn ai_confused(monster_id: usize, game: &mut Game, objects: &mut [Object],
               previous_ai: Box<Ai>, num_turns: i32)
               -> Ai {

    if num_turns >= 0 {
        game.log.add(format!("The {} is confused!",
                                  objects[monster_id].name),
                colors::LIGHT_BLUE);
        ai_move_randomly(monster_id, &game.map, objects);
        Ai::Confused{previous_ai: previous_ai, num_turns: num_turns - 1}
    } else {
        game.log.add(format!("The {} is no longer confused!",
                                  objects[monster_id].name),
                colors::RED);
        *previous_ai
    }
}

fn player_death(player: &mut Object, log: &mut Messages) {
    log.add("You died!", colors::RED);
    player.symbol = '%';
    player.color = colors::DARK_RED;
}

fn monster_death(monster: &mut Object, log: &mut Messages) {
    log.add(format!("{} is dead!", monster.name), colors::ORANGE);
    monster.symbol = '%';
    monster.color = colors::DARK_RED;
    monster.blocks = false;
    monster.fighter = None;
    monster.ai = None;
    monster.name = format!("remains of {}", monster.name);
}

// fn message<T: Into<String>>(messages: &mut Messages, message: T, color: Color) {
//     if messages.len() == consts::MSG_HEIGHT {
//         messages.remove(0);
//     }
//     messages.push((message.into(), color));
// }

fn render_bar(panel: &mut Offscreen,
              x: i32,
              y: i32,
              total_width: i32,
              name: &str,
              value: i32,
              maximum: i32,
              bar_color: Color,
              back_color: Color) {
    let bar_width = (value as f32 / maximum as f32 * total_width as f32) as i32;
    panel.set_default_background(back_color);
    panel.rect(x, y, total_width, 1, false, BackgroundFlag::Screen);

    panel.set_default_background(bar_color);
    if bar_width > 0 {
        panel.rect(x, y, bar_width, 1, false, BackgroundFlag::Screen);
    }

    panel.set_default_foreground(colors::WHITE);
    panel.print_ex(x + total_width / 2, y, BackgroundFlag::None,
                   TextAlignment::Center, &format!("{}: {}/{}",
                                                   name, value, maximum));
}

fn render_all(tcod: &mut Tcod, game: &mut Game, objects: &[Object],
              fov_recompute: bool) {
    if fov_recompute {
        let player = &objects[consts::PLAYER];
        tcod.fov.compute_fov(player.x, player.y, consts::TORCH_RADIUS,
                             consts::FOV_LIGHT_WALLS, consts::FOV_ALGO);

        for y in 0..consts::MAP_HEIGHT {
            for x in 0..consts::MAP_WIDTH {
                let visible = tcod.fov.is_in_fov(x, y);
                // let visible = true;
                let wall = game.map[x as usize][y as usize].blocks_sight;
                let color = match(visible, wall) {
                    (false, true) => consts::COLOR_DARK_WALL,
                    (false, false) => consts::COLOR_DARK_GROUND,
                    (true, true) => consts::COLOR_LIGHT_WALL,
                    (true, false) => consts::COLOR_LIGHT_GROUND,
                };

                let explored =
                    &mut game.map[x as usize][y as usize].explored;
                if visible {
                    *explored = true;
                }

                if *explored {
                    tcod.con.set_char_background(x, y, color,
                                            BackgroundFlag::Set);
                }
            }
        }
    }

    let mut to_draw: Vec<_> = objects.iter()
        .filter(|o| tcod.fov.is_in_fov(o.x, o.y)).collect();

    to_draw.sort_by(|o1, o2| { o1.blocks.cmp(&o2.blocks) });
    for object in &to_draw {
        object.draw(&mut tcod.con);
    }

    blit(&mut tcod.con, (0, 0), (consts::MAP_WIDTH, consts::MAP_HEIGHT),
         &mut tcod.root, (0, 0), 1.0, 1.0);

    tcod.panel.set_default_background(colors::BLACK);
    tcod.panel.clear();

    // print the game messages, one line at a time
    let mut y = consts::MSG_HEIGHT as i32;
    for &(ref msg, color) in game.log.iter().rev() {
        let msg_height = tcod.panel.get_height_rect(consts::MSG_X, y,
                                               consts::MSG_WIDTH, 0, msg);
        y -= msg_height;
        if y < 0 {
            break;
        }
        tcod.panel.set_default_foreground(color);
        tcod.panel.print_rect(consts::MSG_X, y, consts::MSG_WIDTH, 0, msg);
    }

    // show the player's stats
    let hp = objects[consts::PLAYER].fighter.map_or(0, |f| f.hp);
    let max_hp = objects[consts::PLAYER].fighter.map_or(0, |f| f.max_hp);
    render_bar(&mut tcod.panel, 1, 1, consts::BAR_WIDTH, "HP", hp, max_hp,
               colors::LIGHT_RED, colors::DARKER_RED);

    tcod.panel.set_default_foreground(colors::LIGHT_GREY);
    tcod.panel.print_ex(1, 0, BackgroundFlag::None, TextAlignment::Left,
                   get_names_under_mouse(tcod.mouse, objects, &tcod.fov));
    // blit the contents of `panel` to the root console
    blit(&mut tcod.panel, (0, 0), (consts::SCREEN_WIDTH, consts::PANEL_HEIGHT),
         &mut tcod.root, (0, consts::PANEL_Y), 1.0, 1.0);
}

fn menu<T: AsRef<str>>(header: &str, options: &[T], width: i32,
                       root: &mut Root) -> Option<usize> {
    let header_height = if header.is_empty() {
        0
    } else {
        root.get_height_rect(0, 0, width, consts::SCREEN_HEIGHT, header)
    };

    let options_len = options.len() as i32;
    assert!(options_len <= consts::MAX_INVENTORY_ITEMS,
            format!("Cannot have a menu with more than {} options.",
            consts::MAX_INVENTORY_ITEMS));

    let header_height = root.get_height_rect(0, 0,
                                             width, consts::SCREEN_HEIGHT, header);
    let height = options_len + header_height;

    let mut window = Offscreen::new(width, height);

    window.set_default_foreground(colors::WHITE);
    window.print_rect_ex(0, 0, width, height, BackgroundFlag::None,
                         TextAlignment::Left, header);

    for (index, option_text) in options.iter().enumerate() {
        let menu_letter = (b'a' + index as u8) as char;
        let text = format!("({}) {}", menu_letter, option_text.as_ref());
        window.print_ex(0, header_height + index as i32,
                        BackgroundFlag::None, TextAlignment::Left, text);
    }

    let x = consts::SCREEN_WIDTH / 2 - width / 2;
    let y = consts::SCREEN_HEIGHT / 2 - height / 2;
    blit(&mut window, (0, 0), (width, height), root, (x, y),
         1.0, 0.7);

    root.flush();
    let key = root.wait_for_keypress(true);

    if key.printable.is_alphabetic() {
        let index = key.printable.to_ascii_lowercase() as usize - 'a' as usize;
        if index < options.len() {
            Some(index)
        } else {
            None
        }
    } else {
        None
    }
}

fn inventory_menu(inventory: &[Object], header: &str, root: &mut Root)
                  -> Option<usize> {
    let options = if inventory.len() == 0 {
        vec!["Inventory is empty.".into()]
    } else {
        inventory.iter().map(|item| {item.name.clone() }).collect()
    };

    let inventory_index = menu(header, &options,
                               consts::INVENTORY_WIDTH, root);


    if inventory.len() > 0 {
        inventory_index
    } else {
        None
    }
}

fn use_item(tcod: &mut Tcod, game: &mut Game, inventory_id: usize,
            objects: &mut [Object]) {
    if let Some(item) = game.inventory[inventory_id].item {
        let on_use:
        fn(&mut Tcod, &mut Game, usize, &mut [Object])
           -> UseResult = match item {
            Item::Confuse => cast_confuse,
            Item::Fireball => cast_fireball,
            Item::Heal => heal_player,
            Item::Lightning => cast_lightning,
        };
        match on_use(tcod, game, inventory_id, objects) {
            UseResult::UsedUp => {
                game.inventory.remove(inventory_id);
            }
            UseResult::Cancelled => {
                game.log.add( "Cancelled", colors::WHITE);
            }
        }
    } else {
        game.log.add(
                format!("The {} cannot be used.",
                        game.inventory[inventory_id].name),
                colors::WHITE);
    }
}

fn closest_monster(max_range: i32, objects: &mut [Object], tcod: &Tcod)
    -> Option<usize> {
    let mut closest_enemy = None;
    let mut closest_dist = (max_range + 1) as f32;

    for (id, object) in objects.iter().enumerate() {
        if (id != consts::PLAYER) &&
            object.fighter.is_some() &&
            object.ai.is_some() &&
            tcod.fov.is_in_fov(object.x, object.y) {
                let dist = objects[consts::PLAYER].distance_to(object);
                if dist < closest_dist {
                    closest_enemy = Some(id);
                    closest_dist = dist;
                }
            }
    }
    closest_enemy
}

fn heal_player(tcod: &mut Tcod, game: &mut Game, inventory_id: usize,
                 objects: &mut [Object]) -> UseResult {
    if let Some(fighter) = objects[consts::PLAYER].fighter {
        if fighter.hp == fighter.max_hp {
            game.log.add( "You are already at full health.", colors::RED);
            return UseResult::Cancelled;
        }
        game.log.add( "Your wounds start to feel better!",
                colors::LIGHT_VIOLET);
        objects[consts::PLAYER].heal(3);
        return UseResult::UsedUp;
    }
    UseResult::Cancelled
}

fn cast_confuse(tcod: &mut Tcod, game: &mut Game, inventory_id: usize,
                 objects: &mut [Object]) -> UseResult {
    game.log.add( "Left-click an enemy to confuse it, or right-click \
                       to cancel.",
            colors::LIGHT_CYAN);
    let monster_id = target_monster(tcod, game, objects, Some(5.0));
    if let Some(monster_id) = monster_id {
        let old_ai = objects[monster_id].ai.take().unwrap_or(Ai::Basic);
        objects[monster_id].ai = Some(Ai::Confused {
            previous_ai: Box::new(old_ai),
            num_turns: 3,
        });
        game.log.add(
                format!("The eyes of the {} look vacant and it starts to \
                         stumble around!", objects[monster_id].name),
                colors::LIGHT_GREEN);
        UseResult::UsedUp
    } else {
        game.log.add( "No enemy is within range.", colors::RED);
        UseResult::Cancelled
    }
}

fn cast_fireball(tcod: &mut Tcod, game: &mut Game, inventory_id: usize,
                 objects: &mut [Object]) -> UseResult {
    game.log.add( "Left-click a target tile for the fireball, \
                       or right-click to cancel.",
            colors::LIGHT_CYAN);
    let (x, y) = match target_tile(tcod, game, objects, None) {
        Some(tile_pos) => tile_pos,
        None => return UseResult::Cancelled,
        };

    game.log.add(
            format!("The fireball explodes, burning everything within a {} \
                     radius!", 5),
            colors::GREEN);

    for obj in objects {
        if obj.distance(x, y) <= 5.0 && obj.fighter.is_some() {
            game.log.add(
                    format!("The {} gets burned for {} hit points.",
                            obj.name, 10),
                    colors::ORANGE);
            obj.take_damage(10, &mut game.log);
        }
    }
    UseResult::UsedUp
}


fn cast_lightning(tcod: &mut Tcod, game: &mut Game, inventory_id: usize,
                 objects: &mut [Object]) -> UseResult {
    let monster_id = closest_monster(10, objects, tcod);
    if let Some(monster_id) = monster_id {
        game.log.add(
                format!("A lightning bolt strikes the {} with loud thunder! \
                         The damage is {} hit points.",
                        objects[monster_id].name, 10),
                colors::LIGHT_BLUE);
        objects[monster_id].take_damage(10, &mut game.log);
        UseResult::UsedUp
    } else {
        game.log.add( "No enemy is within range.", colors::RED);
        UseResult::Cancelled
    }
}

fn get_names_under_mouse(mouse: Mouse, objects: &[Object], fov_map: &FovMap)
    -> String {
    let (x, y) = (mouse.cx as i32, mouse.cy as i32);

    let names = objects.iter().filter(
        |obj| {obj.pos() == (x, y) && fov_map.is_in_fov(obj.x, obj.y)})
        .map(|obj| obj.name.clone())
        .collect::<Vec<_>>();

    names.join(", ")
}

fn save_game(objects: &[Object], game: &Game) -> Result<(), Box<Error>> {
    let save_data = try!{ json::encode(&(objects, game)) };
    let mut file = try!{ File::create("savegame") };
    try!{ file.write_all(save_data.as_bytes()) };
    Ok(())
}

fn load_game() -> Result<(Vec<Object>, Game), Box<Error>> {
    let mut json_save_state = String::new();
    let mut file = try! { File::open("savegame") };
    try! { file.read_to_string(&mut json_save_state) };
    let result = try! { json::decode::<(Vec<Object>, Game)>(&json_save_state) };
    Ok(result)
}

fn new_game(tcod: &mut Tcod) -> (Vec<Object>, Game) {
    let mut player = Object::new(0, 0, '@', "player", colors::WHITE, true);
    player.alive = true;
    player.fighter = Some(Fighter{
        max_hp: 30, hp: 30, defense: 2, power: 5, on_death: DeathCallback::Player,
    });
    let mut objects = vec![player];
    let mut game = Game {
        map: make_map(&mut objects),
        log: vec![],
        inventory: vec![],
    };

    initialize_fov(&game.map, tcod);

    game.log.add("Meow!", colors::RED);

    (objects, game)

}

fn initialize_fov(map: &Map, tcod: &mut Tcod) {
    for y in 0..consts::MAP_HEIGHT {
        for x in 0..consts::MAP_WIDTH {
            tcod.fov.set(x, y,
                         !map[x as usize][y as usize].blocks_sight,
                         !map[x as usize][y as usize].impassable);
        }
    }
    tcod.con.clear();
}

fn play_game(objects: &mut Vec<Object>, game: &mut Game, tcod: &mut Tcod) {

    let mut previous_player_position = (-1, -1);
    let mut key = Default::default();

    while !tcod.root.window_closed() {
        let fov_recompute = previous_player_position !=
            (objects[consts::PLAYER].pos());
        match input::check_for_event(input::MOUSE | input::KEY_PRESS) {
            Some((_, Event::Mouse(m))) => tcod.mouse = m,
            Some((_, Event::Key(k))) => key = k,
            _ => key = Default::default(),
        }

        render_all(tcod, game, &objects, fov_recompute);

        tcod.root.flush();

        for object in objects.iter_mut() {
            object.clear(&mut tcod.con)
        }

        previous_player_position = objects[consts::PLAYER].pos();
        let player_action = handle_keys(key, tcod, game, objects);
        if player_action == PlayerAction::Exit {
            //# TODO: Catch and handle save game errors
            save_game(objects, game).unwrap();
            break
        }

        if objects[consts::PLAYER].alive &&
            player_action != PlayerAction::DidntTakeTurn {
                for id in 0..objects.len() {
                    if objects[id].ai.is_some() {
                        ai_take_turn(id, game, objects, &tcod.fov);
                    }
                }
            }

    }
}

fn next_level(tcod: &mut Tcod, objects: &mut Vec<Object>, game: &mut Game) {
    game.map = make_map(objects);
    initialize_fov(&game.map, tcod);
}

fn msgbox(text: &str, width: i32, root: &mut Root) {
    let options: &[&str] = &[];
    menu(text, options, width, root);
}

fn main_menu(tcod: &mut Tcod) {
    let img = tcod::image::Image::from_file("menu_background.png")
        .ok().expect("Background image not found");

    while !tcod.root.window_closed() {
        tcod::image::blit_2x(&img, (0, 0), (-1, -1), &mut tcod.root, (0, 0));
        tcod.root.set_default_foreground(colors::LIGHT_YELLOW);
        tcod.root.print_ex(consts::SCREEN_WIDTH/2, consts::SCREEN_HEIGHT/2 - 4,
                           BackgroundFlag::None, TextAlignment::Center,
                           "RustWorld");
        tcod.root.print_ex(consts::SCREEN_WIDTH/2, consts::SCREEN_HEIGHT/2 - 2,
                           BackgroundFlag::None, TextAlignment::Center,
                           "Meow");

        let choices = &["Play a new game", "Continue last game", "Quit"];
        let choice = menu("", choices, 24, &mut tcod.root);

        match choice {
            Some(0) => {
                let (mut objects, mut game) = new_game(tcod);
                play_game(&mut objects, &mut game, tcod);
            }
            Some(1) => {
                match load_game() {
                    Ok((mut objects, mut game)) => {
                        initialize_fov(&game.map, tcod);
                        play_game(&mut objects, &mut game, tcod);
                    }
                    Err(_e) => {
                        msgbox("\nSaved game failed to load.\n",
                               24, &mut tcod.root);
                        continue;
                    }
                }
            }
            Some(2) => {
                break;
            }
            _ => {}
        }
    }
}

fn main() {
    let root = Root::initializer()
        // .font("arial10x10.png", FontLayout::Tcod)
        .font("bluebox.png", FontLayout::AsciiInRow)
        .font_type(FontType::Greyscale)
        .size(consts::SCREEN_WIDTH, consts::SCREEN_HEIGHT)
        .title("RustWorld")
        .init();

    tcod::system::set_fps(consts::LIMIT_FPS);

    let mut tcod = Tcod {
        root: root,
        con: Offscreen::new(consts::MAP_WIDTH, consts::MAP_HEIGHT),
        panel: Offscreen::new(consts::SCREEN_WIDTH, consts::PANEL_HEIGHT),
        fov: FovMap::new(consts::MAP_WIDTH, consts::MAP_HEIGHT),
        mouse: Default::default(),
    };

    main_menu(&mut tcod);
}
