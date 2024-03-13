use crate::{command::*, context::*};
use chardetng::EncodingDetector;

#[derive(Clone)]
struct Handler;

impl CommandHandler for Handler {
    fn get_text(&self, command: &Command, _context: &Context) -> Option<String> {
        //TODO we may need to add the codepage to the context and do proper conversion instead of just using utf8
        // return match from_utf8(&command.data as &[u8]) {
        //     Ok(str) => Some(str.to_string()),
        //     Err(err) => {
        //         print!("UTF8 TEXT ERROR {} {:02X?}", err, &command.data);
        //         None
        //     }
        // };
        let mut encdet = EncodingDetector::new();
        encdet.feed(&command.data as &[u8], true);
        let codec = encdet.guess(None, false);
        return match codec.decode_without_bom_handling_and_without_replacement(&command.data) {
            Some(cowstr) => Some(cowstr.to_string()),
            None => {
                print!("DECODE TO UTF-8 FAILED {:02X?}", &command.data);
                None
            }
        };
        //TODO: decode the text according to the context's codec.
    }
    fn debug(&self, command: &Command, context: &Context) -> String {
        self.get_text(command, context).unwrap_or("".to_string())
    }

    //TODO: impl apply_context trait fn, to detect the encoding of the text, and set codec.
}

pub fn new() -> Command {
    Command::new(
        "Text",
        vec![],
        CommandType::Text,
        DataType::Text,
        Box::new(Handler {}),
    )
}
