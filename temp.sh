SCRATCH=$(mktemp -d)                                                                                       

echo $SCRATCH

mkdir -p "$SCRATCH/.accelerator"                                                                           

cat > "$SCRATCH/.accelerator/config.md" <<'EOF'                                                            
---                                                                                                        
work:                                                                                                      
  integration: jira                                                                                        
  default_project_code: NOPE          # a key absent from your scratch Jira                                
jira:                                                                                                      
  site: atomicinnovation.atlassian.net                                                                         
  email: toby@go-atomic.io                                                                                 
  allowed_sites:                                                                                           
    - your-scratch.atlassian.net                                                                           
---                                                                                                        
EOF
