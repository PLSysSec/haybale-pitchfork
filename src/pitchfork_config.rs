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
    /// If `true`, then even if we encounter an error or violation, we will
    /// continue exploring as many paths as we can in the function before
    /// returning, possibly reporting many different errors and/or violations.
    /// (Although we can't keep going on the errored path itself, we can still try to
    /// explore other paths that don't contain the error.)
    /// If `false`, then as soon as we encounter an error or violation, we will quit
    /// and return the results we have.
    /// It is recommended to only use `keep_going == true` in conjunction with solver
    /// query timeouts; see the `solver_query_timeout` setting in `Config`.
    ///
    /// Default is `false`.
    pub keep_going: bool,

    /// Even if `keep_going` is set to `true`, the `Display` impl for
    /// `ConstantTimeResultForFunction` only displays a summary of the kinds of
    /// errors encountered, and full details about a single error.
    /// With `dump_errors == true`, `pitchfork` will dump detailed descriptions
    /// of all errors encountered to a file.
    ///
    /// This setting only applies if `keep_going == true`; it is completely ignored
    /// if `keep_going == false`.
    ///
    /// Default is `true`, meaning that if `keep_going` is enabled, then detailed
    /// error descriptions will be dumped to a file.
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
            keep_going: false,
            dump_errors: true,
            dump_coverage_stats: true,
            progress_updates: true,
            debug_logging: false,
        }
    }
}
