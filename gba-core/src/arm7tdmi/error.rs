use thiserror::Error;

#[derive(Error, Debug)]
pub enum CpuError {
    #[error("CPU is in a paused state")]
    CpuPaused,
    #[error("Interrupt has been triggered")]
    InterruptTriggered,
    #[error("Instruction could not be decoded")]
    FailedToDecode,
    #[error("CPU has nothing to do")]
    NothingToDo,
}
