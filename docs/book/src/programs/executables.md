# Executables

`bitter build` writes its output marked as executable, but adds nothing to
it. An executable file format's header is BitterASM code that the program
writes itself, like any other data.

## Adding a header

Call a header macro first thing in the first input file, before any
`section` statement, so the header starts the image:

```basm,ignore
from std.formats.elf import *

elf64_executable EM_X86_64, _start
```

| Format | Module | Header macro |
|---|---|---|
| ELF, 64-bit | `std.formats.elf` | `elf64_executable EM_X86_64, _start` |
| ELF, 32-bit | `std.formats.elf` | `elf32_executable EM_RISCV, _start` |
| PE32+ (Windows console) | `std.formats.pe` | `pe64_executable IMAGE_FILE_MACHINE_AMD64, _start` |
| Mach-O, 64-bit | `std.formats.macho` | `macho64_executable CPU_TYPE_X86_64, CPU_SUBTYPE_X86_64_ALL, _start` |

The last argument is the entry point, a `pub` [label](labels.md) that can be
in any input file.

Optional parameters:

- **ELF:** `load_address`, `segment_flags` (`PF_R`, `PF_W`, `PF_X`), `flags`.
- **PE:** `image_base`.
- **Mach-O:** `vm_address`.

Without a header, `bitter build` writes a flat binary, like `nasm -f bin`.

## What the headers support

Each format maps the whole image as one segment, readable and executable by
default. There's no dynamic linking, no imports and no relocations.

## Writing a format of your own

The headers are built from pieces any other format can use:

- **`std.bitter.link`'s `image_start` and `image_end`**, positions `bitter`
  resolves against the linked image. `span(image_start, image_end)` is the
  image's size in bytes. See [Linking](linking.md#positions-bitter-provides).
- **`std.bitter.deferred`'s arithmetic**: `add`, `sub`, `band`, `shr` and
  more, which work on positions that aren't known until `bitter` lays the
  image out.
- **`std.bitter.layout`'s `align n`**, which emits zero bytes up to the next
  multiple of `n`.
- **`std.bitter.layout`'s `pad_image n`**, which pads the finished image with
  zeros to a multiple of `n`. It takes no space where it's written, so a
  header at the start can still pad the end.

Reading `std/formats/elf.basm` is a good way to see how they fit together.
