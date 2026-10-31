use clap::{Arg, ArgAction, Command};
fn value(name: &'static str, default: Option<&'static str>) -> Arg {
    let arg = Arg::new(name).long(name).global(true).num_args(1);
    if let Some(value) = default {
        arg.default_value(value)
    } else {
        arg
    }
}
pub fn command() -> Command {
    let mut c = Command::new("dot-logger")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Bounded Rust logger client; explicit remote or immutable offline session")
        .subcommand_required(true)
        .arg(
            Arg::new("json")
                .long("json")
                .global(true)
                .action(ArgAction::SetTrue)
                .help("Versioned JSON lines"),
        )
        .arg(
            Arg::new("include-payload")
                .long("include-payload")
                .global(true)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("originals")
                .long("originals")
                .global(true)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("disconnect-on-overflow")
                .long("disconnect-on-overflow")
                .global(true)
                .action(ArgAction::SetTrue),
        );
    for (name, default) in [
        ("endpoint", None),
        ("manifest", None),
        ("ca", None),
        ("tls-domain", None),
        ("token-env", Some("DOT_LOGGER_TOKEN")),
        ("timeout-ms", Some("5000")),
        ("max-output-bytes", Some("16777216")),
        ("message-bytes", Some("263168")),
        ("limit", Some("1000")),
        ("from", Some("0")),
        ("to", Some("18446744073709551616")),
        ("basis", Some("mono")),
        ("clock-domain", None),
        ("channel", None),
        ("level", Some("trace")),
        ("snapshot", Some("0")),
        ("cursor", Some("0")),
        ("handle", Some("0")),
        ("generation", Some("0")),
        ("speed", Some("1/1")),
        ("action", Some("start")),
        ("records", Some("1")),
        ("record-bytes", Some("8388608")),
        ("age-ms", Some("10000")),
        ("external-bytes", Some("134217728")),
        ("offset", Some("0")),
        ("length", Some("4096")),
        ("blob", Some("0")),
        ("session", None),
        ("producer", Some("0")),
        ("sequence", None),
        ("target", Some("1")),
        ("boundary", Some("written")),
        ("patch", None),
        ("base-revision", Some("0")),
        ("plan", Some("0")),
        ("before", None),
        ("after", None),
        ("output", None),
    ] {
        let arg = value(name, default);
        c = c.arg(if matches!(name, "from" | "to") {
            arg.allow_hyphen_values(true)
        } else {
            arg
        });
    }
    for (name, help) in [
        (
            "status",
            "Session identity and explicitly unavailable metrics",
        ),
        ("query", "Bounded time/channel query"),
        ("replay", "Remote replay transition or local timed playback"),
        (
            "tail",
            "Bounded subscription; ACK only after stdout handoff",
        ),
        ("blob", "Remote capability range read"),
        ("completion", "Observe a receipt's completion boundary"),
        ("compare", "Compare host-owned report capabilities"),
        (
            "recover",
            "Recover an immutable source into an independent new file",
        ),
    ] {
        c = c.subcommand(Command::new(name).about(help));
    }
    c.subcommand(
        Command::new("config")
            .subcommand_required(true)
            .subcommand(
                Command::new("validate")
                    .about("Validate an offline session declaration and its memory budgets"),
            )
            .subcommand(
                Command::new("diff")
                    .about("Compare two bounded declarations without exposing their values"),
            )
            .subcommand(Command::new("apply").about("Post a bounded patch to the host")),
    )
    .subcommand(
        Command::new("check")
            .subcommand_required(true)
            .subcommand(Command::new("run").about("Run a pre-registered host plan"))
            .subcommand(
                Command::new("plan").about("Requires the later host check-planning backend"),
            ),
    )
}
