//! Quick analytical probe: when does each canonical substrate's
//! stream actually run out of events? Counts events and reports
//! the last-event tick per substrate.

use relatum_v2::{
    runtime::Event,
    test_substrates::{
        long5k::build_5k_stream, narrow_a::build_narrow_a_stream,
        oq1::build_long_stream, oq2::build_oq2_stream,
    },
};

fn analyze(label: &str, stream: Vec<(u64, Event)>) {
    let count = stream.len();
    let last_tick = stream.iter().map(|(t, _)| *t).max().unwrap_or(0);
    let first_tick = stream.iter().map(|(t, _)| *t).min().unwrap_or(0);
    println!(
        " {:<10} {:>8} events, ticks {}..{} (span {})",
        label, count, first_tick, last_tick, last_tick - first_tick + 1,
    );
}

fn main() {
    println!("════════════════════════════════════════════════════════");
    println!(" Stream lifecycle — when do substrates dry up?");
    println!("════════════════════════════════════════════════════════");
    println!();
    println!(" Each substrate has a finite, deterministic event list.");
    println!(" After the last event tick, the runtime sees nothing");
    println!(" further from the environment.");
    println!();
    println!(" {:<10} {:>8} {:>10}", "substrate", "events", "last tick");
    analyze("OQ#1", build_long_stream());
    analyze("long5k", build_5k_stream());
    analyze("narrow_a", build_narrow_a_stream());
    analyze("OQ#2", build_oq2_stream());

    println!();
    println!(" None of v2's canonical substrates are infinite.");
    println!(" SyntheticStreamEnvironment takes Vec<(u64, Event)> by");
    println!(" construction — once exhausted, no further events.");

    println!();
    println!("--- end ---");
}
