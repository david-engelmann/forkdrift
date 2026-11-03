# forkdrift

It watches concurrent runs and their diffs for semantic divergence, three agents building the same concept three incompatible ways with all tests green, and raises the alarm before the merge finds it.

## What this is

Parallel drift is the failure class unique to swarms and invisible to every per-run tool, because observability flattens runs into spans and nothing compares what two live agents believe about the same concept. Forkdrift reads the fleet's live tapes and diffs and flags divergence as it forms, two agents quietly defining the same interface in incompatible ways. That catch is the most expensive hour in parallel agent work made visible while it can still be fixed cheaply. The boundary with its sibling is one sentence. Forkdrift finds the divergence, crosscurrent reconciles it.

## Status

Round 5 landed in PR 11 and is recorded closed here. It ports forkdrift to a native Rust crate with no change to the contracted behaviour, proven against the Python implementation it replaces over the committed examples, a generated command line corpus, and differential runs over the divergence classes, the value grammar, and the path spellings the read refusals name. The Python implementation is removed in the same slice, and the port stands stated under contract obligations FD-24 to FD-32 with four probe corrections recorded against FD-2, FD-10, FD-11, and FD-19. Round 4 landed in PR 9 and is recorded closed here. The sweep stands stated under contract obligations FD-1 to FD-23, so one baseline artifact compares against each of several others in turn, every comparison exactly the pairwise comparison, and the sweep document attributes every divergence to the pair that raised it. Round 3 landed in PR 7 and is recorded closed here. The grammar of a number stands stated under contract obligations FD-1 to FD-19, so a value compares by exact decimal worth only when it matches the grammar of this format, and a value written with digit underscores compares as text where it once read as a number. Closed rounds 5 of 100. Part of the impatient programming portfolio, in the Fleet Operations arc.

## Licence

MIT. See LICENSE.
