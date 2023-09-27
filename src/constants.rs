// pub const DEFAULT_SEQ: [&'static str; 11] =
//     ["0", "0.5", "1", "2", "3", "5", "8", "13", "20", "40", "100"];
pub const DEFAULT_SEQ: [&'static str; 11] =
    ["0", "1", "2", "3", "5", "8", "13", "21", "34", "55", "89"];
pub const FUNC_BUTTONS: [(&'static str, &'static str); 2] =
    [("Перезапустить", "restart"), ("Завершить", "finish")];

pub const URL_REGEX: &'static str =
    r"https?://(www\.)?[-a-zA-Z0-9@:%._\+~#=]{2,256}\.[a-z]{2,4}\b([-a-zA-Z0-9@:%_\+.~#?&//=]*)";

pub const ISSUE_ID_REGEX: &'static str = r"[A-Z]+-\d+";

pub const EMOJI_SET: [&'static str; 4] = ["♦️", "♠️", "♣️", "♥️"];
