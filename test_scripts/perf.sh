~!/bin/sh

# Simple performance test script using hyperfine
hyperfine --runs 10000 --warmup 5 \
    'curl -s -X POST \
      -H "Content-Type: application/json" \
      -H "custom_test_header: custom value" \
      -d "{\"app\": \"app1\", \"user_email\": \"hello@example.com\", \"user_name\": 
  \"Prabhat Sharma\"}" \
      http://localhost:3700/api/v1/a46fadd7-ddb3-48a2-b9e0-83d2a341dab1/trigger'
