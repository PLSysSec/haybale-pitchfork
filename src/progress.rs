#![cfg(feature = "progress-updates")]

use crate::{ConstantTimeResultForPath, PathStatistics};
use colored::*;
use crossterm::{QueueableCommand, cursor, terminal};
use haybale::{Config, Result, State};
use haybale::backend::Backend;
use std::convert::TryInto;
use std::io::{Write, stdout, Stdout};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use log4rs::encode::pattern::PatternEncoder;
use log4rs::encode::Encode;
use log4rs::encode::writer::simple::SimpleWriter;

// Layout:

// Progress on <funcname>:
//
// <------path statistics block----->
// < rows_of_stats rows (minimum 2) >
// <-------------------------------->
// <blank line>
// backtrack points remaining:
// <blank line>
// warnings generated:
// Most recent log message (INFO or higher):
// <---------log message---------->
// < rows_of_log rows (minimum 1) >
// <------------------------------>
// <blank line>
// Currently at location:
// <---Location block (includes LLVM and src location)--->
// <            rows_of_loc rows (minimum 1)             >
// <----------------------------------------------------->
// <blank line>
// Time elapsed:
// <blank line>    <-- cursor is always on this line at entry/exit of any function in this module

struct ProgressDisplayState {
    /// Receiver for the channel where we will receive ordinary progress messages
    rx: mpsc::Receiver<ProgressMsg>,
    /// Receiver for the channel where we will receive termination messages.
    /// Receiving a message on this channel indicates that the entire analysis is
    /// done, and tells the progress-display-updater thread to reset the display
    /// and terminate itself, ignoring any remaining messages in the ordinary
    /// channel.
    termination_rx: mpsc::Receiver<()>,
    /// `PathStatistics` where we accumulate the statistics on the paths we've
    /// finished
    path_stats: PathStatistics,
    /// current number of backtrack points remaining
    backtrack_points_remaining: usize,
    /// current number of warnings generated
    warnings_generated: usize,
    /// the number of console rows taken up by the most recent log message.
    /// Starts at 1 and is monotonically increasing - we don't reclaim lines
    /// after replacing a long message with a short one.
    rows_of_log: u16,
    /// the number of console rows taken up by the path-statistics block.
    /// Starts at 2 and is monotonically increasing, just due to the logic in
    /// the `Display` impl for `PathStatistics`.
    rows_of_stats: u16,
    /// the number of console rows taken up by the current location
    rows_of_loc: u16,
    /// we assume this doesn't change (if it does, we have bigger problems than this)
    terminal_cols: u16,
    /// time at which the operation started (defined as when the progress UI was
    /// initialized - so this won't include creating the Project)
    start_time: Instant,
    /// elapsed time since start, or at least, what is currently being displayed
    /// as the elapsed time since start
    elapsed_secs: u64,
}

/// The various messages which can be sent from the main thread to the
/// progress-display-updater thread
enum ProgressMsg {
    /// a progress update (i.e., new location), with the LLVM and source
    /// locations already formatted into `String`s. This means the formatting has
    /// to happen on the main thread; someday maybe we can do the formatting on
    /// the progress-display-updater thread
    ProgressUpdate {
        llvm_location: String,
        src_location: String,
        num_backtrack_points: usize,
    },
    /// a log message, already fully formatted into bytes ready to dump to stdout.
    /// This means the formatting has to happen on the main thread; someday maybe
    /// we can do the formatting on the progress-display-updater thread
    LogMessage {
        msg: Vec<u8>,
        level: log::Level,
    },
    /// we completed a path. The progress-display-updater thread really only
    /// requires a read-only reference to the path result, but for now we have an
    /// owned one here. Note that it should be fairly quick to `clone()` on the
    /// main thread, and this message is infrequent compared to the other messages.
    PathCompleted(ConstantTimeResultForPath),
}

