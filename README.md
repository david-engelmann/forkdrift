# forkdrift

Forkdrift compares two recorded artifacts that state the same interface and reports where their statements diverge.

## What this is

An artifact states the terms and operations of one interface. The comparison reports four kinds of divergence, a term stated with a different value, an operation stated with a different result type, a name one side excludes and the other defines, and a name only one side defines, and it quotes the lines that disagree. A sweep compares one baseline artifact against several others in turn and attributes every divergence to the pair that raised it. The boundary with its sibling is one sentence. Forkdrift finds the divergence, crosscurrent reconciles it.

## Status

Round 6 landed in PR 13 and is recorded closed here. A source name that carries a line break is refused by the comparison and the sweep before anything prints it, so a name can no longer splice a forged row into a report, and the refusal stands stated under contract obligations FD-33 to FD-35. Round 5 landed in PR 11 and is recorded closed here. It ports forkdrift to a native Rust crate with no change to the contracted behaviour, proven against the Python implementation it replaces over the committed examples, a generated command line corpus, and differential runs over the divergence classes, the value grammar, and the path spellings the read refusals name. The Python implementation is removed in the same slice, and the port stands stated under contract obligations FD-24 to FD-32 with four probe corrections recorded against FD-2, FD-10, FD-11, and FD-19. Round 4 landed in PR 9 and is recorded closed here. The sweep stands stated under contract obligations FD-1 to FD-23, so one baseline artifact compares against each of several others in turn, every comparison exactly the pairwise comparison, and the sweep document attributes every divergence to the pair that raised it. Round 3 landed in PR 7 and is recorded closed here. The grammar of a number stands stated under contract obligations FD-1 to FD-19, so a value compares by exact decimal worth only when it matches the grammar of this format, and a value written with digit underscores compares as text where it once read as a number. Closed rounds 6 of 100. Part of the impatient programming portfolio, in the Fleet Operations arc.

## Licence

MIT. See LICENSE.
