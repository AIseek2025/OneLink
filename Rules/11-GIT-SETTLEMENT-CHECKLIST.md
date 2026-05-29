# Git Settlement Checklist

## 1. 用途

本文件是对 `rules/10-MIGRATION-NOTES.md` 的执行版补充，用来指导团队把旧 `Rules/`、`Rules-V2/`、新 `rules/` 与 `docs/archive/` 的目录关系整理成干净、可审阅、跨平台稳定的 Git 历史。

它回答两个问题：

- 目录迁移应该如何提交
- 如何避免 macOS 大小写不敏感带来的协作混乱

## 2. 收口目标

最终仓库状态应满足：

- 仓库根目录只有一个现行规则目录：`rules/`
- 旧 `Rules/` 与 `Rules-V2/` 仅保留在 `docs/archive/rules-legacy-2026-05-15/`
- 根 README、Agent README、工程 README、服务 README、测试文档全部指向 `rules/`
- archive 不再作为当前默认任务入口或冻结契约入口

## 3. 提交拆分原则

不要把目录迁移、规则重写、README 联动、服务说明修订全部混在一个提交里。

推荐拆成三类提交：

1. **目录迁移提交**
   - 把旧 `Rules/`、`Rules-V2/` 迁入 `docs/archive/...`
   - 引入新的 `rules/` 目录
2. **规则内容提交**
   - 修改 `rules/` 内各规则文档
   - 修正事件名、阶段、真相源优先级
3. **入口联动提交**
   - 更新 `README.md`
   - 更新 `docs/AGENT_MEMORY_BRIEF.md`
   - 更新 `repo/README.md`
   - 更新服务 README、测试文档、contracts/README 等

## 4. 执行前检查

在执行 Git 收口前，先确认：

- 当前工作区里哪些改动是规则迁移相关
- 哪些改动是业务代码或其他并行工作
- 是否存在仅大小写不同的路径切换
- 团队成员是否已知“新目录名固定为 `rules/`”
- `.DS_Store` 是否已进入 `.gitignore`，且不再作为新变更加入规则或归档提交

若当前工作区混有大量无关改动，优先先做文档类变更收束，再做目录迁移提交。

## 5. 目录收口步骤

### Step 1：确认现行目录名

目标目录名固定为：

```text
OneLink/rules/
```

不要再保留新的现行目录名变体，例如：

- `Rules/`
- `RULES/`
- `Rules-v3/`

### Step 2：确认 archive 结构

archive 结构固定为：

```text
docs/archive/rules-legacy-2026-05-15/
  Rules/
  Rules-V2/
```

如果以后再归档下一代规则，建议新建新的 dated archive 根目录，而不是覆盖本目录。

### Step 3：确认入口联动

至少检查以下文件是否已指向 `rules/`：

- `README.md`
- `docs/AGENT_MEMORY_BRIEF.md`
- `.cursor/rules/read-agent-memory-brief.mdc`
- `repo/README.md`
- `repo/tests/integration/CHAT_MEMORY_PROFILE_SLICE.md`
- `repo/platform/contracts/README.md`
- `repo/data-platform/*/README.md`
- `repo/services/*/README.md`

### Step 4：确认 archive 降级成功

检查 archive 文档是否只保留以下角色：

- 历史依据
- 审计追溯
- 冻结字段核对
- 旧验收基线说明

若 archive 仍被某文件写成“最高优先级”或“当前 canonical 入口”，需要先修文档再提交。

## 6. macOS / 大小写注意事项

macOS 常见文件系统默认大小写不敏感，因此：

- 本机上 `Rules/` 与 `rules/` 容易被误认为无差异
- Git 历史中可能出现“旧目录删除 + 新目录未跟踪”而肉眼不易察觉

因此建议：

- 目录命名变更单独提交
- 在提交前再次检查 `git status`
- 在 PR 说明里明确声明“现行目录名统一为 `rules/`”

## 7. 推荐检查项

提交前逐项确认：

- [ ] `rules/` 下文件已完整且可读
- [ ] `docs/archive/.../Rules/` 与 `Rules-V2/` 仍可追溯
- [ ] 根目录 README 已声明 `rules/` 为唯一入口
- [ ] Agent 入口不再把 `Rules/`、`Rules-V2/` 当当前裁决源
- [ ] `repo/README.md` 已指向 `rules/`
- [ ] 服务 README 大多已采用“当前规范 / 历史依据”结构
- [ ] contracts / schema README 已声明裁决顺序
- [ ] archive README 已声明“不直接派生当前任务”
- [ ] `.gitignore` 已覆盖 `.DS_Store`
- [ ] 当前波次工单出口已明确为 `docs/execution/<yyyymmdd>-<topic>/` 或 issue tracker

## 8. 推荐提交说明模板

可参考如下提交说明：

```text
docs: settle rules migration and archive legacy planning docs

- promote `rules/` as the only current planning entry
- archive legacy `Rules/` and `Rules-V2/` under docs/archive/
- align README, agent brief, repo docs and service docs with new source-of-truth order
- keep archive for audit trail and historical reference only
```

## 9. 收口后的团队约定

收口完成后，团队默认遵守：

- 新规划写入 `rules/`
- 新工程契约写入 `repo/platform/contracts/` 或 `repo/data-platform/`
- 追溯历史时再查 `docs/archive/`
- 不再从 `Rules-V2/EXECUTION/` 直接派生当前开发任务

## 10. 最终验收

若一个新成员只阅读以下文件就能正确进入项目，说明收口成功：

- `README.md`
- `docs/AGENT_MEMORY_BRIEF.md`
- `rules/README.md`
- `rules/10-MIGRATION-NOTES.md`
- `repo/README.md`

若仍必须先阅读 archive 才能判断当前入口，说明收口还未完成。
