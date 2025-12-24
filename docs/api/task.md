
#### 查询等待列表

测试环境：

```bash
curl http://10.98.93.22:11015/cv/gen/v1/list -H "x-access-token:eyJhbGciOiJIUzUxMiJ9.eyJ1c2VySWQiOjkyLCJkZXZpY2VJZCI6ImYzOTQyZjU2MzUzZmFkODQyOTU2ZjQwZDNmMjY3Y2MxIiwiYXBwSWQiOiJ4U1c1YTRCYlZCIiwiZXQiOjAsInBpZCI6MTIsImV4cCI6MTY4NTI3NDMzM30.4uMrxm9iYt940RDL6UWFw2iN5y3p27w6H4ALtBBiE1ESnuyqBqznEIs1f240omN7bC7CpQoVhi73IZmxVTJjwA" -H "app-id:1" -H "user-id:1" -H "x-request-id:1" -H "device-id:1"
```

本地环境：

```bash
curl http://127.0.0.1:8000/cv/gen/v1/render-list -H "x-access-token:eyJhbGciOiJIUzUxMiJ9.eyJ1c2VySWQiOjkyLCJkZXZpY2VJZCI6ImYzOTQyZjU2MzUzZmFkODQyOTU2ZjQwZDNmMjY3Y2MxIiwiYXBwSWQiOiJ4U1c1YTRCYlZCIiwiZXQiOjAsInBpZCI6MTIsImV4cCI6MTY4NTI3NDMzM30.4uMrxm9iYt940RDL6UWFw2iN5y3p27w6H4ALtBBiE1ESnuyqBqznEIs1f240omN7bC7CpQoVhi73IZmxVTJjwA" -H "app-id:1" -H "user-id:1" -H "x-request-id:1" -H "device-id:1"
```

#### 更新编译队列状态

```bash
curl -X PUT -I http://tex-service.reddwarf-pro.svc.cluster.local:8000/tex/project/compile/status -H "x-access-token:eyJhbGciOiJIUzUxMiJ9.eyJ1c2VySWQiOjkyLCJkZXZpY2VJZCI6ImYzOTQyZjU2MzUzZmFkODQyOTU2ZjQwZDNmMjY3Y2MxIiwiYXBwSWQiOiJ4U1c1YTRCYlZCIiwiZXQiOjAsInBpZCI6MTIsImV4cCI6MTY4NTI3NDMzM30.4uMrxm9iYt940RDL6UWFw2iN5y3p27w6H4ALtBBiE1ESnuyqBqznEIs1f240omN7bC7CpQoVhi73IZmxVTJjwA" -H "app-id:1" -H "user-id:1" -H "x-request-id:1" -H "device-id:1"
```

从本地请求：


```bash
curl -I http://localhost:8000/tex/project/compile/status -H "x-access-token:eyJhbGciOiJIUzUxMiJ9.eyJ1c2VySWQiOjkyLCJkZXZpY2VJZCI6ImYzOTQyZjU2MzUzZmFkODQyOTU2ZjQwZDNmMjY3Y2MxIiwiYXBwSWQiOiJ4U1c1YTRCYlZCIiwiZXQiOjAsInBpZCI6MTIsImV4cCI6MTY4NTI3NDMzM30.4uMrxm9iYt940RDL6UWFw2iN5y3p27w6H4ALtBBiE1ESnuyqBqznEIs1f240omN7bC7CpQoVhi73IZmxVTJjwA" -H "app-id:1" -H "user-id:1" -H "x-request-id:1" -H "device-id:1"
```


更新长时间等待的队列：


```bash
curl -v -X POST "http://tex-service.reddwarf-pro.svc.cluster.local:8000/inner-tex/project/queue/expire-check" \
  -H "app-id:1" \
  -H "user-id:1" \
  -H "x-request-id:1" \
  -H "device-id:1" \
  -H "Content-Type: application/json" \
  -d "{}"
```

```bash
curl -v -X POST "http://127.0.0.1:8000/inner-tex/project/queue/expire-check" \
  -H "app-id:1" \
  -H "user-id:1" \
  -H "x-request-id:1" \
  -H "device-id:1" \
  -H "Content-Type: application/json" \
  -d "{}"
```