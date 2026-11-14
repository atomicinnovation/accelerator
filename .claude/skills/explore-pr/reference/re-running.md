# Re-running on an existing workspace

The workspace is reused, and must be: `data/heads/` holds the diff at every
head the page was built for, which is what lets the reviewer's verdicts
survive a moved head (`reference/content-model.md`, "When the head moves").

## When the head has moved

1. Export and extract again (step 4 of the skill).
2. Set `pr.head` in `explore.json` to the new head, and `pr.base` if the
   merge base moved.
3. Run `python3 {skill dir}/scripts/build.py {workspace} --reanchor`. Every
   stop and ledger row quotes the code it stands on (content model,
   "Anchors"), so this moves each one whose code merely shifted and warns
   about each whose code is gone or ambiguous.
4. Decide what to re-read from what it reports and from the files that
   changed (`files*.json` maps each file to its chapter):

   - a chapter none of whose files changed needs nothing;
   - a chapter whose stops and rows all held or moved, and whose changed
     hunks fall outside every stop and row, needs only its new hunks
     checked for something a stop should now cover;
   - a chapter with an anchor that was not found, or a changed hunk inside
     a stop or row, needs its pass re-run.
5. Refresh the design and the overview if the shape of the change moved.
6. Build and verify as in step 9, then publish the hosted page to the same
   artifact, never a new one: the reviewer's records live with it.
7. Tell the reviewer what they will see: lines changed since their review
   are unread again, and each verdict whose code changed says which head it
   was recorded at.

An unchanged head needs only the steps the user asks for.
