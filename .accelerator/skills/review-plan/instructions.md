When making edits to the plan:
- Use test-driven development as far as applicable
- Ensure all phases are independently integratable/mergeable
- Never include comments that describe what code could otherwise express. 
  Comments only provide value if they signal something extremely non-obvious to
  the reader. We have a *very* low tolerance for comments. Actively remove them
  from plans you create. Further, references to ADRs, work items, ACs etc. in
  comments can go stale fast so don't include them.
