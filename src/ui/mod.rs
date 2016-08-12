extern crate tcod;
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
use tcod::input::Mouse;
use tcod::map::{Map as FovMap, FovAlgorithm};
use tcod::colors::{self, Color};

use std::ascii::AsciiExt;

use consts;
use game;
use game::Game;
use log::MessageType;
use map::{self, Map};
use object::{self, Object};

pub struct Ui {
    pub root: Root,
    pub con: Offscreen,
    pub panel: Offscreen,
    pub fov: FovMap,
    pub mouse: Mouse,
}

const COLOR_ALERT: Color = colors::RED;
const COLOR_INFO: Color = colors::LIGHTER_GREY;
const COLOR_SUCCESS: Color = colors::GREEN;
const COLOR_STATUS_CHANGE: Color = colors::WHITE;

const COLOR_DARK_WALL: Color = colors::BLACK;
const COLOR_LIGHT_WALL: Color = colors::DARKEST_GREY;
const COLOR_DARK_GROUND: Color = colors::DARKER_GREY;
const COLOR_LIGHT_GROUND: Color = colors::GREY;

const FOV_ALGO: FovAlgorithm = FovAlgorithm::Shadow;
const FOV_LIGHT_WALLS: bool = true;
const TORCH_RADIUS: i32 = 3;


pub fn initialize(title: &str) -> Ui {
    let root = Root::initializer()
    // .font("arial10x10.png", FontLayout::Ui)
        .font("bluebox.png", FontLayout::AsciiInRow)
        .font_type(FontType::Greyscale)
        .size(consts::SCREEN_WIDTH, consts::SCREEN_HEIGHT)
        .title(title)
        .init();

    tcod::system::set_fps(consts::LIMIT_FPS);

    Ui {
        root: root,
        con: Offscreen::new(map::MAP_WIDTH, map::MAP_HEIGHT),
        panel: Offscreen::new(consts::SCREEN_WIDTH, consts::PANEL_HEIGHT),
        // fov: FovMap::new(map::MAP_WIDTH, map::MAP_HEIGHT),
        fov: FovMap::new(map::FLOOR_WIDTH, map::FLOOR_HEIGHT),
        mouse: Default::default(),
    }

}

pub fn initialize_fov(map: &Map, game_ui: &mut Ui) {
    for y in 0..map::FLOOR_HEIGHT {
        for x in 0..map::FLOOR_WIDTH {
            game_ui.fov.set(x, y,
                         map[x as usize][y as usize].blocks_view() !=
                            object::Blocks::Full,
                         map[x as usize][y as usize].is_blocked() !=
                            object::Blocks::Full
            );
        }
    }
    game_ui.con.clear();
}

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

fn get_message_color(message_type: &MessageType) -> Color {
    match message_type {
        &MessageType::Alert => COLOR_ALERT,
        &MessageType::Info => COLOR_INFO,
        &MessageType::StatusChange => COLOR_STATUS_CHANGE,
        &MessageType::Success => COLOR_SUCCESS,
    }
}


