# 东方财富接口档案（2026-07 实测）

本文档记录 eastmoney-cli 所用接口，全部经 curl 实测验证。

## 域名与反爬

| 域名 | 用途 | 说明 |
|---|---|---|
| `push2.eastmoney.com` | 实时行情（ulist/stock/get/clist/trends/fflow） | **有 IP 级限流**：短时间高频请求后 TCP 层直接掐断响应（连接成功但无响应，curl 返回 000）。封禁持续 10 分钟以上，波及 `N.push2` 全部编号镜像 |
| `push2delay.eastmoney.com` | 同上（A股准实时，港股延迟） | 独立集群，push2 被封时仍可用。**不提供 K线，不提供五档盘口（f11-f40 返回空）** |
| `push2his.eastmoney.com` | K线/历史数据 | 与 push2 同样的限流策略 |
| `push2ex.eastmoney.com` | 涨停池/跌停池/炸板池/涨跌分布 | 独立集群 |
| `datacenter-web.eastmoney.com` | 数据中心（龙虎榜/股东/资金流明细） | 无明显限流 |
| `datacenter.eastmoney.com` | F10（`/securities/api/data/v1/get`，需 `source=HSF10&client=PC`） | 无明显限流 |
| `emappdata.eastmoney.com` | 股吧人气榜 | POST JSON，需 `appId`、`globalId`（任意 UUID 即可） |
| `gbapi.eastmoney.com` | 股吧帖子 | 无需登录 |
| `searchapi.eastmoney.com` | 搜索/代码表 | 返回 `QuoteID`（即 secid），是代码→市场解析的权威来源 |
| `np-anotice-stock.eastmoney.com` | 公告 | 无需登录 |
| `quote.eastmoney.com/zixuan/api/*` | 自选页资讯（infomines/zixun） | 无需登录 |
| `myfavor.eastmoney.com` | 自选股 | **需登录 Cookie**（缺失时报 `CUToken 为空`）。appkey=`e9166c7e9cdfad3aa3fd7d93b757e9b1`（从 zixuan/build/index.js 提取） |

对策：全局限速（请求间隔 ≥150ms）+ 浏览器 UA + Referer；push2 失败自动降级 push2delay。

## secid 格式

`市场.代码`：`0`=深/北交所、`1`=沪、`90`=板块(BK)、`116`=港股。
例：`0.300339` 润和软件、`1.000001` 上证指数、`0.399006` 创业板指、`90.BK1031` 板块。

## qt 行情接口（push2 / push2delay）

通用参数：`fltt=2`（返回小数而非放大整数）、`invt=2`、`ut=fa5fd1943c7b386f172d6893dbfba10b`（可选）。

1. **批量行情** `GET /api/qt/ulist.np/get?secids=1.000001,0.300339&fields=...`
2. **单股快照** `GET /api/qt/stock/get?secid=0.300339&fields=...`
3. **排行/板块列表** `GET /api/qt/clist/get?pn=1&pz=20&po=1&np=1&fid=f3&fs=<筛选>`
   - 全A股：`fs=m:0+t:6,m:0+t:80,m:1+t:2,m:1+t:23,m:0+t:81+s:2048`
   - 行业板块：`fs=m:90+t:2+f:!50`；概念板块：`fs=m:90+t:3+f:!50`；地域板块：`fs=m:90+t:1+f:!50`
   - 板块成分股：`fs=b:BK1328`
4. **分时** `GET /api/qt/stock/trends2/get?secid=..&fields1=f1,f2,f3,f8&fields2=f51,f53,f56,f58&ndays=1&iscr=0`
5. **个股资金流(日)** `GET /api/qt/stock/fflow/kline/get?secid=..&fields1=f1,f2,f3,f7&fields2=f51,f52,f53,f54,f55,f56&klt=101&lmt=N`
   - klines 值：`日期,主力,小单,中单,大单,超大单`（净流入，元）
6. **K线（仅 push2his）** `GET /api/qt/stock/kline/get?secid=..&fields1=f1,f2,f3&fields2=f51,...,f61&klt=101&fqt=1&end=20500101&lmt=N`
   - klt: 1/5/15/30/60分钟, 101日 102周 103月；fqt: 0不复权 1前复权 2后复权
   - fields2: f51日期 f52开 f53收 f54高 f55低 f56量(手) f57额 f58振幅% f59涨跌幅% f60涨跌额 f61换手%

### 常用字段（ulist/clist，fltt=2）
f1精度 f2最新价 f3涨跌幅% f4涨跌额 f5成交量(手) f6成交额(元) f7振幅 f8换手% f9市盈(动) f10量比
f12代码 f13市场 f14名称 f15最高 f16最低 f17今开 f18昨收 f20总市值 f21流通市值 f23市净率
f62主力净流入 f100行业 f104上涨家数(板块) f105下跌家数 f128领涨股 f140领涨股代码

### stock/get 专有字段
f43最新 f44最高 f45最低 f46今开 f47量(手) f48额 f50量比 f51涨停价 f52跌停价 f57代码 f58名称
f60昨收 f71均价 f84总股本 f85流通股本 f92每股净资产 f116总市值 f117流通市值
f162PE(动) f167市净率 f168换手% f169涨跌额 f170涨跌幅% f171振幅
五档（仅实时集群）：卖5→卖1 = f31/f32, f33/f34, f35/f36, f37/f38, f39/f40（价/量）；买1→买5 = f19/f20, f17/f18, f15/f16, f13/f14, f11/f12

## push2ex（涨停/跌停/炸板/涨跌分布）

