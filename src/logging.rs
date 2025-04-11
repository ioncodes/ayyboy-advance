use std::sync::Arc;

use crate::Args;
use spdlog::formatter::{pattern, PatternFormatter};
use spdlog::sink::{FileSink, StdStream, StdStreamSink};
use spdlog::{self, Level, LevelFilter, Logger};

pub fn enable_logger(args: &Args) {
    let logger = if args.trace {
        let path = "trace.log";
        let _ = std::fs::remove_file(&path);
        let file_sink = Arc::new(FileSink::builder().path(path).build().unwrap());
        let logger = Arc::new(Logger::builder().sink(file_sink).build().unwrap());
        logger.set_level_filter(LevelFilter::All);
        logger
    } else if args.debug {
        let path = "debug.log";
        let _ = std::fs::remove_file(&path);
        let file_sink = Arc::new(FileSink::builder().path(path).build().unwrap());
        let logger = Arc::new(Logger::builder().sink(file_sink).build().unwrap());
        logger.set_level_filter(LevelFilter::Equal(Level::Debug));
        logger
    } else {
        let std_sink = Arc::new(StdStreamSink::builder().std_stream(StdStream::Stderr).build().unwrap());
        let logger = Arc::new(Logger::builder().sink(std_sink).build().unwrap());
        logger.set_level_filter(LevelFilter::MoreSevereEqual(Level::Info));
        logger
    };

    let formatter = Box::new(PatternFormatter::new(pattern!(
        "[{^{level}} {module_path}] {payload}{eol}"
    )));

    for sink in logger.sinks() {
        sink.set_formatter(formatter.clone());
    }

    spdlog::set_default_logger(logger);
}
