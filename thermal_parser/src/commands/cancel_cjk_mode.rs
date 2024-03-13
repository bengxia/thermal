use crate::{command::*, constants::*, context::*};

#[derive(Clone)]
struct Handler;

impl CommandHandler for Handler {
    fn apply_context(&self, _command: &Command, context: &mut Context) {
        context.text.is_cjk = false;
    }
}

pub fn new() -> Command {
    Command::new(
        "Cancel CJK Mode",
        vec![FS, '.' as u8],
        CommandType::Context,
        DataType::Empty,
        Box::new(Handler {}),
    )
}
