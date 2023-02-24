## rust-eutils

`eutils http api` 的二次封装， 方便获取我们指定的数据， 开启并行限制和缓存

## 当前支持的http 请求

> 服务部署在`http://192.168.2.27:4321/`

#### 统一的返回格式

```json
正常:
    {
        "ok": XXX     // ok 对应的请求内容, 格式有特定的请求接口指定
    }

错误:
    {
        "error": {"msg": XXX}  // msg 对应错误提示
    }
```

> 有些请求实际上是下载文件, 所以成功就是下载文件, 错误就是 `404`

#### `/api/pubmed/pmid/<pmid>`
* `method`: `GET`

通过`pmid`请求对应的`json`格式内容

例子:

```
请求:
  http://192.168.2.27:4321/api/pubmed/pmid/28250621

返回:
    {
  "ok": {
    "Abstract": "222",
    "AuthorFirst": "Yamuna Devi Bakthavatchalam",
    "AuthorLast": "Balaji Veeraraghavan",
    "DOI": "10.4103/0974-777X.199997",
    "EpubMonth": "",
    "EpubYear": "",
    "ISSN": "0974-777X",
    "JournalAbbr": "J Glob Infect Dis",
    "JournalTitle": "Journal of global infectious diseases",
    "PMID": "28250621",
    "PubDateMonth": "",
    "PubDateYear": "2017",
    "PublicationType": "Journal Article | Review",
    "Title": "1."
  }
}
```


#### `/api/pubmed/total/<query>`
* `method`: `GET`

通过`query`查询字符串, 自动查询并解析生成`[pmid]` 列表, 每个`pmid` 即为解析后的内容

> 该接口返回所有查询到的数据, 所以请求会比较慢, 如果需要分页,或者获取查询到的总数, 请用下面的接口`/api/pubmed/<query>?<cur_page>&<page_size>`

`query` 字段的手动获取可以这样
1.  先在 https://pubmed.ncbi.nlm.nih.gov/advanced/ 上组装字符串, 比如是这样一个查询字符串: `((target combination[Title/Abstract]) AND (("2019/12/11"[Date - Publication] : "2023/1/1"[Date - Publication]))) AND (Review[Publication Type])`

2. 在`pubmed`网页上点搜索得到链接 https://pubmed.ncbi.nlm.nih.gov/?term=%28%28target+combination%5BTitle%2FAbstract%5D%29+AND+%28%28%222019%2F12%2F11%22%5BDate+-+Publication%5D+%3A+%222023%2F1%2F1%22%5BDate+-+Publication%5D%29%29%29+AND+%28Review%5BPublication+Type%5D%29&sort=date&size=100 

3. 我们的需要的`query`就是 `term`后面的内容`%28%28target+combination%5BTitle%2FAbstract%5D%29+AND+%28%28%222019%2F12%2F11%22%5BDate+-+Publication%5D+%3A+%222023%2F1%2F1%22%5BDate+-+Publication%5D%29%29%29+AND+%28Review%5BPublication+Type%5D%29`

4. 然后可以在本地这样请求 http://192.168.2.27:4321/api/pubmed/%28%28target+combination%5BTitle%2FAbstract%5D%29+AND+%28%28%222019%2F1%2F1%22%5BDate+-+Publication%5D+%3A+%222023%2F1%2F1%22%5BDate+-+Publication%5D%29%29%29+AND+%28Review%5BPublication+Type%5D%29 即可