`ut=7eea3edcaed734bea9cbfc24409ed989&dpt=wz.ztzt`

- 涨跌分布 `GET /getTopicZDFenBu` → `fenbu:[{"-1":318},...]`（涨跌幅桶→家数，±11 为 >10.5%）
- 涨停池 `GET /getTopicZTPool?...&Pageindex=0&pagesize=N&sort=fbt:asc&date=20260707`
  - 字段：c代码 n名称 p价格(×1000) zdp涨跌幅 lbc连板数 fbt首封时间(HHMMSS) lbt最后封板 zbc炸板次数 fund封单资金 hs换手 hybk行业 zttj{days,ct}=X天Y板
- 跌停池 `getTopicDTPool`、炸板池 `getTopicZBPool`（同参数）

## emappdata 股吧人气榜（POST JSON）

body 公共字段：`{"appId":"appId01","globalId":"<任意uuid>"}`

- 人气榜 `POST /stockrank/getAllCurrentList` + `pageNo,pageSize,marketType:""` → `[{sc:"SZ002185",rk:1,rc:0,hisRc:22}]`（rc=名次变化，hisRc=历史新高距离?）
- 飙升榜 `POST /stockrank/getAllHisRcList` + `pageNo,pageSize` → `[{sc,rk,hrc飙升值,hrcrk飙升榜名次}]`
- 个股当前排名 `POST /stockrank/getCurrentLatest` + `srcSecurityCode:"SZ002497"` → `{rank,rankChange,marketAllCount,calcTime}`
- 个股排名历史 `POST /stockrank/getHisList` + `srcSecurityCode` → `[{calcTime,rank}]`（约120天）

## datacenter 数据中心

`GET https://datacenter-web.eastmoney.com/api/data/v1/get?reportName=<RPT>&columns=ALL&filter=(...)&sortColumns=..&sortTypes=-1&pageSize=N&pageNumber=1`

- 龙虎榜：`RPT_DAILYBILLBOARD_DETAILSNEW`，filter `(TRADE_DATE='2026-07-07')`；字段 SECURITY_CODE/SECURITY_NAME_ABBR/CLOSE_PRICE/CHANGE_RATE/EXPLAIN/EXPLANATION/BILLBOARD_NET_AMT/BILLBOARD_BUY_AMT/BILLBOARD_SELL_AMT/TURNOVERRATE
- 个股资金流明细：`RPT_DMSK_TS_STOCKNEW`，filter `(SECURITY_CODE="300339")`
- 股东户数：`RPT_HOLDERNUMLATEST`，字段 HOLDER_NUM/PRE_HOLDER_NUM/HOLDER_NUM_CHANGE/HOLDER_NUM_RATIO/END_DATE
- F10 概况（datacenter.eastmoney.com/securities/api/data/v1/get + `source=HSF10&client=PC`）：
  - `RPT_F10_BASIC_ORGINFO` filter `(SECUCODE="300339.SZ")` → 公司全称/行业(EM2016)/董事长/简介(ORG_PROFILE)/注册资本等
  - `RPT_F10_FINANCE_MAINFINADATA` sort REPORT_DATE desc → EPSJB每股收益 BPS每股净资产 TOTAL_OPERATE_INCOME? TOTALOPERATEREVE营收 PARENTNETPROFIT归母净利 TOTALOPERATEREVETZ营收同比 PARENTNETPROFITTZ净利同比 XSMLL毛利率 ROEJQ加权ROE ZCFZL资产负债率 REPORT_DATE_NAME

## 其他

- **搜索** `GET https://searchapi.eastmoney.com/api/suggest/get?input=<kw>&type=14&count=10` → `QuotationCodeTable.Data[]`：Code/Name/PinYin/MktNum/QuoteID/Classify(AStock|Index|Fund|BK...)/SecurityTypeName
- **公告** `GET https://np-anotice-stock.eastmoney.com/api/security/ann?sr=-1&page_size=N&page_index=1&ann_type=A&stock_list=300339` → data.list[]：title/art_code/notice_date/columns[].column_name；详情页 `https://data.eastmoney.com/notices/detail/<code>/<art_code>.html`
- **个股资讯** `POST https://quote.eastmoney.com/zixuan/api/infomines`，form `codes=0.300339`（多个逗号分隔）→ result{secid:{data:[{date,title,url}]}}
- **股吧帖子** `GET https://gbapi.eastmoney.com/webarticlelist/api/Article/Articlelist?code=300339&type=0&index=1&pageSize=N&deviceid=100&version=200&product=Guba&plat=Web` → re[]：post_title/post_publish_time/post_click_count/post_comment_count/user_nickname
- **自选股（需 Cookie，已实测）** `https://myfavor.eastmoney.com/v4/webouter/`
  - 分组列表 `GET ggdefstkindexinfos?appkey=e9166c7e9cdfad3aa3fd7d93b757e9b1` → `data.ginfolist:[{gid,gname,ver,...}]`
  - 组内股票 `GET gstkinfos?appkey=...&g=<gid>` → `data.stkinfolist:[{security:"市场$代码$内部id",star,updatetime,price(加自选时价格)}]`
  - 认证走 Cookie（关键键 `ct`/`ut`/`pi`/`uidal`）；Cookie 缺失 → `state=-2 "CUToken 为空"`；缺 `g` 参数 → `state=-4 "参数不足"`
  - 其他方法（未验证）：`as?`增 `ds?`删 `ms?`移动分组
  - 注意：ulist GET 的 secids 超长会被拒（大分组几百只），需分批（CLI 按 80 只/批）
