#!/usr/bin/env python3
"""C.1 batch refactor: replace local `fn build_long_stream` with import
of `relatum_v2::test_substrates::oq1::build_long_stream`."""

import re
import sys
from pathlib import Path

EXAMPLE_DIR = Path("examples")

EXAMPLES = [
    "phase_alpha_axiom_demote.rs",
    "phase_alpha_baseline_timed.rs",
    "phase_alpha_composite_ucb.rs",
    "phase_alpha_cross_precision_demote.rs",
    "phase_alpha_cross_precision_varying_t.rs",
    "phase_alpha_dream_phase.rs",
    "phase_alpha_theory_demote_loop.rs",
    "phase_alpha_theory_demote_loop_n.rs",
    "phase_alpha_theory_merge.rs",
    "phase_alpha_theory_merge_smart.rs",
    "phase_alpha_theory_repair.rs",
    "phase_alpha_theory_tournament.rs",
    "phase_beta_1_shape_families.rs",
    "phase_beta_2_family_demote.rs",
    "phase_h1_long_run.rs",
    "phase_h2_0_oq1_experiment.rs",
]

USE_IMPORT = "use relatum_v2::test_substrates::oq1::build_long_stream;\n"


def find_function_block(lines, fn_signature_re):
    """Find lines covering `fn_signature_re` through its matching `}`.
    Returns (start_idx, end_idx_exclusive) or None if not found."""
    start = None
    for i, line in enumerate(lines):
        if fn_signature_re.match(line):
            start = i
            break
    if start is None:
        return None
    # Match braces from start
    depth = 0
    for i in range(start, len(lines)):
        depth += lines[i].count("{") - lines[i].count("}")
        if depth == 0 and i > start:
            return (start, i + 1)
    return None


FN_RE = re.compile(r"^fn build_long_stream\b")


def refactor(path: Path) -> bool:
    src = path.read_text(encoding="utf-8")
    lines = src.splitlines(keepends=True)

    block = find_function_block(lines, FN_RE)
    if block is None:
        print(f"  SKIP {path.name}: no build_long_stream found")
        return False
    start, end = block

    # Trim adjacent blank lines so we don't leave double-blanks.
    cut_start = start
    while cut_start > 0 and lines[cut_start - 1].strip() == "":
        cut_start -= 1
    cut_end = end
    while cut_end < len(lines) and lines[cut_end].strip() == "":
        cut_end += 1

    # Find a good insertion point for the use statement: after the
    # last `use ...;` line at top level (before any `const`, `fn`, etc.).
    # Easiest: insert right before the deletion point.
    new_lines = (
        lines[:cut_start]
        + [USE_IMPORT, "\n"]
        + lines[cut_end:]
    )
    new_src = "".join(new_lines)

    if new_src == src:
        return False
    path.write_text(new_src, encoding="utf-8")
    print(f"  OK   {path.name}: replaced lines {start+1}..{end} ({end-start} lines) -> import")
    return True


def main():
    if not EXAMPLE_DIR.is_dir():
        print(f"ERROR: {EXAMPLE_DIR} not found", file=sys.stderr)
        sys.exit(1)
    print("C.1: batch refactor of OQ#1 build_long_stream copies")
    n = 0
    for name in EXAMPLES:
        p = EXAMPLE_DIR / name
        if not p.exists():
            print(f"  MISS {name}: file not found")
            continue
        if refactor(p):
            n += 1
    print(f"Done. Refactored {n} files.")


if __name__ == "__main__":
    main()
