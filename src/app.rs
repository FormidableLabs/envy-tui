use std::fmt::Display;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ActiveBlock {
    NetworkRequests,
    RequestDetails,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Mode {
    Insert,
    Normal,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Request {
    pub id: String,
    pub method: http::method::Method,
    pub status: http::status::StatusCode,
    pub uri: String,
    pub time: u32,
}

impl Display for Request {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Method used: {:?}, response status is {:?}, time took: {:?} miliseconds.",
            self.method, self.status, self.time
        )
    }
}

pub struct App {
    pub active_block: ActiveBlock,
    pub mode: Mode,
    pub requests: Vec<Request>,
    pub selection_index: usize,
}

impl App {
    pub fn new() -> App {
        App {
            active_block: ActiveBlock::NetworkRequests,
            mode: Mode::Normal,
            selection_index: 0,
            requests: vec![
                Request {
                    status: http::StatusCode::OK,
                    method: http::method::Method::GET,
                    id: String::from("id"),
                    uri: String::from("https://randomdomain.com/randompath"),
                    time: 234524,
                },
                Request {
                    method: http::method::Method::POST,
                    status: http::StatusCode::CREATED,
                    id: String::from("id2"),
                    uri: String::from("https://randomdomain.com/randompath"),
                    time: 234511,
                },
                Request {
                    method: http::method::Method::POST,
                    status: http::StatusCode::CREATED,
                    id: String::from("id3"),
                    uri: String::from("https://randomdomain.com/randompath/nestedPath"),
                    time: 1111,
                },
            ],
        }
    }
}
