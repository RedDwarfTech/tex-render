#### 下载项目源码

从 texhub 下载项目源码 zip 包。

- **method**: `PUT`
- **path**: `/inner-tex/project/download`
- **request body**: `{"project_id":"<project_id>","version":"latest"}`

测试环境（对应报错日志中的请求）：

```bash
curl -v -X PUT \
  "http://tex-service.reddwarf-pro.svc.cluster.local:8000/inner-tex/project/download" \
  -H "Content-Type: application/json" \
  -d '{"project_id":"e5deffb8930f4271aad1fe05f0179db1","version":"latest"}' \
  --connect-timeout 10 \
  --max-time 30 \
  -o e5deffb8930f4271aad1fe05f0179db1.zip
```

本地环境：

```bash
curl -v -X PUT \
  "http://127.0.0.1:8000/inner-tex/project/download" \
  -H "Content-Type: application/json" \
  -d '{"project_id":"e5deffb8930f4271aad1fe05f0179db1","version":"latest"}' \
  --connect-timeout 10 \
  --max-time 30 \
  -o e5deffb8930f4271aad1fe05f0179db1.zip
```

替换 `project_id` 即可下载其他项目；`-o` 将响应体保存为 zip 文件。
