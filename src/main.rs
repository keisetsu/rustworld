extern crate tcod;
use tcod::console::{
    BackgroundFlag,
    blit,
    Console,
    FontLayout,
    FontType,
    Offscreen,
    Root
};

use tcod::colors::{self, Color};

const SCREEN_WIDTH: i32 = 80;
const SCREEN_HEIGHT: i32 = 50;
const LIMIT_FPS: i32 = 20;

const MAP_WIDTH: i32 = 80;
const MAP_HEIGHT: i32 = 45;

const COLOR_DARK_WALL: Color = Color { r: 0, g: 0, b: 100 };
const COLOR_DARK_GROUND: Color = Color { r: 50, g: 50, b: 150 };

type Map = Vec<Vec<Tile>>;

#[derive(Clone, Copy, Debug)]
struct Tile {
    impassable: bool,
    blocks_sight: bool,
}

impl Tile {
    pub fn empty() -> Self {
        Tile{ impassable: false, blocks_sight: false}
    }

    pub fn wall() -> Self {
        Tile{ impassable: true, blocks_sight: true }
    }
}

#[derive(Debug)]
struct Object {
    x: i32,
    y: i32,
    symbol: char,
    color: Color,
}

impl Object {
    pub fn new(x: i32, y: i32, symbol: char, color: Color)
               -> Self {
        Object {
            x: x,
            y: y,
            symbol: symbol,
            color: color,
        }
    }

    pub fn move_by(&mut self, dx: i32, dy: i32, map: &Map) {
        if !map[(self.x + dx) as usize][(self.y + dy) as usize]
            .impassable {
                self.x += dx;
                self.y += dy;
            }
    }

    pub fn draw(&self, con: &mut Console) {
        con.set_default_foreground(self.color);
        con.put_char(self.x, self.y, self.symbol,
                     BackgroundFlag::None);
    }

    pub fn clear(&self, con: &mut Console) {
        con.put_char(self.x, self.y, ' ', BackgroundFlag::None);
    }
}

fn make_map() -> Map {
    let mut map = vec![vec![Tile::empty(); MAP_HEIGHT as usize];
                       MAP_WIDTH as usize];
    map[30][22] = Tile::wall();
    map[50][22] = Tile::wall();

    map
}

fn render_all(root: &mut Root, con: &mut Offscreen,
              objects: &[Object], map: &Map) {
    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            let wall = map[x as usize][y as usize].blocks_sight;
            if wall {
                println!("Wall");
                con.set_char_background(x, y, COLOR_DARK_WALL,
                                        BackgroundFlag::Set);
            } else {
                con.set_char_background(x, y, COLOR_DARK_GROUND,
                                        BackgroundFlag::Set);
            }
        }
    }

    for object in objects {
        object.draw(con);
    }

    blit(con, (0, 0), (MAP_WIDTH, MAP_HEIGHT), root, (0, 0),
         1.0, 1.0);
}

fn handle_keys(root: &mut Root, player: &mut Object, map: &Map)
               -> bool {
    use tcod::input::Key;
    use tcod::input::KeyCode::*;

    let key = root.wait_for_keypress(true);
    match key {
        // Exit: Ctrl+q
        Key { printable: 'q', ctrl: true, .. } => return true,
        // movement keys
        Key { code: Up, .. } => player.move_by(0, -1, map),
        Key { code: Down, .. } => player.move_by(0, 1, map),
        Key { code: Left, .. } => player.move_by(-1, 0, map),
        Key { code: Right, .. } => player.move_by(1, 0, map),
        _ => {},
    }
    false
}

fn main() {
    let mut root = Root::initializer()
        .font("arial10x10.png", FontLayout::Tcod)
        .font_type(FontType::Greyscale)
        .size(SCREEN_WIDTH, SCREEN_HEIGHT)
        .title("RustWorld")
        .init();

    tcod::system::set_fps(LIMIT_FPS);
    let mut con = Offscreen::new(MAP_WIDTH, MAP_HEIGHT);

    let player = Object::new(SCREEN_WIDTH / 2,
                             SCREEN_HEIGHT / 2,
                             '@', colors::WHITE);
    let npc = Object::new(SCREEN_WIDTH / 2 - 5,
                             SCREEN_HEIGHT / 2,
                             '@', colors::YELLOW);

    let mut objects = [player, npc];

    let map = make_map();

    while !root.window_closed() {
        render_all(&mut root, &mut con, &objects, &map);

        root.flush();

        for object in &objects {
            object.clear(&mut con)
        }

        let player = &mut objects[0];

        let exit = handle_keys(&mut root, player, &map);
        if exit {
            break
        }
    }
}
