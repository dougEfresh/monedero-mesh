use serde::{Deserialize, Serialize};

use crate::app::Task;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum MsgOut {
    Refresh,
    ClearScreen,
    Debug(String),
    /*
    Call(Command),
    Call0(Command),
    CallSilently(Command),
    CallSilently0(Command),
     */
    EnableMouse,
    DisableMouse,
    ToggleMouse,
    StartFifo(String),
    StopFifo,
    ToggleFifo(String),
    ScrollUp,
    ScrollDown,
    ScrollUpHalf,
    ScrollDownHalf,
    Quit,
    Enqueue(Task),
}
