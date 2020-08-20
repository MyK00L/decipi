# decipi
decentralized competitive programming platform
## Objectives:
- create a competitive programming platform with decentralized evaluation that is as lean as possible on the central server
- everything that may add even a very tiny bit of load on the server (eg being able to see the rankings, accepting requests from non-participants during rounds) should be configurable (which results in every server request that is not essential being optional)
- precise time evaluation
- preventing cheating as much as possible given the model of the platform
- the plaftorm should not impose an heavy load on the participants either
- it should still be very flexible and allow for interactive problems, subtasks, and such
- eliminate long queue times or the need for an expensive server

## How to do stuff (WIP, early stage, very susceptible to change)
### Evaluation of a submission
Let T be the number of testcases for the problem at hand and N be the number of participants.
#### model 1
Every participant is given T/M testcases for the problem before the beginning of the contest (maybe encripted and they receive the key for decription at the beginning of the contest).

To evaluate a submission, it is sent to an odd number K of clients for each of the M subdivisions of the testcases (for a total of KM clients), these clients evaluate the submission and report it to the server.

It assumes that the correct verdict is the one reported by the majority among the K evaluators (and thus bans the other ones for cheating).

The server then sends the verdict to the submitter.
### model 2
At the beginning of the contest only the server has the testcases, (or the server and few other workers, maybe the round authors).

Initially to eavaluate a submission it is sent to the server only (the server is trusted and thus does not need to have the evaluation cross-checked).

When someone solves a problem, he receives all the testcases for it (initially on the server, then from peers as well, maybe receiving an hash from the server to check that they are indeed the correct testcases), and thus becomes an evaluator for such problem.

The following is the same as in model one, except there's no subdivision into M.

To evaluate a submission, it is sent to an odd number K of clients, these clients evaluate the submission and report it to the server.

It assumes that the correct verdict is the one reported by the majority among the K evaluators (and thus bans the other ones for cheating).

The server then sends the verdict to the submitter.

### How a client will evaluate the submission
What we would like is for the evaluated time taken by a submission to be as consistent as possible, ideally deterministic.

We could do one of two things:
- evaluators receive the source of a submission and evaluate it normally
- evaluators receive the submission compiled into X (some language, maybe machine language or LLVM) and emulate it counting the number of times each instruction is executed.
Each of these sends some information about a possible solution to the evaluators (expecially the first one), thus rendering cheating easier in [model 1](#model-1)

It is probabilly hard in the first one to have consistent evaluations that dont depend on the evaluator's machine.

In the second one evaluations would be deterministic (save from randomness, which could not be allowed for this reason), but it would only support languages that compile into X (but maybe could support languages interpreted by a program that can compile to X).

## An idea for a rating system (not essential for the project itself, but essential if this is to become an actual platform)
Let R_i be the rating of participant i, S_i be his score in the contest,

We define D_i to be (sum of R_j for all j st S_j < S_i)/(sum of R_j for all j st S_j != S_i)

If D_k is the D_i for k-th last contest participant i participated in,

R_i = sum D_k * C^k

with some C<1

Initially each participant has an infinite sequence of D_k all equal to some constant <=0.5
