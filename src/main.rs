extern crate tcod;
use tcod::console::{Console, Root, Offscreen, BackgroundFlag};


fn main() {
    let (width, height) = (80, 50);
    let mut root = Root::initializer()
        .size(width, height)
        .title("example")
        .fullscreen(false)
        .init();
    let mut offscreen = Offscreen::new(width, height);
    while !root.window_closed() {
        root.clear();
        root.put_char(40, 25, '@', BackgroundFlag::Set);
        root.flush();
        // let keypress = Console::wait_for_keypress(true);
        // match keypress.key {
	//     Special(key_code::Escape) => exit = true,
	//     _ => {}
        // }
    }
}
