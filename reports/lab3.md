# Ch5 进程调度

## 简单总结

### 上一章 syscall 的迁移

对于 `sys_get_time` 基本和上一章一样，通过 `translated_byte_buffer` 获取 buf 集合，然后逐字节从构造的 `TimeVal` 写入。

对于 `sys_mmap` 和 `sys_munmap` 这两个由于这一节引入了 PROCESSOR，直接从 PROCESSOR 获取当前 task，然后调用构造的好的方法即可。只是需要注意参数的合法性检测即可。

### stride 调度算法实现

首先构建 `Stride(u8)` 结构体，并为其实现 PartialOrd 特性(只需要能用于比较即可)。

为 `TaskControlBlock` 添加 `stride、priority` 属性，然后为 `TaskControlBlock` 实现 `Ord` 特性(`BinaryHeap` 需要)。

再为 `TaskControlBlock` 实现一个 `set_priority` 方法，在 `process` 中调用。

将 `TaskManager` 的 `ready_queue` 更换为 `ready_minheap: BinaryHeap<Reverse<Arc<TaskControlBlock>>>`，直接借助 `BinaryHeap` 实现一个最小堆。

最后在进程执行完一个时间片，进行调度切换到下一个进行执行时 `suspend_current_and_run_next()` 更新当前 task 的 stride 值(注意是+= BIG_STRIDE/ inner.priority)。

## 问答作业¶

### stride 算法深入

考虑 Stride(u8), BIG_STRIDE = 0xff 的情况，考虑两个进程的 A, B 各自的 stride 分别为 a0, b0, 且 a0 < b0。

由于优先级 >= 2, 对于最极端情况的 pass 值取到最大为 BIG / 2，

考虑未发生溢出的情况 a1 = a0 + BIG/2, 若不发生溢出(a0 < BIG/2), 

此时显然 a1 > b0(B从比A小获得执行, 最多也就加BIG/2, 所以b0不会比a0大超过 BIG/2), a1 - b0 = a0 + BIG/2 - b0 < BIG/2, 所以不溢出的情况下二者的 diff 必然小于 BIG/2。

考虑发生溢出的情况 a1 = a0 + BIG/2 = BIG + a2,

此时显然 a2 < b0(a0 < b0, a0 + BIG绕一圈仍小于 b0, 显然绕半圈更不可能超过 b0), b0 - a2 = b0 - (a0 + BIG/2 - BIG) = b0 - a0 + BIG/2 > BIG/2, 所以溢出的情况下二者的 diff 必然大于 BIG/2。

## 荣誉准则 

1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 **以下各位** 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：
    > Gemini老师, Chatgpt老师, Kimi老师

2. 此外，我也参考了 **以下资料** ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：
    > 无

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。