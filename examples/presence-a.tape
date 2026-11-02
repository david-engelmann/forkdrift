# synthetic recorded tape from run c, written for this repo
interface shard-store

replicas = 3
timeout = 30
operation fetch = record
operation purge = ack
