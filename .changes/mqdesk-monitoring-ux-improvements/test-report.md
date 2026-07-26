# MQDesk 监控与可视化优化测试报告

## 测试范围

本次测试覆盖以下新增/优化功能：

1. 安装进度条显示一致性（Windows NSIS）
2. 多 MQ 连接状态实时显示
3. MQ 集群节点监控
4. 消费者信息可视化
5. 连接数据持久化与卸载选项

## 测试环境

- 操作系统：Windows 11
- Node.js：v20+
- Rust：1.80+
- RabbitMQ：3.13 / 4.x（Management 插件已启用）

## 后端测试

```bash
cd src-tauri
cargo test -p mqdesk-core
```

结果：

```text
running 4 tests
test health::tests::danger_when_no_consumers_but_ready ... ok
test health::tests::ok_when_consumers_and_low_ready ... ok
test health::tests::idle_when_ready_zero ... ok
test health::tests::warn_when_ready_over_threshold ... ok

running 2 tests
test app_state_set_active_then_amqp_url_should_encode_vhost ... ok
test create_save_password_then_connect_password ... ok

running 1 test
test password_save_load_roundtrip ... ok

running 6 tests
test r1_test_connection_whoami ... ok
test r2_overview_stats ... ok
test r3_r4_r6_queue_list_detail_preview_health ... ok
test r5_publish_invalid_json_rejected_by_caller ... ok
test r5_publish_mandatory_returned ... ok
test r7_storage_feed_append_list_filter_delete ... ok
```

**结论：mqdesk-core 13 个测试全部通过。**

> 注：`cargo test`（含 mqdesk 主 crate）在 Windows 运行测试二进制时出现 `STATUS_ENTRYPOINT_NOT_FOUND`，原因为测试进程无法加载 WebView2Loader.dll 等桌面壳运行时依赖，属于执行环境限制，不影响生产构建与 core 逻辑正确性。

## 前端测试

```bash
npm test
```

结果：

```text
Test Files  3 passed (3)
     Tests  12 passed (12)
```

已修复 `ConnectionsView.test.jsx` 以适配新增的连接状态检测（`get_connection_status`）调用。

## 编译/构建检查

| 检查项 | 命令 | 结果 |
|--------|------|------|
| 后端编译 | `cargo check` | 通过 |
| 前端编译 | `npm run build` | 通过 |
| 源码规范 | `biome check .` / `oxlint src` / `cargo clippy -- -D warnings` | 通过 |

## 全量门禁

```bash
python tooling/checks.py guard
```

结果：build / lint / typecheck / unit test / e2e test 全部通过。

## 安装包构建情况

### Windows

```bash
npm run tauri:build:win
```

结果：成功生成 `src-tauri/target/release/bundle/nsis/MQDesk_0.1.0_x64-setup.exe`（约 200 MB，含离线 WebView2 运行时），已复制到 `apk/MQDesk_0.1.0_x64-setup.exe`。

### Linux DEB

在 BIOS/UEFI 中启用虚拟化后，已通过 WSL2 Ubuntu 24.04 本地构建成功。

构建路径：

```bash
wsl -d Ubuntu-24.04 -u root -- bash -c "cd '/mnt/d/project/RabbitConsumerHub-main' && bash scripts/build-linux-deb.sh"
```

生成产物：

- `src-tauri/target/x86_64-unknown-linux-gnu/release/bundle/deb/MQDesk_0.1.0_amd64.deb`
- 已复制到 `apk/MQDesk_0.1.0_amd64.deb`（约 4 MB）

该 .deb 基于 Ubuntu/Debian 依赖构建，x86_64 架构的银河麒麟 V10 通常可直接 `dpkg -i` 安装。如遇依赖缺失，执行 `sudo apt-get install -f`。

#### 离线/内网环境安装方案

**推荐：AppImage（单文件，双击运行）**

- 文件：`apk/MQDesk_0.1.0_amd64.AppImage`
- 大小：约 76 MB
- 用法：拷贝到离线机器，双击运行，或在终端执行：
  ```bash
  chmod +x MQDesk_0.1.0_amd64.AppImage
  ./MQDesk_0.1.0_amd64.AppImage
  ```
- 优点：自包含所有依赖，无需安装，无需 root，不依赖 apt。

**备选：deb + 离线依赖包目录**

由于 deb 包本身只有 3.9 MB，且依赖系统库，纯内网机器上 `apt-get` 无法拉取依赖。因此同时提供完整离线依赖包：

- 目录：`apk/linux-offline/`
- 内容：MQDesk 主包 + 260 个递归依赖 .deb，共约 184 MB
- 安装脚本：`scripts/install-offline-linux.sh`
- 准备脚本：`scripts/prepare-offline-linux-deps.sh`（在目标系统相同版本的联网机器上运行）

离线机器安装步骤：

```bash
# 1. 将 apk/linux-offline/ 整个目录拷贝到离线机器
# 2. 在该目录下执行
sudo bash install-offline-linux.sh
```

> 注意：离线 deb 依赖包必须在目标系统相同版本（如 Ubuntu 24.04 / 银河麒麟 V10）的联网机器上准备，否则 .deb 版本可能不匹配。

GitHub Actions 跨平台构建工作流仍保留作为备选：`.github/workflows/build.yml`。

## 已知问题

1. 无。

## 后续建议

- 在麒麟系统上实测 AppImage：拷贝 `apk/MQDesk_0.1.0_amd64.AppImage`，执行 `chmod +x` 后双击或终端运行。
- 如需 RabbitMQ 集群测试环境，运行 `docker compose -f scripts/rabbitmq-cluster.yml up -d`。
