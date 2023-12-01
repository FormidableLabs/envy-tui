use std::io::Stdout;
use std::ops::Deref;

use ratatui::prelude::{Alignment, Constraint, CrosstermBackend, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Cell, Row, Table};
use ratatui::Frame;

use crate::components::home::Home;
use crate::render::{get_border_style, get_row_style, get_text_style, RowStyle};
use crate::services::websocket::Trace;
use crate::utils::TraceSort;

pub struct GraphQLPlugin;

pub trait Plugin {
    fn is_match(&self, trace: &Trace) -> bool;

    fn render_request_body(
        &self,
        app: &Home,
        trace: &Trace,
        frame: &mut Frame<CrosstermBackend<Stdout>>,
        area: Rect,
    );
}

impl GraphQLPlugin {
    pub fn new() -> Self {
        GraphQLPlugin {}
    }
}

impl Plugin for GraphQLPlugin {
    fn is_match(&self, trace: &Trace) -> bool {
        trace.graphql.is_some()
    }

    fn render_request_body(
        &self,
        app: &Home,
        trace: &Trace,
        frame: &mut Frame<CrosstermBackend<Stdout>>,
        area: Rect,
    ) {
    }
}
