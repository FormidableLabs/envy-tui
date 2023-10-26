use crossterm::event::KeyEvent;

#[derive(Clone, Debug)]
pub enum Action {
    CopyToClipBoard,
    NavigateLeft(KeyEvent),
    NavigateDown(KeyEvent),
    NavigateUp(KeyEvent),
    NavigateRight(KeyEvent),
    GoToEnd,
    GoToStart,
    NextSection,
    PreviousSection,
    Quit,
    NewSearch,
    UpdateSearchQuery(char),
    DeleteSearchQuery,
    ExitSearch,
    Help,
    ToggleDebug,
    DeleteItem,
    FocusOnTraces,
    ShowTraceDetails,
    NextPane,
    PreviousPane,
}
