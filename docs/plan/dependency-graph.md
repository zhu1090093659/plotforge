# Task Dependency Graph

```mermaid
graph TD
  subgraph P1["Phase 1: Core Runtime Hardening"]
    T1_1["T1.1 ActionIntent schema"]
    T1_2["T1.2 Inject planner port"]
    T1_3["T1.3 Trace contract expansion"]
    T1_4["T1.4 Fixture validation"]
    T1_5["T1.5 Export whitelist audit"]
    T1_1 --> T1_2
    T1_2 --> T1_3
    T1_1 --> T1_3
    T1_3 --> T1_5
  end

  subgraph P2["Phase 2: Story Craft and Provider Contracts"]
    T2_1["T2.1 StoryCraft schema"]
    T2_2["T2.2 Agent output contracts"]
    T2_3["T2.3 TextModelProvider + fake"]
    T2_4["T2.4 Reference compliance"]
    T2_5["T2.5 JSON Schema / TS contracts"]
    T2_1 --> T2_2
    T2_2 --> T2_3
    T2_1 --> T2_4
    T2_1 --> T2_5
  end

  subgraph P3["Phase 3: Desktop Studio Skeleton"]
    T3_1["T3.1 Frontend workspace"]
    T3_2["T3.2 Tauri command bridge"]
    T3_3["T3.3 Dashboard/editor shell"]
    T3_4["T3.4 Playtest/debugger views"]
    T3_5["T3.5 Desktop QA runbook"]
    T3_1 --> T3_2
    T3_2 --> T3_3
    T3_2 --> T3_4
    T3_3 --> T3_5
    T3_4 --> T3_5
  end

  subgraph P4["Phase 4: Media and Job Pipeline"]
    T4_1["T4.1 Asset registry"]
    T4_2["T4.2 Job queue core"]
    T4_3["T4.3 Image provider/fake"]
    T4_4["T4.4 Runtime/export media refs"]
    T4_1 --> T4_3
    T4_2 --> T4_3
    T4_3 --> T4_4
  end

  subgraph P5["Phase 5: Persistence, Debugger, Export v2"]
    T5_1["T5.1 Save/restore"]
    T5_2["T5.2 SQLite cache/index"]
    T5_3["T5.3 Player/export v2"]
    T5_4["T5.4 Export profiles + AI manifest"]
    T5_1 --> T5_2
    T5_3 --> T5_4
  end

  subgraph P6["Phase 6: Steam and Workshop Exploration"]
    T6_1["T6.1 Workshop package schema"]
    T6_2["T6.2 Submission Kit draft"]
    T6_3["T6.3 Compliance QA docs"]
    T6_1 --> T6_2
    T6_2 --> T6_3
  end

  T1_3 --> T2_1
  T1_5 --> T4_1
  T2_1 --> T4_1
  T2_3 --> T4_3
  T1_2 --> T3_1
  T1_3 --> T3_2
  T2_5 --> T3_2
  T1_3 --> T5_1
  T4_1 --> T5_2
  T4_4 --> T5_3
  T1_5 --> T5_3
  T5_4 --> T6_1
```

## Parallel Execution Notes

- Phase 1 lane B (`T1.4`) can run alongside lane A; lane C waits for trace/export boundaries.
- Phase 2 lane C can start once StoryCraft schema is stable, independent of provider fake work.
- Phase 3 UI lanes can split after the Tauri command bridge exists.
- Phase 4 asset registry and job queue can start in parallel, then converge in image provider integration.
- Phase 5 SQLite cache should not start until save and asset contracts are stable.
- Phase 6 remains deliberately late and mostly offline/schema-driven.
