# chrono-harness-ci

生成宿主 CI 工作流并计算执行范围。

运行 `./check.sh` 验证独立组件。证明使用 Verus 0.2026.04.24.f8e1704，通过环境变量 `VERUS` 指定可执行文件。

[固定来源](https://github.com/ChronoAIProject/chrono-harness/tree/5f6892dfa2f73675a84e4ebdc3b06eb83b824d6e/crates/ci)

使用执行单元的宿主还须安装支持 `requirements-v1` 的 `chrono-judge-ci`。宿主通过自己的包管理器固定版本和校验值。scoped 默认使用检查配置中登记的判官程序，full 默认从 PATH 解析 chrono-judge-ci。自定义判官或安装位置可在单元工作流配置的顶层以 `requirements_program` 指定安装位置，例如 `.chrono-harness/bin/chrono-judge-ci`。scoped 和 full 的检测、汇总都使用这个查询入口。普通工作流的生成和核对可单独运行。

本地收集会重新读取已安装 runner 的字节，为单元报告核对 SHA-256。登记的 runner 文件可以是指向包管理器安装位置的符号链接，父目录须在本仓真实目录内。

`chrono-ci` 和其专属测试以本仓清单独立构建。执行 Git 事件、准备检查输入、收集报告和核对测试清单时，CI 使用宿主选定的 `chrono-harness` 可执行程序。宿主启用滚动日志时，`chrono-diagnostics cli-events-v1` 接收 CI 进程的起止状态与诊断。CI 的工作流投影、单元选择和 GitHub 事件处理由本仓维护。
