Create a commit after each completed phase. 

Comments are a last resort. A comment is a signal that the code failed to
express its own intent. Before writing one, do the work to make the code
itself clear — rename, extract, restructure. Only keep a comment when it
captures something genuinely non-obvious to a skilled developer that no amount
of refactoring could convey (e.g. *why* an unusual choice was made, an
external constraint, a subtle invariant). Never include comments that
describe what code could otherwise express — we have a *very* low tolerance
for comments. Actively remove them from plans you create. References to
ADRs, work items, acceptance criteria, plan phases etc. in comments can go
stale fast, so don't include them.
