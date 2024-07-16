# Rust for Linux

The goal of this project is to add support for the Rust language to the Linux kernel. This repository contains the work that will be eventually submitted for review to the LKML.

Feel free to [contribute](https://github.com/Rust-for-Linux/linux/contribute)! To start, take a look at [`Documentation/rust`](https://github.com/Rust-for-Linux/linux/tree/rust/Documentation/rust).

General discussions, announcements, questions, etc. take place on the mailing list at rust-for-linux@vger.kernel.org ([subscribe](mailto:majordomo@vger.kernel.org?body=subscribe%20rust-for-linux), [instructions](http://vger.kernel.org/majordomo-info.html), [archive](https://lore.kernel.org/rust-for-linux/)). For chat, help, quick questions, informal discussion, etc. you may want to join our Zulip at https://rust-for-linux.zulipchat.com ([request an invitation](https://lore.kernel.org/rust-for-linux/CANiq72kW07hWjuc+dyvYH9NxyXoHsQLCtgvtR+8LT-VaoN1J_w@mail.gmail.com/T/)).

All contributors to this effort are understood to have agreed to the Linux kernel development process as explained in the different files under [`Documentation/process`](https://www.kernel.org/doc/html/latest/process/index.html).

<!-- XXX: Only for GitHub -- do not commit into mainline -->
# tty0tty
我是lwz，这是一个使用rust来重新编写tty0tty模块的测试。目前已经完成了tty0tty模块的编写与安全抽象，但是安全抽象部分还不是很完善，并且并没有添加///SAFETY注释，仅作为实验使用。