impl ProgressDisplayState {
    /// Initialize the progress display
    fn initialize(rx: mpsc::Receiver<ProgressMsg>, termination_rx: mpsc::Receiver<()>, funcname: &str) -> Self {
        let mut pdstate = Self {
            rx,
            termination_rx,
            path_stats: PathStatistics::new(),
            backtrack_points_remaining: 0,
            warnings_generated: 0,
            rows_of_log: 1,
            rows_of_stats: 2,
            rows_of_loc: 1,
            terminal_cols: {
                let (cols, _) = terminal::size().unwrap();
                cols
            },
            start_time: Instant::now(),
            elapsed_secs: 0,
        };
        println!("Progress on {}:", funcname);
        println!();
        print!("{}", pdstate.path_stats);  // the `Display` impl here includes the final newline
        println!();  // this is the blank line between the path stats and the backtrack-points line
        pdstate.print_backtrack_line();
        println!();
        pdstate.print_warnings_line();
        println!("Most recent log message (INFO or higher):");
        println!("  <no messages yet>");
        println!();
        println!("Currently at location:");
        {
            // 'allocate' a blank line to get things scrolled to the right place, then return to the line we want
            println!();
            stdout().queue(cursor::MoveToPreviousLine(1)).unwrap();
        }
        let (_, loc_start_row) = cursor::position().unwrap();
        println!("  <allocating/initializing global variables and function arguments>");
        let loc_end_row = {
            let (_, blank_line_row) = cursor::position().unwrap();
            blank_line_row - 1
        };
        pdstate.rows_of_loc = loc_end_row - loc_start_row + 1;
        println!();  // this is the blank line between the location block and the time-elapsed line
        pdstate.print_time_elapsed_line();
        // the terminal cursor will be on the appropriate line when we're done: the blank line below the time-elapsed line
        pdstate
    }

    /// The main progress-display-updater loop, where we listen for progress
    /// updates from the main thread and refresh the display accordingly
    fn listen(&mut self) {
        loop {
            if self.termination_rx.try_recv().is_ok() {
                self.finalize();
                break
            } else if let Ok(msg) = self.rx.try_recv() {
                self.handle_msg(msg)
            } else {
                thread::sleep(Duration::from_millis(5))  // wait 5 ms before checking for a new message
            }

            self.update_elapsed_time();
        }
    }

    fn handle_msg(&mut self, msg: ProgressMsg) {
        match msg {
            ProgressMsg::ProgressUpdate { llvm_location, src_location, num_backtrack_points } => {
                // if the next message is:
                //   (1) already available, and
                //   (2) another progress update,
                // then skip this progress update and go right to the next one
                match self.rx.try_recv() {
                    Ok(new_update@ProgressMsg::ProgressUpdate { .. }) => {
                        // next message is a ProgressUpdate, so drop this one
                        // and handle that one instead
                        self.handle_msg(new_update);
                    },
                    Ok(other_msg) => {
                        // next message is not a ProgressUpdate
                        // so handle this message and then the next, in the ordinary way
                        self.update_progress(&llvm_location, &src_location, num_backtrack_points);
                        self.handle_msg(other_msg);
                    },
                    _ => {
                        // next message is not available yet
                        // so handle this message in the ordinary way
                        self.update_progress(&llvm_location, &src_location, num_backtrack_points);
                    },
                }
            },
            ProgressMsg::LogMessage { msg, level } => {
                match self.rx.try_recv() {
                    Ok(new_logmsg@ProgressMsg::LogMessage { .. }) => {
                        // next message is also a LogMessage, so don't bother
                        // handling/printing this one, except to update the
                        // warning count if necessary.
                        if level <= log::Level::Warn {
                            self.increment_and_update_warning_count();
                        }
                        // then handle the next message
                        self.handle_msg(new_logmsg);
                    },
                    Ok(other_msg) => {
                        // next message is not a LogMessage
                        // so handle this message and then the next, in the ordinary way
                        self.process_log_message(&msg, level);
                        self.handle_msg(other_msg);
                    },
                    _ => {
                        // next message is not available yet
                        // so handle this message in the ordinary way
                        self.process_log_message(&msg, level);
                    },
                }
            },
            ProgressMsg::PathCompleted(ctresult) => self.process_path_result(&ctresult),
        }
    }

    /// prints the backtrack-points-remaining line and moves cursor to the next line
    fn print_backtrack_line(&self) {
        println!("backtrack points remaining: {}", self.backtrack_points_remaining);
    }

    /// prints the warnings-generated line and moves cursor to the next line
    fn print_warnings_line(&self) {
        print!("warnings generated: ");
        if self.warnings_generated > 0 {
            println!("{} (see detailed logs)", self.warnings_generated.to_string().yellow())
        } else {
            println!("0");
        }
    }

