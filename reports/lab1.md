# Ch3 试验报告

## 简单总结

试验要求实现 `sys_trace` 的功能有三。

### 对于 `trace_request == 0`

直接将 `id` 转换为 `*const u8` 的指针，然后在 `unsafe` 块中解引用取值，再转换为需要的 `isize`。

### 对于 `trace_request == 1`

直接将 `id` 转换为 `*mut u8` 指针，然后在 `unsafe` 块内将 `data` 赋值过去即可。

### 对于 `trace_request == 1`

该实现略微麻烦，需要在 `TaskControlBlock` 中维护一个 `task_syscall_records: [isize; 512]` 数组，初始化为 `[0; 512]`，并且为 `TaskManager` 增加 `fn recored_syscall(&self, syscall_id: usize)` 和 `fn get_syscall_record(&self, syscall_id: usize) -> isize` 实现，前者将 `task_syscall_records[syscall_id]` 的值自增1，后者取出 `task_syscall_records[syscall_id]` 存的值。

然后在模块内暴露对外调用的接口，`pub fn recored_syscall(syscall_id: usize)` 和 `pub fn get_syscall_record(syscall_id: usize) -> isize`，二者分别通过 `TASK_MANAGER` 调用对应功能实现。

最后，`syscall` 模块中，每次进行 `syscall` 调用的时候，先行调用 `recored_syscall` 来纪录调用，然后在 `sys_trace` 的 `2` 分支调用 `get_syscall_record` 获取调用纪录。

## 简答作业

### 问题 1：

#### 题目：

正确进入 U 态后，程序的特征还应有：使用 S 态特权指令，访问 S 态寄存器后会报错。 请同学们可以自行测试这些内容（运行 三个 bad 测例 (ch2b_bad_*.rs) ）， 描述程序出错行为，同时注意注明你使用的 sbi 及其版本。

#### 回答：

内核直接杀掉改进程，日志如下：
```txt
[rustsbi] RustSBI version 0.3.0-alpha.2, adapting to RISC-V SBI v1.0.0
...
[kernel] PageFault in application, bad addr = 0x0, bad instruction = 0x804003a4, kernel killed it.
[kernel] IllegalInstruction in application, kernel killed it.
[kernel] IllegalInstruction in application, kernel killed it
```

### 问题 2：

深入理解 trap.S 中两个函数 `__alltraps` 和 `__restore` 的作用，并回答如下问题:

### 问题 2.1：

#### 题目：

L40：刚进入 `__restore` 时，`sp` 代表了什么值。请指出 `__restore` 的两种使用情景。

#### 回答：

第一次进入 `__restore` 的流程： 调用 `__switch` ，在 `__switch` 将 `ra` 设置为需要切换进程的 `askContext.ra` 即初始化的 `__restore` 地址，并且将 `sp` 设置为 `TaskContext.sp` 即初始化的 `init_app_cx(i)` 地址，也就是改程序对应的内核栈的栈底。

使用 `__restore` 的两种场景：①从初始状态开始运行程序。➁从中断状态恢复运行程序。

### 问题 2.2：

#### 题目：

L43-L48：这几行汇编代码特殊处理了哪些寄存器？这些寄存器的的值对于进入用户态有何意义？请分别解释。
```asm
ld t0, 32*8(sp)
ld t1, 33*8(sp)
ld t2, 2*8(sp)
csrw sstatus, t0
csrw sepc, t1
csrw sscratch, t2
```

#### 回答：

从 `TrapContext` 恢复 sstatus、sepc、sscratch 的值，`sstatus` 记录了cpu发生trap之前所在特权级，`sepc` 记录了trap之前最后一条指令的位置，`sscratch` 纪录了trap之前的sp的值也就是用户栈的地址。

### 问题2.3：

#### 题目：

L50-L56：为何跳过了 `x2` 和 `x4`？
```asm
ld x1, 1*8(sp)
ld x3, 3*8(sp)
.set n, 5
.rept 27
   LOAD_GP %n
   .set n, n+1
.endr
```

#### 回答：

`x2` 是 `sp` 寄存器，通过 `sscratch` 恢复，并不直接从 `x2` 恢复，因为后续的恢复操作都会依赖 `sp`，如果此处更改，后续的恢复操作的基址都会改变。

`x4` 是 `tp` 寄存器，本章节并未使用。

### 问题2.4：

### 题目：

L60：该指令之后，`sp` 和 `sscratch` 中的值分别有什么意义？
```asm
csrrw sp, sscratch, sp
```

在该行后，`sp` 指向用户栈，`sscratch` 指向内核栈

### 问题2.5：

#### 题目：

`__restore`：中发生状态切换在哪一条指令？为何该指令执行之后会进入用户态？

#### 回答：

状态切换发生在 `sret` 指令。`sret` 会将 `sepc` 加载到 `pc` 寄存器，并且根据 `sstatus` 切换特权级。

### 问题2.6：

#### 题目：

L13：该指令之后，`sp` 和 `sscratch` 中的值分别有什么意义？
```asm
csrrw sp, sscratch, sp
```

#### 回答：

该行之后，`sp` 指向内核栈，`sscratch` 指向用户栈。

### 问题2.7：

#### 题目：

从 U 态进入 S 态是哪一条指令发生的？

#### 回答：

`ecall` 指令，也就是在进入 `__alltrap` 之前就进入内核态了。

## 荣誉准则 

1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 **以下各位** 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：
    > Gemini 2.5 Pro

2. 此外，我也参考了 **以下资料** ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：
    > 无

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。