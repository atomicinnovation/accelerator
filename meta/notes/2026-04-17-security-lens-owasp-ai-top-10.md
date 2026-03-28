---                                                                                          
date: "2026-04-17"                                                            
author: Toby Clemson                                                            
tags: [review-lenses, security-lens, owasp, ai-governance]                 
status: draft                                                                 
---                                                                             
                                                                            
# Security Lens Should Cover OWASP AI Top 10                                  
                                                                                
## Observation                                                               
                                                                              
The security lens currently anchors on the OWASP Top 10 (web application        
risks) and STRIDE for threat modelling. With AI-integrated features becoming 
common across projects, the lens has a growing blind spot: AI-specific attack 
surfaces are not explicitly covered.                                            
                                                                             
## Proposal                                                                   
                                                                                
Extend the security lens's scope to include the **OWASP Top 10 for LLM       
Applications** (and/or the OWASP Machine Learning Top 10 where applicable)    
as a first-class evaluation dimension when the change touches AI/ML code.       
                                                                             
Risks the current lens does not systematically flag include:                  
                                                                                
- **LLM01 — Prompt Injection** (direct and indirect)
- **LLM02 — Insecure Output Handling** (downstream system trust in model output)
- **LLM03 — Training Data Poisoning**
- **LLM04 — Model Denial of Service**
- **LLM05 — Supply Chain Vulnerabilities** (model weights, datasets, plugins)
- **LLM06 — Sensitive Information Disclosure** (via prompts or outputs)
- **LLM07 — Insecure Plugin/Tool Design**
- **LLM08 — Excessive Agency** (over-broad tool permissions, unbounded          
  autonomous action)
- **LLM09 — Overreliance** (unchecked model output in decision paths)
- **LLM10 — Model Theft**

## Scoping Considerations

- Conditional applicability: only apply when the change touches LLM calls,    
  prompt construction, tool-use configurations, model loading, or AI-generated  
  content flows. Fits the sub-group pattern established by ADR-0004.
- Boundary with safety lens: safety covers unsafe *behaviour* of AI systems   
  (harmful content, misuse). Security covers *attack surfaces* of AI systems    
  (injection, disclosure, theft). Some overlap around excessive agency may need
  clarification.
- Relates to the EU AI Act (August 2026) timeline noted in the gap analysis —   
  increases the urgency of explicit coverage.

## Next Steps

- Add an OWASP-AI sub-group to `skills/review/lenses/security-lens/SKILL.md`  
  with observable-characteristic triggers (imports from LLM SDKs, prompt        
  strings, tool-definition schemas, model-loading APIs).
- Update auto-detect criteria in `review-pr` and `review-plan` so AI-touching
  changes surface the security lens even when they'd otherwise look like        
  ordinary code.
- Consider whether this warrants an ADR or is a lens-internal evolution under
  ADR-0003's concern ownership rules.

## References

- `meta/research/2026-02-22-review-lens-gap-analysis.md` — flagged AI           
  governance as an emerging concern partially covered by security
- OWASP Top 10 for LLM Applications:                                            
  https://owasp.org/www-project-top-10-for-large-language-model-applications/
- OWASP Machine Learning Security Top 10:                                     
  https://owasp.org/www-project-machine-learning-security-top-10/
