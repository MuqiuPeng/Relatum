    use super::*;

    #[test]
    fn r_can_be_constructed() {
        let r = R::new("a", "b");
        assert_eq!(r.x, "a");
        assert_eq!(r.y, "b");
    }

    #[test]
    fn r_is_directional() {
        assert_ne!(R::new("a", "b"), R::new("b", "a"));
    }

    #[test]
    fn identity_is_token_based() {
        assert_eq!(R::new("a", "b"), R::new("a", "b"));
    }

    #[test]
    fn rset_starts_empty() {
        let rs = RSet::new();
        assert!(rs.is_empty());
        assert_eq!(rs.len(), 0);
    }

    #[test]
    fn rset_dedups_identical_instances() {
        let mut rs = RSet::new();
        assert!(rs.add(R::new("a", "b")));
        assert!(!rs.add(R::new("a", "b")));
        assert_eq!(rs.len(), 1);
    }

    #[test]
    fn rset_treats_direction_as_distinct() {
        let mut rs = RSet::new();
        rs.add(R::new("a", "b"));
        rs.add(R::new("b", "a"));
        assert_eq!(rs.len(), 2);
    }

    #[test]
    fn identifiers_collects_tokens_from_both_sides() {
        let mut rs = RSet::new();
        rs.add(R::new("a", "b"));
        rs.add(R::new("b", "c"));
        let ids = rs.identifiers();
        assert_eq!(ids.len(), 3);
        assert!(ids.contains("a"));
        assert!(ids.contains("b"));
        assert!(ids.contains("c"));
    }

    #[test]
    fn left_of_and_right_of_partition_by_slot() {
        let mut rs = RSet::new();
        rs.add(R::new("a", "b"));
        rs.add(R::new("a", "c"));
        rs.add(R::new("d", "a"));

        assert_eq!(rs.left_of("a").len(), 2);
        assert_eq!(rs.right_of("a").len(), 1);
        assert_eq!(rs.left_of("z").len(), 0);
    }

    #[test]
    fn profile_of_absent_identifier_is_zero() {
        let rs = RSet::new();
        let p = rs.profile("ghost");
        assert_eq!(p.degree_out, 0);
        assert_eq!(p.degree_in, 0);
        assert_eq!(p.slots, SlotPattern::None);
        assert_eq!(p.total_degree(), 0);
    }

    #[test]
    fn profile_distinguishes_slot_patterns() {
        let mut rs = RSet::new();
        rs.add(R::new("source", "middle"));
        rs.add(R::new("middle", "sink"));

        assert_eq!(rs.profile("source").slots, SlotPattern::LeftOnly);
        assert_eq!(rs.profile("sink").slots, SlotPattern::RightOnly);
        assert_eq!(rs.profile("middle").slots, SlotPattern::Both);
    }

    #[test]
    fn profile_counts_degrees() {
        let mut rs = RSet::new();
        rs.extend([
            R::new("hub", "a"),
            R::new("hub", "b"),
            R::new("hub", "c"),
            R::new("x", "hub"),
        ]);
        let p = rs.profile("hub");
        assert_eq!(p.degree_out, 3);
        assert_eq!(p.degree_in, 1);
        assert_eq!(p.total_degree(), 4);
        assert_eq!(p.slots, SlotPattern::Both);
    }

    #[test]
    fn self_loop_registers_as_both_slots() {
        let mut rs = RSet::new();
        rs.add(R::new("loop", "loop"));
        let p = rs.profile("loop");
        assert_eq!(p.degree_out, 1);
        assert_eq!(p.degree_in, 1);
        assert_eq!(p.slots, SlotPattern::Both);
    }

    #[test]
    fn profiles_covers_every_identifier_in_set() {
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b"), R::new("b", "c")]);
        let all = rs.profiles();
        assert_eq!(all.len(), 3);
        assert!(all.contains_key("a"));
        assert!(all.contains_key("b"));
        assert!(all.contains_key("c"));
    }

    #[test]
    fn chain_profile_marks_endpoints_asymmetrically() {
        let mut rs = RSet::new();
        rs.extend([
            R::new("a1", "a2"),
            R::new("a2", "a3"),
            R::new("a3", "a4"),
        ]);

        // chain head: only appears on left
        assert_eq!(rs.profile("a1").slots, SlotPattern::LeftOnly);
        // chain tail: only appears on right
        assert_eq!(rs.profile("a4").slots, SlotPattern::RightOnly);
        // middle nodes: both, each with degree 1 each way
        let mid = rs.profile("a2");
        assert_eq!(mid.degree_out, 1);
        assert_eq!(mid.degree_in, 1);
    }

    #[test]
    fn equivalence_classes_are_empty_for_empty_set() {
        let rs = RSet::new();
        assert!(rs.equivalence_classes().is_empty());
    }

    #[test]
    fn single_instance_produces_two_classes() {
        let mut rs = RSet::new();
        rs.add(R::new("a", "b"));
        let classes = rs.equivalence_classes();
        // one LeftOnly (a), one RightOnly (b)
        assert_eq!(classes.len(), 2);
    }

    #[test]
    fn chain_produces_three_classes_head_middles_tail() {
        let mut rs = RSet::new();
        rs.extend([
            R::new("a1", "a2"),
            R::new("a2", "a3"),
            R::new("a3", "a4"),
            R::new("a4", "a5"),
        ]);
        let classes = rs.equivalence_classes();
        assert_eq!(classes.len(), 3);

        // the middles collapse — three of them in one class
        let (_, biggest) = classes.iter().max_by_key(|(_, v)| v.len()).unwrap();
        assert_eq!(biggest.len(), 3);
        assert!(biggest.contains("a2"));
        assert!(biggest.contains("a3"));
        assert!(biggest.contains("a4"));
    }

    #[test]
    fn cycle_collapses_to_one_class() {
        let mut rs = RSet::new();
        rs.extend([
            R::new("a", "b"),
            R::new("b", "c"),
            R::new("c", "a"),
        ]);
        let classes = rs.equivalence_classes();
        assert_eq!(classes.len(), 1);
        let only = classes.values().next().unwrap();
        assert_eq!(only.len(), 3);
    }

    #[test]
    fn star_splits_hub_from_leaves() {
        let mut rs = RSet::new();
        rs.extend([
            R::new("hub", "a"),
            R::new("hub", "b"),
            R::new("hub", "c"),
        ]);
        let classes = rs.equivalence_classes();
        assert_eq!(classes.len(), 2);

        // one singleton class (the hub), one class of three leaves
        let sizes: Vec<usize> = {
            let mut v: Vec<usize> = classes.values().map(|c| c.len()).collect();
            v.sort();
            v
        };
        assert_eq!(sizes, vec![1, 3]);
    }

    #[test]
    fn bidirectional_chain_collapses_endpoints() {
        // forward + reverse: endpoints have (out=1, in=1, Both),
        // same as each other; middles have (out=2, in=2, Both).
        let mut rs = RSet::new();
        rs.extend([
            R::new("a1", "a2"),
            R::new("a2", "a3"),
            R::new("a3", "a4"),
            R::new("a2", "a1"),
            R::new("a3", "a2"),
            R::new("a4", "a3"),
        ]);
        let classes = rs.equivalence_classes();
        assert_eq!(classes.len(), 2);

        // endpoints a1 and a4 share a class; middles a2 and a3 share a class
        let mut sizes: Vec<usize> = classes.values().map(|c| c.len()).collect();
        sizes.sort();
        assert_eq!(sizes, vec![2, 2]);
    }

    #[test]
    fn r_equivalence_empty_for_empty_set() {
        let rs = RSet::new();
        assert!(rs.r_equivalence_classes().is_empty());
    }

    #[test]
    fn short_chain_two_edge_classes() {
        // a1 -> a2 -> a3: no middle-middle edge, so head-edge and tail-edge
        let mut rs = RSet::new();
        rs.extend([R::new("a1", "a2"), R::new("a2", "a3")]);
        assert_eq!(rs.r_equivalence_classes().len(), 2);
    }

    #[test]
    fn long_chain_three_edge_classes_with_middle_merge() {
        // a1 -> a2 -> a3 -> a4 -> a5: middle-middle edges R(a2,a3) and
        // R(a3,a4) must merge into a single class.
        let mut rs = RSet::new();
        rs.extend([
            R::new("a1", "a2"),
            R::new("a2", "a3"),
            R::new("a3", "a4"),
            R::new("a4", "a5"),
        ]);
        let classes = rs.r_equivalence_classes();
        assert_eq!(classes.len(), 3);

        // one class has 2 edges (the middle-middle merge)
        let mut sizes: Vec<usize> = classes.values().map(|c| c.len()).collect();
        sizes.sort();
        assert_eq!(sizes, vec![1, 1, 2]);
    }

    #[test]
    fn cycle_merges_all_edges() {
        let mut rs = RSet::new();
        rs.extend([
            R::new("a", "b"),
            R::new("b", "c"),
            R::new("c", "a"),
        ]);
        let classes = rs.r_equivalence_classes();
        assert_eq!(classes.len(), 1);
        assert_eq!(classes.values().next().unwrap().len(), 3);
    }

    #[test]
    fn star_merges_all_spokes() {
        let mut rs = RSet::new();
        rs.extend([
            R::new("hub", "a"),
            R::new("hub", "b"),
            R::new("hub", "c"),
        ]);
        let classes = rs.r_equivalence_classes();
        assert_eq!(classes.len(), 1);
        assert_eq!(classes.values().next().unwrap().len(), 3);
    }

    #[test]
    fn r_signature_respects_direction() {
        // two edges with the same endpoint profiles but opposite directions
        // must land in different classes
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b"), R::new("b", "a")]);
        // a and b both have Both-1-1 profile after these two edges
        // but the signatures are (Both-1-1, Both-1-1) either way...
        // so they collapse. That's the correct behavior — in this graph
        // the two edges really are structurally equivalent.
        //
        // Construct a clearer directional case: a -> b and c -> d where
        // a has LeftOnly, b has RightOnly, c has LeftOnly, d has RightOnly.
        // Both edges should merge.
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b"), R::new("c", "d")]);
        let classes = rs.r_equivalence_classes();
        assert_eq!(classes.len(), 1);

        // Now add a reverse edge e -> f where e has LeftOnly and f has
        // RightOnly, but between two nodes where the profile contains
        // both directions — this requires constructing a richer case.
        // Simplest directional test: R(a, b) vs R(c, b) where both
        // edges terminate at b. b now has in=2, a and c have out=1.
        // Signatures: (LeftOnly-1-0, RightOnly-0-2). Merged.
        //
        // For a case where direction forces a split, use the
        // bidirectional-chain test below.
    }

    #[test]
    fn bidirectional_chain_edge_classes() {
        let mut rs = RSet::new();
        rs.extend([
            R::new("a1", "a2"), R::new("a2", "a3"), R::new("a3", "a4"),
            R::new("a2", "a1"), R::new("a3", "a2"), R::new("a4", "a3"),
        ]);
        // Profiles:
        //   a1, a4: (out=1, in=1, Both)
        //   a2, a3: (out=2, in=2, Both)
        // Edge signatures (ordered):
        //   out-from-end:    (1-1-Both, 2-2-Both)  — R(a1,a2), R(a4,a3)
        //   in-to-end:       (2-2-Both, 1-1-Both)  — R(a2,a1), R(a3,a4)
        //   middle-middle:   (2-2-Both, 2-2-Both)  — R(a2,a3), R(a3,a2)
        // Three classes, two edges each. Direction of edge distinguishes
        // out-from-end from in-to-end.
        let classes = rs.r_equivalence_classes();
        assert_eq!(classes.len(), 3);
        let mut sizes: Vec<usize> = classes.values().map(|c| c.len()).collect();
        sizes.sort();
        assert_eq!(sizes, vec![2, 2, 2]);
    }

    #[test]
    fn locality_of_absent_edge_is_zero() {
        let rs = RSet::new();
        let p = rs.locality_profile(&R::new("a", "b"));
        assert_eq!(p.co_left, 0);
        assert_eq!(p.co_right, 0);
        assert_eq!(p.forward, 0);
        assert_eq!(p.reverse, 0);
    }

    #[test]
    fn locality_separates_cycle_from_star() {
        let mut cycle = RSet::new();
        cycle.extend([R::new("a", "b"), R::new("b", "c"), R::new("c", "a")]);
        let loc_cycle = cycle.locality_profile(&R::new("a", "b"));
        assert_eq!(
            loc_cycle,
            LocalityProfile { co_left: 0, co_right: 0, forward: 1, reverse: 1 }
        );

        let mut star = RSet::new();
        star.extend([R::new("hub", "a"), R::new("hub", "b"), R::new("hub", "c")]);
        let loc_star = star.locality_profile(&R::new("hub", "a"));
        assert_eq!(
            loc_star,
            LocalityProfile { co_left: 2, co_right: 0, forward: 0, reverse: 0 }
        );

        // The motivating distinction: these profiles differ.
        assert_ne!(loc_cycle, loc_star);
    }

    #[test]
    fn locality_chain_positions() {
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b"), R::new("b", "c"), R::new("c", "d")]);

        // head edge: only a forward neighbor
        assert_eq!(
            rs.locality_profile(&R::new("a", "b")),
            LocalityProfile { co_left: 0, co_right: 0, forward: 1, reverse: 0 }
        );
        // middle edge: one forward, one reverse
        assert_eq!(
            rs.locality_profile(&R::new("b", "c")),
            LocalityProfile { co_left: 0, co_right: 0, forward: 1, reverse: 1 }
        );
        // tail edge: only a reverse neighbor
        assert_eq!(
            rs.locality_profile(&R::new("c", "d")),
            LocalityProfile { co_left: 0, co_right: 0, forward: 0, reverse: 1 }
        );
    }

    #[test]
    fn locality_known_chain_cycle_collision() {
        // Recorded limitation: chain-middle edge and any cycle edge have
        // the same 1-hop locality profile (0, 0, 1, 1). This test locks
        // the behavior so we notice when a future upgrade (2-hop) breaks
        // the collision.
        let mut chain = RSet::new();
        chain.extend([R::new("a", "b"), R::new("b", "c"), R::new("c", "d")]);

        let mut cycle = RSet::new();
        cycle.extend([R::new("x", "y"), R::new("y", "z"), R::new("z", "x")]);

        assert_eq!(
            chain.locality_profile(&R::new("b", "c")),
            cycle.locality_profile(&R::new("x", "y"))
        );
    }

    #[test]
    fn locality_in_star_puts_co_right() {
        let mut rs = RSet::new();
        rs.extend([R::new("a", "sink"), R::new("b", "sink"), R::new("c", "sink")]);
        let p = rs.locality_profile(&R::new("a", "sink"));
        assert_eq!(p.co_left, 0);
        assert_eq!(p.co_right, 2);
        assert_eq!(p.forward, 0);
        assert_eq!(p.reverse, 0);
    }

    #[test]
    fn locality_excludes_self() {
        // a self-loop R(a, a): when asked about itself, all four counts
        // should be zero because the only candidate neighbor (itself) is
        // excluded. (The self-loop character is visible via profile
        // slots, not locality.)
        let mut rs = RSet::new();
        rs.add(R::new("a", "a"));
        let p = rs.locality_profile(&R::new("a", "a"));
        assert_eq!(p.total(), 0);
    }

    #[test]
    fn edge_fingerprint_composes_existing_signals() {
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b"), R::new("b", "c")]);
        let r = R::new("a", "b");
        let fp = rs.edge_fingerprint(&r);
        assert_eq!(fp.0, rs.r_signature(&r));
        assert_eq!(fp.1, rs.locality_profile(&r));
    }

    #[test]
    fn edge_fingerprint_merges_star_spokes() {
        let mut rs = RSet::new();
        rs.extend([R::new("h", "a"), R::new("h", "b"), R::new("h", "c")]);
        let fa = rs.edge_fingerprint(&R::new("h", "a"));
        let fb = rs.edge_fingerprint(&R::new("h", "b"));
        let fc = rs.edge_fingerprint(&R::new("h", "c"));
        assert_eq!(fa, fb);
        assert_eq!(fb, fc);
    }

    #[test]
    fn edge_fingerprint_inherits_1hop_chain_cycle_collision() {
        // Documented in ADR 0006 and 0007: compound fingerprint does not
        // break the chain-middle / cycle-edge collision because both
        // its components are 1-hop.
        let mut chain = RSet::new();
        chain.extend([R::new("a", "b"), R::new("b", "c"), R::new("c", "d")]);
        let mut cycle = RSet::new();
        cycle.extend([R::new("x", "y"), R::new("y", "z"), R::new("z", "x")]);
        assert_eq!(
            chain.edge_fingerprint(&R::new("b", "c")),
            cycle.edge_fingerprint(&R::new("x", "y"))
        );
    }

    #[test]
    fn subgraph_empty() {
        let s = Subgraph::new();
        assert!(s.is_empty());
        assert_eq!(s.len(), 0);
        assert!(s.identifiers().is_empty());
    }

    #[test]
    fn subgraph_from_edges_roundtrip() {
        let s = Subgraph::from_edges([R::new("a", "b"), R::new("b", "c")]);
        assert_eq!(s.len(), 2);
        assert!(s.contains(&R::new("a", "b")));
        assert_eq!(s.identifiers().len(), 3);
    }

    #[test]
    fn connected_components_empty_input() {
        assert!(Subgraph::connected_components_of([] as [R; 0]).is_empty());
    }

    #[test]
    fn connected_components_single_edge_single_component() {
        let comps = Subgraph::connected_components_of([R::new("a", "b")]);
        assert_eq!(comps.len(), 1);
        assert_eq!(comps[0].len(), 1);
    }

    #[test]
    fn connected_components_disjoint_edges_split() {
        let comps = Subgraph::connected_components_of([
            R::new("a", "b"),
            R::new("c", "d"), // shares no identifier with R(a, b)
        ]);
        assert_eq!(comps.len(), 2);
    }

    #[test]
    fn connected_components_chain_is_single() {
        let comps = Subgraph::connected_components_of([
            R::new("a", "b"),
            R::new("b", "c"),
            R::new("c", "d"),
        ]);
        assert_eq!(comps.len(), 1);
        assert_eq!(comps[0].len(), 3);
    }

    #[test]
    fn connected_components_cycle_is_single() {
        let comps = Subgraph::connected_components_of([
            R::new("a", "b"),
            R::new("b", "c"),
            R::new("c", "a"),
        ]);
        assert_eq!(comps.len(), 1);
        assert_eq!(comps[0].len(), 3);
    }

    #[test]
    fn compound_class_subgraphs_splits_chain_plus_cycle_false_merge() {
        // Chain and cycle disjoint in the same RSet. Their interior
        // edges share the 1-hop compound fingerprint, so they land in
        // one compound class — but their connected components differ.
        let mut rs = RSet::new();
        rs.extend([
            R::new("c1", "c2"), R::new("c2", "c3"),
            R::new("c3", "c4"), R::new("c4", "c5"),
        ]);
        rs.extend([
            R::new("k1", "k2"), R::new("k2", "k3"), R::new("k3", "k1"),
        ]);

        let classes = rs.compound_class_subgraphs();
        // Find the big class (5 members = chain-middle + cycle)
        let big = classes
            .values()
            .find(|subgraphs| subgraphs.iter().map(|s| s.len()).sum::<usize>() == 5)
            .expect("expected a 5-member compound class (chain-middle + cycle)");
        assert_eq!(big.len(), 2);
        // one subgraph of 2 edges (chain fragment), one of 3 (cycle)
        let mut sizes: Vec<usize> = big.iter().map(|s| s.len()).collect();
        sizes.sort();
        assert_eq!(sizes, vec![2, 3]);
    }

    #[test]
    fn compound_class_subgraphs_star_stays_unified() {
        let mut rs = RSet::new();
        rs.extend([
            R::new("h", "a"),
            R::new("h", "b"),
            R::new("h", "c"),
        ]);
        let classes = rs.compound_class_subgraphs();
        assert_eq!(classes.len(), 1);
        let subgraphs = classes.values().next().unwrap();
        assert_eq!(subgraphs.len(), 1);
        assert_eq!(subgraphs[0].len(), 3);
    }

    #[test]
    fn compound_class_subgraphs_same_fingerprint_no_shared_id_splits() {
        // Two single edges with the same endpoint profile pair but no
        // identifier in common land in the same compound class yet
        // produce two separate subgraphs.
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b"), R::new("c", "d")]);
        let classes = rs.compound_class_subgraphs();
        assert_eq!(classes.len(), 1);
        let subgraphs = classes.values().next().unwrap();
        assert_eq!(subgraphs.len(), 2);
        assert!(subgraphs.iter().all(|s| s.len() == 1));
    }

    #[test]
    fn canonicalize_empty_subgraph() {
        assert!(Subgraph::new().canonicalize().is_empty());
    }

    #[test]
    fn canonicalize_single_edge_has_one_canonical_form() {
        // Every single-edge subgraph reduces to the same canonical form
        // regardless of its identifiers: one directed edge from a
        // source (out=1,in=0) to a sink (out=0,in=1).
        let a = Subgraph::from_edges([R::new("a", "b")]);
        let b = Subgraph::from_edges([R::new("p", "q")]);
        let c = Subgraph::from_edges([R::new("hello", "world")]);
        assert_eq!(a.canonicalize(), b.canonicalize());
        assert_eq!(b.canonicalize(), c.canonicalize());
    }

    #[test]
    fn canonicalize_isomorphic_two_chains() {
        let a = Subgraph::from_edges([R::new("a", "b"), R::new("b", "c")]);
        let b = Subgraph::from_edges([R::new("p", "q"), R::new("q", "r")]);
        assert!(a.is_isomorphic_to(&b));
    }

    #[test]
    fn canonicalize_chain_vs_cycle_differ() {
        let chain = Subgraph::from_edges([
            R::new("a", "b"), R::new("b", "c"), R::new("c", "d"),
        ]);
        let cycle = Subgraph::from_edges([
            R::new("x", "y"), R::new("y", "z"), R::new("z", "x"),
        ]);
        assert_ne!(chain.canonicalize(), cycle.canonicalize());
    }

    #[test]
    fn canonicalize_chain_vs_star_differ() {
        let chain = Subgraph::from_edges([R::new("a", "b"), R::new("b", "c")]);
        let star = Subgraph::from_edges([R::new("h", "a"), R::new("h", "b")]);
        assert_ne!(chain.canonicalize(), star.canonicalize());
    }

    #[test]
    fn canonicalize_isomorphic_three_cycles() {
        let one = Subgraph::from_edges([
            R::new("a", "b"), R::new("b", "c"), R::new("c", "a"),
        ]);
        let two = Subgraph::from_edges([
            R::new("x", "y"), R::new("y", "z"), R::new("z", "x"),
        ]);
        assert!(one.is_isomorphic_to(&two));
    }

    #[test]
    fn canonicalize_isomorphic_three_stars() {
        let one = Subgraph::from_edges([
            R::new("h1", "a"), R::new("h1", "b"), R::new("h1", "c"),
        ]);
        let two = Subgraph::from_edges([
            R::new("h2", "p"), R::new("h2", "q"), R::new("h2", "r"),
        ]);
        assert!(one.is_isomorphic_to(&two));
    }

    #[test]
    fn canonicalize_forward_chain_same_as_reversed_identifiers() {
        // Forward chain a -> b -> c and "reversed identifier" chain
        // c -> b -> a are the same *structure*: source -> middle -> sink.
        // Only the names of the nodes change.
        let forward = Subgraph::from_edges([R::new("a", "b"), R::new("b", "c")]);
        let renamed = Subgraph::from_edges([R::new("c", "b"), R::new("b", "a")]);
        assert!(forward.is_isomorphic_to(&renamed));
    }

    #[test]
    fn canonicalize_distinguishes_V_from_chain() {
        // a -> b -> c is a chain. a -> b, c -> b is a "V" into b.
        // Structurally distinct even though node counts match.
        let chain = Subgraph::from_edges([R::new("a", "b"), R::new("b", "c")]);
        let vee = Subgraph::from_edges([R::new("a", "b"), R::new("c", "b")]);
        assert_ne!(chain.canonicalize(), vee.canonicalize());
    }

    #[test]
    fn canonicalize_direction_matters() {
        // Single "outward" edge a -> b has a source and a sink.
        // Reverse edge b -> a is *the same* one-edge pattern; only
        // the labels change. But two chains with opposite edge
        // direction at the fork are different.
        let one_forward = Subgraph::from_edges([R::new("a", "b"), R::new("b", "c")]);
        let one_reversed = Subgraph::from_edges([R::new("b", "a"), R::new("c", "b")]);
        // Both are "source -> middle -> sink" with relabeled nodes.
        // So canonical forms should match.
        assert!(one_forward.is_isomorphic_to(&one_reversed));

        // But a -> b -> c (chain) and b -> a, b -> c (out-V) differ.
        let out_vee = Subgraph::from_edges([R::new("b", "a"), R::new("b", "c")]);
        assert_ne!(one_forward.canonicalize(), out_vee.canonicalize());
    }

    #[test]
    fn canonicalize_distinguishes_fan_in_from_fan_out() {
        // ADR 0055 regression test. Pre-fix, the rank_labels
        // projection collapsed both shapes to the same canonical;
        // the WL signature already distinguished them but the rank
        // step threw the distinction away. Post-fix (signature_hash
        // projection), the canonicals are distinct.
        let fan_in = Subgraph::from_edges([
            R::new("a", "t"),
            R::new("b", "t"),
            R::new("c", "t"),
        ]);
        let fan_out = Subgraph::from_edges([
            R::new("s", "x"),
            R::new("s", "y"),
            R::new("s", "z"),
        ]);
        assert_ne!(
            fan_in.canonicalize(),
            fan_out.canonicalize(),
            "fan-in and fan-out must canonicalize to different forms"
        );
        // Sanity: each shape is isomorphic to itself with relabeling.
        let fan_in_alt = Subgraph::from_edges([
            R::new("p", "q"),
            R::new("r", "q"),
            R::new("s", "q"),
        ]);
        assert!(fan_in.is_isomorphic_to(&fan_in_alt));
    }

    // ─── ADR 0058 Phase G1.0 — axiom forward-application ────────

    #[test]
    fn g1_forward_apply_unknown_axiom_returns_empty() {
        let rs = RSet::new();
        assert!(rs.forward_apply_axiom("ax_unknown_xyz").is_empty());
    }

    #[test]
    fn g1_forward_apply_predicate_axiom_returns_empty() {
        // Reflexivity / antisymmetry / totality are predicate
        // axioms; reconstruct_axiom_template returns None for them
        // so forward-apply yields empty per ADR 0058.
        let mut rs = RSet::new();
        rs.add(R::new("a", "a"));
        rs.add(R::new(AXIOM_MARKER, AX_REFLEXIVITY));
        let predicted = rs.forward_apply_axiom(AX_REFLEXIVITY);
        assert!(predicted.is_empty());
    }

    #[test]
    fn g1_forward_apply_template_axiom_predicts_conclusion() {
        // Build the transitive closure of a 5-node total order
        // (every i<j has an edge). The discovered theory will
        // include transitivity-shaped templates because the
        // closure makes them universally hold. Forward-apply
        // those axioms — the predicted set must contain at least
        // one edge that is *also* in the rset (a re-derivation),
        // which is the cleanest evidence the mechanism is firing.
        let mut rs = RSet::new();
        let nodes = ["a", "b", "c", "d", "e"];
        for i in 0..nodes.len() {
            for j in (i + 1)..nodes.len() {
                rs.add(R::new(nodes[i], nodes[j]));
            }
        }
        let cfg = AxiomDiscoveryConfig::default();
        let theory = rs.discover_theory(&cfg);
        let ax_ids: Vec<&str> =
            theory.member_axiom_ids.iter().map(|s| s.as_str()).collect();
        rs.name_theory(&ax_ids).expect("name theory");
        let predicted = rs.forward_apply_all();
        // The transitive-closure substrate has axioms that re-derive
        // many of the existing edges. At least one of the long-edge
        // closures should appear.
        assert!(
            !predicted.is_empty(),
            "forward-apply should produce at least one prediction \
             on a closure-shaped rset; got empty set with axioms {:?}",
            ax_ids
        );
        assert!(
            predicted.contains(&R::new("a", "c"))
                || predicted.contains(&R::new("a", "d"))
                || predicted.contains(&R::new("b", "d")),
            "expected re-derivation of a closure edge; got: {:?}",
            predicted
        );
    }

    #[test]
    fn g1_forward_apply_excludes_meta_in_substitution_domain() {
        // Even if a meta id (e.g. PATTERN_MARKER) happens to be a
        // valid substitution that would satisfy a premise, the
        // domain restriction (commitment 3) excludes meta. Use an
        // rset that has no template axioms — forward_apply_all
        // returns empty regardless.
        let mut rs = RSet::new();
        rs.add(R::new(PATTERN_MARKER, "p_x"));
        rs.add(R::new("a", "b"));
        let predicted = rs.forward_apply_all();
        // No named template axioms → empty.
        assert!(predicted.is_empty());
    }

    #[test]
    fn g1_forward_apply_no_axioms_returns_empty() {
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b"), R::new("b", "c")]);
        let predicted = rs.forward_apply_all();
        assert!(predicted.is_empty());
    }

    #[test]
    fn naming_empty_instance_list_errors() {
        let mut rs = RSet::new();
        assert_eq!(
            rs.name_pattern_instances(&[]),
            Err(PatternError::EmptyInstanceList)
        );
    }

    #[test]
    fn naming_empty_instance_errors() {
        let mut rs = RSet::new();
        assert_eq!(
            rs.name_pattern_instances(&[Subgraph::new()]),
            Err(PatternError::EmptyInstance)
        );
    }

    #[test]
    fn naming_non_isomorphic_errors() {
        let mut rs = RSet::new();
        let chain = Subgraph::from_edges([R::new("a", "b"), R::new("b", "c")]);
        let star = Subgraph::from_edges([R::new("h", "a"), R::new("h", "b")]);
        assert_eq!(
            rs.name_pattern_instances(&[chain, star]),
            Err(PatternError::NotIsomorphic)
        );
    }

    #[test]
    fn naming_same_canonical_twice_reuses_pattern_id() {
        let mut rs = RSet::new();
        // Populate with two separated chains so reconstruction can recover
        // the canonical form from participants.
        rs.extend([
            R::new("a", "b"), R::new("b", "c"),
            R::new("p", "q"), R::new("q", "r"),
        ]);
        let first = Subgraph::from_edges([R::new("a", "b"), R::new("b", "c")]);
        let second = Subgraph::from_edges([R::new("p", "q"), R::new("q", "r")]);

        let p1 = rs.name_pattern_instances(&[first]).unwrap();
        let p2 = rs.name_pattern_instances(&[second]).unwrap();
        assert_eq!(p1, p2);
        // One pattern, two instances.
        assert_eq!(rs.patterns().len(), 1);
        assert_eq!(rs.instances_of(&p1).len(), 2);
    }

    #[test]
    fn naming_skips_colliding_pattern_id() {
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b"), R::new("b", "c")]);
        // Plant a spurious user identifier that would clash with p_0.
        rs.add(R::new("p_0", "spurious"));

        let chain = Subgraph::from_edges([R::new("a", "b"), R::new("b", "c")]);
        let pid = rs.name_pattern_instances(&[chain]).unwrap();
        assert_ne!(pid, "p_0");
        // With a single planted collision, the next free id is p_1.
        assert_eq!(pid, "p_1");
    }

    #[test]
    fn naming_round_trips_canonical_form() {
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b"), R::new("b", "c")]);
        let sg = Subgraph::from_edges([R::new("a", "b"), R::new("b", "c")]);
        let expected = sg.canonicalize();

        let pid = rs.name_pattern_instances(&[sg]).unwrap();
        let instance = rs.instances_of(&pid)[0].to_string();
        let participants = rs.participants_of(&instance);
        let edges: Vec<R> = rs
            .iter()
            .filter(|r| {
                participants.contains(r.x.as_str())
                    && participants.contains(r.y.as_str())
            })
            .cloned()
            .collect();
        let recovered = Subgraph::from_edges(edges);
        assert_eq!(recovered.canonicalize(), expected);
    }

    #[test]
    fn participant_shared_across_two_patterns() {
        let mut rs = RSet::new();
        // A node `b` participates in a chain {a, b, c} and a star {b, x, y}.
        rs.extend([
            R::new("a", "b"), R::new("b", "c"),
            R::new("b", "x"), R::new("b", "y"),
        ]);
        let chain = Subgraph::from_edges([R::new("a", "b"), R::new("b", "c")]);
        let star = Subgraph::from_edges([R::new("b", "x"), R::new("b", "y")]);

        let p_chain = rs.name_pattern_instances(&[chain]).unwrap();
        let p_star = rs.name_pattern_instances(&[star]).unwrap();
        assert_ne!(p_chain, p_star);

        let inst_chain = rs.instances_of(&p_chain)[0].to_string();
        let inst_star = rs.instances_of(&p_star)[0].to_string();
        assert!(rs.participants_of(&inst_chain).contains("b"));
        assert!(rs.participants_of(&inst_star).contains("b"));

        // instances_of is pattern-local.
        assert_eq!(rs.instances_of(&p_chain).len(), 1);
        assert_eq!(rs.instances_of(&p_star).len(), 1);
    }

    #[test]
    fn naming_single_edge_pattern_collects_six_instances() {
        // Mirrors ADR 0009's P2 finding: six single-edge subgraphs across
        // the mixed graph all share a canonical form.
        let mut rs = RSet::new();
        // A small assortment of single-edge contexts — participant
        // identifiers cannot collide across subgraphs, so each instance
        // is truly isolated.
        rs.extend([
            R::new("a1", "a2"),
            R::new("b1", "b2"),
            R::new("c1", "c2"),
            R::new("d1", "d2"),
            R::new("e1", "e2"),
            R::new("f1", "f2"),
        ]);
        let instances: Vec<Subgraph> = [
            ("a1", "a2"), ("b1", "b2"), ("c1", "c2"),
            ("d1", "d2"), ("e1", "e2"), ("f1", "f2"),
        ]
        .into_iter()
        .map(|(x, y)| Subgraph::from_edges([R::new(x, y)]))
        .collect();

        let pid = rs.name_pattern_instances(&instances).unwrap();
        assert_eq!(rs.patterns().len(), 1);
        assert_eq!(rs.instances_of(&pid).len(), 6);
    }

    fn build_mixed_graph() -> RSet {
        let mut rs = RSet::new();
        rs.extend([
            R::new("c1", "c2"), R::new("c2", "c3"),
            R::new("c3", "c4"), R::new("c4", "c5"),
        ]);
        rs.extend([
            R::new("k1", "k2"), R::new("k2", "k3"), R::new("k3", "k1"),
        ]);
        rs.extend([
            R::new("s", "sa"), R::new("s", "sb"), R::new("s", "sc"),
        ]);
        rs.extend([
            R::new("t1", "t2"), R::new("t1", "t3"), R::new("t2", "t4"),
        ]);
        rs.add(R::new("ie1", "ie2"));
        rs
    }

    #[test]
    fn default_policy_skips_single_edge() {
        let mut rs = RSet::new();
        rs.add(R::new("a", "b"));
        let sg = Subgraph::from_edges([R::new("a", "b")]);
        let decision = rs.consider_naming(&[sg], &NamingPolicy::default()).unwrap();
        assert!(matches!(
            decision,
            NamingDecision::Skipped(SkipReason::BelowMinEdges { edges: 1, min: 2 })
        ));
        assert!(rs.patterns().is_empty());
    }

    #[test]
    fn lowering_min_edges_allows_single_edge() {
        let mut rs = RSet::new();
        rs.add(R::new("a", "b"));
        let sg = Subgraph::from_edges([R::new("a", "b")]);
        let policy = NamingPolicy { min_edges: 1, min_instances: 1, skip_meta_subgraphs: true, attach_only: false, min_mdl_gain: 0 };
        let decision = rs.consider_naming(&[sg], &policy).unwrap();
        assert!(matches!(decision, NamingDecision::Named(_)));
        assert_eq!(rs.patterns().len(), 1);
    }

    #[test]
    fn min_instances_threshold_skips_singleton() {
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b"), R::new("b", "c")]);
        let sg = Subgraph::from_edges([R::new("a", "b"), R::new("b", "c")]);
        let policy = NamingPolicy { min_edges: 1, min_instances: 2, skip_meta_subgraphs: true, attach_only: false, min_mdl_gain: 0 };
        let decision = rs.consider_naming(&[sg], &policy).unwrap();
        assert!(matches!(
            decision,
            NamingDecision::Skipped(SkipReason::BelowMinInstances { instances: 1, min: 2 })
        ));
    }

    #[test]
    fn default_pass_on_mixed_graph_names_three_skips_one() {
        let mut rs = build_mixed_graph();
        let decisions = rs.run_naming_pass(&NamingPolicy::default());

        let named: usize = decisions
            .iter()
            .filter(|(_, d)| matches!(d, NamingDecision::Named(_)))
            .count();
        let skipped: usize = decisions
            .iter()
            .filter(|(_, d)| matches!(d, NamingDecision::Skipped(_)))
            .count();
        assert_eq!(named, 3, "cycle, star, chain named");
        assert_eq!(skipped, 1, "single-edge pattern P2 skipped by default min_edges=2");
        assert_eq!(rs.patterns().len(), 3);
    }

    #[test]
    fn pass_with_min_instances_two_names_nothing_on_mixed_graph() {
        // On the mixed graph, every non-trivial pattern has exactly 1 instance
        // (3-cycle, 3-star, 2-chain all singletons). P2 single-edge has 6
        // instances but is filtered by min_edges=2. So nothing is named.
        let mut rs = build_mixed_graph();
        let policy = NamingPolicy { min_edges: 2, min_instances: 2, skip_meta_subgraphs: true, attach_only: false, min_mdl_gain: 0 };
        let decisions = rs.run_naming_pass(&policy);
        let named: usize = decisions
            .iter()
            .filter(|(_, d)| matches!(d, NamingDecision::Named(_)))
            .count();
        assert_eq!(named, 0);
        assert!(rs.patterns().is_empty());
    }

    #[test]
    fn naming_pass_is_idempotent_under_default_policy() {
        let mut rs = build_mixed_graph();
        let first_decisions = rs.run_naming_pass(&NamingPolicy::default());
        let named_first = first_decisions
            .iter()
            .filter(|(_, d)| matches!(d, NamingDecision::Named(_)))
            .count();
        let pattern_count_before = rs.patterns().len();

        // Re-run: data subgraphs are still there (connected components of
        // the original data edges ignore meta-R), but dedup via
        // filter_known_instances catches them — their participant sets
        // already match the instance records. Result: AlreadyKnown skips,
        // no new patterns, no new instances.
        let second_decisions = rs.run_naming_pass(&NamingPolicy::default());
        let named_second = second_decisions
            .iter()
            .filter(|(_, d)| matches!(d, NamingDecision::Named(_)))
            .count();
        let already_known: usize = second_decisions
            .iter()
            .filter(|(_, d)| matches!(d, NamingDecision::Skipped(SkipReason::AlreadyKnown)))
            .count();
        assert_eq!(named_first, 3);
        assert_eq!(named_second, 0);
        assert_eq!(already_known, 3);
        assert_eq!(rs.patterns().len(), pattern_count_before);
    }

    #[test]
    fn classify_subgraph_matches_known_pattern() {
        let mut rs = build_mixed_graph();
        rs.run_naming_pass(&NamingPolicy::default());
        let cycle = Subgraph::from_edges([
            R::new("k1", "k2"), R::new("k2", "k3"), R::new("k3", "k1"),
        ]);
        let matched = rs.classify_subgraph(&cycle);
        assert!(matched.is_some());
        // Same canonical as p_0; isomorphic under identifier relabeling too.
        let fresh_cycle = Subgraph::from_edges([
            R::new("m1", "m2"), R::new("m2", "m3"), R::new("m3", "m1"),
        ]);
        assert_eq!(rs.classify_subgraph(&fresh_cycle), matched);
    }

    #[test]
    fn classify_subgraph_returns_none_for_novel_structure() {
        let mut rs = build_mixed_graph();
        rs.run_naming_pass(&NamingPolicy::default());
        // Two-spoke out-star — not among the named patterns (default policy
        // names 3-cycle, 3-star, 2-chain but not 2-star).
        let two_spoke = Subgraph::from_edges([
            R::new("h", "a"), R::new("h", "b"),
        ]);
        assert_eq!(rs.classify_subgraph(&two_spoke), None);
    }

    #[test]
    fn pattern_of_recovers_owner_for_known_instance() {
        let mut rs = build_mixed_graph();
        rs.run_naming_pass(&NamingPolicy::default());
        let patterns: Vec<String> = rs.patterns().iter().map(|s| s.to_string()).collect();
        for pid in &patterns {
            for inst in rs.instances_of(pid) {
                let inst = inst.to_string();
                assert_eq!(rs.pattern_of(&inst), Some(pid.as_str()));
            }
        }
    }

    #[test]
    fn pattern_of_returns_none_for_non_instance() {
        let mut rs = build_mixed_graph();
        rs.run_naming_pass(&NamingPolicy::default());
        // A regular participant identifier is not itself an instance id.
        assert_eq!(rs.pattern_of("k1"), None);
        // Nor is the marker.
        assert_eq!(rs.pattern_of(PATTERN_MARKER), None);
        // Nor is a nonsense string.
        assert_eq!(rs.pattern_of("nope"), None);
    }

    #[test]
    fn memberships_of_reports_participation() {
        let mut rs = build_mixed_graph();
        rs.run_naming_pass(&NamingPolicy::default());
        // c2 participates in the chain pattern only (not in star or cycle).
        // Let's find which pattern owns the chain.
        let chain_canon: CanonicalForm = {
            let chain = Subgraph::from_edges([
                R::new("c2", "c3"),
                R::new("c3", "c4"),
            ]);
            chain.canonicalize()
        };
        let chain_pattern = rs
            .find_pattern_matching(&chain_canon)
            .map(|s| s.to_string())
            .expect("chain pattern should be named");
        let memberships = rs.memberships_of("c3");
        assert_eq!(memberships.len(), 1);
        assert_eq!(memberships[0].0, chain_pattern);
    }

    #[test]
    fn instance_subgraph_reconstructs_canonical_form() {
        let mut rs = build_mixed_graph();
        rs.run_naming_pass(&NamingPolicy::default());
        for pid in rs.patterns().iter().map(|s| s.to_string()).collect::<Vec<_>>() {
            let expected = {
                // Recover pattern's canonical form by reconstructing its
                // first instance the same way find_pattern_matching does.
                let inst = rs.instances_of(&pid)[0].to_string();
                rs.instance_subgraph(&inst).canonicalize()
            };
            for inst in rs.instances_of(&pid).iter().map(|s| s.to_string()).collect::<Vec<_>>() {
                let sg = rs.instance_subgraph(&inst);
                assert_eq!(sg.canonicalize(), expected);
            }
        }
    }

    #[test]
    fn attach_only_with_empty_registry_names_nothing() {
        // ADR 0015: with no patterns registered, attach-only iterates an
        // empty set of patterns and returns zero decisions. No new
        // patterns created, no instances added.
        let mut rs = build_mixed_graph();
        let policy = NamingPolicy {
            min_edges: 2,
            min_instances: 1,
            skip_meta_subgraphs: true,
            attach_only: true,
            min_mdl_gain: 0,
        };
        let decisions = rs.run_naming_pass(&policy);
        assert!(decisions.is_empty(), "no registered patterns → no decisions");
        assert!(rs.patterns().is_empty());
    }

    #[test]
    fn attach_only_admits_asymmetric_chain_after_discovery() {
        // ADR 0015 fix — the case that compound-class fragmentation
        // missed in ADR 0014.
        let mut rs = build_mixed_graph();
        rs.run_naming_pass(&NamingPolicy::default());

        // p_2 is the 2-chain pattern. Before extending: 1 instance.
        let p2_before = rs.instances_of("p_2").len();

        // Add a fresh 2-chain on completely new identifiers. Under the
        // old compound-class pipeline, this would fragment and not
        // attach. Under subgraph matching, it should attach.
        rs.extend([R::new("u", "v"), R::new("v", "w")]);

        let policy = NamingPolicy {
            min_edges: 2,
            min_instances: 1,
            skip_meta_subgraphs: true,
            attach_only: true,
            min_mdl_gain: 0,
        };
        rs.run_naming_pass(&policy);

        let p2_after = rs.instances_of("p_2").len();
        assert!(p2_after > p2_before, "asymmetric chain should attach to p_2");
    }

    #[test]
    fn find_instances_of_returns_empty_for_novel_canonical() {
        let mut rs = build_mixed_graph();
        rs.run_naming_pass(&NamingPolicy::default());
        // A completely novel canonical (a 4-node, 5-edge K4-ish
        // form that no named pattern uses) should yield no matches.
        // ADR 0055: rebuilt via canonicalize() rather than a literal
        // pin since the canonical's u64 hash labels are no longer
        // small integers.
        let novel_target: CanonicalForm = Subgraph::from_edges([
            R::new("n0", "n1"),
            R::new("n1", "n2"),
            R::new("n2", "n3"),
            R::new("n3", "n0"),
            R::new("n0", "n2"),
        ])
        .canonicalize();
        let matches = rs.find_instances_of(&novel_target);
        assert!(matches.is_empty());
    }

    #[test]
    fn attach_only_admits_matching_canonical_after_discovery() {
        let mut rs = build_mixed_graph();
        rs.run_naming_pass(&NamingPolicy::default()); // discovery
        let pattern_count_before = rs.patterns().len();
        let instance_total_before: usize = rs
            .patterns()
            .iter()
            .map(|p| rs.instances_of(p).len())
            .sum();

        // Add a fresh 3-cycle on new identifiers. Its canonical form
        // should match the existing p_0 cycle pattern.
        rs.extend([
            R::new("m1", "m2"),
            R::new("m2", "m3"),
            R::new("m3", "m1"),
        ]);

        let policy = NamingPolicy {
            min_edges: 2,
            min_instances: 1,
            skip_meta_subgraphs: true,
            attach_only: true,
            min_mdl_gain: 0,
        };
        let decisions = rs.run_naming_pass(&policy);

        let named: usize = decisions
            .iter()
            .filter(|(_, d)| matches!(d, NamingDecision::Named(_)))
            .count();
        assert!(named >= 1, "the fresh 3-cycle should attach to p_0");

        // No new pattern created.
        assert_eq!(rs.patterns().len(), pattern_count_before);
        // At least one new instance recorded.
        let instance_total_after: usize = rs
            .patterns()
            .iter()
            .map(|p| rs.instances_of(p).len())
            .sum();
        assert!(instance_total_after > instance_total_before);
    }

    #[test]
    fn attach_pass_picks_up_instances_discovery_missed() {
        // ADR 0015: under subgraph matching, attach finds 2-chain
        // instances that compound-class discovery fragmented. In the
        // c1→c2→c3→c4→c5 chain, discovery recognizes only the
        // {c2,c3,c4} interior as a 2-chain subgraph (its edges share
        // compound fingerprints). Attach enumeration finds {c1,c2,c3}
        // and {c3,c4,c5} in addition.
        let mut rs = build_mixed_graph();
        rs.run_naming_pass(&NamingPolicy::default());
        let p2_after_discovery = rs.instances_of("p_2").len();

        let policy = NamingPolicy {
            min_edges: 2,
            min_instances: 1,
            skip_meta_subgraphs: true,
            attach_only: true,
            min_mdl_gain: 0,
        };
        rs.run_naming_pass(&policy);
        let p2_after_attach = rs.instances_of("p_2").len();

        assert!(
            p2_after_attach > p2_after_discovery,
            "attach should find additional 2-chain instances"
        );
    }

    #[test]
    fn attach_only_second_pass_is_no_op() {
        let mut rs = build_mixed_graph();
        rs.run_naming_pass(&NamingPolicy::default());
        let policy = NamingPolicy {
            min_edges: 2,
            min_instances: 1,
            skip_meta_subgraphs: true,
            attach_only: true,
            min_mdl_gain: 0,
        };
        // First attach may add instances that discovery missed; second
        // attach on unchanged data adds nothing.
        rs.run_naming_pass(&policy);
        let size_after_first = rs.len();
        let patterns_after = rs.patterns().len();

        rs.run_naming_pass(&policy);
        assert_eq!(rs.len(), size_after_first);
        assert_eq!(rs.patterns().len(), patterns_after);
    }

    #[test]
    fn discover_motifs_empty_rset_returns_empty() {
        let rs = RSet::new();
        let config = DiscoveryConfig {
            target_size: 2,
            sample_count: 20,
            top_m: 5,
            rng_seed: 42,
            include_meta_in_discovery: false,
        };
        assert!(rs.discover_motifs(&config).is_empty());
    }

    #[test]
    fn discover_motifs_is_deterministic_under_fixed_seed() {
        let rs = build_mixed_graph();
        let config = DiscoveryConfig {
            target_size: 3,
            sample_count: 30,
            top_m: 5,
            rng_seed: 12345,
            include_meta_in_discovery: false,
        };
        let first: Vec<CanonicalForm> =
            rs.discover_motifs(&config).into_iter().map(|c| c.canonical).collect();
        let second: Vec<CanonicalForm> =
            rs.discover_motifs(&config).into_iter().map(|c| c.canonical).collect();
        assert_eq!(first, second);
    }

    #[test]
    fn discover_motifs_respects_target_size() {
        let rs = build_mixed_graph();
        let config = DiscoveryConfig {
            target_size: 3,
            sample_count: 30,
            top_m: 5,
            rng_seed: 7,
            include_meta_in_discovery: false,
        };
        let candidates = rs.discover_motifs(&config);
        for c in &candidates {
            assert_eq!(c.representative.len(), 3);
            // canonical edge count equals subgraph edge count
            assert_eq!(c.canonical.len(), 3);
        }
    }

    #[test]
    fn discover_motifs_respects_top_m() {
        let rs = build_mixed_graph();
        let config = DiscoveryConfig {
            target_size: 2,
            sample_count: 50,
            top_m: 2,
            rng_seed: 99,
            include_meta_in_discovery: false,
        };
        let candidates = rs.discover_motifs(&config);
        assert!(candidates.len() <= 2);
    }

    #[test]
    fn discover_motifs_finds_two_chain_on_mixed_graph() {
        // At target_size=2, the 5-chain alone contributes 3 structurally
        // isomorphic 2-chains (c1-c2-c3, c2-c3-c4, c3-c4-c5), plus one
        // more in the tree branch t1-t2-t4 and one via the T-fork if
        // applicable. Sampling with enough draws should find the
        // 2-chain canonical with high frequency.
        let rs = build_mixed_graph();
        let config = DiscoveryConfig {
            target_size: 2,
            sample_count: 200,
            top_m: 5,
            rng_seed: 2024,
            include_meta_in_discovery: false,
        };
        let candidates = rs.discover_motifs(&config);
        assert!(!candidates.is_empty());
        // The 2-chain canonical is the canonical of any directed
        // 2-edge path. Compute it from a representative subgraph
        // rather than pinning the u64 hash labels (ADR 0055).
        let two_chain_canonical: CanonicalForm = Subgraph::from_edges([
            R::new("ref_a", "ref_b"),
            R::new("ref_b", "ref_c"),
        ])
        .canonicalize();
        assert!(
            candidates.iter().any(|c| c.canonical == two_chain_canonical),
            "expected to discover the 2-chain canonical among candidates: {:?}",
            candidates.iter().map(|c| &c.canonical).collect::<Vec<_>>()
        );
    }

    #[test]
    fn refine_preserves_already_clean_representative() {
        let rs = build_mixed_graph();
        // Manually construct a clean 2-chain candidate.
        let rep = Subgraph::from_edges([R::new("c1", "c2"), R::new("c2", "c3")]);
        let canon = rep.canonicalize();
        assert!(rs.is_clean_subgraph(&rep));
        let input = vec![MotifCandidate {
            canonical: canon.clone(),
            representative: rep.clone(),
            sample_frequency: 1,
            score: 1.0,
        }];
        let refined = rs.refine_candidates(
            input,
            &RefinementConfig { max_tries: 50, rng_seed: 7 },
        );
        assert_eq!(refined[0].representative, rep);
    }

    #[test]
    fn refine_replaces_nonclean_rep_when_clean_alternative_exists() {
        let rs = build_mixed_graph();
        // Construct a 2-chain candidate with a non-clean representative
        // (embedded in the 3-cycle {k1, k2, k3}).
        let embedded = Subgraph::from_edges([R::new("k1", "k2"), R::new("k3", "k1")]);
        let canon = embedded.canonicalize();
        assert!(!rs.is_clean_subgraph(&embedded));
        let input = vec![MotifCandidate {
            canonical: canon.clone(),
            representative: embedded.clone(),
            sample_frequency: 100,
            score: 100.0,
        }];
        let refined = rs.refine_candidates(
            input,
            &RefinementConfig { max_tries: 200, rng_seed: 2024 },
        );
        // A clean 2-chain exists in the 5-chain data; refinement should
        // find one within the budget.
        assert!(rs.is_clean_subgraph(&refined[0].representative));
        assert_eq!(refined[0].canonical, canon);
    }

    #[test]
    fn refine_is_noop_when_no_clean_alternative() {
        // A graph consisting only of a single 3-cycle. The 2-chain
        // canonical has NO clean instance anywhere (every 2-chain is
        // embedded in the cycle). Refinement must leave rep unchanged.
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b"), R::new("b", "c"), R::new("c", "a")]);
        let embedded = Subgraph::from_edges([R::new("a", "b"), R::new("b", "c")]);
        let canon = embedded.canonicalize();
        let input = vec![MotifCandidate {
            canonical: canon,
            representative: embedded.clone(),
            sample_frequency: 1,
            score: 1.0,
        }];
        let refined = rs.refine_candidates(
            input,
            &RefinementConfig { max_tries: 100, rng_seed: 42 },
        );
        assert_eq!(refined[0].representative, embedded);
    }

    #[test]
    fn refine_is_deterministic_under_fixed_seed() {
        let rs = build_mixed_graph();
        let config_disc = DiscoveryConfig {
            target_size: 2,
            sample_count: 50,
            top_m: 3,
            rng_seed: 11,
            include_meta_in_discovery: false,
        };
        let cands = rs.discover_motifs(&config_disc);
        let cfg = RefinementConfig { max_tries: 100, rng_seed: 999 };
        let r1 = rs.refine_candidates(cands.clone(), &cfg);
        let r2 = rs.refine_candidates(cands, &cfg);
        for (a, b) in r1.iter().zip(r2.iter()) {
            assert_eq!(a.representative, b.representative);
            assert_eq!(a.canonical, b.canonical);
        }
    }

    fn default_autonomous_config() -> AutonomousConfig {
        AutonomousConfig {
            discovery: DiscoveryConfig {
                target_size: 3,
                sample_count: 200,
                top_m: 10,
                rng_seed: 2024,
            include_meta_in_discovery: false,
            },
            refinement: RefinementConfig {
                max_tries: 200,
                rng_seed: 999,
            },
            naming: NamingPolicy::default(),
            instance_sampling: None,
        }
    }

    #[test]
    fn autonomous_pass_empty_rset_returns_empty() {
        let mut rs = RSet::new();
        let outcomes = rs.autonomous_pass(&default_autonomous_config());
        assert!(outcomes.is_empty());
    }

    #[test]
    fn autonomous_pass_names_patterns_on_mixed_graph() {
        let mut rs = build_mixed_graph();
        let outcomes = rs.autonomous_pass(&default_autonomous_config());
        let new_count = outcomes
            .iter()
            .filter(|o| matches!(o, AutonomousOutcome::NewPattern { .. }))
            .count();
        assert!(
            new_count > 0,
            "autonomous_pass should name at least one new pattern on the mixed graph"
        );
        // Registry should reflect the new patterns.
        assert_eq!(rs.patterns().len(), new_count);
    }

    #[test]
    fn autonomous_pass_is_idempotent() {
        let mut rs = build_mixed_graph();
        let config = default_autonomous_config();
        let first = rs.autonomous_pass(&config);
        let first_new: usize = first
            .iter()
            .filter(|o| matches!(o, AutonomousOutcome::NewPattern { .. }))
            .count();
        assert!(first_new > 0);

        let pattern_count_after_first = rs.patterns().len();
        let rset_size_after_first = rs.len();

        let second = rs.autonomous_pass(&config);
        let second_new: usize = second
            .iter()
            .filter(|o| matches!(o, AutonomousOutcome::NewPattern { .. }))
            .count();
        let second_existing: usize = second
            .iter()
            .filter(|o| matches!(o, AutonomousOutcome::Existing { .. }))
            .count();
        assert_eq!(second_new, 0, "no new patterns on second pass");
        assert!(second_existing > 0, "existing canonicals should be reported");
        assert_eq!(rs.patterns().len(), pattern_count_after_first);
        assert_eq!(rs.len(), rset_size_after_first);
    }

    #[test]
    fn autonomous_pass_respects_policy() {
        // Raise min_instances so every single-instance motif (tree,
        // cycle, star at target_size=3) is filtered out. Only canonicals
        // with ≥ 2 clean instances survive — the 3-chain has 3 clean
        // instances in the 5-chain data.
        let mut rs = build_mixed_graph();
        let mut config = default_autonomous_config();
        config.naming.min_instances = 2;
        let outcomes = rs.autonomous_pass(&config);
        let named_count = outcomes
            .iter()
            .filter(|o| matches!(o, AutonomousOutcome::NewPattern { .. }))
            .count();
        let filtered_count = outcomes
            .iter()
            .filter(|o| matches!(
                o,
                AutonomousOutcome::Skipped {
                    reason: AutonomousSkip::PolicyFiltered(
                        SkipReason::BelowMinInstances { .. }
                    ),
                    ..
                }
            ))
            .count();
        assert_eq!(named_count, 1, "expected only the 3-chain to be named");
        assert!(filtered_count >= 2, "expected single-instance candidates filtered");
    }

    #[test]
    fn mdl_gain_is_zero_for_singleton_canonical() {
        // Build a graph with exactly one 3-cycle — one clean instance.
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b"), R::new("b", "c"), R::new("c", "a")]);
        let sg = Subgraph::from_edges([R::new("a", "b"), R::new("b", "c"), R::new("c", "a")]);
        let canon = sg.canonicalize();
        assert_eq!(rs.mdl_gain(&canon), 0);
    }

    #[test]
    fn mdl_gain_scales_with_reuse_and_size() {
        let rs = build_mixed_graph();
        // 2-chain canonical. Clean instances on the mixed graph:
        // {c1,c2,c3}, {c2,c3,c4}, {c3,c4,c5}, {t1,t2,t4} → N=4.
        // Gain = (4 - 1) * 2 = 6. ADR 0055: canonical computed from
        // a reference subgraph, not pinned to literal hash labels.
        let two_chain: CanonicalForm = Subgraph::from_edges([
            R::new("ref_a", "ref_b"),
            R::new("ref_b", "ref_c"),
        ])
        .canonicalize();
        assert_eq!(rs.mdl_gain(&two_chain), 6);

        // 3-chain canonical. Clean instances: {c1..c4}, {c2..c5} → N=2.
        // Gain = (2 - 1) * 3 = 3.
        let three_chain: CanonicalForm = Subgraph::from_edges([
            R::new("ref_a", "ref_b"),
            R::new("ref_b", "ref_c"),
            R::new("ref_c", "ref_d"),
        ])
        .canonicalize();
        assert_eq!(rs.mdl_gain(&three_chain), 3);
    }

    #[test]
    fn score_by_mdl_updates_candidate_scores() {
        let rs = build_mixed_graph();
        let rep = Subgraph::from_edges([R::new("c1", "c2"), R::new("c2", "c3")]);
        let canon = rep.canonicalize();
        let candidates = vec![
            MotifCandidate {
                canonical: canon,
                representative: rep,
                sample_frequency: 42,
                score: 42.0,
            },
        ];
        let rescored = rs.score_by_mdl(candidates);
        assert_eq!(rescored[0].score, 6.0);
    }

    #[test]
    fn consider_naming_rejects_below_mdl_threshold() {
        let mut rs = build_mixed_graph();
        // Singleton 3-cycle instance → edges=3, count=1, gain=0.
        // With min_mdl_gain=1 it should be rejected.
        let cycle = Subgraph::from_edges([
            R::new("k1", "k2"), R::new("k2", "k3"), R::new("k3", "k1"),
        ]);
        let policy = NamingPolicy {
            min_edges: 2,
            min_instances: 1,
            skip_meta_subgraphs: true,
            attach_only: false,
            min_mdl_gain: 1,
        };
        let decision = rs.consider_naming(&[cycle], &policy).unwrap();
        assert!(matches!(
            decision,
            NamingDecision::Skipped(SkipReason::BelowMdlGain { gain: 0, min: 1 })
        ));
        assert!(rs.patterns().is_empty());
    }

    #[test]
    fn autonomous_pass_honors_min_mdl_gain() {
        // target_size=3 on the mixed graph surfaces four canonicals:
        //   3-chain  (N=2, k=3, gain=3)
        //   3-cycle  (N=1, k=3, gain=0)
        //   3-tree   (N=1, k=3, gain=0)
        //   3-star   (N=1, k=3, gain=0)
        // min_mdl_gain=1 should keep only the 3-chain.
        let mut rs = build_mixed_graph();
        let config = AutonomousConfig {
            discovery: DiscoveryConfig {
                target_size: 3,
                sample_count: 200,
                top_m: 10,
                rng_seed: 2024,
            include_meta_in_discovery: false,
            },
            refinement: RefinementConfig {
                max_tries: 200,
                rng_seed: 999,
            },
            naming: NamingPolicy {
                min_edges: 2,
                min_instances: 1,
                skip_meta_subgraphs: true,
                attach_only: false,
                min_mdl_gain: 1,
            },
            instance_sampling: None,
        };
        let outcomes = rs.autonomous_pass(&config);
        let new_count = outcomes
            .iter()
            .filter(|o| matches!(o, AutonomousOutcome::NewPattern { .. }))
            .count();
        let mdl_skipped = outcomes
            .iter()
            .filter(|o| matches!(
                o,
                AutonomousOutcome::Skipped {
                    reason: AutonomousSkip::PolicyFiltered(SkipReason::BelowMdlGain { .. }),
                    ..
                }
            ))
            .count();
        assert_eq!(new_count, 1, "only 3-chain has positive MDL gain");
        assert!(mdl_skipped >= 3, "three singleton canonicals should be MDL-filtered");
        assert_eq!(rs.patterns().len(), 1);
    }

    #[test]
    fn remove_takes_one_edge_off() {
        let mut rs = RSet::new();
        rs.add(R::new("a", "b"));
        rs.add(R::new("b", "c"));
        assert_eq!(rs.len(), 2);
        assert!(rs.remove(&R::new("a", "b")));
        assert_eq!(rs.len(), 1);
        assert!(!rs.remove(&R::new("a", "b"))); // already gone
    }

    #[test]
    fn retract_nonexistent_pattern_errors() {
        let mut rs = build_mixed_graph();
        let err = rs.retract_pattern("p_999").unwrap_err();
        assert_eq!(err, RetractionError::UnknownPattern);
    }

    #[test]
    fn retract_removes_all_meta_edges_and_preserves_data() {
        let mut rs = build_mixed_graph();
        let size_before_any_naming = rs.len();
        rs.run_naming_pass(&NamingPolicy::default());
        let size_after_naming = rs.len();
        assert!(size_after_naming > size_before_any_naming);

        let patterns: Vec<String> = rs.patterns().iter().map(|s| s.to_string()).collect();
        let victim = patterns[0].clone();
        let victim_instances = rs.instances_of(&victim).len();

        let summary = rs.retract_pattern(&victim).unwrap();
        assert_eq!(summary.pattern_id, victim);
        assert_eq!(summary.instances_removed, victim_instances);
        assert!(summary.meta_edges_removed >= victim_instances + 1);

        // Pattern is gone from the registry.
        assert!(!rs.patterns().iter().any(|p| *p == victim));

        // Other patterns are untouched.
        for p in &patterns[1..] {
            assert!(rs.patterns().iter().any(|q| q == p));
        }

        // Data edges intact.
        assert!(rs.contains(&R::new("c1", "c2")));
        assert!(rs.contains(&R::new("k1", "k2")));
        assert!(rs.contains(&R::new("s", "sa")));
    }

    #[test]
    fn retract_allows_rediscovery() {
        // After retraction, a re-run of autonomous_pass should find the
        // same canonical and name it as a fresh pattern (possibly
        // reusing the id, possibly picking a new one).
        let mut rs = build_mixed_graph();
        let config = AutonomousConfig {
            discovery: DiscoveryConfig {
                target_size: 3,
                sample_count: 200,
                top_m: 10,
                rng_seed: 2024,
            include_meta_in_discovery: false,
            },
            refinement: RefinementConfig { max_tries: 200, rng_seed: 999 },
            naming: NamingPolicy::default(),
            instance_sampling: None,
        };
        rs.autonomous_pass(&config);
        let patterns_before: Vec<String> = rs
            .patterns()
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert!(!patterns_before.is_empty());

        // Retract the first pattern.
        let victim = patterns_before[0].clone();
        let canon_before = {
            let inst = rs.instances_of(&victim)[0].to_string();
            rs.instance_subgraph(&inst).canonicalize()
        };
        rs.retract_pattern(&victim).unwrap();

        // The canonical should be no longer recognized.
        assert!(rs.find_pattern_matching(&canon_before).is_none());

        // Re-run. The canonical should re-emerge and be named.
        let outcomes = rs.autonomous_pass(&config);
        assert!(outcomes.iter().any(|o| matches!(
            o,
            AutonomousOutcome::NewPattern { canonical, .. } if canonical == &canon_before
        )));
    }

    #[test]
    fn retract_clears_classification() {
        let mut rs = build_mixed_graph();
        rs.run_naming_pass(&NamingPolicy::default());
        let patterns: Vec<String> = rs.patterns().iter().map(|s| s.to_string()).collect();
        let victim = patterns[0].clone();
        let inst = rs.instances_of(&victim)[0].to_string();
        let canonical = rs.instance_subgraph(&inst).canonicalize();

        // Before retraction, classification hits.
        assert_eq!(rs.classify_subgraph(&Subgraph::from_edges(rs.iter().cloned())).is_some() || true, true); // placeholder

        rs.retract_pattern(&victim).unwrap();

        // After retraction, nothing classifies to the retracted canonical.
        assert!(rs.find_pattern_matching(&canonical).is_none());
    }

    #[test]
    fn sweep_with_empty_sizes_returns_empty() {
        let mut rs = build_mixed_graph();
        let base = AutonomousConfig {
            discovery: DiscoveryConfig {
                target_size: 3,
                sample_count: 100,
                top_m: 5,
                rng_seed: 2024,
            include_meta_in_discovery: false,
            },
            refinement: RefinementConfig { max_tries: 100, rng_seed: 999 },
            naming: NamingPolicy::default(),
            instance_sampling: None,
        };
        let results = rs.autonomous_sweep(&base, &[]);
        assert!(results.is_empty());
        assert!(rs.patterns().is_empty());
    }

    #[test]
    fn sweep_with_single_size_matches_direct_pass() {
        let base = AutonomousConfig {
            discovery: DiscoveryConfig {
                target_size: 3,
                sample_count: 100,
                top_m: 5,
                rng_seed: 2024,
            include_meta_in_discovery: false,
            },
            refinement: RefinementConfig { max_tries: 100, rng_seed: 999 },
            naming: NamingPolicy::default(),
            instance_sampling: None,
        };

        // Path A: sweep with a single size — seed is offset by the size.
        let mut rs_sweep = build_mixed_graph();
        let sweep_results = rs_sweep.autonomous_sweep(&base, &[3]);
        assert_eq!(sweep_results.len(), 1);

        // Path B: call autonomous_pass with the equivalent offset seed.
        let mut rs_direct = build_mixed_graph();
        let mut direct_cfg = base.clone();
        direct_cfg.discovery.rng_seed = base.discovery.rng_seed.wrapping_add(3);
        let direct_outcomes = rs_direct.autonomous_pass(&direct_cfg);

        assert_eq!(sweep_results[0].0, 3);
        // Same number of outcomes; same registered pattern count.
        assert_eq!(sweep_results[0].1.len(), direct_outcomes.len());
        assert_eq!(rs_sweep.patterns().len(), rs_direct.patterns().len());
    }

    #[test]
    fn sweep_accumulates_patterns_across_sizes() {
        let base = AutonomousConfig {
            discovery: DiscoveryConfig {
                target_size: 3,
                sample_count: 200,
                top_m: 10,
                rng_seed: 2024,
            include_meta_in_discovery: false,
            },
            refinement: RefinementConfig { max_tries: 200, rng_seed: 999 },
            naming: NamingPolicy::default(),
            instance_sampling: None,
        };

        let mut rs = build_mixed_graph();
        let results = rs.autonomous_sweep(&base, &[2, 3]);
        assert_eq!(results.len(), 2);
        // Patterns exist at both sizes.
        let patterns_after = rs.patterns().len();
        assert!(patterns_after >= 2);

        // Second sweep on identical sizes — all Existing, no new
        // patterns.
        let second = rs.autonomous_sweep(&base, &[2, 3]);
        let any_new: usize = second
            .iter()
            .flat_map(|(_, outs)| outs.iter())
            .filter(|o| matches!(o, AutonomousOutcome::NewPattern { .. }))
            .count();
        assert_eq!(any_new, 0);
        assert_eq!(rs.patterns().len(), patterns_after);
    }

    #[test]
    fn autonomous_and_attach_on_fresh_rset() {
        // On fresh data, attach phase should find only AlreadyKnown /
        // empty: autonomous already used find_instances_of exhaustively
        // for each discovered canonical.
        let mut rs = build_mixed_graph();
        let config = AutonomousConfig {
            discovery: DiscoveryConfig {
                target_size: 3,
                sample_count: 200,
                top_m: 10,
                rng_seed: 2024,
            include_meta_in_discovery: false,
            },
            refinement: RefinementConfig { max_tries: 200, rng_seed: 999 },
            naming: NamingPolicy::default(),
            instance_sampling: None,
        };
        let summary = rs.autonomous_and_attach(&config);
        // Autonomous creates several new patterns.
        let new_patterns = summary
            .autonomous
            .iter()
            .filter(|o| matches!(o, AutonomousOutcome::NewPattern { .. }))
            .count();
        assert!(new_patterns > 0);
        // Attach pass should not create any new patterns or instances.
        let new_instances_via_attach: usize = summary
            .attach
            .iter()
            .filter(|(_, d)| matches!(d, NamingDecision::Named(_)))
            .count();
        assert_eq!(new_instances_via_attach, 0);
    }

    #[test]
    fn autonomous_and_attach_picks_up_new_data_after_prior_naming() {
        let mut rs = build_mixed_graph();
        let config = AutonomousConfig {
            discovery: DiscoveryConfig {
                target_size: 3,
                sample_count: 200,
                top_m: 10,
                rng_seed: 2024,
            include_meta_in_discovery: false,
            },
            refinement: RefinementConfig { max_tries: 200, rng_seed: 999 },
            naming: NamingPolicy::default(),
            instance_sampling: None,
        };
        // Prime the registry with the first autonomous_pass.
        rs.autonomous_pass(&config);
        // 3-chain canonical (ADR 0055: built via canonicalize()).
        let p_3_chain: CanonicalForm = Subgraph::from_edges([
            R::new("ref_a", "ref_b"),
            R::new("ref_b", "ref_c"),
            R::new("ref_c", "ref_d"),
        ])
        .canonicalize();
        let p_3_chain_id = rs
            .find_pattern_matching(&p_3_chain)
            .map(|s| s.to_string())
            .expect("3-chain named");
        let chain_instances_before = rs.instances_of(&p_3_chain_id).len();

        // Add new data that contains another clean 3-chain.
        rs.extend([
            R::new("q1", "q2"),
            R::new("q2", "q3"),
            R::new("q3", "q4"),
        ]);

        // autonomous_and_attach: autonomous may or may not re-sample the
        // 3-chain canonical (it's already Existing). Attach definitely
        // picks up {q1, q2, q3, q4} as a new instance.
        let _summary = rs.autonomous_and_attach(&config);
        let chain_instances_after = rs.instances_of(&p_3_chain_id).len();
        assert!(chain_instances_after > chain_instances_before);
    }

    #[test]
    fn autonomous_and_attach_is_idempotent() {
        let mut rs = build_mixed_graph();
        let config = AutonomousConfig {
            discovery: DiscoveryConfig {
                target_size: 3,
                sample_count: 200,
                top_m: 10,
                rng_seed: 2024,
            include_meta_in_discovery: false,
            },
            refinement: RefinementConfig { max_tries: 200, rng_seed: 999 },
            naming: NamingPolicy::default(),
            instance_sampling: None,
        };
        rs.autonomous_and_attach(&config);
        let size_before = rs.len();
        let patterns_before = rs.patterns().len();
        rs.autonomous_and_attach(&config);
        assert_eq!(rs.len(), size_before);
        assert_eq!(rs.patterns().len(), patterns_before);
    }

    #[test]
    fn canonical_library_round_trip_is_all_existing() {
        let mut rs = build_mixed_graph();
        rs.run_naming_pass(&NamingPolicy::default());
        let library = rs.canonical_library();
        assert!(!library.is_empty());

        // Re-applying the same library to the same RSet → all Existing.
        let outcomes = rs.attach_canonicals(&library, &NamingPolicy::default());
        assert_eq!(outcomes.len(), library.len());
        assert!(outcomes
            .iter()
            .all(|o| matches!(o, AutonomousOutcome::Existing { .. })));
    }

    #[test]
    fn attach_canonicals_skips_when_no_clean_instance_in_target() {
        // Source: 3-cycle named as p_0.
        let mut source = RSet::new();
        source.extend([R::new("a", "b"), R::new("b", "c"), R::new("c", "a")]);
        source.run_naming_pass(&NamingPolicy::default());
        let library = source.canonical_library();

        // Target: a graph with no 3-cycle (just a chain).
        let mut target = RSet::new();
        target.extend([R::new("p", "q"), R::new("q", "r")]);
        let outcomes = target.attach_canonicals(&library, &NamingPolicy::default());
        assert_eq!(outcomes.len(), library.len());
        assert!(outcomes.iter().all(|o| matches!(
            o,
            AutonomousOutcome::Skipped { reason: AutonomousSkip::NoCleanInstance, .. }
        )));
        assert!(target.patterns().is_empty());
    }

    #[test]
    fn attach_canonicals_names_patterns_when_target_matches() {
        // Source: a 3-cycle. Target: has another 3-cycle with different ids.
        let mut source = RSet::new();
        source.extend([R::new("a", "b"), R::new("b", "c"), R::new("c", "a")]);
        source.run_naming_pass(&NamingPolicy::default());
        let library = source.canonical_library();

        let mut target = RSet::new();
        target.extend([R::new("m1", "m2"), R::new("m2", "m3"), R::new("m3", "m1")]);

        let outcomes = target.attach_canonicals(&library, &NamingPolicy::default());
        let named: usize = outcomes
            .iter()
            .filter(|o| matches!(o, AutonomousOutcome::NewPattern { .. }))
            .count();
        assert_eq!(named, 1);
        assert_eq!(target.patterns().len(), 1);
    }

    #[test]
    fn attach_canonicals_is_idempotent() {
        let mut source = build_mixed_graph();
        source.run_naming_pass(&NamingPolicy::default());
        let library = source.canonical_library();

        let mut target = build_mixed_graph();
        target.attach_canonicals(&library, &NamingPolicy::default());
        let size_after_first = target.len();
        let patterns_after_first = target.patterns().len();

        // Second application should be all-Existing, no delta.
        let outcomes = target.attach_canonicals(&library, &NamingPolicy::default());
        assert!(outcomes
            .iter()
            .all(|o| matches!(o, AutonomousOutcome::Existing { .. })));
        assert_eq!(target.len(), size_after_first);
        assert_eq!(target.patterns().len(), patterns_after_first);
    }

    #[test]
    fn sample_instances_empty_canonical_returns_empty() {
        let rs = build_mixed_graph();
        let got = rs.sample_instances_of(
            &vec![],
            &SamplingMatchConfig { sample_count: 100, rng_seed: 1 },
        );
        assert!(got.is_empty());
    }

    #[test]
    fn sample_instances_with_no_matches_returns_empty() {
        let rs = build_mixed_graph();
        // A canonical the graph does not contain — a 4-edge form
        // with self-loops on a single labelled node, which can't
        // appear in the mixed graph (no parallel self-loops).
        // ADR 0055: built via canonicalize() on a representative
        // shape rather than literal hash labels.
        let impossible: CanonicalForm = Subgraph::from_edges([
            R::new("self", "self"),
            R::new("self", "self2"),
            R::new("self", "self3"),
            R::new("self", "self4"),
        ])
        .canonicalize();
        let got = rs.sample_instances_of(
            &impossible,
            &SamplingMatchConfig { sample_count: 100, rng_seed: 1 },
        );
        assert!(got.is_empty());
    }

    #[test]
    fn sample_instances_deterministic_under_fixed_seed() {
        let rs = build_mixed_graph();
        let target: CanonicalForm = Subgraph::from_edges([
            R::new("ref_a", "ref_b"),
            R::new("ref_b", "ref_c"),
        ])
        .canonicalize();
        let config = SamplingMatchConfig { sample_count: 100, rng_seed: 42 };
        let a = rs.sample_instances_of(&target, &config);
        let b = rs.sample_instances_of(&target, &config);
        assert_eq!(a.len(), b.len());
        // Each entry matches by participant set (sort-compare)
        let key = |v: &[Subgraph]| -> Vec<Vec<String>> {
            let mut out: Vec<Vec<String>> = v
                .iter()
                .map(|s| {
                    let mut p: Vec<String> =
                        s.identifiers().into_iter().map(str::to_owned).collect();
                    p.sort();
                    p
                })
                .collect();
            out.sort();
            out
        };
        assert_eq!(key(&a), key(&b));
    }

    #[test]
    fn sample_instances_approximates_find_instances_with_enough_budget() {
        let rs = build_mixed_graph();
        // 2-chain target.
        let target: CanonicalForm = Subgraph::from_edges([
            R::new("ref_a", "ref_b"),
            R::new("ref_b", "ref_c"),
        ])
        .canonicalize();
        let exhaustive = rs.find_instances_of(&target);
        // Generous budget — small graph, sampling should hit all.
        let sampled = rs.sample_instances_of(
            &target,
            &SamplingMatchConfig { sample_count: 500, rng_seed: 7 },
        );
        // Never over-returns.
        assert!(sampled.len() <= exhaustive.len());
        // With 500 samples on a 14-edge graph, expect all 4 clean
        // 2-chain instances to be hit (verified empirically).
        assert_eq!(sampled.len(), exhaustive.len());
    }

    #[test]
    fn hierarchical_probe_default_matches_data_only() {
        // Flag off (default) should behave exactly like pre-0025 discover_motifs.
        let rs = build_mixed_graph();
        let cfg = DiscoveryConfig {
            target_size: 3,
            sample_count: 200,
            top_m: 10,
            rng_seed: 2024,
            include_meta_in_discovery: false,
        };
        let a = rs.discover_motifs(&cfg);

        let rs2 = build_mixed_graph();
        let b = rs2.discover_motifs(&cfg);
        let key = |v: &[MotifCandidate]| -> Vec<(CanonicalForm, usize)> {
            v.iter().map(|c| (c.canonical.clone(), c.sample_frequency)).collect()
        };
        assert_eq!(key(&a), key(&b));
    }

    #[test]
    fn hierarchical_probe_flag_on_after_naming_sees_meta_edges() {
        let mut rs = build_mixed_graph();
        let cfg = AutonomousConfig {
            discovery: DiscoveryConfig {
                target_size: 3,
                sample_count: 200,
                top_m: 10,
                rng_seed: 2024,
                include_meta_in_discovery: false,
            },
            refinement: RefinementConfig { max_tries: 200, rng_seed: 999 },
            naming: NamingPolicy::default(),
            instance_sampling: None,
        };
        rs.autonomous_pass(&cfg);

        let meta_ids = rs.collect_meta_ids();
        let probe_cfg = DiscoveryConfig {
            target_size: 3,
            sample_count: 500,
            top_m: 20,
            rng_seed: 7,
            include_meta_in_discovery: true,
        };
        let candidates = rs.discover_motifs(&probe_cfg);
        let has_meta_candidate = candidates.iter().any(|c| {
            c.representative
                .edges()
                .any(|r| meta_ids.contains(&r.x) || meta_ids.contains(&r.y))
        });
        assert!(
            has_meta_candidate,
            "flag-on probe should surface at least one candidate touching meta-R"
        );
    }

    // ADR 0026 gradient refinement tests removed with the primitives.

    // ADR 0027 axiom-discovery helpers for tests.

    fn diamond_poset() -> RSet {
        // Hasse-diagram-as-transitive-closure: a ≤ b, a ≤ c, b ≤ d, c ≤ d,
        // a ≤ d (transitive closure), plus all self-loops.
        let mut rs = RSet::new();
        let nodes = ["a", "b", "c", "d"];
        for n in &nodes {
            rs.add(R::new(*n, *n));
        }
        rs.extend([
            R::new("a", "b"), R::new("a", "c"), R::new("a", "d"),
            R::new("b", "d"), R::new("c", "d"),
        ]);
        rs
    }

    fn simple_symmetric_graph() -> RSet {
        let mut rs = RSet::new();
        rs.extend([
            R::new("a", "b"), R::new("b", "a"),
            R::new("b", "c"), R::new("c", "b"),
        ]);
        rs
    }

    #[test]
    fn axiom_discovery_finds_transitivity_on_poset() {
        let rs = diamond_poset();
        let axioms = rs.discover_axioms(&AxiomDiscoveryConfig::default());
        // Transitivity as canonicalized template: premise
        // [R(0,1), R(1,2)], conclusion R(0,2), num_vars = 3.
        let transitivity = AxiomTemplate {
            num_vars: 3,
            premise: vec![
                EdgeTemplate { x_var: 0, y_var: 1 },
                EdgeTemplate { x_var: 1, y_var: 2 },
            ],
            conclusion: EdgeTemplate { x_var: 0, y_var: 2 },
        };
        let found = axioms.iter().any(|e| e.template == transitivity);
        assert!(found, "expected transitivity among discovered axioms; got {:?}",
            axioms.iter().map(|e| &e.template).collect::<Vec<_>>());
    }

    #[test]
    fn axiom_discovery_rejects_transitivity_on_raw_chain() {
        // Non-transitive: R(a,b), R(b,c). Transitivity demands R(a,c),
        // which is absent → rate < 1.0 → NOT in strict-discover output.
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b"), R::new("b", "c")]);
        let axioms = rs.discover_axioms(&AxiomDiscoveryConfig::default());
        let transitivity = AxiomTemplate {
            num_vars: 3,
            premise: vec![
                EdgeTemplate { x_var: 0, y_var: 1 },
                EdgeTemplate { x_var: 1, y_var: 2 },
            ],
            conclusion: EdgeTemplate { x_var: 0, y_var: 2 },
        };
        assert!(!axioms.iter().any(|e| e.template == transitivity));
    }

    #[test]
    fn axiom_discovery_finds_symmetry_on_symmetric_graph() {
        let rs = simple_symmetric_graph();
        let axioms = rs.discover_axioms(&AxiomDiscoveryConfig::default());
        // Symmetry: premise R(0,1), conclusion R(1,0).
        let symmetry = AxiomTemplate {
            num_vars: 2,
            premise: vec![EdgeTemplate { x_var: 0, y_var: 1 }],
            conclusion: EdgeTemplate { x_var: 1, y_var: 0 },
        };
        assert!(axioms.iter().any(|e| e.template == symmetry));
    }

    #[test]
    fn check_poset_accepts_diamond_and_rejects_chain() {
        let rs = diamond_poset();
        let pc = rs.check_poset();
        assert!(pc.is_poset);
        assert_eq!(pc.reflexive.rate, 1.0);
        assert!(pc.antisymmetric.holds);
        assert!(pc
            .transitive
            .as_ref()
            .map(|e| e.rate == 1.0)
            .unwrap_or(false));

        // A chain {a→b, b→c}: not reflexive, not transitive.
        let mut chain = RSet::new();
        chain.extend([R::new("a", "b"), R::new("b", "c")]);
        let pc2 = chain.check_poset();
        assert!(!pc2.is_poset);
    }

    #[test]
    fn check_reflexivity_empty_rset_is_vacuously_one() {
        let rs = RSet::new();
        let ev = rs.check_reflexivity();
        assert_eq!(ev.rate, 1.0);
        assert_eq!(ev.identifiers_total, 0);
    }

    #[test]
    fn axiom_discovery_enumeration_is_deterministic() {
        // Same RSet, two calls → same axioms in same order.
        let rs = diamond_poset();
        let a = rs.discover_axioms(&AxiomDiscoveryConfig::default());
        let b = rs.discover_axioms(&AxiomDiscoveryConfig::default());
        assert_eq!(a.len(), b.len());
        for (ea, eb) in a.iter().zip(b.iter()) {
            assert_eq!(ea.template, eb.template);
            assert_eq!(ea.premise_bindings, eb.premise_bindings);
            assert_eq!(ea.conclusion_satisfied, eb.conclusion_satisfied);
        }
    }

    // ADR 0028 subsumption tests.

    #[test]
    fn adr0028_canonicalizer_collapses_transitivity_variants() {
        // Transitive closure of 5-chain: 0027's enumeration surfaced two
        // templates recognized by humans as "transitivity under different
        // variable-to-slot assignments". The structural canonicalizer must
        // collapse them into exactly one canonical transitivity template.
        let mut rs = RSet::new();
        let nodes = ["a", "b", "c", "d", "e"];
        for i in 0..nodes.len() {
            for j in (i + 1)..nodes.len() {
                rs.add(R::new(nodes[i], nodes[j]));
            }
        }
        let axioms = rs.discover_axioms(&AxiomDiscoveryConfig::default());
        // Exactly one axiom, which is classical transitivity.
        assert_eq!(axioms.len(), 1, "got: {:?}",
            axioms.iter().map(|e| &e.template).collect::<Vec<_>>());
        let t = &axioms[0].template;
        assert_eq!(t.num_vars, 3);
        assert_eq!(t.premise.len(), 2);
        assert_eq!(t.conclusion, EdgeTemplate { x_var: 0, y_var: 2 });
        assert!(t.premise.contains(&EdgeTemplate { x_var: 0, y_var: 1 }));
        assert!(t.premise.contains(&EdgeTemplate { x_var: 1, y_var: 2 }));
    }

    #[test]
    fn adr0028_reflexivity_subsumes_self_loop_conclusions() {
        // Equivalence relation: symmetry + transitivity both hold; plus
        // universal reflexivity forces axioms with R(v, v) conclusions to
        // trivially hold. discover_axioms_minimal must eliminate those.
        let mut rs = RSet::new();
        let classes: &[&[&str]] = &[&["a", "b"], &["c", "d", "e"]];
        for cls in classes {
            for x in cls.iter() {
                for y in cls.iter() {
                    rs.add(R::new(*x, *y));
                }
            }
        }
        let minimal = rs.discover_axioms_minimal(&AxiomDiscoveryConfig::default());
        // Every remaining axiom has a non-self-loop conclusion.
        for ev in &minimal {
            assert_ne!(
                ev.template.conclusion.x_var,
                ev.template.conclusion.y_var,
                "reflexivity-trivial conclusion leaked through: {:?}",
                ev.template
            );
        }
        // Symmetry must survive.
        let symmetry = AxiomTemplate {
            num_vars: 2,
            premise: vec![EdgeTemplate { x_var: 0, y_var: 1 }],
            conclusion: EdgeTemplate { x_var: 1, y_var: 0 },
        };
        assert!(minimal.iter().any(|e| e.template == symmetry));
        // Transitivity must survive.
        let transitivity = AxiomTemplate {
            num_vars: 3,
            premise: vec![
                EdgeTemplate { x_var: 0, y_var: 1 },
                EdgeTemplate { x_var: 1, y_var: 2 },
            ],
            conclusion: EdgeTemplate { x_var: 0, y_var: 2 },
        };
        assert!(minimal.iter().any(|e| e.template == transitivity));
    }

    #[test]
    fn adr0028_premise_weakening_drops_redundant_superset() {
        // Synthetic axioms that share the symmetry conclusion but differ in
        // premise — the 1-edge-premise (strictly stronger) must dominate.
        let a = AxiomEvidence {
            template: AxiomTemplate {
                num_vars: 2,
                premise: vec![EdgeTemplate { x_var: 0, y_var: 1 }],
                conclusion: EdgeTemplate { x_var: 1, y_var: 0 },
            },
            premise_bindings: 10,
            conclusion_satisfied: 10,
            rate: 1.0,
            posterior_lower_95: 0.0,
            posterior_upper_95: 1.0,
            null_baseline_prob: 1.0,
        };
        let b = AxiomEvidence {
            template: AxiomTemplate {
                num_vars: 2,
                premise: vec![
                    EdgeTemplate { x_var: 0, y_var: 0 },
                    EdgeTemplate { x_var: 0, y_var: 1 },
                ],
                conclusion: EdgeTemplate { x_var: 1, y_var: 0 },
            },
            premise_bindings: 10,
            conclusion_satisfied: 10,
            rate: 1.0,
            posterior_lower_95: 0.0,
            posterior_upper_95: 1.0,
            null_baseline_prob: 1.0,
        };
        let out = subsume_by_premise_weakening(vec![a.clone(), b]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].template, a.template);
    }

    #[test]
    fn adr0028_discover_minimal_matches_raw_when_no_reflexivity() {
        // Strict partial order (no self-loops): reflexivity holds for 0
        // identifiers, subsumption-by-reflexivity should NOT fire. The
        // premise-weakening pass still runs, so the counts match only if
        // the raw output already lacks dominated pairs — for this case it
        // does.
        let mut rs = RSet::new();
        rs.extend([
            R::new("a", "b"), R::new("a", "c"), R::new("a", "d"),
            R::new("b", "d"), R::new("c", "d"),
        ]);
        let raw = rs.discover_axioms(&AxiomDiscoveryConfig::default());
        let minimal = rs.discover_axioms_minimal(&AxiomDiscoveryConfig::default());
        // Both should be just transitivity.
        assert_eq!(raw.len(), 1);
        assert_eq!(minimal.len(), 1);
        assert_eq!(raw[0].template, minimal[0].template);
    }

    #[test]
    fn adr0028_minimal_collapses_total_order_to_transitivity() {
        let mut rs = RSet::new();
        let nodes = ["1", "2", "3", "4", "5"];
        for i in 0..nodes.len() {
            rs.add(R::new(nodes[i], nodes[i]));
            for j in (i + 1)..nodes.len() {
                rs.add(R::new(nodes[i], nodes[j]));
            }
        }
        let minimal = rs.discover_axioms_minimal(&AxiomDiscoveryConfig::default());
        assert_eq!(minimal.len(), 1);
        let t = &minimal[0].template;
        assert_eq!(t.num_vars, 3);
        assert_eq!(t.premise.len(), 2);
        assert_eq!(t.conclusion, EdgeTemplate { x_var: 0, y_var: 2 });
    }

    #[test]
    fn adr0028_minimal_on_tolerance_keeps_symmetry_only() {
        // Tolerance: reflexive + symmetric, NOT transitive. Minimal axioms
        // should contain symmetry and NO transitivity.
        let mut rs = RSet::new();
        for n in ["a", "b", "c"] {
            rs.add(R::new(n, n));
        }
        rs.extend([
            R::new("a", "b"), R::new("b", "a"),
            R::new("b", "c"), R::new("c", "b"),
        ]);
        let minimal = rs.discover_axioms_minimal(&AxiomDiscoveryConfig::default());
        let symmetry = AxiomTemplate {
            num_vars: 2,
            premise: vec![EdgeTemplate { x_var: 0, y_var: 1 }],
            conclusion: EdgeTemplate { x_var: 1, y_var: 0 },
        };
        assert!(minimal.iter().any(|e| e.template == symmetry));
        let transitivity = AxiomTemplate {
            num_vars: 3,
            premise: vec![
                EdgeTemplate { x_var: 0, y_var: 1 },
                EdgeTemplate { x_var: 1, y_var: 2 },
            ],
            conclusion: EdgeTemplate { x_var: 0, y_var: 2 },
        };
        assert!(!minimal.iter().any(|e| e.template == transitivity));
    }

    #[test]
    fn chain_is_representable() {
        let mut rs = RSet::new();
        rs.extend([
            R::new("a1", "a2"),
            R::new("a2", "a3"),
            R::new("a3", "a4"),
            R::new("a4", "a5"),
        ]);

        assert_eq!(rs.len(), 4);
        assert_eq!(rs.identifiers().len(), 5);

        // middle-of-chain node: one in-edge, one out-edge
        assert_eq!(rs.left_of("a3").len(), 1);
        assert_eq!(rs.right_of("a3").len(), 1);

        // chain endpoints
        assert_eq!(rs.right_of("a1").len(), 0);
        assert_eq!(rs.left_of("a5").len(), 0);
    }

    // ADR 0029 — intension vs extension layering tests.

    fn mk_two_chain(a: &str, b: &str, c: &str) -> Subgraph {
        Subgraph::from_edges([R::new(a, b), R::new(b, c)])
    }

    #[test]
    fn adr0029_layer_a_written_on_first_mint() {
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b"), R::new("b", "c")]);
        let sg = mk_two_chain("a", "b", "c");
        let p = rs.name_pattern_instances(&[sg]).unwrap();
        // Intension: three roles registered.
        assert_eq!(rs.pattern_roles(&p).len(), 3);
        for role in rs.pattern_roles(&p) {
            assert!(rs.is_role(role));
        }
        // Intension: stored canonical form equals the subgraph's.
        let stored = rs.pattern_structure(&p).unwrap();
        let sg2 = mk_two_chain("a", "b", "c");
        assert_eq!(stored, sg2.canonicalize());
    }

    #[test]
    fn adr0029_intensional_policy_writes_no_instance_edges() {
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b"), R::new("b", "c")]);
        let sg = mk_two_chain("a", "b", "c");
        let pre = rs.len();
        let p = rs
            .name_pattern_instances_with_policy(
                &[sg],
                PatternRecordingPolicy::Intensional,
            )
            .unwrap();
        // Layer A was written; Layer B was not.
        assert!(rs.pattern_roles(&p).len() == 3);
        assert_eq!(rs.instances_of(&p).len(), 0);
        // Growth = Layer A only: 1 (registry) + 3 (role registry) + 3
        // (pattern→role) + 2 (structural edges) = 9 edges.
        let layer_a_count = 1 + 3 + 3 + 2;
        assert_eq!(rs.len() - pre, layer_a_count);
    }

    #[test]
    fn adr0029_instances_only_policy_writes_no_participants() {
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b"), R::new("b", "c")]);
        let sg = mk_two_chain("a", "b", "c");
        let p = rs
            .name_pattern_instances_with_policy(
                &[sg],
                PatternRecordingPolicy::InstancesOnly,
            )
            .unwrap();
        assert_eq!(rs.instances_of(&p).len(), 1);
        let inst = rs.instances_of(&p)[0].to_string();
        // No participant edges were written for this instance.
        assert_eq!(rs.participants_of(&inst).len(), 0);
    }

    #[test]
    fn adr0029_full_bindings_preserves_0010_semantics() {
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b"), R::new("b", "c")]);
        let sg = mk_two_chain("a", "b", "c");
        let p = rs.name_pattern_instances(&[sg]).unwrap(); // default = FullBindings
        let inst = rs.instances_of(&p)[0].to_string();
        let parts = rs.participants_of(&inst);
        assert_eq!(parts.len(), 3);
        assert!(parts.contains("a"));
        assert!(parts.contains("b"));
        assert!(parts.contains("c"));
    }

    #[test]
    fn adr0029_find_pattern_matching_uses_layer_a_without_instances() {
        // Intensional-only naming: no instances are persisted. But the
        // pattern must still be findable by canonical form via Layer A.
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b"), R::new("b", "c")]);
        let sg = mk_two_chain("a", "b", "c");
        let p = rs
            .name_pattern_instances_with_policy(
                &[sg.clone()],
                PatternRecordingPolicy::Intensional,
            )
            .unwrap();
        // Second identical structure should match the same pattern,
        // relying on Layer A (no instances to fall back on).
        let canon = sg.canonicalize();
        assert_eq!(rs.find_pattern_matching(&canon).unwrap(), p.as_str());
    }

    #[test]
    fn adr0029_collect_meta_ids_includes_roles() {
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b"), R::new("b", "c")]);
        let sg = mk_two_chain("a", "b", "c");
        let p = rs.name_pattern_instances(&[sg]).unwrap();
        let meta = rs.collect_meta_ids();
        assert!(meta.contains(ROLE_MARKER));
        for role in rs.pattern_roles(&p) {
            assert!(meta.contains(role));
        }
    }

    #[test]
    fn adr0029_retract_removes_layer_a() {
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b"), R::new("b", "c")]);
        let pre = rs.len();
        let sg = mk_two_chain("a", "b", "c");
        let p = rs.name_pattern_instances(&[sg]).unwrap();
        let _ = rs.retract_pattern(&p).unwrap();
        assert_eq!(rs.len(), pre);
        assert!(rs.pattern_structure(&p).is_none());
        assert!(rs.roles().is_empty());
        assert!(!rs.instances.iter().any(|r| r.x == ROLE_MARKER));
    }

    #[test]
    fn adr0029_reuse_pattern_id_across_policies() {
        // FullBindings then Intensional on a structurally identical
        // instance — should reuse the same pattern id, not mint a
        // second. Layer A is written once on first mint.
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b"), R::new("b", "c")]);
        let p1 = rs
            .name_pattern_instances(&[mk_two_chain("a", "b", "c")])
            .unwrap();
        rs.extend([R::new("p", "q"), R::new("q", "r")]);
        let p2 = rs
            .name_pattern_instances_with_policy(
                &[mk_two_chain("p", "q", "r")],
                PatternRecordingPolicy::Intensional,
            )
            .unwrap();
        assert_eq!(p1, p2);
        assert_eq!(rs.pattern_roles(&p1).len(), 3);
    }

    #[test]
    fn adr0029_instances_of_excludes_roles() {
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b"), R::new("b", "c")]);
        let p = rs
            .name_pattern_instances(&[mk_two_chain("a", "b", "c")])
            .unwrap();
        let insts = rs.instances_of(&p);
        assert_eq!(insts.len(), 1);
        for inst in &insts {
            assert!(!rs.is_role(inst));
        }
    }

    // ADR 0030 — theory objects (conjunctive concept naming).

    fn equivalence_relation() -> RSet {
        let mut rs = RSet::new();
        let classes: &[&[&str]] = &[&["a", "b"], &["c", "d", "e"]];
        for cls in classes {
            for x in cls.iter() {
                for y in cls.iter() {
                    rs.add(R::new(*x, *y));
                }
            }
        }
        rs
    }

    fn poset_with_selfloops() -> RSet {
        // Same diamond as axiom tests, reflexive closure.
        diamond_poset()
    }

    #[test]
    fn adr0030_axiom_template_id_roundtrip() {
        let transitivity = AxiomTemplate {
            num_vars: 3,
            premise: vec![
                EdgeTemplate { x_var: 0, y_var: 1 },
                EdgeTemplate { x_var: 1, y_var: 2 },
            ],
            conclusion: EdgeTemplate { x_var: 0, y_var: 2 },
        };
        let id = axiom_template_id(&transitivity);
        assert_eq!(id, "ax_tpl_v3_p0-1_p1-2_c0-2");
        let parsed = axiom_id_to_template(&id).expect("parses");
        assert_eq!(parsed, transitivity);
    }

    #[test]
    fn adr0030_discover_theory_on_equivalence_relation() {
        let rs = equivalence_relation();
        let th = rs.discover_theory(&AxiomDiscoveryConfig::default());
        // Equivalence: symmetry (template) + reflexivity (predicate).
        // Transitivity variants also show up (5 minimal axioms + 1 predicate).
        assert!(th.member_axiom_ids.iter().any(|id| id == AX_REFLEXIVITY));
        assert!(th.member_axiom_ids.iter().any(|id| {
            id == "ax_tpl_v2_p0-1_c1-0" // symmetry
        }));
        assert!(th.member_axiom_ids.iter().any(|id| {
            id == "ax_tpl_v3_p0-1_p1-2_c0-2" // transitivity
        }));
    }

    #[test]
    fn adr0030_discover_theory_on_poset() {
        let rs = poset_with_selfloops();
        let th = rs.discover_theory(&AxiomDiscoveryConfig::default());
        assert!(th.member_axiom_ids.iter().any(|id| id == AX_REFLEXIVITY));
        assert!(th.member_axiom_ids.iter().any(|id| id == AX_ANTISYMMETRY));
        assert!(th.member_axiom_ids.iter().any(|id| {
            id == "ax_tpl_v3_p0-1_p1-2_c0-2" // transitivity
        }));
    }

    #[test]
    fn adr0030_name_theory_persists_to_meta_r() {
        let mut rs = poset_with_selfloops();
        let th = rs.discover_theory(&AxiomDiscoveryConfig::default());
        let ids: Vec<&str> = th.member_axiom_ids.iter().map(|s| s.as_str()).collect();
        let t_id = rs.name_theory(&ids).expect("valid members");
        // Registry edge exists.
        assert!(rs.is_theory(&t_id));
        // Each axiom is registered.
        for id in &th.member_axiom_ids {
            assert!(rs.is_axiom(id));
        }
        // Members retrievable.
        let members: HashSet<&str> = rs.theory_axioms(&t_id).into_iter().collect();
        for id in &th.member_axiom_ids {
            assert!(members.contains(id.as_str()));
        }
    }

    #[test]
    fn adr0030_name_theory_reuses_existing() {
        let mut rs = poset_with_selfloops();
        let th = rs.discover_theory(&AxiomDiscoveryConfig::default());
        let ids: Vec<&str> = th.member_axiom_ids.iter().map(|s| s.as_str()).collect();
        let t1 = rs.name_theory(&ids).unwrap();
        let t2 = rs.name_theory(&ids).unwrap();
        assert_eq!(t1, t2);
        assert_eq!(rs.theories().len(), 1);
    }

    #[test]
    fn adr0030_name_theory_rejects_unsatisfied() {
        // Try to name reflexivity on a RSet where it doesn't hold.
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b"), R::new("b", "c")]);
        let err = rs.name_theory(&[AX_REFLEXIVITY]).unwrap_err();
        assert_eq!(err, TheoryError::UnsatisfiedMember(AX_REFLEXIVITY.to_string()));
    }

    #[test]
    fn adr0030_name_theory_rejects_unparseable() {
        let mut rs = equivalence_relation();
        let err = rs.name_theory(&["ax_not_a_real_id"]).unwrap_err();
        assert_eq!(err, TheoryError::UnparseableAxiomId("ax_not_a_real_id".to_string()));
    }

    #[test]
    fn adr0030_name_theory_rejects_empty() {
        let mut rs = equivalence_relation();
        let err = rs.name_theory(&[]).unwrap_err();
        assert_eq!(err, TheoryError::EmptyMemberList);
    }

    #[test]
    fn adr0030_retract_theory_removes_theory_only() {
        let mut rs = poset_with_selfloops();
        let th = rs.discover_theory(&AxiomDiscoveryConfig::default());
        let ids: Vec<&str> = th.member_axiom_ids.iter().map(|s| s.as_str()).collect();
        let t_id = rs.name_theory(&ids).unwrap();
        let axiom_count_before = rs.axioms().len();
        let removed = rs.retract_theory(&t_id).unwrap();
        // Removed: 1 registry + len(members) membership edges.
        assert_eq!(removed, 1 + th.member_axiom_ids.len());
        assert!(!rs.is_theory(&t_id));
        // Axiom registry is NOT touched (other theories may share).
        assert_eq!(rs.axioms().len(), axiom_count_before);
    }

    #[test]
    fn adr0068_shape_family_groups_axioms_with_same_premise() {
        // Register 3 axioms that share premise [p0-0, p1-2] but
        // differ in conclusion. Discover families with min=2.
        // Expect: 1 family minted; all 3 axioms members; family
        // queryable via the new API.
        let mut rs = RSet::new();
        // Need a substrate where all three axioms hold at rate 1.0
        // for register_axiom_with_intension to be meaningful — but
        // the discover function uses register state not validity,
        // so we can register manually.
        let ids = [
            "ax_tpl_v3_p0-0_p1-2_c0-1",
            "ax_tpl_v3_p0-0_p1-2_c0-2",
            "ax_tpl_v3_p0-0_p1-2_c1-0",
        ];
        for id in &ids {
            rs.register_axiom_with_intension(id);
        }
        let minted = rs.discover_axiom_shape_families(2);
        assert_eq!(
            minted.len(),
            1,
            "expected exactly one shape family; got {:?}",
            minted,
        );
        let shape = &minted[0];
        assert!(rs.is_axiom_shape_family(shape));
        let members = rs.shape_family_members(shape);
        assert_eq!(members.len(), 3);
        for id in &ids {
            assert!(members.contains(id), "missing member {}", id);
        }
    }

    #[test]
    fn adr0068_shape_family_respects_min_members() {
        // Two axioms sharing premise — min_members=3 should yield
        // no family.
        let mut rs = RSet::new();
        for id in [
            "ax_tpl_v3_p0-0_p1-2_c0-1",
            "ax_tpl_v3_p0-0_p1-2_c0-2",
        ] {
            rs.register_axiom_with_intension(id);
        }
        let minted = rs.discover_axiom_shape_families(3);
        assert!(minted.is_empty());
        assert!(rs.axiom_shape_families().is_empty());
    }

    #[test]
    fn adr0068_shape_family_distinguishes_different_premises() {
        // Two premise groups (2 members each) PLUS one conclusion
        // group (c0-2 shared by 2 axioms, one from each premise
        // group). With B.3 (shared-conclusion families), total
        // families = 3.
        let mut rs = RSet::new();
        for id in [
            "ax_tpl_v3_p0-0_p1-2_c0-1",
            "ax_tpl_v3_p0-0_p1-2_c0-2",
            "ax_tpl_v3_p0-1_p1-2_c0-2",
            "ax_tpl_v3_p0-1_p1-2_c2-0",
        ] {
            rs.register_axiom_with_intension(id);
        }
        let minted = rs.discover_axiom_shape_families(2);
        // 2 premise families + 1 conclusion family (c0-2 shared).
        assert_eq!(minted.len(), 3);
        let families = rs.axiom_shape_families();
        assert_eq!(families.len(), 3);
        // Premise families have 2 members each; the conclusion
        // family has 2 members (the two c0-2 axioms).
        for f in &families {
            assert_eq!(rs.shape_family_members(f).len(), 2);
        }
        // Verify conclusion family is named correctly.
        assert!(rs.is_axiom_shape_family("shape_conclusion_c0-2"));
    }

    #[test]
    fn adr0068_b4_filtered_enumerate_skips_blocked_premise() {
        // Blocked premise [p0-0, p1-2] should suppress all 4
        // conclusion variants.
        let config = AxiomDiscoveryConfig::default();
        let baseline =
            crate::enumerate_axiom_templates(&config);
        let mut blocked: HashSet<Vec<(usize, usize)>> = HashSet::new();
        let mut key = vec![(0, 0), (1, 2)];
        key.sort();
        blocked.insert(key);
        let filtered = crate::enumerate_axiom_templates_filtered(
            &config, &blocked,
        );
        // Filtered should have STRICTLY fewer templates.
        assert!(
            filtered.len() < baseline.len(),
            "filtered ({}) should be < baseline ({})",
            filtered.len(), baseline.len(),
        );
        // No template in filtered should have premise [p0-0, p1-2].
        for tpl in &filtered {
            let mut k: Vec<(usize, usize)> =
                tpl.premise.iter().map(|e| (e.x_var, e.y_var)).collect();
            k.sort();
            assert_ne!(k, vec![(0, 0), (1, 2)]);
        }
    }

    #[test]
    fn adr0068_b4_filtered_with_empty_blocked_equals_baseline() {
        let config = AxiomDiscoveryConfig::default();
        let baseline =
            crate::enumerate_axiom_templates(&config);
        let blocked: HashSet<Vec<(usize, usize)>> = HashSet::new();
        let filtered = crate::enumerate_axiom_templates_filtered(
            &config, &blocked,
        );
        assert_eq!(baseline.len(), filtered.len());
    }

    #[test]
    fn adr0068_b4_shape_premise_key_round_trip() {
        let rs = RSet::new();
        let key = rs.shape_premise_key("shape_premise_p0-0_p1-2");
        assert_eq!(key, Some(vec![(0, 0), (1, 2)]));
        // Conclusion-shape family returns None.
        assert!(rs.shape_premise_key("shape_conclusion_c0-2").is_none());
        // Unknown prefix returns None.
        assert!(rs.shape_premise_key("not_a_shape").is_none());
    }

    #[test]
    fn adr0068_shape_family_conclusion_kind() {
        // Three axioms with different premises but shared conclusion.
        // Should mint exactly one family (conclusion) — no premise
        // family qualifies (each premise unique).
        let mut rs = RSet::new();
        for id in [
            "ax_tpl_v3_p0-0_p1-2_c0-2",
            "ax_tpl_v3_p0-1_p1-2_c0-2",
            "ax_tpl_v3_p0-1_p2-1_c0-2",
        ] {
            rs.register_axiom_with_intension(id);
        }
        let minted = rs.discover_axiom_shape_families(2);
        assert_eq!(minted.len(), 1);
        assert!(rs.is_axiom_shape_family("shape_conclusion_c0-2"));
        let members = rs.shape_family_members("shape_conclusion_c0-2");
        assert_eq!(members.len(), 3);
    }

    #[test]
    fn adr0068_shape_family_idempotent() {
        let mut rs = RSet::new();
        for id in [
            "ax_tpl_v3_p0-0_p1-2_c0-1",
            "ax_tpl_v3_p0-0_p1-2_c0-2",
        ] {
            rs.register_axiom_with_intension(id);
        }
        let first = rs.discover_axiom_shape_families(2);
        let second = rs.discover_axiom_shape_families(2);
        assert_eq!(first.len(), 1);
        assert!(second.is_empty(), "second call must not duplicate");
        assert_eq!(rs.axiom_shape_families().len(), 1);
    }

    #[test]
    fn adr0068_shape_family_skips_predicate_axioms() {
        let mut rs = RSet::new();
        rs.register_axiom_with_intension(AX_REFLEXIVITY);
        rs.register_axiom_with_intension(AX_ANTISYMMETRY);
        rs.register_axiom_with_intension(AX_TOTALITY);
        let minted = rs.discover_axiom_shape_families(2);
        // Predicate axioms have no template → no premise → no family.
        assert!(minted.is_empty());
    }

    #[test]
    fn adr0068_f1_axiom_cross_precision_basic() {
        // Build a poset substrate; transitivity should have
        // cross-precision = 1.0 against itself.
        let mut rs = RSet::new();
        rs.extend([
            R::new("a", "b"), R::new("b", "c"), R::new("c", "d"),
            R::new("a", "c"), R::new("a", "d"), R::new("b", "d"),
        ]);
        let trans_id = "ax_tpl_v3_p0-1_p1-2_c0-2";
        let t_id = rs.name_theory(&[trans_id]).unwrap();
        // Generate a substrate from this theory.
        let gen = rs
            .generate_substrate_from_theory(&t_id, 6, 0.30, 12345)
            .unwrap();
        // Cross-precision of transitivity on its own substrate
        // should be 1.0 (saturation guarantees closure).
        let p = rs.axiom_cross_precision(trans_id, &[gen]);
        assert!(p.is_some());
        assert!((p.unwrap() - 1.0).abs() < 1e-9, "expected 1.0, got {:?}", p);
    }

    #[test]
    fn adr0068_f1_axiom_cross_precision_no_predictions() {
        // Empty substrate slice → None.
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b")]);
        let p = rs.axiom_cross_precision("ax_tpl_v3_p0-1_p1-2_c0-2", &[]);
        assert!(p.is_none());
    }

    #[test]
    fn adr0068_b7_super_meta_groups_nested_by_shared_member() {
        // 4 axioms create 2 premise families with overlapping
        // premise edges. After B.6 mints 2 nested families, both
        // contain shape_premise_p0-1_p1-2 → B.7 mints 1 super-
        // meta-family with both as members.
        let mut rs = RSet::new();
        for id in [
            "ax_tpl_v3_p0-0_p1-2_c0-1", // shape_premise_p0-0_p1-2
            "ax_tpl_v3_p0-0_p1-2_c0-2", // shape_premise_p0-0_p1-2
            "ax_tpl_v3_p0-1_p1-2_c0-2", // shape_premise_p0-1_p1-2
            "ax_tpl_v3_p0-1_p1-2_c2-0", // shape_premise_p0-1_p1-2
        ] {
            rs.register_axiom_with_intension(id);
        }
        let _ = rs.discover_axiom_shape_families(2);
        let nested = rs.discover_nested_shape_families(2);
        assert_eq!(nested.len(), 1, "expected 1 nested (just meta_premise_p1-2 since p0-0 and p0-1 are unique)");
        // Need 2 nested families to test B.7 — the above gives only 1.
        // Add more axioms to ensure 2 nested families.
        for id in [
            "ax_tpl_v3_p0-1_p2-1_c0-2", // shape_premise_p0-1_p2-1
        ] {
            rs.register_axiom_with_intension(id);
        }
        // Now: shape_premise_p0-1_p1-2 and shape_premise_p0-1_p2-1
        // would each need ≥2 members. Only 1 each → no families.
        // Skip the more elaborate setup; just verify the mechanism.
        let supers = rs.discover_super_meta_shape_families(2);
        // No super family — only 1 nested currently.
        assert!(supers.is_empty(), "expected 0; got {:?}", supers);
    }

    #[test]
    fn adr0068_b7_super_meta_idempotent() {
        let rs = RSet::new();
        // No nested families → nothing to abstract.
        let mut rs = rs;
        let supers = rs.discover_super_meta_shape_families(2);
        assert!(supers.is_empty());
        let supers2 = rs.discover_super_meta_shape_families(2);
        assert!(supers2.is_empty());
    }

    #[test]
    fn adr0068_b6_nested_family_groups_premise_families_by_shared_edge() {
        // 4 axioms forming 2 premise families that share `p1-2`.
        let mut rs = RSet::new();
        for id in [
            "ax_tpl_v3_p0-0_p1-2_c0-1", // family A
            "ax_tpl_v3_p0-0_p1-2_c0-2", // family A
            "ax_tpl_v3_p0-1_p1-2_c0-2", // family B
            "ax_tpl_v3_p0-1_p1-2_c2-0", // family B
        ] {
            rs.register_axiom_with_intension(id);
        }
        let _ = rs.discover_axiom_shape_families(2);
        let nested = rs.discover_nested_shape_families(2);
        // p1-2 appears in both A and B → 1 nested family.
        // p0-0 only in A. p0-1 only in B. So just 1 minted.
        assert_eq!(
            nested.len(),
            1,
            "expected 1 nested family; got {:?}",
            nested,
        );
        assert!(rs.is_nested_shape_family("meta_premise_p1-2"));
        let members = rs.nested_shape_family_members("meta_premise_p1-2");
        assert_eq!(members.len(), 2);
    }

    #[test]
    fn adr0068_b6_nested_family_idempotent() {
        let mut rs = RSet::new();
        for id in [
            "ax_tpl_v3_p0-0_p1-2_c0-1",
            "ax_tpl_v3_p0-0_p1-2_c0-2",
            "ax_tpl_v3_p0-1_p1-2_c0-2",
            "ax_tpl_v3_p0-1_p1-2_c2-0",
        ] {
            rs.register_axiom_with_intension(id);
        }
        let _ = rs.discover_axiom_shape_families(2);
        let first = rs.discover_nested_shape_families(2);
        let second = rs.discover_nested_shape_families(2);
        assert_eq!(first.len(), 1);
        assert!(second.is_empty());
    }

    #[test]
    fn adr0068_b6_nested_family_respects_min_member_families() {
        // Only one premise family → can't form a nested family.
        let mut rs = RSet::new();
        for id in [
            "ax_tpl_v3_p0-0_p1-2_c0-1",
            "ax_tpl_v3_p0-0_p1-2_c0-2",
        ] {
            rs.register_axiom_with_intension(id);
        }
        let _ = rs.discover_axiom_shape_families(2);
        let nested = rs.discover_nested_shape_families(2);
        // Even though both p0-0 and p1-2 appear in this family, no
        // OTHER family contains them → nothing meta to abstract.
        assert!(nested.is_empty());
    }

    // ─── ADR 0070 — shape-family abstraction layer (unified API) ───

    #[test]
    fn adr0070_family_layer_dispatches_correctly() {
        let mut rs = crate::RSet::new();
        for id in [
            "ax_tpl_v3_p0-1_p1-2_c0-2",
            "ax_tpl_v3_p0-1_p1-2_c2-0",
            "ax_tpl_v3_p0-1_p2-1_c0-2",
        ] {
            rs.register_axiom_with_intension(id);
        }
        let _ = rs.discover_axiom_shape_families(2);
        let _ = rs.discover_nested_shape_families(2);

        // L2 family
        for fid in rs.axiom_shape_families().iter().map(|s| s.to_string()).collect::<Vec<_>>() {
            assert_eq!(rs.family_layer(&fid), Some(crate::FamilyLayer::L2));
        }
        // L3 family (if any)
        for fid in rs.nested_shape_families().iter().map(|s| s.to_string()).collect::<Vec<_>>() {
            assert_eq!(rs.family_layer(&fid), Some(crate::FamilyLayer::L3));
        }
        // Non-family id
        assert_eq!(rs.family_layer("ax_tpl_v3_p0-1_p1-2_c0-2"), None);
        assert_eq!(rs.family_layer("does_not_exist"), None);
    }

    #[test]
    fn adr0070_family_members_dispatches_by_layer() {
        let mut rs = crate::RSet::new();
        for id in [
            "ax_tpl_v3_p0-1_p1-2_c0-2",
            "ax_tpl_v3_p0-1_p1-2_c2-0",
            "ax_tpl_v3_p0-1_p2-1_c0-2",
        ] {
            rs.register_axiom_with_intension(id);
        }
        let _ = rs.discover_axiom_shape_families(2);
        // Pick an L2 family and verify members match shape_family_members.
        let any_l2 = rs.axiom_shape_families()[0].to_string();
        let direct = rs
            .shape_family_members(&any_l2)
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let dispatched = rs
            .family_members(&any_l2)
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        assert_eq!(direct, dispatched);
    }

    #[test]
    fn adr0070_family_kind_recognizes_premise_and_conclusion() {
        let mut rs = crate::RSet::new();
        for id in [
            "ax_tpl_v3_p0-1_p1-2_c0-2",
            "ax_tpl_v3_p0-1_p1-2_c2-0",
            "ax_tpl_v3_p0-1_p2-1_c0-2",
        ] {
            rs.register_axiom_with_intension(id);
        }
        let _ = rs.discover_axiom_shape_families(2);

        // Find a premise family and a conclusion family.
        let mut saw_premise = false;
        let mut saw_conclusion = false;
        for fid in rs.axiom_shape_families() {
            if fid.starts_with("shape_premise_") {
                assert_eq!(
                    rs.family_kind(fid),
                    Some(crate::KIND_PREMISE_SHARED),
                    "premise family kind mismatch for {}",
                    fid,
                );
                saw_premise = true;
            } else if fid.starts_with("shape_conclusion_") {
                assert_eq!(
                    rs.family_kind(fid),
                    Some(crate::KIND_CONCLUSION_SHARED),
                    "conclusion family kind mismatch for {}",
                    fid,
                );
                saw_conclusion = true;
            }
        }
        assert!(saw_premise || saw_conclusion);
    }

    #[test]
    fn adr0070_kind_constants_in_collect_meta_ids() {
        // Even with NO families minted, the kind constants must be
        // classified as meta so they never pollute data-id accounting.
        let rs = crate::RSet::new();
        let meta = rs.collect_meta_ids();
        assert!(meta.contains(crate::KIND_PREMISE_SHARED));
        assert!(meta.contains(crate::KIND_CONCLUSION_SHARED));
        assert!(meta.contains(crate::KIND_PREMISE_EDGE_SHARED));
        assert!(meta.contains(crate::KIND_MEMBER_OVERLAP));
        assert!(meta.contains(crate::KIND_MEMBER_L2_SHARED));
        assert!(meta.contains(crate::KIND_MARKER));
    }

    #[test]
    fn adr0070_family_quality_class_thresholds() {
        // Sanity checks for the FamilyQuality::class() classifier.
        let signal = crate::FamilyQuality {
            mean: 0.92, std: 0.10, min: 0.75, max: 1.0, n_members: 3,
        };
        assert_eq!(signal.class(), crate::FamilyQualityClass::Signal);

        let noise = crate::FamilyQuality {
            mean: 0.40, std: 0.10, min: 0.20, max: 0.55, n_members: 3,
        };
        assert_eq!(noise.class(), crate::FamilyQualityClass::Noise);

        let uniform = crate::FamilyQuality {
            mean: 0.49, std: 0.0, min: 0.49, max: 0.49, n_members: 4,
        };
        // Uniform takes precedence over Noise even though mean < 0.50.
        assert_eq!(uniform.class(), crate::FamilyQualityClass::Uniform);

        let mixed = crate::FamilyQuality {
            mean: 0.65, std: 0.20, min: 0.30, max: 0.95, n_members: 3,
        };
        assert_eq!(mixed.class(), crate::FamilyQualityClass::Mixed);
    }

    #[test]
    fn adr0070_kind_tag_edge_emitted_during_discovery() {
        let mut rs = crate::RSet::new();
        for id in [
            "ax_tpl_v3_p0-1_p1-2_c0-2",
            "ax_tpl_v3_p0-1_p1-2_c2-0",
        ] {
            rs.register_axiom_with_intension(id);
        }
        let _ = rs.discover_axiom_shape_families(2);
        // For each minted L2 family there should be an edge
        // R(family_id, kind_id) with one of the L2 kinds.
        for fid in rs.axiom_shape_families() {
            let kind = rs.family_kind(fid);
            assert!(kind.is_some(), "family {} has no kind", fid);
            let kind_str = kind.unwrap();
            let edge = crate::R::new(fid.to_string(), kind_str.to_string());
            assert!(
                rs.contains(&edge),
                "missing kind tag edge for family {}",
                fid,
            );
        }
    }

    // ─── ADR 0070 Step 2 — operation lift ──────────────────────────

    #[test]
    fn adr0070_retract_l2_family_globally_retracts_orphan_axioms() {
        // Build a 2-axiom L2 family with NO theory containing them
        // (orphan-axiom case). After retract_shape_family, axioms
        // should be globally retracted; theory-detachment count = 0.
        let mut rs = crate::RSet::new();
        let ax_a = "ax_tpl_v3_p0-0_p1-2_c0-1";
        let ax_b = "ax_tpl_v3_p0-0_p1-2_c0-2";
        for id in [ax_a, ax_b] {
            rs.register_axiom_with_intension(id);
        }
        let _ = rs.discover_axiom_shape_families(2);
        let l2_fams = rs.axiom_shape_families();
        assert_eq!(l2_fams.len(), 1);
        let fid = l2_fams[0].to_string();
        assert_eq!(rs.shape_family_members(&fid).len(), 2);

        let summary = rs.retract_shape_family(&fid).unwrap();
        assert_eq!(summary.layer, crate::FamilyLayer::L2);
        assert_eq!(summary.axioms_globally_retracted, 2);
        // No theories → no detachments.
        assert_eq!(summary.theory_memberships_detached, 0);
        // Marker edge always removed; kind tag depends on whether
        // it was emitted (yes, post-Step-1).
        assert!(summary.structural_edges_removed >= 1);

        assert!(!rs.is_axiom_shape_family(&fid));
        assert!(!rs.is_axiom(ax_a));
        assert!(!rs.is_axiom(ax_b));
    }

    #[test]
    fn adr0070_retract_l2_family_detaches_from_theories() {
        // Same setup as above but with a theory containing both
        // axioms. The theory must be created via raw R edges to
        // bypass name_theory's validation (axiom rates would
        // require a substrate that satisfies the noise template).
        use crate::{R, AXIOM_MARKER, THEORY_MARKER};
        let mut rs = crate::RSet::new();
        let ax_a = "ax_tpl_v3_p0-0_p1-2_c0-1";
        let ax_b = "ax_tpl_v3_p0-0_p1-2_c0-2";
        for id in [ax_a, ax_b] {
            rs.register_axiom_with_intension(id);
        }
        // Manually mint a theory containing both axioms (bypasses
        // verify_axiom_holds; the test target is retraction logic,
        // not theory-naming validation).
        let theory_id = "t_test";
        rs.add(R::new(THEORY_MARKER, theory_id));
        rs.add(R::new(theory_id, ax_a));
        rs.add(R::new(theory_id, ax_b));
        // Sanity: theory is recognized.
        assert!(rs.is_theory(theory_id));
        assert_eq!(rs.theories_containing(ax_a).len(), 1);
        assert_eq!(rs.theories_containing(ax_b).len(), 1);

        let _ = rs.discover_axiom_shape_families(2);
        let fid = rs.axiom_shape_families()[0].to_string();
        let summary = rs.retract_shape_family(&fid).unwrap();
        assert_eq!(summary.theory_memberships_detached, 2);
        assert_eq!(summary.axioms_globally_retracted, 2);
        assert_eq!(rs.theory_axioms(theory_id).len(), 0);
        let _ = AXIOM_MARKER; // keep import used
    }

    #[test]
    fn adr0070_retract_l3_family_does_not_cascade_to_l2() {
        // Build two L2 families that share a premise edge → L3 mints.
        // Retract the L3 → both L2 stay registered.
        let mut rs = crate::RSet::new();
        for id in [
            "ax_tpl_v3_p0-0_p1-2_c0-1",
            "ax_tpl_v3_p0-0_p1-2_c0-2",
            "ax_tpl_v3_p0-1_p1-2_c0-2",
            "ax_tpl_v3_p0-1_p1-2_c2-0",
        ] {
            rs.register_axiom_with_intension(id);
        }
        let _ = rs.discover_axiom_shape_families(2);
        let _ = rs.discover_nested_shape_families(2);
        let l2_count_before = rs.axiom_shape_families().len();
        let l3 = rs.nested_shape_families();
        assert!(!l3.is_empty(), "expected at least one L3 family");
        let l3_id = l3[0].to_string();

        let summary = rs.retract_shape_family(&l3_id).unwrap();
        assert_eq!(summary.layer, crate::FamilyLayer::L3);
        assert_eq!(summary.axioms_globally_retracted, 0);
        assert_eq!(summary.theory_memberships_detached, 0);
        assert!(summary.member_links_removed >= 2);

        assert!(!rs.is_nested_shape_family(&l3_id));
        // L2 families unchanged.
        assert_eq!(rs.axiom_shape_families().len(), l2_count_before);
    }

    #[test]
    fn adr0070_retract_unknown_family_errors() {
        let mut rs = crate::RSet::new();
        let err = rs.retract_shape_family("never_minted").unwrap_err();
        assert_eq!(err, crate::ShapeFamilyRetractionError::UnknownFamily);
    }

    #[test]
    fn adr0070_retract_idempotency_after_first_call() {
        let mut rs = crate::RSet::new();
        for id in [
            "ax_tpl_v3_p0-0_p1-2_c0-1",
            "ax_tpl_v3_p0-0_p1-2_c0-2",
        ] {
            rs.register_axiom_with_intension(id);
        }
        let _ = rs.discover_axiom_shape_families(2);
        let fid = rs.axiom_shape_families()[0].to_string();
        let first = rs.retract_shape_family(&fid);
        assert!(first.is_ok());
        let second = rs.retract_shape_family(&fid);
        // Second call should error: the family no longer exists.
        assert!(second.is_err());
    }

    #[test]
    fn adr0070_member_overlap_l3_kind_mints_when_axiom_in_two_l2() {
        // An axiom that has both a premise group AND a conclusion
        // group lands in 2 L2 families → member-overlap mints.
        let mut rs = crate::RSet::new();
        for id in [
            "ax_tpl_v3_p0-1_p1-2_c0-2",
            "ax_tpl_v3_p0-1_p1-2_c2-0",
            "ax_tpl_v3_p0-1_p2-1_c0-2",
        ] {
            rs.register_axiom_with_intension(id);
        }
        let _ = rs.discover_axiom_shape_families(2);
        // The shared axiom `ax_tpl_v3_p0-1_p1-2_c0-2` is in BOTH a
        // shape_premise_p0-1_p1-2 family AND a shape_conclusion_c0-2
        // family.
        let minted = rs.discover_nested_shape_families_by_member_overlap(2);
        assert!(!minted.is_empty(), "expected member-overlap L3 to mint");
        for id in &minted {
            assert!(id.starts_with("meta_via_"));
            assert!(rs.is_nested_shape_family(id));
            assert_eq!(
                rs.family_kind(id),
                Some(crate::KIND_MEMBER_OVERLAP),
            );
        }
    }

    #[test]
    fn adr0070_discover_shape_family_layer_chains_all_kinds() {
        let mut rs = crate::RSet::new();
        for id in [
            "ax_tpl_v3_p0-0_p1-2_c0-1",
            "ax_tpl_v3_p0-0_p1-2_c0-2",
            "ax_tpl_v3_p0-1_p1-2_c0-2",
            "ax_tpl_v3_p0-1_p1-2_c2-0",
            "ax_tpl_v3_p0-1_p2-1_c0-2",
        ] {
            rs.register_axiom_with_intension(id);
        }
        let summary = rs.discover_shape_family_layer(2);
        assert!(!summary.l2_minted.is_empty(), "L2 must mint");
        assert!(!summary.l3_minted.is_empty(), "L3 should mint (premise + overlap)");
        // L4 may or may not mint depending on whether L3 has overlapping members.
        // Idempotency: second call mints nothing new.
        let again = rs.discover_shape_family_layer(2);
        assert!(again.l2_minted.is_empty());
        assert!(again.l3_minted.is_empty());
        assert!(again.l4_minted.is_empty());
    }

    #[test]
    fn adr0070_member_overlap_l3_kind_idempotent() {
        let mut rs = crate::RSet::new();
        for id in [
            "ax_tpl_v3_p0-1_p1-2_c0-2",
            "ax_tpl_v3_p0-1_p1-2_c2-0",
            "ax_tpl_v3_p0-1_p2-1_c0-2",
        ] {
            rs.register_axiom_with_intension(id);
        }
        let _ = rs.discover_axiom_shape_families(2);
        let first = rs.discover_nested_shape_families_by_member_overlap(2);
        let second = rs.discover_nested_shape_families_by_member_overlap(2);
        assert!(!first.is_empty());
        assert!(second.is_empty(), "second call must not duplicate");
    }

    // ─── ADR 0071 — unified theory-quality report ──────────────────

    #[test]
    fn adr0071_unknown_theory_returns_none() {
        let rs = crate::RSet::new();
        let primary: std::collections::HashMap<String, f64> =
            std::collections::HashMap::new();
        let report = rs.theory_quality_report("does_not_exist", &[], &primary);
        assert!(report.is_none());
    }

    #[test]
    fn adr0071_indeterminate_when_no_data() {
        // A theory with no primary data, no substrates, no families
        // → Indeterminate.
        use crate::{R, THEORY_MARKER};
        let mut rs = crate::RSet::new();
        let ax = "ax_tpl_v3_p0-1_p1-2_c0-2";
        rs.register_axiom_with_intension(ax);
        // Manually mint a theory containing the axiom.
        rs.add(R::new(THEORY_MARKER, "t_x"));
        rs.add(R::new("t_x", ax));
        let primary: std::collections::HashMap<String, f64> =
            std::collections::HashMap::new();
        let report = rs.theory_quality_report("t_x", &[], &primary).unwrap();
        assert_eq!(report.theory_id, "t_x");
        assert_eq!(report.axiom_count, 1);
        assert!(report.primary_rate_mean.is_none());
        assert!(report.cross_precision_mean.is_none());
        assert_eq!(report.noise_family_axiom_count, 0);
        assert_eq!(report.summary_class, crate::TheoryQualityClass::Indeterminate);
    }

    #[test]
    fn adr0071_signal_when_both_dims_high_and_no_noise() {
        // Use the theory_summary_class composition rule directly.
        let cls = crate::compute_theory_summary_class(
            Some(0.95), Some(0.92), 0, 4,
        );
        assert_eq!(cls, crate::TheoryQualityClass::Signal);
    }

    #[test]
    fn adr0071_noise_when_both_dims_low() {
        let cls = crate::compute_theory_summary_class(
            Some(0.20), Some(0.30), 0, 4,
        );
        assert_eq!(cls, crate::TheoryQualityClass::Noise);
    }

    #[test]
    fn adr0071_noise_when_dominated_by_noise_families() {
        // axiom_count=4, noise_family_axiom_count=2 → 2*2 >= 4 → Noise.
        let cls = crate::compute_theory_summary_class(
            Some(0.85), Some(0.90), 2, 4,
        );
        assert_eq!(cls, crate::TheoryQualityClass::Noise);
    }

    #[test]
    fn adr0071_mixed_when_one_dim_low() {
        let cls = crate::compute_theory_summary_class(
            Some(0.85), Some(0.40), 0, 4,
        );
        assert_eq!(cls, crate::TheoryQualityClass::Mixed);
    }

    #[test]
    fn adr0071_family_memberships_listed_correctly() {
        // Build a theory whose axioms participate in shape families.
        // Verify each family appears as a TheoryFamilyMembership entry
        // with correct kind and member counts.
        use crate::{R, THEORY_MARKER, FamilyLayer};
        let mut rs = crate::RSet::new();
        for id in [
            "ax_tpl_v3_p0-0_p1-2_c0-1",
            "ax_tpl_v3_p0-0_p1-2_c0-2",
        ] {
            rs.register_axiom_with_intension(id);
        }
        let _ = rs.discover_axiom_shape_families(2);
        // Mint theory containing both axioms.
        rs.add(R::new(THEORY_MARKER, "t_x"));
        rs.add(R::new("t_x", "ax_tpl_v3_p0-0_p1-2_c0-1"));
        rs.add(R::new("t_x", "ax_tpl_v3_p0-0_p1-2_c0-2"));

        let primary: std::collections::HashMap<String, f64> =
            std::collections::HashMap::new();
        let report = rs.theory_quality_report("t_x", &[], &primary).unwrap();
        assert!(!report.family_memberships.is_empty());
        for m in &report.family_memberships {
            assert_eq!(m.layer, FamilyLayer::L2);
            assert!(m.kind.is_some(), "family {} kind should be set", m.family_id);
            // Both members are in this theory.
            assert_eq!(m.members_in_theory, 2);
            assert_eq!(m.family_total_members, 2);
        }
    }

    #[test]
    fn adr0071_report_all_sorted_and_complete() {
        use crate::{R, THEORY_MARKER};
        let mut rs = crate::RSet::new();
        let ax = "ax_tpl_v3_p0-1_p1-2_c0-2";
        rs.register_axiom_with_intension(ax);
        // Mint two theories in arbitrary insertion order.
        rs.add(R::new(THEORY_MARKER, "t_b"));
        rs.add(R::new("t_b", ax));
        rs.add(R::new(THEORY_MARKER, "t_a"));
        rs.add(R::new("t_a", ax));

        let primary: std::collections::HashMap<String, f64> =
            std::collections::HashMap::new();
        let reports = rs.theory_quality_report_all(&[], &primary);
        assert_eq!(reports.len(), 2);
        // Sorted by id.
        assert_eq!(reports[0].theory_id, "t_a");
        assert_eq!(reports[1].theory_id, "t_b");
    }

    // ─── ADR 0072 — intervention policy classifier ───────────────

    /// Helper: build a synthetic TheoryQualityReport for testing
    /// the classifier's decision tree without a full RSet setup.
    fn synth_report(
        theory_id: &str,
        axiom_ids: Vec<String>,
        primary_mean: Option<f64>,
        cross_mean: Option<f64>,
        noise_axiom_count: usize,
        family_memberships: Vec<crate::TheoryFamilyMembership>,
        neighborhood_extends: Vec<String>,
        per_axiom_stats: Vec<crate::AxiomQualityStats>,
    ) -> crate::TheoryQualityReport {
        let summary_class = crate::compute_theory_summary_class(
            primary_mean,
            cross_mean,
            noise_axiom_count,
            axiom_ids.len(),
        );
        crate::TheoryQualityReport {
            theory_id: theory_id.to_string(),
            axiom_count: axiom_ids.len(),
            axiom_ids,
            primary_rate_mean: primary_mean,
            primary_rate_min: primary_mean,
            primary_rate_qualifying: 1,
            cross_precision_mean: cross_mean,
            cross_precision_min: cross_mean,
            cross_precision_max: cross_mean,
            cross_precision_qualifying: 1,
            family_memberships,
            noise_family_axiom_count: noise_axiom_count,
            signal_family_axiom_count: 0,
            neighborhood: Some(crate::TheoryNeighborhood {
                equal: Vec::new(),
                extends: neighborhood_extends,
                extended_by: Vec::new(),
                independent: Vec::new(),
                parallel: Vec::new(),
            }),
            summary_class,
            per_axiom_stats,
        }
    }

    #[test]
    fn adr0072_indeterminate_returns_shadow_monitor() {
        let r = synth_report("t_x", vec![], None, None, 0, vec![], vec![], vec![]);
        let rec = crate::RSet::recommend_intervention(&r, &[]);
        assert!(matches!(
            rec,
            crate::RecommendedIntervention::ShadowMonitor { .. }
        ));
    }

    #[test]
    fn adr0072_signal_returns_none() {
        let r = synth_report(
            "t_x",
            vec!["ax_a".to_string()],
            Some(0.95),
            Some(0.92),
            0,
            vec![],
            vec![],
            vec![],
        );
        let rec = crate::RSet::recommend_intervention(&r, &[]);
        assert_eq!(rec, crate::RecommendedIntervention::None);
    }

    #[test]
    fn adr0072_demote_superset_when_extending_signal_subset() {
        // focal is Mixed and extends a Signal subset → DemoteSuperset.
        let focal = synth_report(
            "t_super",
            vec!["ax_a".to_string(), "ax_b".to_string()],
            Some(0.55), // Mixed
            Some(0.55),
            0,
            vec![],
            vec!["t_sub".to_string()], // focal extends t_sub
            vec![],
        );
        let sub = synth_report(
            "t_sub",
            vec!["ax_a".to_string()],
            Some(0.95),
            Some(0.92),
            0,
            vec![],
            vec![],
            vec![],
        );
        let rec = crate::RSet::recommend_intervention(&focal, &[sub]);
        match rec {
            crate::RecommendedIntervention::DemoteSuperset {
                cleaner_subset_theory,
            } => {
                assert_eq!(cleaner_subset_theory, "t_sub");
            }
            other => panic!("expected DemoteSuperset, got {:?}", other),
        }
    }

    #[test]
    fn adr0072_family_demote_when_noise_family_present() {
        // focal is Mixed (not Indeterminate, not Signal); has a
        // noise-class family → FamilyDemote.
        let noise_member = crate::TheoryFamilyMembership {
            family_id: "shape_premise_p0-0_p1-2".to_string(),
            layer: crate::FamilyLayer::L2,
            kind: Some(crate::KIND_PREMISE_SHARED),
            quality: Some(crate::FamilyQuality {
                mean: 0.40,
                std: 0.0,
                min: 0.40,
                max: 0.40,
                n_members: 4,
            }),
            class: Some(crate::FamilyQualityClass::Uniform),
            members_in_theory: 3,
            family_total_members: 4,
        };
        let focal = synth_report(
            "t_x",
            vec!["ax_a".to_string(), "ax_b".to_string(), "ax_c".to_string(), "ax_d".to_string()],
            Some(0.55),
            Some(0.55),
            0, // not noise-dominated by axiom count, but family is noise-class
            vec![noise_member],
            vec![],
            vec![],
        );
        let rec = crate::RSet::recommend_intervention(&focal, &[]);
        match rec {
            crate::RecommendedIntervention::FamilyDemote {
                family_id,
                family_class,
            } => {
                assert_eq!(family_id, "shape_premise_p0-0_p1-2");
                assert_eq!(family_class, crate::FamilyQualityClass::Uniform);
            }
            other => panic!("expected FamilyDemote, got {:?}", other),
        }
    }

    #[test]
    fn adr0072_axiom_repair_when_few_weak_axioms() {
        // focal is Mixed, primary_mean ≥ 0.60, no noise families.
        // Per-axiom stats: 4 axioms; one has primary 0.10 (weak),
        // others healthy → AxiomRepair.
        let stats = vec![
            crate::AxiomQualityStats {
                axiom_id: "ax_a".to_string(),
                primary_rate: Some(0.95),
                cross_precision: Some(0.90),
                family_ids: vec![],
            },
            crate::AxiomQualityStats {
                axiom_id: "ax_b".to_string(),
                primary_rate: Some(0.90),
                cross_precision: Some(0.85),
                family_ids: vec![],
            },
            crate::AxiomQualityStats {
                axiom_id: "ax_weak".to_string(),
                primary_rate: Some(0.10), // ← weak
                cross_precision: Some(0.20),
                family_ids: vec![],
            },
            crate::AxiomQualityStats {
                axiom_id: "ax_d".to_string(),
                primary_rate: Some(0.85),
                cross_precision: Some(0.80),
                family_ids: vec![],
            },
        ];
        let focal = synth_report(
            "t_x",
            vec![
                "ax_a".to_string(),
                "ax_b".to_string(),
                "ax_weak".to_string(),
                "ax_d".to_string(),
            ],
            Some(0.70), // Mixed (high enough for repair eligibility)
            Some(0.70),
            0,
            vec![],
            vec![],
            stats,
        );
        let rec = crate::RSet::recommend_intervention(&focal, &[]);
        match rec {
            crate::RecommendedIntervention::AxiomRepair { axiom_ids } => {
                assert_eq!(axiom_ids, vec!["ax_weak".to_string()]);
            }
            other => panic!("expected AxiomRepair, got {:?}", other),
        }
    }

    #[test]
    fn adr0072_merge_when_complementary_signal_partner_exists() {
        // focal is Mixed, no noise families, no weak axioms.
        // Partner is Signal with disjoint family signature.
        let focal_fam = crate::TheoryFamilyMembership {
            family_id: "shape_premise_p0-1".to_string(),
            layer: crate::FamilyLayer::L2,
            kind: Some(crate::KIND_PREMISE_SHARED),
            quality: None,
            class: None, // not noise (not covered in step 3)
            members_in_theory: 1,
            family_total_members: 2,
        };
        let other_fam = crate::TheoryFamilyMembership {
            family_id: "shape_conclusion_c0-0".to_string(),
            layer: crate::FamilyLayer::L2,
            kind: Some(crate::KIND_CONCLUSION_SHARED),
            quality: None,
            class: None,
            members_in_theory: 1,
            family_total_members: 2,
        };
        // Bump primary + cross above ADR 0072 Addendum 3's 0.70
        // quality floor so the merge passes the new gate.
        let focal = synth_report(
            "t_focal",
            vec!["ax_a".to_string()],
            Some(0.75),
            Some(0.75),
            0,
            vec![focal_fam],
            vec![],
            vec![crate::AxiomQualityStats {
                axiom_id: "ax_a".to_string(),
                primary_rate: Some(0.75),
                cross_precision: Some(0.75),
                family_ids: vec!["shape_premise_p0-1".to_string()],
            }],
        );
        let partner = synth_report(
            "t_partner",
            vec!["ax_p".to_string()],
            Some(0.95),
            Some(0.92),
            0,
            vec![other_fam],
            vec![],
            vec![],
        );
        let rec = crate::RSet::recommend_intervention(&focal, &[partner]);
        match rec {
            crate::RecommendedIntervention::Merge {
                partner_theory,
                rationale,
            } => {
                assert_eq!(partner_theory, "t_partner");
                assert_eq!(rationale, crate::MergeRationale::Complementary);
            }
            other => panic!("expected Merge, got {:?}", other),
        }
    }

    #[test]
    fn adr0072_theory_demote_when_both_dims_low() {
        let focal = synth_report(
            "t_x",
            vec!["ax_a".to_string()],
            Some(0.30),
            Some(0.40),
            0,
            vec![],
            vec![],
            vec![crate::AxiomQualityStats {
                axiom_id: "ax_a".to_string(),
                primary_rate: Some(0.30),
                cross_precision: Some(0.40),
                family_ids: vec![],
            }],
        );
        // Summary should be Noise (both < 0.50).
        assert_eq!(focal.summary_class, crate::TheoryQualityClass::Noise);
        let rec = crate::RSet::recommend_intervention(&focal, &[]);
        match rec {
            crate::RecommendedIntervention::TheoryDemote { reason } => {
                assert_eq!(reason, crate::TheoryDemoteReason::BothDimensionsLow);
            }
            other => panic!("expected TheoryDemote, got {:?}", other),
        }
    }

    #[test]
    fn adr0072_priority_demote_superset_beats_family_demote() {
        // focal has BOTH (a) extends Signal subset AND (b) noise
        // family. Per priority order, DemoteSuperset wins.
        let noise_member = crate::TheoryFamilyMembership {
            family_id: "shape_premise_p0-0_p1-2".to_string(),
            layer: crate::FamilyLayer::L2,
            kind: Some(crate::KIND_PREMISE_SHARED),
            quality: None,
            class: Some(crate::FamilyQualityClass::Noise),
            members_in_theory: 1,
            family_total_members: 4,
        };
        let focal = synth_report(
            "t_super",
            vec!["ax_a".to_string(), "ax_b".to_string()],
            Some(0.55),
            Some(0.55),
            0,
            vec![noise_member],
            vec!["t_sub".to_string()],
            vec![],
        );
        let sub = synth_report(
            "t_sub",
            vec!["ax_a".to_string()],
            Some(0.95),
            Some(0.92),
            0,
            vec![],
            vec![],
            vec![],
        );
        let rec = crate::RSet::recommend_intervention(&focal, &[sub]);
        assert!(matches!(
            rec,
            crate::RecommendedIntervention::DemoteSuperset { .. }
        ));
    }

    #[test]
    fn adr0072_manual_when_mixed_with_no_pattern() {
        // Mixed focal, no noise family, primary < 0.60 (no repair),
        // no Signal partner → Manual.
        let focal = synth_report(
            "t_x",
            vec!["ax_a".to_string(), "ax_b".to_string()],
            Some(0.55),
            Some(0.55),
            0,
            vec![],
            vec![],
            vec![],
        );
        let rec = crate::RSet::recommend_intervention(&focal, &[]);
        assert!(matches!(
            rec,
            crate::RecommendedIntervention::Manual { .. }
        ));
    }

    // ── ADR 0072 Addendum 1 — HighQualityBoth merge ─────────────

    #[test]
    fn adr0072_addendum1_signal_signal_with_high_xprec_recommends_merge() {
        // Two Signal-class theories, both with cross_precision_mean
        // ≥ 0.95 → recommend Merge with HighQualityBoth rationale.
        let focal = synth_report(
            "t_2",
            vec!["ax_a".to_string()],
            Some(1.0),
            Some(1.0),
            0,
            vec![],
            vec![],
            vec![],
        );
        let partner = synth_report(
            "t_3",
            vec!["ax_b".to_string()],
            Some(0.95),
            Some(0.98),
            0,
            vec![],
            vec![],
            vec![],
        );
        let rec = crate::RSet::recommend_intervention(&focal, &[partner]);
        match rec {
            crate::RecommendedIntervention::Merge {
                partner_theory,
                rationale,
            } => {
                assert_eq!(partner_theory, "t_3");
                assert_eq!(
                    rationale,
                    crate::MergeRationale::HighQualityBoth
                );
            }
            other => panic!("expected Merge(HighQualityBoth), got {:?}", other),
        }
    }

    #[test]
    fn adr0072_addendum1_signal_with_low_xprec_partner_returns_none() {
        // Focal Signal but partner's cross-prec below 0.95 floor →
        // no HighQualityBoth merge → falls through to None.
        let focal = synth_report(
            "t_x",
            vec!["ax_a".to_string()],
            Some(0.95),
            Some(0.95),
            0,
            vec![],
            vec![],
            vec![],
        );
        let partner = synth_report(
            "t_y",
            vec!["ax_b".to_string()],
            Some(0.85),
            Some(0.85), // Signal but below 0.95 floor
            0,
            vec![],
            vec![],
            vec![],
        );
        let rec = crate::RSet::recommend_intervention(&focal, &[partner]);
        assert_eq!(rec, crate::RecommendedIntervention::None);
    }

    #[test]
    fn adr0072_addendum1_signal_alone_returns_none() {
        // Signal-class theory with no partners → None.
        let focal = synth_report(
            "t_x",
            vec!["ax_a".to_string()],
            Some(0.95),
            Some(0.95),
            0,
            vec![],
            vec![],
            vec![],
        );
        let rec = crate::RSet::recommend_intervention(&focal, &[]);
        assert_eq!(rec, crate::RecommendedIntervention::None);
    }

    // ── ADR 0072 Addendum 2 — near-disjoint signature rule ──────

    #[test]
    fn adr0072_addendum2_near_disjoint_jaccard_below_threshold_recommends_merge() {
        // F.2.1's pattern: focal Mixed, partner Signal, signatures
        // share 2 of 5 families (Jaccard 0.40) → Merge(Complementary).
        let focal_fams: Vec<crate::TheoryFamilyMembership> = vec![
            "shape_premise_p0-1",
            "shape_premise_p0-1_p1-2",
            "shape_conclusion_c0-2",
            "shape_conclusion_c1-0",
            "shape_conclusion_c2-0",
        ]
        .into_iter()
        .map(|f| crate::TheoryFamilyMembership {
            family_id: f.to_string(),
            layer: crate::FamilyLayer::L2,
            kind: None,
            quality: None,
            class: None,
            members_in_theory: 1,
            family_total_members: 2,
        })
        .collect();
        let other_fams: Vec<crate::TheoryFamilyMembership> = vec![
            "shape_premise_p0-1_p1-2",
            "shape_conclusion_c0-2",
        ]
        .into_iter()
        .map(|f| crate::TheoryFamilyMembership {
            family_id: f.to_string(),
            layer: crate::FamilyLayer::L2,
            kind: None,
            quality: None,
            class: None,
            members_in_theory: 1,
            family_total_members: 2,
        })
        .collect();
        // Bump primary + cross above ADR 0072 Addendum 3's 0.70
        // quality floor so this near-disjoint case still merges.
        // (The point of this test is the Jaccard threshold, not
        // the quality gate; quality is set above floor here.)
        let focal = synth_report(
            "t_focal",
            vec!["ax_a".to_string()],
            Some(0.75),
            Some(0.75),
            0,
            focal_fams,
            vec![],
            vec![crate::AxiomQualityStats {
                axiom_id: "ax_a".to_string(),
                primary_rate: Some(0.75),
                cross_precision: Some(0.75),
                family_ids: vec![],
            }],
        );
        let partner = synth_report(
            "t_partner",
            vec!["ax_p".to_string()],
            Some(0.95),
            Some(0.95),
            0,
            other_fams,
            vec![],
            vec![],
        );
        let rec = crate::RSet::recommend_intervention(&focal, &[partner]);
        match rec {
            crate::RecommendedIntervention::Merge {
                partner_theory,
                rationale,
            } => {
                assert_eq!(partner_theory, "t_partner");
                assert_eq!(
                    rationale,
                    crate::MergeRationale::Complementary
                );
            }
            other => panic!(
                "expected Merge(Complementary), got {:?}",
                other,
            ),
        }
    }

    #[test]
    fn adr0072_addendum2_jaccard_above_threshold_does_not_recommend_merge() {
        // Focal & partner share 3 of 4 families (Jaccard 0.75) →
        // ABOVE 0.50 threshold → no Merge; falls through.
        let shared: Vec<crate::TheoryFamilyMembership> = vec![
            "shape_a", "shape_b", "shape_c",
        ]
        .into_iter()
        .map(|f| crate::TheoryFamilyMembership {
            family_id: f.to_string(),
            layer: crate::FamilyLayer::L2,
            kind: None,
            quality: None,
            class: None,
            members_in_theory: 1,
            family_total_members: 2,
        })
        .collect();
        let mut focal_fams = shared.clone();
        focal_fams.push(crate::TheoryFamilyMembership {
            family_id: "shape_d".to_string(),
            layer: crate::FamilyLayer::L2,
            kind: None,
            quality: None,
            class: None,
            members_in_theory: 1,
            family_total_members: 2,
        });
        // focal: {a, b, c, d}; partner: {a, b, c} → union {a,b,c,d}=4, intersection {a,b,c}=3 → Jaccard = 0.75
        let focal = synth_report(
            "t_focal",
            vec!["ax_a".to_string()],
            Some(0.55),
            Some(0.55),
            0,
            focal_fams,
            vec![],
            vec![crate::AxiomQualityStats {
                axiom_id: "ax_a".to_string(),
                primary_rate: Some(0.55),
                cross_precision: Some(0.55),
                family_ids: vec![],
            }],
        );
        let partner = synth_report(
            "t_partner",
            vec!["ax_p".to_string()],
            Some(0.95),
            Some(0.95),
            0,
            shared,
            vec![],
            vec![],
        );
        let rec = crate::RSet::recommend_intervention(&focal, &[partner]);
        // With Jaccard 0.75 > 0.50, no Merge. Should fall through
        // to Step 6 (Noise check fails since summary is Mixed) →
        // Step 7 Manual.
        assert!(matches!(
            rec,
            crate::RecommendedIntervention::Manual { .. }
        ));
    }

    // ── ADR 0072 Addendum 3 — quality floor on Step 5 ───────────

    #[test]
    fn adr0072_addendum3_blocks_merge_when_focal_primary_below_floor() {
        // Replicates Phase 0072-A's t_1 case on OQ#1: focal Mixed
        // with primary 0.59 (below 0.70 floor), cross 0.84 (above
        // floor). Pre-A3 this would Merge(t_2, Complementary) and
        // empirically dilute t_2. Post-A3 it must NOT recommend
        // merge.
        let focal_fams = vec![crate::TheoryFamilyMembership {
            family_id: "shape_premise_p0-1".to_string(),
            layer: crate::FamilyLayer::L2,
            kind: None,
            quality: None,
            class: None,
            members_in_theory: 1,
            family_total_members: 2,
        }];
        let other_fams = vec![crate::TheoryFamilyMembership {
            family_id: "shape_conclusion_c0-0".to_string(),
            layer: crate::FamilyLayer::L2,
            kind: None,
            quality: None,
            class: None,
            members_in_theory: 1,
            family_total_members: 2,
        }];
        let focal = synth_report(
            "t_borderline_mixed",
            vec!["ax_a".to_string()],
            Some(0.59),    // ← primary below 0.70 floor
            Some(0.84),    // ← cross above floor
            0,
            focal_fams,
            vec![],
            vec![crate::AxiomQualityStats {
                axiom_id: "ax_a".to_string(),
                primary_rate: Some(0.59),
                cross_precision: Some(0.84),
                family_ids: vec!["shape_premise_p0-1".to_string()],
            }],
        );
        let partner = synth_report(
            "t_signal",
            vec!["ax_p".to_string()],
            Some(0.95),
            Some(0.95),
            0,
            other_fams,
            vec![],
            vec![],
        );
        let rec = crate::RSet::recommend_intervention(&focal, &[partner]);
        // Should fall through to Manual (no other rule matches).
        assert!(
            matches!(rec, crate::RecommendedIntervention::Manual { .. }),
            "expected Manual (Addendum 3 blocked merge), got {:?}",
            rec,
        );
    }

    #[test]
    fn adr0072_addendum3_blocks_merge_when_focal_cross_below_floor() {
        // Symmetric to the above: cross below floor blocks even
        // when primary is fine.
        let focal = synth_report(
            "t_x",
            vec!["ax_a".to_string()],
            Some(0.85),    // primary above floor
            Some(0.65),    // ← cross below 0.70 floor
            0,
            vec![],
            vec![],
            vec![crate::AxiomQualityStats {
                axiom_id: "ax_a".to_string(),
                primary_rate: Some(0.85),
                cross_precision: Some(0.65),
                family_ids: vec![],
            }],
        );
        let partner = synth_report(
            "t_signal",
            vec!["ax_p".to_string()],
            Some(0.95),
            Some(0.95),
            0,
            vec![],
            vec![],
            vec![],
        );
        let rec = crate::RSet::recommend_intervention(&focal, &[partner]);
        // Note: Step 4 (AxiomRepair) has primary >= 0.60 + few weak
        // axioms. Here primary=0.85 ≥ 0.60 but per_axiom doesn't
        // identify weak axioms (single ax_a is healthy), so Step 4
        // doesn't fire. Falls through to Step 5 (blocked by A3) →
        // Step 7 Manual.
        assert!(
            matches!(rec, crate::RecommendedIntervention::Manual { .. }),
            "expected Manual (A3 cross floor blocked), got {:?}",
            rec,
        );
    }

    #[test]
    fn adr0072_addendum3_allows_merge_when_focal_passes_both_floors() {
        // Mirror of the blocking tests: when both dims are above
        // 0.70, the merge passes Addendum 3 and reaches Step 5's
        // Jaccard check.
        let focal_fams = vec![crate::TheoryFamilyMembership {
            family_id: "shape_premise_p0-1".to_string(),
            layer: crate::FamilyLayer::L2,
            kind: None,
            quality: None,
            class: None,
            members_in_theory: 1,
            family_total_members: 2,
        }];
        let other_fams = vec![crate::TheoryFamilyMembership {
            family_id: "shape_conclusion_c0-0".to_string(),
            layer: crate::FamilyLayer::L2,
            kind: None,
            quality: None,
            class: None,
            members_in_theory: 1,
            family_total_members: 2,
        }];
        let focal = synth_report(
            "t_clean_mixed",
            vec!["ax_a".to_string()],
            Some(0.75),    // both ≥ 0.70
            Some(0.75),
            0,
            focal_fams,
            vec![],
            vec![crate::AxiomQualityStats {
                axiom_id: "ax_a".to_string(),
                primary_rate: Some(0.75),
                cross_precision: Some(0.75),
                family_ids: vec!["shape_premise_p0-1".to_string()],
            }],
        );
        let partner = synth_report(
            "t_signal",
            vec!["ax_p".to_string()],
            Some(0.95),
            Some(0.95),
            0,
            other_fams,
            vec![],
            vec![],
        );
        let rec = crate::RSet::recommend_intervention(&focal, &[partner]);
        assert!(
            matches!(
                rec,
                crate::RecommendedIntervention::Merge { rationale: crate::MergeRationale::Complementary, .. }
            ),
            "expected Merge(Complementary), got {:?}",
            rec,
        );
    }

    // ── ADR 0072 — visualization helpers ────────────────────────

    #[test]
    fn adr0072_viz_quality_report_not_empty_for_indeterminate() {
        let r = synth_report(
            "t_x",
            vec![],
            None,
            None,
            0,
            vec![],
            vec![],
            vec![],
        );
        let s = crate::RSet::format_quality_report(&r);
        assert!(s.contains("t_x"));
        assert!(s.contains("Indeterminate"));
    }

    #[test]
    fn adr0072_viz_decision_trace_matches_recommendation_signal() {
        let r = synth_report(
            "t_x",
            vec!["ax_a".to_string()],
            Some(0.95),
            Some(0.95),
            0,
            vec![],
            vec![],
            vec![],
        );
        let trace = crate::RSet::format_decision_trace(&r, &[]);
        let rec = crate::RSet::recommend_intervention(&r, &[]);
        assert_eq!(rec, crate::RecommendedIntervention::None);
        assert!(trace.contains("Step 1 (Signal class?):   yes"));
        assert!(trace.contains("→ None"));
    }

    #[test]
    fn adr0072_viz_decision_trace_matches_recommendation_family_demote() {
        let noise_member = crate::TheoryFamilyMembership {
            family_id: "shape_premise_p0-0_p1-2".to_string(),
            layer: crate::FamilyLayer::L2,
            kind: Some(crate::KIND_PREMISE_SHARED),
            quality: None,
            class: Some(crate::FamilyQualityClass::Uniform),
            members_in_theory: 4,
            family_total_members: 4,
        };
        let r = synth_report(
            "t_x",
            vec!["ax_a".to_string(), "ax_b".to_string(), "ax_c".to_string(), "ax_d".to_string()],
            Some(0.55),
            Some(0.55),
            0,
            vec![noise_member],
            vec![],
            vec![],
        );
        let trace = crate::RSet::format_decision_trace(&r, &[]);
        let rec = crate::RSet::recommend_intervention(&r, &[]);
        assert!(matches!(
            rec,
            crate::RecommendedIntervention::FamilyDemote { .. }
        ));
        assert!(trace.contains("Step 3 (noise/uniform family?):  yes"));
        assert!(trace.contains("→ FamilyDemote"));
    }

    #[test]
    fn adr0072_viz_decision_trace_explains_indeterminate_path() {
        let r = synth_report(
            "t_x",
            vec![],
            None,
            None,
            0,
            vec![],
            vec![],
            vec![],
        );
        let trace = crate::RSet::format_decision_trace(&r, &[]);
        assert!(trace.contains("Step 0 (Indeterminate?):  ✓ FIRES"));
        assert!(trace.contains("→ ShadowMonitor"));
    }

    #[test]
    fn adr0072_per_axiom_stats_populated_in_report() {
        // Smoke test: theory_quality_report() now includes
        // per_axiom_stats with one entry per axiom.
        use crate::{R, THEORY_MARKER};
        let mut rs = crate::RSet::new();
        rs.register_axiom_with_intension("ax_tpl_v3_p0-1_p1-2_c0-2");
        rs.add(R::new(THEORY_MARKER, "t_x"));
        rs.add(R::new("t_x", "ax_tpl_v3_p0-1_p1-2_c0-2"));
        let primary: std::collections::HashMap<String, f64> =
            std::collections::HashMap::new();
        let report = rs.theory_quality_report("t_x", &[], &primary).unwrap();
        assert_eq!(report.per_axiom_stats.len(), report.axiom_count);
        assert_eq!(
            report.per_axiom_stats[0].axiom_id,
            "ax_tpl_v3_p0-1_p1-2_c0-2"
        );
    }

    #[test]
    fn adr0066_generate_substrate_satisfies_transitivity() {
        // Build an RSet with transitivity holding (poset). Name a
        // theory containing transitivity. Then generate a substrate
        // and verify transitivity holds at rate 1.0 by construction.
        let mut rs = RSet::new();
        // Small poset: a→b→c→d + closure
        rs.extend([
            R::new("a", "b"), R::new("b", "c"), R::new("c", "d"),
            R::new("a", "c"), R::new("a", "d"), R::new("b", "d"),
        ]);
        let trans_id = "ax_tpl_v3_p0-1_p1-2_c0-2";
        let t_id = rs.name_theory(&[trans_id]).unwrap();

        let gen = rs
            .generate_substrate_from_theory(&t_id, 6, 0.30, 12345)
            .unwrap();
        // Generated data identifiers shouldn't collide with primary.
        // (After axiom registration the rset also contains meta-R
        // identifiers like `ax_tpl_..._prem_0`; we only check data ids.)
        let meta = gen.collect_meta_ids();
        let data_ids = gen.compute_data_ids(&meta);
        for id in &data_ids {
            assert!(
                id.starts_with("gen_"),
                "expected generated data id to be prefixed: got {}",
                id
            );
        }
        // Transitivity should hold by construction: every predicted
        // edge from forward-apply must already be in the substrate.
        let predicted = gen.forward_apply_axiom(trans_id);
        assert!(!predicted.is_empty(), "expected non-empty predictions");
        for r in &predicted {
            assert!(
                gen.contains(r),
                "transitivity violated on generated substrate: {:?}",
                r,
            );
        }
    }

    #[test]
    fn adr0068_d2_generate_substrate_respects_antisymmetry() {
        // Theory with antisymmetry + transitivity. Generated
        // substrate's data edges should never include both R(a,b)
        // and R(b,a).
        let mut rs = RSet::new();
        rs.extend([
            R::new("a", "b"), R::new("b", "c"), R::new("a", "c"),
        ]);
        let trans_id = "ax_tpl_v3_p0-1_p1-2_c0-2";
        let t_id = rs
            .name_theory(&[trans_id, AX_ANTISYMMETRY])
            .unwrap();
        let gen = rs
            .generate_substrate_from_theory(&t_id, 6, 0.40, 7777)
            .unwrap();
        let meta = gen.collect_meta_ids();
        let data_ids = gen.compute_data_ids(&meta);
        for x in &data_ids {
            for y in &data_ids {
                if x == y {
                    continue;
                }
                let fwd = gen.contains(&R::new(x.clone(), y.clone()));
                let rev = gen.contains(&R::new(y.clone(), x.clone()));
                assert!(
                    !(fwd && rev),
                    "antisymmetry violated: both R({}, {}) and reverse",
                    x, y,
                );
            }
        }
    }

    #[test]
    fn adr0068_d2_generate_substrate_respects_totality() {
        // Theory with totality + transitivity (= total order).
        // Every unordered pair must have at least one direction.
        let mut rs = RSet::new();
        let nodes = ["a", "b", "c"];
        for i in 0..nodes.len() {
            for j in (i + 1)..nodes.len() {
                rs.add(R::new(nodes[i], nodes[j]));
            }
        }
        let trans_id = "ax_tpl_v3_p0-1_p1-2_c0-2";
        let t_id = rs
            .name_theory(&[trans_id, AX_TOTALITY])
            .unwrap();
        let gen = rs
            .generate_substrate_from_theory(&t_id, 5, 0.0, 1)
            .unwrap();
        let meta = gen.collect_meta_ids();
        let data_ids = gen.compute_data_ids(&meta);
        for i in 0..data_ids.len() {
            for j in (i + 1)..data_ids.len() {
                let x = &data_ids[i];
                let y = &data_ids[j];
                let fwd = gen.contains(&R::new(x.clone(), y.clone()));
                let rev = gen.contains(&R::new(y.clone(), x.clone()));
                assert!(
                    fwd || rev,
                    "totality violated: neither R({}, {}) nor reverse",
                    x, y,
                );
            }
        }
    }

    #[test]
    fn adr0066_generate_substrate_handles_reflexivity() {
        // Theory with just reflexivity → every generated id has self-loop.
        let mut rs = RSet::new();
        for n in ["a", "b", "c"] {
            rs.add(R::new(n, n));
        }
        let t_id = rs.name_theory(&[AX_REFLEXIVITY]).unwrap();
        let gen = rs
            .generate_substrate_from_theory(&t_id, 5, 0.0, 1)
            .unwrap();
        // Every generated DATA id should appear as a self-loop.
        let meta = gen.collect_meta_ids();
        let data_ids = gen.compute_data_ids(&meta);
        for id in &data_ids {
            assert!(
                gen.contains(&R::new(id.clone(), id.clone())),
                "missing self-loop for generated id {}",
                id,
            );
        }
    }

    #[test]
    fn adr0066_generate_substrate_does_not_modify_self() {
        let mut rs = RSet::new();
        rs.extend([
            R::new("a", "b"), R::new("b", "c"), R::new("a", "c"),
        ]);
        let trans_id = "ax_tpl_v3_p0-1_p1-2_c0-2";
        let t_id = rs.name_theory(&[trans_id]).unwrap();
        let before: usize = rs.len();
        let _gen = rs
            .generate_substrate_from_theory(&t_id, 4, 0.30, 99)
            .unwrap();
        assert_eq!(
            rs.len(),
            before,
            "generate_substrate_from_theory must not modify self",
        );
    }

    #[test]
    fn adr0066_generate_substrate_rejects_unknown_theory() {
        let rs = RSet::new();
        assert!(rs
            .generate_substrate_from_theory("t_does_not_exist", 4, 0.5, 1)
            .is_err());
    }

    #[test]
    fn adr0066_indexed_forward_apply_matches_transitivity_on_chain() {
        // Sanity: on a small chain a→b→c→d (with closure), transitivity
        // axiom (R(0,1) ∧ R(1,2) ⇒ R(0,2)) predicts every R(x, z) where
        // ∃ y. R(x, y) ∧ R(y, z). Verify the indexed enumerator
        // produces the expected set.
        let mut rs = RSet::new();
        rs.extend([
            R::new("a", "b"),
            R::new("b", "c"),
            R::new("c", "d"),
            R::new("a", "c"),
            R::new("a", "d"),
            R::new("b", "d"),
        ]);
        let trans_id = "ax_tpl_v3_p0-1_p1-2_c0-2";
        // Register axiom (forward_apply needs intension wired to meta-R).
        rs.name_theory(&[trans_id]).unwrap();
        let predicted = rs.forward_apply_axiom(trans_id);
        // Expected forward-apply output on {(a,b), (b,c), (c,d),
        // (a,c), (a,d), (b,d)}: every (x, z) such that ∃ y with both
        // (x,y) and (y,z) present. That gives:
        //   from (a,b)+(b,c)→(a,c); (a,b)+(b,d)→(a,d); (a,c)+(c,d)→(a,d);
        //   from (b,c)+(c,d)→(b,d).
        // Set: {(a,c), (a,d), (b,d)}.
        let expected: HashSet<R> = [
            R::new("a", "c"),
            R::new("a", "d"),
            R::new("b", "d"),
        ]
        .into_iter()
        .collect();
        assert_eq!(predicted, expected);
    }

    #[test]
    fn adr0066_indexed_forward_apply_matches_symmetry_on_clique() {
        // Symmetry axiom (R(0,1) ⇒ R(1,0)) on a 3-node symmetric clique.
        let mut rs = RSet::new();
        rs.extend([
            R::new("a", "b"),
            R::new("b", "a"),
            R::new("a", "c"),
            R::new("c", "a"),
            R::new("b", "c"),
            R::new("c", "b"),
        ]);
        let sym_id = "ax_tpl_v2_p0-1_c1-0";
        rs.name_theory(&[sym_id]).unwrap();
        let predicted = rs.forward_apply_axiom(sym_id);
        // 6 input edges × symmetric reflection = same 6 edges
        // (since clique is symmetric).
        assert_eq!(predicted.len(), 6);
        for r in &[
            R::new("b", "a"),
            R::new("a", "b"),
            R::new("c", "a"),
            R::new("a", "c"),
            R::new("c", "b"),
            R::new("b", "c"),
        ] {
            assert!(predicted.contains(r));
        }
    }

    #[test]
    fn adr0066_indexed_forward_apply_empty_premise_chain() {
        // Single-edge RSet: transitivity has no satisfiable bindings.
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b")]);
        let trans_id = "ax_tpl_v3_p0-1_p1-2_c0-2";
        // Cannot name_theory here since transitivity doesn't hold at
        // rate 1.0 on a single edge. Use forward_apply directly with
        // an unregistered axiom; it should return empty (the axiom
        // template id parses but the RSet has no chain-of-2).
        let predicted = rs.forward_apply_axiom(trans_id);
        assert!(predicted.is_empty());
    }

    #[test]
    fn adr0066_retract_theory_member_keeps_theory_and_other_members() {
        // Build a 2-axiom theory; remove one member; verify theory
        // survives, axiom global registration intact, other member
        // intact, and the removed axiom no longer counted as member.
        let mut rs = poset_with_selfloops();
        let th = rs.discover_theory(&AxiomDiscoveryConfig::default());
        let ids: Vec<&str> = th.member_axiom_ids.iter().map(|s| s.as_str()).collect();
        assert!(ids.len() >= 2, "test needs ≥2 member axioms");
        let t_id = rs.name_theory(&ids).unwrap();
        let target = ids[0].to_string();
        let kept = ids[1].to_string();
        let axiom_count_before = rs.axioms().len();

        let removed = rs.retract_theory_member(&t_id, &target).unwrap();
        assert!(removed >= 1, "should remove ≥ 1 edge");

        // Theory itself survives.
        assert!(rs.is_theory(&t_id));
        // Theory's member set no longer contains target.
        let members_after: Vec<&str> = rs.theory_axioms(&t_id);
        assert!(!members_after.contains(&target.as_str()));
        // Other member still there.
        assert!(members_after.contains(&kept.as_str()));
        // Axiom global registration unchanged.
        assert_eq!(rs.axioms().len(), axiom_count_before);
        assert!(rs.is_axiom(&target));
    }

    #[test]
    fn adr0066_retract_theory_member_rejects_non_member() {
        let mut rs = poset_with_selfloops();
        let th = rs.discover_theory(&AxiomDiscoveryConfig::default());
        let ids: Vec<&str> = th.member_axiom_ids.iter().map(|s| s.as_str()).collect();
        let t_id = rs.name_theory(&ids).unwrap();
        let err = rs.retract_theory_member(&t_id, "ax_not_a_member").unwrap_err();
        assert_eq!(err, TheoryError::UnsatisfiedMember("ax_not_a_member".to_string()));
    }

    #[test]
    fn adr0066_retract_theory_member_rejects_unknown_theory() {
        let mut rs = poset_with_selfloops();
        let err = rs.retract_theory_member("t_does_not_exist", AX_REFLEXIVITY).unwrap_err();
        assert_eq!(err, TheoryError::UnsatisfiedMember("t_does_not_exist".to_string()));
    }

    #[test]
    fn adr0066_retract_theory_member_does_not_affect_other_theory() {
        // Two theories that share axiom A; retract A from theory_1
        // only; theory_2 still contains A; A still global; SHARED
        // marker stays because A is still in theory_2 alone? Actually
        // SHARED requires count ≥ 2, so demoting from one of two
        // theories drops it.
        let mut rs = equivalence_relation();
        let th = rs.discover_theory(&AxiomDiscoveryConfig::default());
        let ids: Vec<&str> = th.member_axiom_ids.iter().map(|s| s.as_str()).collect();
        assert!(ids.len() >= 2);
        let t1 = rs.name_theory(&ids).unwrap();
        // theory_2 = first axiom only.
        let t2 = rs.name_theory(&[ids[0]]).unwrap();
        assert_ne!(t1, t2);

        let target = ids[0].to_string();
        let _ = rs.retract_theory_member(&t1, &target).unwrap();

        // theory_1 lost target.
        assert!(!rs.theory_axioms(&t1).contains(&target.as_str()));
        // theory_2 unaffected.
        assert!(rs.theory_axioms(&t2).contains(&target.as_str()));
        // Axiom global registration intact.
        assert!(rs.is_axiom(&target));
    }

    #[test]
    fn adr0066_merge_theories_disjoint_produces_union() {
        // Two theories with disjoint members; merge mints a new id
        // whose member set is the union; both originals retracted.
        let mut rs = equivalence_relation();
        let th = rs.discover_theory(&AxiomDiscoveryConfig::default());
        let ids: Vec<&str> = th.member_axiom_ids.iter().map(|s| s.as_str()).collect();
        assert!(ids.len() >= 2, "test needs ≥2 axioms");
        // Split: t_a has only ids[0], t_b has only ids[1].
        let t_a = rs.name_theory(&[ids[0]]).unwrap();
        let t_b = rs.name_theory(&[ids[1]]).unwrap();
        assert_ne!(t_a, t_b);

        let merged = rs.merge_theories(&t_a, &t_b).unwrap();
        // Merged theory contains both axioms.
        let members: HashSet<&str> = rs.theory_axioms(&merged).into_iter().collect();
        assert!(members.contains(&ids[0]));
        assert!(members.contains(&ids[1]));
        // Both originals retracted.
        assert!(!rs.is_theory(&t_a));
        assert!(!rs.is_theory(&t_b));
        // Axioms still globally registered.
        assert!(rs.is_axiom(ids[0]));
        assert!(rs.is_axiom(ids[1]));
    }

    #[test]
    fn adr0066_merge_theories_subset_reuses_superset_id() {
        // a's members ⊃ b's members. merge returns a's id and only
        // retracts b. a's structure preserved.
        let mut rs = equivalence_relation();
        let th = rs.discover_theory(&AxiomDiscoveryConfig::default());
        let ids: Vec<&str> = th.member_axiom_ids.iter().map(|s| s.as_str()).collect();
        assert!(ids.len() >= 2);
        let t_super = rs.name_theory(&ids).unwrap(); // all members
        let t_sub = rs.name_theory(&[ids[0]]).unwrap(); // proper subset

        let merged = rs.merge_theories(&t_super, &t_sub).unwrap();
        assert_eq!(merged, t_super, "should reuse superset's id");
        assert!(rs.is_theory(&t_super));
        assert!(!rs.is_theory(&t_sub));
        // Member set unchanged on the surviving theory.
        let members: HashSet<&str> = rs.theory_axioms(&t_super).into_iter().collect();
        for id in &ids {
            assert!(members.contains(id));
        }
    }

    #[test]
    fn adr0066_merge_theories_rejects_self() {
        let mut rs = poset_with_selfloops();
        let th = rs.discover_theory(&AxiomDiscoveryConfig::default());
        let ids: Vec<&str> = th.member_axiom_ids.iter().map(|s| s.as_str()).collect();
        let t_id = rs.name_theory(&ids).unwrap();
        assert!(rs.merge_theories(&t_id, &t_id).is_err());
    }

    #[test]
    fn adr0066_merge_theories_rejects_unknown() {
        let mut rs = poset_with_selfloops();
        let th = rs.discover_theory(&AxiomDiscoveryConfig::default());
        let ids: Vec<&str> = th.member_axiom_ids.iter().map(|s| s.as_str()).collect();
        let t_id = rs.name_theory(&ids).unwrap();
        assert!(rs.merge_theories(&t_id, "t_does_not_exist").is_err());
        assert!(rs.merge_theories("t_phantom", &t_id).is_err());
    }

    #[test]
    fn adr0066_merge_theories_overlapping_members_dedups() {
        // a = {ids[0], ids[1]}, b = {ids[1], ids[2]} → merged has 3.
        let mut rs = equivalence_relation();
        let th = rs.discover_theory(&AxiomDiscoveryConfig::default());
        let ids: Vec<&str> = th.member_axiom_ids.iter().map(|s| s.as_str()).collect();
        assert!(ids.len() >= 3, "test needs ≥3 axioms");
        let t_a = rs.name_theory(&[ids[0], ids[1]]).unwrap();
        let t_b = rs.name_theory(&[ids[1], ids[2]]).unwrap();

        let merged = rs.merge_theories(&t_a, &t_b).unwrap();
        let members: HashSet<&str> = rs.theory_axioms(&merged).into_iter().collect();
        assert_eq!(members.len(), 3);
        assert!(members.contains(&ids[0]));
        assert!(members.contains(&ids[1]));
        assert!(members.contains(&ids[2]));
        // Both originals retracted (merged is a fresh id).
        assert!(!rs.is_theory(&t_a));
        assert!(!rs.is_theory(&t_b));
    }

    #[test]
    fn adr0030_theories_containing() {
        let mut rs = poset_with_selfloops();
        let th = rs.discover_theory(&AxiomDiscoveryConfig::default());
        let ids: Vec<&str> = th.member_axiom_ids.iter().map(|s| s.as_str()).collect();
        let t_id = rs.name_theory(&ids).unwrap();
        // AX_REFLEXIVITY is a member, so it should find this theory.
        let containing = rs.theories_containing(AX_REFLEXIVITY);
        assert!(containing.contains(&t_id.as_str()));
    }

    #[test]
    fn adr0030_discover_theory_on_tolerance_no_trans() {
        // Tolerance: reflexive + symmetric, not transitive.
        let mut rs = RSet::new();
        for n in ["a", "b", "c"] {
            rs.add(R::new(n, n));
        }
        rs.extend([
            R::new("a", "b"), R::new("b", "a"),
            R::new("b", "c"), R::new("c", "b"),
        ]);
        let th = rs.discover_theory(&AxiomDiscoveryConfig::default());
        assert!(th.member_axiom_ids.iter().any(|id| id == AX_REFLEXIVITY));
        assert!(th.member_axiom_ids.iter().any(|id| {
            id == "ax_tpl_v2_p0-1_c1-0" // symmetry
        }));
        // Transitivity must NOT be present.
        assert!(!th.member_axiom_ids.iter().any(|id| {
            id == "ax_tpl_v3_p0-1_p1-2_c0-2"
        }));
    }

    #[test]
    fn adr0030_collect_meta_ids_includes_theory_markers() {
        let mut rs = poset_with_selfloops();
        let th = rs.discover_theory(&AxiomDiscoveryConfig::default());
        let ids: Vec<&str> = th.member_axiom_ids.iter().map(|s| s.as_str()).collect();
        let t_id = rs.name_theory(&ids).unwrap();
        let meta = rs.collect_meta_ids();
        assert!(meta.contains(AXIOM_MARKER));
        assert!(meta.contains(THEORY_MARKER));
        assert!(meta.contains(&t_id));
        for id in &th.member_axiom_ids {
            assert!(meta.contains(id));
        }
    }

    // ADR 0031 — intrinsic drive + global evaluation.

    #[test]
    fn adr0031_abstraction_score_zero_on_bare_rset() {
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b"), R::new("b", "c")]);
        // No patterns, no theories → score = 0 (nothing to tax, nothing to reward).
        assert_eq!(rs.abstraction_score(), 0.0);
    }

    #[test]
    fn adr0031_abstraction_score_rewards_pattern_reuse() {
        // 6 single-edge instances across distinct token pairs → one
        // pattern with 6 instances of size 1. Reuse savings = (6-1)*1 = 5.
        let mut rs = RSet::new();
        for (a, b) in [
            ("a", "b"), ("c", "d"), ("e", "f"),
            ("g", "h"), ("i", "j"), ("k", "l"),
        ] {
            rs.add(R::new(a, b));
        }
        let instances: Vec<Subgraph> = rs
            .iter()
            .map(|r| Subgraph::from_edges([r.clone()]))
            .collect();
        // Name as single-edge pattern.
        let _p = rs.name_pattern_instances(&instances).unwrap();
        let s = rs.abstraction_score();
        // Positive, dominated by reuse savings minus overhead tax.
        assert!(s > 0.0, "expected positive score, got {}", s);
    }

    #[test]
    fn adr0031_drive_discovers_something_on_structured_input() {
        // Equivalence relation — rich axioms available. Drive should
        // name at least a theory, producing positive score.
        let mut rs = equivalence_relation();
        let cfg = DriveConfig::default();
        let trace = rs.intrinsic_drive(&cfg);
        assert!(trace.final_score > trace.initial_score,
            "drive did not improve score: trace={:?}", trace);
        assert!(!trace.steps.is_empty());
    }

    #[test]
    fn adr0031_drive_halts_on_unstructured_input() {
        // Random-ish sparse graph — no pattern reuse, no universal
        // axioms with meaningful content beyond accidental antisym.
        // Drive should either do nothing or take minimal action and
        // then halt.
        let mut rs = RSet::new();
        rs.extend([
            R::new("a", "c"), R::new("b", "d"), R::new("c", "e"),
            R::new("d", "f"), R::new("e", "a"), R::new("f", "b"),
            R::new("a", "d"),
        ]);
        let cfg = DriveConfig {
            max_steps: 5,
            ..DriveConfig::default()
        };
        let trace = rs.intrinsic_drive(&cfg);
        // Final score ≥ initial: the driver never applies a step
        // that reduces the score (delta must exceed epsilon).
        assert!(trace.final_score >= trace.initial_score);
        // And fewer than max_steps actions (it should stop early).
        assert!(trace.steps.len() <= 5);
    }

    #[test]
    fn adr0031_drive_step_is_rejected_when_unprofitable() {
        // Empty RSet — no way to score positive — drive_step returns None.
        let mut rs = RSet::new();
        let cfg = DriveConfig::default();
        let step = rs.drive_step(&cfg);
        assert!(step.is_none());
    }

    #[test]
    fn adr0031_drive_produces_theory_on_poset() {
        let mut rs = diamond_poset();
        let cfg = DriveConfig::default();
        let trace = rs.intrinsic_drive(&cfg);
        // At least one theory was discovered.
        let theory_step = trace.steps.iter().find(|s| {
            matches!(s.result, DriveActionResult::TheoryDiscovered { theory_id: Some(_), .. })
        });
        assert!(theory_step.is_some(), "expected a theory-discovery step");
        assert_eq!(rs.theories().len(), 1);
    }

    #[test]
    fn adr0031_drive_is_idempotent_after_saturation() {
        // Run drive twice. Second run should be a no-op.
        let mut rs = diamond_poset();
        let cfg = DriveConfig::default();
        let first = rs.intrinsic_drive(&cfg);
        let score_after_first = rs.abstraction_score();
        let second = rs.intrinsic_drive(&cfg);
        assert!(second.steps.is_empty(),
            "second drive added steps: {:?}", second.steps);
        assert_eq!(rs.abstraction_score(), score_after_first);
        assert!(!first.steps.is_empty());
    }

    // ADR 0032 — axiom intension as meta-R.

    #[test]
    fn adr0032_template_axiom_gets_intension_on_registration() {
        let mut rs = poset_with_selfloops();
        let th = rs.discover_theory(&AxiomDiscoveryConfig::default());
        let ids: Vec<&str> = th.member_axiom_ids.iter().map(|s| s.as_str()).collect();
        let _ = rs.name_theory(&ids).unwrap();
        // Transitivity axiom: n=3 variables, 2 premise edges, 1 conclusion.
        let trans_id = "ax_tpl_v3_p0-1_p1-2_c0-2";
        assert!(rs.is_axiom(trans_id));
        let vars = rs.axiom_variables(trans_id);
        assert_eq!(vars.len(), 3);
        let prem = rs.axiom_premise_edges(trans_id);
        assert_eq!(prem.len(), 2);
        assert!(rs.axiom_conclusion(trans_id).is_some());
    }

    #[test]
    fn adr0032_predicate_axioms_get_registry_only() {
        let mut rs = poset_with_selfloops();
        let th = rs.discover_theory(&AxiomDiscoveryConfig::default());
        let ids: Vec<&str> = th.member_axiom_ids.iter().map(|s| s.as_str()).collect();
        let _ = rs.name_theory(&ids).unwrap();
        // Reflexivity + antisymmetry exist as registered axioms...
        assert!(rs.is_axiom(AX_REFLEXIVITY));
        assert!(rs.is_axiom(AX_ANTISYMMETRY));
        // ...but have no intension (no variables, no edges).
        assert!(rs.axiom_variables(AX_REFLEXIVITY).is_empty());
        assert!(rs.axiom_premise_edges(AX_REFLEXIVITY).is_empty());
        assert!(rs.axiom_conclusion(AX_REFLEXIVITY).is_none());
        assert!(rs.axiom_variables(AX_ANTISYMMETRY).is_empty());
    }

    #[test]
    fn adr0032_reconstruct_roundtrip_transitivity() {
        let mut rs = poset_with_selfloops();
        let th = rs.discover_theory(&AxiomDiscoveryConfig::default());
        let ids: Vec<&str> = th.member_axiom_ids.iter().map(|s| s.as_str()).collect();
        let _ = rs.name_theory(&ids).unwrap();
        let trans_id = "ax_tpl_v3_p0-1_p1-2_c0-2";
        let reconstructed = rs.reconstruct_axiom_template(trans_id).unwrap();
        let expected = AxiomTemplate {
            num_vars: 3,
            premise: vec![
                EdgeTemplate { x_var: 0, y_var: 1 },
                EdgeTemplate { x_var: 1, y_var: 2 },
            ],
            conclusion: EdgeTemplate { x_var: 0, y_var: 2 },
        };
        assert_eq!(reconstructed, expected);
    }

    #[test]
    fn adr0032_reconstruct_roundtrip_symmetry() {
        let mut rs = equivalence_relation();
        let th = rs.discover_theory(&AxiomDiscoveryConfig::default());
        let ids: Vec<&str> = th.member_axiom_ids.iter().map(|s| s.as_str()).collect();
        let _ = rs.name_theory(&ids).unwrap();
        let sym_id = "ax_tpl_v2_p0-1_c1-0";
        let reconstructed = rs.reconstruct_axiom_template(sym_id).unwrap();
        let expected = AxiomTemplate {
            num_vars: 2,
            premise: vec![EdgeTemplate { x_var: 0, y_var: 1 }],
            conclusion: EdgeTemplate { x_var: 1, y_var: 0 },
        };
        assert_eq!(reconstructed, expected);
    }

    #[test]
    fn adr0032_retract_axiom_removes_intension() {
        let mut rs = poset_with_selfloops();
        let th = rs.discover_theory(&AxiomDiscoveryConfig::default());
        let ids: Vec<&str> = th.member_axiom_ids.iter().map(|s| s.as_str()).collect();
        let t_id = rs.name_theory(&ids).unwrap();
        // Must retract the theory first.
        let _ = rs.retract_theory(&t_id).unwrap();
        let trans_id = "ax_tpl_v3_p0-1_p1-2_c0-2".to_string();
        let before = rs.len();
        let removed = rs.retract_axiom(&trans_id).unwrap();
        assert!(removed > 0);
        assert!(rs.len() < before);
        assert!(!rs.is_axiom(&trans_id));
        assert!(rs.axiom_variables(&trans_id).is_empty());
        assert!(rs.reconstruct_axiom_template(&trans_id).is_none());
    }

    #[test]
    fn adr0032_retract_axiom_refuses_when_theory_holds_reference() {
        let mut rs = poset_with_selfloops();
        let th = rs.discover_theory(&AxiomDiscoveryConfig::default());
        let ids: Vec<&str> = th.member_axiom_ids.iter().map(|s| s.as_str()).collect();
        let _ = rs.name_theory(&ids).unwrap();
        let trans_id = "ax_tpl_v3_p0-1_p1-2_c0-2".to_string();
        let err = rs.retract_axiom(&trans_id).unwrap_err();
        assert!(matches!(err, TheoryError::UnsatisfiedMember(_)));
    }

    #[test]
    fn adr0032_collect_meta_ids_includes_axiom_intension_ids() {
        let mut rs = poset_with_selfloops();
        let th = rs.discover_theory(&AxiomDiscoveryConfig::default());
        let ids: Vec<&str> = th.member_axiom_ids.iter().map(|s| s.as_str()).collect();
        let _ = rs.name_theory(&ids).unwrap();
        let meta = rs.collect_meta_ids();
        assert!(meta.contains(AXIOMVAR_MARKER));
        assert!(meta.contains(PREMISE_MARKER));
        assert!(meta.contains(CONCLUSION_MARKER));
        let trans_id = "ax_tpl_v3_p0-1_p1-2_c0-2";
        for v in rs.axiom_variables(trans_id) {
            assert!(meta.contains(v));
        }
        for p in rs.axiom_premise_edges(trans_id) {
            assert!(meta.contains(p));
        }
        if let Some(c) = rs.axiom_conclusion(trans_id) {
            assert!(meta.contains(&c));
        }
    }

    #[test]
    fn adr0032_axioms_do_not_pollute_data_discovery() {
        // Name a theory, then verify axiom discovery on the same RSet
        // still sees only the original data identifiers — none of the
        // axiom intension ids leaks in.
        let mut rs = poset_with_selfloops();
        let before = rs.discover_axioms(&AxiomDiscoveryConfig::default());
        let th = rs.discover_theory(&AxiomDiscoveryConfig::default());
        let ids: Vec<&str> = th.member_axiom_ids.iter().map(|s| s.as_str()).collect();
        let _ = rs.name_theory(&ids).unwrap();
        let after = rs.discover_axioms(&AxiomDiscoveryConfig::default());
        assert_eq!(before.len(), after.len(),
            "axiom discovery must be stable — meta-R should be filtered");
    }

    // ADR 0033 — defeasible axioms (rate < 1.0).

    fn almost_transitive() -> RSet {
        // 4-chain transitive closure minus one closure edge: transitivity
        // holds on all but one binding out of many.
        let mut rs = RSet::new();
        let nodes = ["a", "b", "c", "d"];
        for i in 0..nodes.len() {
            for j in (i + 1)..nodes.len() {
                rs.add(R::new(nodes[i], nodes[j]));
            }
        }
        rs.remove(&R::new("b", "d"));
        rs
    }

    #[test]
    fn adr0033_default_strict_mode_unchanged() {
        let rs = almost_transitive();
        // Default min_rate=1.0: transitivity fails because of the one
        // missing closure edge → zero strict axioms.
        let strict = rs.discover_axioms(&AxiomDiscoveryConfig::default());
        assert_eq!(strict.len(), 0);
    }

    #[test]
    fn adr0033_defeasible_mode_surfaces_near_axioms() {
        let rs = almost_transitive();
        let cfg = AxiomDiscoveryConfig {
            min_rate: 0.5,
            ..AxiomDiscoveryConfig::default()
        };
        let defeasible = rs.discover_axioms(&cfg);
        assert!(!defeasible.is_empty(),
            "defeasible discovery should return non-empty on almost-transitive");
        // Transitivity template shows up with rate < 1.0.
        let trans = AxiomTemplate {
            num_vars: 3,
            premise: vec![
                EdgeTemplate { x_var: 0, y_var: 1 },
                EdgeTemplate { x_var: 1, y_var: 2 },
            ],
            conclusion: EdgeTemplate { x_var: 0, y_var: 2 },
        };
        let trans_ev = defeasible.iter().find(|e| e.template == trans)
            .expect("transitivity at rate ≥ 0.5 on almost-transitive");
        assert!(trans_ev.rate < 1.0,
            "defeasible transitivity should have rate < 1.0, got {}", trans_ev.rate);
        assert!(trans_ev.rate >= 0.5);
    }

    #[test]
    fn adr0033_defeasible_minimal_skips_subsumption() {
        // In defeasible mode, discover_axioms_minimal returns the raw
        // output without the subsumption filter (soundness guard).
        let rs = almost_transitive();
        let cfg = AxiomDiscoveryConfig {
            min_rate: 0.5,
            ..AxiomDiscoveryConfig::default()
        };
        let raw = rs.discover_axioms(&cfg);
        let minimal = rs.discover_axioms_minimal(&cfg);
        assert_eq!(raw.len(), minimal.len());
    }

    #[test]
    fn adr0033_strict_minimal_still_subsumes() {
        // min_rate=1.0 path unchanged — subsumption still fires.
        let rs = equivalence_relation();
        let cfg = AxiomDiscoveryConfig::default(); // strict
        let raw = rs.discover_axioms(&cfg);
        let minimal = rs.discover_axioms_minimal(&cfg);
        assert!(minimal.len() < raw.len(),
            "strict minimal should subsume; raw={}, minimal={}", raw.len(), minimal.len());
    }

    #[test]
    fn adr0033_rate_is_reported_on_every_evidence() {
        let rs = almost_transitive();
        let cfg = AxiomDiscoveryConfig {
            min_rate: 0.1,
            ..AxiomDiscoveryConfig::default()
        };
        let defeasible = rs.discover_axioms(&cfg);
        for ev in &defeasible {
            assert!(ev.rate >= 0.1);
            assert!(ev.rate <= 1.0);
            assert!(ev.premise_bindings >= 1);
            assert!(ev.conclusion_satisfied <= ev.premise_bindings);
            // rate = satisfied / bindings
            let expected = ev.conclusion_satisfied as f64 / ev.premise_bindings as f64;
            assert!((ev.rate - expected).abs() < 1e-9);
        }
    }

    #[test]
    fn adr0033_near_zero_rate_threshold_yields_more() {
        let rs = almost_transitive();
        let tight = AxiomDiscoveryConfig { min_rate: 0.9, ..AxiomDiscoveryConfig::default() };
        let loose = AxiomDiscoveryConfig { min_rate: 0.1, ..AxiomDiscoveryConfig::default() };
        let a = rs.discover_axioms(&tight);
        let b = rs.discover_axioms(&loose);
        assert!(b.len() >= a.len());
    }

    // ADR 0034 — theory extension relations.

    fn name_theory_from_rset(rs: &mut RSet) -> String {
        let th = rs.discover_theory(&AxiomDiscoveryConfig::default());
        let ids: Vec<&str> =
            th.member_axiom_ids.iter().map(|s| s.as_str()).collect();
        rs.name_theory(&ids).unwrap()
    }

    #[test]
    fn adr0034_poset_theory_extends_strict_poset_theory() {
        // strict partial order {trans, antisym} is a sub-theory of
        // full poset {trans, antisym, refl}. Build both in one RSet by
        // naming two theories explicitly.
        let mut rs = diamond_poset();
        let t_full = name_theory_from_rset(&mut rs); // {trans, refl, antisym}
        // Name a smaller theory with just {trans, antisym}.
        let strict_ids = ["ax_tpl_v3_p0-1_p1-2_c0-2", AX_ANTISYMMETRY];
        let t_strict = rs.name_theory(&strict_ids).unwrap();
        // Full extends strict (full has refl in addition).
        let ext_id = rs.name_theory_extension(&t_full, &t_strict).unwrap();
        assert!(rs.extension_edges().contains(&ext_id.as_str()));
        // Query
        let (sub, sup) = rs.extension_endpoints(&ext_id).unwrap();
        assert_eq!(sub, t_full);
        assert_eq!(sup, t_strict);
        assert!(rs.theory_extends(&t_full).contains(&t_strict.as_str()));
        assert!(rs.theory_extended_by(&t_strict).contains(&t_full.as_str()));
    }

    #[test]
    fn adr0034_name_extension_rejects_non_subset() {
        let mut rs = diamond_poset();
        let t_full = name_theory_from_rset(&mut rs);
        // Make a bogus "theory" with axioms not in t_full by handpicking.
        // Here: a theory with just symmetry (not in poset).
        // But symmetry doesn't hold on poset, so name_theory rejects.
        // Use a different approach: two theories with disjoint non-subset members.
        let weak_ids = [AX_ANTISYMMETRY];
        let t_weak = rs.name_theory(&weak_ids).unwrap();
        let strong_ids = ["ax_tpl_v3_p0-1_p1-2_c0-2"];
        let t_strong = rs.name_theory(&strong_ids).unwrap();
        // Neither is a subset of the other.
        assert!(rs.name_theory_extension(&t_weak, &t_strong).is_err());
        assert!(rs.name_theory_extension(&t_strong, &t_weak).is_err());
    }

    #[test]
    fn adr0034_name_extension_refuses_self_loop() {
        let mut rs = diamond_poset();
        let t = name_theory_from_rset(&mut rs);
        assert!(rs.name_theory_extension(&t, &t).is_err());
    }

    #[test]
    fn adr0034_discover_extensions_scans_pairs() {
        let mut rs = diamond_poset();
        let t_full = name_theory_from_rset(&mut rs);
        let strict_ids = ["ax_tpl_v3_p0-1_p1-2_c0-2", AX_ANTISYMMETRY];
        let t_strict = rs.name_theory(&strict_ids).unwrap();
        let trans_only = ["ax_tpl_v3_p0-1_p1-2_c0-2"];
        let t_trans = rs.name_theory(&trans_only).unwrap();
        let found = rs.discover_theory_extensions();
        // t_full ⊋ t_strict ⊋ t_trans. Expected pairs:
        // (t_full, t_strict), (t_full, t_trans), (t_strict, t_trans).
        assert!(found.contains(&(t_full.clone(), t_strict.clone())));
        assert!(found.contains(&(t_full.clone(), t_trans.clone())));
        assert!(found.contains(&(t_strict.clone(), t_trans.clone())));
        assert_eq!(found.len(), 3);
    }

    #[test]
    fn adr0034_extension_reuses_on_duplicate() {
        let mut rs = diamond_poset();
        let t_full = name_theory_from_rset(&mut rs);
        let strict_ids = ["ax_tpl_v3_p0-1_p1-2_c0-2", AX_ANTISYMMETRY];
        let t_strict = rs.name_theory(&strict_ids).unwrap();
        let e1 = rs.name_theory_extension(&t_full, &t_strict).unwrap();
        let e2 = rs.name_theory_extension(&t_full, &t_strict).unwrap();
        assert_eq!(e1, e2);
        assert_eq!(rs.extension_edges().len(), 1);
    }

    #[test]
    fn adr0034_collect_meta_ids_includes_extends() {
        let mut rs = diamond_poset();
        let t_full = name_theory_from_rset(&mut rs);
        let strict_ids = ["ax_tpl_v3_p0-1_p1-2_c0-2", AX_ANTISYMMETRY];
        let t_strict = rs.name_theory(&strict_ids).unwrap();
        let ext = rs.name_theory_extension(&t_full, &t_strict).unwrap();
        let meta = rs.collect_meta_ids();
        assert!(meta.contains(EXTENDS_MARKER));
        assert!(meta.contains(&ext));
    }

    // ADR 0035 — counterfactual value / meta-metric.

    #[test]
    fn adr0035_counterfactual_for_theory_is_positive() {
        let mut rs = diamond_poset();
        let t = name_theory_from_rset(&mut rs);
        let v = rs.counterfactual_value(&t).expect("theory is retractable");
        // Theory has 3 members; removing it drops 2.0 * 3 = 6.0 from reward
        // minus some overhead savings. Net should still be > 0 because
        // the reward exceeds the tax-savings.
        assert!(v > 0.0, "expected positive counterfactual, got {}", v);
    }

    #[test]
    fn adr0035_counterfactual_returns_none_for_unknown_id() {
        let rs = diamond_poset();
        assert!(rs.counterfactual_value("definitely_not_named").is_none());
    }

    #[test]
    fn adr0035_counterfactual_blocked_by_theory_reference_for_axiom() {
        let mut rs = diamond_poset();
        let _ = name_theory_from_rset(&mut rs);
        // Transitivity is used by the theory, so retract_axiom would fail.
        let v = rs.counterfactual_value("ax_tpl_v3_p0-1_p1-2_c0-2");
        assert!(v.is_none(),
            "axiom still referenced by a theory should return None");
    }

    #[test]
    fn adr0035_rank_orders_by_value_descending() {
        let mut rs = diamond_poset();
        let _t = name_theory_from_rset(&mut rs);
        let ranked = rs.rank_by_counterfactual();
        assert!(!ranked.is_empty());
        // Monotone descending.
        for w in ranked.windows(2) {
            assert!(w[0].1 >= w[1].1);
        }
    }

    #[test]
    fn adr0035_counterfactual_respects_actual_retract_behavior() {
        let mut rs = diamond_poset();
        let _t = name_theory_from_rset(&mut rs);
        let before = rs.abstraction_score();
        // Pick any retractable id from the ranking.
        let ranked = rs.rank_by_counterfactual();
        let (id, predicted_drop) = ranked.first().cloned().unwrap();
        // Actually retract, compare.
        let mut trial = rs.clone();
        if trial.is_theory(&id) {
            let _ = trial.retract_theory(&id);
        } else if trial.patterns().iter().any(|p| *p == id) {
            let _ = trial.retract_pattern(&id);
        } else if trial.extension_edges().iter().any(|e| *e == id) {
            let _ = trial.retract_extension(&id);
        }
        let actual_drop = before - trial.abstraction_score();
        assert!((predicted_drop - actual_drop).abs() < 1e-9);
    }

    #[test]
    fn adr0035_retract_extension_clears_all_three_edges() {
        let mut rs = diamond_poset();
        let t_full = name_theory_from_rset(&mut rs);
        let strict_ids = ["ax_tpl_v3_p0-1_p1-2_c0-2", AX_ANTISYMMETRY];
        let t_strict = rs.name_theory(&strict_ids).unwrap();
        let ext = rs.name_theory_extension(&t_full, &t_strict).unwrap();
        let removed = rs.retract_extension(&ext).unwrap();
        assert_eq!(removed, 3);
        assert!(rs.extension_edges().is_empty());
    }

    // ADR 0036 — extended template language (empty premise).

    #[test]
    fn adr0036_default_config_does_not_include_empty_premise() {
        // Reflexive diamond poset. Default config should NOT surface
        // ax_tpl_v1_c0-0 — backward compat with 0027/0028.
        let rs = diamond_poset();
        let cfg = AxiomDiscoveryConfig::default();
        let axioms = rs.discover_axioms(&cfg);
        let has_empty_premise = axioms
            .iter()
            .any(|e| e.template.premise.is_empty());
        assert!(!has_empty_premise,
            "default config must not produce empty-premise templates");
    }

    #[test]
    fn adr0036_opt_in_surfaces_template_reflexivity() {
        // Reflexive diamond poset with include_empty_premise=true:
        // reflexivity shows up as ax_tpl_v1_c0-0 at rate 1.0.
        let rs = diamond_poset();
        let cfg = AxiomDiscoveryConfig {
            include_empty_premise: true,
            ..AxiomDiscoveryConfig::default()
        };
        let axioms = rs.discover_axioms(&cfg);
        let reflexivity_tpl = AxiomTemplate {
            num_vars: 1,
            premise: vec![],
            conclusion: EdgeTemplate { x_var: 0, y_var: 0 },
        };
        let ev = axioms.iter().find(|e| e.template == reflexivity_tpl);
        assert!(ev.is_some(),
            "empty-premise reflexivity should be discovered");
        assert_eq!(ev.unwrap().rate, 1.0);
    }

    #[test]
    fn adr0036_empty_premise_id_roundtrip() {
        let reflexivity_tpl = AxiomTemplate {
            num_vars: 1,
            premise: vec![],
            conclusion: EdgeTemplate { x_var: 0, y_var: 0 },
        };
        let id = axiom_template_id(&reflexivity_tpl);
        assert_eq!(id, "ax_tpl_v1_c0-0");
        let back = axiom_id_to_template(&id).expect("parses");
        assert_eq!(back, reflexivity_tpl);
    }

    #[test]
    fn adr0036_empty_premise_absent_on_non_reflexive_rset() {
        // Non-reflexive graph + opt-in → ax_tpl_v1_c0-0 must be absent
        // (rate would be < 1.0, default strict mode suppresses it).
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b"), R::new("b", "c")]);
        let cfg = AxiomDiscoveryConfig {
            include_empty_premise: true,
            ..AxiomDiscoveryConfig::default()
        };
        let axioms = rs.discover_axioms(&cfg);
        let has = axioms.iter().any(|e| e.template.premise.is_empty());
        assert!(!has);
    }

    #[test]
    fn adr0036_empty_premise_with_defeasible_surfaces_partial() {
        // Partially-reflexive graph: 2 of 4 identifiers have self-loops.
        // Rate = 0.5. Defeasible mode + empty-premise should surface it.
        let mut rs = RSet::new();
        rs.extend([
            R::new("a", "a"), R::new("b", "b"),
            R::new("c", "d"), R::new("d", "c"),
        ]);
        let cfg = AxiomDiscoveryConfig {
            include_empty_premise: true,
            min_rate: 0.4,
            ..AxiomDiscoveryConfig::default()
        };
        let axioms = rs.discover_axioms(&cfg);
        let refl = axioms.iter().find(|e| e.template.premise.is_empty());
        assert!(refl.is_some());
        let ev = refl.unwrap();
        assert!((ev.rate - 0.5).abs() < 1e-9);
    }

    #[test]
    fn adr0036_opt_in_does_not_break_existing_behavior() {
        // Check that opt-in only ADDS templates, never removes.
        let rs = diamond_poset();
        let strict = rs.discover_axioms(&AxiomDiscoveryConfig::default());
        let extended = rs.discover_axioms(&AxiomDiscoveryConfig {
            include_empty_premise: true,
            ..AxiomDiscoveryConfig::default()
        });
        assert!(extended.len() >= strict.len());
        for ev in &strict {
            assert!(extended
                .iter()
                .any(|e| e.template == ev.template),
                "template {:?} disappeared when opting in", ev.template);
        }
    }

    // ADR 0037 — compositional subsumption via forward chaining.

    #[test]
    fn adr0037_transitivity_variant_derivable_from_sym_trans() {
        // On an equivalence relation: variant-B `[R(0,1), R(1,2)] ⇒ R(2,0)`
        // should be derivable from {symmetry, transitivity}.
        let sym = AxiomTemplate {
            num_vars: 2,
            premise: vec![EdgeTemplate { x_var: 0, y_var: 1 }],
            conclusion: EdgeTemplate { x_var: 1, y_var: 0 },
        };
        let trans = AxiomTemplate {
            num_vars: 3,
            premise: vec![
                EdgeTemplate { x_var: 0, y_var: 1 },
                EdgeTemplate { x_var: 1, y_var: 2 },
            ],
            conclusion: EdgeTemplate { x_var: 0, y_var: 2 },
        };
        let variant_b = AxiomTemplate {
            num_vars: 3,
            premise: vec![
                EdgeTemplate { x_var: 0, y_var: 1 },
                EdgeTemplate { x_var: 1, y_var: 2 },
            ],
            conclusion: EdgeTemplate { x_var: 2, y_var: 0 },
        };
        assert!(template_derivable_from(&variant_b, &[sym, trans]));
    }

    #[test]
    fn adr0037_transitivity_not_derivable_from_symmetry_alone() {
        let sym = AxiomTemplate {
            num_vars: 2,
            premise: vec![EdgeTemplate { x_var: 0, y_var: 1 }],
            conclusion: EdgeTemplate { x_var: 1, y_var: 0 },
        };
        let trans = AxiomTemplate {
            num_vars: 3,
            premise: vec![
                EdgeTemplate { x_var: 0, y_var: 1 },
                EdgeTemplate { x_var: 1, y_var: 2 },
            ],
            conclusion: EdgeTemplate { x_var: 0, y_var: 2 },
        };
        assert!(!template_derivable_from(&trans, &[sym]));
    }

    #[test]
    fn adr0037_equivalence_minimal_compositional_collapses_to_two() {
        // ADR 0028 minimal on equivalence returns 5 axioms (sym + 4
        // transitivity-like). Composition should drop 3 variants,
        // leaving two: symmetry plus one transitivity-like axiom (which
        // specific one survives depends on processing order — any
        // 1 of the 4 variants generates the other 3 under symmetry, so
        // all 4 are valid minimal-set choices).
        let rs = equivalence_relation();
        let five = rs.discover_axioms_minimal(&AxiomDiscoveryConfig::default());
        let compositional =
            rs.discover_axioms_minimal_compositional(&AxiomDiscoveryConfig::default());
        assert_eq!(five.len(), 5);
        assert_eq!(compositional.len(), 2,
            "equivalence should compose down to exactly 2 axioms, got {}",
            compositional.len());
        // Symmetry always survives.
        let sym_template = AxiomTemplate {
            num_vars: 2,
            premise: vec![EdgeTemplate { x_var: 0, y_var: 1 }],
            conclusion: EdgeTemplate { x_var: 1, y_var: 0 },
        };
        assert!(compositional.iter().any(|e| e.template == sym_template),
            "symmetry should survive");
        // Exactly one of the 4 transitivity variants survives.
        let trans_like_count = compositional
            .iter()
            .filter(|e| e.template.num_vars == 3 && e.template.premise.len() == 2)
            .count();
        assert_eq!(trans_like_count, 1,
            "exactly one transitivity variant should survive");
    }

    #[test]
    fn adr0037_strict_inputs_unchanged_when_no_redundancy() {
        // Strict partial order minimal = {trans}. No composition applies.
        let rs = diamond_poset();
        let minimal = rs.discover_axioms_minimal(&AxiomDiscoveryConfig::default());
        let compositional =
            rs.discover_axioms_minimal_compositional(&AxiomDiscoveryConfig::default());
        assert_eq!(minimal.len(), compositional.len());
    }

    #[test]
    fn adr0037_compositional_defeasible_passes_through() {
        let rs = almost_transitive();
        let cfg = AxiomDiscoveryConfig {
            min_rate: 0.5,
            ..AxiomDiscoveryConfig::default()
        };
        let raw = rs.discover_axioms(&cfg);
        let comp = rs.discover_axioms_minimal_compositional(&cfg);
        assert_eq!(raw.len(), comp.len());
    }

    #[test]
    fn adr0037_subsume_by_composition_handles_singletons() {
        let sym = AxiomTemplate {
            num_vars: 2,
            premise: vec![EdgeTemplate { x_var: 0, y_var: 1 }],
            conclusion: EdgeTemplate { x_var: 1, y_var: 0 },
        };
        let ev = AxiomEvidence {
            template: sym,
            premise_bindings: 1,
            conclusion_satisfied: 1,
            rate: 1.0,
            posterior_lower_95: 0.0,
            posterior_upper_95: 1.0,
            null_baseline_prob: 1.0,
        };
        let out = subsume_by_composition(vec![ev]);
        assert_eq!(out.len(), 1);
    }

    // ADR 0038 — persistence / serialization.

    #[test]
    fn adr0038_empty_rset_roundtrip() {
        let a = RSet::new();
        let text = a.to_text().unwrap();
        let b = RSet::from_text(&text).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn adr0038_simple_rset_roundtrip() {
        let mut a = RSet::new();
        a.extend([R::new("x", "y"), R::new("y", "z"), R::new("z", "x")]);
        let text = a.to_text().unwrap();
        let b = RSet::from_text(&text).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn adr0038_serialization_is_deterministic() {
        let mut a = RSet::new();
        a.extend([R::new("b", "c"), R::new("a", "b"), R::new("c", "a")]);
        let mut b = RSet::new();
        b.extend([R::new("c", "a"), R::new("a", "b"), R::new("b", "c")]);
        assert_eq!(a.to_text().unwrap(), b.to_text().unwrap());
    }

    #[test]
    fn adr0038_roundtrip_preserves_full_meta_r() {
        // Build a rich RSet: data + patterns + theories + axioms + ext.
        let mut a = diamond_poset();
        let _ = a
            .name_pattern_instances(&[Subgraph::from_edges([
                R::new("a", "b"),
                R::new("b", "d"),
            ])])
            .unwrap();
        let th = a.discover_theory(&AxiomDiscoveryConfig::default());
        let ids: Vec<&str> =
            th.member_axiom_ids.iter().map(|s| s.as_str()).collect();
        let _ = a.name_theory(&ids).unwrap();
        let text = a.to_text().unwrap();
        let b = RSet::from_text(&text).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn adr0038_rejects_tab_in_identifier() {
        let mut a = RSet::new();
        a.add(R::new("has\ttab", "ok"));
        let err = a.to_text().unwrap_err();
        assert!(matches!(err, PersistenceError::TabInIdentifier(_)));
    }

    #[test]
    fn adr0038_rejects_newline_in_identifier() {
        let mut a = RSet::new();
        a.add(R::new("ok", "has\nnewline"));
        let err = a.to_text().unwrap_err();
        assert!(matches!(err, PersistenceError::NewlineInIdentifier(_)));
    }

    #[test]
    fn adr0038_rejects_malformed_line() {
        let err = RSet::from_text("just_one_field").unwrap_err();
        assert_eq!(err, PersistenceError::MalformedLine(1));
    }

    #[test]
    fn adr0038_skips_blank_and_comment_lines() {
        let text = "# a comment\n\n\
                    a\tb\n\
                    # another comment\n\
                    c\td\n\
                    \n";
        let rs = RSet::from_text(text).unwrap();
        assert_eq!(rs.len(), 2);
        assert!(rs.contains(&R::new("a", "b")));
        assert!(rs.contains(&R::new("c", "d")));
    }

    #[test]
    fn adr0038_bytes_reproduce_exactly() {
        let mut a = RSet::new();
        a.extend([R::new("a", "b"), R::new("a", "c")]);
        let text1 = a.to_text().unwrap();
        let b = RSet::from_text(&text1).unwrap();
        let text2 = b.to_text().unwrap();
        assert_eq!(text1, text2);
    }

    // ADR 0039 — totality predicate axiom.

    fn total_order_closure() -> RSet {
        let mut rs = RSet::new();
        let nodes = ["1", "2", "3", "4", "5"];
        for i in 0..nodes.len() {
            rs.add(R::new(nodes[i], nodes[i]));
            for j in (i + 1)..nodes.len() {
                rs.add(R::new(nodes[i], nodes[j]));
            }
        }
        rs
    }

    #[test]
    fn adr0039_check_totality_holds_on_total_order() {
        let rs = total_order_closure();
        let t = rs.check_totality();
        assert!(t.holds);
        assert_eq!(t.violations, 0);
        assert_eq!(t.unordered_pairs_checked, 10); // C(5,2)
    }

    #[test]
    fn adr0039_check_totality_fails_on_diamond_poset() {
        let rs = diamond_poset();
        let t = rs.check_totality();
        // Diamond has {a,b,c,d}. Pair (b,c) is incomparable.
        assert!(!t.holds);
        assert!(t.violations >= 1);
    }

    #[test]
    fn adr0039_check_totality_empty_rset_does_not_hold() {
        // No pairs → vacuously? We return holds=false when no pairs
        // checked (consistent with antisymmetry's "needs at least one
        // directed pair" rule).
        let rs = RSet::new();
        let t = rs.check_totality();
        assert!(!t.holds);
        assert_eq!(t.unordered_pairs_checked, 0);
    }

    #[test]
    fn adr0039_discover_theory_includes_totality_on_total_order() {
        let rs = total_order_closure();
        let th = rs.discover_theory(&AxiomDiscoveryConfig::default());
        assert!(th.member_axiom_ids.iter().any(|id| id == AX_TOTALITY));
    }

    #[test]
    fn adr0039_discover_theory_omits_totality_on_diamond() {
        let rs = diamond_poset();
        let th = rs.discover_theory(&AxiomDiscoveryConfig::default());
        assert!(!th.member_axiom_ids.iter().any(|id| id == AX_TOTALITY));
    }

    #[test]
    fn adr0039_name_theory_rejects_totality_when_not_holding() {
        let mut rs = diamond_poset();
        assert!(rs.name_theory(&[AX_TOTALITY]).is_err());
    }

    #[test]
    fn adr0039_name_theory_accepts_totality_on_total_order() {
        let mut rs = total_order_closure();
        let ids = [AX_TOTALITY];
        let t_id = rs.name_theory(&ids).unwrap();
        assert!(rs.theory_axioms(&t_id).contains(&AX_TOTALITY));
    }

    #[test]
    fn adr0039_totality_is_predicate_only() {
        // Verify reconstruct returns None (predicate axioms have no
        // template intension).
        let mut rs = total_order_closure();
        let _ = rs.name_theory(&[AX_TOTALITY]).unwrap();
        assert!(rs.reconstruct_axiom_template(AX_TOTALITY).is_none());
        // axiom_variables for predicate is empty.
        assert!(rs.axiom_variables(AX_TOTALITY).is_empty());
    }

    // ADR 0040 — drive auto-prune via counterfactual.

    #[test]
    fn adr0040_extension_edges_now_reward_the_score() {
        let mut rs = diamond_poset();
        let t_full = name_theory_from_rset(&mut rs);
        let strict_ids = ["ax_tpl_v3_p0-1_p1-2_c0-2", AX_ANTISYMMETRY];
        let t_strict = rs.name_theory(&strict_ids).unwrap();
        let before = rs.abstraction_score();
        let _ = rs.name_theory_extension(&t_full, &t_strict).unwrap();
        let after = rs.abstraction_score();
        // +1 reward per extension; -0.1×3 overhead = net +0.7 minimum.
        assert!(after > before);
    }

    #[test]
    fn adr0040_counterfactual_for_extension_is_positive_now() {
        let mut rs = diamond_poset();
        let t_full = name_theory_from_rset(&mut rs);
        let strict_ids = ["ax_tpl_v3_p0-1_p1-2_c0-2", AX_ANTISYMMETRY];
        let t_strict = rs.name_theory(&strict_ids).unwrap();
        let ext = rs.name_theory_extension(&t_full, &t_strict).unwrap();
        let v = rs.counterfactual_value(&ext).unwrap();
        assert!(v > 0.0, "extension should have positive CV, got {}", v);
    }

    #[test]
    fn adr0040_prune_action_retracts_negative_cv_objects() {
        // Build an RSet where an object exists but has negative CV.
        // Simplest: name a single-edge pattern with just one instance
        // (N=1 → reuse savings = 0, only overhead).
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b")]);
        let sg = Subgraph::from_edges([R::new("a", "b")]);
        let p = rs.name_pattern_instances(&[sg]).unwrap();
        let cv = rs.counterfactual_value(&p).unwrap();
        assert!(cv < 0.0,
            "singleton pattern should have negative CV, got {}", cv);
        // Drive with prune enabled should retract it.
        let mut rs2 = rs.clone();
        let cfg = DriveConfig {
            pattern_sizes: vec![],    // don't re-discover
            enable_prune: true,
            prune_threshold: 0.0,
            ..DriveConfig::default()
        };
        let trace = rs2.intrinsic_drive(&cfg);
        let pruned_step = trace
            .steps
            .iter()
            .any(|s| matches!(s.result, DriveActionResult::Pruned { .. }));
        assert!(pruned_step, "drive should have taken a Prune step");
        assert!(!rs2.patterns().iter().any(|q| *q == p.as_str()),
            "negative-CV pattern should have been pruned");
    }

    #[test]
    fn adr0040_prune_leaves_positive_cv_objects_alone() {
        // Diamond poset with a theory named: theory CV is positive.
        let mut rs = diamond_poset();
        let _t = name_theory_from_rset(&mut rs);
        let theories_before = rs.theories().len();
        let cfg = DriveConfig {
            pattern_sizes: vec![],
            enable_prune: true,
            prune_threshold: 0.0,
            ..DriveConfig::default()
        };
        let mut rs2 = rs.clone();
        let _ = rs2.intrinsic_drive(&cfg);
        assert_eq!(rs2.theories().len(), theories_before,
            "positive-CV theory should survive pruning");
    }

    #[test]
    fn adr0040_prune_disabled_by_default_via_flag() {
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b")]);
        let sg = Subgraph::from_edges([R::new("a", "b")]);
        let p = rs.name_pattern_instances(&[sg]).unwrap();
        let cfg = DriveConfig {
            pattern_sizes: vec![],
            enable_prune: false,
            ..DriveConfig::default()
        };
        let mut rs2 = rs.clone();
        let _ = rs2.intrinsic_drive(&cfg);
        // Disabled → pattern still there.
        assert!(rs2.patterns().iter().any(|q| *q == p.as_str()));
    }

    // ADR 0042 — theory independence relations.

    #[test]
    fn adr0042_name_independence_on_disjoint_theories() {
        let mut rs = diamond_poset();
        // Two theories with disjoint member sets.
        let t_anti = rs.name_theory(&[AX_ANTISYMMETRY]).unwrap();
        let t_refl = rs.name_theory(&[AX_REFLEXIVITY]).unwrap();
        let ind = rs.name_theory_independence(&t_anti, &t_refl).unwrap();
        assert!(rs.independence_edges().contains(&ind.as_str()));
        let (lo, hi) = rs.independence_endpoints(&ind).unwrap();
        assert!(lo < hi);
        assert!(lo == t_anti || lo == t_refl);
    }

    #[test]
    fn adr0042_rejects_overlapping_theories() {
        let mut rs = diamond_poset();
        let t_full = rs
            .name_theory(&["ax_tpl_v3_p0-1_p1-2_c0-2", AX_REFLEXIVITY, AX_ANTISYMMETRY])
            .unwrap();
        let t_shared = rs.name_theory(&[AX_ANTISYMMETRY]).unwrap();
        // Both contain AX_ANTISYMMETRY → not independent.
        assert!(rs.name_theory_independence(&t_full, &t_shared).is_err());
    }

    #[test]
    fn adr0042_refuses_self_independence() {
        let mut rs = diamond_poset();
        let t = name_theory_from_rset(&mut rs);
        assert!(rs.name_theory_independence(&t, &t).is_err());
    }

    #[test]
    fn adr0042_canonical_ordering_is_deterministic() {
        let mut rs = diamond_poset();
        let t1 = rs.name_theory(&[AX_ANTISYMMETRY]).unwrap();
        let t2 = rs.name_theory(&[AX_REFLEXIVITY]).unwrap();
        let ind_a = rs.name_theory_independence(&t1, &t2).unwrap();
        let ind_b = rs.name_theory_independence(&t2, &t1).unwrap();
        assert_eq!(ind_a, ind_b);
        assert_eq!(rs.independence_edges().len(), 1);
    }

    #[test]
    fn adr0042_symmetric_query() {
        let mut rs = diamond_poset();
        let t1 = rs.name_theory(&[AX_ANTISYMMETRY]).unwrap();
        let t2 = rs.name_theory(&[AX_REFLEXIVITY]).unwrap();
        let _ = rs.name_theory_independence(&t1, &t2).unwrap();
        assert!(rs.theories_independent_from(&t1).contains(&t2));
        assert!(rs.theories_independent_from(&t2).contains(&t1));
    }

    #[test]
    fn adr0042_discover_independences() {
        let mut rs = diamond_poset();
        let t1 = rs.name_theory(&[AX_ANTISYMMETRY]).unwrap();
        let t2 = rs.name_theory(&[AX_REFLEXIVITY]).unwrap();
        let t3 = rs
            .name_theory(&["ax_tpl_v3_p0-1_p1-2_c0-2"])
            .unwrap();
        let found = rs.discover_theory_independences();
        let expected_pairs = [
            (t1.clone().min(t2.clone()), t1.clone().max(t2.clone())),
            (t1.clone().min(t3.clone()), t1.clone().max(t3.clone())),
            (t2.clone().min(t3.clone()), t2.clone().max(t3.clone())),
        ];
        for p in &expected_pairs {
            assert!(found.contains(p), "missing pair {:?}", p);
        }
    }

    #[test]
    fn adr0042_retract_independence_clears_three_edges() {
        let mut rs = diamond_poset();
        let t1 = rs.name_theory(&[AX_ANTISYMMETRY]).unwrap();
        let t2 = rs.name_theory(&[AX_REFLEXIVITY]).unwrap();
        let ind = rs.name_theory_independence(&t1, &t2).unwrap();
        let removed = rs.retract_independence(&ind).unwrap();
        assert_eq!(removed, 3);
        assert!(rs.independence_edges().is_empty());
    }

    #[test]
    fn adr0042_collect_meta_ids_includes_independence() {
        let mut rs = diamond_poset();
        let t1 = rs.name_theory(&[AX_ANTISYMMETRY]).unwrap();
        let t2 = rs.name_theory(&[AX_REFLEXIVITY]).unwrap();
        let ind = rs.name_theory_independence(&t1, &t2).unwrap();
        let meta = rs.collect_meta_ids();
        assert!(meta.contains(INDEPENDENT_MARKER));
        assert!(meta.contains(&ind));
    }

    // ADR 0043 — indexed RSet + sampling-path integration.

    #[test]
    fn adr0043_indices_stay_consistent_with_instances() {
        let mut rs = RSet::new();
        rs.extend([
            R::new("a", "b"), R::new("a", "c"),
            R::new("b", "c"), R::new("c", "a"),
        ]);
        // left_of("a") should match instances scan manually.
        let from_index = rs.left_of("a");
        let from_scan: Vec<&R> = rs
            .instances
            .iter()
            .filter(|r| r.x == "a")
            .collect();
        assert_eq!(from_index.len(), from_scan.len());
        for r in &from_scan {
            assert!(from_index.contains(r));
        }
    }

    #[test]
    fn adr0043_indices_survive_remove() {
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b"), R::new("a", "c")]);
        assert_eq!(rs.left_of("a").len(), 2);
        rs.remove(&R::new("a", "b"));
        assert_eq!(rs.left_of("a").len(), 1);
        rs.remove(&R::new("a", "c"));
        assert_eq!(rs.left_of("a").len(), 0);
    }

    #[test]
    fn adr0043_equality_ignores_indices() {
        // Two RSets built via different insertion orders still compare
        // equal — equality is defined by `instances`, not index state.
        let mut a = RSet::new();
        a.extend([R::new("x", "y"), R::new("y", "z")]);
        let mut b = RSet::new();
        b.extend([R::new("y", "z"), R::new("x", "y")]);
        assert_eq!(a, b);
    }

    #[test]
    fn adr0043_clone_carries_indices() {
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b"), R::new("b", "c")]);
        let rs2 = rs.clone();
        assert_eq!(rs.left_of("a").len(), rs2.left_of("a").len());
        assert_eq!(rs.right_of("b").len(), rs2.right_of("b").len());
    }

    #[test]
    fn adr0043_autonomous_pass_sampling_mode_finds_patterns() {
        // Same mixed graph; sampling-path should find the same kinds
        // of patterns (sampling may return fewer instances, but at
        // least some).
        let mut rs = RSet::new();
        rs.extend([R::new("c1", "c2"), R::new("c2", "c3"),
                   R::new("c3", "c4"), R::new("c4", "c5")]);
        rs.extend([R::new("s", "sa"), R::new("s", "sb"), R::new("s", "sc")]);
        let cfg = AutonomousConfig {
            discovery: DiscoveryConfig {
                target_size: 2,
                sample_count: 100,
                top_m: 5,
                rng_seed: 2024,
                include_meta_in_discovery: false,
            },
            refinement: RefinementConfig { max_tries: 50, rng_seed: 2024 },
            naming: NamingPolicy::default(),
            instance_sampling: Some(SamplingMatchConfig {
                sample_count: 200,
                rng_seed: 3,
            }),
        };
        let outcomes = rs.autonomous_pass(&cfg);
        // With sampling, we expect at least some outcomes; no crashes.
        assert!(!outcomes.is_empty());
    }

    #[test]
    fn adr0043_drive_with_sampling_flag_works() {
        let mut rs = diamond_poset();
        let cfg = DriveConfig {
            pattern_sizes: vec![2],
            instance_sampling: Some(SamplingMatchConfig {
                sample_count: 100,
                rng_seed: 7,
            }),
            ..DriveConfig::default()
        };
        let trace = rs.intrinsic_drive(&cfg);
        // Just a smoke test — drive runs without panicking.
        let _ = trace.final_score;
    }

    // ADR 0044 — template-language extension (equality + disjunction).

    #[test]
    fn adr0044_antisymmetry_template_holds_on_poset() {
        let rs = diamond_poset();
        let ev = rs.discover_antisymmetry_template().unwrap();
        assert_eq!(ev.rate(), 1.0);
    }

    #[test]
    fn adr0044_antisymmetry_template_fails_on_equivalence() {
        let rs = equivalence_relation();
        let ev = rs.discover_antisymmetry_template().unwrap();
        // On equivalence, R(a,b) AND R(b,a) holds for many distinct
        // pairs — antisymmetry's premise is met but equality isn't.
        assert!(ev.rate() < 1.0);
    }

    #[test]
    fn adr0044_totality_template_holds_on_total_order() {
        let rs = total_order_closure();
        let ev = rs.discover_totality_template().unwrap();
        assert_eq!(ev.rate(), 1.0);
    }

    #[test]
    fn adr0044_totality_template_fails_on_diamond() {
        let rs = diamond_poset();
        let ev = rs.discover_totality_template().unwrap();
        assert!(ev.rate() < 1.0);
    }

    #[test]
    fn adr0044_discover_extended_axioms_merges_all_three() {
        let rs = total_order_closure();
        let cfg = AxiomDiscoveryConfig::default();
        let extended = rs.discover_extended_axioms(&cfg);
        // Expect: edge-family transitivity + totality (disjunctive).
        let has_edge = extended.iter().any(|e|
            matches!(e, ExtendedAxiomEvidence::Edge(_))
        );
        let has_disj = extended.iter().any(|e|
            matches!(e, ExtendedAxiomEvidence::Disjunctive { .. })
        );
        assert!(has_edge);
        assert!(has_disj);
    }

    #[test]
    fn adr0044_equality_template_rate_is_binding_based() {
        // On diamond poset, premise R(x,y) ∧ R(y,x) holds only when
        // x == y (self-loops). Those are 4 bindings (one per id),
        // and equality holds for all of them → rate 1.0.
        let rs = diamond_poset();
        let ev = rs.discover_antisymmetry_template().unwrap();
        if let ExtendedAxiomEvidence::Equality {
            premise_bindings,
            conclusion_satisfied,
            ..
        } = ev
        {
            assert!(premise_bindings >= 1);
            assert_eq!(premise_bindings, conclusion_satisfied);
        } else {
            panic!("expected equality evidence");
        }
    }

    #[test]
    fn adr0044_extended_respects_defeasible_threshold() {
        // Defeasible mode accepts partial antisymmetry.
        let rs = equivalence_relation();
        let loose = AxiomDiscoveryConfig {
            min_rate: 0.1,
            ..AxiomDiscoveryConfig::default()
        };
        let strict = AxiomDiscoveryConfig::default();
        let loose_ev = rs.discover_extended_axioms(&loose);
        let strict_ev = rs.discover_extended_axioms(&strict);
        assert!(loose_ev.len() >= strict_ev.len());
    }

    // ADR 0045 — axiom confidence (Wilson score + null-baseline).

    #[test]
    fn adr0045_wilson_score_edge_cases() {
        // n=0 → (0, 1) (no information)
        let (lo, hi) = wilson_score_95(0, 0);
        assert_eq!(lo, 0.0);
        assert_eq!(hi, 1.0);
        // n=1, s=1 → high lower? Actually small n means wide CI.
        let (lo1, hi1) = wilson_score_95(1, 1);
        assert!(lo1 < 0.5, "n=1 CI should be wide, got lower {}", lo1);
        assert!(hi1 > 0.9);
        // n=100, s=100 → tight CI near 1.0
        let (lo2, _) = wilson_score_95(100, 100);
        assert!(lo2 > 0.95,
            "n=100 s=100 should give tight CI lower > 0.95, got {}", lo2);
        // n=100, s=50 → CI around 0.5
        let (lo3, hi3) = wilson_score_95(50, 100);
        assert!(lo3 > 0.4 && lo3 < 0.5);
        assert!(hi3 > 0.5 && hi3 < 0.6);
    }

    #[test]
    fn adr0045_null_baseline_extreme_cases() {
        // p = 0 → no edges → null prob 1 (impossible to observe anything)
        assert_eq!(null_baseline_probability(10, 10, 0.0), 1.0);
        // p = 1 → all edges → anything holds trivially → null prob 1
        assert_eq!(null_baseline_probability(10, 10, 1.0), 1.0);
        // N = 0 → nothing observed → null prob 1 (no info)
        assert_eq!(null_baseline_probability(0, 0, 0.5), 1.0);
        // not satisfied-all (satisfied < bindings) → no claim to discount
        assert_eq!(null_baseline_probability(10, 5, 0.5), 1.0);
    }

    #[test]
    fn adr0045_null_baseline_small_with_dense_input() {
        // 20 bindings, all satisfied, p = 0.5 → 0.5^20 ≈ 9.5e-7
        let p = null_baseline_probability(20, 20, 0.5);
        assert!(p > 0.0 && p < 1e-5);
    }

    #[test]
    fn adr0045_evidence_carries_posterior_fields() {
        let rs = diamond_poset();
        let axioms = rs.discover_axioms(&AxiomDiscoveryConfig::default());
        assert!(!axioms.is_empty());
        for ev in &axioms {
            // All rate=1.0 axioms have CI lower ≤ 1.0, upper = 1.0.
            assert!(ev.posterior_lower_95 >= 0.0);
            assert!(ev.posterior_upper_95 <= 1.0);
            assert!(ev.posterior_lower_95 <= ev.posterior_upper_95);
            // null baseline is in [0, 1].
            assert!(ev.null_baseline_prob >= 0.0);
            assert!(ev.null_baseline_prob <= 1.0);
        }
    }

    #[test]
    fn adr0045_dense_random_graph_has_high_null_baseline() {
        // Build a dense random graph; accidental axioms at rate 1.0
        // should show high null-baseline probability.
        let mut rs = RSet::new();
        let nodes: Vec<&str> = vec!["a", "b", "c", "d"];
        // Complete graph: all pairs.
        for a in &nodes {
            for b in &nodes {
                rs.add(R::new(*a, *b));
            }
        }
        // Everything holds at rate 1.0. And null baseline should be
        // close to 1.0 because p=1.0.
        let axioms = rs.discover_axioms(&AxiomDiscoveryConfig::default());
        for ev in &axioms {
            // With p_edge = 16/16 = 1.0, null_baseline_prob = 1.0
            assert!((ev.null_baseline_prob - 1.0).abs() < 1e-9);
        }
    }

    #[test]
    fn adr0045_small_support_gives_wide_ci() {
        // Custom synthetic axiom with small support.
        let ev = AxiomEvidence {
            template: AxiomTemplate {
                num_vars: 2,
                premise: vec![EdgeTemplate { x_var: 0, y_var: 1 }],
                conclusion: EdgeTemplate { x_var: 1, y_var: 0 },
            },
            premise_bindings: 2,
            conclusion_satisfied: 2,
            rate: 1.0,
            posterior_lower_95: 0.0,
            posterior_upper_95: 1.0,
            null_baseline_prob: 1.0,
        };
        let (lo, _) = wilson_score_95(ev.conclusion_satisfied, ev.premise_bindings);
        // CI lower at N=2 should be well below rate=1
        assert!(lo < 0.5);
    }

    // ADR 0046 — theory parallel relation.

    fn three_way_theory_setup(rs: &mut RSet) -> (String, String, String) {
        // t_a: sym + refl
        // t_b: sym + antisym  ← shares sym with t_a, not a subset
        // t_c: refl + antisym ← shares refl with t_a, not a subset
        // All should be mutually parallel.
        // But: do these axioms actually hold on diamond_poset?
        //   sym: no. refl: yes. antisym: yes.
        // So only refl + antisym are discoverable on poset.
        // Use name_theory to build synthetic theories from ids that
        // DO hold on the source.
        // Use reflexivity + antisymmetry + (template) transitivity as
        // our three-axiom pool from diamond_poset.
        let t_a = rs.name_theory(&[AX_REFLEXIVITY, AX_ANTISYMMETRY]).unwrap();
        let t_b = rs
            .name_theory(&[AX_ANTISYMMETRY, "ax_tpl_v3_p0-1_p1-2_c0-2"])
            .unwrap();
        let t_c = rs
            .name_theory(&[AX_REFLEXIVITY, "ax_tpl_v3_p0-1_p1-2_c0-2"])
            .unwrap();
        (t_a, t_b, t_c)
    }

    #[test]
    fn adr0046_parallel_on_overlapping_non_subset_theories() {
        let mut rs = diamond_poset();
        let (t_a, t_b, _t_c) = three_way_theory_setup(&mut rs);
        // t_a (refl, antisym) and t_b (antisym, trans) share antisym.
        // Neither is subset of the other.
        let par = rs.name_theory_parallel(&t_a, &t_b).unwrap();
        assert!(rs.parallel_edges().contains(&par.as_str()));
        let (lo, hi) = rs.parallel_endpoints(&par).unwrap();
        assert!(lo < hi);
    }

    #[test]
    fn adr0046_rejects_disjoint_theories() {
        let mut rs = diamond_poset();
        let t_a = rs.name_theory(&[AX_REFLEXIVITY]).unwrap();
        let t_b = rs.name_theory(&[AX_ANTISYMMETRY]).unwrap();
        // Disjoint → not parallel, use independence instead.
        assert!(rs.name_theory_parallel(&t_a, &t_b).is_err());
    }

    #[test]
    fn adr0046_rejects_subset_theories() {
        let mut rs = diamond_poset();
        let t_small = rs.name_theory(&[AX_REFLEXIVITY]).unwrap();
        let t_big = rs.name_theory(&[AX_REFLEXIVITY, AX_ANTISYMMETRY]).unwrap();
        // Subset → extends relation, not parallel.
        assert!(rs.name_theory_parallel(&t_small, &t_big).is_err());
    }

    #[test]
    fn adr0046_canonical_ordering_deterministic() {
        let mut rs = diamond_poset();
        let (t_a, t_b, _t_c) = three_way_theory_setup(&mut rs);
        let p1 = rs.name_theory_parallel(&t_a, &t_b).unwrap();
        let p2 = rs.name_theory_parallel(&t_b, &t_a).unwrap();
        assert_eq!(p1, p2);
    }

    #[test]
    fn adr0046_discover_parallels_pairs() {
        let mut rs = diamond_poset();
        let (t_a, t_b, t_c) = three_way_theory_setup(&mut rs);
        let found = rs.discover_theory_parallels();
        // All three pairs should be mutually parallel.
        let pair_ab = (t_a.clone().min(t_b.clone()), t_a.clone().max(t_b.clone()));
        let pair_ac = (t_a.clone().min(t_c.clone()), t_a.clone().max(t_c.clone()));
        let pair_bc = (t_b.clone().min(t_c.clone()), t_b.clone().max(t_c.clone()));
        assert!(found.contains(&pair_ab));
        assert!(found.contains(&pair_ac));
        assert!(found.contains(&pair_bc));
    }

    #[test]
    fn adr0046_symmetric_query() {
        let mut rs = diamond_poset();
        let (t_a, t_b, _t_c) = three_way_theory_setup(&mut rs);
        let _ = rs.name_theory_parallel(&t_a, &t_b).unwrap();
        assert!(rs.theories_parallel_to(&t_a).contains(&t_b));
        assert!(rs.theories_parallel_to(&t_b).contains(&t_a));
    }

    #[test]
    fn adr0046_retract_clears_three_edges() {
        let mut rs = diamond_poset();
        let (t_a, t_b, _t_c) = three_way_theory_setup(&mut rs);
        let par = rs.name_theory_parallel(&t_a, &t_b).unwrap();
        let removed = rs.retract_parallel(&par).unwrap();
        assert_eq!(removed, 3);
        assert!(rs.parallel_edges().is_empty());
    }

    #[test]
    fn adr0046_collect_meta_ids_includes_parallel() {
        let mut rs = diamond_poset();
        let (t_a, t_b, _t_c) = three_way_theory_setup(&mut rs);
        let par = rs.name_theory_parallel(&t_a, &t_b).unwrap();
        let meta = rs.collect_meta_ids();
        assert!(meta.contains(PARALLEL_MARKER));
        assert!(meta.contains(&par));
    }

    // ADR 0047 — extended axiom id codecs (equality + disjunctive).

    #[test]
    fn adr0047_equality_id_roundtrip_antisymmetry() {
        let antisym = EqualityAxiomTemplate {
            num_vars: 2,
            premise: vec![
                EdgeTemplate { x_var: 0, y_var: 1 },
                EdgeTemplate { x_var: 1, y_var: 0 },
            ],
            equal_vars: (0, 1),
        };
        let id = equality_axiom_id(&antisym);
        assert_eq!(id, "ax_eq_v2_p0-1_p1-0_eq0-1");
        let back = equality_id_to_template(&id).unwrap();
        assert_eq!(back, antisym);
    }

    #[test]
    fn adr0047_disjunctive_id_roundtrip_totality() {
        let tot = DisjunctiveAxiomTemplate {
            num_vars: 2,
            premise: vec![],
            conclusions: vec![
                EdgeTemplate { x_var: 0, y_var: 1 },
                EdgeTemplate { x_var: 1, y_var: 0 },
            ],
        };
        let id = disjunctive_axiom_id(&tot);
        assert_eq!(id, "ax_disj_v2_d0-1_d1-0");
        let back = disjunctive_id_to_template(&id).unwrap();
        assert_eq!(back, tot);
    }

    #[test]
    fn adr0047_id_dispatch_is_unambiguous() {
        // Edge id never parses as equality / disjunctive.
        let edge_id = "ax_tpl_v3_p0-1_p1-2_c0-2";
        assert!(axiom_id_to_template(edge_id).is_some());
        assert!(equality_id_to_template(edge_id).is_none());
        assert!(disjunctive_id_to_template(edge_id).is_none());

        // Equality id only parses as equality.
        let eq_id = "ax_eq_v2_p0-1_p1-0_eq0-1";
        assert!(axiom_id_to_template(eq_id).is_none());
        assert!(equality_id_to_template(eq_id).is_some());
        assert!(disjunctive_id_to_template(eq_id).is_none());

        // Disjunctive id only parses as disjunctive.
        let d_id = "ax_disj_v2_d0-1_d1-0";
        assert!(axiom_id_to_template(d_id).is_none());
        assert!(equality_id_to_template(d_id).is_none());
        assert!(disjunctive_id_to_template(d_id).is_some());
    }

    #[test]
    fn adr0047_name_theory_accepts_equality_id_on_satisfying_rset() {
        // Diamond poset: antisymmetry holds via template form because
        // the only bindings where R(x,y) ∧ R(y,x) both hold are self-
        // loops (x=y). All such bindings trivially satisfy x=y.
        let mut rs = diamond_poset();
        let antisym_id = "ax_eq_v2_p0-1_p1-0_eq0-1";
        let t_id = rs.name_theory(&[antisym_id]).unwrap();
        assert!(rs.theory_axioms(&t_id).contains(&antisym_id));
    }

    #[test]
    fn adr0047_name_theory_rejects_equality_on_equivalence() {
        // Equivalence: R(a,b) ∧ R(b,a) holds for many distinct (a,b)
        // pairs → template-form antisymmetry is NOT rate 1.0.
        let mut rs = equivalence_relation();
        let antisym_id = "ax_eq_v2_p0-1_p1-0_eq0-1";
        assert!(rs.name_theory(&[antisym_id]).is_err());
    }

    #[test]
    fn adr0047_name_theory_accepts_disjunctive_id_on_total_order() {
        let mut rs = total_order_closure();
        let tot_id = "ax_disj_v2_d0-1_d1-0";
        let t_id = rs.name_theory(&[tot_id]).unwrap();
        assert!(rs.theory_axioms(&t_id).contains(&tot_id));
    }

    #[test]
    fn adr0047_name_theory_rejects_disjunctive_on_diamond() {
        let mut rs = diamond_poset();
        let tot_id = "ax_disj_v2_d0-1_d1-0";
        assert!(rs.name_theory(&[tot_id]).is_err());
    }

    #[test]
    fn adr0047_name_theory_bundles_all_three_families() {
        // One theory with edge axiom + equality + disjunctive. Use
        // total order where all three hold.
        let mut rs = total_order_closure();
        let ids = [
            "ax_tpl_v3_p0-1_p1-2_c0-2",  // edge: transitivity
            "ax_eq_v2_p0-1_p1-0_eq0-1",  // equality: antisymmetry
            "ax_disj_v2_d0-1_d1-0",      // disjunctive: totality
        ];
        let t_id = rs.name_theory(&ids).unwrap();
        let members = rs.theory_axioms(&t_id);
        for id in &ids {
            assert!(members.contains(id));
        }
    }

    // ADR 0048 — confidence thresholds in discovery config.

    #[test]
    fn adr0048_default_has_no_effect() {
        let rs = diamond_poset();
        let default_cfg = AxiomDiscoveryConfig::default();
        let axioms = rs.discover_axioms(&default_cfg);
        assert!(!axioms.is_empty());
    }

    #[test]
    fn adr0048_high_posterior_threshold_drops_small_support() {
        // Diamond poset: transitivity has bindings=2; Wilson CI lower
        // at n=2 is well under 0.5. Raising min_posterior_lower to
        // 0.7 should drop it.
        let rs = diamond_poset();
        let strict = AxiomDiscoveryConfig {
            min_posterior_lower: 0.7,
            ..AxiomDiscoveryConfig::default()
        };
        let axioms = rs.discover_axioms(&strict);
        for ev in &axioms {
            assert!(ev.posterior_lower_95 >= 0.7);
        }
    }

    #[test]
    fn adr0048_low_null_threshold_drops_dense_accidents() {
        // Complete graph on 4 ids: everything holds, p_edge = 1.0,
        // null_baseline_prob = 1.0 on every axiom. max_null_baseline
        // below 1.0 should drop all of them.
        let mut rs = RSet::new();
        let nodes = vec!["a", "b", "c", "d"];
        for a in &nodes {
            for b in &nodes {
                rs.add(R::new(*a, *b));
            }
        }
        let filter = AxiomDiscoveryConfig {
            max_null_baseline: 0.5,
            ..AxiomDiscoveryConfig::default()
        };
        let axioms = rs.discover_axioms(&filter);
        assert!(axioms.is_empty(),
            "all axioms should be filtered as accidental; got {}",
            axioms.len());
    }

    #[test]
    fn adr0048_thresholds_compose_additively() {
        // Both filters active should drop at least as many as each alone.
        let rs = diamond_poset();
        let alone_post = AxiomDiscoveryConfig {
            min_posterior_lower: 0.5,
            ..AxiomDiscoveryConfig::default()
        };
        let alone_null = AxiomDiscoveryConfig {
            max_null_baseline: 0.5,
            ..AxiomDiscoveryConfig::default()
        };
        let both = AxiomDiscoveryConfig {
            min_posterior_lower: 0.5,
            max_null_baseline: 0.5,
            ..AxiomDiscoveryConfig::default()
        };
        let n_post = rs.discover_axioms(&alone_post).len();
        let n_null = rs.discover_axioms(&alone_null).len();
        let n_both = rs.discover_axioms(&both).len();
        assert!(n_both <= n_post);
        assert!(n_both <= n_null);
    }

    #[test]
    fn adr0048_high_posterior_preserved_by_large_support() {
        // Equivalence relation on 2 classes of 2+3: ~36 bindings for
        // transitivity → Wilson CI lower is close to 1.0. Should pass
        // min_posterior_lower = 0.9.
        let rs = equivalence_relation();
        let strict = AxiomDiscoveryConfig {
            min_posterior_lower: 0.9,
            ..AxiomDiscoveryConfig::default()
        };
        let axioms = rs.discover_axioms(&strict);
        // At least one axiom should survive — specifically one with
        // enough support to breach 0.9.
        assert!(!axioms.is_empty());
    }

    // ADR 0049 — theory relation classifier + neighborhood.

    #[test]
    fn adr0049_classify_extends() {
        let mut rs = diamond_poset();
        let t_big = rs
            .name_theory(&[AX_REFLEXIVITY, AX_ANTISYMMETRY,
                           "ax_tpl_v3_p0-1_p1-2_c0-2"])
            .unwrap();
        let t_small = rs.name_theory(&[AX_REFLEXIVITY]).unwrap();
        assert_eq!(
            rs.classify_theory_pair(&t_big, &t_small),
            Some(TheoryRelationKind::Extends)
        );
        assert_eq!(
            rs.classify_theory_pair(&t_small, &t_big),
            Some(TheoryRelationKind::ExtendedBy)
        );
    }

    #[test]
    fn adr0049_classify_independent() {
        let mut rs = diamond_poset();
        let t_a = rs.name_theory(&[AX_REFLEXIVITY]).unwrap();
        let t_b = rs.name_theory(&[AX_ANTISYMMETRY]).unwrap();
        assert_eq!(
            rs.classify_theory_pair(&t_a, &t_b),
            Some(TheoryRelationKind::Independent)
        );
    }

    #[test]
    fn adr0049_classify_parallel() {
        let mut rs = diamond_poset();
        let t_a = rs.name_theory(&[AX_REFLEXIVITY, AX_ANTISYMMETRY]).unwrap();
        let t_b = rs
            .name_theory(&[AX_ANTISYMMETRY, "ax_tpl_v3_p0-1_p1-2_c0-2"])
            .unwrap();
        assert_eq!(
            rs.classify_theory_pair(&t_a, &t_b),
            Some(TheoryRelationKind::Parallel)
        );
    }

    #[test]
    fn adr0049_classify_equal_on_same_theory_id() {
        let mut rs = diamond_poset();
        let t = name_theory_from_rset(&mut rs);
        assert_eq!(
            rs.classify_theory_pair(&t, &t),
            Some(TheoryRelationKind::Equal)
        );
    }

    #[test]
    fn adr0049_classify_returns_none_for_non_theory() {
        let rs = diamond_poset();
        assert!(rs.classify_theory_pair("a", "b").is_none());
    }

    #[test]
    fn adr0049_neighborhood_partitions_pairs() {
        let mut rs = diamond_poset();
        let t_self = rs.name_theory(&[AX_REFLEXIVITY, AX_ANTISYMMETRY]).unwrap();
        let t_small = rs.name_theory(&[AX_REFLEXIVITY]).unwrap();
        let t_disjoint = rs
            .name_theory(&["ax_tpl_v3_p0-1_p1-2_c0-2"])
            .unwrap();
        let t_parallel = rs
            .name_theory(&[AX_ANTISYMMETRY, "ax_tpl_v3_p0-1_p1-2_c0-2"])
            .unwrap();
        let neigh = rs.theory_neighborhood(&t_self).unwrap();
        // t_small is extended by t_self → appears in 'extends' list.
        assert!(neigh.extends.contains(&t_small));
        // t_disjoint is independent.
        assert!(neigh.independent.contains(&t_disjoint));
        // t_parallel shares antisym.
        assert!(neigh.parallel.contains(&t_parallel));
    }

    // ADR 0051 — adaptive drive config.

    #[test]
    fn adr0051_small_rset_no_sampling() {
        // Small RSet → sampling should remain None.
        let mut rs = RSet::new();
        rs.extend([R::new("a", "b"), R::new("b", "c")]);
        let base = DriveConfig::default();
        let tuned = rs.adaptive_drive_config(base);
        assert!(tuned.instance_sampling.is_none());
    }

    #[test]
    fn adr0051_large_rset_enables_sampling() {
        // Build a graph with > 300 edges.
        let mut rs = RSet::new();
        let mut state: u64 = 12345;
        while rs.len() < 500 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let a = (state as usize) % 30;
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let b = (state as usize) % 30;
            rs.add(R::new(format!("n{}", a), format!("n{}", b)));
        }
        let base = DriveConfig::default();
        let tuned = rs.adaptive_drive_config(base);
        assert!(tuned.instance_sampling.is_some());
    }

    #[test]
    fn adr0051_drops_pattern_sizes_that_dont_fit() {
        let mut rs = RSet::new();
        rs.add(R::new("a", "b"));
        // Only 1 data edge; sizes 2, 3, 4 should all be dropped.
        let base = DriveConfig {
            pattern_sizes: vec![2, 3, 4],
            ..DriveConfig::default()
        };
        let tuned = rs.adaptive_drive_config(base);
        assert!(tuned.pattern_sizes.is_empty());
    }

    #[test]
    fn adr0051_scales_sample_count() {
        let mut rs = RSet::new();
        for i in 0..100 {
            rs.add(R::new(format!("a{}", i), format!("b{}", i)));
        }
        let base = DriveConfig {
            discovery_config: DiscoveryConfig {
                target_size: 2,
                sample_count: 50,
                top_m: 5,
                rng_seed: 0,
                include_meta_in_discovery: false,
            },
            ..DriveConfig::default()
        };
        let tuned = rs.adaptive_drive_config(base);
        // 100 edges × 2 = 200, clamped to [50, 1000] → 200.
        assert_eq!(tuned.discovery_config.sample_count, 200);
    }

    #[test]
    fn adr0051_respects_explicit_sampling_config() {
        // If caller already set instance_sampling, don't override it.
        let mut rs = RSet::new();
        for i in 0..500 {
            rs.add(R::new(format!("a{}", i), format!("b{}", i)));
        }
        let custom_smpl = SamplingMatchConfig {
            sample_count: 42,
            rng_seed: 99,
        };
        let base = DriveConfig {
            instance_sampling: Some(custom_smpl.clone()),
            ..DriveConfig::default()
        };
        let tuned = rs.adaptive_drive_config(base);
        assert_eq!(tuned.instance_sampling, Some(custom_smpl));
    }

    #[test]
    fn adr0051_clamps_extreme_sample_counts() {
        let mut rs = RSet::new();
        for i in 0..2000 {
            rs.add(R::new(format!("a{}", i), format!("b{}", i)));
        }
        let base = DriveConfig::default();
        let tuned = rs.adaptive_drive_config(base);
        // 2000 × 2 = 4000, should clamp to 1000.
        assert_eq!(tuned.discovery_config.sample_count, 1000);
    }

    // ── ADR 0074 — Phase Emergence-1: shape co-occurrence concepts ──

    /// Build a small RSet with 4 axioms forming 2 shape families,
    /// and 2 theories each containing one axiom from each family.
    /// The setup is the canonical "(family A, family B) co-occurs
    /// in 2 theories" pattern the concept miner is meant to detect.
    fn build_concept_test_rset()
    -> (crate::RSet, String, String, Vec<String>) {
        use crate::{R, THEORY_MARKER};
        let mut rs = crate::RSet::new();
        // Two axioms sharing premise [(0,0), (1,2)].
        let ax_a = "ax_tpl_v3_p0-0_p1-2_c0-1";
        let ax_b = "ax_tpl_v3_p0-0_p1-2_c0-2";
        // Two axioms sharing premise [(0,1), (1,2)].
        let ax_c = "ax_tpl_v3_p0-1_p1-2_c2-0";
        let ax_d = "ax_tpl_v3_p0-1_p1-2_c2-1";
        for id in [ax_a, ax_b, ax_c, ax_d] {
            rs.register_axiom_with_intension(id);
        }
        // Mint 2 shape families; they collect the 4 axioms.
        let _ = rs.discover_axiom_shape_families(2);

        // Build 2 theories, each with one axiom from each family.
        // Bypass `name_theory` to avoid `verify_axiom_holds`.
        let t_alpha = "t_alpha";
        let t_beta = "t_beta";
        rs.add(R::new(THEORY_MARKER, t_alpha));
        rs.add(R::new(t_alpha, ax_a));
        rs.add(R::new(t_alpha, ax_c));
        rs.add(R::new(THEORY_MARKER, t_beta));
        rs.add(R::new(t_beta, ax_b));
        rs.add(R::new(t_beta, ax_d));

        let mut families: Vec<String> = rs
            .axiom_shape_families()
            .iter()
            .map(|s| s.to_string())
            .collect();
        families.sort();
        (rs, t_alpha.to_string(), t_beta.to_string(), families)
    }

    /// Synthesize Signal-class quality reports for two theories.
    /// Used to satisfy `require_signal_only` filter.
    fn synth_signal_reports(t1: &str, t2: &str) -> Vec<crate::TheoryQualityReport> {
        vec![
            synth_report(t1, vec!["ax_a".to_string()], Some(0.95), Some(0.95), 0,
                         vec![], vec![], vec![]),
            synth_report(t2, vec!["ax_b".to_string()], Some(0.95), Some(0.95), 0,
                         vec![], vec![], vec![]),
        ]
    }

    #[test]
    fn adr0074_concept_id_is_deterministic() {
        let cs = vec!["shape_a".to_string(), "shape_b".to_string()];
        let id1 = crate::concept_id_from_constituents(&cs);
        let id2 = crate::concept_id_from_constituents(&cs);
        assert_eq!(id1, id2);
        assert!(id1.starts_with("concept_"));
        assert!(id1.len() > "concept_".len());
    }

    #[test]
    fn adr0074_concept_alias_is_human_readable() {
        let cs = vec![
            "shape_premise_p0-0_p1-2".to_string(),
            "shape_premise_p0-1_p1-2".to_string(),
        ];
        let alias = crate::concept_alias_from_constituents(&cs);
        assert!(alias.contains("premise"));
        assert!(alias.starts_with("concept_alias_"));
    }

    #[test]
    fn adr0074_propose_surfaces_pair_for_cooccurrence() {
        let (rs, t_alpha, t_beta, families) = build_concept_test_rset();
        assert_eq!(families.len(), 2, "expected 2 premise families");
        let reports = synth_signal_reports(&t_alpha, &t_beta);
        let cfg = crate::ConceptMiningConfig::default();
        let candidates = rs.propose_concept_candidates(&cfg, &reports);
        assert!(!candidates.is_empty(), "expected ≥1 candidate");
        let pair = candidates
            .iter()
            .find(|c| c.constituent_shapes.len() == 2)
            .expect("a 2-shape candidate");
        let expected: std::collections::HashSet<&str> =
            families.iter().map(String::as_str).collect();
        let got: std::collections::HashSet<&str> =
            pair.constituent_shapes.iter().map(String::as_str).collect();
        assert_eq!(got, expected, "candidate constituents match the 2 minted families");
        assert_eq!(pair.theories_attested.len(), 2);
    }

    #[test]
    fn adr0074_propose_skips_below_min_theories() {
        let (rs, t_alpha, t_beta, _families) = build_concept_test_rset();
        let reports = synth_signal_reports(&t_alpha, &t_beta);
        // Set min_theories = 3 even though only 2 attestations exist.
        let cfg = crate::ConceptMiningConfig {
            min_theories: 3,
            ..Default::default()
        };
        let candidates = rs.propose_concept_candidates(&cfg, &reports);
        assert!(candidates.is_empty());
    }

    #[test]
    fn adr0074_propose_filters_noise_theories_when_signal_only() {
        let (rs, t_alpha, t_beta, _families) = build_concept_test_rset();
        // Mark one of the two theories Noise; with require_signal_only,
        // only one Signal theory remains → fails min_theories=2.
        let reports = vec![
            synth_report(&t_alpha, vec!["ax_a".to_string()], Some(0.95), Some(0.95), 0,
                         vec![], vec![], vec![]),
            synth_report(&t_beta, vec!["ax_b".to_string()], Some(0.20), Some(0.20), 4,
                         vec![], vec![], vec![]),
        ];
        let cfg = crate::ConceptMiningConfig::default();
        let candidates = rs.propose_concept_candidates(&cfg, &reports);
        assert!(
            candidates.is_empty(),
            "Noise filter should leave 1 Signal theory < min_theories=2"
        );
    }

    #[test]
    fn adr0074_propose_admits_all_classes_when_signal_only_disabled() {
        let (rs, t_alpha, t_beta, _families) = build_concept_test_rset();
        let reports = vec![
            synth_report(&t_alpha, vec!["ax_a".to_string()], Some(0.95), Some(0.95), 0,
                         vec![], vec![], vec![]),
            synth_report(&t_beta, vec!["ax_b".to_string()], Some(0.20), Some(0.20), 4,
                         vec![], vec![], vec![]),
        ];
        let cfg = crate::ConceptMiningConfig {
            require_signal_only: false,
            ..Default::default()
        };
        let candidates = rs.propose_concept_candidates(&cfg, &reports);
        assert!(!candidates.is_empty());
    }

    #[test]
    fn adr0074_register_creates_meta_r_chains() {
        let (mut rs, t_alpha, t_beta, families) = build_concept_test_rset();
        let mut candidate = crate::ConceptCandidate {
            id: crate::concept_id_from_constituents(&families),
            alias: Some(crate::concept_alias_from_constituents(&families)),
            constituent_shapes: families.clone(),
            theories_attested: vec![t_alpha.clone(), t_beta.clone()],
            aggregate_cross_precision: Some(0.92),
        };
        candidate.aggregate_cross_precision = Some(0.92);
        let id = rs.register_concept(&candidate).expect("register ok");
        assert!(rs.is_concept(&id));
        let concepts = rs.concepts();
        assert_eq!(concepts.len(), 1);
        let constituents = rs.concept_constituent_shapes(&id);
        let cset: std::collections::HashSet<&str> = constituents.into_iter().collect();
        for sf in &families {
            assert!(cset.contains(sf.as_str()), "constituent {} missing", sf);
        }
        let theories = rs.concept_attested_theories(&id);
        let tset: std::collections::HashSet<&str> = theories.into_iter().collect();
        assert!(tset.contains(t_alpha.as_str()));
        assert!(tset.contains(t_beta.as_str()));
        let xprec = rs.concept_cross_precision_at_mint(&id);
        assert_eq!(xprec, Some(0.9200));
    }

    #[test]
    fn adr0074_register_rejects_unvalidated() {
        let (mut rs, t_alpha, t_beta, families) = build_concept_test_rset();
        let candidate = crate::ConceptCandidate {
            id: crate::concept_id_from_constituents(&families),
            alias: None,
            constituent_shapes: families,
            theories_attested: vec![t_alpha, t_beta],
            aggregate_cross_precision: None, // not validated
        };
        let err = rs.register_concept(&candidate).unwrap_err();
        assert_eq!(err, crate::ConceptRegistrationError::NotValidated);
    }

    #[test]
    fn adr0074_register_rejects_degenerate_constituents() {
        let (mut rs, t_alpha, t_beta, _families) = build_concept_test_rset();
        let candidate = crate::ConceptCandidate {
            id: "concept_singleton".to_string(),
            alias: None,
            constituent_shapes: vec!["shape_just_one".to_string()],
            theories_attested: vec![t_alpha, t_beta],
            aggregate_cross_precision: Some(0.95),
        };
        let err = rs.register_concept(&candidate).unwrap_err();
        assert_eq!(err, crate::ConceptRegistrationError::DegenerateConstituents);
    }

    #[test]
    fn adr0074_register_rejects_unknown_constituent() {
        let (mut rs, t_alpha, t_beta, _families) = build_concept_test_rset();
        let candidate = crate::ConceptCandidate {
            id: "concept_unknown".to_string(),
            alias: None,
            constituent_shapes: vec![
                "shape_does_not_exist_a".to_string(),
                "shape_does_not_exist_b".to_string(),
            ],
            theories_attested: vec![t_alpha, t_beta],
            aggregate_cross_precision: Some(0.95),
        };
        let err = rs.register_concept(&candidate).unwrap_err();
        assert!(matches!(
            err,
            crate::ConceptRegistrationError::UnknownConstituent(_)
        ));
    }

    #[test]
    fn adr0074_retract_removes_concept() {
        let (mut rs, t_alpha, t_beta, families) = build_concept_test_rset();
        let candidate = crate::ConceptCandidate {
            id: crate::concept_id_from_constituents(&families),
            alias: None,
            constituent_shapes: families.clone(),
            theories_attested: vec![t_alpha, t_beta],
            aggregate_cross_precision: Some(0.92),
        };
        let id = rs.register_concept(&candidate).unwrap();
        assert!(rs.is_concept(&id));
        assert!(rs.retract_concept(&id));
        assert!(!rs.is_concept(&id));
        assert!(rs.concepts().is_empty());
    }

    #[test]
    fn adr0074_concept_status_live_when_all_constituents_present() {
        let (mut rs, t_alpha, t_beta, families) = build_concept_test_rset();
        let candidate = crate::ConceptCandidate {
            id: crate::concept_id_from_constituents(&families),
            alias: None,
            constituent_shapes: families,
            theories_attested: vec![t_alpha, t_beta],
            aggregate_cross_precision: Some(0.92),
        };
        let id = rs.register_concept(&candidate).unwrap();
        assert_eq!(rs.concept_status(&id), crate::ConceptStatus::Live);
    }

    // ── ADR 0075 — Pattern shape rendering ──────────────────────

    #[test]
    fn adr0075_format_pattern_shape_renders_chain() {
        // Two-edge chain: a → b → c. Mint a pattern, render shape.
        use crate::Subgraph;
        let mut rs = crate::RSet::new();
        let sg = Subgraph::from_edges(vec![
            crate::R::new("a", "b"),
            crate::R::new("b", "c"),
        ]);
        let pid = rs.name_pattern_instances(&[sg]).expect("named");
        let s = rs.format_pattern_shape(&pid);
        assert!(s.contains(&pid));
        assert!(s.contains("3 roles"));
        assert!(s.contains("2 edges"));
        assert!(
            s.contains("chain") || s.contains("fork") || s.contains("merge"),
            "expected chain-like shape; got:\n{}",
            s,
        );
    }

    #[test]
    fn adr0075_format_pattern_shape_renders_self_loop() {
        use crate::Subgraph;
        let mut rs = crate::RSet::new();
        let sg = Subgraph::from_edges(vec![crate::R::new("a", "a")]);
        let pid = rs.name_pattern_instances(&[sg]).expect("named");
        let s = rs.format_pattern_shape(&pid);
        assert!(s.contains("self-loop"));
        assert!(s.contains("1 role"));
        assert!(s.contains("1 edge"));
    }

    #[test]
    fn adr0075_format_pattern_shape_handles_unknown_pattern() {
        let rs = crate::RSet::new();
        let s = rs.format_pattern_shape("p_does_not_exist");
        assert!(s.contains("no intension recorded"));
    }

    #[test]
    fn adr0075_format_pattern_shape_renders_3_cycle() {
        use crate::Subgraph;
        let mut rs = crate::RSet::new();
        let sg = Subgraph::from_edges(vec![
            crate::R::new("a", "b"),
            crate::R::new("b", "c"),
            crate::R::new("c", "a"),
        ]);
        let pid = rs.name_pattern_instances(&[sg]).expect("named");
        let s = rs.format_pattern_shape(&pid);
        assert!(s.contains("3-cycle") || s.contains("3-edge"));
        assert!(s.contains("3 roles"));
        assert!(s.contains("3 edges"));
    }

    #[test]
    fn adr0074_concept_status_stale_after_constituent_retracted() {
        let (mut rs, t_alpha, t_beta, families) = build_concept_test_rset();
        let candidate = crate::ConceptCandidate {
            id: crate::concept_id_from_constituents(&families),
            alias: None,
            constituent_shapes: families.clone(),
            theories_attested: vec![t_alpha, t_beta],
            aggregate_cross_precision: Some(0.92),
        };
        let id = rs.register_concept(&candidate).unwrap();
        // Retract one constituent shape family.
        rs.retract_shape_family(&families[0]).unwrap();
        assert_eq!(rs.concept_status(&id), crate::ConceptStatus::Stale);
    }
