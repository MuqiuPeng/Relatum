# Relatum 研究方案
## 关系闭包驱动的数学概念自主涌现

**版本**：v0.1
**状态**：草案

---

## 一、研究背景与核心猜想

### 1.1 出发点

Relatum 是一个以纯关系逻辑为基础的结论闭包生成器。系统中唯一的基本实体是项（Term）与关系（Relation）。给定一组关系声明，系统通过不动点迭代计算推理闭包，生成当前信息所能推及的最大关系边界。

在开发过程中，一个更深层的问题浮现：

> **数学概念的演化，是否是某种选择压力下的必然涌现？这种选择压力是否可以被形式化和计算？**

### 1.2 核心猜想

```
数学概念的演化 ≈ 某种选择压力下的筛选过程

具体形式：
  给定初始关系种子 F₀
  存在一个函数 score(rule, closure)
  在 score 引导下自主搜索新关系模式
  系统能够独立重新发现已有的数学结构
  且涌现顺序与数学史高度吻合
```

这个猜想有两种可能的结果，都有价值：

- **支持**：说明现有数学结构是某种选择压力下的必然涌现，不是人类任意选择的结果
- **证伪**：说明数学史的发展有偶然性，现有结构不是唯一可能的路径

---

## 二、验证策略：从计算逻辑开始

### 2.1 为什么选择计算逻辑作为验证场

计算逻辑具备其他数学领域不具备的实验优势：

| 维度 | 计算逻辑 | 其他领域 |
|------|---------|---------|
| 论域性质 | 完全离散 | 代数需预设结构，分析涉及连续 |
| 发展顺序 | 有明确历史记录 | 拓扑、分析的概念边界模糊 |
| 价值标准 | 完备性、可判定性是硬标准 | 价值判断依赖数学直觉 |
| 负样本 | 随机逻辑规则大多无价值，可合成 | 历史负样本几乎不可获得 |
| 终点已知 | 知道系统应该推演出什么 | 开放问题，无法验证 |

### 2.2 内在一致性风险

计算逻辑作为验证场存在一个循环论证风险：

```
Relatum 本身基于逻辑推导
用逻辑推导去重新发现逻辑
→ 系统可能只是在镜子里看自己
```

**应对策略**：初始种子必须严格控制，不能包含任何逻辑连接词的影子。`derive/2` 的语义需要在不预设推理规则的情况下定义。这是整个实验设计中最需要谨慎处理的地方，优先级高于 loss 函数的选择。

---

## 三、实验设计

### 3.1 初始种子（最小化原则）

不预设 AND、OR、NOT，不预设任何逻辑连接词：

```prolog
rel symbol/1      % 某个符号存在
rel formula/1     % 某个公式存在
rel holds/1       % 某个公式在当前模型下成立
rel derive/2      % 从一个公式集能推出另一个公式
                  % 注意：derive 的语义不预设任何具体推理规则
```

种子选择的判断标准：去掉任何一条，系统是否就无法开始推演？如果仍然可以，则继续精简。

### 3.2 期望的涌现顺序

```
第一阶段（基础连接词）：
  否定     ← derive 关系的对称破缺自然诱导
  合取     ← 多个 holds 合并为单一 holds
  蕴含     ← derive 关系的抽象化

第二阶段（基础性质）：
  重言式   ← 在所有模型下 holds 的公式
  矛盾式   ← 在所有模型下 ¬holds 的公式
  等价关系 ← derive 的对称闭包

第三阶段（元性质）：
  完备性概念    ← 关于 derive 关系本身的关系
  可判定性概念  ← 关于闭包计算是否终止的关系
  推理规则的元性质
```

**检验标准（提前声明，不事后调整）**：

- **强支持**：涌现顺序与历史顺序高度吻合（Spearman 相关 > 0.7），且系统不需要任何逻辑先验
- **弱支持**：涌现了正确的结构，但顺序与历史不同（说明历史发展有偶然性）
- **证伪**：系统在相同 loss 下推演出与逻辑完全无关的结构（loss 函数设计有根本问题）

### 3.3 候选 Score 函数

不预设哪个正确，让数据来判断：

```rust
// 候选一：生成力
fn score_generativity(rule, closure) -> f64 {
    let delta = closure.with(rule).size() - closure.size();
    delta as f64 / rule.complexity() as f64
}

// 候选二：压缩力
fn score_compression(rule, closure) -> f64 {
    let subsumed = closure.facts_subsumed_by(rule);
    subsumed as f64 / rule.length() as f64
}

// 候选三：复用度（自引用信号）
fn score_reuse(rule, closure_history) -> f64 {
    closure_history.iter()
        .map(|snapshot| snapshot.reference_count(rule.pattern()))
        .sum::<usize>() as f64
}

// 候选四：跨域连接力
fn score_connectivity(rule, closure) -> f64 {
    let before = closure.cross_domain_connections();
    let after = closure.with(rule).cross_domain_connections();
    (after - before) as f64
}

// 综合（权重本身是研究变量）
fn score_combined(rule, closure, w: [f64; 4]) -> f64 {
    w[0] * score_generativity(rule, closure)
    + w[1] * score_compression(rule, closure)
    + w[2] * score_reuse(rule, closure_history)
    + w[3] * score_connectivity(rule, closure)
}
```

