# Systems: Binary Formats — ELF, Mach-O, PE, and Debug Information

Binary formats standardize how executable code, data, and metadata are stored in files and loaded into memory. Understanding format structure is essential for implementing loaders, debuggers, and analysis tools.

## ELF: Executable and Linkable Format

**ELF** (developed for System V Release 4, adopted as Unix standard in 1999) is the primary format for executable code on Linux, BSD, and most non-Windows Unix systems.

### Structure

An ELF file consists of:

1. **ELF header** (64 bytes on 64-bit): magic number (0x7F 'E' 'L' 'F'), file class (32/64-bit), data format (little/big endian), OS ABI, entry point, offsets to program and section header tables.
2. **Program header table**: describes memory segments (PT_LOAD for executable code, PT_DYNAMIC for dynamic linking metadata, PT_INTERP for the dynamic linker path). Each entry specifies the segment's virtual address, memory size, file offset, permissions (readable, writable, executable), and alignment.
3. **Section header table**: describes logical sections (.text for code, .data for initialized globals, .bss for zero-initialized globals, .rodata for read-only data, .strtab for strings (symbol names, etc.), .symtab for symbol table, .rel/.rela for relocation entries, .dynsym for dynamic symbols, .dynamic for dynamic linking data).
4. **Sections and segments**: sections are for linking; segments are for execution. A segment may contain multiple sections. The loader uses the program header table to set up memory; the linker uses sections.

### Symbol Table

The `.symtab` section lists all symbols (functions, global variables, static symbols). Each entry specifies:

- **Name**: offset into the `.strtab` string table.
- **Value**: the symbol's address (or offset, for relocatable files).
- **Size**: size of the data or code (e.g., function length).
- **Type**: function, object, or section.
- **Binding**: local visibil(only in this file), global (exported), or weak (can be overridden).
- **Section**: which section contains this symbol (0 = undefined, special values for absolute/common symbols).

The `.dynsym` section is a subset of `.symtab`, containing only symbols needed at runtime (exported functions and imported symbols).

### Relocations

The `.rel` or `.rela` sections describe adjustments needed at link time or load time. Each relocation specifies:

- **Offset**: where in the section the adjustment is needed.
- **Type**: how to compute the new value (e.g., R_X86_64_RELATIVE adds the image base).
- **Symbol**: which symbol is referenced (if any).
- **Addend**: immediate value used in the computation (in .rela; in .rel, addend is at the target location).

Dynamic linking stores compressed relocation information in `.dynsym`, .reloc, or .rel.dyn sections, enabling the dynamic linker to efficiently bind symbols as the program loads.

## Mach-O: Mach Object Format

**Mach-O** (NeXTSTEP heritage, used on macOS and iOS) organizes data as a sequence of load commands following a header.

### Structure

1. **Mach-O header**: specifies CPU type/subtype, file type (executable, dynamic library, relocatable object, core dump), number of load commands, entry point.
2. **Load commands**: variable-length directives specifying how to load the binary. Types include:
   - **LC_SEGMENT / LC_SEGMENT_64**: a memory segment with start address, size, permissions, and contained sections.
   - **LC_DYLD_INFO / LC_DYLD_INFO_ONLY**: compressed linking information (replaces older symbol/relocation tables).
   - **LC_LOAD_DYLIB / LC_LOAD_DYLIB_WEAK_DYLIB**: paths to dependent dynamic libraries.
   - **LC_MAIN**: entry point and stack size.
   - **LC_UUID**: unique identifier for the binary (used to associate symbol files).
   - **LC_BUILD_VERSION**: OS version, SDK version, and build tools (Clang, Swift, ld).

3. **__LINKEDIT segment**: contains symbol tables, string tables, relocation info, and exported symbols.

### Universal Binaries

Mach-O supports fat binaries (magic number 0xCAFEBABE in big-endian): multiple Mach-O images concatenated with offsets in a header. This allows a single file to contain ARM64 and x86-64 code, letting one executable run on different Apple hardwares.

### Load Commands vs. Sections

Unlike ELF's two-view design (segments for loading, sections for linking), Mach-O segments contain sections for convenience, but the load commands are the primary authority. Sections are named (e.g., __text, __data) and grouped by segment (e.g., all __text sections belong to the __TEXT segment).

## PE: Portable Executable

