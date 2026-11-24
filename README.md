# chrono-harness-ci

生成宿主 CI 工作流并计算执行范围。

运行 `./check.sh` 验证独立组件。证明使用 Verus 0.2026.04.24.f8e1704，通过环境变量 `VERUS` 指定可执行文件。

[固定来源](https://github.com/ChronoAIProject/chrono-harness/tree/5f6892dfa2f73675a84e4ebdc3b06eb83b824d6e/crates/ci)

使用执行单元的宿主还须安装支持 `requirements-v1` 的 `chrono-judge-ci`。宿主通过自己的包管理器固定版本和校验值。scoped 默认使用检查配置中登记的判官程序，full 默认从 PATH 解析 chrono-judge-ci。自定义判官或安装位置可在单元工作流配置的顶层以 `requirements_program` 指定安装位置，例如 `.chrono-harness/bin/chrono-judge-ci`。scoped 和 full 的检测、汇总都使用这个查询入口。普通工作流的生成和核对可单独运行。

本地收集会重新读取已安装 runner 的字节，为单元报告核对 SHA-256。登记的 runner 文件可以是指向包管理器安装位置的符号链接，父目录须在本仓真实目录内。

`chrono-ci` 和其专属测试以本仓清单独立构建。执行 Git 事件、准备检查输入、收集报告和核对测试清单时，CI 使用宿主选定的 `chrono-harness` 可执行程序。宿主启用滚动日志时，`chrono-diagnostics cli-events-v1` 接收 CI 进程的起止状态与诊断。CI 的工作流投影、单元选择和 GitHub 事件处理由本仓维护。

独立仓库可以在现有 GitHub 作业中使用自己的原生检查命令和编译缓存。在 `.chrono-harness/ci/` 登记 `chrono-github-cache-actions/v1`，其中 `jobs.<job>.command` 是原命令的参数数组，`action_path` 是本仓 `.github/actions/<名称>/action.yml`，`persistent_cache` 采用现有 `chrono-github-cache/v1` 登记。运行 `chrono-ci generate --host-root . --config <登记路径>`，在工具已安装的 GitHub 作业中以 `uses: ./.github/actions/<名称>` 调用生成的 action。相同登记使用 `chrono-ci verify` 检查。重复保存者会在写任何 action 前报错，缓存目录由规划器核对。原生 action 的下载目录也作为规划器缓存登记。

需要在 detector 的启动步骤读取 Actions 原始产物时，宿主可登记 `job_gating.detector.bootstrap_github_token: true`。生成的步骤通过 `GH_TOKEN` 接收该工作流已有的只读凭据，默认不传递。

## 工作流判定的证明

在本仓运行 `mise exec -- ./check.sh`。构建实际库和运行原生测试后，该入口按生产根与测试根分别执行固定的 Verus，再核对每个操作的实际参数、进程输出与编译器依赖清单。

| 生产根 | 业务调用与独立预期 |
| --- | --- |
| `verified_literals.rs` | `gating.rs` 核对原生 job 的状态和身份，生成器与缓存、发布、启动步骤按登记选择其投影。`ci-tests/tests/verified_literals.rs` 覆盖相应谓词及生产投影。 |
| `verified_download.rs` | `gather.rs` 从原下载进程的 stderr 解析 HTTP 状态，以次数、时间预算和登记的状态码决定是否重试。`ci-tests/tests/verified_download.rs` 核对冲突状态、成功、失败和预算边界。 |
| `verified_inventory.rs` | `test-inventory` 校验真实测试程序的列表、忽略项和分组，核对分组非空、互不重叠、并集等于完整清单。`ci-tests/tests/verified_inventory.rs` 核对列表与集合的边界。 |

上述证明处理进程所返回的数据。GitHub 的传输、宿主选择的测试程序、它的列表输出和实际文件内容由各自的生产者提供，宿主验收须核对那些原件。
