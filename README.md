# subaru_web_ql
### An experimental port of SubaruWebQL from C/C++ to Rust

The purpose of this project is to discover pitfalls and limits of Rust when used in interactive client-server applications at the Japanese Virtual Observatory (JVO). This project is not meant for general public use. It can only be run internally as part of the JVO Portal since it connects to various other internal network services.

### Remarks

After a while the first impression of Rust is very very bad! In the quest for "safety" it seems to be banning too many things that come handy in the hands of a *careful* programmer. Blanket bans for the sake of safety only make life difficult for a low-level C/C++/VHDL programmer more used to total control over all aspects of the code. Blanket bans will produce "safe" and reliable code but so will a very *careful* C/C++ programmer.

