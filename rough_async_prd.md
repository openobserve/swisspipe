
currently all triggers are implement synchronously. we need to change this to async.
we need to accept the request and respond with http 202. we need to maintain every request in a table and its execution state as well so if execution stops in the middle due to system crash then execution could start after that.


stripped headers

 - authorization, cookie, x-forwarded-for, x-real-ip
  - x-forwarded-proto, host, origin, referer
  - x-csrf-token, x-api-key, x-auth-token, bearer
  - www-authenticate, proxy-authorization, proxy-authenticate
  