---
name: eastmoney
description: 东方财富 A 股数据查询 CLI（命令名 em）。当用户询问 A 股行情、个股详情/K线/资金流/财务/公告、大盘指数、涨停池、龙虎榜、板块行情、北向南向资金/沪深港通、可转债、ETF、十大流通股东、7x24财经快讯、股吧人气榜/热门股、自选股时使用。支持股票代码、中文名、拼音查询。
---

# eastmoney CLI (em)

查询东方财富 A 股数据的命令行工具。**所有命令都支持 `--json`，Agent 使用时务必加 `--json` 获得结构化输出。**

## 快速用法

```bash
em quote 300339 600570 --json      # 实时行情（多只）
em detail 润和软件 --json          # 个股详情（估值/盘口/涨跌停价），支持中文名
em kline 300339 --period day -n 30 --json   # K线: day/week/month/5m/15m/30m/60m; --fq qfq/hfq/none
em trend 300339 --json             # 分时走势
em market --json                   # 大盘: 核心指数+涨跌家数+涨停跌停数+两市成交额
em rank --by change -n 20 --json   # 全A排行: change/amount/turnover/volume/pe/mcap/flow/speed; --asc 升序
em board industry --json           # 板块: industry/concept/region
em board-stocks BK1031 --json      # 板块成分股（BK代码从 em board 拿）
em flow 300339 --json              # 个股资金流（主力/超大/大/中/小单，按日）
em flow-rank --json                # 主力净流入排行
em hot --json                      # 股吧人气榜（东财人气排名）
em hot --soar --json               # 股吧飙升榜
em hot-history 002497 --json       # 个股人气排名: 当前名次+历史走势
em guba 300339 --json              # 股吧最新帖子（标题/阅读/评论）
em zt --json                       # 涨停池（连板数/首封时间/封单额）; em zt dt 跌停池; em zt zb 炸板池
em lhb --json                      # 龙虎榜（默认最近交易日）; --date 2026-07-04
em info 300339 --json              # F10 公司概况
em finance 300339 -n 5 --json      # 主要财务指标（近5个报告期）
em holders 300339 --json           # 股东户数
em holders10 300339 --json         # 十大流通股东
em convertible --json              # 可转债行情列表（默认按成交额排序）
em etf --json                      # ETF 行情列表（默认按成交额排序）
em northbound --json               # 北向资金分时净流入; --south 看南向
em kuaixun --json                  # 7x24 财经快讯（--column 102重要/101全部/104公司/105市场/106机构/107宏观）
em news 300339 --json              # 个股资讯
em ann 300339 --json               # 个股公告
em search 半导体 --json            # 搜索证券/板块代码
em watch --json                    # 我的自选股（需 Cookie，默认第一个分组）
em watch --groups --json           # 列出自选股分组
em watch -g 分组名 --json          # 查看指定分组（分组名或ID）
```

## 代码格式

- 6位代码自动识别市场：`300339`(深) `600570`(沪) `920527`(北交)
- 指数需加前缀：`sh000001` 上证指数、`sz399006` 创业板指（大盘直接用 `em market` 更方便）
- 支持中文名/拼音：`em quote 润和软件`、`em quote rhrj`
- 板块用 BK 代码：`BK1031`

## 注意事项

- **限流**：东财行情接口有 IP 级限流，工具已内置 150ms 间隔限速和备用域名自动切换。批量查询用一次 `em quote 多个代码` 而不是循环单查。
- **自选股需登录 Cookie**：`em config set-cookie` 后粘贴浏览器 Cookie（登录 quote.eastmoney.com 后 F12 复制），或设置环境变量 `EM_COOKIE`。Cookie 过期时 watch 会报错提示。
- 金额单位：JSON 输出中 amount/市值等为元，volume_hand 为手。
- 非交易时间/停牌时部分字段可能为 null。
- `em zt` 默认自动回溯到最近有数据的交易日。

## 典型组合场景

- "今天大盘怎么样" → `em market --json`
- "润和软件现在什么情况" → `em detail 润和软件 --json` + `em flow 300339 --json` + `em news 300339 -n 5 --json`
- "股吧最热门的股票" → `em hot --json`（人气榜）、`em hot --soar --json`（飙升）
- "今天涨停的有哪些" → `em zt --json`
- "XX板块今天表现" → `em board industry --json` 找到板块 → `em board-stocks BKxxxx --json`
- "我的自选股" → `em watch --json`
- "今天北向资金流入多少" → `em northbound --json`
- "可转债有哪些机会" → `em convertible --by premium --json`
- "最近有什么大事" → `em kuaixun --json`
- "XX股票十大股东" → `em holders10 300339 --json`
