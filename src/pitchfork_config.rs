/// `pitchfork`-specific configuration options, in addition to the configuration
/// options in `haybale::Config`.
///
/// Like `haybale::Config`, `PitchforkConfig` uses the (new-to-Rust-1.40)
/// `#[non_exhaustive]` attribute to indicate that fields may be added even in a
/// point release (that is, without incrementing the major or minor version).
/// Users should start with `PitchforkConfig::default()` and then change the
/// settings they want to change.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct PitchforkConfig {
    /// See notes on the [`KeepGoing`](../enum.KeepGoing.html) enum.
    ///
    /// Default is `KeepGoing::Stop`.
    pub keep_going: KeepGoing,

    /// Regardless of the setting of `keep_going`, the `Display` impl for
    /// `FunctionResult` only displays a summary of the kinds of
    /// errors encountered, and full details about a single error.
    /// With `dump_errors == true`, `pitchfork` will dump detailed descriptions
    /// of all errors encountered to a file.
    ///
    /// If `keep_going == KeepGoing::Stop`, then this setting is completely
    /// ignored (treated as `false` regardless of its actual value).
    ///
    /// Default is `true`.
    pub dump_errors: bool,

    /// If `true`, `pitchfork` will dump detailed coverage stats for the analysis
    /// to a file.
    ///
    /// Default is `true`.
    pub dump_coverage_stats: bool,

    /// If `true`, `pitchfork` will provide detailed progress updates in a
    /// continuously-updated terminal display. This includes counts of paths
    /// verified / errors encountered / warnings generated; the current code
    /// location being executed (in terms of both LLVM and source if available),
    /// the most recent log message generated, etc.
    ///
    /// Also, if `true`, log messages other than the most recent will not be
    /// shown in the terminal; instead, all log messages will be routed to a
    /// file.
    ///
    /// `progress_updates == true` requires `pitchfork` to take control of the
    /// global logger; users should not initialize their own logging backends
    /// such as `env_logger`.
    /// On the other hand, if `progress_updates == false`, `pitchfork` will not
    /// touch the global logger, and it is up to users to initialize a logging
    /// backend such as `env_logger` if they want to see log messages.
    ///
    /// This setting requires the `progress-updates` crate feature, which is
    /// enabled by default. If the `progress-updates` feature is disabled, this
    /// setting will be treated as `false` regardless of its actual value.
    ///
    /// If you encounter a Rust panic (as opposed to merely a `haybale::Error`),
    /// you may want to temporarily disable `progress_updates` for debugging, in
    /// order to get a clear panic message; otherwise, the
    /// progress-display-updater thread may interfere with the printing of the
    /// panic message.
    ///
    /// Default is `true`.
    pub progress_updates: bool,

    /// If `progress_updates == true`, `pitchfork` takes control of the global
    /// logger, as noted in docs there.
    /// This setting controls which log messages will be recorded in the
    /// designated log file: messages with `DEBUG` and higher priority (`true`),
    /// or only messages with `INFO` and higher priority (`false`).
    ///
    /// If `progress_updates == false`, this setting has no effect; you should
    /// configure debug logging via your own chosen logging backend such as
    /// `env_logger`.
    ///
    /// Default is `false`.
    pub debug_logging: bool,
}

impl Default for PitchforkConfig {
    fn default() -> Self {
        Self {
            keep_going: KeepGoing::Stop,
            dump_errors: true,
            dump_coverage_stats: true,
            progress_updates: true,
            debug_logging: false,
        }
    }
}

/// Enum for the `keep_going` option in `PitchforkConfig`
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeepGoing {
    /// Stop at the first error encountered (and return the results we have).
    ///
    /// For constant-time violations, finishes the current path until either it
    /// ends or we get a hard error, then stops. This can result in multiple
    /// constant-time violations reported.
    Stop,
    /// Stop at the first error or constant-time violation on each path,
    /// but continue exploring other paths, potentially finding many errors
    /// and/or violations.
    ///
    /// It is recommended to only use this in conjunction with solver query
    /// timeouts; see the `solver_query_timeout` setting in `Config`.
    ///
    /// This functionality is _currently not working_: this option currently
    /// behaves the same as `Full`.
    StopPerPath,
    /// Like `StopPerPath`, but also don't stop at constant-time violations.
    /// (We still have to stop at errors.)
    /// This allows us to find subsequent constant-time violations (or errors)
    /// even on the same path where we've already found a violation.
    /// The analysis loses some soundness, and finding violations after the first
    /// on each path is on a best-effort basis.
    ///
    /// It is recommended to only use this in conjunction with solver query
    /// timeouts; see the `solver_query_timeout` setting in `Config`.
    Full,
}