    /// prints the time-elapsed line and moves cursor to the next line
    fn print_time_elapsed_line(&self) {
        print!("Time elapsed: ");
        if self.elapsed_secs < 60 {
            println!("{}s", self.elapsed_secs);
        } else {
            let elapsed_minutes = self.elapsed_secs / 60;
            let secs_remainder = self.elapsed_secs % 60;
            if elapsed_minutes < 60 {
                println!("{}m {}s", elapsed_minutes, secs_remainder);
            } else {
                let elapsed_hours = elapsed_minutes / 60;
                let mins_remainder = elapsed_minutes % 60;
                println!("{}h {}m {}s", elapsed_hours, mins_remainder, secs_remainder);
            }
        }
    }

    fn update_progress(&mut self, llvm_loc: &str, src_loc: &str, num_backtrack_points: usize) {
        let mut stdout = stdout();
        self.update_location(llvm_loc, src_loc, &mut stdout);
        self.update_backtrack_points(num_backtrack_points, &mut stdout);
        stdout.flush().unwrap();
    }

    fn update_location(&mut self, llvm_loc: &str, src_loc: &str, stdout: &mut Stdout) {
        stdout
            .queue(cursor::MoveToPreviousLine(2 + self.rows_of_loc)).unwrap()
            .queue(terminal::Clear(terminal::ClearType::FromCursorDown)).unwrap()
            // that also cleared the time-elapsed line, which we'll repaint
            .queue(cursor::MoveToColumn(0)).unwrap();

        // if the location is too much longer than the previous location, printing
        // it will cause the terminal to scroll, which will throw off the
        // calculation of `self.rows_of_loc`.
        // Thus, as a hack, we calculate the rough number of rows needed, and if
        // it's much larger than the current `rows_of_loc` plus two (for the
        // time-elapsed line), we'll add some blank lines at the bottom first,
        // so that the terminal won't scroll during the actual print operation.
        let llvm_loc_string = format!("LLVM location: {}", llvm_loc);
        let llvm_loc_string_len: u16 = llvm_loc_string.len().try_into().unwrap_or_else(|_| panic!("Unexpectedly large location string length: {} characters", llvm_loc_string.len()));
        let src_loc_string = format!("Source location: {}", src_loc);
        let src_loc_string_len: u16 = src_loc_string.len().try_into().unwrap_or_else(|_| panic!("Unexpectedly large source string length: {} characters", src_loc_string.len()));
        let llvm_rows_needed: u16 = llvm_loc_string_len / self.terminal_cols + 1;
        let src_rows_needed: u16 = src_loc_string_len / self.terminal_cols + 1;
        let rows_needed = llvm_rows_needed + src_rows_needed;
        if rows_needed > self.rows_of_loc + 2 {
            for _ in 0 .. rows_needed {
                println!();
            }
            stdout.queue(cursor::MoveToPreviousLine(rows_needed)).unwrap();
        }

        // now do the actual print
        let (_, loc_start_row) = cursor::position().unwrap();
        println!("{}\n{}", llvm_loc_string, src_loc_string);
        let loc_end_row = {
            let (_, blank_line_row) = cursor::position().unwrap();
            blank_line_row - 1
        };
        let loc_rows = loc_end_row - loc_start_row + 1;
        assert!(loc_rows <= rows_needed);
        self.rows_of_loc = loc_rows;
        println!();
        self.print_time_elapsed_line();
        // that puts us on the blank line after the time-elapsed line, as desired
    }

    fn update_backtrack_points(&mut self, num_backtrack_points: usize, stdout: &mut Stdout) {
        // only update if the number has changed
        if num_backtrack_points != self.backtrack_points_remaining {
            self.backtrack_points_remaining = num_backtrack_points;
            stdout
                .queue(cursor::MoveToPreviousLine(2 + self.rows_of_loc + 2 + self.rows_of_log + 4)).unwrap()  // puts us on the backtrack line
                .queue(terminal::Clear(terminal::ClearType::CurrentLine)).unwrap();
            self.print_backtrack_line();
            stdout.queue(cursor::MoveToNextLine(3 + self.rows_of_log + 2 + self.rows_of_loc + 2)).unwrap();
        }
    }

