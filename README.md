# decipi
decentralized competitive programming platform

## Objectives:
- create a competitive programming platform with decentralized evaluation that is as lean as possible on the central server
- everything that may add even a very tiny bit of load on the server (eg being able to see the rankings, accepting requests from non-participants during rounds) should be configurable (which results in every server request that is not essential being optional)
- precise time evaluation
- preventing cheating as much as possible given the model of the platform
- the plaftorm should not impose an heavy load on the participants either
- it should still be very flexible and allow for interactive problems, subtasks, simultaneous contests, and such
- eliminate long queue times or the need for an expensive server

## Server
The server is a queue, it's there just to put in order the participants' messages in a deterministic way.
It might also be used for hole punching.
If any other functionality is needed it may be added later, but I'd like to keep it as simple as possible.

## Contest master
Since the server is that simple, there needs to be another entity to manage the contest itself.
It will interact with the participants through the message queue managed by the server.

## Participant
A participant also interacts with the message queue, what they do is submit and evaluate.

## Evaluation
Every sumbission is in wasi, this way we can evaluate them deterministically.
The contest files (generator, scorer, interactor,...) are also in wasi.

The submissions for a problem are evaluated by participants who already solved it.
They will evaluate the submission and publish the general result, plus H(H(data),n),
where H is a hash function, n is a random number provided by the server, data is info about the execution (memory, fuel consumed,...).
When evaluatore published their result, they will publish H(data), which should be the same for all of them.

## Distribution of files
The "contest master" needs to distribute: problem statements, evaluation files.
The "participants" need to distribute submissions.

Problem statements are encrypted before the contest.
Evaluation files and submissions are encrypted with a key given only to problem solvers.

How files are actually distributed is still undecided.