### 3.4 候选规则的生成机制

采用**从闭包结构归纳元模式**的策略：

```rust
fn generate_candidates(closure) -> Vec<Rule> {
    // 1. 提取闭包中的高频关系模式
    let patterns = closure.extract_frequent_patterns(min_freq: 3);
    
    // 2. 对每个高频模式，生成其"泛化版本"
    let generalized = patterns.iter()
        .map(|p| p.generalize_with_vars())
        .collect();
    
    // 3. 生成已有规则的组合
    let composed = closure.rules.iter()
        .flat_map(|r1| closure.rules.iter()
            .map(|r2| r1.compose_with(r2)))
        .filter(|r| r.is_well_formed())
        .collect();
    
    [generalized, composed].concat()
}
```

### 3.5 主推演循环

```
初始状态：最小种子 F₀

loop:
    candidates = generate_candidates(current_closure)
    scored = candidates.map(|r| (r, score(r, current_closure)))
    selected = scored.top_k(k)
    current_closure = current_closure.with(selected)
    emergence_log.record(round, selected, current_closure.size())
    if growth_rate < threshold:
        break

输出：涌现顺序日志 + 最终关系结构
```

---

## 四、与现有工作的关系

### 4.1 最接近的工作：HR 系统

Colton (2002) 的 HR 系统（*Automated Theory Formation in Pure Mathematics*）。

| 维度 | HR 系统 | Relatum |
|------|---------|---------|
| 基础语义 | 谓词逻辑 | 关系闭包 |
| 搜索机制 | 启发式搜索 | loss 函数显式驱动 |
| 元推理 | 无 | 元关系内生化 |
| 可撤销性 | 无 | Cell + Provenance 支持增量撤销 |

### 4.2 其他相关工作

- **Datalog / Souffle**：关系闭包计算，但无 loss 驱动的自主搜索
- **ILP（归纳逻辑编程）**：依赖正负样本，Relatum 无需外部标签
- **Answer Set Programming**：含否定的闭包语义，Relatum 目前不含否定

---

## 五、实施步骤

### 第一步：验证可表达性（当前）

手动编码计算逻辑的核心概念，验证它们能否在关系框架内被正确表达和推导。

**完成标志**：能在 Relatum 里推导出命题逻辑的完备性（所有重言式都可被推出）。

### 第二步：实现最小 score（只用生成力）

实现 `score_generativity` 和 `generate_candidates` 的最简版本。

**完成标志**：系统在无人工干预下，在 10 轮内发现合取或蕴含的雏形。

### 第三步：对比涌现顺序

将系统的涌现日志与计算逻辑的历史发展顺序对比。

**完成标志**：得到明确的支持或证伪判断。

### 第四步：扩展 score 维度

根据第三步结果决定是否引入压缩力、复用度、连接力。

### 第五步（如果支持）：扩展到群论

验证系统是否能在不同数学领域重复相同的涌现模式。

---

## 六、风险与应对

| 风险 | 描述 | 应对 |
|------|------|------|
| 循环论证 | 系统基于逻辑，验证对象也是逻辑 | 种子严格控制，derive/2 不预设推理规则 |
| 搜索空间爆炸 | 候选规则数量不可处理 | 复杂度上限：body ≤ 3，vars ≤ 2 |
| score 与直觉不符 | 生成力最高的规则可能是平凡规则 | 复杂度惩罚 + 人工"惊喜度"校验 |
| 过早终止 | 发现核心结构前就饱和 | 增长率而非饱和作为终止条件 |

---

## 七、潜在论文贡献

**主要**：可运行的数学概念自主涌现模型，在计算逻辑领域验证涌现顺序与历史一致性。

**次要**：
- 元关系内生化
- 基于复用度的无监督概念价值信号
- 与 HR 系统的精确对比

---

## 八、参考文献

- Colton, S. (2002). *Automated Theory Formation in Pure Mathematics*. Springer.
- Odrzywolek, A. (2026). *All elementary functions from a single binary operator*. arXiv:2603.21852
- Abiteboul, S., Hull, R., Vianu, V. (1995). *Foundations of Databases*.
- Ritt, J. F. (1948). *Integration in Finite Terms*.