    fn update_elapsed_time(&mut self) {
        // only update if the number has changed
        let elapsed_secs = self.start_time.elapsed().as_secs();
        if elapsed_secs != self.elapsed_secs {
            self.elapsed_secs = elapsed_secs;
            stdout()
                .queue(cursor::MoveToPreviousLine(1)).unwrap()  // puts us on the time-elapsed line
                .queue(terminal::Clear(terminal::ClearType::CurrentLine)).unwrap();
            self.print_time_elapsed_line();
            // that puts us on the blank line after the time-elapsed line, as desired
        }
    }

    fn process_log_message(&mut self, log_message: &[u8], level: log::Level) {
        let mut stdout = stdout();
        let mut q = stdout
            .queue(cursor::MoveToPreviousLine(2 + self.rows_of_loc + 2)).unwrap();
        for _ in 0 .. self.rows_of_log {
            q = q
                .queue(cursor::MoveToPreviousLine(1)).unwrap()
                .queue(terminal::Clear(terminal::ClearType::CurrentLine)).unwrap();
        }

        // Cursor is currently on the first line of log text, having cleared the previous log message.
        // if this is a WARN or worse, update warnings-generated now, then come back.
        if level <= log::Level::Warn {
            self.warnings_generated += 1;
            stdout
                .queue(cursor::MoveToPreviousLine(2)).unwrap()
                .queue(terminal::Clear(terminal::ClearType::CurrentLine)).unwrap();
            self.print_warnings_line();
            stdout.queue(cursor::MoveToNextLine(1)).unwrap();  // back to the first line of log text
        }

        let (_, log_start_row) = cursor::position().unwrap();
        stdout.write(log_message).unwrap();
        let (_, log_end_row) = cursor::position().unwrap();
        let rows_of_this_log = log_end_row - log_start_row + 1;
        if rows_of_this_log > self.rows_of_log {
            // need to allocate more rows for log messages now
            self.rows_of_log = rows_of_this_log;
            println!();  // move to blank line
            stdout.queue(terminal::Clear(terminal::ClearType::FromCursorDown)).unwrap();
            println!();  // move to the "Currently at location" line
            println!("Currently at location:");
            println!();
            self.rows_of_loc = 1;
            // due to the println!, terminal cursor is now on the blank line between the location block and the time-elapsed line
            println!();
            self.print_time_elapsed_line();
            // that puts us on the blank line after the time-elapsed line, as desired
        } else if rows_of_this_log < self.rows_of_log {
            stdout
                .queue(cursor::MoveToNextLine(self.rows_of_log - rows_of_this_log)).unwrap()
                .queue(cursor::MoveToNextLine(3 + self.rows_of_loc + 2)).unwrap();
        } else {
            stdout.queue(cursor::MoveToNextLine(3 + self.rows_of_loc + 2)).unwrap();
        }
        stdout.flush().unwrap();
    }

    /// increment and update the warning count, without changing/repainting the
    /// "most recent log message" section
    fn increment_and_update_warning_count(&mut self) {
        self.warnings_generated += 1;
        let mut stdout = stdout();
        stdout
            .queue(cursor::MoveToPreviousLine(2 + self.rows_of_loc + 2 + self.rows_of_log + 2)).unwrap()
            .queue(terminal::Clear(terminal::ClearType::CurrentLine)).unwrap();
        self.print_warnings_line();
        stdout.queue(cursor::MoveToNextLine(1 + self.rows_of_log + 2 + self.rows_of_loc + 2)).unwrap();
    }

