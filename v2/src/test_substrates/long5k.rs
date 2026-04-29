//! long5k 5-regime substrate stream — used for long-horizon
//! experiments (Phase H2.0 step3b α follow-ups).
//!
//! 5000-tick HORIZON with 5 regimes × 10 phases each:
//! - Regime A (1-990): diamond posets
//! - Regime B (1001-1990): bipartite 2×3
//! - Regime C (2001-2990): clique families (2 + 3)
//! - Regime D (3001-3990): diamonds + PATTERN/ESTABLISHED markers
//! - Regime E (4001-4990): alternating diamond/bipartite/clique to
//!   stress regime-switching

use crate::{ESTABLISHED_MARKER, PATTERN_MARKER, R};
use crate::runtime::Event;

/// Build the canonical long5k stream (5000 ticks).
pub fn build_5k_stream() -> Vec<(u64, Event)> {
    let mut s = Vec::new();
    // Regime A (1-990): diamond posets, 10 phases × 100 ticks.
    for phase in 0..10 {
        let off = 1 + (phase as u64) * 100;
        let nodes: [String; 4] = std::array::from_fn(|i| {
            format!("a{}_{}", phase, i)
        });
        for k in 0..4 {
            s.push((
                off + k as u64,
                Event::AddEdge(R::new(&nodes[k][..], &nodes[k][..])),
            ));
        }
        s.push((off + 7, Event::AddEdge(R::new(&nodes[0][..], &nodes[1][..]))));
        s.push((off + 11, Event::AddEdge(R::new(&nodes[0][..], &nodes[2][..]))));
        s.push((off + 15, Event::AddEdge(R::new(&nodes[0][..], &nodes[3][..]))));
        s.push((off + 19, Event::AddEdge(R::new(&nodes[1][..], &nodes[3][..]))));
        s.push((off + 23, Event::AddEdge(R::new(&nodes[2][..], &nodes[3][..]))));
    }
    // Regime B (1001-1990): bipartite 2x3 × 10 phases.
    for phase in 0..10 {
        let off = 1001 + (phase as u64) * 100;
        let mut t = 0u64;
        for li in 0..2 {
            for ri in 0..3 {
                let l = format!("bl_{}_{}", phase, li);
                let r = format!("br_{}_{}", phase, ri);
                s.push((off + t, Event::AddEdge(R::new(&l[..], &r[..]))));
                t += 2;
            }
        }
    }
    // Regime C (2001-2990): clique families × 10 phases.
    for phase in 0..10 {
        let off = 2001 + (phase as u64) * 100;
        let mut t = 0u64;
        for cls in 0..2 {
            let size = if cls == 0 { 2 } else { 3 };
            let names: Vec<String> = (0..size)
                .map(|i| format!("c_{}_{}_{}", phase, cls, i))
                .collect();
            for x in &names {
                for y in &names {
                    s.push((
                        off + t,
                        Event::AddEdge(R::new(&x[..], &y[..])),
                    ));
                    t += 1;
                }
            }
        }
    }
    // Regime D (3001-3990): diamonds + pattern markers × 10.
    for phase in 0..10 {
        let off = 3001 + (phase as u64) * 100;
        let nodes: [String; 4] = std::array::from_fn(|i| {
            format!("d{}_{}", phase, i)
        });
        for k in 0..4 {
            s.push((
                off + k as u64,
                Event::AddEdge(R::new(&nodes[k][..], &nodes[k][..])),
            ));
        }
        s.push((off + 5, Event::AddEdge(R::new(PATTERN_MARKER, &nodes[0][..]))));
        s.push((off + 6, Event::AddEdge(R::new(&nodes[0][..], ESTABLISHED_MARKER))));
        s.push((off + 7, Event::AddEdge(R::new(&nodes[0][..], &nodes[1][..]))));
        s.push((off + 11, Event::AddEdge(R::new(&nodes[0][..], &nodes[2][..]))));
        s.push((off + 15, Event::AddEdge(R::new(&nodes[0][..], &nodes[3][..]))));
        s.push((off + 19, Event::AddEdge(R::new(&nodes[1][..], &nodes[3][..]))));
        s.push((off + 23, Event::AddEdge(R::new(&nodes[2][..], &nodes[3][..]))));
    }
    // Regime E (4001-4990): mixed shapes × 10. Alternates
    // diamond / bipartite / clique to stress regime-switching.
    for phase in 0..10 {
        let off = 4001 + (phase as u64) * 100;
        let kind = phase % 3;
        match kind {
            0 => {
                let nodes: [String; 4] = std::array::from_fn(|i| {
                    format!("e_d_{}_{}", phase, i)
                });
                for k in 0..4 {
                    s.push((
                        off + k as u64,
                        Event::AddEdge(R::new(&nodes[k][..], &nodes[k][..])),
                    ));
                }
                s.push((off + 7, Event::AddEdge(R::new(&nodes[0][..], &nodes[1][..]))));
                s.push((off + 11, Event::AddEdge(R::new(&nodes[0][..], &nodes[3][..]))));
            }
            1 => {
                let mut t = 0u64;
                for li in 0..2 {
                    for ri in 0..2 {
                        let l = format!("e_bl_{}_{}", phase, li);
                        let r = format!("e_br_{}_{}", phase, ri);
                        s.push((
                            off + t,
                            Event::AddEdge(R::new(&l[..], &r[..])),
                        ));
                        t += 2;
                    }
                }
            }
            _ => {
                let names: Vec<String> = (0..3)
                    .map(|i| format!("e_c_{}_{}", phase, i))
                    .collect();
                let mut t = 0u64;
                for x in &names {
                    for y in &names {
                        s.push((
                            off + t,
                            Event::AddEdge(R::new(&x[..], &y[..])),
                        ));
                        t += 1;
                    }
                }
            }
        }
    }
    s
}
