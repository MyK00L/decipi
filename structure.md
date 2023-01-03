# Project structure
## Contest server
The server is needed to have messages ordered the same way across all clients.
- receives message from client
- adds incremental id
- adds timestamp
- adds random number
- signs
- broadcasts message

It also changes contest settings and files, and distributes keys.

## Contest client
This is the decentralized thing where contests will be run.

There is a single client for server, participant, worker, spectator, ...

What clients can take part in the contest as:
- trusted worker (decided by server, workers that will evaluate submissions expecially at the start of the contest, must be at least 1)
- participant
- spectator (cannot send stuff, but will receive stuff)

The log of public events broadcasted by the server is all you need to reconstruct the contest.

Contests may need a key or passphrase to be accessed.

### contest info
TODO

### events
Events produced/verified and broadcasted by server.

This events along with the problem files should be enough to participate in a virtual contest offline.
- Server
    - set trusted workers
    - add torrent
    - set contest info
    - public announcement
- Client
    - join as participant
    - join as spectator
    - leave
    - submit
    - evaluation
    - ready for evaluation for problem

### use cases
Submission:
- client send message to the server saying what he's submitting and adding private torrent file
- server encrypts the torrent file with problem, makes server message and broadcasts
- clients decides who should evaluate the submission (according to some algorithm based on the message queue)
- evaluators download from torrent
- evaluators evaluate and send to server, server broadcasts
- if more evaluators have to evaluate, this repeats from step 3
- if the solution gets AC, the server sends the torrent for the problem to the submitter
- the submitter downloads it and then sends an event saying it's ready to evaluate that problem

Q: What if an evaluator doesn't respond.

A: ??

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

