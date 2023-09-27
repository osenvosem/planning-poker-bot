use crate::constants::{ISSUE_ID_REGEX, URL_REGEX};
use regex::{Match, Regex};

pub fn extract_issue_id(url: &str) -> Option<Match> {
    let re = Regex::new(ISSUE_ID_REGEX).unwrap();
    re.find(url)
}

pub fn is_url_valid(url: &str) -> bool {
    Regex::new(URL_REGEX).unwrap().is_match(url)
}

pub fn parse_title_and_description(payload: &String) -> (String, String) {
    let mut str_iter = payload.splitn(2, "\n");
    let iter_count = str_iter.clone().count();

    let (title, description) = if iter_count == 1 {
        (str_iter.next().unwrap(), "")
    } else {
        (str_iter.next().unwrap(), str_iter.next().unwrap())
    };

    (title.to_string(), description.to_string())
}

pub fn make_username_line(first_name: &String, last_name: &String, username: &String) -> String {
    let mut line = String::from(first_name);

    if !last_name.is_empty() {
        line.push_str(format!(" {}", last_name).as_str())
    }

    if !username.is_empty() {
        line.push_str(format!(" (@{})", username).as_str())
    }

    line
}
