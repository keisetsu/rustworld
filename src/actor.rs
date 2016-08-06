extern crate rustc_serialize

mod log


#[derive(Clone, Copy, Debug, PartialEq, RustcEncodable, RustcDecodable)]
enum DeathCallback {
    Player,
    Monster,
}

impl DeathCallback {
    fn callback(self, object: &mut Object, messages: &mut log::Messages) {
        use DeathCallback::*;
        let callback: fn(&mut Object, &mut log::Messages) = match self {
            Player => player_death,
            Monster => monster_death,
        };
        callback(object, messages);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, RustcEncodable, RustcDecodable)]
struct Fighter {
    max_hp: i32,
    hp: i32,
    defense: i32,
    power: i32,
    on_death: DeathCallback,
}
