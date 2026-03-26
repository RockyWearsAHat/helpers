# Systems: Linking and Loading — Symbol Resolution, Relocation, and Dynamic Binding

Linking and loading are the processes that transform compiled object code into executable programs. They handle symbol resolution, address relocation, and dynamic binding of dependencies. Understanding these mechanisms is essential for debugging, performance optimization, and cross-platform compatibility.

## The Linking Pipeline

**Static linking** occurs at compile time and combines object files and libraries into a single executable. The linker reads each object file's symbols—defined (exported), undefined (imported), and local—and resolves cross-module references by matching undefined symbols to exported ones. When all symbols resolve, the linker writes the executable to disk. This produces a self-contained binary with no external dependencies, but at the cost of code duplication across multiple programs using the same libraries.

**Dynamic linking** defers resolution until runtime. The executable contains a list of required libraries and unresolved symbol references. The dynamic linker (on Unix: `ld.so` or `ld-musl.so.1`; on Windows: `ntdll.dll`) loads shared libraries into the process address space and performs final symbol resolution. This reduces disk and memory footprint by sharing library code, but introduces the possibility of version incompatibility ("DLL hell" on Windows).

## Symbol Resolution and Relocation

Compilers cannot know where their code will reside in the final process address space, so they generate code assuming a fixed base address (often zero). The linker must adjust these assumptions through **relocation**, rewriting absolute references and jump targets to reflect actual addresses.

### Relocation Entries

Object files contain relocation entries describing which addresses must be adjusted and how. Common relocation types:

- **R_X86_64_ABS64**: Write the absolute address of a symbol at this location.
- **R_X86_64_REL32**: Write a relative 32-bit offset from the current location to the symbol.
- **R_X86_64_COPY**: Copy a symbol's data from a shared library into writable memory (dynamic linking only).
- **R_X86_64_JUMP_SLOT**: Reserve space for a lazy-bound function pointer (dynamic linking).

During static linking, the linker applies relocations once using known addresses. During dynamic linking, relocations are deferred to load time or, in lazy binding, to the first function call.

### Address Space Layout Randomization (ASLR)

Modern systems randomize the base address at which executables and libraries load, to mitigate address-space attacks. Position-independent executables (PIE) and position-independent code (PIC) use RIP-relative addressing to avoid absolute references, eliminating the need for runtime relocation. On systems without PIE, the linker stores a relocation table that the dynamic linker uses to adjust absolute references.

## Binary Formats

The linker's output depends on the target platform's binary format.

**ELF** (Executable and Linkable Format) is the standard on Linux and most Unix-like systems. An ELF file has two views: segments (for runtime, describing what to load into memory) and sections (for linking, describing code, data, and metadata). The program header table lists segments with their memory protections and alignment; the section header table lists sections and their types (SHT_PROGBITS for code/data, SHT_STRTAB for strings, SHT_SYMTAB for symbols, SHT_REL/SHT_RELA for relocations, SHT_DYNAMIC for dynamic linking metadata).

**Mach-O** (Mach Object) is used on macOS and iOS. It organizes data into load commands describing how to set up the process image. Mach-O supports fat (universal) binaries containing code for multiple architectures (e.g., ARM64 and x86-64), allowing a single file to run on different hardware. Load commands specify segments, dynamic libraries, entry points, and UUID identifiers. The __LINKEDIT segment contains compressed or uncompressed link edit information.

**PE** (Portable Executable) is used on Windows and UEFI. Like ELF, it has sections (.text, .data, .reloc) and relies on relocation tables for ASLR. The import address table (IAT) hooks function calls by library, allowing the dynamic linker to insert library functions into the executable's address space.

## Linker Scripts

Complex systems—especially embedded systems with unusual memory layouts—require fine-grained control over output layout. Linker scripts (on Unix: GNU `ld` syntax) specify where sections should be placed, how to handle overlays, and what symbols should be exported. A script might, for example, place init code at a specific reset vector, place the kernel at beginning of high memory, and designate a memory-mapped I/O region.

## Lazy Binding and Indirect Calls

**Lazy binding** defers resolution of a library function until its first call, improving startup time. A GOT (Global Offset Table) entry initially points to a PLT (Procedure Linkage Table) stub; the stub calls into the dynamic linker to resolve the symbol and patch the GOT. Subsequent calls jump directly through the GOT, avoiding the resolver overhead. This mechanism relies on a trampoline function (e.g., `dyld_stub_binder` on macOS) that the linker installs.

On systems without lazy binding or for performance-critical code, direct binding or preemption can be used to resolve symbols statically or at bind time.

## Linker Implementations

- **GNU `ld`** is the traditional Unix linker, derived from earlier tools. It supports linker scripts and complex relocation schemes, but can be slow on large projects.
- **Gold** is a faster, ELF-only alternative from the LLVM project, supporting parallelized linking.
- **lld** is LLVM's cross-platform linker, designed as a drop-in replacement with similar performance to Gold and support for ELF, Mach-O, COFF, and more.
- **mold** is a highly parallelized linker for Linux ELF, optimized for build speed at some cost in memory footprint.

## Trade-offs and Considerations

Static linking eliminates runtime symbol resolution overhead and simplifies deployment (no library mismatches), but increases binary size and complicates library updates (each program must be relinked). Dynamic linking saves space and enables centralized bug fixes, but introduces runtime overhead, versioning complexity, and the risk of breaking changes.

For large codebases, limiting the number of unique symbols (by using internal visibility or hiding symbols inside namespaces) can significantly improve linking and loading performance. Symbol stripping removes debugging information and internal symbols to reduce size.

See also: ELF binary format, memory management, debuggers, process startup.