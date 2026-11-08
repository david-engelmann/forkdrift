# forkdrift

Tells whether two recorded artifacts that state the same interface actually diverge. Both files are read, compared structurally, and reported without executing either.

Forkdrift is a command line tool that compares recorded artifacts, the plain text statements of one interface that two runs of a system leave behind. It reads two artifacts, compares their statements one by one, and prints a report of every divergence it finds, quoting the lines that disagree. A sweep compares one baseline artifact against several others in turn and attributes every divergence to the pair that raised it. Forkdrift detects divergence only, and crosscurrent reconciles the pairs it catches.

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

Every command reads only the files it is handed. An artifact that cannot be read or does not parse refuses with one error line and exit status 2, and a malformed invocation exits with status 2. A source name that carries a line break is refused before anything prints it, so a name cannot splice a forged row into a report. A value whose exponent runs past the conversion limit refuses at exit status 1 when the comparison has to convert it, and two values written identically agree before any conversion.

## Status

The tool is complete at the surface its contract describes, with no stubbed or held back part, and the full CI gate runs on every change. Round 10 landed in PR 22, widening the help short guard to every command in the tree and closing the root table gap in the long pin. Closed rounds 10 of 100.

## License

MIT. See [LICENSE](LICENSE).