    fn process_path_result(&mut self, path_result: &ConstantTimeResultForPath) {
        let mut stdout = stdout();
        let mut q = stdout
            .queue(cursor::MoveToPreviousLine(2 + self.rows_of_loc + 2 + self.rows_of_log + 5)).unwrap();
        for _ in 0 .. self.rows_of_stats {
            q = q
                .queue(cursor::MoveToPreviousLine(1)).unwrap()
                .queue(terminal::Clear(terminal::ClearType::CurrentLine)).unwrap();
        }
        self.path_stats.add_path_result(path_result);
        let (_, stats_start_row) = cursor::position().unwrap();
        print!("{}", self.path_stats);  // the `Display` impl here includes the final newline
        let stats_end_row = {
            // we ended on the blank line from the print, so the last row of stats is one above the current cursor position
            let (_, blank_line_row) = cursor::position().unwrap();
            blank_line_row - 1
        };
        let new_rows_of_stats = stats_end_row - stats_start_row + 1;
        if new_rows_of_stats > self.rows_of_stats {
            // need to allocate another row for stats now
            // unfortunately that means we have to temporarily wipe some of the other progress indicators, they'll be refreshed soon enough
            self.rows_of_stats = new_rows_of_stats;
            stdout.queue(terminal::Clear(terminal::ClearType::FromCursorDown)).unwrap();
            println!();
            self.print_backtrack_line();
            println!();
            self.print_warnings_line();
            println!("Most recent log message (INFO or higher):");
            for _ in 0 .. self.rows_of_log {
                println!();
            }
            println!();
            println!("Currently at location:");
            println!("  <just finished a path>");
            self.rows_of_loc = 1;
            // due to the println!, terminal cursor is now on the blank line between the location block and the time-elapsed line
            println!();
            self.print_time_elapsed_line();
        } else if new_rows_of_stats == self.rows_of_stats {
            stdout.queue(cursor::MoveToNextLine(5 + self.rows_of_log + 2 + self.rows_of_loc + 2)).unwrap();
        } else {
            panic!("rows_of_stats decreased, which we didn't think should ever happen");
        }
        stdout.flush().unwrap();
    }

    fn finalize(&mut self) {
        stdout()
            .queue(cursor::MoveToPreviousLine(2 + self.rows_of_loc + 2 + self.rows_of_log + 5 + self.rows_of_stats + 2)).unwrap()
            .queue(terminal::Clear(terminal::ClearType::FromCursorDown)).unwrap()
            .flush().unwrap();
        self.print_time_elapsed_line();
        println!();
    }
}

// The `MainThreadState` is stored in a thread-local static so that the main
// thread can grab a reference to it even inside the callback from `haybale` and
// inside the `log4rs` appender.
// The `Rc` is used so that we can have some functions in this module grab a
// reference to it from here, while the caller code in lib.rs can also have an
// owned reference.
//
// The inner value will only be `None` until `MainThreadState::initialize()` is
// called; after that it will always be `Some`.
use std::cell::RefCell;
use std::rc::Rc;
thread_local! {
    static MAIN_THREAD_STATE: Rc<RefCell<Option<MainThreadState>>> = Rc::new(RefCell::new(None));
}

/// `MainThreadState` just contains the state which the main thread needs in
/// order to communicate with the progress-display-updater thread, which does the
/// actual work (and stores the state it needs in `ProgressDisplayState`)
pub struct MainThreadState {
    tx: mpsc::Sender<ProgressMsg>,
    termination_tx: mpsc::Sender<()>,
    /// join handle is `Some` while the progress-display-updater thread is alive,
    /// or `None` if the thread is not alive (either hasn't been started, or has
    /// been joined)
    display_thread_join_handle: Option<thread::JoinHandle<()>>,
    /// we store this here just so we only have to construct it once
    encoder: PatternEncoder,
}

impl MainThreadState {
    fn initialize<B: Backend>(log_filename: &str, funcname: impl Into<String>, config: &mut Config<B>, debug_logging: bool) -> Rc<RefCell<Option<Self>>> {
        // spawn the progress-display-updater thread, which will initialize the progress-display view
        let (tx, rx) = mpsc::channel();
        let (termination_tx, termination_rx) = mpsc::channel();
        let funcname = funcname.into();
        let join_handle = thread::spawn(move || {
            let mut pdstate = ProgressDisplayState::initialize(rx, termination_rx, &funcname);
            pdstate.listen();
        });

        // add our callbacks used to update progress indicators
        config.callbacks.add_instruction_callback(update_progress_inst);
        config.callbacks.add_terminator_callback(update_progress_term);

        // direct log messages to dedicated log file, if we haven't done that yet
        // (e.g. during a previous call to this function)
        if log::log_enabled!(log::Level::Error) {
            println!("Logging was already initialized, so detailed logs for this run will be available wherever they were previously initialized to.\n");
        } else {
            println!("\nDetailed logs for this run are available at {}.\nYou may run `tail -f` on this file in a separate terminal.\n", log_filename);
            crate::logging::init(log_filename, debug_logging);
        }

        let updater = Self {
            tx,
            termination_tx,
            display_thread_join_handle: Some(join_handle),
            encoder: PatternEncoder::new("{d(%Y-%m-%d %H:%M:%S)} [{l}] {m}"),
        };
        MAIN_THREAD_STATE.with(|mts| {
            *mts.borrow_mut() = Some(updater);
            mts.clone()
        })
    }
}

