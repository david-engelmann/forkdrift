# forkdrift

It watches concurrent runs and their diffs for semantic divergence, three agents building the same concept three incompatible ways with all tests green, and raises the alarm before the merge finds it.

## What this is

Parallel drift is the failure class unique to swarms and invisible to every per-run tool, because observability flattens runs into spans and nothing compares what two live agents believe about the same concept. Forkdrift reads the fleet's live tapes and diffs and flags divergence as it forms, two agents quietly defining the same interface in incompatible ways. That catch is the most expensive hour in parallel agent work made visible while it can still be fixed cheaply. The boundary with its sibling is one sentence. Forkdrift finds the divergence, crosscurrent reconciles it.

## Status

Seeded in October 2026. The build has not started. Part of the impatient programming portfolio, in the Fleet Operations arc.

## Licence

MIT. See LICENSE.
