# forkdrift

Tells whether two recorded artifacts that state the same interface actually diverge, and reconciles the divergences over recorded branch states. Everything works from recorded files, and nothing is executed.

Forkdrift is a command line tool that works over recorded artifacts, the plain text statements of one interface that two runs of a system leave behind. Its compare command reads two artifacts, compares their statements one by one, and prints a report of every divergence it finds, quoting the lines that disagree. A sweep compares one baseline artifact against several others in turn and attributes every divergence to the pair that raised it. The reconcile subcommand takes those divergences as records, one divergence block at a time in the form compare prints, and reconciles the pair over two recorded branch states. Everything forkdrift does works from recorded files, and nothing is executed.

[![CI](https://github.com/david-engelmann/forkdrift/actions/workflows/ci.yml/badge.svg)](https://github.com/david-engelmann/forkdrift/actions/workflows/ci.yml) [![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

```console
$ ./target/debug/forkdrift compare examples/run-a.tape examples/run-b.tape
divergence shard-store store result-type
  left (examples/run-a.tape) operation store = ack
  right (examples/run-b.tape) operation store = receipt
divergence shard-store timeout term-value
  left (examples/run-a.tape) timeout = 30
  right (examples/run-b.tape) timeout = 45
```

## Install

Forkdrift is a Rust crate that builds from source with cargo.

```sh
git clone https://github.com/david-engelmann/forkdrift.git
cd forkdrift
cargo build
```

The binary lands at target/debug/forkdrift, and the test suite runs with `cargo test`.

## Quickstart

From the repository root, with the committed examples.

```sh
./target/debug/forkdrift compare examples/run-a.tape examples/run-b.tape
```

The command prints the divergence report at the top of this page and exits with status 1, because the two artifacts diverge. Two artifacts that agree print nothing and exit with status 0.

## Usage

### compare

`forkdrift compare <left> <right>` reads two recorded artifacts and prints one block for each divergence, naming the interface, the name the divergence stands over, and its kind, and quoting the line each side states. The kinds are term-value, where a term carries a different value, result-type, where an operation states a different result type, excluded-but-defined, where one side excludes a name the other defines, and presence, where one side makes a statement, a term or an operation, that the other side does not make in the same form. A value compares by exact decimal worth only when it matches the number grammar of the format, and a value outside the grammar compares as text. The run at the top of this page is a compare, and the committed examples carry its output beside the inputs as examples/divergences.txt.

### sweep

`forkdrift sweep <baseline> <other>...` compares one baseline artifact against each of several others in turn. Every comparison in a sweep is exactly the pairwise comparison, and the document prints one pair header per pair and attributes every divergence to the pair that raised it.

```console
$ ./target/debug/forkdrift sweep examples/run-a.tape examples/run-b.tape examples/presence-b.tape
pair left (examples/run-a.tape) right (examples/run-b.tape)
divergence shard-store store result-type
  left (examples/run-a.tape) operation store = ack
  right (examples/run-b.tape) operation store = receipt
divergence shard-store timeout term-value
  left (examples/run-a.tape) timeout = 30
  right (examples/run-b.tape) timeout = 45
pair left (examples/run-a.tape) right (examples/presence-b.tape)
divergence shard-store replicas term-value
  left (examples/run-a.tape) replicas = 3
  right (examples/presence-b.tape) replicas = 4
divergence shard-store store presence
  left (examples/run-a.tape) operation store = ack
  right (examples/presence-b.tape) interface shard-store
```

### reconcile

`forkdrift reconcile` does the session work over the detector's records. The divergence record format the detector prints and the branch state format each stand at version 1 under the contract, and the reconcile forms read the detector's records as printed, against two recorded branch states. A record is one divergence block. A sweep document is not a record, because it carries its blocks under pair headers, and the forms refuse it whole, while a block lifted out of it reads as the record it is. The subcommand performs no detection of its own.

The stage form, `forkdrift reconcile stage <left> <right> <record>`, reads the two branch states and one record and prints the cross-apply report, staging each reported name as clean, conflicting, one-sided, or missing across the two states. The handback form, `forkdrift reconcile handback <left> <right> <record>...`, reads the two branch states and a bundle of one or more records and prints the handback, one sized task per divergence in the bundle.

The committed examples carry one session as `examples/branch-a.state`, `examples/branch-b.state`, and `examples/reconcile-divergences.txt`, and the handback it prints stands beside the inputs as `examples/handback.txt`.

```console
$ ./target/debug/forkdrift reconcile handback examples/branch-a.state examples/branch-b.state examples/reconcile-divergences.txt
task shard-store store
authors engineer-a engineer-b
kind result-type
  left (examples/branch-a.state) operation store = ack
  right (examples/branch-b.state) operation store = receipt
size large score 7
factors definitions 2, result-type yes, breaking yes
staging clean 2, conflicting 1, one-sided 1, missing 0
```

Every command reads only the files it is handed. An artifact that cannot be read or does not parse refuses with one error line and exit status 2, and a malformed invocation exits with status 2. A source name that carries a line break is refused before anything prints it, so a name cannot splice a forged row into a report. A value whose exponent runs past the conversion limit refuses at exit status 1 when the comparison has to convert it, and two values written identically agree before any conversion.

## Status

The tool is complete at the surface its contract describes, with no stubbed or held back part, and the full CI gate runs on every change. Round 10 landed in PR 22, widening the help short guard to every command in the tree and closing the root table gap in the long pin. Round 11 landed in PR 24, stating the fold contract for crosscurrent to join this repo as a reconcile subcommand, with the hand-off formats standing as versioned contracts. Round 12 landed in PR 26, building the reconcile subcommand with crosscurrent's suite and corpus carried across, proven against the standing crosscurrent binary with no behavioural difference. Round 13 landed in PR 28, closing the fold in the storefront and the voice, with the one description restated for the combined tool and the two carried print failure lines renamed to name forkdrift under a stated amendment. Round 14 landed in PR 30, completing the description pin's positional form and closing the fold's three open notes, the key construction's double clone removed, the bundle identity's two readings reconciled on the record, and the never equal sentence adjudicated with pins. Round 15 landed in PR 32, pinning the preprocess walk's branches by sixty-seven direct unit tests, with the separator arms separated on the empty tail the census had first stated as a bound. Round 16 landed in PR 34, firing the branch state reader behind the handback command in thirty-one committed corpus cases. Round 17 landed in PR 36, firing the divergence record reader behind the handback command in thirty-two committed corpus cases. Round 18 landed in PR 38, firing the divergence record reader's sibling shapes behind the handback command in fifteen committed corpus cases. Round 19 landed in PR 40, firing the branch state reader's keyword spellings behind the handback command in twelve committed corpus cases. Round 20 landed in PR 42, firing the cross-apply classification behind the stage command in eight committed corpus cases. Round 21 landed in PR 44, firing the handback sizing rule behind the handback command in six committed corpus cases. Round 22 landed in PR 46, firing the comparison values and documents behind the compare and sweep commands in six committed corpus cases. Round 23 landed in PR 48, firing the staging value grammar behind the reconcile stage command in six committed corpus cases. Round 24 landed in PR 50, firing the bundle union behind the reconcile handback command in six committed corpus cases. Closed rounds 24 of 100.

## License

MIT. See [LICENSE](LICENSE).
