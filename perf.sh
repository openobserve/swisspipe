~!/bin/sh

# Simple performance test script using hyperfine
hyperfine --runs 100 --warmup 5 \
    'curl -s -X POST \
      -H "Content-Type: application/json" \
      -H "custom_test_header: custom value" \
      -d "{\"app\": \"app1\", \"user_email\": \"hi.prabhat@gmail.com\", \"user_name\": 
  \"Prabhat Sharma\"}" \
      http://localhost:3700/api/v1/d855aafc-c5a1-44a2-9393-31e3c586b698/ep'
