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

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum HttpMethod {
    POST,
    GET,
    DELETE,
    OPTION,
    PATCH,
    PUT,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum HttpStatus {
    OK,
    CREATED,
    NOT_FOUND,
}

impl Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Display for HttpStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Request {
    pub id: String,
    pub method: HttpMethod,
    pub status: HttpStatus,
    pub uri: String,
    pub time: u32,
}

impl Display for Request {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:?}, {:?}",
            self.method,
            match self.status {
                HttpStatus::OK => "OK - 200",
                HttpStatus::CREATED => "CREATED - 201",
                HttpStatus::NOT_FOUND => "NOT_FOUND - 404",
            }
        )
    }
}

pub struct App {
    pub active_block: ActiveBlock,
    pub mode: Mode,
    pub requests: Vec<Request>,
    pub selected_request: String,
}

// "1.58s",
impl App {
    pub fn new() -> App {
        App {
            selected_request: "id".to_string(),
            active_block: ActiveBlock::NetworkRequests,
            mode: Mode::Normal,
            requests: vec![Request {
                method: HttpMethod::GET,
                status: HttpStatus::OK,
                id: String::from("id"),
                uri: String::from("https://randomdomain.com/randompath"),
                time: 234524,
            }],
        }
    }
}
