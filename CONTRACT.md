# Contract - forkdrift

This contract is the executable spec for forkdrift. Every obligation below is stated so it can fail. The full contract is the first build act of this repo and it extends the stub obligations written here.

## Stub obligations

FD-S1. Two runs that define the same interface in incompatible ways raise an alarm, and an alarm that does not quote both conflicting definitions is a defect the tests must catch.

FD-S2. Two runs whose definitions of a shared interface agree raise no alarm, and a false alarm on agreeing runs is a defect the tests must catch.

FD-S3. Every alarm names the concept that diverged and the runs that diverged on it, and an alarm missing either is rejected rather than reported incomplete.

FD-S4. Forkdrift detects and reports only, and it never edits a run, a branch, or a definition in order to reconcile a divergence it found.

FD-S5. Every fixture and example in this repo is synthetic and built for this repo, and no real trace or session data appears anywhere in the tree.
