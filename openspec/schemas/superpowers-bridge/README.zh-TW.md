# superpowers-bridge Schema

[English](./README.md) · [繁體中文](./README.zh-TW.md)

> 把 OpenSpec 的 artifact 治理流程(**做什麼**)與 [obra/superpowers](https://github.com/obra/superpowers) 的執行技能(**怎麼做**)整合為單一工作流。額外提供 evidence-first 的 `retrospective` artifact,補上 Superpowers 沒有的 retro 能力。
>
> 整合**完全發生在 prompt 層**——不修改 Superpowers 任何程式碼,不修改 OpenSpec CLI。Schema 版本:v1。

---

## 安裝

### 方法 1:Claude Code 一鍵 prompt(推薦)

在你專案的根目錄打開 Claude Code,把下面這段貼進去:

```
Install the superpowers-bridge schema for OpenSpec into this project:

1. Verify the project has an `openspec/` directory (run `openspec init` if missing).
2. Clone https://github.com/JiangWay/openspec-schemas to a temp dir.
3. Copy the `superpowers-bridge/` subdirectory to `openspec/schemas/superpowers-bridge/`.
4. Run `openspec schema validate superpowers-bridge` to verify.
5. Run `openspec schemas` and confirm `superpowers-bridge` is listed.
6. Clean up the temp directory.
7. Verify Superpowers plugin is installed by running `claude plugin list`.
   If not listed, run `claude plugin install superpowers@claude-plugins-official`.
8. Show me the final state.
```

### 方法 2:手動 bash(CI / 非 Claude 環境)

```bash
git clone https://github.com/JiangWay/openspec-schemas /tmp/oss
cp -R /tmp/oss/superpowers-bridge ~/your-project/openspec/schemas/superpowers-bridge
rm -rf /tmp/oss
cd ~/your-project
openspec schema validate superpowers-bridge
claude plugin install superpowers@claude-plugins-official  # 若尚未安裝
```

---

## 這個 schema 解決什麼問題?

OpenSpec 管 **「做什麼」**(artifact 生命週期:proposal / specs / tasks / verify 等)。Superpowers 管 **「怎麼做」**(執行紀律:brainstorming、writing-plans、TDD、code review)。各自堅實,但實際開發中交替使用會出現三個結構性問題:

1. **產出重複** — brainstorming 寫設計到 `docs/superpowers/specs/`,OpenSpec 又在 change 目錄重寫 `proposal.md` / `design.md`,內容高度重疊。
2. **Task 分裂** — OpenSpec 的 `tasks.md`(粗粒度 checkbox)和 Superpowers 的 `plan.md`(TDD micro-step)描述同一件事,但格式、位置、追蹤各自獨立。
3. **手動編排** — 使用者要自己判斷現在該用哪個 skill,兩個系統不會自己銜接。

### 為什麼用自定義 schema 而非修改現有 skill?

兩個替代方案被排除:

- **在 `config.yaml` 加自定義欄位**(例如 `skill_bindings`):OpenSpec CLI 不認識這些欄位,沒有驗證、沒有發現性,且需要修改多個 SKILL.md 才能讀取。
- **直接修改 opsx skill 檔**:侵入性高(影響每個 change),且 SKILL.md 升版時會被覆蓋。

自定義 schema 用的是 OpenSpec **原生支援的專案級 schema 機制**:CLI 驗證結構、`openspec schemas` 自動列出、每個 change 獨立選擇 schema(`--schema spec-driven` 或 `--schema superpowers-bridge`)、不修改任何現有 SKILL.md 或 command 檔案。

---

## 工作流與整合點

### Artifact DAG

```text
brainstorm ──→ proposal ──→ specs ──→ tasks ──→ plan ──→ [apply] ──→ verify ──→ retrospective
                  │                     ↑
                  └──→ design ──────────┘
                       (optional)
```

與 `spec-driven` 的差異:

| | spec-driven | superpowers-bridge |
|---|---|---|
| 起點 | proposal(手動撰寫) | **brainstorm**(調用 brainstorming skill) |
| Plan 層級 | tasks(粗粒度) | tasks + **plan**(TDD micro-step) |
| apply 需要 | tasks | **plan** |
| apply 方式 | 標準 task-by-task | **worktree + subagent-driven-development**(含 TDD + code-review 傳遞) |
| Post-apply | (無) | **verify** + **retrospective** artifacts |
| 新增 artifacts | — | brainstorm, plan, verify, retrospective |

### 七個 Superpowers 觸點

| # | Superpowers skill | 掛在哪 | 觸發方式 |
|---|---|---|---|
| 1 | `superpowers:brainstorming` | `brainstorm` artifact instruction | 直接(含 PRECHECK) |
| 2 | `superpowers:writing-plans` | `plan` artifact instruction | 直接(含 PRECHECK) |
| 3 | `superpowers:using-git-worktrees` | apply step 1 | 直接 |
| 4 | `superpowers:subagent-driven-development` | apply step 2 | 直接 |
| 5 | `superpowers:test-driven-development` | (#4 內部觸發) | **傳遞** |
| 6 | `superpowers:requesting-code-review` | (#4 內部觸發) | **傳遞** |
| 7 | `superpowers:finishing-a-development-branch` | apply step 4 | 直接 |

加上一個 OpenSpec built-in:`openspec-verify-change`(apply step 3,產出 `verify.md`)。

> **不支援 `executing-plans` fallback**。本 schema 是 opinionated 的:要求 subagent-capable 平台(Claude Code、Codex 等)。替代 executor `superpowers:executing-plans` 並**不會** transitively 觸發 TDD 或 code-review(已對 [SKILL.md](https://github.com/obra/superpowers/blob/main/skills/executing-plans/SKILL.md) 做事實查核 —— body 完全沒提到 TDD 或 code-review,Integration 段也未列出 `test-driven-development` 與 `requesting-code-review`)。退到 2b 等於靜默降級 Superpowers 的核心價值。若你的平台沒有 subagent 支援,改用 OpenSpec 內建的 `spec-driven` schema。

### Output redirection(產出重導)

Superpowers skill 有預設輸出路徑(例如 brainstorming 寫到 `docs/superpowers/specs/`)。本 schema 的 artifact instruction **覆寫**這個行為,透過 prompt 上下文注入,把產出重導到 change 目錄:

- brainstorming → `openspec/changes/<name>/brainstorm.md`(可選 `design.md`)
- writing-plans → `openspec/changes/<name>/plan.md`

純粹透過 invocation-time 上下文注入實現,不修改 skill 源碼。

---

## 使用方式

### 快速流程(推薦)
```bash
/opsx:ff my-feature    # 一條龍:scaffold + brainstorm + proposal + design + specs + tasks + plan
/opsx:apply            # worktree + subagent-driven-development(含 TDD + code-review)
/opsx:verify           # 產出 verify.md(5 項檢查)
/opsx:continue         # → retrospective(產出 retrospective.md,6 sections)
/opsx:archive          # 封存
```

### 逐步流程
```bash
/opsx:new my-feature --schema superpowers-bridge
/opsx:continue         # → brainstorm(互動式對話)
/opsx:continue         # → proposal
/opsx:continue         # → design(optional,僅在需要解釋技術決策時)
/opsx:continue         # → specs
/opsx:continue         # → tasks
/opsx:continue         # → plan
/opsx:apply            # → 實作 + worktree + subagent-driven-development
/opsx:verify           # → verify.md(post-apply,跑 5 項檢查)
/opsx:continue         # → retrospective.md(post-verify,evidence-first 6 sections)
/opsx:archive
```

### 切回 spec-driven
```bash
# 單一 change 用不同 schema
/opsx:new my-simple-fix --schema spec-driven

# 或修改專案預設(openspec/config.yaml: schema: spec-driven)
```

---

## Apply phase 詳細步驟

`/opsx:apply` 會觸發 [schema.yaml](./schema.yaml) `apply.instruction` 中的步驟:

#### 0. Pre-flight — 驗證必要的 Superpowers skill

確認以下 skill 都安裝才繼續:

- `superpowers:using-git-worktrees`
- `superpowers:subagent-driven-development`(傳遞依賴:`test-driven-development`、`requesting-code-review`)
- `superpowers:finishing-a-development-branch`

skill 缺失 → STOP 並通知使用者,不靜默 fallback,本 schema 內也沒有 manual mode。建議使用者在那個 change 改用 OpenSpec 內建的 `spec-driven` schema,或安裝缺失的 skill 後重來。

> 本 schema 的 v0 版本曾在這裡放「自動 commit change artifacts 到當前分支」邏輯,在 [PR #970 review](https://github.com/Fission-AI/OpenSpec/pull/970) 後移除:處理未追蹤的 change 目錄是 worktree skill 的責任,schema 不該主動改寫使用者的 git history。

#### 1. Workspace — `superpowers:using-git-worktrees`

建立 `.worktrees/<change-name>/`、切到新 branch、跑專案 setup、確認 test baseline 乾淨。

#### 2. Executor — `superpowers:subagent-driven-development`

Main agent 讀 `plan.md`,為每個 micro-task 派發 fresh subagent。每個 subagent 自動傳遞:

- **TDD**(`superpowers:test-driven-development`):先寫失敗測試 → 看著它 fail → 寫最小程式碼 → pass;production code 寫在沒測試之前會被刪掉重來
- **per-task code review**(`superpowers:requesting-code-review`):spec compliance review + code quality review;Critical 級問題擋下進度

完成 coarse task 就更新 `tasks.md` checkbox。所有 task 跑完後,對整個 implementation 再做一次 final code review。

本 schema **不支援** `superpowers:executing-plans` 作為 fallback。理由見下方「六個值得記住的設計觸點」段。

#### 3. Verification — `openspec-verify-change`

產出 `verify.md`,跑 5 項檢查:結構驗證(`openspec validate --all --json`)、task 完成度、delta-spec sync 狀態、design/specs 一致性(non-blocking warning)、實作信號(commit 狀態)。

失敗會回到對應 artifact 修正後重跑 verify。

#### 4. Completion — `superpowers:finishing-a-development-branch`

確認 tests 全綠、呈現 merge / PR / keep-branch / discard 選項、清理 worktree。

#### 5. Retrospective — `retrospective` artifact(建議,trivial fix 可跳)

Evidence-first 6 段反思(Wins / Misses / Plan deviations / Skill compliance / Surprises / Promote candidates)。每個 claim 引用 commit / file / 可量化事實。procedure 直接內嵌在 artifact instruction —— 不依賴外部 skill(Decision 3 in 設計 spec:Claude Code plugin 化延後到 v1.x)。

---

## CLI cheat sheet

| 情境 | 指令 |
|---|---|
| 首次 clone 專案後 | `bash scripts/install-git-hooks.sh` |
| 新 change(互動式) | `/opsx:new <name> --schema superpowers-bridge` 接著多次 `/opsx:continue` |
| 新 change(一鍵) | `/opsx:ff <name>` |
| 恢復中斷的 change | `/opsx:continue <name>` |
| 進入實作 | `/opsx:apply <name>` |
| 手動 verify | `/opsx:verify <name>` |
| 歸檔 | `/opsx:archive <name>` |
| 用內建(跳過 brainstorm) | `/opsx:new <name> --schema spec-driven` |
| 列出所有 schema | `openspec schemas` |
| 查看某 change 進度 | `openspec status --change <name> --json` |
| 列出 active changes | `openspec list` |
| 全專案驗證 | `openspec validate --all --json` |

---

## 六個值得記住的設計觸點

### 1. Skill-name PRECHECK(Layer 1 capability detection)

每個 invoke Superpowers skill 的 artifact / apply step 在 instruction 開頭跑 PRECHECK,確認 skill 真的存在於 LLM 的 available skills list。**缺失就 STOP,不靜默 fallback**。這是 [PR #970 review](https://github.com/Fission-AI/OpenSpec/pull/970) 顧慮 #1 第 1 層的具體應對 —— fail loud, fail early。

### 2. Schema-level vs prompt-level 整合

整合**完全**發生在 `instruction:` 欄位(純 prompt)。Superpowers 升版某個 skill 的行為時,本 schema 不用改。只有 skill 被改名或移除時才要 touch `schema.yaml`。

### 3. 傳遞依賴顯式化

TDD 與 code-review 平常藏在 `subagent-driven-development` 的 SKILL.md 裡。本 schema apply step 2a 的 instruction **直接列出**這兩個 transitive activation,讓讀者一眼看懂「apply 階段到底會發生什麼」。

### 4. Opinionated:只支援 subagent 平台,沒有手動 fallback

本 schema 要求 subagent-capable 平台(Claude Code、Codex 等)。替代 executor `superpowers:executing-plans` **不會** transitively 觸發 TDD 或 code-review(已對其 [SKILL.md](https://github.com/obra/superpowers/blob/main/skills/executing-plans/SKILL.md) 做事實查核 —— body 完全沒提及這兩者,Integration 段也未列出 `test-driven-development` 與 `requesting-code-review`)。退到 2b 等於靜默丟掉 Superpowers 帶給整合的核心價值。我們選擇在 Step 0 fail loud,並指引使用者改用內建的 `spec-driven` schema。

### 5. Evidence-based PRECHECK for verify and retrospective(Layer 2 capability detection)

時序敏感的 artifact 在 instruction 開頭跑具體 shell 證據檢查:

- **verify**:`git log <base>..HEAD | wc -l > 0` 且 `grep -c '^- \[x\]' tasks.md > 0`
- **retrospective**:`test -f verify.md` 且 `! grep -q '^- \[x\] ❌ FAIL' verify.md`

LLM 不必解讀 timing 文字 —— 跑指令、看結果即可。這是顧慮 #1 第 2 層,以及顧慮 #2 的緩解。

### 6. verify 與 retrospective 是時序錯位的 artifact(已知限制)

`verify.requires: [plan]` 與 `retrospective.requires: [verify]` 在 schema graph 上是「檔案存在」依賴,但兩者的 instruction 都明寫「MUST run AFTER apply phase / verify pass」。這是刻意錯位 —— OpenSpec 引擎只看前置 artifact 檔案存在,不會檢查 apply 是否真的跑完、verify 是否真的 pass。引擎原生的修法等 OpenSpec 引入 `post_apply` phase(對應 spec-kit 的 `after_implement` hook);上述第 5 點 evidence-based PRECHECK 是 v1 的緩解。

---

## 採用本 schema 的專案建議補一個 snapshot 區段

```markdown
## 本專案現況(snapshot: YYYY-MM-DD)

- **OpenSpec CLI**:v<version>
- **Schema**:`superpowers-bridge` v<n>
- **Specs(bounded-context 粒度)**:<n> domain 存在、<n> domain 預留 lazy backfill
- **Automation**:<pre-commit / CI 跑什麼 openspec 指令>
- **Superpowers plugin**:`superpowers@<version>`,本整合用到 N 個 skill
```

> snapshot 會隨時間 stale;權威狀態請用 `openspec list` + `openspec schemas` 現場查。

---

## 一些值得知道的設計決策

### 為什麼 brainstorm 是 artifact 而非 hook

Brainstorming 是多輪互動對話,需要使用者參與。把它做為第一個 artifact(而非 schema-level hook)有兩個好處:

1. **可跳過** — 如果使用者已知道要做什麼,可以直接寫 `brainstorm.md` 而不調用 skill。
2. **可追蹤** — `openspec status` 能顯示 brainstorm 是否完成,後續 artifacts 有明確依賴關係。

### 為什麼 plan 獨立於 tasks

`tasks.md` 是粗粒度 checkbox(「新增 PdfServiceTest」);`plan.md` 是 micro-step(「建測試骨架 → 寫 downloadPdf 測試 → 跑 → commit」)。兩者粒度與用途不同:

- `tasks.md` → 追蹤整體進度(apply phase 的 `tracks` 欄位解析 checkbox)
- `plan.md` → 指導 subagent 逐步實作(executor 的輸入)

apply 要求 `plan` 而非 `tasks`,因為 executor 需要 micro-step 才能有效工作;`tracks: tasks.md` 確保進度仍由粗粒度 checkbox 追蹤。

### 降級策略

若 Superpowers skill 不可用:

- **`brainstorm` / `plan` artifact**:使用者可明確 opt-in 改成手動撰寫(PRECHECK 會 STOP 並通知;手動模式需要使用者明確選擇,不會靜默降級)
- **`apply` phase**:本 schema 沒有 manual fallback。Step 0 PRECHECK 缺任何必要 skill 就 STOP,建議改用 OpenSpec 內建的 `spec-driven` schema 跑那個 change。理由見上面「設計觸點 #4」—— `executing-plans` 不會 transitively 觸發 TDD 與 code-review,降級的 apply 等於違背 schema 的目的

---

## 相關連結

- [schema.yaml](./schema.yaml) — 機器可讀的 schema 定義
- [templates/](./templates/) — 各 artifact 的 markdown 模板
- [README.md](./README.md) — English version
- [obra/superpowers](https://github.com/obra/superpowers) — Superpowers skill 來源
- [Fission-AI/OpenSpec](https://github.com/Fission-AI/OpenSpec) — OpenSpec
- [OpenSpec PR #970](https://github.com/Fission-AI/OpenSpec/pull/970) — 帶來這個設計的 review thread