**PE** (Windows NT 3.1 onwards) evolved from COFF and maintains backward compatibility with a DOS stub header.

### Structure

1. **DOS header** (legacy): starts with 'MZ' (0x4D 0x5A), points to the PE header offset. Contains a minimal DOS program that prints "This program cannot be run in DOS mode."
2. **PE header**: specifies machine type (0x14c for x86, 0x8664 for x64), link timestamp, symbol table offset (for older formats; modern PE omits this), size of optional header.
3. **Optional header**: size, characteristics (executable, DLL, ASLR-enabled, etc.), entry point, base address (assumed load address, adjusted via .reloc if ASLR changes it), section alignment, subsystem (native kernel, Windows GUI, console, etc.).
4. **Data directories**: array of RVAs (relative virtual addresses) to key structures:
   - **Export table**: symbols exported by this DLL.
   - **Import table**: DLLs and their functions imported.
   - **Resource section**: images, dialogs, version info.
   - **Relocation table**: addresses that need adjustment under ASLR.
   - **.NET metadata**: pointers to CLR metadata if this is a .NET assembly.

5. **Sections**: .text (code), .data (writable data), .rdata (read-only data), .rsrc (resources), .reloc (relocation table).

### Import Address Table (IAT)

To handle ASLR, the import address table is writable but initially zero; the dynamic linker fills it with actual library function addresses. Code calls functions indirectly through the IAT, enabling symbol resolution at load time without modifying code pages (which are read-only and shared for ASLR efficiency).

## DWARF: Debug Information Format

**DWARF** (Debugging With Attributed Record Formats) is a standardized format for embedding debug information in ELF, Mach-O, and PE files. It describes source language constructs (functions, variables, types) and maps instructions to source lines.

### Structure (simplified)

DWARF is organized into "compilation units" (.debug_info section), each describing one source file. Within each unit, DIEs (Debugging Information Entries) form a tree:

- **DW_TAG_compile_unit**: root of a compilation unit.
- **DW_TAG_subprogram**: function or method.
- **DW_TAG_variable**: local variable, parameter, or global.
- **DW_TAG_class_type / DW_TAG_structure_type**: defined class or struct.

Each DIE has attributes (DW_AT_*):

- **DW_AT_name**: symbol name.
- **DW_AT_type**: reference to the type DIE.
- **DW_AT_location**: description of where the variable lives (stack offset, register, optimized away, etc.) as a DWARF expression.
- **DW_AT_decl_file / DW_AT_decl_line**: source file and line where declared.

The .debug_line section maps instruction addresses to source file, line number, and column. Debuggers use this to set breakpoints ("break at line 42") and show source code during stepping.

### Compact Representation

DWARF is verbose; versions 4+ support alternative representations (e.g., debug_str_offsets to reduce string duplication). Version 5 adds a format for compressing debug data into separate .o files.

## Comparison

| Aspect | ELF | Mach-O | PE |
|--------|-----|--------|-----|
| Primary platforms | Linux, BSD, Unix | macOS, iOS | Windows, UEFI |
| Design philosophy | sections for linking, segments for execution | load commands for flexibility | evolutionary from COFF |
| Symbol visibility | local/global/weak via symbol binding | private/external via symbol type bits | export/import tables |
| Relocation approach | explicit relocation entries (.rel/.rela) | compressed binding info in LC_DYLD_INFO | relocation table (.reloc) |
| Multi-arch support | separate files per arch | fat binary single file | separate PE per arch |
| Debug format | DWARF, stabs | DWARF, stabs, custom | DWARF, PDB |

## Tools for Inspection

- **readelf** (GNU binutils): displays ELF structure, sections, symbols.
- **objdump**: shows disassembly, relocations, and raw section data.
- **nm**: lists symbols.
- **objcopy**: copies, strips, and modifies binaries.
- **otool** (macOS): displays Mach-O load commands and sections.
- **ldd** (Linux): lists dynamic library dependencies.
- **objc** (macOS): analyze Objective-C metadata in Mach-O.

## Stripping and Security

**Stripping** removes debugging symbols and static symbol names to reduce binary size and obscure internal structure. `strip` removes most metadata; `strip -x` removes only global symbols (keeping local ones for static linking). Completely stripped binaries are harder to debug but smaller and (marginally) harder to reverse-engineer.

See also: linking and loading, debuggers, memory management, system calls.