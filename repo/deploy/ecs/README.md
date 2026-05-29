# OneLink ECS Deploy Assets

本目录存放 `OneLink` 在共享阿里云 ECS 上的独立部署资产。

目标约束：

- 只新增 `OneLink` 自己的目录、`systemd`、`nginx` 配置与环境文件
- 不修改其他项目的站点、证书、目录和服务
- 在缺少真实生产变量时，部署脚本必须失败退出，禁止伪装上线

## 目录

- `onelink.env.example`：生产环境变量样板
- `systemd/onelink@.service`：Rust 二进制服务模板
- `nginx/onelink.cool.http.conf`：证书签发前的 HTTP 站点模板
- `nginx/onelink.cool.https.conf`：证书签发后的 HTTPS 站点模板

## 推荐目录

- 代码目录：`/opt/onelink/current`
- shared 目录：`/opt/onelink/shared`
- 环境文件：`/opt/onelink/shared/onelink.env`
- per-service 环境目录：`/opt/onelink/shared/services`
- Web 静态目录：`/var/www/onelink/current`
- certbot webroot：`/var/www/certbot`
- 日志目录：`/var/log/onelink`

## 相关脚本

- `repo/scripts/onelink-ecs-preflight-check.sh`
- `repo/scripts/onelink-ecs-deploy.sh`
- `repo/scripts/onelink-ecs-cert.sh`

## 典型执行顺序

1. 把代码同步到 `/opt/onelink/current`
2. 将 `onelink.env.example` 复制为 `/opt/onelink/shared/onelink.env` 并填入真实值
3. 在 ECS 上执行 `bash repo/scripts/onelink-ecs-preflight-check.sh`
4. 执行 `bash repo/scripts/onelink-ecs-deploy.sh`
5. DNS 生效后执行 `bash repo/scripts/onelink-ecs-cert.sh`
6. 复跑健康检查和业务 smoke
