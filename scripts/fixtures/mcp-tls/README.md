# MCP TLS 测试固定件

`ca.crt` / `localhost.crt` / `localhost.key` 是**只供测试**的证书材料，用于
`crates/bbb-server-admin` 的 TLS 终止测试：证明 PEM 解析、rustls 接受器构造与一次真实
握手是通的。

- `ca.crt`：测试 CA（`CN=bbb test CA`，`CA:TRUE`）。测试把它放进自己的 root store。
- `localhost.crt` / `localhost.key`：由该 CA 签发的叶证书，`CN=localhost`、
  SAN `DNS:localhost, IP:127.0.0.1`、`CA:FALSE`、`serverAuth`。服务端出示它。
- 任何真实信任库都不认这条链。私钥公开在仓库里是有意的：它不保护任何东西，也不得用于
  任何真实端点。真实部署的证书与私钥由 CLI 指向受权限保护的文件，绝不进入仓库、argv
  或日志。

重新生成（CA 私钥用完即弃，因此换链就是换这三个文件）：

```sh
openssl req -x509 -newkey rsa:2048 -nodes -keyout ca.key -out ca.crt -days 36500 \
  -subj "/CN=bbb test CA" \
  -addext "basicConstraints=critical,CA:TRUE" -addext "keyUsage=critical,keyCertSign"
openssl req -new -newkey rsa:2048 -nodes -keyout localhost.key -out localhost.csr \
  -subj "/CN=localhost"
printf 'subjectAltName=DNS:localhost,IP:127.0.0.1\nbasicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature,keyEncipherment\nextendedKeyUsage=serverAuth\n' > leaf.ext
openssl x509 -req -in localhost.csr -CA ca.crt -CAkey ca.key -CAcreateserial \
  -out localhost.crt -days 36500 -extfile leaf.ext
rm -f localhost.csr leaf.ext ca.key ca.srl
```
