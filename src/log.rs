extern crate tcod;

use tcod::colors::{self, Color};

pub const ALERT: Color = colors::RED;
pub const INFO: Color = colors::BLUE;
pub const SUCCESS: Color = colors::GREEN;


pub type Messages = Vec<(String, Color)>;

pub trait MessageLog {
    fn add<T: Into<String>>(&mut self, message: T, color: Color);
    fn alert<T: Into<String>>(&mut self, message: T);
    fn info<T: Into<String>>(&mut self, message: T);
    fn success<T: Into<String>>(&mut self, message: T);
}

impl MessageLog for Vec<(String, Color)> {
    fn add<T: Into<String>>(&mut self, message: T, color: Color) {
        self.push((message.into(), color));
    }

    fn alert<T: Into<String>>(&mut self, message: T) {
        self.add(message, ALERT);
    }

    fn info<T: Into<String>>(&mut self, message: T) {
        self.add(message, INFO);
    }

    fn success<T: Into<String>>(&mut self, message: T) {
        self.add(message, SUCCESS);
    }
}
