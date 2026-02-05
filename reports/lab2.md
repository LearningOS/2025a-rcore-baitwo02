# ch4 地址空间

## 功能简述

### sys_get_time 和 sys_trace 重写

在 `satp` 写入构造的 `token` 之后，cpu就开启了虚拟内存，之后所有的地址 `cpu` 都会视作虚拟地址，并自动将其翻译为物理地址，然后由于 `rcore` 单独设置了内核的地址空间，也就是 `kernel space`，并且在陷入内核的时候，会将页表替换为内核的页表，所以从应用拿到的虚拟地址并不能翻译为正确的地址，需要先通过应用的页表将其翻译为物理地址，再使用这个物理地址进行操作。注意使用这个物理地址的时候，`cpu` 仍然会将其视为虚拟地址，然而因为内核直接将 `ekernel..MEMORY_END` 这一块空间进行了恒等映射，所以在内核空间，`cpu` 仍然能将其解析到正确的物理地址。

所以 `sys_get_time` 的重构就非常简单，借助 `translated_byte_buffer` 用传入的虚拟地址得到物理地址，并且在这些物理地址在内核上构建可用的 `buffers`，然后将构造的 `TimeVal` 逐字节拷贝到 `buffers` 中。注意 vec 维护的是一个指针加长度，虽然指针也是虚拟地址，由上文提到的恒等映射的缘故，仍然能将值写到正确的物理地址当中。

对于 `sys_trace` 则更简单一点，先将 `ch3` 的内容进行迁移，只有读写的 `trace_request` 涉及到访存，所以只需更改 0 和 1 分支即可。对于 0 分支只需要从 `TASKMANAGER` 拿到当前应用的 `token`，并由 `token` 构建当前应用的页表映像，通过页表将虚拟页翻译为物理页，再通过 `get_bytes_array` 获取一整页的切片，更具 `offset` 在正确的位置读或写，由于数据均只有一个字节，故不需要 `translated_byte_buffer` 的参与。

### mmap 实现

对于 `mmap` 的实现，即对于一个虚拟地址区域，逐页构建映射。借助 `MemorySet` 的 `insert_framed_area` 方法实现，需通过传入的 `start` 和 `len` 构建一个 `MapArea` 然后将其 `push` 到 `MemorySet` 中，通过 `push` 方法会自动为每个虚拟页创建映射，所以只需要在 `push` 之前检查页的合法性以及权限的合法性即可。为 `TaskManager` 添加了 `mmap` 实现，通过 `TaskManager` 获取到当前 `task`，然后检查页的合法性，如有虚拟页范围内有一页可以翻译为有效页表项，则失败返回，检查合法性后就使用 `insert_framed_area` 为 `task.memory_set` 插入 `map_area`。注意要对权限进行检查，3位有效地址全0，或者3为有效地址之外的位有非0，不予通过。

### munmap 实现

对于 `munmap` 则稍微复杂一点，因为 `MemorySet` 的字段是私有的，所以额外为 `MemorySet` 实现了 `unmap_area` 方法，在 `areas` 里遍历找到起止页相符合的 `area`，将其从 `areas` 中移除，并且调用 `area.unmap` 为每一个页取消映射，权限检查、页检查等和 `mmap` 类似，不再赘述。

### 问答作业

---
1. 请列举 SV39 页表页表项的组成，描述其中的标志位有何作用？

```rust
impl PageTableEntry {
    /// Create a new page table entry
    pub fn new(ppn: PhysPageNum, flags: PTEFlags) -> Self {
        PageTableEntry {
            bits: ppn.0 << 10 | flags.bits as usize,
        }
    }
}
```

一个页表项由 44 位的物理页和 10 位的标志位组成，其中标志位的 8 位是有效位，2 位保留位。

---
缺页，缺页指的是进程访问页面时页面不在页表中或在页表中无效的现象，此时 MMU 将会返回一个中断， 告知 os 进程内存访问出了问题。os 选择填补页表并重新执行异常指令或者杀死进程。


2. 请问哪些异常可能是缺页导致的？

Exception::StorePageFault、Exception::LoadPageFault

3. 发生缺页时，描述相关重要寄存器的值，上次实验描述过的可以简略。

scause：记录发生错误的具体原因

stval：发生问题的地址

sepc：发生问题是的pc地址

---
缺页有两个常见的原因，其一是 Lazy 策略，也就是直到内存页面被访问才实际进行页表操作。 比如，一个程序被执行时，进程的代码段理论上需要从磁盘加载到内存。但是 os 并不会马上这样做， 而是会保存 .text 段在磁盘的位置信息，在这些代码第一次被执行时才完成从磁盘的加载操作。

4. 这样做有哪些好处？

减少实际内存占，避免申请较大空间，但是实际没有用上；

减少页表项数量及构建时间，不需要一次性构建所有的映射；

避免不必要的磁盘 I/O，首次访问再按需加载。

---
其实，我们的 mmap 也可以采取 Lazy 策略，比如：一个用户进程先后申请了 10G 的内存空间， 然后用了其中 1M 就直接退出了。按照现在的做法，我们显然亏大了，进行了很多没有意义的页表操作。

5. 处理 10G 连续的内存页面，对应的 SV39 页表大致占用多少内存 (估算数量级即可)？

一页是 4 KiB，10 GiB 需要 2.5 * 10^6 页，即需要 2.5 * 10^6 个页表项；

一个 3 级页可以放 512 个页表项，250,0000 个页表项需要 4883 个页，即需要 4883 个三级页表项；

同理 4883 个二级页表项需要 10 个二级页，1 个一级页，所以总共需要 1 + 10 + 4883 = 4897 页，及 4879 * 4 KiB = 19 MiB

6. 请简单思考如何才能实现 Lazy 策略，缺页时又如何处理？描述合理即可，不需要考虑实现。
在压入 area 的时候，不调用 map 进行实际的映射；

缺页的时候，先判定 vpn 是否在 areas 中，即遍历 areas 的 area，查看每个 area 的vpnrange，然后看是否囊括访问的 vpn。

---
缺页的另一个常见原因是 swap 策略，也就是内存页面可能被换到磁盘上了，导致对应页面失效。

7. 此时页面失效如何表现在页表项(PTE)上？
```rust
/// page table entry flags
pub struct PTEFlags: u8 {
    /// Valid
    const V = 1 << 0;
    /// Readable
    const R = 1 << 1;
    /// Writable
    const W = 1 << 2;
    /// eXecutable
    const X = 1 << 3;
    /// User
    const U = 1 << 4;
    /// Global
    const G = 1 << 5;
    /// Accessed
    const A = 1 << 6;
    /// Dirty
    const D = 1 << 7;
}
```

页表项的标志位只有这些，所以只能将 Valid 位置零，swap 信息应该放在别处保存