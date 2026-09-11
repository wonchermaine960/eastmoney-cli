# eastmoney-cli (em)

东方财富（eastmoney.com）A 股数据命令行工具，Rust 编写。覆盖个股行情、K线、资金流、财务、公告、大盘概览、涨停池、龙虎榜、板块行情、股吧人气榜和自选股。

为 Agent 设计：所有命令支持 `--json` 结构化输出；配套 skill 见 `skills/eastmoney/SKILL.md`。

## 安装

```bash
cargo build --release
# 二进制: target/release/em，可复制到 PATH：
cp target/release/em /usr/local/bin/
```

## 命令一览

| 命令 | 说明 |
|---|---|
| `em quote <code...>` | 实时行情（多只，支持中文名/拼音/sh sz前缀） |
| `em detail <code>` | 个股详细快照（估值、五档盘口、涨跌停价） |
| `em kline <code> [--period day\|week\|month\|5m\|15m\|30m\|60m] [-n 30] [--fq qfq\|hfq\|none]` | K线 |
| `em trend <code> [--days 1]` | 分时走势 |
| `em market` | 大盘概览：核心指数、涨跌家数、涨停/跌停数、两市成交额、涨跌分布 |
| `em rank [--by change\|amount\|turnover\|volume\|pe\|mcap\|flow\|speed] [--asc]` | 全A排行榜 |
| `em board [industry\|concept\|region]` | 板块行情 |
| `em board-stocks <BK代码>` | 板块成分股 |
| `em flow <code>` | 个股资金流（日） |
| `em flow-rank` | 主力净流入排行 |
| `em hot [--soar]` | 股吧人气榜 / 飙升榜 |
| `em hot-history <code>` | 个股人气排名历史 |
| `em guba <code>` | 股吧最新帖子 |
| `em zt [zt\|dt\|zb] [--date YYYYMMDD]` | 涨停/跌停/炸板池 |
| `em lhb [--date YYYY-MM-DD]` | 龙虎榜 |
| `em info <code>` | F10 公司概况 |
| `em finance <code>` | 主要财务指标 |
| `em holders <code>` | 股东户数 |
| `em news <code>` / `em ann <code>` | 个股资讯 / 公告 |
| `em search <关键词>` | 搜索证券/板块 |
| `em watch [--groups] [-g 分组名]` | 自选股（需 Cookie），支持多分组 |
| `em config set-cookie / show` | Cookie 管理 |

## 自选股登录

自选股接口需要东财登录态：

```bash
em config set-cookie   # 回车后粘贴 Cookie，Ctrl-D 结束
```

Cookie 获取：浏览器登录 [quote.eastmoney.com](https://quote.eastmoney.com/zixuan/) → F12 → Network → 任意请求 → Request Headers → 复制完整 `Cookie` 值。也可用环境变量 `EM_COOKIE`（优先级更高）。Cookie 保存在 `~/.config/eastmoney-cli/cookie.txt`（权限 600）。

## 反爬说明

东财 push2 行情集群有 IP 级限流（高频请求后直接掐断连接）。本工具的对策：

- 全局限速：请求间隔 ≥150ms
- 浏览器 UA + Referer
- push2 不可达时自动降级到 push2delay 集群（A股数据一致；但该集群无五档盘口和K线）

如果 K线报错"可能被限流"，等几分钟再试。请勿用脚本高频循环调用。

## 接口文档

所有数据接口的 URL、参数、字段含义见 [docs/API.md](docs/API.md)。
