# synthetic recorded tape from run a, written for this repo
interface shard-store

replicas = 3
timeout = 30
operation fetch = record
operation store = ack
label primary = node-a
excludes = legacy-mode