5. 在用`python`代码时候就是要把`((target combination[Title/Abstract]) AND (("2019/12/11"[Date - Publication] : "2023/1/1"[Date - Publication]))) AND (Review[Publication Type])`
` 做个`urlencode` 就可以了

结果如下

```json
{
  "ok": [
    {
      "Abstract": "Glioblastoma (GBM) remains a cancer of high unmet clinical need. Current standard of care for GBM, consisting of maximal surgical resection, followed by ionisation radiation (IR) plus concomitant and adjuvant temozolomide (TMZ), provides less than 15-month survival benefit. Efforts by conventional drug discovery to improve overall survival have failed to overcome challenges presented by inherent tumor heterogeneity, therapeutic resistance attributed to GBM stem cells, and tumor niches supporting self-renewal. In this review we describe the steps academic researchers are taking to address these limitations in high throughput screening programs to identify novel GBM combinatorial targets. We detail how they are implementing more physiologically relevant phenotypic assays which better recapitulate key areas of disease biology coupled with more focussed libraries of small compounds, such as drug repurposing, target discovery, pharmacologically active and novel, more comprehensive anti-cancer target-annotated compound libraries. Herein, we discuss the rationale for current GBM combination trials and the need for more systematic and transparent strategies for identification, validation and prioritisation of combinations that lead to clinical trials. Finally, we make specific recommendations to the preclinical, small compound screening paradigm that could increase the likelihood of identifying tractable, combinatorial, small molecule inhibitors and better drug targets specific to GBM.",
      "AuthorFirst": "Timothy Johanssen",
      "AuthorLast": "Daniel Ebner",
      "DOI": "10.3389/fonc.2022.1075559",
      "EpubMonth": "01",
      "EpubYear": "2023",
      "ISSN": "2234-943X",
      "JournalAbbr": "Front Oncol",
      "JournalTitle": "Frontiers in oncology",
      "PMID": "36733367",
      "PubDateMonth": "",
      "PubDateYear": "2022",
      "PublicationType": "Journal Article | Review",
      "Title": "Glioblastoma and the search for non-hypothesis driven combination therapeutics in academia."
    },
...
  ]
}

```


#### `/api/pubmed/<query>?<cur_page>&<page_size>`
* `method`: `GET`

分页接口查询, `<query>`的参数设置和上面的`../total/..`接口一直, 区别是加入下面两个参数
  * `cur_page`:  当前页设置, 默认为0   // 0: 表示第一页
  * `page_size`:  每页数量设置, 默认为10

例子

```
/api/pubmed/<query>?cur_page=2       # 获取第三页数据, 默认每页10, 也就是获取第 20-30的结果

```

返回:  http://192.168.2.27:4321/api/pubmed/%28%28target+combination%5BTitle%2FAbstract%5D%29+AND+%28%28%222018%2F12%2F1%22%5BDate+-+Publication%5D+%3A+%222023%2F1%2F1%22%5BDate+-+Publication%5D%29%29%29+AND+%28Review%5BPublication+Type%5D%29?cur_page=2&page_size=1 

``` json
{
  "ok": {
    "count": 8,                                    # 该请求总共数据8个
    "cur_page": 2,                                 #  返回第三页 
    "data": [
      {
        "Abstract": "Open access to 3D structure information from the Protein Data Bank (PDB) facilitated discovery and development of >90% of the 79 new antineoplastic agents (54 small molecules, 25 biologics) with known molecular targets approved by the FDA 2010-2018. Analyses of PDB holdings, the scientific literature and related documents for each drug-target combination revealed that the impact of public-domain 3D structure data was broad and substantial, ranging from understanding target biology (∼95% of all targets) to identifying a given target as probably druggable (∼95% of all targets) to structure-guided lead optimization (>70% of all small-molecule drugs). In addition to aggregate impact assessments, illustrative case studies are presented for three protein kinase inhibitors, an allosteric enzyme inhibitor and seven advanced-stage melanoma therapeutics.",
        "AuthorFirst": "John D Westbrook",
        "AuthorLast": "Stephen K Burley",
        "DOI": "10.1016/j.drudis.2020.02.002",
        "EpubMonth": "02",
        "EpubYear": "2020",
        "ISSN": "1878-5832",
        "JournalAbbr": "Drug Discov Today",
        "JournalTitle": "Drug discovery today",
        "PMID": "32068073",
        "PubDateMonth": "May",
        "PubDateYear": "2020",
        "PublicationType": "Journal Article | Research Support, N.I.H., Extramural | Research Support, U.S. Gov't, Non-P.H.S. | Review",
        "Title": "Impact of the Protein Data Bank on antineoplastic approvals."
      }
    ],
    "page_size": 1,                                # 每页大小1
    "query_text": "((target+combination[Title/Abstract])+AND+((\"2018/12/1\"[Date+-+Publication]+:+\"2023/1/1\"[Date+-+Publication])))+AND+(Review[Publication+Type])"
  }
}
```


#### `/pubmed/<pmid>`
* `method`: `GET`

通过`pmid` 下载对应的`csv`文件

例子
```
请求:
    http://192.168.2.27:4321/pubmed/28250621
```