pub fn render_all(game_ui: &mut Ui, game: &mut Game, objects: &[Object],
              fov_recompute: bool) {
    if fov_recompute {
        let player = &objects[consts::PLAYER];
        game_ui.fov.compute_fov(player.x, player.y, TORCH_RADIUS,
                             FOV_LIGHT_WALLS, FOV_ALGO);

        for x in 0..map::FLOOR_WIDTH {
            for y in 0..map::FLOOR_HEIGHT {
                let game_tile = &mut game.map[x as usize][y as usize];
                let visible = game_ui.fov.is_in_fov(x, y);
                // let visible = true;

                // let wall = game.map[x as usize][y as usize].blocks_view();
                let wall = game_tile.blocks_view();
                let color = match(visible, wall) {
                    (false, object::Blocks::Full) => COLOR_DARK_WALL,
                    (false, object::Blocks::No) => COLOR_DARK_GROUND,
                    (true, object::Blocks::Full) => COLOR_LIGHT_WALL,
                    (true, object::Blocks::No) => COLOR_LIGHT_GROUND,
                    (_, _) => COLOR_LIGHT_GROUND,
                };
                let explored =
                    &mut game_tile.explored;
                if visible {
                    for item in &game_tile.items {
                        item.draw(&mut game_ui.con);
                    }

                    *explored = true;
                }

                if *explored {
                    game_ui.con.set_char_background(x, y, color,
                                                    BackgroundFlag::Set);
                }
            }
        }
    }


    let mut to_draw: Vec<_> = objects.iter()
        .filter(|o| game_ui.fov.is_in_fov(o.x, o.y)).collect();

    to_draw.sort_by(|o1, o2| { o1.blocks.cmp(&o2.blocks) });
    for object in &to_draw {
        object.draw(&mut game_ui.con);
    }

    blit(&mut game_ui.con, (0, 0), (map::MAP_WIDTH, map::MAP_HEIGHT),
         &mut game_ui.root, (0, 0), 1.0, 1.0);

    game_ui.panel.set_default_background(colors::BLACK);
    game_ui.panel.clear();

    // print the game messages, one line at a time
    let mut y = consts::MSG_HEIGHT as i32;
    for &(ref msg, ref message_type) in game.log.iter().rev() {
        let msg_height = game_ui.panel.get_height_rect(consts::MSG_X, y,
                                               consts::MSG_WIDTH, 0, msg);
        y -= msg_height;
        if y < 0 {
            break;
        }
        game_ui.panel.set_default_foreground(get_message_color(message_type));
        game_ui.panel.print_rect(consts::MSG_X, y, consts::MSG_WIDTH, 0, msg);
    }

    // show the player's stats
    let hp = objects[consts::PLAYER].fighter.map_or(0, |f| f.hp);
    let max_hp = objects[consts::PLAYER].fighter.map_or(0, |f| f.max_hp);
    render_bar(&mut game_ui.panel, 1, 1, consts::BAR_WIDTH, "HP", hp, max_hp,
               colors::LIGHT_RED, colors::DARKER_RED);

    game_ui.panel.set_default_foreground(colors::LIGHT_GREY);
    game_ui.panel.print_ex(1, 0, BackgroundFlag::None, TextAlignment::Left,
                   get_names_under_mouse(game_ui.mouse, objects, &game_ui.fov));
    // blit the contents of `panel` to the root console
    blit(&mut game_ui.panel, (0, 0), (consts::SCREEN_WIDTH, consts::PANEL_HEIGHT),
         &mut game_ui.root, (0, consts::PANEL_Y), 1.0, 1.0);
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

    // let header_height = root.get_height_rect(0, 0,
    //                                          width, consts::SCREEN_HEIGHT, header);
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

pub fn inventory_menu(inventory: &[Object], header: &str, root: &mut Root)
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

fn get_names_under_mouse(mouse: Mouse, objects: &[Object], fov_map: &FovMap)
    -> String {
    let (x, y) = (mouse.cx as i32, mouse.cy as i32);

    let names = objects.iter().filter(
        |obj| {obj.pos() == (x, y) && fov_map.is_in_fov(obj.x, obj.y)})
        .map(|obj| obj.name.clone())
        .collect::<Vec<_>>();

    names.join(", ")
}

fn msgbox(text: &str, width: i32, root: &mut Root) {
    let options: &[&str] = &[];
    menu(text, options, width, root);
}

pub fn main_menu(game_ui: &mut Ui) {
    let img = tcod::image::Image::from_file("menu_background.png")
        .ok().expect("Background image not found");

    while !game_ui.root.window_closed() {
        tcod::image::blit_2x(&img, (0, 0), (-1, -1), &mut game_ui.root, (0, 0));
        game_ui.root.set_default_foreground(colors::LIGHT_YELLOW);
        game_ui.root.print_ex(consts::SCREEN_WIDTH/2, consts::SCREEN_HEIGHT/2 - 4,
                           BackgroundFlag::None, TextAlignment::Center,
                           "RustWorld");
        game_ui.root.print_ex(consts::SCREEN_WIDTH/2, consts::SCREEN_HEIGHT/2 - 2,
                           BackgroundFlag::None, TextAlignment::Center,
                           "Meow");

        let choices = &["Play a new game", "Continue last game", "Quit"];
        let choice = menu("", choices, 24, &mut game_ui.root);

        match choice {
            Some(0) => {
                let (mut objects, mut game) = game::new_game(game_ui);
                game::play_game(&mut objects, &mut game, game_ui);
            }
            Some(1) => {
                match game::load_game() {
                    Ok((mut objects, mut game)) => {
                        initialize_fov(&game.map, game_ui);
                        game::play_game(&mut objects, &mut game, game_ui);
                    }
                    Err(_e) => {
                        msgbox("\nSaved game failed to load.\n",
                               24, &mut game_ui.root);
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
