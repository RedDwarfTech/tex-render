测试环境：

```bash
curl -v -X POST \
  http://tex-service.reddwarf-pro.svc.cluster.local:8000/inner-tex/project/download \
  -H "app-id:1" \
  -H "user-id:1" \
  -H "x-request-id:1" \
  -H "device-id:1" \
  -H "Content-Type: application/json" \
  -d '{"project_id": "e5deffb8930f4271aad1fe05f0179db1", "version": "latest"}' \
  --connect-timeout 5 \
  --max-time 30
```

