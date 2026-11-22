# chrono-harness-ci

生成宿主 CI 工作流并计算执行范围。

运行 `./check.sh` 验证独立组件。证明使用 Verus 0.2026.04.24.f8e1704，通过环境变量 `VERUS` 指定可执行文件。

[固定来源](https://github.com/ChronoAIProject/chrono-harness/tree/5f6892dfa2f73675a84e4ebdc3b06eb83b824d6e/crates/ci)

使用执行单元的宿主还须安装支持 `requirements-v1` 的 `chrono-judge-ci`。宿主通过自己的包管理器固定版本和校验值。默认从 PATH 解析该命令，也可以在单元工作流配置的顶层以 `requirements_program` 指定安装位置，例如 `.chrono-harness/bin/chrono-judge-ci`。scoped 和 full 的检测、汇总都使用这个查询入口。普通工作流的生成和核对可单独运行。
