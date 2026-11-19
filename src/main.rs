//! The `forge` program: the command line in `cli::run_from`, on this
//! process's own streams and terminal.

use mollendorff_forge::cli::{self, commands::terminal_width, Console};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args_os().collect();

    // `forge functions | head` must end quietly, like other command-line tools.
    // The servers keep Rust's default, so a client that disconnects is an
    // error on that connection, not the end of the process.
    #[cfg(unix)]
    if !cli::starts_a_server(&args) {
        // SAFETY: restoring the default action for SIGPIPE before any output.
        unsafe {
            libc::signal(libc::SIGPIPE, libc::SIG_DFL);
        }
    }

    // Not locked for the whole run: `forge mcp` serves on the same streams,
    // and a lock held here would block it. Each write locks, as `println!` does.
    let (mut out, mut err) = (std::io::stdout(), std::io::stderr());
    let mut input = std::io::BufReader::new(std::io::stdin());
    let mut console = Console::new(&mut out, &mut err, &mut input);
    console.width = terminal_width();
    let colours = |choice| choice == anstream::ColorChoice::Always;
    console.color_out = colours(anstream::AutoStream::choice(&std::io::stdout()));
    console.color_err = colours(anstream::AutoStream::choice(&std::io::stderr()));
    ExitCode::from(cli::run_from(args, &mut console))
}
