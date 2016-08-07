#[derive(RustcEncodable, RustcDecodable)]
pub enum MessageType {
    Alert,
    Info,
    StatusChange,
    Success
}

pub type Messages = Vec<(String, MessageType)>;

pub trait MessageLog {
    fn add<T: Into<String>>(&mut self, message: T, message_type: MessageType);
    fn alert<T: Into<String>>(&mut self, message: T);
    fn info<T: Into<String>>(&mut self, message: T);
    fn status_change<T: Into<String>>(&mut self, message: T);
    fn success<T: Into<String>>(&mut self, message: T);
}

impl MessageLog for Vec<(String, MessageType)> {
    fn add<T: Into<String>>(&mut self, message: T, message_type: MessageType) {
        self.push((message.into(), message_type));
    }

    fn alert<T: Into<String>>(&mut self, message: T) {
        self.add(message, MessageType::Alert);
    }

    fn info<T: Into<String>>(&mut self, message: T) {
        self.add(message, MessageType::Info);
    }

    fn status_change<T: Into<String>>(&mut self, message: T) {
        self.add(message, MessageType::StatusChange);
    }
    fn success<T: Into<String>>(&mut self, message: T) {
        self.add(message, MessageType::Success);
    }
}
