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

#### `/api/pubmed/<pmid>`
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


#### `/api/pubmed/<query>`
* `method`: `GET`

通过`query`查询字符串, 自动查询并解析生成`[pmid]` 列表, 每个`pmid` 即为解析后的内容

`query` 字段的手动获取可以这样
1.  先在[https://pubmed.ncbi.nlm.nih.gov/advanced/]上组装字符串, 比如是这样一个查询字符串: `((target combination[Title/Abstract]) AND (("2019/12/11"[Date - Publication] : "2023/1/1"[Date - Publication]))) AND (Review[Publication Type])`

2. 在`pubmed`网页上点搜索得到链接[https://pubmed.ncbi.nlm.nih.gov/?term=%28%28target+combination%5BTitle%2FAbstract%5D%29+AND+%28%28%222019%2F12%2F11%22%5BDate+-+Publication%5D+%3A+%222023%2F1%2F1%22%5BDate+-+Publication%5D%29%29%29+AND+%28Review%5BPublication+Type%5D%29&sort=date&size=100]

3. 我们的需要的`query`就是 `term`后面的内容`%28%28target+combination%5BTitle%2FAbstract%5D%29+AND+%28%28%222019%2F12%2F11%22%5BDate+-+Publication%5D+%3A+%222023%2F1%2F1%22%5BDate+-+Publication%5D%29%29%29+AND+%28Review%5BPublication+Type%5D%29`

4. 然后可以在本地这样请求[http://192.168.2.27:4321/api/pubmed/%28%28target+combination%5BTitle%2FAbstract%5D%29+AND+%28%28%222019%2F1%2F1%22%5BDate+-+Publication%5D+%3A+%222023%2F1%2F1%22%5BDate+-+Publication%5D%29%29%29+AND+%28Review%5BPublication+Type%5D%29]即可

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
    {
      "Abstract": "Catastrophic antiphospholipid syndrome (CAPS) is a rare condition characterized by multiple thromboses affecting mainly small vessels in a short period of time in patients with antiphospholipid antibodies. A high suspicion index is mandatory in order to initiate rapidly aggressive immunomodulatory therapy to avoid a very poor prognosis. Systemic lupus erythematosus (SLE) is often associated with antiphospholipid syndrome, with a worse outcome when the catastrophic features occur. We report the case of a 64-year-old woman with a clinical debut of SLE who presented concomitantly with CAPS with several thrombosis affecting the kidney, spleen and bilateral limbs with blue toe syndrome in both legs. Furthermore, she presented with aortitis, with a malaise and myalgias and general syndrome (asthenia, hyporexia and mild weight loss). Fortunately, she had a good response to multi-target combination therapy (anticoagulants, corticosteroids, hydroxychloroquine, intravenous immunoglobulins, plasma exchange and rituximab). Here, we discuss the association between aortitis and CAPS secondary to SLE, and review the literature regarding similar conditions.",
      "AuthorFirst": "Andrés González-García",
      "AuthorLast": "Luis Manzano",
      "DOI": "10.1177/0961203320931173",
      "EpubMonth": "06",
      "EpubYear": "2020",
      "ISSN": "1477-0962",
      "JournalAbbr": "Lupus",
      "JournalTitle": "Lupus",
      "PMID": "32517572",
      "PubDateMonth": "Aug",
      "PubDateYear": "2020",
      "PublicationType": "Case Reports | Journal Article | Review",
      "Title": "Aortitis in the setting of catastrophic antiphospholipid syndrome in a patient with systemic lupus erythematosus."
    },
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
  ]
}

```


#### `/pubmed/<pmid>`
* `method`: "GET"

通过`pmid` 下载对应的`csv`文件

例子
```
请求:
    http://192.168.2.27:4321/pubmed/28250621
```