impl<B: Backend> crate::ProgressUpdater<B> for MainThreadState {
    fn update_progress(&self, state: &State<B>) -> Result<()> {
        self.tx.send(ProgressMsg::ProgressUpdate {
            llvm_location: state.cur_loc.to_string_short_module(),
            src_location: if let Some(debugloc) = state.cur_loc.source_loc {
                debugloc.to_string()
            } else {
                "<unknown>".into()
            },
            num_backtrack_points: state.count_backtracking_points(),
        }).unwrap();
        Ok(())
    }

    fn update_path_result(&self, path_result: &ConstantTimeResultForPath) {
        self.tx.send(ProgressMsg::PathCompleted(path_result.clone())).unwrap();
    }

    fn process_log_message(&self, record: &log::Record) -> anyhow::Result<()> {
        let mut formatted_msg = Vec::new();
        self.encoder.encode(&mut SimpleWriter(&mut formatted_msg), record)?;
        self.tx.send(ProgressMsg::LogMessage {
            msg: formatted_msg,
            level: record.level(),
        }).unwrap();
        Ok(())
    }

    fn finalize(&mut self) {
        self.termination_tx.send(()).unwrap();
        self.display_thread_join_handle.take().unwrap().join().expect("Failed to join with progress-display-updater thread");
    }
}

fn update_progress_inst<B: Backend>(_inst: &llvm_ir::Instruction, state: &State<B>) -> Result<()> {
    update_progress(state)
}

fn update_progress_term<B: Backend>(_term: &llvm_ir::Terminator, state: &State<B>) -> Result<()> {
    update_progress(state)
}

fn update_progress<B: Backend>(state: &State<B>) -> Result<()> {
    MAIN_THREAD_STATE.with(|mts| {
        let mut guard = mts.borrow_mut();
        let mts: &mut MainThreadState = guard.as_mut().expect("process_log_message: expected a MainThreadState to exist");
        <MainThreadState as crate::ProgressUpdater::<B>>::update_progress(mts, state)
    })
}

pub fn process_log_message(record: &log::Record) -> anyhow::Result<()> {
    MAIN_THREAD_STATE.with(|mts| {
        let mut guard = mts.borrow_mut();
        let mts: &mut MainThreadState = guard.as_mut().expect("process_log_message: expected a MainThreadState to exist");
        <MainThreadState as crate::ProgressUpdater::<crate::secret::Backend>>::process_log_message(mts, record)
    })
}

/// As a convenience, we provide wrappers which implement the ProgressUpdater
/// trait for Rc<RefCell<Option<MainThreadState>>>
pub type ProgressUpdater = Rc<RefCell<Option<MainThreadState>>>;

pub fn initialize_progress_updater<B: Backend>(log_filename: &str, funcname: &str, config: &mut Config<B>, debug_logging: bool) -> ProgressUpdater {
    MainThreadState::initialize(log_filename, funcname, config, debug_logging)
}

impl<B: Backend> crate::ProgressUpdater<B> for ProgressUpdater {
    fn update_progress(&self, state: &State<B>) -> Result<()> {
        let guard = self.borrow();
        let mts: &MainThreadState = guard.as_ref().unwrap();
        <MainThreadState as crate::ProgressUpdater::<B>>::update_progress(mts, state)
    }

    fn update_path_result(&self, path_result: &ConstantTimeResultForPath) {
        let guard = self.borrow();
        let mts: &MainThreadState = guard.as_ref().unwrap();
        <MainThreadState as crate::ProgressUpdater::<B>>::update_path_result(mts, path_result)
    }

    fn process_log_message(&self, record: &log::Record) -> anyhow::Result<()> {
        let guard = self.borrow();
        let mts: &MainThreadState = guard.as_ref().unwrap();
        <MainThreadState as crate::ProgressUpdater::<B>>::process_log_message(mts, record)
    }

    fn finalize(&mut self) {
        let mut guard = self.borrow_mut();
        let mts: &mut MainThreadState = guard.as_mut().unwrap();
        <MainThreadState as crate::ProgressUpdater::<B>>::finalize(mts)
    }
}
