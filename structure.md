# Project structure
## Contest client
This is the decentralized thing where contests will be run.

There is a single client for server, participant, worker, spectator, ...

What clients can take part in the contest as:
- contest master (holds all the files and the power, this is the central server)
- trusted worker (workers that will evaluate submissions exp at the start of the contest, must be at least 1, usually includes contest master)
- participant (cannot be anything else)
- spectator (cannot send stuff, but will receive stuff, cannot be anything else)

The log of public events produced by contest master is all you need to reconstruct the contest.

note: I had an idea of the concept of contest managers to offload the work of contest master, but having a single entity is simpler to keep events synced across clients.

Contests may need a key or passphrase to be accessed.

### static contest info
- contest master

### events
All events below have to be produced and signed by contest master,
should have an incremental id, be timestamped
and synced across all clients taking part in the contest.
This events along with the problem files should be enough to participate in a virtual contest offline.
- add worker
- rem worker
- add participant
- rem participant
- add spectator
- rem spectator
- set start time
- set end time
- public announcement
- add problem
- rem problem
- submission received (includes who is going to evaluate it)
- submission evaluated

TODO: sharing files (problem statements, testcases, interactor), how to spread the events.

### evaluation
When a submission is done, the contest master sends an event (submission received) containing:
- participant that submitted
- size and hash of the wasm file
- who is going to evaluate (either a single trusted worker or multiple participants that already solved the problem)

The participant then sends the submission to the evaluators.

The evaluators run the submission and relay the result to the contest master.

The contest master then relays the result as another public event (submission evaluated).

Q: What if an evaluator doesn't respond.

The submission is in wasm, this is because wasm can be [run deterministically](https://medium.com/haderech-dev/determinism-wasm-40e0a03a9b45) and can be precisely [metered](https://docs.rs/wasmtime/latest/wasmtime/struct.Config.html#method.consume_fuel),

so all evaluators should return the exact same result.

Wasm should also already provide a safe environment to execute untrusted code, and the languages that can compile to wasm is probably only going to increase in the coming years.

The interactor (if present) should also be in wasm.

## Online platform
This is not necessary, but can be a nice addition in the future (should be easy to make independently from the client).
An online platform to distribute contest information and keep ratings and rankings.
Users can link their client id/public key with their account on this platform
and a rating will be kept among the registered users through contests marked as rated (the platform will have a bot spectating contests marked as rated).
They can also add a contest (listed or unlisted, rated or unrated) which will simply be a link to the contest info
needed by the client to connect to the contest.
The contest log and files can also be published for people to participate virtually on their own or for post-contest cheat detection.

