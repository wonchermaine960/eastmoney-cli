# AGENTS.md

本文件为 AI 编码 Agent（Claude Code、Cursor、Aider、Codex CLI 等）在本仓库中工作的指导文档。

## 项目概述

东方财富（eastmoney.com）A 股数据 CLI，Rust 编写，二进制名 `em`。22 个子命令覆盖行情/K线/大盘/涨停池/龙虎榜/板块/资金流/股吧人气榜/F10/自选股。为 Agent 设计：所有命令支持 `--json`；配套 skill 在 `skills/eastmoney/SKILL.md`。

## 常用命令

```bash
cargo build                    # 开发构建（无测试，验证靠真实调用）
cargo build --release          # 发布构建（已配 strip+lto，约 3.3MB）
./target/debug/em market       # 冒烟测试：大盘概览
./target/debug/em quote 300339 --json   # 验证 JSON 输出
```

没有单元测试。改动后的验证方式是对真实接口跑对应子命令（数据源是线上 API，交易时间内外字段完整度不同，停牌股数值字段为 `"-"`）。

## 关键约束：东财反爬（改代码前必读）

- `push2` / `push2his` 集群有 **IP 级 WAF 封禁**：短时高频请求后 TCP 层直接掐断（连 80 端口都封），波及全部编号镜像（`N.push2his`），持续 1 小时以上。**调试时不要用脚本高频 curl 这些域名**，开发期间曾因此被封导致 K线不可测。
- `push2delay`（准实时行情，无 K线、无五档）、`push2ex`（涨停池）、`datacenter`、`emappdata`、`searchapi` 是独立集群，不受牵连。
- 对策已内置于 `src/client.rs`：全局 150ms 限速（`throttle`）、浏览器 UA + Referer、主备域名链（`QT_HOSTS` 实时→push2delay 降级；`HIS_HOSTS` 4 个 push2his 镜像轮询）。新增接口必须走 `Http` 的方法，不要绕过限速自建请求。

所有接口的 URL/参数/字段号含义/坑（实测记录）在 **`docs/API.md`**，新增数据源时先查它、改完同步更新它。

## 架构

数据流：`main.rs`(clap 定义+分发) → `commands.rs`(命令实现+表格/JSON 双输出) → `api.rs`(接口封装，返回 `serde_json::Value`) → `client.rs`(统一 HTTP：限速/降级/JSONP 剥壳)。

- **`secid.rs`** — 一切用户输入（6位代码/sh sz 前缀/BKxxxx/中文名/拼音）解析为东财 secid（`市场.代码`，0=深/北交、1=沪、90=板块、116=港股）。本地规则优先，兜底走 searchapi suggest（其返回的 `QuoteID` 就是 secid，权威）。指数有歧义（000001 既是上证指数又是平安银行），裸 6 位按股票解析，指数需 `sh` 前缀。
- **`api.rs`** — 每个接口一个函数。刻意不做强类型 struct，字段用 `fXX` 编号且可能为 `"-"`，全部经 `util.rs` 的 `num_at`/`str_at` 防御式取值。行情字段语义注意：`fltt=2` 让接口直接返回小数（否则是放大整数）；`quotes()` 按 80 只/批分批（URL 超长会被拒，自选股大分组 700+ 只依赖此机制）。
- **`commands.rs`** — 每个命令先判 `ctx.json` 走 JSON 分支（英文键名，Agent 用），否则渲染 `util.rs::Table`（CJK 宽度对齐、TTY 下红涨绿跌）。日期计算不用 chrono，`today_str` 内置北京时间(+8)civil date 算法，`em zt` 靠它自动回溯最近交易日。
- **`config.rs` + 自选股** — Cookie 存 `~/.config/eastmoney-cli/cookie.txt`（600），`EM_COOKIE` 环境变量优先。myfavor 流程：`ggdefstkindexinfos` 拿分组 → `gstkinfos?g=<gid>` 拿股票（`security` 格式 `市场$代码$内部id`）；appkey 硬编码在 `client.rs`（提取自东财自选页 JS，页面改版才会失效）。

## 修改惯例

- 新增子命令：`main.rs` 加 enum variant 和分发 → `commands.rs` 实现（含 `--json` 分支）→ 需要新接口则加 `api.rs` → 同步 `skills/eastmoney/SKILL.md`、README 表格和 `docs/API.md`。
- 输出中数值格式统一用 `util.rs` 的 `fmt_amount`(万/亿)、`fmt_vol`(手)、`colorize_pct`；不要在命令里手写格式化。
- 用户交流与代码注释用中文。
