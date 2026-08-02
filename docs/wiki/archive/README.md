# 历史资料说明

仓库不保留完成 slice 的合同正文或历史取证散文。当前机制以 Wiki 和代码为准，当前
排程与缺口分别以 `goal.md` 和 ledger 为准；历史实现的完整权威是 Git objects/history。

[`docs/goal-archive.md`](../../goal-archive.md)只提供按日期和主题查找 commit 的紧凑
milestone 索引，不复述实现详情，也不链接已经删除的历史页面。需要追溯时先从索引选定
日期/主题，再使用 `git log --since=<date> --until=<date>`、`git log --grep=<term>`、
`git show <commit>:<path>` 或 `git log --all -- <path>`。删除文档不会重写历史。
