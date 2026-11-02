# Contract - forkdrift

This contract is the executable spec for forkdrift. Every obligation below is stated so it can fail. The full contract is the first build act of this repo and it extends the stub obligations written here.

## Stub obligations

FD-S1. Two runs that define the same interface in incompatible ways raise an alarm, and an alarm that does not quote both conflicting definitions is a defect the tests must catch.

FD-S2. Two runs whose definitions of a shared interface agree raise no alarm, and a false alarm on agreeing runs is a defect the tests must catch.

FD-S3. Every alarm names the concept that diverged and the runs that diverged on it, and an alarm missing either is rejected rather than reported incomplete.

FD-S4. Forkdrift detects and reports only, and it never edits a run, a branch, or a definition in order to reconcile a divergence it found.

FD-S5. Every fixture and example in this repo is synthetic and built for this repo, and no real trace or session data appears anywhere in the tree.

## Round 1 obligations

Round 1 defines the artifact format in this contract and builds the reader and the comparison over it. The format is forkdrift's own, so no outside tool behaviour needs probing before the obligations can rest on it. An artifact is a recorded text that one run left behind for one interface, and the comparison runs on two recorded artifacts only and never on a live run.

The format in brief is shown here once, and the obligations below state it as rules that can fail.

```
interface shard-store

replicas = 3
timeout = 30
operation fetch = record
label primary = node-a
excludes = legacy-mode
```

A meaningful line is a line that is neither blank nor a comment, and a comment line is a line whose first character that is not a space is the hash character. The first meaningful line names the interface. Every later meaningful line is a term line, an operation line, a label line, or an excludes line. A term line is a term name, an equals sign, and a value. An operation line is the word operation, an operation name, an equals sign, and a result type. A label line is the word label, a label name, an equals sign, and a value. An excludes line is the word excludes, an equals sign, and a comma separated list of names, and an artifact carries at most one excludes line. Term names, operation names, label names, and interface names are identifiers that start with a letter or an underscore and continue with letters, digits, underscores, dashes, or dots.

FD-1. An artifact parses only when its first meaningful line names exactly one interface and every later meaningful line matches one of the statement forms this contract defines, and a text that breaks that shape is not an artifact and is never compared.

FD-2. A malformed artifact is refused with an error that names the source and the defect, and the refusal covers a missing interface line, a meaningful line that matches no statement form, a duplicate term name or operation name or label name, an empty value, and an artifact that defines a name its own excludes line forbids, and no refusal ever surfaces as a traceback or as a partial report.

FD-3. When both artifacts state the same term and the two values differ after the normalizations named in FD-6, the comparison raises exactly one divergence for that term, and the report quotes the term line from each artifact verbatim and names the source each line came from.

FD-4. When both artifacts state the same operation and the two result types differ after the normalizations named in FD-6, the comparison raises exactly one divergence for that operation, and the report quotes the operation line from each artifact verbatim and names the source each line came from.

FD-5. When one artifact excludes a name and the other artifact defines that name as a term or as an operation, the comparison raises exactly one divergence for that name, and the report quotes the excludes line and the defining line verbatim and names the source each line came from.

FD-6. The only normalizations are the ones named here, namely leading and trailing whitespace is stripped from every line, runs of whitespace inside a value collapse to a single space, and a value that reads as a finite number on both sides compares by numeric worth rather than by its characters, and statement order never matters, and a difference that one of these normalizations removes raises no divergence.

FD-7. Label lines are local annotations that this format declares non-semantic, and a label that is renamed or revalued between the two artifacts raises no divergence, and label names are never dereferenced inside term values or operation result types in this round.

FD-8. A term or an operation stated in only one of the two artifacts raises no divergence in this round unless the other artifact excludes that name, and this limit is a stated limit of the round and is recorded here so a later round can retire it on purpose.

FD-9. Two artifacts that name different interfaces are refused with an error that names both interface names, because the comparison is defined only over two artifacts that describe one and the same interface.

FD-10. The report lists divergences sorted first by interface name and then by the name that diverged and then by divergence kind, and running the comparison twice over the same pair of artifacts produces byte identical reports.

FD-11. The command line tool exits with code 0 when the pair agrees, with code 1 when at least one divergence is found, and with code 2 when either artifact is malformed or the two artifacts name different interfaces or an input file cannot be read, and a refusal prints one error line to the error stream and never a traceback.

FD-12. The exactly one divergence that FD-5 promises holds per excluded name and per direction, and when the other artifact defines the excluded name both as a term and as an operation, the single divergence quotes the excludes line and both defining lines, each with its source named.

FD-13. The lines the report quotes under FD-3, FD-4, and FD-5 are the statement lines after the leading and trailing whitespace strip named in FD-6 has been applied, and the word verbatim in those obligations is read subject to that strip, with all internal spacing preserved exactly.

FD-14. The self-exclusion refusal in FD-2 covers names defined as terms or as operations, and a name carried only as a label is not a definition for that refusal, because FD-7 declares labels non-semantic.
